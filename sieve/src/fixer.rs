use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

pub struct FixResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Replacement {
    pub line: usize,      // 1-based
    pub start_col: usize, // 1-based, inclusive
    pub end_col: usize,   // 1-based, exclusive
    pub new_text: String,
}

pub fn apply_placeholder(_secret: &str) -> String {
    "REDACTED_SECRET".to_string()
}

pub fn fix_file(file_path: &str, replacements: Vec<Replacement>) -> Result<FixResult> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Ok(FixResult {
            success: false,
            message: format!("File not found: {}", file_path),
        });
    }

    let mut content = String::new();
    File::open(path)
        .context("Failed to open file")?
        .read_to_string(&mut content)
        .context("Failed to read file")?;

    // Detect line ending (simple heuristic: first occurrence)
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let ends_with_newline = content.ends_with('\n');

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Sort replacements: Line descending, then Column descending
    let mut replacements = replacements;
    replacements.sort_by(|a, b| {
        b.line
            .cmp(&a.line)
            .then_with(|| b.start_col.cmp(&a.start_col))
    });

    for replace in replacements {
        let line_idx = replace.line - 1; // 0-based
        if line_idx >= lines.len() {
            continue;
        }

        let line = &lines[line_idx];
        let chars: Vec<char> = line.chars().collect();

        // 0-based column indices
        let start_idx = replace.start_col.saturating_sub(1);
        let end_idx = replace.end_col.saturating_sub(1);

        if start_idx > chars.len() || end_idx > chars.len() || start_idx > end_idx {
            continue;
        }

        let mut new_line = String::new();
        new_line.extend(&chars[..start_idx]);
        new_line.push_str(&replace.new_text);
        new_line.extend(&chars[end_idx..]);

        lines[line_idx] = new_line;
    }

    let mut new_content = lines.join(newline);
    if ends_with_newline {
        new_content.push_str(newline);
    }

    // Atomic write (Write to temp, then rename)
    let tmp_path = path.with_extension("tmp");
    {
        let mut tmp_file = File::create(&tmp_path).context("Failed to create temp file")?;
        tmp_file
            .write_all(new_content.as_bytes())
            .context("Failed to write temp file")?;
    } // Ensure file is closed before rename

    // Windows rename workaround
    if fs::rename(&tmp_path, path).is_err() {
        // Attempt to remove original and retry rename
        let _ = fs::remove_file(path);
        fs::rename(&tmp_path, path).context("Failed to rename temp file to target")?;
    }

    Ok(FixResult {
        success: true,
        message: "File fixed successfully".to_string(),
    })
}
