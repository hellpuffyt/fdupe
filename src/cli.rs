//! Command-line interface definition.

use clap::Parser;
use fdupe::model::KeepStrategy;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "fdupe", version, about = "Find duplicate files by content, safely.")]
#[allow(clippy::struct_excessive_bools)] // each flag is an independent, orthogonal CLI switch
pub struct Cli {
    /// Directories (or files) to scan. Defaults to the current directory.
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Ignore files smaller than this many bytes.
    #[arg(long, default_value_t = 0)]
    pub min_size: u64,

    /// Glob pattern to exclude (matched against the full path). May be
    /// given multiple times.
    #[arg(long = "exclude")]
    pub exclude: Vec<String>,

    /// Follow symlinks while walking (off by default).
    #[arg(long, default_value_t = false)]
    pub follow_symlinks: bool,

    /// Number of bytes to sample from the head and tail of each file in
    /// stage 2 of the cascade.
    #[arg(long)]
    pub sample_size: Option<u64>,

    /// Emit the report as JSON instead of human-readable text.
    #[arg(long, default_value_t = false)]
    pub json: bool,

    /// Delete duplicate files directly, keeping one copy per group
    /// according to `--keep`. Requires `--keep`.
    #[arg(long, default_value_t = false)]
    pub delete: bool,

    /// Which copy to keep in each duplicate group when deleting.
    #[arg(long, value_enum)]
    pub keep: Option<KeepStrategy>,

    /// Show what would be deleted without deleting anything. Works with
    /// both `--delete` and `--from-manifest`.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Write a JSON review manifest describing the delete plan instead of
    /// deleting anything. Requires `--keep`.
    #[arg(long)]
    pub manifest: Option<PathBuf>,

    /// Execute a previously written review manifest. Combine with
    /// `--dry-run` to preview, or `--delete` to actually remove files.
    #[arg(long)]
    pub from_manifest: Option<PathBuf>,

    /// When executing a manifest, skip re-hashing each file to confirm it
    /// still matches the content recorded when the manifest was built.
    /// Not recommended.
    #[arg(long, default_value_t = false)]
    pub skip_verify: bool,
}
