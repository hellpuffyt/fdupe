#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;

use assert_cmd::Command;
use common::{new_tempdir, write_file};
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("fdupe").expect("binary should build")
}

#[test]
fn reports_duplicates_in_human_output() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"cli human output content");
    write_file(dir.path(), "b.txt", b"cli human output content");

    cmd().arg(dir.path()).assert().success().stdout(predicate::str::contains("duplicate groups found:   1"));
}

#[test]
fn reports_no_duplicates_message() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"only one of me");

    cmd().arg(dir.path()).assert().success().stdout(predicate::str::contains("no content duplicates found"));
}

#[test]
fn json_output_is_valid_json_with_expected_shape() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"json shape check content");
    write_file(dir.path(), "b.txt", b"json shape check content");

    let output = cmd().arg(dir.path()).arg("--json").output().expect("run fdupe");
    assert!(output.status.success());

    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(parsed["duplicate_groups"].as_array().expect("array").len(), 1);
    assert!(parsed["stats"]["reclaimable_bytes"].as_u64().expect("number") > 0);
}

#[test]
fn min_size_flag_filters_out_small_files() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"tiny");
    write_file(dir.path(), "b.txt", b"tiny");

    cmd()
        .arg(dir.path())
        .arg("--min-size")
        .arg("1000")
        .assert()
        .success()
        .stdout(predicate::str::contains("no content duplicates found"));
}

#[test]
fn exclude_flag_filters_matching_paths() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.log", b"excluded via cli flag content");
    write_file(dir.path(), "b.log", b"excluded via cli flag content");

    cmd()
        .arg(dir.path())
        .arg("--exclude")
        .arg("*.log")
        .assert()
        .success()
        .stdout(predicate::str::contains("no content duplicates found"));
}

#[test]
fn delete_without_keep_fails_with_helpful_error() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"needs keep flag content");
    write_file(dir.path(), "b.txt", b"needs keep flag content");

    cmd().arg(dir.path()).arg("--delete").assert().failure().stderr(predicate::str::contains("--keep"));
}

#[test]
fn manifest_without_keep_fails() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"needs keep for manifest content");
    write_file(dir.path(), "b.txt", b"needs keep for manifest content");

    let manifest_path = dir.path().join("out.json");
    cmd()
        .arg(dir.path())
        .arg("--manifest")
        .arg(&manifest_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("--keep"));
    assert!(!manifest_path.exists());
}

#[test]
fn delete_dry_run_does_not_remove_files() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"cli dry run content payload");
    write_file(dir.path(), "b.txt", b"cli dry run content payload");

    cmd().arg(dir.path()).arg("--delete").arg("--keep").arg("first").arg("--dry-run").assert().success();

    assert!(dir.path().join("a.txt").exists());
    assert!(dir.path().join("b.txt").exists());
}

#[test]
fn delete_with_keep_first_removes_duplicate() {
    let dir = new_tempdir();
    write_file(dir.path(), "a_first.txt", b"cli real delete content payload");
    write_file(dir.path(), "z_second.txt", b"cli real delete content payload");

    cmd().arg(dir.path()).arg("--delete").arg("--keep").arg("first").assert().success();

    assert!(dir.path().join("a_first.txt").exists());
    assert!(!dir.path().join("z_second.txt").exists());
}

#[test]
fn manifest_flag_writes_review_file_without_deleting() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"manifest cli flag content payload");
    write_file(dir.path(), "b.txt", b"manifest cli flag content payload");
    let manifest_path = dir.path().join("review.json");

    cmd().arg(dir.path()).arg("--manifest").arg(&manifest_path).arg("--keep").arg("first").assert().success();

    assert!(manifest_path.exists());
    assert!(dir.path().join("a.txt").exists());
    assert!(dir.path().join("b.txt").exists());

    let contents = std::fs::read_to_string(&manifest_path).expect("read manifest");
    let parsed: serde_json::Value = serde_json::from_str(&contents).expect("valid json manifest");
    assert_eq!(parsed["groups"].as_array().expect("array").len(), 1);
}

#[test]
fn from_manifest_executes_previously_written_plan() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"from manifest execution content");
    write_file(dir.path(), "b.txt", b"from manifest execution content");
    let manifest_path = dir.path().join("review.json");

    cmd().arg(dir.path()).arg("--manifest").arg(&manifest_path).arg("--keep").arg("first").assert().success();

    cmd()
        .arg("--from-manifest")
        .arg(&manifest_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted 1 files"));

    let remaining = std::fs::read_dir(dir.path())
        .expect("read dir")
        .filter(|e| e.as_ref().expect("entry").path() != manifest_path)
        .count();
    assert_eq!(remaining, 1);
}

#[test]
fn from_manifest_dry_run_previews_only() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"from manifest dry run content");
    write_file(dir.path(), "b.txt", b"from manifest dry run content");
    let manifest_path = dir.path().join("review.json");

    cmd().arg(dir.path()).arg("--manifest").arg(&manifest_path).arg("--keep").arg("first").assert().success();

    cmd()
        .arg("--from-manifest")
        .arg(&manifest_path)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("dry run: would delete 1 files"));

    assert!(dir.path().join("a.txt").exists());
    assert!(dir.path().join("b.txt").exists());
}

#[test]
fn help_flag_lists_expected_options() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--keep"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--from-manifest"));
}

#[test]
fn version_flag_prints_version() {
    cmd().arg("--version").assert().success().stdout(predicate::str::contains("fdupe"));
}

#[test]
fn scanning_nonexistent_path_does_not_crash() {
    cmd()
        .arg("/definitely/does/not/exist/anywhere")
        .assert()
        .success()
        .stdout(predicate::str::contains("files scanned:            0"));
}
