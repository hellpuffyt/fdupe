//! Selecting which path in a duplicate group survives a delete.

use crate::model::KeepStrategy;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Choose the surviving path from a non-empty group of duplicate paths.
///
/// # Panics
/// Panics if `paths` is empty; callers must only invoke this on real
/// duplicate groups, which always have at least two members.
#[must_use]
pub fn choose_survivor(paths: &[PathBuf], strategy: KeepStrategy) -> PathBuf {
    assert!(!paths.is_empty(), "choose_survivor requires a non-empty group");

    match strategy {
        KeepStrategy::First => {
            let mut sorted: Vec<&PathBuf> = paths.iter().collect();
            sorted.sort();
            (*sorted[0]).clone()
        }
        KeepStrategy::ShortestPath => {
            let mut sorted: Vec<&PathBuf> = paths.iter().collect();
            sorted.sort_by(|a, b| path_len(a).cmp(&path_len(b)).then_with(|| a.cmp(b)));
            (*sorted[0]).clone()
        }
        KeepStrategy::Oldest => pick_by_mtime(paths, true),
        KeepStrategy::Newest => pick_by_mtime(paths, false),
    }
}

fn path_len(p: &Path) -> usize {
    p.as_os_str().len()
}

fn mtime(p: &Path) -> SystemTime {
    fs::metadata(p).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH)
}

fn pick_by_mtime(paths: &[PathBuf], oldest: bool) -> PathBuf {
    let mut sorted: Vec<&PathBuf> = paths.iter().collect();
    sorted.sort_by(|a, b| {
        let ord = mtime(a).cmp(&mtime(b));
        let ord = if oldest { ord } else { ord.reverse() };
        ord.then_with(|| a.cmp(b))
    });
    (*sorted[0]).clone()
}
