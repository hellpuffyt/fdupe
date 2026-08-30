//! Direct `--delete` flow: build a manifest in memory from the freshly
//! computed duplicate groups and execute it immediately. This reuses the
//! exact same refusal and reporting logic as `--from-manifest`, so the two
//! code paths cannot drift apart.

use crate::manifest::{build_manifest, execute_manifest, ExecutionReport};
use crate::model::{DupeGroup, KeepStrategy};

/// Delete duplicates directly (or simulate with `dry_run`), keeping one
/// survivor per group according to `strategy`.
///
/// Skips the re-hash verification step that `--from-manifest` performs,
/// since the groups were computed moments ago in this same run.
#[must_use]
pub fn delete_duplicates(groups: &[DupeGroup], strategy: KeepStrategy, dry_run: bool) -> ExecutionReport {
    let manifest = build_manifest(groups, strategy);
    execute_manifest(&manifest, dry_run, true)
}
