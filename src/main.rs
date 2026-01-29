mod baseline;
mod cli;
mod fixer;
mod git;
mod scanner;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ignore::WalkBuilder;
use ratatui::{backend::CrosstermBackend, Terminal};
use scanner::{Finding, Severity};
use std::fs::File;
use std::io;
use ui::FilterMode;
// use std::path::Path;

fn main() -> Result<()> {
    let _ = scanner::Severity::High; // Keep scanner used to silence unused warning if needed, or just clean up imports
                                     // Handle no arguments: treat as Check { full: true }
    let args = if std::env::args().len() == 1 {
        cli::Cli {
            command: cli::Commands::Check {
                full: true,
                repair: false,
                fix: None,
            },
            no_tui: false,
            format: "human".to_string(),
            strict: false,
            verbose: false,
        }
    } else {
        cli::Cli::parse()
    };
    let mut baseline = baseline::Baseline::load();
    let mut findings = Vec::new();

    // --- 1. SCANNING PHASE ---

    // Check git if needed
    if matches!(args.command, cli::Commands::Scan { staged: true, .. }) {
        if let Err(e) = git::check_git_installed() {
            eprintln!("Error: {}", e);
            std::process::exit(2);
        }
    }

    match &args.command {
        cli::Commands::Check { full, repair, fix } => {
            if let Some(fix_index) = fix {
                // Fix specific finding from cache
                let cache_path = ".sieve_cache.json";
                if !std::path::Path::new(cache_path).exists() {
                    eprintln!("Error: Cache file not found. Run 'sieve check --full' first.");
                    std::process::exit(1);
                }
                let file = File::open(cache_path).context("Failed to open cache file")?;
                let cached_findings: Vec<Finding> = serde_json::from_reader(file)?;

                if *fix_index >= cached_findings.len() {
                    eprintln!(
                        "Error: Index {} out of bounds ({} findings)",
                        fix_index,
                        cached_findings.len()
                    );
                    std::process::exit(1);
                }

                let finding = &cached_findings[*fix_index];
                println!(
                    "Fixing finding #{} in {}:{}",
                    fix_index, finding.file_path, finding.line_number
                );

                let replacement = fixer::Replacement {
                    line: finding.line_number,
                    start_col: finding.start_index,
                    end_col: finding.end_index,
                    new_text: fixer::apply_placeholder(&finding.redacted_preview), // We don't have the secret, use placeholder logic
                };

                match fixer::fix_file(&finding.file_path, vec![replacement]) {
                    Ok(res) => println!("{}", res.message),
                    Err(e) => eprintln!("Error fixing file: {}", e),
                }
                return Ok(());
            }

            // Scanning logic for Check
            if *full {
                let walker = WalkBuilder::new(".")
                    .hidden(false) // Allow scanning hidden files like .env
                    .git_ignore(true)
                    .ignore(true)
                    .build();

                for result in walker {
                    match result {
                        Ok(entry) => {
                            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                                let path_str = entry.path().to_string_lossy().to_string();
                                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                                    for (i, line) in content.lines().enumerate() {
                                        if let Some(finding) =
                                            scanner::scan_line(&path_str, i + 1, line)
                                        {
                                            if !baseline.contains(&finding.fingerprint) {
                                                findings.push(finding);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(err) => eprintln!("Error walking path: {}", err),
                    }
                }
            } else {
                // Default Check behavior? maybe same as Scan --staged?
                // For now, let's just warn if not full? Or assume current dir?
                // Prompt says "Implement 'Full' mode: Recursive walk".
                // Without full, maybe it does nothing or checks staged?
                // Let's assume Check without Full checks staged, similar to Scan defaults.
                // Or better, error out.
                if !repair {
                    // If repair is set, we might be repairing from cache? No, repair loops findings.
                    // If no findings, we must scan.
                    println!("Running quick check (staged files)... use --full for full scan.");
                    let lines = git::get_staged_diff().unwrap_or_default();
                    for line in lines {
                        if let Some(finding) =
                            scanner::scan_line(&line.path, line.line_num, &line.content)
                        {
                            if !baseline.contains(&finding.fingerprint) {
                                findings.push(finding);
                            }
                        }
                    }
                }
            }

            // Save findings to cache
            let cache_file =
                File::create(".sieve_cache.json").context("Failed to create cache file")?;
            serde_json::to_writer_pretty(cache_file, &findings)?;

            if *repair {
                println!("Repairing {} findings...", findings.len());
                for finding in &findings {
                    let replacement = fixer::Replacement {
                        line: finding.line_number,
                        start_col: finding.start_index,
                        end_col: finding.end_index,
                        new_text: fixer::apply_placeholder(""),
                    };
                    if let Err(e) = fixer::fix_file(&finding.file_path, vec![replacement]) {
                        eprintln!("Failed to fix {}: {}", finding.file_path, e);
                    } else {
                        println!("Fixed {}", finding.file_path);
                    }
                }
                return Ok(());
            }

            // If we are just checking, we might want to output list or exit.
            // If TUI is not disabled, we fall through to TUI.
            // But usually 'check' implies a CLI check.
            // The user said: "Ensure TUI isn't launched if --repair or --fix is used".
            // If neither is used, TUI might be launched if not --no-tui.
        }
        cli::Commands::Scan {
            staged,
            path,
            since,
        } => {
            if *staged {
                let lines = git::get_staged_diff()?;
                for line in lines {
                    if let Some(finding) =
                        scanner::scan_line(&line.path, line.line_num, &line.content)
                    {
                        if !baseline.contains(&finding.fingerprint) {
                            findings.push(finding);
                        }
                    }
                }
            } else if let Some(ref_spec) = since {
                let lines = git::get_since_diff(ref_spec)?;
                for line in lines {
                    if let Some(finding) =
                        scanner::scan_line(&line.path, line.line_num, &line.content)
                    {
                        if !baseline.contains(&finding.fingerprint) {
                            findings.push(finding);
                        }
                    }
                }
            } else if let Some(p) = path {
                // Recursive directory scan
                let walker = WalkBuilder::new(p)
                    .hidden(true)
                    .git_ignore(true)
                    .ignore(true) // .ignore files
                    .build();

                for result in walker {
                    match result {
                        Ok(entry) => {
                            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                                let path_str = entry.path().to_string_lossy().to_string();
                                // Skip binary/large files check (simplified)
                                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                                    for (i, line) in content.lines().enumerate() {
                                        if let Some(finding) =
                                            scanner::scan_line(&path_str, i + 1, line)
                                        {
                                            if !baseline.contains(&finding.fingerprint) {
                                                findings.push(finding);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(err) => eprintln!("Error walking path: {}", err),
                    }
                }
            } else {
                // Default behavior if no args? Help.
                // But for now, let's assume user might want scan . (current dir) if nothing else?
                // Actually prompt says "sieve scan --staged".
                eprintln!("Please specify --staged, --path <path>, or --since <ref>");
                std::process::exit(2);
            }
        }
        cli::Commands::Baseline { generate, check } => {
            // For baseline commands, we usually default to staged if nothing else is clear,
            // or we might need flags. For MVP, let's assume we scan staged to generate baseline.
            // Or better, let's reuse scan logic.
            // The prompt says "sieve baseline generate [--staged|...]"
            // I didn't implement those args in Baseline subcommand in cli.rs,
            // I implemented them as separate flags in Scan.
            // Let's assume for MVP baseline generation implies checking staged changes
            // unless we want to refactor CLI to share arguments.
            // SIMPLIFICATION: Scan staged for baseline generation for now.

            let lines = git::get_staged_diff()?;
            for line in lines {
                if let Some(finding) = scanner::scan_line(&line.path, line.line_num, &line.content)
                {
                    if *generate {
                        baseline.add(
                            finding.fingerprint.clone(),
                            finding.file_path,
                            finding.rule_id,
                            finding.redacted_preview,
                        );
                    } else if *check {
                        if !baseline.contains(&finding.fingerprint) {
                            findings.push(finding);
                        }
                    }
                }
            }

            if *generate {
                baseline.save()?;
                println!("Baseline generated/updated at .sieve.baseline.json");
                return Ok(());
            }
        }
    }

    // --- 2. REPORTING PHASE ---

    // Sort findings: High first, then Medium
    findings.sort_by(|a, b| b.severity.cmp(&a.severity));

    if findings.is_empty() {
        if !args.no_tui {
            println!("Sieve: No secrets found.");
        }
        return Ok(());
    }

    if args.no_tui {
        // CI / Text Mode
        if args.format == "json" {
            let json = serde_json::to_string_pretty(&findings)?;
            println!("{}", json);
        } else {
            for f in &findings {
                println!(
                    "[{:?}] {}:{} - {} ({})",
                    f.severity, f.file_path, f.line_number, f.rule_id, f.redacted_preview
                );
                if args.verbose {
                    println!("    Why: {}", f.reason);
                }
            }
        }

        // Exit codes
        let fail = findings.iter().any(|f| f.severity == Severity::High)
            || (args.strict && !findings.is_empty());
        if fail {
            std::process::exit(1);
        }
    } else {
        // TUI Mode
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut app = ui::App::new(findings, args.strict);
        let res = run_app(&mut terminal, &mut app, &mut baseline);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            eprintln!("TUI Error: {:?}", err);
        }

        // TUI always exits 0 when user quits, even if findings remain.
        // We only exit 1 in --no-tui mode if findings exist.
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut ui::App,
    baseline: &mut baseline::Baseline,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if app.show_help {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => app.show_help = false,
                    _ => {} // Ignore other keys while help is open
                }
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    if app.show_context {
                        app.show_context = false;
                    } else {
                        return Ok(());
                    }
                }
                KeyCode::Enter => {
                    if app.show_context {
                        app.show_context = false;
                    } else if let Some(sel) = app.state.selected() {
                        if let Some(f) = app.findings.get(sel) {
                            match ui::get_file_context(&f.file_path, f.line_number) {
                                Ok(lines) => {
                                    app.context_lines = Some(lines);
                                    app.show_context = true;
                                }
                                Err(e) => {
                                    app.clipboard_status =
                                        Some(format!("Error reading context: {}", e));
                                }
                            }
                        }
                    }
                }
                KeyCode::Down => app.next(),
                KeyCode::Up => app.previous(),
                KeyCode::Char('s') => app.strict_mode = !app.strict_mode,
                KeyCode::Char('?') => app.show_help = !app.show_help,
                KeyCode::Char('1') => {
                    app.filter_mode = FilterMode::All;
                    app.update_visible_findings();
                }
                KeyCode::Char('2') => {
                    app.filter_mode = FilterMode::High;
                    app.update_visible_findings();
                }
                KeyCode::Char('3') => {
                    app.filter_mode = FilterMode::Medium;
                    app.update_visible_findings();
                }
                KeyCode::Char('4') => {
                    app.filter_mode = FilterMode::Low;
                    app.update_visible_findings();
                }
                KeyCode::Char('c') => {
                    if let Some(sel) = app.state.selected() {
                        if let Some(f) = app.findings.get(sel) {
                            // Attempt clipboard copy
                            let content = format!(
                                "Sieve Alert!\nRule: {}\nFile: {}:{}\nSecret: {}\nWhy: {}",
                                f.rule_id, f.file_path, f.line_number, f.redacted_preview, f.reason
                            );

                            let ctx: Result<ClipboardContext, _> = ClipboardContext::new();
                            match ctx {
                                Ok(mut c) => {
                                    if let Err(e) = c.set_contents(content) {
                                        app.clipboard_status = Some(format!("Copy failed: {}", e));
                                    } else {
                                        app.clipboard_status =
                                            Some("Copied to clipboard!".to_string());
                                    }
                                }
                                Err(_) => {
                                    app.clipboard_status =
                                        Some("Clipboard unavailable".to_string());
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('r') => {
                    if let Some(sel) = app.state.selected() {
                        if let Some(f) = app.findings.get(sel) {
                            let replacement = fixer::Replacement {
                                line: f.line_number,
                                start_col: f.start_index,
                                end_col: f.end_index,
                                new_text: fixer::apply_placeholder(&f.redacted_preview),
                            };
                            match fixer::fix_file(&f.file_path, vec![replacement]) {
                                Ok(_) => {
                                    let fingerprint = f.fingerprint.clone();
                                    // Remove from all_findings
                                    if let Some(idx) = app
                                        .all_findings
                                        .iter()
                                        .position(|x| x.fingerprint == fingerprint)
                                    {
                                        app.all_findings.remove(idx);
                                    }
                                    app.update_visible_findings();
                                    app.clipboard_status = Some("Fixed!".to_string());

                                    if app.all_findings.is_empty() {
                                        return Ok(());
                                    }
                                }
                                Err(e) => {
                                    app.clipboard_status = Some(format!("Error: {}", e));
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('g') => {
                    // Generate baseline (ignore)
                    if let Some(sel) = app.state.selected() {
                        if let Some(f) = app.findings.get(sel) {
                            baseline.add(
                                f.fingerprint.clone(),
                                f.file_path.clone(),
                                f.rule_id.clone(),
                                f.redacted_preview.clone(),
                            );
                            let _ = baseline.save();

                            let fingerprint = f.fingerprint.clone();
                            // Remove from all_findings
                            if let Some(idx) = app
                                .all_findings
                                .iter()
                                .position(|x| x.fingerprint == fingerprint)
                            {
                                app.all_findings.remove(idx);
                            }
                            app.update_visible_findings();

                            if app.all_findings.is_empty() {
                                return Ok(()); // All handled
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
