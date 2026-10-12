# Feature Specification: srnk Image Resize CLI

**Feature Branch**: `001-srnk-image-resize-cli`

**Created**: 2026-10-11

**Status**: Draft

**Input**: User description: "a cli tool that i can invoke in a directory that 1 - creates a folder called `srnk`, then 2 - had taken a --factor (1-99) and a --recursive switch to determine the resize factor and if it should be recursive to sub folders, and 3 - goes through all the all the jpgs in that folder and resizes them by the factor and if --recursive switch was passed, it does this recursively starting from that folder"

## Clarifications

### Session 2026-10-11

- Q: In a recursive run, should all resized copies land in one `srnk` folder at the top that mirrors the subfolder layout, or should each subfolder get its own `srnk` folder? → A: Option A — a single `srnk` folder in the starting folder, with the source subfolder structure mirrored inside it.
- Q: Does `--factor 50` mean half the width and height, or something else? → A: Option B — the factor is the percentage of the original total pixel count (area) to keep, so each dimension is scaled by the square root of the factor (factor 50 → each dimension ≈70.7%).
- Q: Should resized copies keep the original photo metadata (date, camera, GPS) or be stripped? → A: Preserve metadata by default, and add an optional `--strip` switch that removes all metadata from the copies.
- Q: When a resized copy already exists from an earlier run, should the tool overwrite, skip, or refuse? → A: Option A — always overwrite, so the `srnk` folder always reflects the most recent run's settings.
- Q: Should the tool accept a folder path argument, or only operate on the current directory? → A: Option A — current working directory only; no path argument in this version.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Shrink the JPGs in one folder (Priority: P1)

A person has a folder of JPG photos that are too large to email or upload. They open a terminal in that folder, run the tool with a resize factor, and find a new `srnk` folder containing smaller copies of every JPG in that folder. The originals are untouched.

**Why this priority**: This is the core value of the tool and a complete, usable product on its own. Everything else is an extension of this single behaviour.

**Independent Test**: Place several JPGs in an empty folder, run the tool with a factor of 25, and confirm a `srnk` folder appears containing one resized copy per source JPG at half the original width and height (a quarter of the original pixel count), with all originals unchanged.

**Acceptance Scenarios**:

1. **Given** a folder containing three JPG files and no `srnk` folder, **When** the user runs the tool with factor 25, **Then** a `srnk` folder is created in that folder containing three JPGs, each half the original pixel width and height (a quarter of the original pixel count), and the three originals remain byte-identical.
2. **Given** a folder containing JPGs and other file types, **When** the user runs the tool, **Then** only the JPG files are resized into `srnk` and non-JPG files are ignored.
3. **Given** a folder where a `srnk` folder already exists from a previous run, **When** the user runs the tool again, **Then** the run succeeds, reusing the existing folder and overwriting the previously generated copies of the same names.
4. **Given** a folder with no JPG files at all, **When** the user runs the tool, **Then** the tool reports that no JPG files were found and exits successfully without failing.

---

### User Story 2 - Shrink a whole folder tree at once (Priority: P2)

The same person keeps photos organised in nested subfolders by year and event. They run the tool once at the top of the tree with the recursive switch and get a single `srnk` folder that mirrors the original folder structure, with resized copies in matching subfolders.

**Why this priority**: Significant convenience for real photo libraries, but the tool is already valuable without it.

**Independent Test**: Build a folder tree two to three levels deep with JPGs at several levels, run the tool with factor 50 and the recursive switch, and confirm the `srnk` folder reproduces the subfolder layout with a resized copy for every source JPG found anywhere in the tree.

**Acceptance Scenarios**:

1. **Given** a folder containing JPGs and two subfolders that also contain JPGs, **When** the user runs the tool recursively, **Then** the `srnk` folder contains resized copies of the top-level JPGs plus subfolders of the same names holding the resized copies from each subfolder.
2. **Given** the same folder tree, **When** the user runs the tool without the recursive switch, **Then** only the top-level JPGs are resized and no subfolders are created inside `srnk`.
3. **Given** a recursive run over a tree that already contains a `srnk` folder from a previous run, **When** the tool walks the tree, **Then** the output folder and its contents are excluded from the source scan, so resized copies are never re-resized.
4. **Given** a subfolder that contains no JPG files, **When** a recursive run completes, **Then** no empty mirrored subfolder is created for it inside `srnk`.

---

### User Story 3 - Understand and trust what happened (Priority: P3)

Before and after a run the person wants to know what the tool did: which factor was applied, how many images were processed, which files were skipped and why, and a clear error message when they pass an invalid factor.

**Why this priority**: Improves confidence and debuggability but does not change the resizing outcome.

**Independent Test**: Run the tool once with an invalid factor and once over a folder containing one corrupt JPG, and confirm the error message and the run summary each state exactly what happened.

**Acceptance Scenarios**:

1. **Given** a user runs the tool with a factor of 0, a factor of 100, or a non-numeric factor, **When** the command is parsed, **Then** the tool refuses to run, explains that the factor must be a whole number from 1 to 99, creates no `srnk` folder, and exits with a non-zero status.
2. **Given** a run that processes several files and encounters one unreadable or corrupt JPG, **When** the run completes, **Then** the corrupt file is reported as skipped with a reason, all other files are still processed, and the summary reports counts of processed and skipped files.
3. **Given** any successful run, **When** it finishes, **Then** the user sees a summary stating the factor applied, whether metadata was preserved or stripped, the number of images resized, the number skipped, and the location of the output folder.
4. **Given** a folder of photos carrying capture date, camera model, and GPS metadata, **When** the user runs the tool with `--strip`, **Then** the resized copies contain no metadata, while a run without `--strip` leaves that metadata intact in the copies.

---

### Edge Cases

- A JPG so small that the factor would round a dimension to zero pixels — the output dimension is clamped to a minimum of 1 pixel rather than failing.
- Two source files in the same folder whose names differ only in extension case on a case-insensitive file system — the tool must not silently drop one; the collision is reported as a skip with a reason.
- The starting folder is read-only or the `srnk` folder cannot be created — the tool fails immediately with a clear permission error and processes no files.
- Insufficient disk space mid-run — the failing file is reported and the run ends with a non-zero status rather than leaving a silently truncated image.
- A regular file named `srnk` already exists in the starting folder — the tool reports the conflict and exits without overwriting it.
- Folder symbolic links encountered during a recursive run — links are not followed, preventing infinite loops.
- A source image whose metadata is malformed or unreadable while the image data itself is fine — the image is still resized, and the metadata problem is reported as a warning rather than a skip.
- Very large or deep trees — the run reports progress and completes without needing the entire file list in memory at once.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The tool MUST be invocable from a command line in an arbitrary working directory and operate on that directory as the starting folder.
- **FR-001a**: The tool MUST NOT accept a folder path argument in this version; it MUST reject unrecognised positional arguments with a human-readable error and a non-zero exit status.
- **FR-002**: The tool MUST accept a `--factor` option whose value is a whole number from 1 to 99 inclusive, interpreted as the percentage of the original total pixel count (area) to keep.
- **FR-003**: The tool MUST reject a missing, non-numeric, or out-of-range factor with a human-readable error message and a non-zero exit status, making no changes on disk.
- **FR-004**: The tool MUST accept a `--recursive` switch that, when present, extends processing to every subfolder beneath the starting folder; when absent, only the starting folder's own files are processed.
- **FR-005**: The tool MUST create an output folder named `srnk` directly inside the starting folder and MUST reuse it without error if it already exists.
- **FR-006**: The tool MUST select source files by the `.jpg` and `.jpeg` extensions, matched case-insensitively, and MUST ignore all other files.
- **FR-007**: The tool MUST write a resized copy of each selected source image into the output folder, preserving the source file name.
- **FR-008**: The tool MUST scale both width and height by the square root of the factor (as a fraction of 100), so the output pixel count is the requested percentage of the original, preserving the original aspect ratio, with each output dimension at least 1 pixel.
- **FR-008a**: The tool MUST round each computed output dimension to the nearest whole pixel using a single consistent rule, so the same source image and factor always produce identical output dimensions.
- **FR-009**: The tool MUST leave every source file unmodified and MUST never delete a source file.
- **FR-010**: In a recursive run, the tool MUST mirror the source subfolder structure inside the output folder so each resized copy sits at the path matching its source, and MUST NOT create mirrored folders for subfolders that yielded no output.
- **FR-011**: The tool MUST exclude the `srnk` output folder and everything inside it from its source scan, so generated images are never used as inputs.
- **FR-012**: The tool MUST continue processing remaining files when an individual image cannot be read or written, recording the file and the reason as a skip.
- **FR-013**: The tool MUST print a completion summary reporting the factor applied, whether metadata was preserved or stripped, the count of images resized, the count skipped, and the output folder location.
- **FR-014**: The tool MUST exit with status zero when the scan completed with no file-level failures, including when no JPGs were found, and non-zero when arguments were invalid, the output folder could not be created, or at least one file failed to process.
- **FR-015**: The tool MUST write output images in the same JPEG format as their sources.
- **FR-016**: The tool MUST NOT follow folder symbolic links during a recursive scan.
- **FR-017**: The tool MUST preserve the source image's metadata (including capture date, camera details, GPS, and orientation) in each resized copy by default, correcting any embedded dimension values to match the new size.
- **FR-018**: The tool MUST accept an optional `--strip` switch that, when present, removes all metadata from the resized copies, producing minimal JPEGs; when absent, metadata is preserved per FR-017.
- **FR-019**: The tool MUST report in the run summary whether metadata was preserved or stripped, so the user can tell which output they produced.
- **FR-020**: The tool MUST overwrite any existing output file at the same destination path without prompting, so the output folder always reflects the most recent run's factor and metadata mode.

### Key Entities

- **Starting folder**: The directory the tool is invoked in; the root of the scan and the parent of the output folder.
- **Source image**: A JPEG file discovered by the scan, identified by its path relative to the starting folder; read-only to the tool.
- **Output folder (`srnk`)**: The destination directory created inside the starting folder; mirrors source subfolder structure for recursive runs.
- **Resized copy**: One output image per processed source image — same name and format, pixel count scaled to the factor percentage with aspect ratio preserved.
- **Run summary**: The per-run record of factor applied, metadata mode (preserved or stripped), counts of processed and skipped files, skip reasons, and output location.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A first-time user can shrink a folder of JPGs with a single command and no configuration file, in under one minute for a folder of 50 photos.
- **SC-002**: Every in-scope JPG is accounted for in the run summary as either resized or skipped-with-reason — 100% of in-scope files, with no silent omissions.
- **SC-003**: Output images measure the expected dimensions (each original dimension multiplied by the square root of the factor percentage, rounded, minimum 1 pixel) for 100% of successfully processed files, giving an output pixel count within 1% of the requested percentage for any image larger than 100x100 pixels.
- **SC-004**: Source files are unchanged after any run, verified by content comparison, in 100% of runs including failed and interrupted ones.
- **SC-005**: A user who passes an invalid factor can tell what to fix from the error message alone, without consulting documentation, in at least 9 of 10 usability trials.
- **SC-006**: Repeating the same command on the same folder produces the same output and never re-shrinks already generated images, across any number of consecutive runs.
- **SC-007**: A recursive run over a tree of 1,000 JPGs across at least 20 subfolders completes without manual intervention and reports an accurate total.

## Assumptions

- (Confirmed in Clarifications) `--factor` means "keep this percentage of the original pixel count": each dimension is scaled by the square root of the factor, so factor 25 halves width and height and factor 50 leaves each dimension at about 70.7%. The 1-99 range means the tool only ever shrinks, never enlarges or copies at full size.
- `--factor` is required, with no implicit default, so a user can never accidentally resize at an unintended ratio.
- (Confirmed in Clarifications) The starting folder is always the current working directory at invocation; no path argument or output-location option is part of this version.
- (Confirmed in Clarifications) A single `srnk` folder at the top of the run holds all output, mirroring subfolder structure for recursive runs, rather than creating a `srnk` folder inside every visited subfolder.
- "JPGs" covers both the `.jpg` and `.jpeg` extensions, case-insensitive, since users commonly have both.
- (Confirmed in Clarifications) Resized copies keep the source file name, and re-running unconditionally overwrites prior output of the same name; the output folder is treated as fully regenerable, with no skip or no-clobber mode in this version.
- Output quality settings and image formats other than JPEG are out of scope for this version; metadata handling is in scope and covered by FR-017 through FR-019.
- Parallel processing, progress bars, dry-run mode, and a configurable output folder name are out of scope for this version.
- The tool runs on a local file system where it has permission to create a folder in the starting directory and enough free space for the resized copies.
