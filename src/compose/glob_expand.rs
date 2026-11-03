//! Glob matching with fallible filesystem traversal.
//!
//! `glob::glob` suppresses some metadata errors before yielding its results.
//! Keep its pattern grammar, but own every filesystem operation so a failed
//! search cannot be mistaken for an empty or complete set of imports.

use std::fs::{self, Metadata};
use std::io;
use std::path::{Component, Path, PathBuf};

use glob::{MatchOptions, Pattern};

use super::{ComposeError, Result};

enum Kind {
    Literal,
    Wildcard,
    Recursive,
}

struct Segment {
    pattern: Pattern,
    kind: Kind,
}

struct Pending {
    path: PathBuf,
    metadata: Metadata,
    index: usize,
    // Only the active branch of this recursive component, never a global set.
    ancestors: Vec<PathBuf>,
}

pub(super) fn expand(base: &Path, pattern: &str, importer: &Path) -> Result<Vec<PathBuf>> {
    let compile = |text: &str| {
        Pattern::new(text).map_err(|source| ComposeError::Glob {
            path: importer.to_path_buf(),
            source,
        })
    };
    // Validate the whole expression as well as its individual components.
    compile(pattern)?;
    let mut root = base.to_path_buf();
    let mut explicit_root = false;
    let mut components = Path::new(pattern).components();
    while let Some(component @ (Component::Prefix(_) | Component::RootDir)) =
        components.clone().next()
    {
        root.push(component.as_os_str());
        components.next();
        explicit_root = true;
    }

    // Match glob's treatment of Windows verbatim paths. Canonical drive paths
    // (VerbatimDisk) work; other verbatim namespaces were never searched.
    #[cfg(windows)]
    if let Some(Component::Prefix(prefix)) = root.components().next()
        && prefix.kind().is_verbatim()
        && !matches!(prefix.kind(), std::path::Prefix::VerbatimDisk(_))
    {
        return Ok(Vec::new());
    }

    let mut segments = Vec::<Segment>::new();
    for text in components
        .as_path()
        .to_str()
        .unwrap()
        .split_terminator(std::path::is_separator)
    {
        let kind = if text == "**" {
            if segments
                .last()
                .is_some_and(|s| matches!(s.kind, Kind::Recursive))
            {
                continue;
            }
            Kind::Recursive
        } else if text.contains(['*', '?', '[']) {
            Kind::Wildcard
        } else {
            Kind::Literal
        };
        segments.push(Segment {
            pattern: compile(text)?,
            kind,
        });
    }
    let require_directory = pattern
        .chars()
        .next_back()
        .is_some_and(std::path::is_separator);
    let metadata = if explicit_root {
        // An absent drive/share in an absolute pattern is a genuine no-match.
        let Some(metadata) = literal_metadata(&root)? else {
            return Ok(Vec::new());
        };
        metadata
    } else {
        // The importer was already read from this directory. Its disappearance
        // during expansion must not be reclassified as an empty search.
        fs::metadata(&root).map_err(|e| io_error(&root, e))?
    };
    let mut pending = vec![Pending {
        path: root,
        metadata,
        index: 0,
        ancestors: Vec::new(),
    }];
    let mut matches = Vec::new();
    while let Some(state) = pending.pop() {
        let Some(segment) = segments.get(state.index) else {
            if state.metadata.is_file() && !require_directory {
                matches.push(state.path);
            }
            continue;
        };
        if !state.metadata.is_dir() {
            continue;
        }
        match segment.kind {
            Kind::Literal => {
                let path = state.path.join(segment.pattern.as_str());
                if let Some(metadata) = literal_metadata(&path)? {
                    pending.push(Pending {
                        path,
                        metadata,
                        index: state.index + 1,
                        ancestors: Vec::new(),
                    });
                }
            }
            Kind::Wildcard => {
                let mut paths = read_directory(&state.path)?;
                // glob includes virtual . and .. only for a literal leading dot.
                if segment.pattern.as_str().starts_with('.') {
                    for name in [".", ".."] {
                        if segment.pattern.matches_with(name, MatchOptions::new()) {
                            let path = state.path.join(name);
                            let metadata = fs::metadata(&path).map_err(|e| io_error(&path, e))?;
                            pending.push(Pending {
                                path,
                                metadata,
                                index: state.index + 1,
                                ancestors: Vec::new(),
                            });
                        }
                    }
                }
                paths.retain(|path| matches_name(segment, path));
                for path in paths {
                    // An enumerated match disappearing is an error, not no-match.
                    let metadata = fs::metadata(&path).map_err(|e| io_error(&path, e))?;
                    pending.push(Pending {
                        path,
                        metadata,
                        index: state.index + 1,
                        ancestors: Vec::new(),
                    });
                }
            }
            Kind::Recursive => {
                let identity =
                    fs::canonicalize(&state.path).map_err(|e| io_error(&state.path, e))?;
                if state.ancestors.contains(&identity) {
                    return Err(io_error(
                        &state.path,
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "directory cycle while expanding glob",
                        ),
                    ));
                }
                let mut ancestors = state.ancestors;
                ancestors.push(identity);
                for path in read_directory(&state.path)? {
                    let metadata = fs::metadata(&path).map_err(|e| io_error(&path, e))?;
                    if metadata.is_dir() {
                        pending.push(Pending {
                            path: path.clone(),
                            metadata: metadata.clone(),
                            index: state.index,
                            ancestors: ancestors.clone(),
                        });
                    }
                    // A terminal ** yields directories only in glob, which the
                    // composer excludes. Otherwise match this child against the
                    // next component; this also implements the zero-depth case.
                    if let Some(next) = segments.get(state.index + 1)
                        && matches_name(next, &path)
                    {
                        pending.push(Pending {
                            path,
                            metadata,
                            index: state.index + 2,
                            ancestors: Vec::new(),
                        });
                    }
                }
            }
        }
    }
    matches.sort();
    Ok(matches)
}

fn matches_name(segment: &Segment, path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| segment.pattern.matches_with(name, MatchOptions::new()))
}

fn read_directory(path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = fs::read_dir(path)
        .map_err(|e| io_error(path, e))?
        .map(|entry| entry.map(|e| e.path()).map_err(|e| io_error(path, e)))
        .collect::<Result<Vec<_>>>()?;
    paths.sort();
    Ok(paths)
}

/// Absence of a literal candidate is a valid no-match. A dangling symlink or
/// permission failure is not absence and must not disappear from the result.
fn literal_metadata(path: &Path) -> Result<Option<Metadata>> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(source)
            if matches!(
                source.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            match fs::symlink_metadata(path) {
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                    ) =>
                {
                    Ok(None)
                }
                Err(error) => Err(io_error(path, error)),
                Ok(_) => Err(io_error(path, source)),
            }
        }
        Err(source) => Err(io_error(path, source)),
    }
}

fn io_error(path: &Path, source: io::Error) -> ComposeError {
    ComposeError::Io {
        path: path.to_path_buf(),
        source,
    }
}
