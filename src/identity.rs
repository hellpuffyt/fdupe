//! Detect paths that refer to the *same* underlying file (hard links, and
//! symlinks when `--follow-symlinks` is on) so they are never treated as
//! content duplicates.
//!
//! On Unix this is `(device, inode)`. On Windows we use the file index
//! reported by `GetFileInformationByHandle` (via the `same-file` crate),
//! which is the closest analogue; it is unavailable on some filesystems
//! (e.g. certain network shares), in which case those files simply fall
//! through to normal content comparison instead of being merged.

use crate::model::{FileEntry, IdentityGroup};
use same_file::Handle;
use std::collections::HashMap;

/// Partition files into identity groups (same underlying file) and the
/// remaining files that need content comparison. Only one representative
/// per identity group is passed on to the size/sample/hash cascade.
#[must_use]
pub fn split_by_identity(files: Vec<FileEntry>) -> (Vec<IdentityGroup>, Vec<FileEntry>) {
    let mut by_handle: HashMap<Handle, Vec<FileEntry>> = HashMap::new();
    let mut unresolved: Vec<FileEntry> = Vec::new();

    for file in files {
        match Handle::from_path(&file.path) {
            Ok(handle) => {
                by_handle.entry(handle).or_default().push(file);
            }
            Err(_) => unresolved.push(file),
        }
    }

    let mut identity_groups = Vec::new();
    let mut representatives = Vec::new();

    for group in by_handle.into_values() {
        if group.len() > 1 {
            let size = group[0].size;
            let paths = group.iter().map(|f| f.path.clone()).collect();
            identity_groups.push(IdentityGroup { paths, size });
            // Keep exactly one representative for the content cascade.
            if let Some(rep) = group.into_iter().next() {
                representatives.push(rep);
            }
        } else {
            representatives.extend(group);
        }
    }

    representatives.extend(unresolved);
    (identity_groups, representatives)
}
