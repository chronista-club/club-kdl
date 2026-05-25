//! Error type for [`crate::compose`] and [`crate::from_path`].

use std::path::PathBuf;

/// Anything that can go wrong while composing a multi-file KDL document.
#[derive(Debug, thiserror::Error)]
pub enum ComposeError {
    /// Failed to read a source file from disk.
    #[error("io error reading {path}: {source}")]
    Io {
        /// The path the resolver was trying to read.
        path: PathBuf,
        /// The underlying [`std::io::Error`].
        #[source]
        source: std::io::Error,
    },

    /// The KDL parser rejected the contents of a source file. The
    /// [`kdl::KdlError`] error carries its own span information; this variant
    /// keeps the offending path so the user can locate the file.
    #[error("kdl parse error in {path}: {source}")]
    Parse {
        /// The path whose contents failed to parse.
        path: PathBuf,
        /// The underlying parse error.
        #[source]
        source: kdl::KdlError,
    },

    /// A canonical path appears twice on the include stack — the resolver
    /// would otherwise recurse forever.
    ///
    /// `stack` lists the active path stack at the point of detection, in
    /// include order from the root file to the offending re-include.
    #[error("include cycle detected: {}", display_cycle(.stack))]
    Cycle {
        /// The include stack, root-most first.
        stack: Vec<PathBuf>,
    },

    /// A directive node was structurally invalid — wrong variant name, missing
    /// path argument, unsupported property, malformed `rename={}`, etc.
    #[error("invalid directive in {path}: {message}")]
    InvalidDirective {
        /// The path of the file that contained the bad directive.
        path: PathBuf,
        /// Human-readable diagnostic.
        message: String,
    },

    /// A `(<)glob` pattern was syntactically malformed.
    #[error("invalid glob pattern in {path}: {source}")]
    Glob {
        /// The path of the file that contained the bad pattern.
        path: PathBuf,
        /// The underlying [`glob::PatternError`].
        #[source]
        source: glob::PatternError,
    },

    /// `club-kdl` failed to deserialize the composed document. Only surfaced
    /// from [`crate::from_path`] — `compose()` itself never deserializes.
    #[error("deserialize error: {source}")]
    Deserialize {
        /// The underlying [`club_kdl::Error`].
        #[source]
        source: club_kdl::Error,
    },
}

/// Render an include stack as `a.kdl → b.kdl → a.kdl` for the [`ComposeError::Cycle`]
/// message. A KDL include cycle is much easier to debug when the chain is
/// visible, so we trade a longer message for clarity.
fn display_cycle(stack: &[PathBuf]) -> String {
    stack
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(" → ")
}

/// `Result` alias used throughout [`crate`].
pub type Result<T> = std::result::Result<T, ComposeError>;
