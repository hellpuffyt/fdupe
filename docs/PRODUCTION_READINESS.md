# Production readiness — fdupe

Audit date: 2026-10-22. Branch: `audit/production-readiness`.
Tested commit: `70da87c` (the final commit adds only this file).

## Scope

`fdupe` is a CLI (plus library) that finds byte-identical files with a
size → head/tail sample hash → full SHA-256 cascade (`cascade.rs`,
`hashing.rs`), groups hard links / same-file paths separately
(`identity.rs`, via `same-file`), and optionally deletes duplicates
through `--delete --keep <strategy>` or a reviewed JSON manifest
(`manifest.rs`, `delete.rs`, `keep.rs`). Walking uses `walkdir` with
exclude globs (`walk.rs`); hashing runs on `rayon`.

**Safety contract (verified):** without `--delete`, `fdupe` never removes
or modifies anything in the scanned tree; the only write is the
`--manifest <path>` file the user names. With `--delete`, a group is only
acted on if the kept copy still exists and still matches, and each victim
is re-hashed immediately before removal.

## Platforms and prerequisites

- Rust 1.88+ (declared MSRV; CI has a 1.88.0 job — not run locally; the
  host had 1.89/1.97/1.98).
- Verified on macOS 27 (Apple M5). CI covers Linux, macOS and Windows.
  The permission-error tests are Unix-only and must run as a non-root
  user (they assert that precondition instead of silently passing).

## Clean-clone verification (literal README steps)

```
git clone -b audit/production-readiness <repo> fdupe && cd fdupe
cargo install --locked --path . --root <scratch>/inst   # exit 0
cargo test --all-targets                                 # exit 0 — 69 passed, 0 failed, 0 ignored
cargo fmt --all -- --check                               # exit 0
cargo clippy --all-targets -- -D warnings                # exit 0
```

(`--root` was added to `cargo install` only to avoid installing into the
shared host's `~/.cargo/bin`.) The README usage and the full safety-model
workflow were then run on a small tree: scan; `--exclude
"**/node_modules/**" --min-size 1 --json`; `--manifest ... --keep first`;
`--from-manifest` alone (refused, exit 1, nothing changed);
`--from-manifest --dry-run` (listed 2); `--from-manifest --delete`
(deleted 2, kept `a/1.txt`); `--delete --keep first --dry-run`.

## Commands and results (audit host)

| Command | Exit | Result |
| --- | --- | --- |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | 0 | clean |
| `cargo test --all-targets --locked` (baseline) | 0 | 57 passed, 0 failed, 0 ignored |
| `cargo test --all-targets --locked` (after) | 0 | 69 passed, 0 failed, 0 ignored |
| `cargo test --doc` | 0 | 0 doctests |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked` | 0 | clean |
| `cargo audit --db <local advisory-db> --no-fetch` | 0 | 0 vulnerabilities in 80 crates |
| `cargo deny check` | — | not configured (no `deny.toml`) |

Breakdown after: `cli_tests` 15, `keep_strategy_tests` 6,
`manifest_tests` 13, `safety_tests` 12 (new), `scan_tests` 23.

## CI status

Default branch: `success` (run on `7b89451`, 2026-08-30).
Audit branch: pending until pushed. Actions pinned by tag; unchanged.

## Bugs found and fixed (each with a regression test)

| # | Severity | Defect | Fix | Regression test (`tests/`) |
| --- | --- | --- | --- | --- |
| 1 | **Critical (data loss)** | The kept copy was never checked. Reproduced: `fdupe t --manifest review.json --keep first`, then `rm t/a.txt`, then `fdupe --from-manifest review.json` deleted `t/b.txt` — both copies gone. Same for `--skip-verify` and for a kept file modified after the scan. | `survivor_problem`: kept file must exist, be a regular file and (unless `--skip-verify`) match the hash, else the group is refused and every victim reported as skipped. | `safety_tests::kept_copy_deleted_after_manifest_refuses_the_group`, `kept_copy_modified_after_manifest_refuses_the_group` |
| 2 | High (data loss, crafted/hand-edited manifest) | Victim equal to the kept file under another spelling (`dir/./a.txt` vs `dir/a.txt`) passed the path-equality check and was deleted. | Skip victims that `same_file::is_same_file` the survivor; ignore duplicate victim entries. | `victim_that_is_the_kept_file_under_another_spelling_is_not_deleted`, `hard_link_victim_of_the_kept_file_is_not_deleted` |
| 3 | High (unexpected deletion) | `--from-manifest` deleted without `--delete`, although `--help` says "Combine with `--dry-run` to preview, or `--delete` to actually remove files". | Requires `--delete` or `--dry-run`; otherwise exits 1, nothing changed. Existing test `from_manifest_executes_previously_written_plan` now passes `--delete` (justification: matches the documented flag). | `safety_tests::from_manifest_without_delete_or_dry_run_touches_nothing` |
| 4 | Medium (files changing during scan) | `--delete` skipped re-verification; a file modified between hashing and deletion was deleted. | `delete_duplicates(..., skip_verify)` verifies by default. | `direct_delete_reverifies_files_changed_after_the_scan` |
| 5 | Medium (resource) | Stage-2 hashed files ≤ 2×sample with `read_to_end`; a file that grew after the walk was read whole into memory. | `take(size)` + streaming `io::copy` into the hasher. | `file_growing_after_the_walk_is_hashed_only_up_to_its_walked_size` |
| 6 | Low (wrong result) | A non-existent root exited 0 with "files scanned: 0" (contradicting `scan`'s documented error). Expected value of `cli_tests::scanning_nonexistent_path_does_not_crash` changed accordingly (still asserts no panic). | Root `metadata` checked first. | `missing_root_is_an_error_not_an_empty_report` |
| 7 | Low (transparency) | Unreadable entries / walk errors / hash failures were dropped silently. | Counted in `stats.unreadable_skipped` and shown in the report. | `unreadable_directory_is_skipped_and_counted_not_fatal`, `unreadable_file_is_never_grouped_or_deleted`, `symlink_loop_with_follow_symlinks_terminates_and_is_counted` |
| 8 | Docs | README example output (18,422 files, 842 MiB) could not be traced to any run. | Replaced with a real run on a generated tree. | — |

Also covered: `report_only_modes_never_modify_the_tree` snapshots path,
size and mtime of every entry before/after scan, `--json`, `--keep`
alone, `--dry-run`, and `--delete --dry-run` — unchanged.

## Security review

- `unsafe_code = "forbid"`; no network.
- Deletion only through `execute_manifest`, now with survivor checks.
- Manifest input is trusted by design (it lists paths to delete); this is
  stated in `SECURITY.md` and the README.
- TOCTOU: a file can still change between verification and `remove_file`;
  unavoidable without platform-specific APIs; documented.
- Secret scan of the tree and full history: no hits.
- Workflow permissions: `contents: read`.

## Known limitations

- With `--follow-symlinks`, a duplicate group member may be a symlink;
  deleting it removes only the link (bytes "reclaimed" are overstated).
- There is no counter for files eliminated at the full-hash stage (they
  appear in `fully_hashed` but in no group).
- Windows identity detection uses the NTFS file index via `same-file`;
  not exercised on this host.
- `cargo deny` not configured; MSRV 1.88 not run locally.

## Acceptance checklist

| Item | Expected behaviour | Command / test | Result | Actual | Reference |
| --- | --- | --- | --- | --- | --- |
| Finds identical content | Byte-identical files grouped, different content not | `cargo test --test scan_tests` | pass | 23/23 | `src/cascade.rs` |
| Cascade short-circuits | Unique sizes never hashed | `scan_tests` + demo run | pass | 37 of 47 eliminated by size | `src/cascade.rs` |
| Hard links | Reported as identity, not duplicates | `scan_tests::hard_links_*`, demo | pass | 1 identity group | `src/identity.rs` |
| No modification without `--delete` | Tree unchanged in report-only modes | `safety_tests::report_only_modes_never_modify_the_tree` | pass | identical snapshot | `src/main.rs` |
| Never delete last copy | Group refused if survivor missing/changed/same file | `safety_tests` (4 tests) | pass (failed before) | refused, victims kept | `src/manifest.rs` |
| Changed files not deleted | Victims re-hashed before removal | `manifest_tests::execute_manifest_verifies_hash_before_deleting`, `safety_tests::direct_delete_reverifies_*` | pass | skipped | `src/manifest.rs` |
| Symlink loops / permissions | Scan completes; skips counted | `safety_tests` (Unix) | pass | counted | `src/walk.rs` |
| Keep strategies | first/oldest/newest/shortest-path | `cargo test --test keep_strategy_tests` | pass | 6/6 | `src/keep.rs` |
| Manifest round trip | Write, read, execute, dry-run | `manifest_tests`, clean-clone workflow | pass | as expected | `src/manifest.rs` |

## Classification proposal

**Partially Verified** — the documented scope, including the safety
contract, is tested and passing on macOS after fixing a critical
data-loss defect; not yet verified: CI on Linux/Windows and the MSRV
job for the audit branch (after push), and Windows identity detection.
