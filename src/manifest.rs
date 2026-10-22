//! Review manifest: a JSON file describing exactly what a delete run would
//! do, meant to be inspected by a human before anything is removed. A
//! later `--from-manifest` run executes precisely this plan (after
//! re-verifying each file still matches its recorded hash).

use crate::hashing::full_hash;
use crate::keep::choose_survivor;
use crate::model::{DupeGroup, KeepStrategy};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub size: u64,
    pub hash: String,
    pub keep: PathBuf,
    pub delete: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub keep_strategy: KeepStrategy,
    pub groups: Vec<ManifestEntry>,
}

/// Build a manifest from duplicate groups, choosing a survivor per group
/// with the given strategy. Every entry keeps exactly one path.
#[must_use]
pub fn build_manifest(groups: &[DupeGroup], strategy: KeepStrategy) -> Manifest {
    let entries = groups
        .iter()
        .map(|g| {
            let keep = choose_survivor(&g.paths, strategy);
            let delete = g.paths.iter().filter(|p| **p != keep).cloned().collect();
            ManifestEntry { size: g.size, hash: g.hash.clone(), keep, delete }
        })
        .collect();

    Manifest { version: MANIFEST_VERSION, keep_strategy: strategy, groups: entries }
}

/// Serialize a manifest to a JSON file.
///
/// # Errors
/// Returns an error if the file cannot be created or written.
pub fn write_manifest(manifest: &Manifest, path: &Path) -> io::Result<()> {
    let json =
        serde_json::to_string_pretty(manifest).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

/// Read and parse a manifest JSON file.
///
/// # Errors
/// Returns an error if the file cannot be read or does not parse as a
/// valid manifest.
pub fn read_manifest(path: &Path) -> io::Result<Manifest> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Outcome of executing (or dry-running) a manifest.
#[derive(Debug, Default, Clone, Serialize)]
pub struct ExecutionReport {
    pub deleted: Vec<PathBuf>,
    pub would_delete: Vec<PathBuf>,
    pub skipped: Vec<(PathBuf, String)>,
    pub refused_groups: Vec<String>,
    pub bytes_reclaimed: u64,
}

/// Execute (or, if `dry_run`, simulate) a manifest's deletions.
///
/// Every group is re-validated before anything in it is touched:
///
/// * the file that would be kept must not itself be scheduled for deletion
///   (by path *or* by file identity, so `./a` vs `a` or a hard link cannot
///   sneak the survivor into the delete list);
/// * the kept file must still exist and, unless `skip_verify` is set, still
///   hash to the group's recorded hash. If it is gone or changed, deleting
///   the other copies could destroy the last copy of that content, so the
///   whole group is refused;
/// * unless `skip_verify` is set, each file scheduled for deletion is
///   re-hashed and must still match the recorded hash, so a manifest
///   generated against a since-modified tree cannot delete the wrong bytes.
///
/// Even with `skip_verify`, the survivor's existence and identity checks
/// always run.
#[must_use]
pub fn execute_manifest(manifest: &Manifest, dry_run: bool, skip_verify: bool) -> ExecutionReport {
    let mut report = ExecutionReport::default();

    for entry in &manifest.groups {
        if entry.delete.is_empty() || entry.delete.contains(&entry.keep) {
            report
                .refused_groups
                .push(format!("refusing to delete every copy in group {} (size={})", entry.hash, entry.size));
            continue;
        }

        if let Some(problem) = survivor_problem(entry, skip_verify) {
            report.refused_groups.push(format!(
                "refusing group {} (size={}): kept copy {} {problem}",
                entry.hash,
                entry.size,
                entry.keep.display()
            ));
            for victim in &entry.delete {
                report.skipped.push((victim.clone(), format!("kept copy {problem}")));
            }
            continue;
        }

        let mut seen: Vec<&PathBuf> = Vec::new();
        for victim in &entry.delete {
            if seen.contains(&victim) {
                continue;
            }
            seen.push(victim);

            if fs::symlink_metadata(victim).is_err() {
                report.skipped.push((victim.clone(), "path no longer exists".to_string()));
                continue;
            }

            // A victim that is the very file being kept (another spelling of
            // the same path, or a hard link to it) must never be removed by
            // this path: for a different spelling it *is* the survivor.
            if same_file::is_same_file(victim, &entry.keep).unwrap_or(false) {
                report.skipped.push((victim.clone(), "same file as the kept copy".to_string()));
                continue;
            }

            if !skip_verify {
                match full_hash(victim) {
                    Ok(h) if h == entry.hash => {}
                    Ok(_) => {
                        report.skipped.push((
                            victim.clone(),
                            "content changed since manifest was generated".to_string(),
                        ));
                        continue;
                    }
                    Err(e) => {
                        report.skipped.push((victim.clone(), format!("could not verify: {e}")));
                        continue;
                    }
                }
            }

            if dry_run {
                report.would_delete.push(victim.clone());
                report.bytes_reclaimed += entry.size;
            } else {
                match fs::remove_file(victim) {
                    Ok(()) => {
                        report.deleted.push(victim.clone());
                        report.bytes_reclaimed += entry.size;
                    }
                    Err(e) => report.skipped.push((victim.clone(), format!("delete failed: {e}"))),
                }
            }
        }
    }

    report
}

/// Why the survivor of `entry` cannot be relied on, if it cannot.
fn survivor_problem(entry: &ManifestEntry, skip_verify: bool) -> Option<String> {
    match fs::metadata(&entry.keep) {
        Err(e) => return Some(format!("is not readable ({e})")),
        Ok(meta) if !meta.is_file() => return Some("is not a regular file".to_string()),
        Ok(_) => {}
    }
    if skip_verify {
        return None;
    }
    match full_hash(&entry.keep) {
        Ok(h) if h == entry.hash => None,
        Ok(_) => Some("changed since the manifest was generated".to_string()),
        Err(e) => Some(format!("could not be verified ({e})")),
    }
}
