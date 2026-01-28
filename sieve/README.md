# Sieve (Secret Sieve)

A TUI-first secret leak tripwire for developers. Blocks new secrets before they land in git.

## Features
- **TUI-First Experience:** Interactive terminal UI to review, ignore, or fix leaks.
- **Git Integration:** Scans `git diff --cached` (staged files) for speed.
- **Heuristics:** Uses entropy, keywords, and format detection to score findings.
- **Baseline:** Supports a `.sieve.baseline.json` to ignore legacy secrets.
- **Safe:** Redacts secrets in all outputs (UI and JSON).

## Installation

### Via Cargo (Rust)
```bash
cargo install --path .
```

### Via NPM (Wrapper)
```bash
# Runs the binary (requires release assets in real scenario)
npx sieve-cli scan --staged
```

## Usage

### 1. Scan Staged Changes (Pre-commit)
```bash
sieve scan --staged
```
Launches the TUI if secrets are found.

### 2. Scan Directory
```bash
sieve scan --path ./src
```

### 3. CI Mode (JSON Output)
```bash
sieve scan --staged --no-tui --format json
```

### 4. Baselines
If you have existing secrets you can't fix yet:
1. Run scan.
2. Press `g` in the TUI on a finding to add it to the baseline.
3. Or verify strictness: `sieve baseline --check`

## Pre-commit Hook

Add this to `.git/hooks/pre-commit`:

```bash
#!/bin/sh
# Redirect input to TTY to allow TUI interaction
exec < /dev/tty
sieve scan --staged
```

Make it executable:
```bash
chmod +x .git/hooks/pre-commit
```

## Configuration

Sieve looks for `.sieve.baseline.json` in the current directory.
Default ignores: `node_modules`, `target`, `dist`, `.git`, `vendor`.

## Development

```bash
cargo run -- scan --staged
```

## Release

This project uses GitHub Actions for automated releases.
1. Update the version in `Cargo.toml` and `npm/package.json`.
2. Push a new tag starting with `v` (e.g., `v0.1.0`).
3. The CI workflow will:
   - Build binaries for Linux, Windows, and macOS (x64 and ARM64).
   - Package them into `tar.gz` archives.
   - Create a GitHub Release and upload the assets.
   - The npm package's `install.js` script downloads the appropriate binary from these release assets.

