<p align="center">
  <img src="https://raw.githubusercontent.com/basebin/omnitype/main/.github/assets/thumbnail.png" alt="omnitype" width="100%">
</p>

# Omnitype

[![CI](https://github.com/bniladridas/omnitype/workflows/CI/badge.svg)](https://github.com/bniladridas/omnitype/actions)
[![Release](https://github.com/bniladridas/omnitype/actions/workflows/release.yml/badge.svg)](https://github.com/bniladridas/omnitype/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.89%2B-orange.svg)](https://www.rust-lang.org)

Experimental type-checker for Python and dynamic languages.

## Features

- **Check**: Parse Python files and report diagnostics (e.g., missing annotations).
- **Fix**: Add missing `: Any` and `-> Any` annotations automatically.
- **Trace**: Runtime type tracing for function calls.
- **TUI**: Terminal UI for file analysis and error navigation.

## Usage

```bash
# Check files
cargo run -- check <path>

# Fix annotations
cargo run -- fix <path> --in-place

# Launch TUI
cargo run

# Run tests
cargo test
```

## Development

```bash
cargo check
cargo clippy -- -D warnings
cargo fmt
```

## Conventional Commits

This project uses conventional commit standards to ensure consistent and meaningful commit messages.

### Setup

To enable the commit message hook that enforces conventional commits:

```bash
cp scripts/commit-msg .git/hooks/commit-msg
chmod +x .git/hooks/commit-msg
```

The hook checks that commit messages:
- Start with a type like `feat:`, `fix:`, `docs:`, `style:`, `refactor:`, `test:`, `chore:`, `perf:`, `ci:`, `build:`, `revert:`
- Are lowercase
- Have a first line ≤60 characters

### History Cleanup

To rewrite existing commit messages in the git history to conform to the standards (lowercase and truncated to 60 chars):

```bash
./scripts/rewrite_msg.sh
```

After running, force-push to update the remote repository:

```bash
git push --force-with-lease
```

The git history has been rewritten to follow these standards.

— @omnitype by [harper](https://github.com/harpertoken)
