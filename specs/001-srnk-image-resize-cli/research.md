# Phase 0 Research: srnk Image Resize CLI

**Feature**: `001-srnk-image-resize-cli` | **Date**: 2026-10-11

All items below resolve the unknowns raised in the Technical Context of `plan.md`.
No NEEDS CLARIFICATION items remain.

## R1. Implementation language and distribution

- **Decision**: Rust (edition 2021, MSRV 1.82), shipped as a prebuilt single-file
  binary per OS (Linux x86_64/aarch64, macOS arm64/x86_64, Windows x86_64), built in CI
  with `cargo build --release`. Local development install via `cargo install --path .`.
- **Rationale**: User-selected. A single static binary matches the tool's shape — a
  zero-config CLI a user drops on PATH and runs in a photo folder, with no runtime or
  interpreter to install. Rust also gives fast JPEG decode/encode and an easy
  "fail the whole run with a non-zero status" error model.
- **Alternatives considered**: Python + Pillow/piexif (fastest to write, but requires a
  Python runtime and pipx-style install — rejected because distribution is a stated
  priority); Go (single binary too, but weaker JPEG/EXIF crate ecosystem for
  segment-level metadata surgery); Node + sharp (fastest resizing, but a native libvips
  dependency and a Node runtime defeat the single-binary goal).

## R2. JPEG decode, resize, and encode

- **Decision**: `image` 0.25.10 for decode/resize/encode. Resize with
  `image::imageops::FilterType::Lanczos3`; encode with
  `image::codecs::jpeg::JpegEncoder::new_with_quality(w, 90)`.
- **Rationale**: `image` is the de facto Rust imaging crate (≈50M recent downloads),
  pure Rust (no C dependency, so cross-compiled static binaries stay simple), and
  exposes exactly the three operations needed. Lanczos3 is the best-quality downscale
  filter in the crate and the conventional choice for photo downsizing. Quality 90 is a
  fixed internal constant, not a CLI flag — the spec lists output quality settings as
  out of scope (Assumptions), but an encoder quality must still be chosen, and 90 is the
  standard "visually lossless" photo value.
- **Alternatives considered**: `fast_image_resize` (SIMD, materially faster) — rejected
  for v1 as premature optimisation; SC-007 (1,000 images, no manual intervention) is met
  comfortably by `image` alone, and the resize call site is isolated enough to swap later.
  `zune-jpeg`/`mozjpeg` — rejected (`mozjpeg` needs a C toolchain, breaking the
  single-binary cross-compile story).

## R3. Metadata preservation (FR-017) and stripping (FR-018)

- **Decision**: Two-stage pipeline. (1) Read the source bytes once into memory and parse
  the JPEG container with `img-parts` 0.4.0 (`img_parts::jpeg::Jpeg::from_bytes`) to lift
  the APP1/Exif, APP2/ICC, and APP13 segments. (2) Decode + resize + re-encode the pixels
  with `image`, which emits a metadata-free JPEG. (3) In preserve mode, rebuild the output
  JPEG with `img-parts`, re-inserting the captured segments, after patching the Exif blob;
  in `--strip` mode, write the `image` output as-is.
- **Exif patching**: before re-insertion, rewrite the Exif IFD values that the resize
  invalidates: `PixelXDimension` (0xA002) and `PixelYDimension` (0xA003) to the new size,
  and drop `ImageWidth`/`ImageLength`/thumbnail (IFD1) entries rather than leave them
  stale. `kamadak-exif` 0.6.1 parses the blob to locate the fields and verify the result;
  it is read-only, so the byte edits are applied in place over the captured APP1 payload
  (fixed-width SHORT/LONG values, so the segment length never changes).
- **Orientation**: the Exif `Orientation` tag (0x0112) is carried through unchanged and
  the pixels are NOT auto-rotated. Rotating would change which dimension the factor
  applies to and contradict FR-009's "aspect ratio preserved" expectation measured against
  the stored pixel dimensions. Preserving tag + pixels together keeps the copy rendering
  identically to the source in any viewer.
- **Rationale**: `image` 0.25 does not round-trip EXIF on encode, so container-level
  segment copying is the only way to satisfy FR-017. `img-parts` is purpose-built for
  this (its own keywords are exif/icc/jpeg), is pure Rust, and `no_std`-capable.
- **Malformed metadata (spec edge case)**: a parse failure from `img-parts` or
  `kamadak-exif` must NOT fail the file. The image is still resized and written, the
  metadata is dropped for that file only, and a warning is recorded on the run summary —
  a warning, not a skip.
- **Alternatives considered**: shelling out to `exiftool` (perfect fidelity, but an
  external runtime dependency the single-binary goal forbids); writing fresh EXIF with
  `little_exif` (less mature, and rebuilding tags loses maker-notes the user expects to
  keep); `rexiv2`/gexiv2 bindings (C dependency).

## R4. Directory scan, recursion, and output exclusion

- **Decision**: `walkdir` 2.5.0. Non-recursive runs use `.max_depth(1)`; recursive runs
  use the default unlimited depth. `follow_links(false)` (the default) satisfies FR-016.
  Prune the output directory with `filter_entry`, comparing the canonicalised entry path
  against the canonicalised `srnk` path so FR-011 holds even if a symlink or case variant
  points at it.
- **Rationale**: `walkdir` is the standard crate, yields entries lazily (satisfying the
  "does not need the entire file list in memory" edge case), and `filter_entry` prunes the
  whole `srnk` subtree without descending into it. Using the canonicalised path rather
  than the file name also avoids skipping a legitimately named `srnk` folder nested
  deeper in the tree being wrongly treated as output... which, per FR-011, is only the
  top-level one.
- **Deterministic order**: `.sort_by_file_name()` so run summaries and skip ordering are
  reproducible (SC-006).
- **Alternatives considered**: hand-rolled `std::fs::read_dir` recursion (more code, must
  reimplement symlink and depth handling); `ignore`/`globwalk` (gitignore semantics are
  irrelevant here and add weight).

## R5. CLI parsing and validation

- **Decision**: `clap` 4.5 with the `derive` feature. `--factor <1..=99>` as
  `#[arg(long, value_parser = clap::value_parser!(u8).range(1..=99))]`, required, no
  default. `--recursive` and `--strip` as `ArgAction::SetTrue` flags. No positional
  arguments are declared, so clap rejects any extra positional with a non-zero exit —
  satisfying FR-001a for free.
- **Rationale**: clap's range `value_parser` produces the exact FR-003 behaviour (reject
  missing/non-numeric/out-of-range before any filesystem work) and exits 2 with a message
  naming the valid range; FR-002's "whole number 1-99" maps directly onto `u8` + range.
  The error text is augmented via `.help` so SC-005 ("can tell what to fix from the error
  alone") is met.
- **Factor arithmetic**: `scale = (factor as f64 / 100.0).sqrt()`; each output dimension
  is `((dim as f64) * scale).round().max(1.0) as u32` — `f64` + `round()` half-away-from-zero
  is the single consistent rule required by FR-008a, and `.max(1)` is the clamp the
  "rounds to zero pixels" edge case demands.
- **Alternatives considered**: `argh`/`lexopt` (smaller, but hand-written range validation
  and weaker generated help); accepting a float factor (rejected — spec says whole number).

## R6. Exit codes and error taxonomy

- **Decision**: three-way exit contract — `0` scan completed with no file-level failure
  (including "no JPGs found"); `1` run-level failure (output folder could not be created,
  `srnk` exists as a regular file, or ≥1 file failed to process); `2` invalid arguments
  (clap's own code). Warnings never affect the exit code. `main` returns
  `std::process::ExitCode` rather than `Result`, so the summary always prints before exit.
- **Rationale**: FR-014 names exactly these three conditions. Keeping clap's native `2`
  for usage errors avoids fighting the framework while still being non-zero as required.
- **Fail-fast vs continue**: preflight failures (bad args, unusable output folder) abort
  before any file is touched (FR-003, spec read-only edge case). Per-file failures are
  collected and the run continues (FR-012), with the non-zero status applied at the end.
- **Disk-full**: writes go to a temporary file in the destination directory, then
  `fs::rename` over the final path — so a truncated image is never left behind
  (spec edge case), and the rename gives the unconditional overwrite FR-020 requires.

## R7. Case-insensitive name collisions

- **Decision**: maintain a per-destination-directory map keyed by the lowercased output
  file name. The first file to claim a key wins; a later source whose lowercased name
  collides is recorded as a skip with reason `name collides with <other file>
  (case-insensitive)`. Applied on every platform, not just case-insensitive ones, so
  behaviour and test results are identical everywhere.
- **Rationale**: The spec's edge case forbids silently dropping one of `a.JPG`/`a.jpg`.
  Detecting the collision in-process rather than relying on filesystem behaviour makes it
  deterministic and testable on Linux CI.
- **Alternatives considered**: renaming the loser (`a_1.jpg`) — rejected, FR-007 requires
  the source file name be preserved; probing the filesystem's case sensitivity — rejected
  as non-deterministic across mounted volumes.

## R8. Testing strategy

- **Decision**: `cargo test` only (no extra test framework). Unit tests inline in `src/`
  for the pure logic (factor→dimension arithmetic, collision keys, summary formatting,
  output-path mirroring). Integration tests in `tests/` that build a real fixture tree in
  a `tempfile::TempDir`, run the binary through `assert_cmd`, and assert on files,
  dimensions, metadata, stdout and exit codes. JPEG fixtures are generated at test time
  (encode a known gradient, inject a known EXIF blob) rather than committed as binaries,
  so the inputs are readable and diffable.
- **Dev-dependencies**: `tempfile` 3, `assert_cmd` 2, `predicates` 3.
- **Rationale**: the acceptance scenarios are nearly all end-to-end filesystem
  assertions, so integration tests carry most of the verification weight; the arithmetic
  (SC-003's 1% tolerance) is the one piece worth isolating as fast unit tests.
- **Alternatives considered**: `cargo-nextest` (nicer output, extra tool to install —
  optional, not required); committed binary fixtures (opaque in review).

## R9. Performance and progress reporting

- **Decision**: single-threaded, streaming, one image in memory at a time. Emit a
  progress line to stderr per processed file (`[n] path -> WxH`), keeping stdout for the
  final summary only. No progress bar (explicitly out of scope).
- **Rationale**: SC-001 (50 photos < 1 min) and SC-007 (1,000 photos unattended) are met
  single-threaded; parallelism is listed as out of scope. Splitting progress (stderr) from
  the summary (stdout) keeps the summary machine-readable-ish and pipeable.
- **Alternatives considered**: `rayon` parallel map — out of scope for v1, and would make
  progress output and the skip-ordering guarantee (SC-006) non-deterministic.
