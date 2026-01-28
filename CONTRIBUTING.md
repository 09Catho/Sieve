# Contributing to Sieve

Thank you for your interest in contributing to Sieve! We welcome contributions, especially new secret detection patterns and improvements to the TUI.

## 🚀 Getting Started

Sieve is built with **Rust** (core logic) and wrapped in **Node.js** for easy distribution.

### Prerequisites
- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (optional, only for checking the npm wrapper)

### Setup
1. Clone the repository:
   ```bash
   git clone https://github.com/09Catho/Sieve.git
   cd Sieve
   ```
2. Build the project:
   ```bash
   cargo build
   ```
3. Run the tool locally:
   ```bash
   # Scan the current directory
   cargo run -- check --full
   ```

---

## 🏗️ Project Structure

- **`src/scanner.rs`**: The core detection engine. Contains all Regex patterns (`FMT_*`) and scoring logic.
- **`src/fixer.rs`**: Handles the logic for `sieve check --repair`.
- **`src/ui.rs`**: The TUI implementation using `ratatui`.
- **`src/git.rs`**: Logic for parsing `git diff` output.
- **`npm/`**: The Node.js wrapper that downloads the binary.

---

## ➕ Adding a New Secret Pattern

If Sieve is missing a detection rule (e.g., a specific API key format), follow these steps:

1.  **Open `src/scanner.rs`**.
2.  Add your Regex pattern to the `lazy_static!` block using the `FMT_` prefix convention:
    ```rust
    static ref FMT_NEW_SERVICE: Regex = Regex::new(r"(?i)new_service_key_[a-zA-Z0-9]+").unwrap();
    ```
3.  Update the `scan_line()` function to check your new pattern:
    ```rust
    if FMT_NEW_SERVICE.is_match(content) {
        score = 90; // High confidence
        rule_id = "NEW_SERVICE_KEY".to_string();
        // ... set reasons and extracted_value
    }
    ```
4.  **Add a Test:** Scroll to the `mod tests` module at the bottom of `src/scanner.rs` and add a unit test to verify your pattern works and doesn't flag false positives.

---

## ✅ Testing

Before submitting a Pull Request, ensure all tests pass:

```bash
cargo test
```

If you modified the TUI, manually verify it by running:
```bash
cargo run -- check --full
```

## 📦 Release Process (Maintainers Only)

Releases are automated via GitHub Actions.
1. Bump the version in `Cargo.toml` and `npm/package.json`.
2. Push a new tag: `git tag v0.X.X && git push origin v0.X.X`.
3. CI will build binaries and publish the release.
