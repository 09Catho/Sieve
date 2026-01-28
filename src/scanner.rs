use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub score: u8,
    pub file_path: String,
    pub line_number: usize,
    pub start_index: usize,
    pub end_index: usize,
    #[serde(skip)] // Don't serialize raw content
    #[allow(dead_code)]
    #[allow(dead_code)]
    pub raw_content: String,
    pub redacted_preview: String,
    pub fingerprint: String,
    pub reason: String,
}

lazy_static! {
    static ref SUSPECT_KEYS: Regex = Regex::new(r"(?i)(secret|token|apikey|api_key|password|passwd|private_key|client_secret|auth_token|access_token)").unwrap();

    // High Signal Formats (Score: High)
    static ref FMT_PRIVATE_KEY: Regex = Regex::new(r"-----BEGIN (RSA|EC|OPENSSH|PGP) PRIVATE KEY-----").unwrap();
    static ref FMT_AWS: Regex = Regex::new(r"(?i)(AKIA|ASIA)[0-9A-Z]{16}").unwrap();
    static ref FMT_BEARER: Regex = Regex::new(r"(?i)Authorization:\s*Bearer\s+([a-zA-Z0-9_\-\.]+)").unwrap();
    static ref FMT_SLACK: Regex = Regex::new(r"xox[baprs]-[a-zA-Z0-9\-]+").unwrap();
    static ref FMT_STRIPE: Regex = Regex::new(r"(?i)sk_live_[0-9a-zA-Z]+").unwrap();
    static ref FMT_GENERIC_KEYLIKE: Regex = Regex::new(r"(?i)(sk-[a-zA-Z0-9]{20,})").unwrap();

    // Assignment patterns
    // Matches: key = "value" or key: "value" or key: 'value'
    // Group 2: Key, Group 4: Value
    static ref ASSIGNMENT: Regex = Regex::new(r#"(?i)(const|let|var)?\s*([a-z0-9_]+)\s*[:=]\s*(["'])([^"']+)(["'])"#).unwrap();

    // Dummies to ignore
    static ref DUMMY_VALUES: Regex = Regex::new(r"(?i)(changeme|xxx|test|placeholder|example|your-token|your_token|undefined|null|true|false)").unwrap();
}

pub fn redact(s: &str) -> String {
    if s.len() < 8 {
        return "<redacted>".to_string();
    }
    let start = &s[0..3];
    let end = &s[s.len().saturating_sub(3)..];
    format!("{}...{}", start, end)
}

fn calculate_entropy(s: &str) -> f32 {
    let mut counts = std::collections::HashMap::new();
    let total = s.len() as f32;
    if total == 0.0 {
        return 0.0;
    }

    for c in s.chars() {
        *counts.entry(c).or_insert(0) += 1;
    }

    let mut entropy = 0.0;
    for &count in counts.values() {
        let p = count as f32 / total;
        entropy -= p * p.log2();
    }
    entropy
}

fn is_test_file(path: &str) -> bool {
    let p = path.to_lowercase();
    p.contains("test")
        || p.contains("spec")
        || p.contains("mock")
        || p.contains("fixture")
        || p.contains("example")
}

pub fn scan_line(path: &str, line_num: usize, content: &str) -> Option<Finding> {
    // Optimization: Skip very long lines (minified code)
    if content.len() > 1000 {
        return None;
    }

    let mut score: i32 = 0;
    let mut reasons = Vec::new();
    let mut rule_id = "UNKNOWN".to_string();
    let mut extracted_value = String::new();
    let mut found = false;
    let mut match_range = (0, 0);

    // 1. Direct Regex High-Signal Matches
    if let Some(mat) = FMT_PRIVATE_KEY.find(content) {
        score = 100;
        rule_id = "PRIVATE_KEY_BLOCK".to_string();
        reasons.push("Found Private Key block".to_string());
        extracted_value = "PRIVATE KEY CONTENT".to_string();
        match_range = (mat.start(), mat.end());
        found = true;
    } else if let Some(caps) = FMT_AWS.captures(content) {
        score = 90;
        rule_id = "AWS_ACCESS_KEY".to_string();
        reasons.push("Found AWS Access Key ID".to_string());
        if let Some(m) = caps.get(0) {
            extracted_value = m.as_str().to_string();
            match_range = (m.start(), m.end());
        }
        found = true;
    } else if let Some(caps) = FMT_BEARER.captures(content) {
        score = 80;
        rule_id = "BEARER_TOKEN".to_string();
        reasons.push("Found Bearer Auth header".to_string());
        if let Some(m) = caps.get(1) {
            extracted_value = m.as_str().to_string();
            match_range = (m.start(), m.end());
        }
        found = true;
    } else if let Some(mat) = FMT_SLACK.find(content) {
        score = 90;
        rule_id = "SLACK_TOKEN".to_string();
        reasons.push("Found Slack-like token".to_string());
        extracted_value = mat.as_str().trim().to_string();
        match_range = (mat.start(), mat.end());
        found = true;
    } else if let Some(mat) = FMT_STRIPE.find(content) {
        score = 90;
        rule_id = "STRIPE_KEY".to_string();
        reasons.push("Found Stripe Live key".to_string());
        extracted_value = mat.as_str().trim().to_string();
        match_range = (mat.start(), mat.end());
        found = true;
    }

    // 2. Heuristic Context Scanning (Key/Value)
    if !found {
        if let Some(caps) = ASSIGNMENT.captures(content) {
            let key = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let val_match = caps.get(4);
            let val = val_match.map(|m| m.as_str()).unwrap_or("");

            extracted_value = val.to_string();
            if let Some(m) = val_match {
                match_range = (m.start(), m.end());
            }

            // Check key name
            if SUSPECT_KEYS.is_match(key) {
                score += 40;
                rule_id = "SUSPECT_VARIABLE".to_string();
                reasons.push(format!("Variable '{}' implies secret", key));
            }

            // Check value characteristics
            if val.len() > 16 {
                let ent = calculate_entropy(val);
                if ent > 4.0 {
                    // High entropy hex/b64
                    score += 30;
                    reasons.push("Value has high entropy".to_string());
                } else if ent > 3.0 && val.len() > 20 {
                    score += 20;
                    reasons.push("Value has moderate entropy and length".to_string());
                }
            } else if val.len() < 8 {
                score -= 20; // Too short usually
            }

            if FMT_GENERIC_KEYLIKE.is_match(val) {
                score += 30;
                reasons.push("Value looks like an API key (sk-...)".to_string());
            }
        }
    }

    // 3. Penalties & Adjustments
    if is_test_file(path) {
        score -= 40;
        reasons.push("File appears to be a test/mock".to_string());
    }

    if DUMMY_VALUES.is_match(&extracted_value) {
        score -= 50;
        reasons.push("Value matches known placeholders".to_string());
    }

    // 4. Thresholds
    let final_score = score.clamp(0, 100) as u8;

    let severity = if final_score >= 80 {
        Severity::High
    } else if final_score >= 60 {
        Severity::Medium
    } else {
        Severity::Low
    };

    if final_score < 60 {
        return None;
    }

    // Fingerprinting
    // We use rule+value+path+line to identify it.
    // If line numbers shift, this breaks, but fuzzy matching is hard for MVP.
    // Adding surrounding context to hash would help shift-detection but hurt edit-detection.
    let fingerprint_raw = format!("{}|{}|{}|{}", rule_id, extracted_value, path, line_num);
    let mut hasher = Sha256::new();
    hasher.update(fingerprint_raw);
    let fingerprint = hex::encode(hasher.finalize());

    Some(Finding {
        rule_id,
        severity,
        score: final_score,
        file_path: path.to_string(),
        line_number: line_num,
        start_index: match_range.0,
        end_index: match_range.1,
        raw_content: content.trim().to_string(),
        redacted_preview: redact(&extracted_value),
        fingerprint,
        reason: reasons.join(", "),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redaction() {
        assert_eq!(redact("1234567"), "<redacted>");
        assert_eq!(redact("12345678"), "123...678");
        assert_eq!(redact("abcdefghijklmnop"), "abc...nop");
    }

    #[test]
    fn test_private_key_detection() {
        let line = "-----BEGIN RSA PRIVATE KEY-----";
        // Use a non-test filename to avoid penalty
        let finding = scan_line("prod_keys.pem", 1, line).expect("Should detect private key");
        assert_eq!(finding.rule_id, "PRIVATE_KEY_BLOCK");
        assert_eq!(finding.severity, Severity::High);
    }

    #[test]
    fn test_aws_key_detection() {
        // Avoid "EXAMPLE" in the key string to avoid dummy value penalty
        let line = "aws_access_key_id = AKIAIOSFODNN7REALKEY";
        let finding = scan_line("config.ini", 10, line).expect("Should detect AWS key");
        assert_eq!(finding.rule_id, "AWS_ACCESS_KEY");
        assert_eq!(finding.severity, Severity::High);
        // "aws_access_key_id = " is 20 chars
        assert_eq!(finding.start_index, 20);
        assert_eq!(finding.end_index, 40);
    }

    #[test]
    fn test_dummy_value_ignored() {
        let line = "const apiKey = 'changeme';";
        let finding = scan_line("config.js", 1, line);
        assert!(finding.is_none(), "Should ignore dummy values");
    }

    #[test]
    fn test_high_entropy_assignment() {
        // High entropy string > 16 chars
        let line = "const secret = '7f8a9d1c2b3e4f5a6b7c8d9e0f1a2b3c';";
        let finding = scan_line("keys.js", 1, line).expect("Should detect high entropy assignment");
        assert_eq!(finding.rule_id, "SUSPECT_VARIABLE");
        assert!(finding.score >= 60);
    }

    #[test]
    fn test_short_password_ignored() {
        // Too short to be interesting usually, unless very specific rule
        let line = "const password = '123';";
        let finding = scan_line("test.js", 1, line);
        assert!(finding.is_none());
    }

    #[test]
    fn test_test_file_penalty() {
        let line = "const secret = '7f8a9d1c2b3e4f5a6b7c8d9e0f1a2b3c';";
        // "test.js" triggers is_test_file penalty (-40)
        // Base score for high entropy suspect var might be ~70-90.
        // 40 + 30 (entropy) = 70. 70 - 40 = 30. Should be None (<60).
        let finding = scan_line("test.js", 1, line);
        assert!(
            finding.is_none(),
            "Test file should penalize score below threshold"
        );
    }
}
