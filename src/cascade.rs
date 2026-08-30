//! The three-stage cascade: group by size (free), then a head/tail sample
//! hash, then a full content hash. Each stage only runs on files that
//! survived the previous one, so most files never have their bytes read at
//! all, and almost none are fully hashed.
//!
//! Hashing within stages 2 and 3 runs on `rayon`'s global work-stealing
//! pool. Hashing is CPU-and-IO-bound, embarrassingly parallel across
//! independent files, and needs no coordination between tasks, so a
//! data-parallel `par_iter` over the candidate list is a better fit than
//! hand-rolled channels/threads: rayon sizes the pool to the available
//! cores automatically, which is exactly the "bounded worker pool" the
//! spec asks for, without extra plumbing.

use crate::hashing::{full_hash, sample_hash};
use crate::model::{CascadeStats, DupeGroup, FileEntry};
use rayon::prelude::*;
use std::collections::HashMap;

const DEFAULT_SAMPLE_SIZE: u64 = 4096;

/// Run the cascade over a set of files (already de-duplicated by identity)
/// and return the duplicate groups plus stats describing how much work
/// each stage eliminated.
#[must_use]
pub fn run_cascade(files: Vec<FileEntry>, sample_size: Option<u64>) -> (Vec<DupeGroup>, CascadeStats) {
    let sample_size = sample_size.unwrap_or(DEFAULT_SAMPLE_SIZE);
    let mut stats = CascadeStats { files_scanned: files.len(), ..CascadeStats::default() };

    // Stage 1: group by size. This costs nothing beyond the metadata we
    // already have from the walk.
    let mut by_size: HashMap<u64, Vec<FileEntry>> = HashMap::new();
    for f in files {
        by_size.entry(f.size).or_default().push(f);
    }

    let mut size_candidates: Vec<FileEntry> = Vec::new();
    for (_, group) in by_size {
        if group.len() < 2 {
            stats.unique_size_files += group.len();
        } else {
            stats.size_candidate_files += group.len();
            size_candidates.extend(group);
        }
    }

    if size_candidates.is_empty() {
        return (Vec::new(), stats);
    }

    // Stage 2: head/tail sample hash, computed in parallel.
    stats.sample_hashed = size_candidates.len();
    let sampled: Vec<(FileEntry, Option<String>)> = size_candidates
        .into_par_iter()
        .map(|f| {
            let h = sample_hash(&f.path, f.size, sample_size).ok();
            (f, h)
        })
        .collect();

    let mut by_sample: HashMap<(u64, String), Vec<FileEntry>> = HashMap::new();
    for (f, hash) in sampled {
        if let Some(hash) = hash {
            by_sample.entry((f.size, hash)).or_default().push(f);
        }
        // Files that failed to hash (e.g. vanished mid-scan, permission
        // denied) are silently dropped from consideration; they cannot be
        // proven to be duplicates.
    }

    let mut sample_candidates: Vec<FileEntry> = Vec::new();
    for (_, group) in by_sample {
        if group.len() < 2 {
            stats.unique_sample_files += group.len();
        } else {
            sample_candidates.extend(group);
        }
    }

    if sample_candidates.is_empty() {
        return (Vec::new(), stats);
    }

    // Stage 3: full content hash, only for files that share both size and
    // sample hash with at least one other file.
    stats.fully_hashed = sample_candidates.len();
    let fully_hashed: Vec<(FileEntry, Option<String>)> = sample_candidates
        .into_par_iter()
        .map(|f| {
            let h = full_hash(&f.path).ok();
            (f, h)
        })
        .collect();

    let mut by_full: HashMap<(u64, String), Vec<FileEntry>> = HashMap::new();
    for (f, hash) in fully_hashed {
        if let Some(hash) = hash {
            by_full.entry((f.size, hash)).or_default().push(f);
        }
    }

    let mut groups: Vec<DupeGroup> = Vec::new();
    for ((size, hash), group) in by_full {
        if group.len() > 1 {
            let mut paths: Vec<_> = group.into_iter().map(|f| f.path).collect();
            paths.sort();
            stats.duplicate_groups += 1;
            stats.duplicate_files += paths.len();
            let dg = DupeGroup { size, hash, paths };
            stats.reclaimable_bytes += dg.reclaimable_bytes();
            groups.push(dg);
        }
    }

    groups.sort_by(|a, b| b.reclaimable_bytes().cmp(&a.reclaimable_bytes()).then(a.hash.cmp(&b.hash)));

    (groups, stats)
}
