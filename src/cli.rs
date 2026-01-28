use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sieve")]
#[command(about = "Secret Leak Tripwire", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Disable TUI and output JSON or text to stdout (suitable for CI)
    #[arg(long, global = true)]
    pub no_tui: bool,

    /// Output format when TUI is disabled (human or json)
    #[arg(long, global = true, default_value = "human")]
    pub format: String,

    /// Fail on Medium severity issues
    #[arg(long, global = true)]
    pub strict: bool,

    /// Show detailed info for all findings (in non-TUI mode)
    #[arg(long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan for secrets
    Scan {
        /// Scan staged files (git diff --cached)
        #[arg(long)]
        staged: bool,

        /// Scan a specific path (recursive)
        #[arg(long)]
        path: Option<String>,

        /// Scan changes since a specific git reference
        #[arg(long)]
        since: Option<String>,
    },
    /// Manage baseline (ignore known secrets)
    Baseline {
        /// Generate a baseline file from current findings
        #[arg(long)]
        generate: bool,

        /// Check against baseline (only report new findings)
        #[arg(long)]
        check: bool,
    },
    /// Check for secrets with advanced options (repair, fix)
    Check {
        /// Full recursive scan (ignores git status)
        #[arg(long)]
        full: bool,

        /// Automatically repair findings
        #[arg(long)]
        repair: bool,

        /// Fix a specific finding by index
        #[arg(long)]
        fix: Option<usize>,
    },
}
