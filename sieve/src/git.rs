use anyhow::{Context, Result};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitLine {
    pub path: String,
    pub line_num: usize,
    pub content: String,
}

pub fn check_git_installed() -> Result<()> {
    Command::new("git")
        .arg("--version")
        .output()
        .context("Git is not installed or not in PATH")?;
    Ok(())
}

pub fn get_staged_diff() -> Result<Vec<GitLine>> {
    let output = Command::new("git")
        .args(&[
            "diff",
            "--cached",
            "--unified=0",
            "--no-color",
            "--no-ext-diff",
        ])
        .output()
        .context("Failed to run git diff")?;

    if !output.status.success() {
        // Could be not a git repo
        return Ok(vec![]);
    }

    let diff = String::from_utf8_lossy(&output.stdout);
    parse_diff(&diff)
}

pub fn get_since_diff(ref_spec: &str) -> Result<Vec<GitLine>> {
    let range = format!("{}..HEAD", ref_spec);
    let output = Command::new("git")
        .args(&["diff", &range, "--unified=0", "--no-color", "--no-ext-diff"])
        .output()
        .context("Failed to run git diff for range")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Git diff command failed for range: {}",
            range
        ));
    }

    let diff = String::from_utf8_lossy(&output.stdout);
    parse_diff(&diff)
}

fn parse_diff(diff: &str) -> Result<Vec<GitLine>> {
    let mut lines = Vec::new();
    let mut current_file = String::new();
    let mut current_line_num = 0;

    // Simple state machine
    for line in diff.lines() {
        if line.starts_with("diff --git") {
            // New file header, reset
            current_file = String::new();
        } else if line.starts_with("+++ b/") {
            current_file = line.trim_start_matches("+++ b/").to_string();
        } else if line.starts_with("--- a/") {
            // ignore
        } else if line.starts_with("@@") {
            // Hunk header: @@ -14,0 +15,2 @@
            // We need the start line of the '+' (added) section.
            // Format is usually @@ -start,count +start,count @@
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(added_part) = parts.get(2) {
                // +15,2 or +15
                let clean = added_part.trim_start_matches('+');
                let nums: Vec<&str> = clean.split(',').collect();
                if let Some(start_str) = nums.get(0) {
                    current_line_num = start_str.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with('+') && !line.starts_with("+++") {
            if !current_file.is_empty() && current_line_num > 0 {
                // It's an added line
                lines.push(GitLine {
                    path: current_file.clone(),
                    line_num: current_line_num,
                    content: line[1..].to_string(), // remove the '+'
                });
                current_line_num += 1;
            }
        } else if !line.starts_with('-') && !line.starts_with('\\') {
            // Context line (shouldn't happen much with unified=0 but git sometimes gives one)
            // or just random output. With unified=0 we mostly get hunk headers and changes.
            // If it's a context line, we increment line number but don't capture.
            if !current_file.is_empty() && current_line_num > 0 {
                // Actually with unified=0 we assume mostly packed changes.
                // If there's context, git output usually starts with space.
                if line.starts_with(' ') {
                    current_line_num += 1;
                }
            }
        }
    }

    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_simple() {
        let diff_output = r#"diff --git a/src/main.rs b/src/main.rs
index 8f3a123..1234567 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -10,0 +11,2 @@ use std::io;
+const SECRET: &str = "12345";
+fn main() {
"#;
        let lines = parse_diff(diff_output).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].path, "src/main.rs");
        assert_eq!(lines[0].line_num, 11);
        assert_eq!(lines[0].content, "const SECRET: &str = \"12345\";");

        assert_eq!(lines[1].line_num, 12);
        assert_eq!(lines[1].content, "fn main() {");
    }

    #[test]
    fn test_parse_diff_multiple_files() {
        let diff_output = r#"diff --git a/foo.txt b/foo.txt
index ...
--- a/foo.txt
+++ b/foo.txt
@@ -1,0 +1 @@
+foo content
diff --git a/bar.txt b/bar.txt
index ...
--- a/bar.txt
+++ b/bar.txt
@@ -5 +5,2 @@
-old
+new line 1
+new line 2
"#;
        let lines = parse_diff(diff_output).unwrap();
        assert_eq!(lines.len(), 3);

        assert_eq!(lines[0].path, "foo.txt");
        assert_eq!(lines[0].line_num, 1);

        assert_eq!(lines[1].path, "bar.txt");
        assert_eq!(lines[1].line_num, 5); // Start of + hunk
        assert_eq!(lines[1].content, "new line 1");

        assert_eq!(lines[2].path, "bar.txt");
        assert_eq!(lines[2].line_num, 6);
    }
}
