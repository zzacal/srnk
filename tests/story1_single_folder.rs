//! User Story 1 -- shrink the JPGs in one folder.

#[path = "support/fixtures.rs"]
mod fixtures;

use std::path::Path;

/// quickstart 1.1 -- FR-005, FR-007, FR-008, FR-009
#[test]
fn resizes_every_jpg_in_folder() {
    let dir = fixtures::temp();
    let root = dir.path();
    let sources = [
        ("one.jpg", 400u32, 300u32),
        ("two.jpg", 200, 100),
        ("three.jpg", 100, 50),
    ];
    for (name, width, height) in sources {
        fixtures::write_jpeg(root, name, width, height);
    }
    let before: Vec<Vec<u8>> = sources
        .iter()
        .map(|(name, _, _)| std::fs::read(root.join(name)).unwrap())
        .collect();

    fixtures::srnk(root)
        .args(["--factor", "25"])
        .assert()
        .code(0);

    let out = root.join("srnk");
    assert_eq!(
        fixtures::names_in(&out),
        vec!["one.jpg", "three.jpg", "two.jpg"]
    );
    for (name, width, height) in sources {
        assert_eq!(
            fixtures::dimensions(&out.join(name)),
            (width / 2, height / 2),
            "{name} must be halved in each dimension"
        );
    }
    for ((name, _, _), original) in sources.iter().zip(before) {
        assert_eq!(
            std::fs::read(root.join(name)).unwrap(),
            original,
            "{name} must be untouched"
        );
    }
}

/// quickstart 1.2 -- FR-006
#[test]
fn ignores_non_jpg_files() {
    let dir = fixtures::temp();
    let root = dir.path();
    for name in ["a.jpg", "b.JPG", "c.jpeg", "d.JPEG"] {
        fixtures::write_jpeg(root, name, 200, 100);
    }
    fixtures::write_bytes(root, "e.png", &[0x89, b'P', b'N', b'G']);
    fixtures::write_bytes(root, "f.txt", b"not an image");

    fixtures::srnk(root)
        .args(["--factor", "50"])
        .assert()
        .code(0);

    assert_eq!(
        fixtures::names_in(&root.join("srnk")),
        vec!["a.jpg", "b.JPG", "c.jpeg", "d.JPEG"]
    );
}

/// quickstart 1.3 -- FR-005, FR-020
#[test]
fn reuses_existing_output_folder() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg(root, "photo.jpg", 400, 300);

    fixtures::srnk(root)
        .args(["--factor", "25"])
        .assert()
        .code(0);
    let copy = root.join("srnk").join("photo.jpg");
    assert_eq!(fixtures::dimensions(&copy), (200, 150));

    let assertion = fixtures::srnk(root)
        .args(["--factor", "50"])
        .assert()
        .code(0);
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).to_string();
    assert!(
        !stderr.contains("cannot create output folder"),
        "reusing srnk/ must not error: {stderr}"
    );
    assert_eq!(fixtures::dimensions(&copy), (283, 212));
    assert_eq!(fixtures::names_in(&root.join("srnk")), vec!["photo.jpg"]);
}

/// quickstart 1.4 -- FR-014, contracts/cli.md "No-input case"
#[test]
fn empty_folder_exits_zero() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_bytes(root, "notes.txt", b"nothing to resize");

    let assertion = fixtures::srnk(root)
        .args(["--factor", "50"])
        .assert()
        .code(0);
    let output = assertion.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(stderr.contains("no JPEG files found"), "stderr: {stderr}");
    assert!(stdout.contains("resized:   0"), "stdout: {stdout}");
    assert!(Path::new(&root.join("srnk")).is_dir(), "srnk/ must exist");
}
