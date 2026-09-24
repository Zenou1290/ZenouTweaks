//! Change journal: append-only change-set records powering the Changes page.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::ZResult;
use crate::types::TweakResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSet {
  pub id: String,
  pub label: String,
  pub created_at: DateTime<Utc>,
  /// Backup snapshot id capturing the pre-change state (restore source).
  pub backup_id: Option<String>,
  pub results: Vec<TweakResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSetSummary {
  pub id: String,
  pub label: String,
  pub created_at: DateTime<Utc>,
  pub backup_id: Option<String>,
  pub applied: usize,
  pub failed: usize,
  pub skipped: usize,
}

pub struct Journal {
  dir: std::path::PathBuf,
}

impl Journal {
  pub fn new(dir: std::path::PathBuf) -> Self {
    Self { dir }
  }

  pub fn record(&self, label: &str, backup_id: Option<String>, results: Vec<TweakResult>) -> ZResult<ChangeSet> {
    let id = format!("change-{}", Utc::now().format("%Y%m%d-%H%M%S%3f"));
    let set = ChangeSet {
      id,
      label: label.to_string(),
      created_at: Utc::now(),
      backup_id,
      results,
    };
    let path = self.dir.join(format!("{}.json", set.id));
    std::fs::write(&path, serde_json::to_string_pretty(&set)?)?;
    Ok(set)
  }

  pub fn list(&self) -> ZResult<Vec<ChangeSetSummary>> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&self.dir) {
      Ok(e) => e,
      Err(_) => return Ok(out),
    };
    for entry in entries.flatten() {
      let path = entry.path();
      if path.extension().and_then(|e| e.to_str()) == Some("json") {
        if let Ok(text) = std::fs::read_to_string(&path) {
          if let Ok(set) = serde_json::from_str::<ChangeSet>(&text) {
            let summary = ChangeSetSummary {
              id: set.id,
              label: set.label,
              created_at: set.created_at,
              backup_id: set.backup_id,
              applied: set.results.iter().filter(|r| r.success && !r.skipped).count(),
              failed: set.results.iter().filter(|r| !r.success && !r.skipped).count(),
              skipped: set.results.iter().filter(|r| r.skipped).count(),
            };
            out.push(summary);
          }
        }
      }
    }
    out.sort_by_key(|c| std::cmp::Reverse(c.created_at));
    Ok(out)
  }

  pub fn get(&self, id: &str) -> ZResult<ChangeSet> {
    let path = self.dir.join(format!("{id}.json"));
    let text = std::fs::read_to_string(&path)
      .map_err(|_| crate::error::ZenouError::state("That change record no longer exists."))?;
    Ok(serde_json::from_str(&text)?)
  }
}
