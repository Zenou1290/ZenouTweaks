//! Persisted application settings.

use serde::{Deserialize, Serialize};

use crate::error::ZResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
  pub start_with_windows: bool,
  pub minimize_to_tray: bool,
  pub theme: String,          // "dark" | "light"
  pub accent: String,         // "teal" (only accent shipped in v1)
  pub notifications: bool,
  pub restore_point_risky: bool,
  pub confirm_bulk: bool,
  pub reduced_motion: bool,
  pub favorites: Vec<String>,
}

impl Default for AppSettings {
  fn default() -> Self {
    Self {
      start_with_windows: false,
      minimize_to_tray: true,
      theme: "dark".into(),
      accent: "teal".into(),
      notifications: true,
      restore_point_risky: true,
      confirm_bulk: true,
      reduced_motion: false,
      favorites: vec![],
    }
  }
}

pub struct SettingsStore {
  path: std::path::PathBuf,
}

impl SettingsStore {
  pub fn new(path: std::path::PathBuf) -> Self {
    Self { path }
  }

  pub fn load(&self) -> ZResult<AppSettings> {
    if !self.path.exists() {
      return Ok(AppSettings::default());
    }
    let text = std::fs::read_to_string(&self.path)?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
  }

  pub fn save(&self, settings: &AppSettings) -> ZResult<()> {
    std::fs::write(&self.path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
  }
}
