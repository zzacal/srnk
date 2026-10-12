//! Lazy, deterministic discovery of the in-scope JPEGs.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::summary::{SkipReason, SkipRecord};

/// One JPEG accepted by the scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceImage {
    pub abs_path: PathBuf,
    pub rel_path: PathBuf,
    pub file_name: OsString,
}

/// What the scan yields: a usable source, or a source already known to be skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Candidate {
    Image(SourceImage),
    Skip(SkipRecord),
}

/// `.jpg` / `.jpeg`, compared case-insensitively. Everything else is ignored.
pub fn is_jpeg(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => {
            let ext = ext.to_ascii_lowercase();
            ext == "jpg" || ext == "jpeg"
        }
        None => false,
    }
}

type Claims = HashMap<(PathBuf, String), String>;

/// Claim `name` for `parent`. Returns the existing winner's name on a collision.
pub fn claim(claims: &mut Claims, parent: &Path, name: &str) -> Option<String> {
    match claims.entry((parent.to_path_buf(), name.to_lowercase())) {
        Entry::Occupied(existing) => Some(existing.get().clone()),
        Entry::Vacant(slot) => {
            slot.insert(name.to_string());
            None
        }
    }
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Stream the in-scope JPEGs beneath `root`, pruning the top-level `output` subtree.
/// Entries are yielded one at a time -- never collected into a full list.
pub fn scan(root: &Path, output: &Path, recursive: bool) -> impl Iterator<Item = Candidate> {
    let root = root.to_path_buf();
    let output_canon = canonical(output);

    let mut walker = WalkDir::new(&root)
        .min_depth(1)
        .follow_links(false)
        .sort_by_file_name();
    if !recursive {
        walker = walker.max_depth(1);
    }

    let entries = walker.into_iter().filter_entry(move |entry| {
        if !entry.file_type().is_dir() {
            return true;
        }
        // Compare canonicalised paths so the exclusion holds for case variants and links,
        // and so only the top-level `srnk` folder counts as output.
        canonical(entry.path()) != output_canon
    });

    let mut claims: Claims = HashMap::new();

    entries.filter_map(move |result| match result {
        Err(err) => {
            let path = err.path().map(Path::to_path_buf)?;
            if !is_jpeg(&path) {
                return None;
            }
            Some(Candidate::Skip(SkipRecord {
                rel_path: relative(&root, &path),
                reason: SkipReason::Unreadable,
            }))
        }
        Ok(entry) => {
            if !entry.file_type().is_file() || !is_jpeg(entry.path()) {
                return None;
            }
            let rel_path = relative(&root, entry.path());
            let file_name = entry.file_name().to_os_string();
            let name = file_name.to_string_lossy().to_string();
            let parent = rel_path
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_path_buf();
            if let Some(with) = claim(&mut claims, &parent, &name) {
                return Some(Candidate::Skip(SkipRecord {
                    rel_path,
                    reason: SkipReason::NameCollision { with },
                }));
            }
            Some(Candidate::Image(SourceImage {
                abs_path: entry.path().to_path_buf(),
                rel_path,
                file_name,
            }))
        }
    })
}

fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_match_is_case_insensitive_and_jpeg_only() {
        for name in ["a.jpg", "a.JPG", "a.jpeg", "a.JPEG", "a.JpEg"] {
            assert!(is_jpeg(Path::new(name)), "{name} should be in scope");
        }
        for name in ["a.png", "a.txt", "a", "a.jpg.txt", "jpg"] {
            assert!(!is_jpeg(Path::new(name)), "{name} should be ignored");
        }
    }

    #[test]
    fn first_claim_wins_and_later_case_variants_collide() {
        let mut claims = Claims::new();
        let here = Path::new("");
        assert_eq!(claim(&mut claims, here, "shot.jpg"), None);
        assert_eq!(
            claim(&mut claims, here, "shot.JPG"),
            Some("shot.jpg".to_string())
        );
        // A different destination directory is an independent namespace.
        assert_eq!(claim(&mut claims, Path::new("sub"), "shot.JPG"), None);
    }
}
