//! Directory traversal and filtering.

use crate::model::FileEntry;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;
use walkdir::WalkDir;

/// Compiled exclude-glob set plus the other scan filters.
pub struct ScanOptions {
    pub min_size: u64,
    pub follow_symlinks: bool,
    pub excludes: GlobSet,
}

impl ScanOptions {
    /// Build a `ScanOptions`, compiling the exclude glob patterns.
    ///
    /// # Errors
    /// Returns an error if any glob pattern fails to compile.
    pub fn new(
        min_size: u64,
        follow_symlinks: bool,
        exclude_patterns: &[String],
    ) -> Result<Self, globset::Error> {
        let mut builder = GlobSetBuilder::new();
        for pattern in exclude_patterns {
            builder.add(Glob::new(pattern)?);
        }
        let excludes = builder.build()?;
        Ok(Self { min_size, follow_symlinks, excludes })
    }

    fn is_excluded(&self, path: &Path) -> bool {
        self.excludes.is_match(path)
    }
}

/// Walk every root path and return the files that pass the filters.
///
/// # Errors
/// Returns an error if a root path cannot be read at all.
pub fn collect_files(roots: &[std::path::PathBuf], opts: &ScanOptions) -> anyhow::Result<Vec<FileEntry>> {
    let mut out = Vec::new();
    for root in roots {
        let walker = WalkDir::new(root).follow_links(opts.follow_symlinks);
        for entry in walker {
            // Skip unreadable entries rather than aborting the whole scan.
            let Ok(entry) = entry else {
                continue;
            };

            if opts.is_excluded(entry.path()) {
                continue;
            }

            // Without follow_symlinks, walkdir reports symlinks as their own
            // file type rather than following them; skip those explicitly.
            if entry.file_type().is_symlink() && !opts.follow_symlinks {
                continue;
            }

            if !entry.file_type().is_file() {
                continue;
            }

            let Ok(meta) = entry.metadata() else {
                continue;
            };

            let size = meta.len();
            if size < opts.min_size {
                continue;
            }

            out.push(FileEntry { path: entry.path().to_path_buf(), size, modified: meta.modified().ok() });
        }
    }
    Ok(out)
}
