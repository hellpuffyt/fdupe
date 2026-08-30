# Contributing

Thanks for considering a contribution to `fdupe`.

## Development setup

You need a Rust toolchain matching or newer than the `rust-version` in
`Cargo.toml`.

```sh
cargo build
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

All four must pass before a pull request will be merged.

## Guidelines

- No `unsafe` code; the crate forbids it (`unsafe_code = "forbid"`).
- Keep dependencies light — every new dependency raises the bar for
  review and can raise the minimum supported Rust version.
- Add tests for new behavior. Prefer `tempfile`-built directory trees
  over mocking the filesystem.
- Anything that touches the delete path (`src/delete.rs`,
  `src/manifest.rs`) needs tests that prove data safety: no strategy may
  ever delete every copy of a group, and manifest execution must
  re-verify file contents before removing them.
- Run `cargo fmt` before committing; CI enforces formatting.

## Reporting bugs

Please include: your OS, the `fdupe --version` output, the command you
ran, and, if possible, a minimal directory tree that reproduces the
issue.
