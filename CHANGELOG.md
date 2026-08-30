# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
