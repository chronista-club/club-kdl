//! Successful glob matching stays compatible while traversal errors become visible.

#![cfg(feature = "compose")]

use club_kdl::compose::compose;
use std::fs;

#[cfg(windows)]
#[test]
fn nonexistent_absolute_drive_is_an_empty_match() {
    let missing = ('A'..='Z').find_map(|drive| {
        let root = std::path::PathBuf::from(format!("{drive}:\\"));
        match (fs::metadata(&root), fs::symlink_metadata(&root)) {
            (Err(a), Err(b))
                if a.kind() == std::io::ErrorKind::NotFound
                    && b.kind() == std::io::ErrorKind::NotFound =>
            {
                Some(root)
            }
            _ => None,
        }
    });
    let Some(missing) = missing else {
        eprintln!("SKIP: no nonexistent drive is available for this fixture");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root.kdl");
    let pattern = missing.join("*.kdl");
    fs::write(&root, format!("(<)glob {pattern:?}\nlocal \"after\"")).unwrap();
    let doc = compose(root).expect("a genuinely absent root is not an I/O failure");
    assert_eq!(doc.nodes().len(), 1);
    assert_eq!(doc.nodes()[0].name(), "local");
}

#[test]
fn successful_globs_match_the_existing_pattern_engine() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "a.kdl",
        "parts/a.kdl",
        "parts/b.kdl",
        "parts/Upper.KDL",
        "parts/plain",
        "parts/.hidden.kdl",
        "parts/sub/a.kdl",
        "parts/sub/deeper/b.kdl",
        "parts/.hidden-dir/a.kdl",
    ] {
        let path = dir.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("item {name:?}")).unwrap();
    }
    fs::create_dir_all(dir.path().join("parts/empty")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink("sub", dir.path().join("parts/alias")).unwrap();
        symlink("a.kdl", dir.path().join("parts/file-alias.kdl")).unwrap();
    }
    for pattern in [
        "parts/*",
        "parts/*.kdl",
        "parts/?.kdl",
        "parts/[ab].kdl",
        "parts/[!b]*",
        "parts/**",
        "parts/**/*.kdl",
        "parts/**/**/*.kdl",
        "parts/**/*/*.kdl",
        "parts/**/*/**/*.kdl",
        "parts/.*/a.kdl",
        "parts/[.]*/a.kdl",
        "parts/sub/../a.kdl",
        "parts/*/",
        "parts/no-such/*",
        "parts/**/../*.kdl",
        "parts/**/../**/*.kdl",
    ] {
        let full = dir.path().join(pattern);
        let mut paths = glob::glob(full.to_str().unwrap())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        paths.retain(|path| path.is_file());
        paths.sort();
        let mut expected = Vec::new();
        for path in paths {
            expected.extend(compose(path).unwrap().into_nodes());
        }
        let root = dir.path().join("entry.kdl");
        fs::write(&root, format!("(<)glob {pattern:?}")).unwrap();
        assert_eq!(
            compose(root).unwrap().nodes(),
            expected,
            "pattern: {pattern}"
        );
    }
    let absolute = dir.path().join("parts/*.kdl");
    let root = dir.path().join("entry.kdl");
    fs::write(&root, "(<)glob \"parts/*.kdl\"").unwrap();
    let relative = compose(&root).unwrap();
    // Rust's Debug spelling of a path is also a valid escaped KDL string.
    fs::write(&root, format!("(<)glob {absolute:?}")).unwrap();
    assert_eq!(compose(&root).unwrap(), relative);

    #[cfg(windows)]
    {
        let canonical = dir.path().canonicalize().unwrap().join("parts/*.kdl");
        fs::write(&root, format!("(<)glob {canonical:?}")).unwrap();
        assert_eq!(compose(&root).unwrap(), relative, "verbatim drive path");
        let root_relative: std::path::PathBuf = canonical.components().skip(1).collect();
        fs::write(&root, format!("(<)glob {root_relative:?}")).unwrap();
        assert_eq!(
            compose(&root).unwrap(),
            relative,
            "root-relative path retains importer drive"
        );
    }
}

#[cfg(unix)]
#[test]
fn literal_dangling_symlink_is_an_error_even_when_the_target_does_not_exist() {
    let dir = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink("missing.kdl", dir.path().join("broken.kdl")).unwrap();
    let root = dir.path().join("root.kdl");
    fs::write(&root, "(<)glob \"broken.kdl\"").unwrap();
    match compose(root).expect_err("an existing dangling link is not an absent candidate") {
        club_kdl::compose::ComposeError::Io { path, source } => {
            assert!(path.ends_with("broken.kdl"));
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected metadata error, got {other:?}"),
    }
}

// This fixture requires arbitrary filename bytes; macOS rejects them with EILSEQ.
#[cfg(target_os = "linux")]
#[test]
fn recursion_can_cross_non_utf8_directory_names_without_matching_them_as_text() {
    use std::os::unix::ffi::OsStringExt;
    let dir = tempfile::tempdir().unwrap();
    let parts = dir.path().join("parts");
    let odd = parts.join(std::ffi::OsString::from_vec(vec![0xff]));
    fs::create_dir_all(&odd).unwrap();
    fs::write(odd.join("a.kdl"), "item \"nested\"").unwrap();
    let root = dir.path().join("root.kdl");
    fs::write(&root, "(<)glob \"parts/**/*.kdl\"").unwrap();
    assert_eq!(compose(&root).unwrap().nodes().len(), 1);
    fs::write(&root, "(<)glob \"parts/*/*.kdl\"").unwrap();
    assert!(compose(&root).unwrap().nodes().is_empty());
}

#[cfg(unix)]
#[test]
fn recursive_glob_rejects_directory_symlink_cycles() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("parts")).unwrap();
    fs::write(dir.path().join("parts/a.kdl"), "item \"a\"").unwrap();
    std::os::unix::fs::symlink(".", dir.path().join("parts/loop")).unwrap();
    let root = dir.path().join("root.kdl");
    fs::write(&root, "(<)glob \"parts/**/*.kdl\"").unwrap();
    let error = compose(root).expect_err("recursive directory aliases must not silently truncate");
    assert!(matches!(error, club_kdl::compose::ComposeError::Io { .. }));
}
