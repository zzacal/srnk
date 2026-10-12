# Implementation Plan: srnk Image Resize CLI

**Branch**: `001-srnk-image-resize-cli` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-srnk-image-resize-cli/spec.md`

## Summary

`srnk` is a zero-config command-line tool that shrinks the JPEGs in the directory it is
invoked from. It takes a required `--factor 1..99` (the percentage of the original pixel
*area* to keep), an optional `--recursive` switch, and an optional `--strip` switch, and
writes resized copies into a single `srnk/` folder in the starting directory — mirroring
the source subfolder layout on recursive runs — leaving every original untouched.

Technical approach: a single Rust binary. `clap` validates arguments before any disk work;
`walkdir` streams the scan (depth-limited unless `--recursive`, never following symlinks,
pruning the `srnk` output subtree); `image` decodes, Lanczos3-resizes each dimension by
`sqrt(factor/100)`, and re-encodes JPEG; and `img-parts` + `kamadak-exif` copy the source's
EXIF/ICC segments onto the output (with the embedded pixel-dimension tags corrected) unless
`--strip` was passed. Per-file errors are collected as skips so the run continues; the run
always ends by printing a summary and exits 0 / 1 / 2 per a fixed exit-code contract.

## Technical Context

**Language/Version**: Rust 1.82+ (edition 2021), MSRV pinned in `Cargo.toml` via `rust-version`

**Primary Dependencies**: `clap` 4.5 (derive) — CLI parsing/validation; `walkdir` 2.5 —
directory scan; `image` 0.25 — JPEG decode/resize/encode; `img-parts` 0.4 — JPEG
container surgery for metadata; `kamadak-exif` 0.6 — EXIF parsing/verification.
Dev: `tempfile` 3, `assert_cmd` 2, `predicates` 3.

**Storage**: Local filesystem only. No database, no config file, no state between runs;
the `srnk/` output folder is treated as fully regenerable.

**Testing**: `cargo test` — inline unit tests for pure logic, `tests/` integration tests
that run the real binary against generated JPEG fixtures in a `tempfile::TempDir`.

**Target Platform**: Cross-platform CLI — Linux (x86_64, aarch64), macOS (arm64, x86_64),
Windows (x86_64). Distributed as a prebuilt single-file binary per OS/arch.

**Project Type**: Single-project CLI tool (binary crate with an internal library module tree).

**Performance Goals**: 50 photos in under 60s (SC-001); 1,000 JPEGs across 20+ subfolders
in one unattended recursive run (SC-007). Single-threaded, streaming — one decoded image
resident at a time regardless of tree size.

**Constraints**: No runtime dependency and no C toolchain (pure-Rust crates only, so the
static single-binary build stays trivial); originals strictly read-only (FR-009); output
dimensions deterministic for a given source+factor (FR-008a, SC-006); output images never
left truncated (write-temp-then-rename).

**Scale/Scope**: Three CLI options, one output folder, two metadata modes. ~1,200 LOC of
Rust plus tests. No network, no concurrency, no plugins.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Constitution status**: `.specify/memory/constitution.md` is the unmodified Spec Kit
template — every principle is still a `[PRINCIPLE_N_NAME]` / `[PRINCIPLE_N_DESCRIPTION]`
placeholder. **No project-specific gates are defined, so no gate can be evaluated and
none can be violated.** This is recorded, not silently skipped: if a constitution is
ratified before implementation starts, this plan must be re-checked against it.

As a good-faith substitute, the design was checked against the template's own example
principles:

| Example principle | Status | Note |
|---|---|---|
| Library-first | PASS | Core logic lives in library modules (`scan`, `resize`, `metadata`, `summary`) with the binary as a thin shell; each module is unit-testable without the CLI. |
| CLI interface (text in/out, stdout/stderr split) | PASS | Progress → stderr, summary → stdout, errors → stderr, exit codes 0/1/2. No JSON output mode (not required by the spec; noted as a future extension). |
| Test-first | PASS (planned) | Every acceptance scenario in the spec maps to a named integration test in `quickstart.md`; `tasks.md` is to order tests before implementation. |
| Integration testing | PASS | The feature *is* filesystem behaviour; integration tests on real fixture trees are the primary verification, per `research.md` R8. |
| Simplicity / YAGNI | PASS | No parallelism, no config file, no path argument, no dry-run, no output-name option — all explicitly out of scope in the spec and excluded here. |

**Result**: Gate passes with no violations. Complexity Tracking below is intentionally empty.

## Project Structure

### Documentation (this feature)

```text
specs/001-srnk-image-resize-cli/
├── plan.md              # This file (/speckit-plan command output)
├── spec.md              # Feature specification
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/
│   └── cli.md           # Phase 1 output — command-line contract
├── checklists/
│   └── requirements.md  # Pre-existing spec quality checklist
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
Cargo.toml                   # Crate manifest, MSRV, dependency pins
Cargo.lock                   # Committed (binary crate)
README.md                    # Install + usage
src/
├── main.rs                  # Thin shell: parse args, run, print summary, map to ExitCode
├── cli.rs                   # clap derive struct: --factor (1..=99), --recursive, --strip
├── scan.rs                  # walkdir traversal, depth limit, symlink + srnk-subtree pruning,
│                            #   JPEG extension filter, case-collision detection
├── geometry.rs              # factor -> scale -> output dimensions (round, clamp to >= 1)
├── resize.rs                # decode, Lanczos3 resize, JPEG re-encode
├── metadata.rs              # img-parts segment capture/re-insert, EXIF dimension patching, strip mode
├── output.rs                # srnk/ creation, mirrored destination paths, temp-write + rename
└── summary.rs               # counters, skip/warning records, summary rendering

tests/
├── story1_single_folder.rs  # US1 acceptance scenarios
├── story2_recursive.rs      # US2 acceptance scenarios
├── story3_reporting.rs      # US3 acceptance scenarios (errors, summary, --strip)
├── edge_cases.rs            # spec Edge Cases section
└── support/
    └── fixtures.rs          # generate JPEGs (+ known EXIF) into a TempDir
```

**Structure Decision**: Single-project layout. The tool is one binary with no frontend,
no service, and no second deployable, so the web/mobile options do not apply. The
`src/*.rs` modules above are a flat internal library (declared from `main.rs`) rather than
a separate crate — a workspace split would add manifest overhead with no consumer to
justify it. The one deliberate shape choice: `geometry.rs` and `summary.rs` hold pure
functions with no filesystem or image-crate contact, so the numeric guarantees (FR-008,
FR-008a, SC-003) and the report format (FR-013) are unit-testable in milliseconds.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

No violations. No entries.
