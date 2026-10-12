//! The starting folder, the `srnk` output folder preflight, and destination writes.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// The folder the tool was invoked in. Never taken from an argument.
#[derive(Debug, Clone)]
pub struct StartingFolder {
    pub root: PathBuf,
}

impl StartingFolder {
    pub fn current() -> io::Result<Self> {
        let cwd = std::env::current_dir()?;
        let root = fs::canonicalize(&cwd).map(plain).unwrap_or(cwd);
        Ok(Self { root })
    }
}

/// `./srnk` -- created if absent, reused if present.
#[derive(Debug, Clone)]
pub struct OutputFolder {
    pub path: PathBuf,
    /// Whether this run created the folder (vs. reused an existing one) -- part of the
    /// `OutputFolder` contract in data-model.md.
    #[allow(dead_code)]
    pub created: bool,
}

/// A run-level failure detected before any image is read.
#[derive(Debug)]
pub enum PreflightError {
    NotADirectory,
    CreateFailed(io::Error),
}

impl fmt::Display for PreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreflightError::NotADirectory => write!(
                f,
                "error: cannot create output folder: './srnk' exists and is not a directory"
            ),
            PreflightError::CreateFailed(err) => {
                write!(f, "error: cannot create output folder './srnk': {err}")
            }
        }
    }
}

impl OutputFolder {
    /// The four preflight rules, all evaluated before any image is read.
    pub fn prepare(root: &Path) -> Result<Self, PreflightError> {
        let path = root.join("srnk");
        match fs::metadata(&path) {
            Ok(meta) if meta.is_dir() => Ok(Self {
                path: canonical(&path),
                created: false,
            }),
            Ok(_) => Err(PreflightError::NotADirectory),
            Err(err) if err.kind() == io::ErrorKind::NotFound => match fs::create_dir(&path) {
                Ok(()) => Ok(Self {
                    path: canonical(&path),
                    created: true,
                }),
                Err(err) => Err(PreflightError::CreateFailed(err)),
            },
            Err(err) => Err(PreflightError::CreateFailed(err)),
        }
    }

    /// `output/<relative source path>` -- flat for a flat run, mirrored for a recursive one.
    pub fn destination(&self, rel_path: &Path) -> PathBuf {
        self.path.join(rel_path)
    }
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write `bytes` to `dest`, creating the parent directory lazily and swapping the file
/// in with a rename so a partial write can never survive.
pub fn write_copy(dest: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp = parent.join(format!(".srnk-tmp-{}-{unique}", std::process::id()));
    if let Err(err) = fs::write(&temp, bytes) {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    if let Err(err) = fs::rename(&temp, dest) {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    Ok(())
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path)
        .map(plain)
        .unwrap_or_else(|_| path.to_path_buf())
}

/// Drop Windows' `\\?\` verbatim prefix so reported paths stay readable.
fn plain(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy().to_string();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if rest.len() > 2 && rest.as_bytes()[1] == b':' => PathBuf::from(rest),
        _ => path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_mirrors_relative_paths() {
        let folder = OutputFolder {
            path: PathBuf::from("/root/srnk"),
            created: true,
        };
        assert_eq!(
            folder.destination(Path::new("a.jpg")),
            PathBuf::from("/root/srnk/a.jpg")
        );
        assert_eq!(
            folder.destination(Path::new("a/b/c.jpg")),
            PathBuf::from("/root/srnk/a/b/c.jpg")
        );
    }

    #[test]
    fn preflight_messages_match_the_contract() {
        assert_eq!(
            PreflightError::NotADirectory.to_string(),
            "error: cannot create output folder: './srnk' exists and is not a directory"
        );
        let err =
            PreflightError::CreateFailed(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
        assert!(err
            .to_string()
            .starts_with("error: cannot create output folder './srnk': "));
    }
}
