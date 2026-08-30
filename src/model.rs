//! Core data types shared across the crate.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

/// A single file discovered during the scan, with the metadata the cascade
/// needs before any bytes are read.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

/// A group of paths that are not duplicates at all: they are the *same*
/// file on disk, reached through a hard link or (optionally) a followed
/// symlink. Reported separately from content duplicates because deleting
/// one would not reclaim any space.
#[derive(Debug, Clone, Serialize)]
pub struct IdentityGroup {
    pub paths: Vec<PathBuf>,
    pub size: u64,
}

/// A group of paths whose full contents are byte-for-byte identical.
#[derive(Debug, Clone, Serialize)]
pub struct DupeGroup {
    pub size: u64,
    pub hash: String,
    pub paths: Vec<PathBuf>,
}

impl DupeGroup {
    /// Bytes that could be reclaimed by keeping exactly one copy.
    #[must_use]
    pub fn reclaimable_bytes(&self) -> u64 {
        self.size.saturating_mul(self.paths.len().saturating_sub(1) as u64)
    }
}

/// Counters describing how much work the cascade skipped. This is the
/// headline number: almost no file should ever reach `fully_hashed`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CascadeStats {
    pub files_scanned: usize,
    pub identity_groups: usize,
    pub identity_files_skipped: usize,
    pub unique_size_files: usize,
    pub size_candidate_files: usize,
    pub sample_hashed: usize,
    pub unique_sample_files: usize,
    pub fully_hashed: usize,
    pub duplicate_groups: usize,
    pub duplicate_files: usize,
    pub reclaimable_bytes: u64,
}

/// Full result of a scan: duplicate groups, identity groups, and stats.
#[derive(Debug, Clone, Serialize)]
pub struct ScanReport {
    pub duplicate_groups: Vec<DupeGroup>,
    pub identity_groups: Vec<IdentityGroup>,
    pub stats: CascadeStats,
}

/// Strategy for choosing which file in a duplicate group survives a delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum KeepStrategy {
    /// Keep whichever path sorts first lexicographically.
    First,
    /// Keep the file with the oldest modification time.
    Oldest,
    /// Keep the file with the newest modification time.
    Newest,
    /// Keep the file whose path is shortest (ties broken lexicographically).
    ShortestPath,
}
