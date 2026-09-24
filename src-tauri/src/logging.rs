//! JSONL activity log for the Logs page.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{ZenouError, ZResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
  pub timestamp: DateTime<Utc>,
  /// "apply" | "restore" | "cleanup" | "backup" | "scan" | "system"
  pub action: String,
  pub tweak_id: Option<String>,
  pub tweak_name: Option<String>,
  /// "success" | "failed" | "partial" | "skipped"
  pub result: String,
  pub message: String,
  pub error_details: Option<String>,
}

pub struct ActivityLog {
  path: std::path::PathBuf,
}

impl ActivityLog {
  pub fn new(path: std::path::PathBuf) -> Self {
    Self { path }
  }

  pub fn append(&self, entry: &LogEntry) {
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&self.path) {
      use std::io::Write;
      if let Ok(line) = serde_json::to_string(entry) {
        let _ = writeln!(file, "{line}");
      }
    }
  }

  pub fn read(&self) -> ZResult<Vec<LogEntry>> {
    if !self.path.exists() {
      return Ok(vec![]);
    }
    let text = std::fs::read_to_string(&self.path)?;
    let mut out = Vec::new();
    for line in text.lines() {
      if line.trim().is_empty() {
        continue;
      }
      if let Ok(entry) = serde_json::from_str::<LogEntry>(line) {
        out.push(entry);
      }
    }
    out.reverse(); // newest first
    Ok(out)
  }

  pub fn clear(&self) -> ZResult<()> {
    if self.path.exists() {
      std::fs::remove_file(&self.path)
        .map_err(|e| ZenouError::io("Could not clear the activity log.").set_details(format!("{e}")))?;
    }
    Ok(())
  }

  pub fn export_to(&self, target: &std::path::Path) -> ZResult<String> {
    let text = if self.path.exists() {
      std::fs::read_to_string(&self.path)?
    } else {
      String::new()
    };
    std::fs::write(target, text)?;
    Ok(target.display().to_string())
  }
}
