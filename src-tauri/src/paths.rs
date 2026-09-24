//! Application data directories and files.

use std::fs;
use std::path::PathBuf;

use crate::error::{ZenouError, ZResult};

pub struct AppPaths {
  pub root: PathBuf,
}

impl AppPaths {
  pub fn init() -> ZResult<Self> {
    let base = dirs::config_dir()
      .ok_or_else(|| ZenouError::io("Could not locate the application data directory."))?;
    let root = base.join("ZenouTweaks");
    fs::create_dir_all(root.join("backups"))?;
    fs::create_dir_all(root.join("logs"))?;
    fs::create_dir_all(root.join("changes"))?;
    Ok(Self { root })
  }

  pub fn backups(&self) -> PathBuf {
    self.root.join("backups")
  }

  pub fn logs(&self) -> PathBuf {
    self.root.join("logs")
  }

  pub fn changes(&self) -> PathBuf {
    self.root.join("changes")
  }

  pub fn settings(&self) -> PathBuf {
    self.root.join("settings.json")
  }
}
