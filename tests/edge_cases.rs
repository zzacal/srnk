//! Edge cases and cross-cutting rules from spec.md and quickstart.md.

#[path = "support/fixtures.rs"]
mod fixtures;

/// quickstart E.1 -- rounding to zero pixels
#[test]
fn tiny_image_clamps_to_one_pixel() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg(root, "tiny.jpg", 2, 2);

    fixtures::srnk(root)
        .args(["--factor", "1"])
        .assert()
        .code(0);

    let copy = root.join("srnk").join("tiny.jpg");
    assert_eq!(fixtures::dimensions(&copy), (1, 1));
}

/// quickstart E.2 -- case-insensitive name collision
#[test]
fn case_insensitive_name_collision_reported() {
    let dir = fixtures::temp();
    let root = dir.path();
    if !fixtures::case_sensitive_fs(root) {
        eprintln!("skipped: this filesystem cannot hold shot.jpg and shot.JPG side by side");
        return;
    }
    fixtures::write_jpeg(root, "shot.jpg", 200, 100);
    fixtures::write_jpeg(root, "shot.JPG", 400, 300);

    let assertion = fixtures::srnk(root)
        .args(["--factor", "50"])
        .assert()
        .code(1);
    let output = assertion.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(
        stderr.contains("name collides with") && stderr.contains("(case-insensitive)"),
        "stderr: {stderr}"
    );
    assert_eq!(fixtures::summary_field(&stdout, "resized"), "1");
    assert_eq!(fixtures::summary_field(&stdout, "skipped"), "1");
    assert_eq!(fixtures::names_in(&root.join("srnk")).len(), 1);
}

/// quickstart E.3 -- the output name is taken by a regular file
#[test]
fn output_name_taken_by_a_file() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_bytes(root, "srnk", b"i am not a folder");
    fixtures::write_jpeg(root, "photo.jpg", 200, 100);

    let assertion = fixtures::srnk(root)
        .args(["--factor", "50"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).to_string();

    assert!(
        stderr
            .contains("error: cannot create output folder: './srnk' exists and is not a directory"),
        "stderr: {stderr}"
    );
    assert_eq!(
        std::fs::read(root.join("srnk")).unwrap(),
        b"i am not a folder",
        "the existing file must not be overwritten"
    );
    assert!(
        !stderr.contains("photo.jpg"),
        "no image may be processed: {stderr}"
    );
}

/// quickstart E.4 -- read-only starting folder fails fast
#[test]
fn read_only_folder_fails_fast() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg(root, "photo.jpg", 200, 100);

    let mut permissions = std::fs::metadata(root).unwrap().permissions();
    permissions.set_readonly(true);
    if std::fs::set_permissions(root, permissions).is_err() {
        eprintln!("skipped: this OS cannot make the folder read-only");
        return;
    }
    let enforced = std::fs::create_dir(root.join("probe-dir")).is_err();
    if !enforced {
        let _ = std::fs::remove_dir(root.join("probe-dir"));
        restore_write_permission(root);
        eprintln!("skipped: this OS does not enforce read-only folders for creation");
        return;
    }

    let assertion = fixtures::srnk(root)
        .args(["--factor", "50"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).to_string();

    restore_write_permission(root);

    assert!(
        stderr.contains("error: cannot create output folder './srnk': "),
        "stderr: {stderr}"
    );
    assert!(
        !stderr.contains("photo.jpg"),
        "no image may be read: {stderr}"
    );
}

/// Make the fixture folder writable again so the temp dir can be cleaned up.
/// `set_readonly(false)` is deliberate here: it only has to undo the flag this test set.
#[allow(clippy::permissions_set_readonly_false)]
fn restore_write_permission(root: &std::path::Path) {
    if let Ok(metadata) = std::fs::metadata(root) {
        let mut permissions = metadata.permissions();
        permissions.set_readonly(false);
        let _ = std::fs::set_permissions(root, permissions);
    }
}

/// quickstart E.5 -- malformed metadata warns, never skips
#[test]
fn malformed_metadata_warns_not_skips() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg_with_corrupt_exif(root, "photo.jpg", 400, 300);

    let assertion = fixtures::srnk(root)
        .args(["--factor", "25"])
        .assert()
        .code(0);
    let output = assertion.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let copy = root.join("srnk").join("photo.jpg");
    assert_eq!(fixtures::dimensions(&copy), (200, 150));
    assert!(
        stderr.contains("warning: photo.jpg: metadata unreadable, copy written without metadata"),
        "stderr: {stderr}"
    );
    assert_eq!(fixtures::summary_field(&stdout, "resized"), "1");
    assert_eq!(fixtures::summary_field(&stdout, "skipped"), "0");
    assert_eq!(fixtures::summary_field(&stdout, "warnings"), "1");
}

/// quickstart E.6 -- a large tree streams through
#[test]
fn large_tree_streams() {
    let dir = fixtures::temp();
    let root = dir.path();
    let mut expected = 0usize;
    for folder in 0..21 {
        for index in 0..10 {
            fixtures::write_jpeg(root, &format!("dir{folder:02}/img{index:02}.jpg"), 24, 16);
            expected += 1;
        }
    }
    assert!(expected >= 200);

    let assertion = fixtures::srnk(root)
        .args(["--factor", "25", "--recursive"])
        .assert()
        .code(0);
    let stdout = String::from_utf8_lossy(&assertion.get_output().stdout).to_string();

    assert_eq!(
        fixtures::summary_field(&stdout, "resized"),
        expected.to_string()
    );
    let copy = root.join("srnk").join("dir00").join("img00.jpg");
    assert_eq!(fixtures::dimensions(&copy), (12, 8));
}
