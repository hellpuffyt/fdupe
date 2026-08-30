#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;

use common::{new_tempdir, write_file};
use fdupe::scan;

fn scan_default(root: &std::path::Path) -> fdupe::model::ScanReport {
    scan(&[root.to_path_buf()], 0, false, &[], None).expect("scan should succeed")
}

#[test]
fn identical_files_are_reported_as_duplicates() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"hello world");
    write_file(dir.path(), "b.txt", b"hello world");

    let report = scan_default(dir.path());
    assert_eq!(report.duplicate_groups.len(), 1);
    assert_eq!(report.duplicate_groups[0].paths.len(), 2);
}

#[test]
fn same_size_different_content_is_not_a_duplicate() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"aaaaaaaaaa");
    write_file(dir.path(), "b.txt", b"bbbbbbbbbb");

    let report = scan_default(dir.path());
    assert!(
        report.duplicate_groups.is_empty(),
        "same-size different-content files must never be reported as duplicates"
    );
}

#[test]
fn empty_files_are_treated_as_duplicates_of_each_other() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.empty", b"");
    write_file(dir.path(), "b.empty", b"");

    let report = scan_default(dir.path());
    assert_eq!(report.duplicate_groups.len(), 1);
    assert_eq!(report.duplicate_groups[0].size, 0);
}

#[test]
fn single_empty_file_is_not_a_duplicate() {
    let dir = new_tempdir();
    write_file(dir.path(), "only.empty", b"");

    let report = scan_default(dir.path());
    assert!(report.duplicate_groups.is_empty());
}

#[test]
fn single_file_tree_has_no_duplicates() {
    let dir = new_tempdir();
    write_file(dir.path(), "solo.txt", b"only file here");

    let report = scan_default(dir.path());
    assert!(report.duplicate_groups.is_empty());
    assert_eq!(report.stats.files_scanned, 1);
}

#[test]
fn empty_directory_has_no_files() {
    let dir = new_tempdir();
    let report = scan_default(dir.path());
    assert_eq!(report.stats.files_scanned, 0);
    assert!(report.duplicate_groups.is_empty());
}

#[test]
fn nested_directories_are_scanned_recursively() {
    let dir = new_tempdir();
    write_file(dir.path(), "top.txt", b"nested duplicate content");
    write_file(dir.path(), "a/b/c/deep.txt", b"nested duplicate content");
    write_file(dir.path(), "a/other.txt", b"unrelated content here");

    let report = scan_default(dir.path());
    assert_eq!(report.duplicate_groups.len(), 1);
    assert_eq!(report.duplicate_groups[0].paths.len(), 2);
    assert_eq!(report.stats.files_scanned, 3);
}

#[test]
fn three_way_duplicate_group() {
    let dir = new_tempdir();
    write_file(dir.path(), "1.txt", b"triplicate payload data");
    write_file(dir.path(), "2.txt", b"triplicate payload data");
    write_file(dir.path(), "3.txt", b"triplicate payload data");

    let report = scan_default(dir.path());
    assert_eq!(report.duplicate_groups.len(), 1);
    assert_eq!(report.duplicate_groups[0].paths.len(), 3);
    assert_eq!(report.stats.duplicate_files, 3);
}

#[test]
fn multiple_independent_duplicate_groups() {
    let dir = new_tempdir();
    write_file(dir.path(), "a1.txt", b"group A content here");
    write_file(dir.path(), "a2.txt", b"group A content here");
    write_file(dir.path(), "b1.txt", b"group B content different");
    write_file(dir.path(), "b2.txt", b"group B content different");
    write_file(dir.path(), "unique.txt", b"nothing matches me at all");

    let report = scan_default(dir.path());
    assert_eq!(report.duplicate_groups.len(), 2);
}

#[test]
fn excluded_glob_removes_matching_files() {
    let dir = new_tempdir();
    write_file(dir.path(), "keep_a.txt", b"excluded test content");
    write_file(dir.path(), "keep_b.txt", b"excluded test content");
    write_file(dir.path(), "skip/ignored_a.log", b"excluded test content");
    write_file(dir.path(), "skip/ignored_b.log", b"excluded test content");

    let report = scan(&[dir.path().to_path_buf()], 0, false, &["**/*.log".to_string()], None)
        .expect("scan should succeed");

    assert_eq!(report.duplicate_groups.len(), 1);
    assert_eq!(report.duplicate_groups[0].paths.len(), 2);
    for p in &report.duplicate_groups[0].paths {
        assert!(!p.to_string_lossy().ends_with(".log"));
    }
}

#[test]
fn exclude_by_extension_glob() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.tmp", b"tmp duplicate payload");
    write_file(dir.path(), "b.tmp", b"tmp duplicate payload");

    let report = scan(&[dir.path().to_path_buf()], 0, false, &["*.tmp".to_string()], None)
        .expect("scan should succeed");

    assert!(report.duplicate_groups.is_empty());
    assert_eq!(report.stats.files_scanned, 0);
}

#[test]
fn min_size_filter_drops_small_files() {
    let dir = new_tempdir();
    write_file(dir.path(), "small_a.txt", b"tiny");
    write_file(dir.path(), "small_b.txt", b"tiny");
    write_file(dir.path(), "big_a.txt", b"this file is definitely bigger than tiny");
    write_file(dir.path(), "big_b.txt", b"this file is definitely bigger than tiny");

    let report = scan(&[dir.path().to_path_buf()], 10, false, &[], None).expect("scan should succeed");

    assert_eq!(report.duplicate_groups.len(), 1);
    assert!(report.duplicate_groups[0].size >= 10);
    assert_eq!(report.stats.files_scanned, 2);
}

#[test]
fn min_size_zero_keeps_everything() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"x");
    write_file(dir.path(), "b.txt", b"x");

    let report = scan(&[dir.path().to_path_buf()], 0, false, &[], None).expect("scan should succeed");
    assert_eq!(report.duplicate_groups.len(), 1);
}

#[cfg(unix)]
#[test]
fn hard_links_are_reported_as_identity_not_duplicates() {
    let dir = new_tempdir();
    let original = write_file(dir.path(), "original.txt", b"hard linked content");
    let link = dir.path().join("link.txt");
    std::fs::hard_link(&original, &link).expect("create hard link");

    let report = scan_default(dir.path());
    assert!(report.duplicate_groups.is_empty(), "hard links must not be reported as content duplicates");
    assert_eq!(report.identity_groups.len(), 1);
    assert_eq!(report.identity_groups[0].paths.len(), 2);
    assert_eq!(report.stats.identity_files_skipped, 1);
}

#[cfg(unix)]
#[test]
fn hard_link_plus_a_true_duplicate_elsewhere() {
    let dir = new_tempdir();
    let original = write_file(dir.path(), "original.txt", b"shared bytes for linking");
    let link = dir.path().join("link.txt");
    std::fs::hard_link(&original, &link).expect("create hard link");
    write_file(dir.path(), "separate_copy.txt", b"shared bytes for linking");

    let report = scan_default(dir.path());
    assert_eq!(report.identity_groups.len(), 1);
    assert_eq!(report.duplicate_groups.len(), 1);
    assert_eq!(report.duplicate_groups[0].paths.len(), 2);
}

#[test]
fn unique_size_file_never_gets_fully_hashed() {
    let dir = new_tempdir();
    write_file(dir.path(), "unique_size.txt", b"a very specific and unique length!");
    write_file(dir.path(), "dup_a.txt", b"same as dup_b");
    write_file(dir.path(), "dup_b.txt", b"same as dup_b");

    let report = scan_default(dir.path());
    assert_eq!(report.stats.unique_size_files, 1);
    // Only the two same-size duplicate candidates should ever reach stage 3.
    assert_eq!(report.stats.fully_hashed, 2);
}

#[test]
fn cascade_short_circuits_when_all_sizes_unique() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"one");
    write_file(dir.path(), "b.txt", b"twotwo");
    write_file(dir.path(), "c.txt", b"threethreethree");

    let report = scan_default(dir.path());
    assert_eq!(report.stats.unique_size_files, 3);
    assert_eq!(report.stats.sample_hashed, 0);
    assert_eq!(report.stats.fully_hashed, 0);
    assert!(report.duplicate_groups.is_empty());
}

#[test]
fn sample_stage_eliminates_same_size_different_content() {
    let dir = new_tempdir();
    // Same size, different content -> should be eliminated by the sample
    // stage (or at worst the full-hash stage), never reported as dupes.
    write_file(dir.path(), "a.bin", &[1u8; 5000]);
    write_file(dir.path(), "b.bin", &[2u8; 5000]);

    let report = scan_default(dir.path());
    assert!(report.duplicate_groups.is_empty());
    assert_eq!(report.stats.sample_hashed, 2);
}

#[test]
fn large_head_tail_match_but_middle_differs_is_not_a_duplicate() {
    let dir = new_tempdir();
    let size = 20_000usize;
    let mut a = vec![7u8; size];
    let mut b = vec![7u8; size];
    // Same head and tail (within default 4096-byte sample window), but the
    // middle differs -- this is exactly the case stage 2 alone would miss,
    // and stage 3's full hash must catch it.
    a[size / 2] = 1;
    b[size / 2] = 2;

    write_file(dir.path(), "a.bin", &a);
    write_file(dir.path(), "b.bin", &b);

    let report = scan_default(dir.path());
    assert!(
        report.duplicate_groups.is_empty(),
        "files differing only in the middle must not be reported as duplicates"
    );
    assert_eq!(report.stats.fully_hashed, 2, "both must reach the full-hash stage");
}

#[test]
fn reclaimable_bytes_matches_group_math() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"twelve bytes");
    write_file(dir.path(), "b.txt", b"twelve bytes");
    write_file(dir.path(), "c.txt", b"twelve bytes");

    let report = scan_default(dir.path());
    let group = &report.duplicate_groups[0];
    assert_eq!(group.reclaimable_bytes(), group.size * 2);
    assert_eq!(report.stats.reclaimable_bytes, group.size * 2);
}

#[test]
fn multiple_roots_are_all_scanned() {
    let dir_a = new_tempdir();
    let dir_b = new_tempdir();
    write_file(dir_a.path(), "x.txt", b"cross-root duplicate value");
    write_file(dir_b.path(), "y.txt", b"cross-root duplicate value");

    let report = scan(&[dir_a.path().to_path_buf(), dir_b.path().to_path_buf()], 0, false, &[], None)
        .expect("scan should succeed");

    assert_eq!(report.duplicate_groups.len(), 1);
    assert_eq!(report.duplicate_groups[0].paths.len(), 2);
}

#[test]
fn custom_sample_size_still_finds_duplicates() {
    let dir = new_tempdir();
    write_file(dir.path(), "a.txt", b"custom sample size content payload");
    write_file(dir.path(), "b.txt", b"custom sample size content payload");

    let report = scan(&[dir.path().to_path_buf()], 0, false, &[], Some(8)).expect("scan should succeed");
    assert_eq!(report.duplicate_groups.len(), 1);
}

#[test]
fn symlinks_are_not_followed_by_default() {
    let dir = new_tempdir();
    let target = write_file(dir.path(), "target.txt", b"symlink target content");
    let link = dir.path().join("link.txt");

    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &link).expect("create symlink");
    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_file(&target, &link).is_err() {
            // Creating symlinks on Windows CI may require a privilege we
            // don't have; skip rather than fail spuriously.
            return;
        }
    }

    let report = scan_default(dir.path());
    // The symlink itself should not be walked into as a duplicate of its
    // target when follow_symlinks is false.
    assert!(report.duplicate_groups.is_empty());
    assert_eq!(report.stats.files_scanned, 1);
}
