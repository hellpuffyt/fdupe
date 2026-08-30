#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;

use common::{new_tempdir, write_file};
use fdupe::delete::delete_duplicates;
use fdupe::manifest::{
    build_manifest, execute_manifest, read_manifest, write_manifest, Manifest, ManifestEntry,
};
use fdupe::model::KeepStrategy;
use fdupe::scan;

fn scan_dir(root: &std::path::Path) -> fdupe::model::ScanReport {
    scan(&[root.to_path_buf()], 0, false, &[], None).expect("scan should succeed")
}

#[test]
fn build_manifest_keeps_exactly_one_file_per_group() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"manifest build content");
    write_file(dir.path(), "b.txt", b"manifest build content");
    write_file(dir.path(), "c.txt", b"manifest build content");

    let report = scan_dir(dir.path());
    let manifest = build_manifest(&report.duplicate_groups, KeepStrategy::First);

    assert_eq!(manifest.groups.len(), 1);
    let entry = &manifest.groups[0];
    assert_eq!(entry.delete.len(), 2);
    assert!(!entry.delete.contains(&entry.keep));
}

#[test]
fn manifest_round_trips_through_json() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"round trip json content");
    write_file(dir.path(), "b.txt", b"round trip json content");

    let report = scan_dir(dir.path());
    let manifest = build_manifest(&report.duplicate_groups, KeepStrategy::First);

    let manifest_path = dir.path().join("manifest.json");
    write_manifest(&manifest, &manifest_path).expect("write manifest");

    let loaded = read_manifest(&manifest_path).expect("read manifest");
    assert_eq!(loaded.groups.len(), manifest.groups.len());
    assert_eq!(loaded.groups[0].keep, manifest.groups[0].keep);
    assert_eq!(loaded.groups[0].delete, manifest.groups[0].delete);
}

#[test]
fn execute_manifest_dry_run_deletes_nothing() {
    let dir = new_tempdir();
    let a = write_file(dir.path(), "a.txt", b"dry run manifest content");
    let b = write_file(dir.path(), "b.txt", b"dry run manifest content");

    let report = scan_dir(dir.path());
    let manifest = build_manifest(&report.duplicate_groups, KeepStrategy::First);

    let exec = execute_manifest(&manifest, true, false);
    assert!(exec.deleted.is_empty());
    assert_eq!(exec.would_delete.len(), 1);
    assert!(a.exists());
    assert!(b.exists());
}

#[test]
fn execute_manifest_actually_deletes_and_keeps_one() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"real deletion manifest content");
    write_file(dir.path(), "b.txt", b"real deletion manifest content");

    let report = scan_dir(dir.path());
    let manifest = build_manifest(&report.duplicate_groups, KeepStrategy::First);
    let keep_path = manifest.groups[0].keep.clone();

    let exec = execute_manifest(&manifest, false, false);
    assert_eq!(exec.deleted.len(), 1);
    assert!(keep_path.exists(), "the kept file must survive");
    assert!(exec.bytes_reclaimed > 0);
}

#[test]
fn execute_manifest_refuses_when_keep_is_also_in_delete_list() {
    let bogus = ManifestEntry {
        size: 10,
        hash: "deadbeef".to_string(),
        keep: "/tmp/does-not-matter-a.txt".into(),
        delete: vec!["/tmp/does-not-matter-a.txt".into(), "/tmp/does-not-matter-b.txt".into()],
    };
    let manifest = Manifest { version: 1, keep_strategy: KeepStrategy::First, groups: vec![bogus] };

    let exec = execute_manifest(&manifest, false, true);
    assert_eq!(exec.refused_groups.len(), 1);
    assert!(exec.deleted.is_empty());
}

#[test]
fn execute_manifest_skips_files_that_no_longer_exist() {
    let manifest = Manifest {
        version: 1,
        keep_strategy: KeepStrategy::First,
        groups: vec![ManifestEntry {
            size: 5,
            hash: "abc123".to_string(),
            keep: "/tmp/fdupe-test-keep-nonexistent.txt".into(),
            delete: vec!["/tmp/fdupe-test-delete-nonexistent.txt".into()],
        }],
    };

    let exec = execute_manifest(&manifest, false, true);
    assert!(exec.deleted.is_empty());
    assert_eq!(exec.skipped.len(), 1);
}

#[test]
fn execute_manifest_verifies_hash_before_deleting() {
    let dir = new_tempdir();
    let a = write_file(dir.path(), "a.txt", b"verify me before delete");
    let b = write_file(dir.path(), "b.txt", b"verify me before delete");

    let report = scan_dir(dir.path());
    let manifest = build_manifest(&report.duplicate_groups, KeepStrategy::First);

    // Mutate one of the files after the manifest was generated but before
    // it is executed: content changed, so it must not be deleted.
    std::fs::write(&a, b"content has changed since scan").expect("mutate file");
    std::fs::write(&b, b"content has changed since scan too").expect("mutate other file");

    let exec = execute_manifest(&manifest, false, false);
    assert!(exec.deleted.is_empty(), "changed files must not be deleted");
    assert!(!exec.skipped.is_empty());
    assert!(a.exists());
    assert!(b.exists());
}

#[test]
fn skip_verify_bypasses_hash_check() {
    let dir = new_tempdir();
    let a = write_file(dir.path(), "a.txt", b"skip verify content here");
    write_file(dir.path(), "b.txt", b"skip verify content here");

    let report = scan_dir(dir.path());
    let manifest = build_manifest(&report.duplicate_groups, KeepStrategy::First);
    let victim = manifest.groups[0].delete[0].clone();

    if victim == a {
        std::fs::write(&a, b"mutated but we skip verification").expect("mutate");
    }

    let exec = execute_manifest(&manifest, false, true);
    assert_eq!(exec.deleted.len(), 1);
}

#[test]
fn delete_duplicates_keeps_first_and_removes_rest() {
    let dir = new_tempdir();
    write_file(dir.path(), "a_first.txt", b"delete duplicates test content");
    write_file(dir.path(), "b_second.txt", b"delete duplicates test content");
    write_file(dir.path(), "c_third.txt", b"delete duplicates test content");

    let report = scan_dir(dir.path());
    let exec = delete_duplicates(&report.duplicate_groups, KeepStrategy::First, false);

    assert_eq!(exec.deleted.len(), 2);
    assert!(dir.path().join("a_first.txt").exists());
    assert!(!dir.path().join("b_second.txt").exists());
    assert!(!dir.path().join("c_third.txt").exists());
}

#[test]
fn delete_duplicates_dry_run_leaves_all_files() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"dry run delete duplicates content");
    write_file(dir.path(), "b.txt", b"dry run delete duplicates content");

    let report = scan_dir(dir.path());
    let exec = delete_duplicates(&report.duplicate_groups, KeepStrategy::First, true);

    assert_eq!(exec.would_delete.len(), 1);
    assert!(exec.deleted.is_empty());
    assert!(dir.path().join("a.txt").exists());
    assert!(dir.path().join("b.txt").exists());
}

#[test]
fn delete_duplicates_never_removes_every_copy() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"never remove every copy content");
    write_file(dir.path(), "b.txt", b"never remove every copy content");

    let report = scan_dir(dir.path());
    let exec = delete_duplicates(&report.duplicate_groups, KeepStrategy::First, false);

    let remaining = std::fs::read_dir(dir.path()).expect("read dir").count();
    assert_eq!(remaining, 1, "exactly one copy must survive");
    assert!(exec.refused_groups.is_empty());
}

#[test]
fn delete_duplicates_with_shortest_path_strategy() {
    let dir = new_tempdir();
    write_file(dir.path(), "a_very_long_name.txt", b"shortest path deletion content");
    write_file(dir.path(), "s.txt", b"shortest path deletion content");

    let report = scan_dir(dir.path());
    let exec = delete_duplicates(&report.duplicate_groups, KeepStrategy::ShortestPath, false);

    assert_eq!(exec.deleted.len(), 1);
    assert!(dir.path().join("s.txt").exists());
    assert!(!dir.path().join("a_very_long_name.txt").exists());
}

#[test]
fn reclaimable_bytes_reported_by_delete_matches_scan_stats() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"reclaim bytes match content");
    write_file(dir.path(), "b.txt", b"reclaim bytes match content");

    let report = scan_dir(dir.path());
    let expected = report.stats.reclaimable_bytes;
    let exec = delete_duplicates(&report.duplicate_groups, KeepStrategy::First, false);

    assert_eq!(exec.bytes_reclaimed, expected);
}
