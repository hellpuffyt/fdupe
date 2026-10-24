# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Fixed

- **Data loss:** executing a manifest (or `--delete`) never checked the
  copy being kept. If it had been deleted or changed after the manifest
  was written, the remaining copies were deleted anyway, destroying the
  last copy. The kept file must now exist and (unless `--skip-verify`)
  still match, or the whole group is refused.
- A victim naming the kept file under another spelling (`./a` vs `a`) or
  as a hard link could be deleted; victims that are the same file as the
  survivor are now skipped.
- A group whose delete list names the kept file under another spelling
  (`sub/../a`, an absolute path, a case variant on a case-insensitive
  filesystem) or as a hard link was only partly refused: the alias was
  skipped but the rest of the group was deleted and the command exited 0.
  Such a group is now refused whole, as documented.
- `--from-manifest` deleted files without `--delete`, although `--help`
  documented `--delete` as required. It now refuses unless `--delete` or
  `--dry-run` is given.
- `--delete` skipped re-verification; files changed between the scan and
  the deletion are now re-hashed and skipped if they no longer match.
- Stage-2 hashing read small files with `read_to_end`, so a file that grew
  after the walk was read whole into memory; reads are now limited to the
  walked size and streamed.
- A missing root path exited 0 with an empty report; it is now an error.

### Added

- `unreadable_skipped` in the report: entries skipped because of
  permission errors, symlink loops or files vanishing mid-scan.
- `tests/safety_tests.rs`; `SECURITY.md`, `.editorconfig`,
  `docs/PRODUCTION_READINESS.md`.

### Changed

- `delete_duplicates` takes a `skip_verify` argument.
- README example output replaced with a real run on a generated tree.

## [0.1.0] - 2026-08-30

### Added

- Three-stage duplicate detection cascade: size grouping, head/tail sample
  hash, then full content hash, so most files are never fully read.
- Parallel hashing of stages 2 and 3 via `rayon`.
- Hard-link and (optional) followed-symlink identity detection, reported
  separately from content duplicates.
- Safe deletion workflow: `--delete` requires an explicit `--keep`
  strategy (`first`, `oldest`, `newest`, `shortest-path`), supports
  `--dry-run`, and refuses to delete every copy of a group.
- JSON review manifest (`--manifest`) that a later `--from-manifest` run
  can execute, re-verifying file hashes before deleting anything.
- Filters: `--min-size`, `--exclude` globs, `--follow-symlinks` (off by
  default).
- Human-readable and `--json` report output, including total reclaimable
  bytes and a breakdown of how much work each cascade stage eliminated.
