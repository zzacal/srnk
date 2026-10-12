# CLI Contract: `srnk`

**Feature**: `001-srnk-image-resize-cli` | **Date**: 2026-10-11

This is the tool's only external interface: a command invoked in a terminal. The contract
below is normative — integration tests assert against it, and any change here is a
breaking change.

## Synopsis

```text
srnk --factor <1-99> [--recursive] [--strip]
srnk --help
srnk --version
```

The tool operates on the **current working directory**. It accepts no positional
arguments and no path option.

## Options

| Option | Short | Type | Required | Default | Meaning |
|---|---|---|---|---|---|
| `--factor <N>` | — | integer 1–99 | **yes** | none | Percentage of the original pixel *area* to keep. Each dimension is scaled by `sqrt(N/100)`. |
| `--recursive` | — | flag | no | off | Also process every subfolder beneath the current directory. |
| `--strip` | — | flag | no | off | Remove all metadata from the resized copies. Without it, metadata is preserved. |
| `--help` | `-h` | flag | no | — | Print usage and exit 0. |
| `--version` | `-V` | flag | no | — | Print `srnk <semver>` and exit 0. |

Unknown options and any positional argument are usage errors.

## Inputs read

- Files directly in the current directory, or anywhere beneath it when `--recursive` is set,
  whose extension is `.jpg` or `.jpeg` (case-insensitive).
- Everything inside `./srnk/` is excluded from the scan.
- Folder symlinks are not followed.
- Source files are opened read-only and are never modified, moved, or deleted.

## Outputs written

- A folder `./srnk/` created in the current directory (reused if it already exists).
- One JPEG copy per successfully processed source, keeping the source's file name.
- On a recursive run, the source subfolder layout is mirrored inside `./srnk/`. Subfolders
  that produced no copies are not created.
- An existing destination file at the same path is overwritten without prompting.

## Streams

| Stream | Content |
|---|---|
| `stdout` | The final run summary only. |
| `stderr` | Per-file progress lines, warnings, skip notices, and error messages. |

### Progress line (stderr, one per processed file)

```text
[<n>/<total-so-far>] <relative source path> -> <width>x<height>
```

### Skip notice (stderr)

```text
skip: <relative source path>: <reason>
```

Reasons are one of: `unreadable`, `decode failed: <detail>`, `encode failed: <detail>`,
`write failed: <detail>`, `name collides with <other file name> (case-insensitive)`.

### Warning notice (stderr)

```text
warning: <relative source path>: <message>
```

e.g. `warning: IMG_0042.jpg: metadata unreadable, copy written without metadata`.

### Run summary (stdout, always printed once a run reaches the scan)

```text
srnk summary
  factor:    <N>% of original pixel area (each dimension x <scale, 4dp>)
  metadata:  preserved | stripped
  mode:      current folder only | recursive
  resized:   <count>
  skipped:   <count>
  warnings:  <count>
  output:    <absolute path to srnk folder>
```

When `skipped > 0`, the skipped files and their reasons have already been written to
stderr; the summary reports only the count. Every in-scope JPEG appears exactly once as
either resized or skipped.

### No-input case

With zero in-scope JPEGs, the tool prints `no JPEG files found` to stderr, prints the
summary with `resized: 0`, and exits 0. The `srnk` folder is still created.

## Exit codes

| Code | Condition |
|---|---|
| `0` | Scan completed with no file-level failures — including when no JPEGs were found. |
| `1` | Run-level failure: `./srnk` exists as a regular file; the folder could not be created; or at least one file failed to process. |
| `2` | Usage error: missing, non-numeric, or out-of-range `--factor`; unknown option; any positional argument. Nothing is written to disk. |

Warnings never affect the exit code.

## Error messages (normative wording)

| Situation | stderr message |
|---|---|
| `--factor` missing | `error: the following required arguments were not provided:` / `  --factor <N>` (clap) |
| `--factor` out of range or non-numeric | `error: invalid value '<v>' for '--factor <N>': --factor must be a whole number from 1 to 99 (the percentage of the original pixel area to keep)` |
| Positional argument given | `error: unexpected argument '<v>' found` + note that `srnk` always operates on the current directory and takes no path |
| `srnk` exists as a file | `error: cannot create output folder: './srnk' exists and is not a directory` |
| Output folder creation denied | `error: cannot create output folder './srnk': <os error>` |

Each message states the valid input or the blocking condition, so a user can correct the
invocation from the message alone (SC-005).

## Determinism guarantees

- The same source image and factor always produce identical output dimensions.
- Re-running the same command on the same folder produces the same `srnk` contents and
  never re-shrinks a previously generated copy.
- Files are processed in sorted order within each directory, so progress output and
  case-collision winners are stable across runs and platforms.

## Out of scope for this version

No path or output-location argument, no quality option, no dry-run, no JSON output mode,
no no-clobber/skip-existing mode, no formats other than JPEG, no parallelism, no
configuration file or environment variables.
