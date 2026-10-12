//! User Story 3 -- understand and trust what happened.

#[path = "support/fixtures.rs"]
mod fixtures;

const FACTOR_RULE: &str = "--factor must be a whole number from 1 to 99";

/// quickstart 3.1 -- FR-002, FR-003, SC-005
#[test]
fn rejects_invalid_factor() {
    for bad in ["0", "100", "abc"] {
        let dir = fixtures::temp();
        let root = dir.path();
        let source = fixtures::write_jpeg(root, "photo.jpg", 200, 100);
        let before = std::fs::read(&source).unwrap();

        let assertion = fixtures::srnk(root)
            .args(["--factor", bad])
            .assert()
            .code(2);
        let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).to_string();

        assert!(
            stderr.contains(FACTOR_RULE),
            "--factor {bad} stderr: {stderr}"
        );
        assert!(
            !root.join("srnk").exists(),
            "--factor {bad} must write nothing"
        );
        assert_eq!(std::fs::read(&source).unwrap(), before);
    }

    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg(root, "photo.jpg", 200, 100);
    let assertion = fixtures::srnk(root).assert().code(2);
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).to_string();
    assert!(
        stderr.contains("the following required arguments were not provided")
            && stderr.contains("--factor <N>"),
        "missing factor stderr: {stderr}"
    );
    assert!(!root.join("srnk").exists());
}

/// quickstart 3.2 -- FR-001a
#[test]
fn rejects_positional_argument() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg(root, "photo.jpg", 200, 100);

    let assertion = fixtures::srnk(root)
        .args(["--factor", "50", "./photos"])
        .assert()
        .code(2);
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).to_string();

    assert!(stderr.contains("unexpected argument"), "stderr: {stderr}");
    assert!(
        stderr.contains("takes no path argument"),
        "the error must explain the no-path rule: {stderr}"
    );
    assert!(!root.join("srnk").exists(), "nothing may be written");
}

/// quickstart 3.3 -- FR-012, SC-002
#[test]
fn corrupt_file_is_skipped_run_continues() {
    let dir = fixtures::temp();
    let root = dir.path();
    for name in ["a.jpg", "b.jpg", "c.jpg"] {
        fixtures::write_jpeg(root, name, 200, 100);
    }
    fixtures::write_truncated_jpeg(root, "broken.jpg");

    let assertion = fixtures::srnk(root)
        .args(["--factor", "50"])
        .assert()
        .code(1);
    let output = assertion.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(stderr.contains("skip: broken.jpg: "), "stderr: {stderr}");
    assert!(stdout.contains("resized:   3"), "stdout: {stdout}");
    assert!(stdout.contains("skipped:   1"), "stdout: {stdout}");
    assert_eq!(
        fixtures::names_in(&root.join("srnk")),
        vec!["a.jpg", "b.jpg", "c.jpg"]
    );
}

/// quickstart 3.4 -- FR-013, FR-019
#[test]
fn summary_reports_all_fields() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg(root, "a.jpg", 200, 100);
    fixtures::write_jpeg(root, "b.jpg", 200, 100);

    let assertion = fixtures::srnk(root)
        .args(["--factor", "40"])
        .assert()
        .code(0);
    let stdout = String::from_utf8_lossy(&assertion.get_output().stdout).to_string();

    assert!(stdout.starts_with("srnk summary\n"), "stdout: {stdout}");
    assert_eq!(
        fixtures::summary_field(&stdout, "factor"),
        "40% of original pixel area (each dimension x 0.6325)"
    );
    assert_eq!(fixtures::summary_field(&stdout, "metadata"), "preserved");
    assert_eq!(
        fixtures::summary_field(&stdout, "mode"),
        "current folder only"
    );
    assert_eq!(fixtures::summary_field(&stdout, "resized"), "2");
    assert_eq!(fixtures::summary_field(&stdout, "skipped"), "0");
    assert_eq!(fixtures::summary_field(&stdout, "warnings"), "0");

    let reported = std::path::PathBuf::from(fixtures::summary_field(&stdout, "output"));
    assert!(reported.is_absolute(), "output path must be absolute");
    assert!(reported.ends_with("srnk"));
    assert!(reported.is_dir());
}

/// quickstart 3.5 -- FR-017
#[test]
fn preserves_metadata_by_default() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg_with_exif(root, "photo.jpg", 400, 300);

    fixtures::srnk(root)
        .args(["--factor", "25"])
        .assert()
        .code(0);

    let copy = root.join("srnk").join("photo.jpg");
    assert_eq!(fixtures::dimensions(&copy), (200, 150));

    let meta = fixtures::exif_of(&copy).expect("copy must carry EXIF");
    let text = |tag: exif::Tag| {
        meta.get_field(tag, exif::In::PRIMARY)
            .or_else(|| meta.get_field(tag, exif::In::THUMBNAIL))
            .map(|field| field.display_value().to_string())
    };

    assert!(
        text(exif::Tag::DateTimeOriginal)
            .unwrap_or_default()
            .contains("2021-01-02"),
        "capture date must survive: {:?}",
        text(exif::Tag::DateTimeOriginal)
    );
    assert_eq!(text(exif::Tag::Model).as_deref(), Some("\"SRNK Camera\""));
    assert!(text(exif::Tag::GPSLatitude).is_some(), "GPS must survive");
    assert_eq!(
        meta.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
            .and_then(|f| f.value.get_uint(0)),
        Some(6),
        "orientation is carried through unchanged"
    );
    assert_eq!(
        meta.get_field(exif::Tag::PixelXDimension, exif::In::PRIMARY)
            .and_then(|f| f.value.get_uint(0)),
        Some(200)
    );
    assert_eq!(
        meta.get_field(exif::Tag::PixelYDimension, exif::In::PRIMARY)
            .and_then(|f| f.value.get_uint(0)),
        Some(150)
    );
    assert!(fixtures::has_icc(&copy), "ICC profile must survive");
}

/// quickstart 3.6 -- FR-018, FR-019
#[test]
fn strip_removes_metadata() {
    let dir = fixtures::temp();
    let root = dir.path();
    fixtures::write_jpeg_with_exif(root, "photo.jpg", 400, 300);

    let assertion = fixtures::srnk(root)
        .args(["--factor", "50", "--strip"])
        .assert()
        .code(0);
    let stdout = String::from_utf8_lossy(&assertion.get_output().stdout).to_string();

    let copy = root.join("srnk").join("photo.jpg");
    assert!(copy.is_file());
    assert!(!fixtures::has_exif(&copy), "no EXIF segment may remain");
    assert!(!fixtures::has_icc(&copy), "no ICC segment may remain");
    assert_eq!(fixtures::summary_field(&stdout, "metadata"), "stripped");
}
