//! Glob regression coverage through the public composer and real scratch files.

#![cfg(feature = "compose")]

use club_kdl::compose::{ComposeError, compose};
use club_kdl::{Document, Value};
use std::fs;

fn fixture(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("scratch directory");
    for (name, text) in files {
        let path = dir.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).expect("fixture directory");
        fs::write(path, text).expect("fixture file");
    }
    dir
}

fn names(doc: &Document<'_>) -> Vec<String> {
    doc.nodes()
        .iter()
        .map(|node| {
            node.arg(0)
                .and_then(Value::as_string)
                .expect("fixture node has a string argument")
                .to_owned()
        })
        .collect()
}

#[cfg(unix)]
fn assert_io_error(
    result: Result<Document<'static>, ComposeError>,
    suffix: &str,
    kind: std::io::ErrorKind,
) {
    match result.expect_err("glob must not return a partial successful document") {
        ComposeError::Io { path, source } => {
            assert!(path.ends_with(suffix), "unexpected error path: {path:?}");
            assert_eq!(source.kind(), kind, "{path:?}: {source}");
        }
        other => panic!("expected Io for {suffix}, got {other:?}"),
    }
}

#[cfg(unix)]
mod permissions {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    pub fn unprivileged() -> bool {
        let output = std::process::Command::new("id")
            .arg("-u")
            .output()
            .expect("read effective UID");
        assert!(output.status.success(), "id -u failed: {output:?}");
        if String::from_utf8(output.stdout)
            .expect("numeric effective UID")
            .trim()
            == "0"
        {
            eprintln!("SKIP: effective UID 0 bypasses fixture permission restrictions");
            false
        } else {
            true
        }
    }

    /// Restores permissions during unwinding before the TempDir is removed.
    pub struct Denied {
        path: PathBuf,
        original: fs::Permissions,
    }

    impl Denied {
        pub fn new(path: &Path) -> Self {
            let original = fs::metadata(path)
                .expect("original permissions")
                .permissions();
            fs::set_permissions(path, fs::Permissions::from_mode(0o000))
                .expect("deny fixture access");
            Self {
                path: path.to_owned(),
                original,
            }
        }
    }

    impl Drop for Denied {
        fn drop(&mut self) {
            if let Err(error) = fs::set_permissions(&self.path, self.original.clone()) {
                eprintln!("failed to restore permissions for {:?}: {error}", self.path);
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn recursive_glob_rejects_unreadable_branch_instead_of_partial_success() {
    if !permissions::unprivileged() {
        return;
    }
    let dir = fixture(&[
        ("root.kdl", "local \"before\"\n(<)glob \"./parts/**/*.kdl\""),
        ("parts/a-good.kdl", "item \"good\""),
        ("parts/blocked/secret.kdl", "item \"secret\""),
    ]);
    let blocked = dir.path().join("parts/blocked");
    let _permissions = permissions::Denied::new(&blocked);
    assert_eq!(
        fs::read_dir(&blocked).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied,
        "fixture must actually deny directory traversal"
    );
    assert_io_error(
        compose(dir.path().join("root.kdl")),
        "parts/blocked",
        std::io::ErrorKind::PermissionDenied,
    );
}

#[cfg(unix)]
#[test]
fn literal_glob_rejects_unsearchable_parent_instead_of_zero_matches() {
    if !permissions::unprivileged() {
        return;
    }
    let dir = fixture(&[
        (
            "root.kdl",
            "local \"before\"\n(<)glob \"./blocked/target.kdl\"",
        ),
        ("blocked/target.kdl", "item \"secret\""),
    ]);
    let blocked = dir.path().join("blocked");
    let target = blocked.join("target.kdl");
    let _permissions = permissions::Denied::new(&blocked);
    assert_eq!(
        fs::metadata(&target).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied,
        "fixture must actually deny target lookup"
    );
    match compose(dir.path().join("root.kdl")).expect_err("inaccessible is not zero matches") {
        ComposeError::Io { path, source } => {
            assert!(
                path.ends_with("blocked") || path.ends_with("blocked/target.kdl"),
                "error must identify the inaccessible path: {path:?}"
            );
            assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
        }
        other => panic!("expected Io, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn glob_rejects_broken_symlink_instead_of_skipping_metadata_failure() {
    let dir = fixture(&[
        ("root.kdl", "local \"before\"\n(<)glob \"./parts/*.kdl\""),
        ("parts/a-good.kdl", "item \"good\""),
    ]);
    std::os::unix::fs::symlink("missing.kdl", dir.path().join("parts/z-broken.kdl"))
        .expect("broken fixture symlink");
    assert_io_error(
        compose(dir.path().join("root.kdl")),
        "parts/z-broken.kdl",
        std::io::ErrorKind::NotFound,
    );
}

#[cfg(unix)]
#[test]
fn glob_read_failure_rejects_preceding_successful_files() {
    if !permissions::unprivileged() {
        return;
    }
    let dir = fixture(&[
        ("root.kdl", "local \"before\"\n(<)glob \"./parts/*.kdl\""),
        ("parts/a-good.kdl", "item \"good\""),
        ("parts/z-denied.kdl", "item \"secret\""),
    ]);
    let denied = dir.path().join("parts/z-denied.kdl");
    let _permissions = permissions::Denied::new(&denied);
    assert_eq!(
        fs::read_to_string(&denied).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied,
        "fixture must actually deny file reads"
    );
    assert_io_error(
        compose(dir.path().join("root.kdl")),
        "parts/z-denied.kdl",
        std::io::ErrorKind::PermissionDenied,
    );
}

#[test]
fn glob_parse_failure_rejects_preceding_successful_files() {
    let dir = fixture(&[
        ("root.kdl", "local \"before\"\n(<)glob \"./parts/*.kdl\""),
        ("parts/a-good.kdl", "item \"good\""),
        ("parts/z-broken.kdl", "item {"),
    ]);
    match compose(dir.path().join("root.kdl")).expect_err("invalid KDL must fail composition") {
        ComposeError::Parse { path, .. } => assert!(path.ends_with("parts/z-broken.kdl")),
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn glob_file_cycle_rejects_preceding_successful_files() {
    let dir = fixture(&[
        ("root.kdl", "local \"before\"\n(<)glob \"./parts/*.kdl\""),
        ("parts/a-good.kdl", "item \"good\""),
        ("parts/z-cycle.kdl", "(<)file \"../root.kdl\""),
    ]);
    match compose(dir.path().join("root.kdl")).expect_err("glob/file cycle must fail") {
        ComposeError::Cycle { stack } => {
            assert_eq!(stack.len(), 3);
            assert!(stack[0].ends_with("root.kdl"));
            assert!(stack[1].ends_with("parts/z-cycle.kdl"));
            assert_eq!(stack[0], stack[2]);
        }
        other => panic!("expected Cycle, got {other:?}"),
    }
}

#[test]
fn glob_preserves_extensionless_and_other_extensions_in_filename_order() {
    let dir = fixture(&[
        (
            "root.kdl",
            "local \"before\"\n(<)glob \"./parts/*\"\nlocal \"after\"",
        ),
        ("parts/a.kdl", "item \"kdl\""),
        ("parts/a.data", "item \"data\""),
        ("parts/a", "item \"extensionless\""),
    ]);
    let doc = compose(dir.path().join("root.kdl")).expect("all extensions remain literal");
    assert_eq!(
        names(&doc),
        ["before", "extensionless", "data", "kdl", "after"]
    );
}

#[test]
fn file_glob_file_chain_uses_each_importers_directory_and_preserves_order() {
    let dir = fixture(&[
        (
            "root.kdl",
            "local \"root-before\"\n(<)file \"./nested/entry.kdl\"\nlocal \"root-after\"",
        ),
        (
            "nested/entry.kdl",
            "local \"entry-before\"\n(<)glob \"./parts/*.kdl\"\nlocal \"entry-after\"",
        ),
        (
            "nested/parts/a.kdl",
            "local \"a-before\"\n(<)file \"../shared.kdl\"\nlocal \"a-after\"",
        ),
        ("nested/parts/b.kdl", "local \"b\""),
        ("nested/shared.kdl", "item \"nested-shared\""),
        ("parts/a.kdl", "item \"wrong-root-parts\""),
        ("shared.kdl", "item \"wrong-root-shared\""),
    ]);
    let doc = compose(dir.path().join("root.kdl")).expect("relative recursive composition");
    assert_eq!(
        names(&doc),
        [
            "root-before",
            "entry-before",
            "a-before",
            "nested-shared",
            "a-after",
            "b",
            "entry-after",
            "root-after"
        ]
    );
}

#[test]
fn glob_options_transform_each_file_without_changing_surrounding_siblings() {
    let dir = fixture(&[
        (
            "root.kdl",
            "local \"User\"\n(<)glob \"./parts/*.kdl\" as=\"ns\" {\n only \"User\" \"Memory\" \"Internal\"\n except \"Internal\"\n rename \"User\" \"Account\"\n}\nlocal \"Internal\"",
        ),
        ("parts/a.kdl", "struct \"User\"\nstruct \"Internal\""),
        ("parts/b.kdl", "struct \"Memory\"\nstruct \"Ignored\""),
    ]);
    let doc = compose(dir.path().join("root.kdl")).expect("glob options");
    assert_eq!(names(&doc), ["User", "ns.Account", "ns.Memory", "Internal"]);
    assert_eq!(doc.nodes()[0].name(), "local");
    assert_eq!(doc.nodes()[3].name(), "local");
}

#[test]
fn empty_and_missing_globs_succeed_and_real_directories_are_excluded() {
    let dir = fixture(&[
        (
            "root.kdl",
            "(<)glob \"./parts/*.absent\"\n(<)glob \"./missing/*.kdl\"\n(<)glob \"./missing.kdl\"\n(<)glob \"./parts/*\"\nlocal \"after\"",
        ),
        ("parts/a.kdl", "item \"real\""),
        ("parts/sub/ignored.kdl", "item \"must-not-recurse\""),
    ]);
    let doc = compose(dir.path().join("root.kdl")).expect("true no-match is not an error");
    assert_eq!(names(&doc), ["real", "after"]);
}
