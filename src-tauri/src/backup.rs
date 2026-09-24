//! Backup snapshots: recorded originals per tweak, restorable at any time.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::engine::Tweak;
use crate::error::{ZenouError, ZResult};
use crate::types::SavedValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotEntry {
  pub tweak_id: String,
  pub tweak_name: String,
  pub values: Vec<SavedValue>,
  /// For task/service tweaks: original enable-state per task/service.
  pub task_states: Vec<(String, bool)>,
  pub service_states: Vec<(String, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
  pub id: String,
  pub label: String,
  pub created_at: DateTime<Utc>,
  pub automatic: bool,
  pub entries: Vec<SnapshotEntry>,
}

pub struct BackupStore {
  dir: std::path::PathBuf,
}

impl BackupStore {
  pub fn new(dir: std::path::PathBuf) -> Self {
    Self { dir }
  }

  fn path_for(&self, id: &str) -> std::path::PathBuf {
    self.dir.join(format!("{id}.json"))
  }

  pub fn create(
    &self,
    label: &str,
    automatic: bool,
    entries: Vec<SnapshotEntry>,
  ) -> ZResult<Snapshot> {
    let id = format!("backup-{}", Utc::now().format("%Y%m%d-%H%M%S%3f"));
    let snapshot = Snapshot {
      id,
      label: label.to_string(),
      created_at: Utc::now(),
      automatic,
      entries,
    };
    let path = self.path_for(&snapshot.id);
    std::fs::write(&path, serde_json::to_string_pretty(&snapshot)?)?;
    Ok(snapshot)
  }

  pub fn list(&self) -> ZResult<Vec<Snapshot>> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&self.dir).map_err(|e| {
      ZenouError::io("Could not read the backups folder.").set_details(format!("{e}"))
    })?;
    for entry in entries.flatten() {
      let path = entry.path();
      if path.extension().and_then(|e| e.to_str()) == Some("json") {
        if let Ok(text) = std::fs::read_to_string(&path) {
          if let Ok(snap) = serde_json::from_str::<Snapshot>(&text) {
            out.push(snap);
          }
        }
      }
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.created_at));
    Ok(out)
  }

  pub fn get(&self, id: &str) -> ZResult<Snapshot> {
    let path = self.path_for(id);
    let text = std::fs::read_to_string(&path)
      .map_err(|_| ZenouError::state("That backup no longer exists."))?;
    serde_json::from_str(&text).map_err(|e| ZenouError::state("That backup file is damaged.").set_details(format!("{e}")))
  }

  pub fn delete(&self, id: &str) -> ZResult<()> {
    let path = self.path_for(id);
    std::fs::remove_file(&path)
      .map_err(|_| ZenouError::state("That backup no longer exists."))
  }

  /// Build capture entries for the given tweaks (their current live values).
  pub fn capture_entries(&self, tweaks: &[&Tweak]) -> ZResult<Vec<SnapshotEntry>> {
    let mut entries = Vec::new();
    for t in tweaks {
      let values = t.capture_all()?;
      entries.push(SnapshotEntry {
        tweak_id: t.meta.id.clone(),
        tweak_name: t.meta.name.clone(),
        values,
        task_states: vec![],
        service_states: vec![],
      });
    }
    Ok(entries)
  }
}

impl ZenouError {
  pub fn set_details(mut self, details: String) -> Self {
    self.details = Some(details);
    self
  }
}
