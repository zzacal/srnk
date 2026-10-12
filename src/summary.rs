//! Run reporting: skip/warning records, the stdout summary, and the exit-code rule.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Whether copies keep the source metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataMode {
    Preserve,
    Strip,
}

/// Why a source produced no output copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    Unreadable,
    DecodeFailed(String),
    EncodeFailed(String),
    WriteFailed(String),
    NameCollision { with: String },
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkipReason::Unreadable => write!(f, "unreadable"),
            SkipReason::DecodeFailed(detail) => write!(f, "decode failed: {detail}"),
            SkipReason::EncodeFailed(detail) => write!(f, "encode failed: {detail}"),
            SkipReason::WriteFailed(detail) => write!(f, "write failed: {detail}"),
            SkipReason::NameCollision { with } => {
                write!(f, "name collides with {with} (case-insensitive)")
            }
        }
    }
}

/// A source that produced no output but did not abort the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkipRecord {
    pub rel_path: PathBuf,
    pub reason: SkipReason,
}

/// A non-fatal problem: reported, counted, never affects the exit status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarningRecord {
    pub rel_path: PathBuf,
    pub message: String,
}

/// Accumulated during the run; rendered once to stdout at the end.
#[derive(Debug, Clone)]
pub struct RunSummary {
    pub factor: u8,
    pub metadata_mode: MetadataMode,
    pub recursive: bool,
    pub resized: usize,
    pub skipped: Vec<SkipRecord>,
    pub warnings: Vec<WarningRecord>,
    pub output_path: PathBuf,
}

impl RunSummary {
    pub fn new(
        factor: u8,
        metadata_mode: MetadataMode,
        recursive: bool,
        output_path: PathBuf,
    ) -> Self {
        Self {
            factor,
            metadata_mode,
            recursive,
            resized: 0,
            skipped: Vec::new(),
            warnings: Vec::new(),
            output_path,
        }
    }

    /// Record a skip and report it on stderr.
    pub fn push_skip(&mut self, rel_path: PathBuf, reason: SkipReason) {
        eprintln!("skip: {}: {}", rel_path.display(), reason);
        self.skipped.push(SkipRecord { rel_path, reason });
    }

    /// Record a warning and report it on stderr.
    pub fn push_warning(&mut self, rel_path: &Path, message: &str) {
        eprintln!("warning: {}: {}", rel_path.display(), message);
        self.warnings.push(WarningRecord {
            rel_path: rel_path.to_path_buf(),
            message: message.to_string(),
        });
    }

    /// The run summary, rendered to stdout exactly as `contracts/cli.md` specifies.
    pub fn print(&self) {
        print!("{}", self.render());
    }

    pub fn render(&self) -> String {
        let scale = crate::geometry::scale_for(self.factor);
        let metadata = match self.metadata_mode {
            MetadataMode::Preserve => "preserved",
            MetadataMode::Strip => "stripped",
        };
        let mode = if self.recursive {
            "recursive"
        } else {
            "current folder only"
        };
        format!(
            "srnk summary\n  \
             factor:    {factor}% of original pixel area (each dimension x {scale:.4})\n  \
             metadata:  {metadata}\n  \
             mode:      {mode}\n  \
             resized:   {resized}\n  \
             skipped:   {skipped}\n  \
             warnings:  {warnings}\n  \
             output:    {output}\n",
            factor = self.factor,
            scale = scale,
            metadata = metadata,
            mode = mode,
            resized = self.resized,
            skipped = self.skipped.len(),
            warnings = self.warnings.len(),
            output = self.output_path.display(),
        )
    }

    /// `0` when no file-level failure occurred (including zero JPEGs found), else `1`.
    /// Warnings never affect the status.
    pub fn exit_code(&self) -> ExitCode {
        if self.skipped.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary() -> RunSummary {
        RunSummary::new(
            40,
            MetadataMode::Preserve,
            false,
            PathBuf::from("/tmp/srnk"),
        )
    }

    #[test]
    fn renders_every_contract_field() {
        let mut s = summary();
        s.resized = 2;
        let rendered = s.render();
        let expected = "srnk summary\n\
            \x20 factor:    40% of original pixel area (each dimension x 0.6325)\n\
            \x20 metadata:  preserved\n\
            \x20 mode:      current folder only\n\
            \x20 resized:   2\n\
            \x20 skipped:   0\n\
            \x20 warnings:  0\n\
            \x20 output:    /tmp/srnk\n";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn exit_code_is_zero_without_skips_and_one_with() {
        let mut s = summary();
        assert_eq!(
            format!("{:?}", s.exit_code()),
            format!("{:?}", ExitCode::SUCCESS)
        );
        s.skipped.push(SkipRecord {
            rel_path: PathBuf::from("a.jpg"),
            reason: SkipReason::Unreadable,
        });
        assert_eq!(
            format!("{:?}", s.exit_code()),
            format!("{:?}", ExitCode::from(1))
        );
    }

    #[test]
    fn warnings_do_not_change_the_exit_code() {
        let mut s = summary();
        s.warnings.push(WarningRecord {
            rel_path: PathBuf::from("a.jpg"),
            message: "metadata unreadable, copy written without metadata".into(),
        });
        assert_eq!(
            format!("{:?}", s.exit_code()),
            format!("{:?}", ExitCode::SUCCESS)
        );
    }

    #[test]
    fn skip_reasons_render_as_the_contract_wording() {
        assert_eq!(SkipReason::Unreadable.to_string(), "unreadable");
        assert_eq!(
            SkipReason::DecodeFailed("bad".into()).to_string(),
            "decode failed: bad"
        );
        assert_eq!(
            SkipReason::EncodeFailed("bad".into()).to_string(),
            "encode failed: bad"
        );
        assert_eq!(
            SkipReason::WriteFailed("bad".into()).to_string(),
            "write failed: bad"
        );
        assert_eq!(
            SkipReason::NameCollision {
                with: "a.JPG".into()
            }
            .to_string(),
            "name collides with a.JPG (case-insensitive)"
        );
    }
}
