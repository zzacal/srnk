# Phase 1 Data Model: srnk Image Resize CLI

**Feature**: `001-srnk-image-resize-cli` | **Date**: 2026-10-11

These are in-process values only — `srnk` persists no state between runs. Each entity maps
to a Rust type in the module named beside it.

## RunConfig — `cli.rs`

The validated invocation. Produced by `clap`; immutable for the whole run.

| Field | Type | Rules |
|---|---|---|
| `factor` | `u8` | Required. 1–99 inclusive, whole number. Rejected by the parser otherwise (FR-002, FR-003). |
| `recursive` | `bool` | Default `false`. `true` extends the scan below the starting folder (FR-004). |
| `strip` | `bool` | Default `false`. `true` removes all metadata from copies (FR-018). |

Derived, not stored as input:

| Derived | Type | Rule |
|---|---|---|
| `scale` | `f64` | `(factor as f64 / 100.0).sqrt()` (FR-008). Always `< 1.0` because factor ≤ 99 — the tool only ever shrinks. |
| `metadata_mode` | enum `Preserve` \| `Strip` | `Strip` iff `strip`; reported in the summary (FR-019). |

No positional arguments exist; any extra positional is a usage error (FR-001a).

## StartingFolder — `output.rs`

| Field | Type | Rules |
|---|---|---|
| `root` | `PathBuf` | `std::env::current_dir()` at startup, canonicalised. Never taken from an argument (FR-001, FR-001a). |

## OutputFolder — `output.rs`

| Field | Type | Rules |
|---|---|---|
| `path` | `PathBuf` | Always `root.join("srnk")`, canonicalised after creation. |
| `created` | `bool` | Whether this run created it (vs. reused an existing one). |

Preflight rules, all evaluated before any image is read:

1. Path absent → create it (FR-005).
2. Path is an existing directory → reuse, no error (FR-005).
3. Path exists and is **not** a directory → abort with a conflict error, overwrite nothing (edge case), exit 1.
4. Creation fails (permission, read-only volume) → abort with the OS error, process no files (edge case), exit 1.

## SourceImage — `scan.rs`

One per JPEG accepted by the scan. Yielded lazily; never collected into a full list.

| Field | Type | Rules |
|---|---|---|
| `abs_path` | `PathBuf` | Absolute path on disk. Opened read-only; never modified or deleted (FR-009). |
| `rel_path` | `PathBuf` | Path relative to `StartingFolder.root`; the identity of the image and the basis of the mirrored destination. |
| `file_name` | `OsString` | Preserved verbatim on output (FR-007). |

Selection rules:

- Extension is `jpg` or `jpeg`, compared case-insensitively; everything else is ignored (FR-006).
- Regular files only; directories and non-file entries are not sources.
- Depth: `max_depth(1)` unless `recursive` (FR-004).
- Folder symlinks are not followed (FR-016).
- Any entry inside `OutputFolder.path` is pruned before descent (FR-011).
- Traversal is sorted by file name for deterministic ordering (SC-006).

### Case-collision rule

Per destination directory, the key `file_name.to_lowercase()` may be claimed once. The
first source (in sorted order) claims it; a later source with the same key becomes a
`SkipRecord` with a collision reason rather than silently overwriting (edge case).

## Dimensions — `geometry.rs`

| Field | Type | Rules |
|---|---|---|
| `width` | `u32` | `> 0` |
| `height` | `u32` | `> 0` |

Transformation (`source: Dimensions, factor: u8) -> Dimensions`):

```text
scale = sqrt(factor / 100)
out.w = max(1, round(source.w * scale))
out.h = max(1, round(source.h * scale))
```

- `round` is half-away-from-zero on `f64`, the single consistent rule (FR-008a) — same
  input always yields the same output.
- `max(1, _)` clamps a dimension that would round to 0 (edge case).
- Aspect ratio is preserved up to integer rounding; output area is within 1% of the
  requested percentage for sources larger than 100×100 (SC-003).
- No orientation-aware swapping: the EXIF `Orientation` tag and the pixel buffer are both
  carried through unchanged (`research.md` R3).

## ResizedCopy — `resize.rs` / `output.rs`

The output artifact. One per successfully processed `SourceImage`.

| Field | Type | Rules |
|---|---|---|
| `dest_path` | `PathBuf` | `output.path.join(source.rel_path)` — identical for a flat run; mirrors subfolders on a recursive run (FR-010). |
| `dimensions` | `Dimensions` | Per the transformation above. |
| `format` | always JPEG | Same format as the source (FR-015). |
| `metadata` | `MetadataOutcome` | See below. |

Write rules:

- Parent directory is created lazily — only when a copy is actually about to be written,
  so subfolders that yielded nothing produce no empty mirror (FR-010).
- Bytes go to a temporary file in the destination directory, then `rename` onto
  `dest_path`: unconditional overwrite with no prompt (FR-020) and no possibility of a
  truncated file surviving a disk-full failure (edge case).

## MetadataOutcome — `metadata.rs`

| Variant | When | Effect |
|---|---|---|
| `Preserved` | default mode, source segments parsed successfully | EXIF/ICC/IPTC segments copied onto the copy; `PixelXDimension`/`PixelYDimension` patched to the new size; stale `ImageWidth`/`ImageLength` and the IFD1 thumbnail dropped (FR-017). |
| `Stripped` | `--strip` passed | Copy contains no metadata segments (FR-018). |
| `UnreadableDropped` | default mode, segment/EXIF parse failed | Image still resized and written; metadata omitted for this file; a `WarningRecord` is raised — a warning, **not** a skip (edge case). |

## SkipRecord — `summary.rs`

A source that produced no output but did not abort the run (FR-012).

| Field | Type | Notes |
|---|---|---|
| `rel_path` | `PathBuf` | The source that was skipped. |
| `reason` | `SkipReason` | `Unreadable` \| `DecodeFailed` \| `EncodeFailed` \| `WriteFailed` \| `NameCollision { with }` |

Every skip is printed with its reason and counted, so no in-scope file is ever silently
omitted (SC-002). Any skip makes the final exit status non-zero (FR-014).

## WarningRecord — `summary.rs`

| Field | Type | Notes |
|---|---|---|
| `rel_path` | `PathBuf` | File affected. |
| `message` | `String` | e.g. malformed EXIF dropped. |

Warnings are reported but never change the exit status.

## RunSummary — `summary.rs`

Accumulated during the run; rendered once to stdout at the end, including on failure paths
after preflight (FR-013).

| Field | Type | Notes |
|---|---|---|
| `factor` | `u8` | Factor applied. |
| `metadata_mode` | `Preserve` \| `Strip` | Reported verbatim (FR-019). |
| `recursive` | `bool` | Whether the tree was walked. |
| `resized` | `usize` | Count of copies written. |
| `skipped` | `Vec<SkipRecord>` | Count + reasons. |
| `warnings` | `Vec<WarningRecord>` | Count + messages. |
| `output_path` | `PathBuf` | Location of `srnk/` (FR-013). |

### Exit status derivation (FR-014)

| Condition | Status |
|---|---|
| Invalid arguments (missing / non-numeric / out-of-range factor, unknown positional) | `2`, nothing written |
| Output folder unusable (`srnk` is a file, creation denied) | `1`, no files processed |
| Scan completed, `skipped` empty — including zero JPEGs found | `0` |
| Scan completed, `skipped` non-empty | `1` |
| Warnings only | does not affect status |

## State transitions

Per source image:

```text
Discovered ─┬─> Claimed ─┬─> Decoded ─┬─> Resized ─┬─> Encoded ─┬─> Written      => resized++
            │            │            │            │            └─> WriteFailed  => skip
            │            │            │            └─> EncodeFailed               => skip
            │            │            └─> DecodeFailed                            => skip
            │            └─> Unreadable                                           => skip
            └─> NameCollision                                                     => skip
```

Metadata handling branches off `Encoded`: `Preserve` → patch + re-insert (or
`UnreadableDropped` + warning), `Strip` → write as encoded. A metadata failure never moves
the image into a skip state.

Per run:

```text
Args parsed -> Output folder ready -> Scan streaming -> Summary printed -> ExitCode
```
