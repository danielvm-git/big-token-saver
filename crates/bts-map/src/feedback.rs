//! Regret-learning feedback database for bts-map.
//!
//! Tracks which files from a previous map were actually used ("good")
//! vs. shown but ignored ("bad"), and applies a multiplicative adjustment
//! to PageRank scores so the map improves with real usage.
//!
//! Storage format: a JSON file (`~/.config/bts/map-feedback.json`) with
//! per-file good/bad counts. The `bts map feedback` shell verb manages it.
//!
//! Adjustment formula:
//!   factor = 1.0 + α * (good - bad) / (good + bad + 1)
//!   where α = 0.3 (learning rate).
//!
//! Files with zero feedback get factor = 1.0 (no adjustment).
//! Positive feedback boosts (factor > 1.0); negative penalises (factor < 1.0).
//! The +1 denominator term prevents division by zero and adds a prior.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Learning rate: how aggressively to adjust scores.
const ALPHA: f64 = 0.3;

/// Per-file feedback entry stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFeedback {
    /// How many times this file was marked as useful.
    pub good: u32,
    /// How many times this file was shown but skipped/ignored.
    pub bad: u32,
}

/// The full feedback database as persisted.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeedbackDb {
    /// File path → feedback counts.
    #[serde(default)]
    pub files: HashMap<String, FileFeedback>,
}

impl FeedbackDb {
    /// Load the feedback database from a JSON file.
    ///
    /// Returns `Ok(FeedbackDb)` on success, `Err` on I/O or parse errors.
    /// An absent file is treated as an empty database (no error).
    pub fn load(path: &Path) -> Result<Self, FeedbackError> {
        match fs::read_to_string(path) {
            Ok(raw) => {
                let db: FeedbackDb = serde_json::from_str(&raw)
                    .map_err(|e| FeedbackError::Parse(e.to_string()))?;
                Ok(db)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(FeedbackDb {
                    files: HashMap::new(),
                })
            }
            Err(e) => Err(FeedbackError::Io(e.to_string())),
        }
    }

    /// Save the feedback database to a JSON file.
    #[allow(dead_code)]
    pub fn save(&self, path: &Path) -> Result<(), FeedbackError> {
        let json =
            serde_json::to_string_pretty(self).map_err(|e| FeedbackError::Parse(e.to_string()))?;
        fs::write(path, json).map_err(|e| FeedbackError::Io(e.to_string()))
    }

    /// Record a good or bad selection for a file.
    #[allow(dead_code)]
    pub fn record(&mut self, fname: &str, good: bool) {
        let entry = self.files.entry(fname.to_string()).or_insert(FileFeedback {
            good: 0,
            bad: 0,
        });
        if good {
            entry.good += 1;
        } else {
            entry.bad += 1;
        }
    }

    /// Get the score adjustment factor for a file.
    ///
    /// Returns 1.0 if the file has no feedback (no adjustment).
    pub fn factor(&self, fname: &str) -> f64 {
        let entry = match self.files.get(fname) {
            Some(e) => e,
            None => return 1.0,
        };
        let total = (entry.good + entry.bad + 1) as f64;
        let diff = entry.good as f64 - entry.bad as f64;
        1.0 + ALPHA * diff / total
    }
}

/// Error type for feedback operations.
#[derive(Debug)]
pub enum FeedbackError {
    Io(String),
    Parse(String),
}

impl std::fmt::Display for FeedbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedbackError::Io(s) => write!(f, "feedback I/O error: {}", s),
            FeedbackError::Parse(s) => write!(f, "feedback parse error: {}", s),
        }
    }
}

impl std::error::Error for FeedbackError {}
