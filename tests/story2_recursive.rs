//! User Story 2 -- shrink a whole folder tree at once.

#[path = "support/fixtures.rs"]
mod fixtures;

/// quickstart 2.1 -- FR-004, FR-010
#[test]
fn mirrors_subfolder_structure() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg(root, "root.jpg", 400, 300);
    fixtures::write_jpeg(root, "a/inner.jpg", 400, 300);
    fixtures::write_jpeg(root, "a/b/deep.jpg", 400, 300);

    fixtures::srnk(root)
        .args(["--factor", "50", "--recursive"])
        .assert()
        .code(0);

    let out = root.join("srnk");
    for rel in ["root.jpg", "a/inner.jpg", "a/b/deep.jpg"] {
        let copy = out.join(rel);
        assert!(copy.is_file(), "{rel} must be mirrored");
        assert_eq!(fixtures::dimensions(&copy), (283, 212));
    }
}

/// quickstart 2.2 -- FR-004
#[test]
fn non_recursive_ignores_subfolders() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg(root, "root.jpg", 400, 300);
    fixtures::write_jpeg(root, "a/inner.jpg", 400, 300);
    fixtures::write_jpeg(root, "a/b/deep.jpg", 400, 300);

    fixtures::srnk(root)
        .args(["--factor", "50"])
        .assert()
        .code(0);

    assert_eq!(fixtures::names_in(&root.join("srnk")), vec!["root.jpg"]);
    assert!(!root.join("srnk").join("a").exists());
}

/// quickstart 2.3 -- FR-011, SC-006
#[test]
fn never_rescans_output_folder() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg(root, "root.jpg", 400, 300);
    fixtures::write_jpeg(root, "a/inner.jpg", 400, 300);

    let first = fixtures::srnk(root)
        .args(["--factor", "50", "--recursive"])
        .assert()
        .code(0);
    let first_stdout = String::from_utf8_lossy(&first.get_output().stdout).to_string();
    let dims_after_first = fixtures::dimensions(&root.join("srnk").join("root.jpg"));

    let second = fixtures::srnk(root)
        .args(["--factor", "50", "--recursive"])
        .assert()
        .code(0);
    let second_stdout = String::from_utf8_lossy(&second.get_output().stdout).to_string();

    assert_eq!(
        fixtures::summary_field(&first_stdout, "resized"),
        fixtures::summary_field(&second_stdout, "resized"),
        "the resized count must be stable across runs"
    );
    assert_eq!(fixtures::summary_field(&second_stdout, "resized"), "2");
    assert!(!root.join("srnk").join("srnk").exists(), "no srnk/srnk");
    assert_eq!(
        fixtures::dimensions(&root.join("srnk").join("root.jpg")),
        dims_after_first,
        "copies must never be re-shrunk"
    );
}

/// quickstart 2.4 -- FR-010
#[test]
fn no_empty_mirrored_folders() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg(root, "root.jpg", 200, 100);
    fixtures::write_bytes(root, "docs/readme.txt", b"text only");

    fixtures::srnk(root)
        .args(["--factor", "50", "--recursive"])
        .assert()
        .code(0);

    assert!(!root.join("srnk").join("docs").exists());
    assert_eq!(fixtures::names_in(&root.join("srnk")), vec!["root.jpg"]);
}

/// quickstart 2.5 -- FR-016
#[test]
fn does_not_follow_folder_symlinks() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg(root, "root.jpg", 200, 100);
    fixtures::write_jpeg(root, "a/inner.jpg", 200, 100);

    let link = root.join("a").join("loop");
    #[cfg(unix)]
    let created = std::os::unix::fs::symlink(root, &link).is_ok();
    #[cfg(windows)]
    let created = std::os::windows::fs::symlink_dir(root, &link).is_ok();
    if !created {
        eprintln!("skipped: this platform does not permit creating folder symlinks");
        return;
    }

    let assertion = fixtures::srnk(root)
        .args(["--factor", "50", "--recursive"])
        .assert()
        .code(0);
    let stdout = String::from_utf8_lossy(&assertion.get_output().stdout).to_string();

    assert_eq!(
        fixtures::summary_field(&stdout, "resized"),
        "2",
        "the symlink target must not be traversed"
    );
    assert!(!root.join("srnk").join("a").join("loop").exists());
}
