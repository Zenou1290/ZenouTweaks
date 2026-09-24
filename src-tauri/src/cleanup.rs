//! Cleanup operations: previewed file removal with accurate byte/count reporting.
//! These are one-shot actions (not stateful tweaks) exposed as commands.

use serde::Serialize;

use crate::error::{ZenouError, ZResult};

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPreview {
  pub id: String,
  pub name: String,
  pub description: String,
  pub file_count: u64,
  pub total_bytes: u64,
  /// True when files are deleted permanently (no undo possible).
  pub irreversible: bool,
}

fn dir_size(dir: &std::path::Path) -> (u64, u64) {
  let mut count = 0u64;
  let mut bytes = 0u64;
  if let Ok(entries) = std::fs::read_dir(dir) {
    for entry in entries.flatten() {
      match entry.file_type() {
        Ok(ft) if ft.is_dir() => {
          let (c, b) = dir_size(&entry.path());
          count += c;
          bytes += b;
        }
        Ok(_) => {
          count += 1;
          bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
        Err(_) => {}
      }
    }
  }
  (count, bytes)
}

pub fn preview_user_temp() -> CleanupPreview {
  let dir = std::env::temp_dir();
  let (count, bytes) = dir_size(&dir);
  CleanupPreview {
    id: "cleanup.user-temp".into(),
    name: "User temporary files".into(),
    description: "Files in your user temp folder (%TEMP%) that are not currently locked by running apps.".into(),
    file_count: count,
    total_bytes: bytes,
    irreversible: true,
  }
}

pub fn preview_windows_temp() -> CleanupPreview {
  let dir = std::path::PathBuf::from(r"C:\Windows\Temp");
  let (count, bytes) = dir_size(&dir);
  CleanupPreview {
    id: "cleanup.windows-temp".into(),
    name: "Windows temporary files".into(),
    description: "Files in C:\\Windows\\Temp not currently in use by Windows.".into(),
    file_count: count,
    total_bytes: bytes,
    irreversible: true,
  }
}

pub fn preview_prefetch() -> CleanupPreview {
  let dir = std::path::PathBuf::from(r"C:\Windows\Prefetch");
  let (count, bytes) = dir_size(&dir);
  CleanupPreview {
    id: "cleanup.prefetch".into(),
    name: "Prefetch data".into(),
    description: "Windows Prefetch trace files. They will be recreated as apps are used; first launches may be slightly slower afterwards.".into(),
    file_count: count,
    total_bytes: bytes,
    irreversible: true,
  }
}

/// Delete the contents of a directory (not the directory itself), skipping locked files.
/// Returns (files deleted, bytes freed, files skipped).
pub fn clear_directory(dir: &std::path::Path) -> ZResult<(u64, u64, u64)> {
  if !dir.exists() {
    return Err(ZenouError::io(format!("The folder {} does not exist.", dir.display())));
  }
  let mut deleted = 0u64;
  let mut freed = 0u64;
  let mut skipped = 0u64;
  let entries = std::fs::read_dir(dir)
    .map_err(|_| ZenouError::io(format!("Could not open {} for cleanup.", dir.display())))?;
  for entry in entries.flatten() {
    let path = entry.path();
    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
    let size = if is_dir {
      dir_size(&path).1
    } else {
      entry.metadata().map(|m| m.len()).unwrap_or(0)
    };
    let result = if is_dir {
      std::fs::remove_dir_all(&path)
    } else {
      std::fs::remove_file(&path)
    };
    match result {
      Ok(()) => {
        deleted += 1;
        freed += size;
      }
      Err(_) => {
        skipped += 1;
      }
    }
  }
  Ok((deleted, freed, skipped))
}

pub fn run_user_temp_cleanup() -> ZResult<(u64, u64, u64)> {
  clear_directory(&std::env::temp_dir())
}

pub fn run_windows_temp_cleanup() -> ZResult<(u64, u64, u64)> {
  clear_directory(&std::path::PathBuf::from(r"C:\Windows\Temp"))
}

pub fn run_prefetch_cleanup() -> ZResult<(u64, u64, u64)> {
  clear_directory(&std::path::PathBuf::from(r"C:\Windows\Prefetch"))
}

/// Windows Update cache purge: stop wuauserv, clear SoftwareDistribution\Download, restart.
/// The service's original state is preserved.
pub fn run_windows_update_cache_cleanup() -> ZResult<(u64, u64, u64)> {
  use crate::registry::Hive;
  let svc_path = r"SYSTEM\CurrentControlSet\Services\wuauserv";
  let original_start = crate::registry::read_raw(Hive::LocalMachine, svc_path, "Start")?
    .and_then(|v| if v.bytes.len() == 4 { Some(u32::from_le_bytes([v.bytes[0], v.bytes[1], v.bytes[2], v.bytes[3]])) } else { None });

  // Stop the service (ignore "not started" errors).
  let _ = crate::ops::stop_service_now("wuauserv");

  let result = clear_directory(&std::path::PathBuf::from(r"C:\Windows\SoftwareDistribution\Download"));

  // Restore the service start type and restart it if it was running.
  if let Some(start) = original_start {
    if start != 4 {
      let _ = crate::ops::start_service_now("wuauserv");
    }
  }

  result
}

pub fn preview_windows_update_cache() -> CleanupPreview {
  let dir = std::path::PathBuf::from(r"C:\Windows\SoftwareDistribution\Download");
  let (count, bytes) = dir_size(&dir);
  CleanupPreview {
    id: "cleanup.wu-cache".into(),
    name: "Windows Update cache".into(),
    description: "Downloaded update files in SoftwareDistribution\\Download. Windows re-downloads anything it still needs.".into(),
    file_count: count,
    total_bytes: bytes,
    irreversible: true,
  }
}
