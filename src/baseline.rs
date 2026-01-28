use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Baseline {
    pub generated_at: Option<DateTime<Utc>>,
    pub fingerprints: HashSet<String>,
    // Optional: store details for debugging if user wants to inspect baseline
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub metadata: HashMap<String, BaselineEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BaselineEntry {
    pub file: String,
    pub rule: String,
    pub preview: String,
}

impl Baseline {
    pub fn load() -> Self {
        if let Ok(content) = fs::read_to_string(".sieve.baseline.json") {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&mut self) -> Result<()> {
        self.generated_at = Some(Utc::now());
        let content = serde_json::to_string_pretty(self)?;
        fs::write(".sieve.baseline.json", content)?;
        Ok(())
    }

    pub fn add(&mut self, fingerprint: String, file: String, rule: String, preview: String) {
        if self.fingerprints.insert(fingerprint.clone()) {
            self.metadata.insert(
                fingerprint,
                BaselineEntry {
                    file,
                    rule,
                    preview,
                },
            );
        }
    }

    pub fn contains(&self, fingerprint: &str) -> bool {
        self.fingerprints.contains(fingerprint)
    }
}
