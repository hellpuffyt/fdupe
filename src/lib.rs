//! Core library for `fdupe`: a duplicate-file finder built around a
//! three-stage cascade (size, then a head/tail sample hash, then a full
//! content hash) so that only files that are genuinely likely to be
//! duplicates ever have their full contents read.
#![forbid(unsafe_code)]

pub mod cascade;
pub mod delete;
pub mod hashing;
pub mod identity;
pub mod keep;
pub mod manifest;
pub mod model;
pub mod walk;

use model::ScanReport;
use std::path::PathBuf;

/// Run a full scan: walk the given roots, split off files that are the
/// same underlying file (hard links / followed symlinks), then run the
/// size/sample/hash cascade over what remains.
///
/// # Errors
/// Returns an error if the exclude globs fail to compile or a root cannot
/// be walked at all.
pub fn scan(
    roots: &[PathBuf],
    min_size: u64,
    follow_symlinks: bool,
    exclude_patterns: &[String],
    sample_size: Option<u64>,
) -> anyhow::Result<ScanReport> {
    let opts = walk::ScanOptions::new(min_size, follow_symlinks, exclude_patterns)?;
    let files = walk::collect_files(roots, &opts)?;

    let (identity_groups, candidates) = identity::split_by_identity(files);
    let identity_extra_files: usize = identity_groups.iter().map(|g| g.paths.len() - 1).sum();

    let (duplicate_groups, mut stats) = cascade::run_cascade(candidates, sample_size);
    stats.files_scanned += identity_extra_files;
    stats.identity_groups = identity_groups.len();
    stats.identity_files_skipped = identity_extra_files;

    Ok(ScanReport { duplicate_groups, identity_groups, stats })
}
