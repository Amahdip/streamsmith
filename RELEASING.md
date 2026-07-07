# Releasing streamsmith

Releases are automated with [`cargo-release`](https://github.com/crate-ci/cargo-release).
One command bumps the version, commits, tags, publishes to crates.io, and pushes —
atomically, so the crate version, the git tag, and the GitHub release can never
drift apart (which is exactly what a manual `git tag` once got wrong).

## One-time setup

```bash
cargo install cargo-release          # the tool
cargo login                          # a crates.io token (once per machine)
```

## Cutting a release

From a clean `main` that's up to date with `origin`:

```bash
cargo release patch --execute        # 0.1.1 -> 0.1.2  (bug fixes)
cargo release minor --execute        # 0.1.1 -> 0.2.0  (new features, e.g. --dash)
cargo release major --execute        # 0.1.1 -> 1.0.0  (breaking changes)
```

Drop `--execute` (or use `--dry-run`) first to see exactly what it will do
without changing anything.

`cargo release` will, in order:

1. Refuse to proceed if the tree is dirty, you're not on `main`, or the version
   already exists on crates.io.
2. Bump `version` in `Cargo.toml`, commit as `chore: release X.Y.Z`.
3. Create and push the `vX.Y.Z` tag and the commit.
4. Publish the crate to crates.io.

Pushing the tag triggers `.github/workflows/release.yml`, which **re-checks the
tag matches `Cargo.toml`**, creates the GitHub Release, and attaches prebuilt
binaries for Linux (x64/arm64), macOS (Intel/Apple Silicon), and Windows.

## Rules that bite

- **crates.io versions are immutable** — never reuse a number. `cargo release`
  enforces this; a manual `cargo publish` will error with "already exists".
- **The tag must equal the `Cargo.toml` version.** The release workflow now
  aborts on a mismatch, so a stray tag can't produce a phantom release.
- If you ever ship something broken: `cargo yank --version X.Y.Z` hides it from
  new dependents (it does not delete it or break existing users).
