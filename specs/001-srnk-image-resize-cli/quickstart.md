# Quickstart & Validation Guide: srnk Image Resize CLI

**Feature**: `001-srnk-image-resize-cli` | **Date**: 2026-10-11

How to build, run, and prove this feature works end to end. Interface details live in
[contracts/cli.md](./contracts/cli.md); entity and rule details in
[data-model.md](./data-model.md); technology choices in [research.md](./research.md).

## Prerequisites

- Rust 1.82+ with `cargo` on PATH (`rustup` is the easy path — not currently installed on
  this machine; `cargo --version` must succeed before any step below).
- A few real JPEG photos for manual validation (automated tests generate their own).
- No other runtime, library, or C toolchain is required.

## Build and install

```bash
cargo build                       # debug build -> target/debug/srnk
cargo build --release             # release build -> target/release/srnk
cargo install --path .            # put srnk on PATH for local use
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Release artifacts for distribution are the per-OS `target/release/srnk` binaries
(`srnk.exe` on Windows), built once per target triple.

## Automated test suite

```bash
cargo test                        # unit + integration
cargo test --test story1_single_folder
cargo test --test story2_recursive
cargo test --test story3_reporting
cargo test --test edge_cases
```

Integration tests build a fixture tree in a `tempfile::TempDir`, set it as the working
directory, invoke the built binary via `assert_cmd`, then assert on the resulting files,
their decoded dimensions, their metadata, the captured streams, and the exit code.
Fixtures are generated at test time (a known gradient plus a known EXIF blob), so no
binary images are committed.

## Validation scenarios

Each scenario below is both a manual check and the acceptance criterion for the named
automated test. The spec requirement it proves is listed for traceability.

### US1 — Shrink the JPGs in one folder (P1)

| # | Test name | Setup | Command | Expected | Proves |
|---|---|---|---|---|---|
| 1.1 | `resizes_every_jpg_in_folder` | 3 JPGs of known size, no `srnk/` | `srnk --factor 25` | `srnk/` holds 3 JPGs, each exactly half the source width and height; all 3 originals byte-identical to before; exit 0 | FR-005, FR-007, FR-008, FR-009 |
| 1.2 | `ignores_non_jpg_files` | JPGs plus `.png`, `.txt`, `.JPEG` | `srnk --factor 50` | Only the `.jpg`/`.jpeg` files (any case) appear in `srnk/`; the `.png` and `.txt` do not | FR-006 |
| 1.3 | `reuses_existing_output_folder` | Run 1.1, then change factor | `srnk --factor 50` | Run succeeds, no error about the existing folder, previous copies overwritten with the new dimensions | FR-005, FR-020 |
| 1.4 | `empty_folder_exits_zero` | Folder with no JPGs | `srnk --factor 50` | stderr says no JPEG files found, summary shows `resized: 0`, exit 0 | FR-014 |

Manual check for 1.1: `identify srnk/*.jpg` (ImageMagick) or any image viewer — a
2000×1500 source at `--factor 25` must come out 1000×750.

### US2 — Shrink a whole folder tree (P2)

| # | Test name | Setup | Command | Expected | Proves |
|---|---|---|---|---|---|
| 2.1 | `mirrors_subfolder_structure` | JPGs at root + in `a/` and `a/b/` | `srnk --factor 50 --recursive` | `srnk/` contains the root copies plus `srnk/a/` and `srnk/a/b/` holding their copies at matching relative paths | FR-004, FR-010 |
| 2.2 | `non_recursive_ignores_subfolders` | Same tree | `srnk --factor 50` | Only root JPGs copied; no subfolders created inside `srnk/` | FR-004 |
| 2.3 | `never_rescans_output_folder` | Run 2.1, then run it again | `srnk --factor 50 --recursive` | Resized count is identical to the first run; no `srnk/srnk` exists; no copy is re-shrunk (dimensions unchanged between runs) | FR-011, SC-006 |
| 2.4 | `no_empty_mirrored_folders` | Tree with a subfolder containing only `.txt` | `srnk --factor 50 --recursive` | No mirrored folder exists for that subfolder | FR-010 |
| 2.5 | `does_not_follow_folder_symlinks` | Subfolder symlink pointing at an ancestor (skip on platforms without symlink permission) | `srnk --factor 50 --recursive` | Run terminates, no infinite loop, link target not traversed | FR-016 |

### US3 — Understand and trust what happened (P3)

| # | Test name | Setup | Command | Expected | Proves |
|---|---|---|---|---|---|
| 3.1 | `rejects_invalid_factor` | Folder with JPGs | `srnk --factor 0`, `--factor 100`, `--factor abc`, and no `--factor` | Each exits 2, message names the 1–99 whole-number rule, **no `srnk/` folder created**, originals untouched | FR-002, FR-003, SC-005 |
| 3.2 | `rejects_positional_argument` | Any folder | `srnk --factor 50 ./photos` | Exits 2 with an unexpected-argument error; nothing written | FR-001a |
| 3.3 | `corrupt_file_is_skipped_run_continues` | 3 valid JPGs + 1 truncated `.jpg` | `srnk --factor 50` | 3 copies written, the corrupt file reported on stderr with a reason, summary shows `resized: 3` / `skipped: 1`, exit 1 | FR-012, SC-002 |
| 3.4 | `summary_reports_all_fields` | 2 JPGs | `srnk --factor 40` | stdout summary includes factor, `metadata: preserved`, resized count, skipped count, and the absolute output path | FR-013, FR-019 |
| 3.5 | `preserves_metadata_by_default` | JPG with known capture date, camera model, GPS, orientation | `srnk --factor 50` | Copy still carries all four tags; `PixelXDimension`/`PixelYDimension` equal the new size | FR-017 |
| 3.6 | `strip_removes_metadata` | Same source | `srnk --factor 50 --strip` | Copy has no EXIF/ICC segments; summary says `metadata: stripped` | FR-018, FR-019 |

Manual check for 3.5/3.6: `exiftool srnk/photo.jpg` — the default run lists DateTimeOriginal,
Model, GPS, Orientation and the corrected pixel dimensions; the `--strip` run lists
essentially nothing.

### Edge cases

| # | Test name | Setup | Expected | Proves |
|---|---|---|---|---|
| E.1 | `tiny_image_clamps_to_one_pixel` | 2×2 JPG, `--factor 1` | Output is 1×1, not an error | Edge case: rounding to zero |
| E.2 | `case_insensitive_name_collision_reported` | `shot.jpg` and `shot.JPG` in one folder | One copy written, the other reported as a skip naming the collision, exit 1 — neither silently dropped | Edge case: case collision |
| E.3 | `output_name_taken_by_a_file` | A regular file named `srnk` in the folder | Exits 1 with a conflict error, the file is not overwritten, no images processed | Edge case: name conflict |
| E.4 | `read_only_folder_fails_fast` | Folder with write permission removed (skip where the OS/CI cannot enforce it) | Exits 1 with a permission error before any image is read | Edge case: read-only |
| E.5 | `malformed_metadata_warns_not_skips` | JPG with a corrupt EXIF block but valid pixels | Copy is written (resized correctly), a warning is reported, the file counts as resized, exit 0 | Edge case: bad metadata |
| E.6 | `large_tree_streams` | 200+ generated JPGs across 20+ subfolders, recursive | Completes, count is accurate, memory stays flat (one image at a time) | SC-007, Edge case: deep trees |

### Numeric guarantee (unit tests, `geometry.rs`)

| Source | Factor | Expected output | Proves |
|---|---|---|---|
| 2000×1500 | 25 | 1000×750 | FR-008 (area quarter, dimensions halved) |
| 2000×1500 | 50 | 1414×1061 | FR-008 (√0.5 ≈ 0.7071), SC-003 (area within 1%) |
| 1000×1000 | 99 | 995×995 | FR-008, monotonic shrink |
| 3×1 | 1 | 1×1 | Clamp to minimum 1 px |
| any | any | identical on repeat calls | FR-008a determinism |

## Done when

- `cargo test` is green and every table row above has a passing named test.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass.
- A manual run of `srnk --factor 50 --recursive` on a real photo tree produces a correct
  mirrored `srnk/` folder, the summary matches
  [contracts/cli.md](./contracts/cli.md), and `exiftool` confirms the metadata mode.
