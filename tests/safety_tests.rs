//! Regression tests for the deletion safety rules: fdupe must never remove
//! the last copy of anything, never act without an explicit flag, and must
//! cope with trees that change or cannot be read during a scan.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use assert_cmd::Command;
use common::{new_tempdir, write_file};
use fdupe::delete::delete_duplicates;
use fdupe::manifest::{build_manifest, execute_manifest, Manifest, ManifestEntry};
use fdupe::model::KeepStrategy;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

fn cmd() -> Command {
    Command::cargo_bin("fdupe").expect("binary built")
}

fn scan(dir: &Path) -> fdupe::model::ScanReport {
    fdupe::scan(&[dir.to_path_buf()], 0, false, &[], None).expect("scan")
}

fn manifest_for(dir: &Path) -> Manifest {
    build_manifest(&scan(dir).duplicate_groups, KeepStrategy::First)
}

#[test]
fn kept_copy_deleted_after_manifest_refuses_the_group() {
    // Regression: the survivor was never checked, so removing it after the
    // manifest was written and then executing the manifest deleted the
    // only remaining copy.
    let dir = new_tempdir();
    let a = write_file(dir.path(), "a.txt", b"the only data that matters");
    let b = write_file(dir.path(), "b.txt", b"the only data that matters");
    let manifest = manifest_for(dir.path());
    assert_eq!(manifest.groups[0].keep, a);

    fs::remove_file(&a).unwrap();
    for skip_verify in [false, true] {
        let exec = execute_manifest(&manifest, false, skip_verify);
        assert_eq!(exec.deleted, [] as [std::path::PathBuf; 0]);
        assert_eq!(exec.refused_groups.len(), 1);
    }
    assert!(b.exists(), "last copy must survive");
}

#[test]
fn kept_copy_modified_after_manifest_refuses_the_group() {
    let dir = new_tempdir();
    let a = write_file(dir.path(), "a.txt", b"original contents of a");
    let b = write_file(dir.path(), "b.txt", b"original contents of a");
    let manifest = manifest_for(dir.path());

    fs::write(&a, b"a was rewritten since the scan").unwrap();
    let exec = execute_manifest(&manifest, false, false);
    assert_eq!(exec.deleted, [] as [std::path::PathBuf; 0]);
    assert_eq!(exec.refused_groups.len(), 1);
    assert_eq!(fs::read(&b).unwrap(), b"original contents of a");
}

#[test]
fn victim_that_is_the_kept_file_under_another_spelling_is_not_deleted() {
    // A hand-edited (or path-spelling-inconsistent) manifest naming the
    // survivor twice must not delete it.
    let dir = new_tempdir();
    let a = write_file(dir.path(), "a.txt", b"spelled two ways");
    let hash = fdupe::hashing::full_hash(&a).unwrap();
    let other_spelling = dir.path().join(".").join("a.txt");
    let manifest = Manifest {
        version: 1,
        keep_strategy: KeepStrategy::First,
        groups: vec![ManifestEntry { size: 16, hash, keep: a.clone(), delete: vec![other_spelling] }],
    };
    let exec = execute_manifest(&manifest, false, false);
    assert_eq!(exec.deleted, [] as [std::path::PathBuf; 0]);
    assert!(a.exists());
}

#[test]
fn hard_link_victim_of_the_kept_file_is_not_deleted() {
    let dir = new_tempdir();
    let a = write_file(dir.path(), "a.txt", b"hard linked content");
    let link = dir.path().join("link.txt");
    fs::hard_link(&a, &link).unwrap();
    let hash = fdupe::hashing::full_hash(&a).unwrap();
    let manifest = Manifest {
        version: 1,
        keep_strategy: KeepStrategy::First,
        groups: vec![ManifestEntry { size: 19, hash, keep: a.clone(), delete: vec![link.clone()] }],
    };
    let exec = execute_manifest(&manifest, false, false);
    assert_eq!(exec.deleted, [] as [std::path::PathBuf; 0]);
    assert!(link.exists());
}

#[test]
fn group_listing_the_kept_file_under_another_spelling_is_refused_whole() {
    // Regression: the alias was skipped but the rest of the group was still
    // deleted and the command exited 0, although the documented contract is
    // that such a group is refused and nothing in it is touched.
    let dir = new_tempdir();
    let a = write_file(dir.path(), "a.txt", b"aliased survivor");
    let b = write_file(dir.path(), "b.txt", b"aliased survivor");
    fs::create_dir(dir.path().join("sub")).unwrap();
    let alias = dir.path().join("sub").join("..").join("a.txt");
    let hash = fdupe::hashing::full_hash(&a).unwrap();
    let manifest = Manifest {
        version: 1,
        keep_strategy: KeepStrategy::First,
        groups: vec![ManifestEntry { size: 16, hash, keep: a.clone(), delete: vec![b.clone(), alias] }],
    };
    let exec = execute_manifest(&manifest, false, false);
    assert_eq!(exec.deleted, [] as [std::path::PathBuf; 0]);
    assert_eq!(exec.refused_groups.len(), 1);
    assert!(a.exists());
    assert!(b.exists());
}

#[test]
fn direct_delete_reverifies_files_changed_after_the_scan() {
    // Regression: `--delete` skipped verification ("computed moments ago"),
    // but a file can change between being hashed and being deleted.
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"same bytes in both");
    let b = write_file(dir.path(), "b.txt", b"same bytes in both");
    let report = scan(dir.path());

    fs::write(&b, b"b changed after the scan").unwrap();
    let exec = delete_duplicates(&report.duplicate_groups, KeepStrategy::First, false, false);
    assert_eq!(exec.deleted, [] as [std::path::PathBuf; 0]);
    assert_eq!(fs::read(&b).unwrap(), b"b changed after the scan");
}

#[test]
fn from_manifest_without_delete_or_dry_run_touches_nothing() {
    // Regression: `--help` said `--from-manifest` needs `--delete` to remove
    // files, but it deleted without it.
    let dir = new_tempdir();
    let a = write_file(dir.path(), "a.txt", b"needs an explicit flag");
    let b = write_file(dir.path(), "b.txt", b"needs an explicit flag");
    let manifest_path = dir.path().join("review.json");
    cmd().arg(dir.path()).arg("--manifest").arg(&manifest_path).args(["--keep", "first"]).assert().success();

    cmd()
        .arg("--from-manifest")
        .arg(&manifest_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("needs --delete"));
    assert!(a.exists() && b.exists());
}

#[test]
fn report_only_modes_never_modify_the_tree() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"report only");
    write_file(dir.path(), "b.txt", b"report only");
    write_file(dir.path(), "sub/c.txt", b"report only");
    let snapshot = |d: &Path| {
        let mut v: Vec<_> = walkdir::WalkDir::new(d)
            .into_iter()
            .map(|e| {
                let e = e.unwrap();
                let m = e.metadata().unwrap();
                // Directory mtimes are not compared: NTFS updates them lazily, so a
                // directory written during setup can change its mtime later with no
                // action by fdupe. Files keep size + mtime; directories keep presence.
                if m.is_dir() {
                    (e.path().to_path_buf(), 0, None)
                } else {
                    (e.path().to_path_buf(), m.len(), m.modified().ok())
                }
            })
            .collect();
        v.sort();
        v
    };
    let before = snapshot(dir.path());
    for args in [
        vec![],
        vec!["--json"],
        vec!["--keep", "first"],
        vec!["--dry-run"],
        vec!["--delete", "--keep", "first", "--dry-run"],
    ] {
        cmd().arg(dir.path()).args(&args).assert().success();
    }
    assert_eq!(before, snapshot(dir.path()));
}

#[test]
fn missing_root_is_an_error_not_an_empty_report() {
    let dir = new_tempdir();
    cmd()
        .arg(dir.path().join("does-not-exist"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot scan"));
}

#[test]
fn file_growing_after_the_walk_is_hashed_only_up_to_its_walked_size() {
    // Stage-2 hashing of small files used `read_to_end`, so a file that
    // grew after the walk (a log being written) was read whole into memory.
    let dir = new_tempdir();
    let p = write_file(dir.path(), "grow.log", b"0123456789");
    let before = fdupe::hashing::sample_hash(&p, 10, 4096).unwrap();
    let mut f = fs::OpenOptions::new().append(true).open(&p).unwrap();
    std::io::Write::write_all(&mut f, &vec![b'x'; 1 << 20]).unwrap();
    assert_eq!(fdupe::hashing::sample_hash(&p, 10, 4096).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn symlink_loop_with_follow_symlinks_terminates_and_is_counted() {
    let dir = new_tempdir();
    write_file(dir.path(), "d/a.txt", b"loop content");
    write_file(dir.path(), "d/b.txt", b"loop content");
    std::os::unix::fs::symlink(dir.path().join("d"), dir.path().join("d/loop")).unwrap();
    let report = fdupe::scan(&[dir.path().to_path_buf()], 0, true, &[], None).unwrap();
    assert_eq!(report.stats.duplicate_groups, 1);
    assert_eq!(report.duplicate_groups[0].paths.len(), 2);
    assert!(report.stats.unreadable_skipped >= 1, "loop should be reported");
}

#[cfg(unix)]
#[test]
fn unreadable_directory_is_skipped_and_counted_not_fatal() {
    use std::os::unix::fs::PermissionsExt;
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"perm content");
    write_file(dir.path(), "b.txt", b"perm content");
    let locked = dir.path().join("locked");
    write_file(dir.path(), "locked/c.txt", b"perm content");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    // Running as root bypasses permission bits, so the precondition fails
    // loudly rather than the test passing without testing anything.
    let readable_anyway = fs::read_dir(&locked).is_ok();
    let report = scan(dir.path());
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(!readable_anyway, "precondition: run as a non-root user");
    assert_eq!(report.stats.duplicate_groups, 1);
    assert_eq!(report.duplicate_groups[0].paths.len(), 2);
    assert_eq!(report.stats.unreadable_skipped, 1);
}

#[cfg(unix)]
#[test]
fn unreadable_file_is_never_grouped_or_deleted() {
    use std::os::unix::fs::PermissionsExt;
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"unreadable file content");
    let b = write_file(dir.path(), "b.txt", b"unreadable file content");
    fs::set_permissions(&b, fs::Permissions::from_mode(0o000)).unwrap();
    let readable_anyway = fs::File::open(&b).is_ok();
    let report = scan(dir.path());
    fs::set_permissions(&b, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(!readable_anyway, "precondition: run as a non-root user");
    assert_eq!(report.stats.duplicate_groups, 0);
    assert!(report.stats.unreadable_skipped >= 1);
}
