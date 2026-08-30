#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub fn write_file(dir: &Path, rel: &str, content: &[u8]) -> PathBuf {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(&path, content).expect("write test file");
    path
}

pub fn new_tempdir() -> TempDir {
    tempfile::tempdir().expect("create tempdir")
}
