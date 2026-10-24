# fdupe

Find duplicate files by content across large directory trees, in
parallel, without hashing bytes it doesn't have to — and without deleting
anything until you've reviewed exactly what it plans to do.

## What

`fdupe` scans one or more directories, finds files whose contents are
byte-for-byte identical, and reports them grouped together with the total
space you'd reclaim by keeping just one copy of each. It never deletes
anything unless you explicitly ask it to, and even then only through a
narrow, auditable path.

## Why

Most duplicate finders pick one of two bad defaults: they hash the full
contents of every file up front (slow on large trees), or they guess at
duplicates from filenames/sizes and delete on that guess (unsafe).
`fdupe` is fast because it's lazy about reading bytes, and safe because
deciding *that* something is a duplicate is a completely separate step
from deciding *what to do about it*.

## How the cascade works

Comparing file contents is the expensive part, so `fdupe` avoids it for
as long as possible using three stages, each one only run on files that
survived the last:

1. **Group by size.** Two files of different sizes cannot be identical.
   This costs nothing — the size is already in the metadata from the
   directory walk. Most trees have very few files sharing an exact size,
   so this alone eliminates the vast majority of candidates.
2. **Sample hash.** For files that share a size with at least one other
   file, hash a small sample: the first 4 KiB and the last 4 KiB (files
   smaller than 8 KiB are hashed in full here, since that's cheaper than
   reading twice). Files with different content almost always differ
   somewhere in that sample, so this eliminates almost everything that
   made it past stage 1.
3. **Full hash.** Only files that share both size *and* sample hash — the
   rare case where two different files happen to start and end the same
   way — get their entire contents read and hashed. This is the only
   stage that can produce a false negative if skipped, and it's the only
   one that's actually expensive, which is exactly why it runs last and
   on the fewest files.

The report tells you how many files were eliminated at each stage, so you
can see the cascade working: on a real tree, `fully_hashed` is typically
a tiny fraction of `files_scanned`.

Hard links (and, if `--follow-symlinks` is set, symlinks) pointing at the
same underlying file are detected up front and reported as an *identity*
group, separate from content duplicates — they're not two files that
happen to match, they're one file with two names, and deleting one
wouldn't reclaim anything.

## Features

- Three-stage size → sample → full-hash cascade with per-stage stats.
- Parallel hashing via [`rayon`](https://docs.rs/rayon)'s work-stealing
  pool (see [Parallelism](#parallelism) below).
- Hard-link / same-inode detection, reported separately from duplicates.
- `--min-size`, `--exclude <glob>` (repeatable), `--follow-symlinks`
  (off by default).
- Safe deletion: `--delete` requires `--keep <strategy>`, supports
  `--dry-run`, and refuses to delete every copy of a group. Before
  anything in a group is removed, the kept copy is checked to still exist
  and still match, and every file to be removed is re-hashed.
- JSON review manifest (`--manifest`) that `--from-manifest --delete`
  later executes under the same checks.
- Unreadable entries (permission denied, symlink loops, files that vanish
  mid-scan) are skipped and counted in the report rather than aborting the
  scan; a root path that does not exist is an error.
- Human-readable or `--json` output, including total reclaimable bytes.

### Parallelism

Hashing is CPU- and IO-bound work over an independent list of files with
no coordination needed between them — a textbook data-parallel workload.
`rayon`'s `par_iter` sizes its pool to the available cores automatically
and handles work-stealing, which is a better fit here than hand-rolled
`std::thread` + channel plumbing for the same result with far less code
to get wrong.

## Installation

```sh
git clone https://github.com/hellpuffyt/fdupe.git
cd fdupe
cargo install --path .
```

## Usage

```
fdupe [PATHS...] [OPTIONS]
```

```sh
# Scan the current directory, print a human-readable report
fdupe .

# Scan multiple trees, ignore anything under a build/vendor dir, and
# skip files smaller than 1 KiB
fdupe ~/Photos ~/Backups --exclude "**/node_modules/**" --min-size 1024

# Machine-readable output
fdupe . --json
```

### Safety model

Deletion is deliberately harder than reporting:

```sh
# 1. See what's there. Nothing is touched.
fdupe . 

# 2. Write a review manifest describing exactly what would be kept and
#    deleted, without deleting anything.
fdupe . --manifest review.json --keep first

# 3. Inspect review.json by hand (it's plain JSON: one entry per
#    duplicate group, with a `keep` path and a `delete` list).

# 4. Execute exactly that plan (`--delete` is required). The kept copy
#    of each group must still exist and match, or the whole group is
#    refused; each file to delete is re-hashed immediately before
#    deletion — if it changed since step 2, it is skipped.
fdupe --from-manifest review.json --delete

# Or preview the execution without deleting anything:
fdupe --from-manifest review.json --dry-run
```

If you'd rather skip the manifest step, `--delete` runs the same plan
immediately, with the same re-verification (files can change between
being hashed during the scan and being deleted):

```sh
fdupe . --delete --keep first --dry-run   # preview
fdupe . --delete --keep first             # actually delete
```

`--keep` strategies:

| Strategy         | Keeps                                            |
|------------------|---------------------------------------------------|
| `first`          | The path that sorts first lexicographically        |
| `oldest`         | The file with the earliest modification time       |
| `newest`         | The file with the latest modification time         |
| `shortest-path`  | The file with the shortest path (ties broken lexicographically) |

`fdupe` will always keep at least one file per duplicate group. If a
manifest (hand-edited or otherwise) would result in every copy of a
group being deleted — the kept path also listed for deletion (under any
spelling, or as a hard link), or the kept file missing or changed since
the manifest was written — that group is refused and reported, nothing
in it is touched, and the command exits non-zero.

Without `--delete`, nothing is ever removed: scanning, `--json`,
`--manifest` (which only writes the manifest file you name) and every
`--dry-run` leave the scanned tree untouched. `--skip-verify` turns off
the re-hashing (not the kept-copy existence check) and is not
recommended.

Remaining limits: there is an unavoidable window between verifying a file
and removing it; and with `--follow-symlinks` a duplicate "copy" can be a
symlink, in which case deleting it removes only the link.

### Platform differences

Identity detection (hard links / same file via a followed symlink) uses
`(device, inode)` on Unix-like systems and the NTFS file index on
Windows (via [`same-file`](https://docs.rs/same-file)). On some
filesystems — notably certain network shares — a reliable file index
isn't available; in that case those files simply aren't merged into an
identity group and fall through to ordinary content comparison, which is
strictly safe (at worst, a hard-linked pair shows up as an ordinary
duplicate group instead of an identity group).

## Examples

Real output (truncated after the summary) from a generated demo tree of 47
files: 40 random "photos", copies of three of them in a `backup/` folder
(one twice), one hard link, and two same-size files that share their
first and last 4 KiB but differ in the middle:

```
$ fdupe ./photos
fdupe scan report
==================
files scanned:            47
identical files (links):  1 (1 extra paths, not counted as duplicates)
eliminated by size stage: 37 files (unique size)
sample-hashed:             9 files
eliminated by sample stage: 0 files (unique sample)
fully hashed:              9 files

duplicate groups found:   3
duplicate files:          7
reclaimable space:        110.15 KiB
```

The two same-size files survive the sample stage (identical head and
tail) and are only told apart by the full hash, which is exactly the case
stage 3 exists for.

## Benchmarks

Not formally benchmarked with recorded numbers in this repository yet —
the cascade's per-stage counters in every report (`sample_hashed` vs.
`fully_hashed` vs. `files_scanned`) are the honest, reproducible evidence
that it does far less full-file hashing than a naive hash-everything
approach on any tree with more unique-sized files than duplicates, which
is the common case.

## Testing

```sh
cargo test --all-targets
```

70 tests (0 ignored). The test suite builds real directory trees with `tempfile` and covers:
identical-content detection, the same-size/different-content false
positive guard, empty files, single-file trees, nested directories,
exclude globs, the min-size filter, hard-link identity grouping, each
`--keep` strategy choosing the correct survivor, the refusal to delete
every copy of a group, manifest round-tripping and re-verification, and
that the cascade actually short-circuits (a uniquely sized file is never
sample- or fully-hashed). `tests/safety_tests.rs` covers the deletion
safety rules: a kept copy deleted or modified after the manifest was
written, the kept file listed again under another spelling or as a hard
link, files changed between scan and `--delete`, `--from-manifest`
without `--delete`, report-only modes leaving the tree byte-for-byte
unchanged, symlink loops, unreadable directories and files (Unix; these
must run as a non-root user), and files growing during the scan.

## Security

`fdupe` only reads file contents and metadata it's pointed at, and only
writes to disk when you pass `--delete` or `--manifest`/`--from-manifest`
with the appropriate flags. It contains no `unsafe` code
(`#![forbid(unsafe_code)]`). If you find a security issue, please report it privately as described in
[SECURITY.md](SECURITY.md) rather than opening a public issue.

## License

MIT. See [LICENSE](LICENSE).
