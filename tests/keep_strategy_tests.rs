#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;

use common::{new_tempdir, write_file};
use fdupe::keep::choose_survivor;
use fdupe::model::KeepStrategy;
use std::fs;
use std::time::{Duration, SystemTime};

#[test]
fn first_strategy_picks_lexicographically_first_path() {
    let dir = new_tempdir();
    let a = write_file(dir.path(), "b_second.txt", b"x");
    let b = write_file(dir.path(), "a_first.txt", b"x");

    let survivor = choose_survivor(&[a, b.clone()], KeepStrategy::First);
    assert_eq!(survivor, b);
}

#[test]
fn shortest_path_strategy_picks_shortest_name() {
    let dir = new_tempdir();
    let long = write_file(dir.path(), "a_much_longer_filename.txt", b"x");
    let short = write_file(dir.path(), "s.txt", b"x");

    let survivor = choose_survivor(&[long, short.clone()], KeepStrategy::ShortestPath);
    assert_eq!(survivor, short);
}

#[test]
fn shortest_path_ties_break_lexicographically() {
    let dir = new_tempdir();
    let a = write_file(dir.path(), "bbb.txt", b"x");
    let b = write_file(dir.path(), "aaa.txt", b"x");

    let survivor = choose_survivor(&[a, b.clone()], KeepStrategy::ShortestPath);
    assert_eq!(survivor, b);
}

#[test]
fn oldest_strategy_picks_earliest_mtime() {
    let dir = new_tempdir();
    let old = write_file(dir.path(), "old.txt", b"x");
    let new = write_file(dir.path(), "new.txt", b"x");

    set_mtime(&old, SystemTime::UNIX_EPOCH + Duration::from_secs(1000));
    set_mtime(&new, SystemTime::UNIX_EPOCH + Duration::from_secs(2000));

    let survivor = choose_survivor(&[new, old.clone()], KeepStrategy::Oldest);
    assert_eq!(survivor, old);
}

#[test]
fn newest_strategy_picks_latest_mtime() {
    let dir = new_tempdir();
    let old = write_file(dir.path(), "old.txt", b"x");
    let new = write_file(dir.path(), "new.txt", b"x");

    set_mtime(&old, SystemTime::UNIX_EPOCH + Duration::from_secs(1000));
    set_mtime(&new, SystemTime::UNIX_EPOCH + Duration::from_secs(2000));

    let survivor = choose_survivor(&[old, new.clone()], KeepStrategy::Newest);
    assert_eq!(survivor, new);
}

#[test]
fn oldest_strategy_with_three_files() {
    let dir = new_tempdir();
    let a = write_file(dir.path(), "a.txt", b"x");
    let b = write_file(dir.path(), "b.txt", b"x");
    let c = write_file(dir.path(), "c.txt", b"x");

    set_mtime(&a, SystemTime::UNIX_EPOCH + Duration::from_secs(3000));
    set_mtime(&b, SystemTime::UNIX_EPOCH + Duration::from_secs(1000));
    set_mtime(&c, SystemTime::UNIX_EPOCH + Duration::from_secs(2000));

    let survivor = choose_survivor(&[a, b.clone(), c], KeepStrategy::Oldest);
    assert_eq!(survivor, b);
}

fn set_mtime(path: &std::path::Path, time: SystemTime) {
    let file = fs::File::options().write(true).open(path).expect("open for mtime set");
    file.set_modified(time).expect("set mtime");
}
