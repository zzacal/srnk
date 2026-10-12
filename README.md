# srnk

Shrink the JPEGs in the current folder into a `srnk/` folder of smaller copies.
Originals are never modified.

## Install

```bash
cargo install --path .
```

This puts the binary in Cargo's bin directory -- `~/.cargo/bin` on Linux/macOS,
`%USERPROFILE%\.cargo\bin` on Windows. If `srnk` is not found afterwards, that
directory is not on your `PATH`:

```bash
# bash / zsh -- add to ~/.bashrc or ~/.zshrc, then restart the shell
export PATH="$HOME/.cargo/bin:$PATH"
```

```powershell
# PowerShell -- persist for your user account
[Environment]::SetEnvironmentVariable(
  'Path',
  [Environment]::GetEnvironmentVariable('Path','User') + ";$env:USERPROFILE\.cargo\bin",
  'User')
```

A persistent `PATH` change only applies to newly started shells. To use `srnk` in the
shell you are already in, update that session as well:

```powershell
$env:Path += ";$env:USERPROFILE\.cargo\bin"   # PowerShell
```

Or build the single-file binary directly (no runtime and no C toolchain required):

```bash
cargo build --release     # target/release/srnk  (srnk.exe on Windows)
```

The release binary is self-contained, so you can also skip `PATH` entirely and run it by
path, or copy it to a directory you already have on `PATH`.

## Usage

```text
srnk --factor <1-99> [--recursive] [--strip]
srnk --help
srnk --version
```

`srnk` always operates on the current working directory. It takes no positional
arguments and no path option.

| Option | Meaning |
|---|---|
| `--factor <N>` | **Required.** Whole number 1-99: the percentage of the original pixel *area* to keep. Each dimension is scaled by `sqrt(N/100)`, so `--factor 25` halves the width and the height. |
| `--recursive` | Also process every subfolder beneath the current folder, mirroring the layout inside `srnk/`. |
| `--strip` | Remove all metadata from the copies. Without it, EXIF/ICC/IPTC are preserved and the stored pixel dimensions are corrected. |

## What it does

- Reads every `.jpg` / `.jpeg` file (case-insensitive) in the current folder, or anywhere
  beneath it with `--recursive`.
- Writes one resized JPEG per source into `./srnk/`, keeping the source file name.
  The folder is created if absent and reused if it already exists; existing copies are
  overwritten without prompting.
- On a recursive run the source subfolder layout is mirrored inside `srnk/`; subfolders
  that produced no copies are not created. The `srnk/` folder itself is never rescanned,
  and folder symlinks are never followed.
- Progress, warnings and skip notices go to stderr; the final run summary goes to stdout:

```text
srnk summary
  factor:    25% of original pixel area (each dimension x 0.5000)
  metadata:  preserved
  mode:      current folder only
  resized:   12
  skipped:   0
  warnings:  0
  output:    /home/you/photos/srnk
```

A file that cannot be read, decoded, encoded or written is reported as a skip and the run
continues. Unreadable metadata is a warning, not a skip: the copy is still written.

## Exit codes

| Code | Condition |
|---|---|
| `0` | The scan completed with no file-level failure -- including when no JPEGs were found. |
| `1` | Run-level failure: `./srnk` exists as a regular file, the folder could not be created, or at least one file failed to process. |
| `2` | Usage error: missing, non-numeric or out-of-range `--factor`, an unknown option, or any positional argument. Nothing is written to disk. |

Warnings never affect the exit code.

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The full interface contract lives in
[`specs/001-srnk-image-resize-cli/contracts/cli.md`](specs/001-srnk-image-resize-cli/contracts/cli.md).
