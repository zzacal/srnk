---

description: "Task list for srnk Image Resize CLI implementation"
---

# Tasks: srnk Image Resize CLI

**Input**: Design documents from `/specs/001-srnk-image-resize-cli/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli.md, quickstart.md

**Tests**: INCLUDED. The feature specification's acceptance scenarios and `quickstart.md` name
an explicit test per scenario, and plan.md's Constitution Check records "Test-first: PASS
(planned) — `tasks.md` is to order tests before implementation". Every test task below is
therefore written **before** the implementation tasks in its phase and must FAIL first.

**Organization**: Tasks are grouped by user story so each story can be implemented, tested,
and delivered independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Exact file paths are included in every task

## Path Conventions

Single-project Rust binary crate, per plan.md "Project Structure":

- Crate manifest: `Cargo.toml`, `Cargo.lock` at repository root (`D:\srnk`)
- Source: `src/main.rs`, `src/cli.rs`, `src/scan.rs`, `src/geometry.rs`, `src/resize.rs`,
  `src/metadata.rs`, `src/output.rs`, `src/summary.rs`
- Tests: `tests/story1_single_folder.rs`, `tests/story2_recursive.rs`,
  `tests/story3_reporting.rs`, `tests/edge_cases.rs`, `tests/support/fixtures.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the Rust crate, pin dependencies, and make the lint/format gates runnable.

- [X] T001 Create the binary crate at repository root: `Cargo.toml` with `name = "srnk"`, `version = "0.1.0"`, `edition = "2021"`, `rust-version = "1.82"`, and a placeholder `src/main.rs` containing `fn main() {}`; verify `cargo build` succeeds
- [X] T002 Add pinned runtime dependencies to the `[dependencies]` section of `Cargo.toml`: `clap = { version = "4.5", features = ["derive"] }`, `walkdir = "2.5"`, `image = "0.25"`, `img-parts = "0.4"`, `kamadak-exif = "0.6"` (per plan.md Primary Dependencies — pure-Rust crates only, no C toolchain)
- [X] T003 [P] Add pinned dev dependencies to the `[dev-dependencies]` section of `Cargo.toml`: `tempfile = "3"`, `assert_cmd = "2"`, `predicates = "3"` (per research.md R8)
- [X] T004 [P] Create `rustfmt.toml` (defaults, `edition = "2021"`) and add a `[lints.clippy]` section to `Cargo.toml` so `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are the project's lint gates (quickstart.md "Build and install")
- [X] T005 [P] Create `.gitignore` at repository root ignoring `/target`, and commit `Cargo.lock` (binary crate — plan.md "Cargo.lock Committed")

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The module skeleton, the validated CLI config, the pure geometry arithmetic, the
summary/exit-code types, the output-folder preflight, and the test fixture generator. Every
user story depends on these.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T006 Declare the internal module tree in `src/main.rs` (`mod cli; mod scan; mod geometry; mod resize; mod metadata; mod output; mod summary;`) and create empty compiling stub files `src/cli.rs`, `src/scan.rs`, `src/geometry.rs`, `src/resize.rs`, `src/metadata.rs`, `src/output.rs`, `src/summary.rs` (plan.md "Structure Decision": flat internal library declared from `main.rs`, no separate crate)
- [X] T007 [P] Implement the `RunConfig` clap derive struct in `src/cli.rs` with exactly three options and no positional arguments: `factor: u8` — `#[arg(long, value_parser = clap::value_parser!(u8).range(1..=99))]`, **required, no default**; `recursive: bool` — `#[arg(long, action = ArgAction::SetTrue)]`, default `false`; `strip: bool` — `#[arg(long, action = ArgAction::SetTrue)]`, default `false`. Add the derived accessors `scale(&self) -> f64` returning `(factor as f64 / 100.0).sqrt()` and `metadata_mode(&self) -> MetadataMode` returning `Strip` iff `strip` (data-model.md "RunConfig"; FR-002, FR-004, FR-018, FR-001a)
- [X] T008 [P] Implement `Dimensions { width: u32, height: u32 }` (both invariant `> 0`) and `pub fn resized(source: Dimensions, factor: u8) -> Dimensions` in `src/geometry.rs` using exactly `out = max(1, round(dim as f64 * sqrt(factor as f64 / 100.0)))` with `f64::round` (half-away-from-zero — the single consistent rule) and the `max(1, _)` clamp (data-model.md "Dimensions"; FR-008, FR-008a)
- [X] T009 [P] Add inline `#[cfg(test)]` unit tests to `src/geometry.rs` asserting the quickstart.md numeric table verbatim: 2000x1500 @25 -> 1000x750; 2000x1500 @50 -> 1414x1061; 1000x1000 @99 -> 995x995; 3x1 @1 -> 1x1; plus a determinism test that repeated calls on the same input are identical, and an area test that output pixel count is within 1% of the requested percentage for sources larger than 100x100 (FR-008, FR-008a, SC-003)
- [X] T010 Implement the report types in `src/summary.rs`: `MetadataMode { Preserve, Strip }`, `SkipReason { Unreadable, DecodeFailed(String), EncodeFailed(String), WriteFailed(String), NameCollision { with: String } }`, `SkipRecord { rel_path: PathBuf, reason: SkipReason }`, `WarningRecord { rel_path: PathBuf, message: String }`, and `RunSummary { factor: u8, metadata_mode: MetadataMode, recursive: bool, resized: usize, skipped: Vec<SkipRecord>, warnings: Vec<WarningRecord>, output_path: PathBuf }` with `pub fn exit_code(&self) -> ExitCode` returning `0` when `skipped` is empty (including zero JPEGs found) and `1` when `skipped` is non-empty; warnings never affect status (data-model.md "SkipRecord"/"RunSummary"/"Exit status derivation"; FR-012, FR-014)
- [X] T011 Implement `StartingFolder { root: PathBuf }` from `std::env::current_dir()` canonicalised, and `OutputFolder { path: PathBuf, created: bool }` as `root.join("srnk")` with the four preflight rules in `src/output.rs`, all evaluated before any image is read: absent -> create; existing directory -> reuse with no error; exists and is not a directory -> abort with `error: cannot create output folder: './srnk' exists and is not a directory` and exit 1; creation fails -> abort with `error: cannot create output folder './srnk': <os error>` and exit 1 (data-model.md "OutputFolder"; FR-001, FR-005; contracts/cli.md "Error messages")
- [X] T012 Implement the thin shell in `src/main.rs`: `fn main() -> std::process::ExitCode` that parses `RunConfig`, runs the output-folder preflight, runs the scan/process loop, then **always** prints the summary to stdout before returning `summary.exit_code()` — never returns `Result`, so the summary prints on every path that reaches the scan (research.md R6; FR-013, FR-014)
- [X] T013 [P] Create the fixture generator `tests/support/fixtures.rs`: helpers that build a `tempfile::TempDir`, write generated JPEGs of caller-specified pixel dimensions (a known gradient encoded with the `image` crate) at caller-specified relative paths, inject a known EXIF blob (DateTimeOriginal, Model, GPS, Orientation) into a fixture, and produce a truncated/corrupt `.jpg`. No binary images are committed (research.md R8)

**Checkpoint**: Crate builds, `--factor` validation and the geometry arithmetic are unit-tested, the output folder preflight and fixture tooling exist — user story work can begin.

---

## Phase 3: User Story 1 - Shrink the JPGs in one folder (Priority: P1) 🎯 MVP

**Goal**: Running `srnk --factor N` in a folder produces a `srnk/` folder holding one resized
copy of every `.jpg`/`.jpeg` in that folder, with the originals untouched.

**Independent Test**: Place several JPGs in an empty folder, run `srnk --factor 25`, and confirm
a `srnk` folder appears containing one resized copy per source JPG at half the original width and
height (a quarter of the original pixel count), with all originals byte-identical.

### Tests for User Story 1 ⚠️

> **NOTE: Write these tests FIRST and ensure they FAIL before implementation**

- [X] T014 [P] [US1] Integration test `resizes_every_jpg_in_folder` in `tests/story1_single_folder.rs`: 3 JPGs of known size and no `srnk/`, run `srnk --factor 25` via `assert_cmd` with the TempDir as cwd, assert `srnk/` holds 3 JPGs each exactly half the source width and height, all 3 originals byte-identical to before, exit 0 (quickstart 1.1; FR-005, FR-007, FR-008, FR-009)
- [X] T015 [P] [US1] Integration test `ignores_non_jpg_files` in `tests/story1_single_folder.rs`: a folder of `.jpg`, `.JPG`, `.jpeg`, `.JPEG`, `.png`, `.txt`; run `srnk --factor 50`; assert only the `.jpg`/`.jpeg` files (any case) appear in `srnk/` and the `.png`/`.txt` do not (quickstart 1.2; FR-006)
- [X] T016 [P] [US1] Integration test `reuses_existing_output_folder` in `tests/story1_single_folder.rs`: run `srnk --factor 25`, then `srnk --factor 50`; assert the second run succeeds with no error about the existing folder and the previous copies are overwritten with the new dimensions (quickstart 1.3; FR-005, FR-020)
- [X] T017 [P] [US1] Integration test `empty_folder_exits_zero` in `tests/story1_single_folder.rs`: folder with no JPGs, run `srnk --factor 50`; assert stderr contains `no JPEG files found`, stdout summary shows `resized:   0`, `srnk/` is still created, exit 0 (quickstart 1.4; contracts/cli.md "No-input case"; FR-014)

### Implementation for User Story 1

- [X] T018 [US1] Implement the flat scan in `src/scan.rs`: a lazy iterator of `SourceImage { abs_path: PathBuf, rel_path: PathBuf, file_name: OsString }` built on `walkdir::WalkDir` with `.max_depth(1)`, `.sort_by_file_name()`, regular files only, accepting an extension of `jpg` or `jpeg` compared **case-insensitively** and ignoring everything else. Entries are yielded one at a time and never collected into a full list (data-model.md "SourceImage"; FR-004, FR-006, SC-006)
- [X] T019 [US1] Implement `src/resize.rs`: decode the source bytes with `image`, resize to the `geometry::resized` dimensions using `image::imageops::FilterType::Lanczos3`, and re-encode JPEG with `image::codecs::jpeg::JpegEncoder::new_with_quality(w, 90)` (quality 90 is a fixed internal constant, not a CLI flag), returning the encoded bytes plus the decode/encode failure as distinct error variants mapping to `SkipReason::DecodeFailed` / `EncodeFailed` (research.md R2; FR-008, FR-015)
- [X] T020 [US1] Implement metadata **preserve** mode in `src/metadata.rs`: read the source bytes once, parse the container with `img_parts::jpeg::Jpeg::from_bytes` to lift the APP1/Exif, APP2/ICC and APP13/IPTC segments, patch the captured Exif payload in place (`PixelXDimension` 0xA002 and `PixelYDimension` 0xA003 set to the new size — fixed-width SHORT/LONG so the segment length never changes — and drop stale `ImageWidth`/`ImageLength` plus the IFD1 thumbnail), verify with `kamadak-exif`, then re-insert the segments onto the re-encoded output. The Exif `Orientation` tag (0x0112) is carried through unchanged and pixels are NOT auto-rotated (research.md R3; FR-017)
- [X] T021 [US1] Implement destination writing in `src/output.rs`: `dest_path = output.path.join(source.rel_path)`, create the parent directory **lazily** only when a copy is about to be written, then write the bytes to a temporary file in the destination directory and `fs::rename` it onto `dest_path` — unconditional overwrite with no prompt and no possibility of a truncated file surviving a disk-full failure; map a write failure to `SkipReason::WriteFailed` (data-model.md "ResizedCopy" write rules; FR-007, FR-010, FR-020)
- [X] T022 [US1] Implement `RunSummary` rendering to stdout in `src/summary.rs`, matching contracts/cli.md "Run summary" byte for byte: the header line `srnk summary` followed by the indented `factor:` (`<N>% of original pixel area (each dimension x <scale, 4dp>)`), `metadata:` (`preserved` | `stripped`), `mode:` (`current folder only` | `recursive`), `resized:`, `skipped:`, `warnings:` and `output:` (absolute path to the `srnk` folder) fields (FR-013, FR-019)
- [X] T023 [US1] Wire the pipeline in `src/main.rs`: for each `SourceImage` from the scan, resize (T019), apply preserve-mode metadata (T020), write via temp+rename (T021), increment `resized`; when the scan yielded zero in-scope JPEGs print `no JPEG files found` to stderr and still print the summary with `resized: 0` and exit 0 (FR-012, FR-014; contracts/cli.md "No-input case")

**Checkpoint**: `cargo test --test story1_single_folder` is green. `srnk --factor N` is a complete, usable tool for a single folder — this is the MVP.

---

## Phase 4: User Story 2 - Shrink a whole folder tree at once (Priority: P2)

**Goal**: `srnk --factor N --recursive` walks the whole tree beneath the current folder and
produces a single `srnk/` folder mirroring the source subfolder layout.

**Independent Test**: Build a folder tree two to three levels deep with JPGs at several levels,
run `srnk --factor 50 --recursive`, and confirm `srnk/` reproduces the subfolder layout with a
resized copy for every source JPG found anywhere in the tree.

### Tests for User Story 2 ⚠️

> **NOTE: Write these tests FIRST and ensure they FAIL before implementation**

- [X] T024 [P] [US2] Integration test `mirrors_subfolder_structure` in `tests/story2_recursive.rs`: JPGs at the root plus in `a/` and `a/b/`, run `srnk --factor 50 --recursive`; assert `srnk/` contains the root copies plus `srnk/a/` and `srnk/a/b/` holding their copies at matching relative paths (quickstart 2.1; FR-004, FR-010)
- [X] T025 [P] [US2] Integration test `non_recursive_ignores_subfolders` in `tests/story2_recursive.rs`: the same tree, run `srnk --factor 50` with no switch; assert only the root JPGs are copied and no subfolders exist inside `srnk/` (quickstart 2.2; FR-004)
- [X] T026 [P] [US2] Integration test `never_rescans_output_folder` in `tests/story2_recursive.rs`: run `srnk --factor 50 --recursive` twice; assert the resized count is identical across runs, no `srnk/srnk` exists, and no copy is re-shrunk (dimensions unchanged between runs) (quickstart 2.3; FR-011, SC-006)
- [X] T027 [P] [US2] Integration test `no_empty_mirrored_folders` in `tests/story2_recursive.rs`: a tree with a subfolder containing only `.txt`, run `srnk --factor 50 --recursive`; assert no mirrored folder exists for that subfolder inside `srnk/` (quickstart 2.4; FR-010)
- [X] T028 [P] [US2] Integration test `does_not_follow_folder_symlinks` in `tests/story2_recursive.rs`: a subfolder symlink pointing at an ancestor (gated to skip on platforms without symlink permission), run `srnk --factor 50 --recursive`; assert the run terminates with no infinite loop and the link target was not traversed (quickstart 2.5; FR-016)

### Implementation for User Story 2

- [X] T029 [US2] Extend `src/scan.rs` to honour `RunConfig.recursive`: unlimited `walkdir` depth when `true`, `.max_depth(1)` when `false`, keeping `.sort_by_file_name()` for deterministic ordering in both modes (research.md R4; FR-004, SC-006)
- [X] T030 [US2] Add output-subtree pruning to `src/scan.rs` using `WalkDir::filter_entry`, comparing the **canonicalised** entry path against the canonicalised top-level `srnk` path so the whole output subtree is pruned without descending into it, and so the exclusion holds even if a symlink or case variant points at it; only the top-level `srnk` folder is treated as output, not a legitimately named `srnk` folder nested deeper (research.md R4; FR-011)
- [X] T031 [US2] Set `follow_links(false)` explicitly on the `WalkDir` builder in `src/scan.rs` so folder symbolic links are never followed during a recursive scan, preventing infinite loops (FR-016)
- [X] T032 [US2] Verify mirrored destination paths end-to-end in `src/output.rs`/`src/main.rs`: `output.path.join(source.rel_path)` already reproduces nested paths, and because the parent directory is created lazily at write time (T021), subfolders that yielded no output produce no empty mirror — add the assertion-level wiring and a `mode: recursive` value into the summary (FR-010, FR-013)

**Checkpoint**: `cargo test --test story1_single_folder --test story2_recursive` green. US1 and US2 both work independently.

---

## Phase 5: User Story 3 - Understand and trust what happened (Priority: P3)

**Goal**: Every run explains itself — progress, skips with reasons, warnings, a full summary, the
`--strip` metadata mode, and normative error messages for invalid input.

**Independent Test**: Run the tool once with an invalid factor and once over a folder containing
one corrupt JPG, and confirm the error message and the run summary each state exactly what
happened.

### Tests for User Story 3 ⚠️

> **NOTE: Write these tests FIRST and ensure they FAIL before implementation**

- [X] T033 [P] [US3] Integration test `rejects_invalid_factor` in `tests/story3_reporting.rs`: run `srnk --factor 0`, `--factor 100`, `--factor abc`, and with no `--factor` at all; assert each exits 2, the stderr message names the whole-number 1-to-99 rule, **no `srnk/` folder is created**, and the originals are untouched (quickstart 3.1; FR-002, FR-003, SC-005)
- [X] T034 [P] [US3] Integration test `rejects_positional_argument` in `tests/story3_reporting.rs`: run `srnk --factor 50 ./photos`; assert exit 2 with an unexpected-argument error and nothing written to disk (quickstart 3.2; FR-001a)
- [X] T035 [P] [US3] Integration test `corrupt_file_is_skipped_run_continues` in `tests/story3_reporting.rs`: 3 valid JPGs plus 1 truncated `.jpg`, run `srnk --factor 50`; assert 3 copies written, the corrupt file reported on stderr as `skip: <path>: <reason>`, the summary shows `resized:   3` and `skipped:   1`, exit 1 (quickstart 3.3; FR-012, SC-002)
- [X] T036 [P] [US3] Integration test `summary_reports_all_fields` in `tests/story3_reporting.rs`: 2 JPGs, run `srnk --factor 40`; assert the stdout summary includes the factor with its 4-decimal scale, `metadata:  preserved`, `mode:      current folder only`, the resized count, the skipped count, the warnings count, and the absolute output path (quickstart 3.4; FR-013, FR-019)
- [X] T037 [P] [US3] Integration test `preserves_metadata_by_default` in `tests/story3_reporting.rs`: a JPG with known capture date, camera model, GPS and orientation, run `srnk --factor 50`; assert the copy still carries all four tags and that `PixelXDimension`/`PixelYDimension` equal the new size (quickstart 3.5; FR-017)
- [X] T038 [P] [US3] Integration test `strip_removes_metadata` in `tests/story3_reporting.rs`: the same source, run `srnk --factor 50 --strip`; assert the copy has no EXIF/ICC segments and the summary says `metadata:  stripped` (quickstart 3.6; FR-018, FR-019)

### Implementation for User Story 3

- [X] T039 [US3] Add the normative `--factor` error text in `src/cli.rs` via clap's value-parser/`help` customisation so an out-of-range or non-numeric value produces exactly `error: invalid value '<v>' for '--factor <N>': --factor must be a whole number from 1 to 99 (the percentage of the original pixel area to keep)`, and a missing factor produces clap's `error: the following required arguments were not provided:` / `  --factor <N>` (contracts/cli.md "Error messages"; FR-003, SC-005)
- [X] T040 [US3] Add the positional-argument error note in `src/cli.rs`: clap's `error: unexpected argument '<v>' found` augmented with a note that `srnk` always operates on the current directory and takes no path argument; exit code 2 with nothing written to disk (contracts/cli.md; FR-001a)
- [X] T041 [US3] Implement the per-file progress line on **stderr** in `src/main.rs`, formatted exactly `[<n>/<total-so-far>] <relative source path> -> <width>x<height>`, one per processed file, keeping stdout reserved for the final summary only (research.md R9; contracts/cli.md "Streams")
- [X] T042 [US3] Implement skip notices on stderr in `src/summary.rs`: `skip: <relative source path>: <reason>` where the reason renders as exactly one of `unreadable`, `decode failed: <detail>`, `encode failed: <detail>`, `write failed: <detail>`, `name collides with <other file name> (case-insensitive)`; each skip is also pushed onto `RunSummary.skipped` so it is counted (contracts/cli.md; FR-012, SC-002)
- [X] T043 [US3] Implement warning notices on stderr in `src/summary.rs`: `warning: <relative source path>: <message>`, pushed onto `RunSummary.warnings`, with warnings never affecting the exit code (contracts/cli.md; FR-019)
- [X] T044 [US3] Implement the `MetadataOutcome::UnreadableDropped` branch in `src/metadata.rs`: when `img-parts` or `kamadak-exif` fails to parse in preserve mode, still resize and write the image, omit metadata for that file only, and raise a `WarningRecord` (`metadata unreadable, copy written without metadata`) — a warning, **not** a skip, and the file still counts as resized (research.md R3; data-model.md "MetadataOutcome")
- [X] T045 [US3] Implement `MetadataOutcome::Stripped` in `src/metadata.rs`: when `RunConfig.strip` is set, write the `image`-encoded bytes as-is so the copy contains no metadata segments, and report `metadata: stripped` in the summary (FR-018, FR-019)
- [X] T046 [US3] Finalise the exit-code contract in `src/main.rs` and `src/summary.rs`: `0` when the scan completed with no file-level failure (including zero JPEGs found), `1` for a run-level failure (`./srnk` is a regular file, folder creation denied, or at least one file failed), `2` for clap usage errors; preflight failures abort before any file is touched while per-file failures are collected and the run continues (research.md R6; FR-014)

**Checkpoint**: All three user stories are independently functional and fully reported.

---

## Phase 6: Polish, Edge Cases & Cross-Cutting Concerns

**Purpose**: The spec's Edge Cases section, the collision rule that spans stories, and the
release/doc gates.

- [X] T047 [P] Integration test `tiny_image_clamps_to_one_pixel` in `tests/edge_cases.rs`: a 2x2 JPG with `--factor 1`; assert the output is 1x1 and not an error (quickstart E.1)
- [X] T048 [P] Integration test `case_insensitive_name_collision_reported` in `tests/edge_cases.rs`: `shot.jpg` and `shot.JPG` in one folder; assert one copy is written, the other is reported as a skip naming the collision, exit 1, and neither source is silently dropped (quickstart E.2)
- [X] T049 [P] Integration test `output_name_taken_by_a_file` in `tests/edge_cases.rs`: a regular file named `srnk` in the folder; assert exit 1 with the conflict error, the file is not overwritten, and no images are processed (quickstart E.3)
- [X] T050 [P] Integration test `read_only_folder_fails_fast` in `tests/edge_cases.rs`: a folder with write permission removed (gated to skip where the OS/CI cannot enforce it); assert exit 1 with a permission error before any image is read (quickstart E.4)
- [X] T051 [P] Integration test `malformed_metadata_warns_not_skips` in `tests/edge_cases.rs`: a JPG with a corrupt EXIF block but valid pixels; assert the copy is written at the correct size, a warning is reported, the file counts as resized, exit 0 (quickstart E.5)
- [X] T052 [P] Integration test `large_tree_streams` in `tests/edge_cases.rs`: 200+ generated JPGs across 20+ subfolders run recursively; assert completion, an accurate total, and that only one decoded image is resident at a time (quickstart E.6; SC-007)
- [X] T053 Implement the case-collision rule in `src/scan.rs`: a per-destination-directory map keyed by `file_name.to_lowercase()`; the first source in sorted order claims the key, and a later source with the same key becomes a `SkipRecord` with reason `NameCollision { with }` rather than silently overwriting. Applied on every platform, not just case-insensitive ones, so behaviour is identical everywhere (research.md R7; data-model.md "Case-collision rule")
- [X] T054 [P] Write `README.md` at repository root: install (`cargo install --path .`), the synopsis `srnk --factor <1-99> [--recursive] [--strip]`, the area-based meaning of `--factor`, the `srnk/` output behaviour, and the 0/1/2 exit codes — consistent with `contracts/cli.md`
- [X] T055 Run and fix `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` until both pass clean (quickstart "Done when")
- [X] T056 [P] Verify the release build and distribution shape: `cargo build --release` produces a single-file `target/release/srnk` (`srnk.exe` on Windows) with no runtime or C dependency (plan.md Target Platform; research.md R1)
- [X] T057 Run the full `cargo test` suite and confirm every row of every quickstart.md table maps to a passing named test, with no scenario unmapped (quickstart "Done when")
- [X] T058 Perform the quickstart.md manual validation: run `srnk --factor 50 --recursive` on a real photo tree, confirm the mirrored `srnk/` folder is correct, the stdout summary matches `contracts/cli.md` exactly, and `exiftool` confirms the metadata mode for both a default and a `--strip` run

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Phase 2 only. No dependency on US2 or US3
- **User Story 2 (Phase 4)**: Depends on Phase 2. Extends `src/scan.rs` and so touches a file US1 created — if US1 and US2 run in parallel, coordinate on `src/scan.rs`
- **User Story 3 (Phase 5)**: Depends on Phase 2. Adds reporting around the pipeline; independently testable (its invalid-argument and summary tests need no resize work beyond Phase 2 + US1)
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)**: Can start right after Phase 2. Self-contained MVP
- **US2 (P2)**: Can start right after Phase 2. Shares `src/scan.rs` with US1 — US1 first is the smoother sequence
- **US3 (P3)**: Can start right after Phase 2. Its error-path tests (T033, T034) pass with only Phase 2 + T039/T040; its skip and metadata tests assume the US1 pipeline exists

### Within Each User Story

- Tests are written and MUST FAIL before implementation
- Pure logic (`geometry`, `summary` types) before I/O
- Scan before resize before metadata before write
- Core implementation before wiring in `main.rs`
- Story complete and its `cargo test --test story<N>_*` green before moving to the next priority

### Parallel Opportunities

- T003, T004, T005 in Setup
- T007, T008, T009, T013 in Foundational (different files, no shared edits)
- All four US1 tests (T014-T017) together
- All five US2 tests (T024-T028) together
- All six US3 tests (T033-T038) together
- All six edge-case tests (T047-T052) together
- T054 and T056 in Polish
- With three developers: after Phase 2, one takes US1, one takes US2's tests + traversal work, one takes US3's reporting work

---

## Parallel Example: User Story 1

```bash
# Launch all four US1 integration tests together (they must FAIL first):
Task: "Integration test resizes_every_jpg_in_folder in tests/story1_single_folder.rs"
Task: "Integration test ignores_non_jpg_files in tests/story1_single_folder.rs"
Task: "Integration test reuses_existing_output_folder in tests/story1_single_folder.rs"
Task: "Integration test empty_folder_exits_zero in tests/story1_single_folder.rs"

# Then the pipeline modules, in dependency order (scan -> resize -> metadata -> output):
Task: "Implement flat scan in src/scan.rs"
Task: "Implement decode/Lanczos3/encode in src/resize.rs"
```

## Parallel Example: Foundational

```bash
# These four touch different files and can run fully in parallel:
Task: "Implement RunConfig clap derive in src/cli.rs"
Task: "Implement Dimensions + resized() in src/geometry.rs"
Task: "Add geometry unit tests to src/geometry.rs"
Task: "Create fixture generator in tests/support/fixtures.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T005)
2. Complete Phase 2: Foundational (T006-T013) — CRITICAL, blocks all stories
3. Complete Phase 3: User Story 1 (T014-T023)
4. **STOP and VALIDATE**: `cargo test --test story1_single_folder`, then run `srnk --factor 25` in a real folder of photos
5. Ship — a single-folder JPEG shrinker is a complete product

### Incremental Delivery

1. Setup + Foundational -> crate builds, arithmetic proven by unit tests
2. Add US1 -> test independently -> **MVP**: shrink one folder
3. Add US2 -> test independently -> shrink a whole tree with a mirrored output folder
4. Add US3 -> test independently -> full reporting, `--strip`, normative errors and exit codes
5. Polish -> edge cases, collision rule, README, lint gates, release binary

### Parallel Team Strategy

With three developers after Phase 2 completes:

- Developer A: US1 (`scan` flat mode, `resize`, `metadata` preserve, `output` write)
- Developer B: US2 (recursion, pruning, symlinks, mirroring) — coordinates with A on `src/scan.rs`
- Developer C: US3 (clap error wording, progress/skip/warning streams, `--strip`, exit codes)

Then one developer sweeps Phase 6.

---

## Notes

- Tests are included because the spec's acceptance scenarios and quickstart.md name one test
  per scenario, and plan.md records test-first as the planned ordering. Verify each test FAILS
  before writing its implementation
- `[P]` tasks touch different files and have no dependency on an incomplete task
- `[Story]` labels (US1/US2/US3) map tasks to spec.md user stories for traceability; Setup,
  Foundational and Polish tasks carry no story label by design
- Field-level constraints from data-model.md (the 1-99 factor range, the `max(1, round(...))`
  rule, the exit-code table, the normative stderr/stdout wording) are quoted verbatim in the
  task descriptions, not left to implementation-time discretion
- `.specify/memory/constitution.md` is still the unmodified Spec Kit template, so no
  project-specific gate was evaluated. If a constitution is ratified before implementation
  starts, re-check plan.md and this task list against it
- Commit after each task or logical group; stop at any checkpoint to validate a story
  independently
