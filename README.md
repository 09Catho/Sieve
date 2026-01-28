# Sieve (Secret Sieve)

A TUI-first secret leak tripwire for developers. Blocks new secrets before they land in git.

## Features
- **TUI-First Experience:** Interactive terminal UI to review, ignore, or fix leaks.
- **Git Integration:** Scans `git diff --cached` (staged files) for speed.
- **Deep Scan:** Recursive directory scanning with `.gitignore` support.
- **Auto-Repair:** Automatically fix secrets by replacing them with placeholders.
- **Heuristics:** Uses entropy, keywords, and format detection to score findings.
- **Baseline:** Supports a `.sieve.baseline.json` to ignore legacy secrets.
- **Safe:** Redacts secrets in all outputs (UI and JSON).

## Installation

### Via NPM (Recommended)
This installs the pre-compiled binary for your OS (Windows, Linux, macOS).

```bash
npm install -g sieve-secrets
```

> Note: The npm package runs a small **postinstall** step that only downloads the prebuilt Rust binary from the GitHub release. If you are in a locked-down environment, set `SIEVE_SKIP_POSTINSTALL=1` to skip it and build from source instead.

Run immediately without installing:
```bash
npx sieve-secrets check --full
```

### Via Cargo (Rust)
```bash
cargo install --path .
```

## Usage

### 1. Quick Check (Pre-commit)
Scans only staged files (`git diff --cached`). Ideal for git hooks.
```bash
sieve check
```

### 2. Full Project Scan
Recursively scans the current directory, respecting `.gitignore`.
```bash
sieve check --full
```

### 3. Automatic Repair
Automatically replaces all found secrets with `REDACTED_SECRET`.
```bash
sieve check --full --repair
```

### 4. Interactive Mode (TUI)
Running `sieve check` automatically launches the interactive TUI **if secrets are found**. If no secrets are detected, the tool exits silently (success).

**TUI Controls:**
- **Navigation:** `Up`/`Down` Arrow keys
- **Actions:**
  - `r`: **Repair** (Auto-fix the selected finding with placeholders)
  - `g`: **Ignore** (Add to baseline/allowlist)
  - `c`: **Copy** finding details to clipboard
  - `s`: **Switch** mode (Strict/Normal)
  - `q`: **Quit**

### 5. CI Mode (JSON Output)
For build pipelines, disable the TUI and output JSON.
```bash
sieve check --full --no-tui --format json
```

## Pre-commit Hook

Add this to `.git/hooks/pre-commit`:

```bash
#!/bin/sh
# Redirect input to TTY to allow TUI interaction
exec < /dev/tty
sieve check
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
cargo run -- check --full
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
