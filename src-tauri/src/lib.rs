//! Zenou Tweaks — library root.

pub mod backup;
pub mod catalog;
pub mod catalog_ext;
pub mod cleanup;
pub mod commands;
pub mod engine;
pub mod error;
pub mod flagged;
pub mod journal;
pub mod logging;
pub mod ops;
pub mod paths;
pub mod registry;
pub mod restore_point;
pub mod scan;
pub mod settings;
pub mod types;
pub mod winapi;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod bench;

use std::sync::Mutex;

use error::{ZenouError, ZResult};
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

pub struct AppState {
  pub paths: paths::AppPaths,
  pub backups: Mutex<backup::BackupStore>,
  pub journal: Mutex<journal::Journal>,
  pub log: Mutex<logging::ActivityLog>,
  pub settings: Mutex<settings::SettingsStore>,
  /// Per-tweak operation locks (prevent duplicate simultaneous operations).
  pub busy: Mutex<std::collections::HashSet<String>>,
  /// True once a restore point has been created this session.
  pub restore_point_done: Mutex<bool>,
}

pub static APP_PATHS: std::sync::OnceLock<paths::AppPaths> = std::sync::OnceLock::new();

/// Catalog with hooks attached (tasks/services).
pub fn build_catalog() -> Vec<engine::Tweak> {
  let mut tweaks = catalog::all_tweaks();
  for t in tweaks.iter_mut() {
    match t.meta.id.as_str() {
      "win.telemetry-tasks" => catalog_ext::attach_group_hooks(t, "win.telemetry-tasks"),
      "win.unnecessary-services" => catalog_ext::attach_group_hooks(t, "win.unnecessary-services"),
      "gpu.nvidia-telemetry" => catalog_ext::attach_group_hooks(t, "gpu.nvidia-telemetry"),
      _ => {}
    }
  }
  tweaks
}

pub fn find_tweak<'a>(catalog: &'a [engine::Tweak], id: &str) -> ZResult<&'a engine::Tweak> {
  catalog
    .iter()
    .find(|t| t.meta.id == id)
    .ok_or_else(|| ZenouError::state(format!("Unknown tweak \"{id}\".")))
}

/// Acquire the per-tweak operation lock; Err when an operation is already running.
pub fn acquire_lock(state: &AppState, id: &str) -> ZResult<()> {
  let mut busy = state
    .busy
    .lock()
    .map_err(|_| ZenouError::state("The operation lock is unavailable."))?;
  if busy.contains(id) {
    return Err(ZenouError::state(
      "This tweak already has an operation running. Wait for it to finish.",
    ));
  }
  busy.insert(id.to_string());
  Ok(())
}

pub fn release_lock(state: &AppState, id: &str) {
  if let Ok(mut busy) = state.busy.lock() {
    busy.remove(id);
  }
}

/// Ensure a restore point exists before risky work (once per session).
pub fn ensure_restore_point(state: &AppState, forced: bool, setting_enabled: bool) -> ZResult<bool> {
  let need = {
    let mut done = state
      .restore_point_done
      .lock()
      .map_err(|_| ZenouError::state("Internal state error."))?;
    if *done && !forced {
      false
    } else if forced || setting_enabled {
      *done = true; // set before the slow call so parallel ops don't pile up
      true
    } else {
      false
    }
  };
  if !need {
    return Ok(false);
  }
  restore_point::create_restore_point("Zenou Tweaks — before system changes")?;
  Ok(true)
}

/// Append per-tweak results to the activity log.
pub fn log_group(state: &AppState, action: &str, label: &str, results: &[types::TweakResult]) {
  if let Ok(log) = state.log.lock() {
    for r in results {
      log.append(&logging::LogEntry {
        timestamp: chrono::Utc::now(),
        action: action.into(),
        tweak_id: Some(r.tweak_id.clone()),
        tweak_name: Some(r.tweak_id.clone()),
        result: if r.skipped {
          "skipped".into()
        } else if r.success {
          "success".into()
        } else {
          "failed".into()
        },
        message: r.error.clone().unwrap_or_else(|| match action {
          "apply" => "Applied and verified.".to_string(),
          "restore" => "Restored to recorded original values.".to_string(),
          _ => "Completed.".to_string(),
        }),
        error_details: r.details.clone(),
      });
    }
  }
  let _ = label;
}

pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_autostart::init(
      MacosLauncher::LaunchAgent,
      Some(vec![]),
    ))
    .setup(|app| {
      let paths = paths::AppPaths::init()?;
      let _ = APP_PATHS.set(paths::AppPaths::init()?);
      let backups = backup::BackupStore::new(paths.backups());
      let journal = journal::Journal::new(paths.changes());
      let log = logging::ActivityLog::new(paths.logs().join("activity.jsonl"));
      let settings = settings::SettingsStore::new(paths.settings());
      app.manage(AppState {
        paths,
        backups: Mutex::new(backups),
        journal: Mutex::new(journal),
        log: Mutex::new(log),
        settings: Mutex::new(settings),
        busy: Mutex::new(std::collections::HashSet::new()),
        restore_point_done: Mutex::new(false),
      });

      // Apply autostart setting from persisted preferences.
      {
        use tauri_plugin_autostart::ManagerExt as _;
        let start_enabled = {
          let state = app.state::<AppState>();
          state
            .settings
            .lock()
            .ok()
            .and_then(|s| s.load().ok())
            .map(|s| s.start_with_windows)
            .unwrap_or(false)
        };
        let auto = app.autolaunch();
        if start_enabled {
          let _ = auto.enable();
        } else {
          let _ = auto.disable();
        }
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::scan_system,
      commands::is_elevated,
      commands::flagged_operations,
      commands::list_tweaks,
      commands::tweak_details,
      commands::apply_tweak,
      commands::restore_tweak,
      commands::apply_group,
      commands::list_profiles,
      commands::save_custom_profile,
      commands::delete_custom_profile,
      commands::create_backup,
      commands::list_backups,
      commands::delete_backup,
      commands::restore_backup,
      commands::list_changes,
      commands::get_change,
      commands::restore_change_set,
      commands::read_logs,
      commands::clear_logs,
      commands::export_logs,
      commands::pick_save_file,
      commands::get_settings,
      commands::set_settings,
      commands::toggle_favorite,
      commands::cleanup_previews,
      commands::run_cleanup,
      commands::create_restore_point,
      commands::request_elevation,
    ])
    .on_window_event(|window, event| {
      // Minimize to tray: hide instead of closing when the setting is on.
      if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        let state: tauri::State<AppState> = window.app_handle().state();
        let minimize = state
          .settings
          .lock()
          .ok()
          .and_then(|s| s.load().ok())
          .map(|s| s.minimize_to_tray)
          .unwrap_or(true);
        if minimize {
          api.prevent_close();
          let _ = window.hide();
        }
      }
    })
    .run(tauri::generate_context!())
    .expect("error while running Zenou Tweaks");
}


