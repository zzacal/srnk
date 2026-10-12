//! `srnk` -- shrink the JPEGs in the current folder into a `srnk/` copy folder.

mod cli;
mod geometry;
mod metadata;
mod output;
mod resize;
mod scan;
mod summary;

use std::fs;
use std::process::ExitCode;

use crate::metadata::MetadataOutcome;
use crate::resize::ResizeError;
use crate::scan::Candidate;
use crate::summary::{RunSummary, SkipReason};

const METADATA_WARNING: &str = "metadata unreadable, copy written without metadata";

/// Never returns `Result`: the summary always prints on any path that reaches the scan.
fn main() -> ExitCode {
    let config = cli::parse();

    let start = match output::StartingFolder::current() {
        Ok(start) => start,
        Err(err) => {
            eprintln!("error: cannot determine the current directory: {err}");
            return ExitCode::from(1);
        }
    };

    let folder = match output::OutputFolder::prepare(&start.root) {
        Ok(folder) => folder,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(1);
        }
    };

    let mut report = RunSummary::new(
        config.factor,
        config.metadata_mode(),
        config.recursive,
        folder.path.clone(),
    );

    let mut discovered = 0usize;

    for candidate in scan::scan(&start.root, &folder.path, config.recursive) {
        discovered += 1;
        let source = match candidate {
            Candidate::Skip(record) => {
                report.push_skip(record.rel_path, record.reason);
                continue;
            }
            Candidate::Image(source) => source,
        };

        let bytes = match fs::read(&source.abs_path) {
            Ok(bytes) => bytes,
            Err(_) => {
                report.push_skip(source.rel_path, SkipReason::Unreadable);
                continue;
            }
        };

        let resized = match resize::resize_jpeg(&bytes, config.factor) {
            Ok(resized) => resized,
            Err(ResizeError::Decode(detail)) => {
                report.push_skip(source.rel_path, SkipReason::DecodeFailed(detail));
                continue;
            }
            Err(ResizeError::Encode(detail)) => {
                report.push_skip(source.rel_path, SkipReason::EncodeFailed(detail));
                continue;
            }
        };

        let applied = metadata::apply(
            &bytes,
            resized.bytes,
            config.metadata_mode(),
            resized.dimensions,
        );
        if applied.outcome == MetadataOutcome::UnreadableDropped {
            report.push_warning(&source.rel_path, METADATA_WARNING);
        }

        let dest = folder.destination(&source.rel_path);
        match output::write_copy(&dest, &applied.bytes) {
            Ok(()) => {
                report.resized += 1;
                eprintln!(
                    "[{}/{}] {} -> {}x{}",
                    report.resized,
                    discovered,
                    source.rel_path.display(),
                    resized.dimensions.width,
                    resized.dimensions.height
                );
            }
            Err(err) => {
                report.push_skip(source.rel_path, SkipReason::WriteFailed(err.to_string()));
            }
        }
    }

    if discovered == 0 {
        eprintln!("no JPEG files found");
    }

    report.print();
    report.exit_code()
}
