//! Tauri commands — thin layer: validate → engine → log → return.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::backup::{BackupStore, SnapshotEntry};
use crate::catalog_ext;
use crate::log_group;
use crate::engine::Tweak;
use crate::error::{ZenouError, ZResult};
use crate::flagged::FlaggedEntry;
use crate::journal::{ChangeSet, ChangeSetSummary};
use crate::logging::LogEntry;
use crate::settings::AppSettings;
use crate::types::{ChangePreview, GroupResult, Risk, TweakMeta, TweakResult, TweakState};
use crate::{backup, AppState, build_catalog, ensure_restore_point, find_tweak, flagged, registry, restore_point, scan, types};

type Ctx<'a> = State<'a, AppState>;

fn load_settings(state: &AppState) -> AppSettings {
  state
    .settings
    .lock()
    .ok()
    .and_then(|s| s.load().ok())
    .unwrap_or_default()
}

// ---------------------------------------------------------------- system

#[tauri::command]
pub async fn scan_system(refresh: Option<bool>) -> ZResult<scan::SystemScan> {
  if refresh.unwrap_or(false) {
    scan::invalidate_scan_cache();
  }
  scan::scan_system()
}

#[tauri::command]
pub async fn is_elevated() -> bool {
  crate::winapi::is_elevated()
}

#[tauri::command]
pub async fn flagged_operations() -> Vec<FlaggedEntry> {
  flagged::flagged_entries()
}

// ---------------------------------------------------------------- tweaks

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TweakView {
  pub meta: TweakMeta,
  pub state: TweakState,
  pub previews: Vec<ChangePreview>,
  pub favorite: bool,
}

#[tauri::command]
pub async fn list_tweaks(state: Ctx<'_>) -> ZResult<Vec<TweakView>> {
  let favorites = load_settings(&state).favorites;
  let catalog = build_catalog();

  // Serial state checks: every check is a registry read (instant). The only
  // subprocess left on this path is one cached PowerShell query (memory
  // compression); a parallel split measured no gain over that single dominant
  // call, so the simple serial path wins.
  let mut views = Vec::with_capacity(catalog.len());
  for t in catalog.iter() {
    let tweak_state = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.check_state()))
      .unwrap_or(TweakState::Unknown);
    views.push(TweakView {
      meta: t.meta.clone(),
      state: tweak_state,
      previews: t.previews(),
      favorite: favorites.contains(&t.meta.id),
    });
  }
  Ok(views)
}

#[tauri::command]
pub async fn tweak_details(id: String) -> ZResult<TweakView> {
  let catalog = build_catalog();
  let t = find_tweak(&catalog, &id)?;
  Ok(TweakView {
    meta: t.meta.clone(),
    state: t.check_state(),
    previews: t.previews(),
    favorite: false,
  })
}

/// The shared execution core for apply/restore of a single tweak.
fn execute_tweak(state: &AppState, id: &str, action: &str) -> ZResult<TweakResult> {
  let catalog = build_catalog();
  let tweak = find_tweak(&catalog, id)?;
  let meta = tweak.meta.clone();

  // Busy lock.
  crate::acquire_lock(state, id)?;

  let result: ZResult<TweakResult> = (|| {
    // Administrator requirement.
    if meta.requires_admin && !crate::winapi::is_elevated() {
      // Attempt a per-operation elevation via the Zenou helper is not possible
      // in-process; surface a clear, actionable error.
      return Ok(TweakResult {
        tweak_id: meta.id.clone(),
        success: false,
        skipped: false,
        error: Some(
          "Administrator permission is required. Right-click Zenou Tweaks in the Start menu and choose \"Run as administrator\", then apply this tweak again.".into(),
        ),
        details: None,
        restart_required: false,
        steps: vec![],
      });
    }

    // Restore point for medium/high risk tweaks (forced for high).
    let settings = load_settings(state);
    let forced = meta.risk == Risk::High;
    if action == "apply" && (meta.risk == Risk::High || meta.risk == Risk::Medium) {
      match ensure_restore_point(state, forced, settings.restore_point_risky) {
        Ok(_) => {}
        Err(e) if forced => return Ok(TweakResult {
          tweak_id: meta.id.clone(),
          success: false,
          skipped: false,
          error: Some(e.message),
          details: e.details,
          restart_required: false,
          steps: vec![],
        }),
        Err(_) => { /* non-forced failures don't block */ }
      }
    }

    // Automatic pre-change backup of this tweak's originals.
    let backup_id = if action == "apply" {
      match state.backups.lock() {
        Ok(store) => {
          let entries = store.capture_entries(std::slice::from_ref(&tweak))?;
          let snap = store.create(&format!("Before: {}", meta.name), true, entries)?;
          Some(snap.id)
        }
        Err(_) => None,
      }
    } else {
      None
    };

    // Execute.
    let _ = &backup_id;    let steps = match action {
      "apply" => tweak.apply(),
      "restore" => {
        // Restore sources: latest automatic backup entries for this tweak.
        let saved = state
          .backups
          .lock()
          .ok()
          .and_then(|store| store.list().ok())
          .and_then(|snaps| {
            snaps
              .iter()
              .find_map(|s| {
                s.entries.iter().find(|e| e.tweak_id == meta.id).map(|e| e.values.clone())
              })
          })
          .unwrap_or_default();
        tweak.restore(&saved)
      }
      _ => return Err(ZenouError::state("Unknown operation.")),
    };

    let success = steps.iter().all(|s| s.ok);
    // Re-read live state from the system — never trust the operation result.
    let verified_state = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tweak.check_state()))
      .unwrap_or(TweakState::Unknown);
    let _ = verified_state;

    Ok(TweakResult {
      tweak_id: meta.id.clone(),
      success,
      skipped: false,
      error: if success { None } else { Some(last_error(&steps)) },
      details: if success { None } else { Some(format_steps(&steps)) },
      restart_required: meta.restart_required && success,
      steps,
    })
  })();

  crate::release_lock(state, id);
  result
}

fn last_error(steps: &[types::StepOutcome]) -> String {
  steps
    .iter()
    .rev()
    .find_map(|s| s.error.clone())
    .unwrap_or_else(|| "The operation did not complete.".into())
}

fn format_steps(steps: &[types::StepOutcome]) -> String {
  steps
    .iter()
    .map(|s| {
      format!(
        "{} -> {}{}",
        s.location,
        if s.ok { "OK" } else { "FAILED" },
        s.error.as_ref().map(|e| format!(": {e}")).unwrap_or_default()
      )
    })
    .collect::<Vec<_>>()
    .join("\n")
}

#[tauri::command]
pub async fn apply_tweak(app: AppHandle, id: String) -> ZResult<TweakResult> {
  let state = app.state::<AppState>();
  let result = execute_tweak(&state, &id, "apply")?;
  log_group(&state, "apply", &id, std::slice::from_ref(&result));
  Ok(result)
}

#[tauri::command]
pub async fn restore_tweak(app: AppHandle, id: String) -> ZResult<TweakResult> {
  let state = app.state::<AppState>();
  let result = execute_tweak(&state, &id, "restore")?;
  log_group(&state, "restore", &id, std::slice::from_ref(&result));
  Ok(result)
}

/// Apply a group of tweaks (profile / gaming group / quick optimize selection).
/// Confirmation and restore-point logic mirror single applies.
#[tauri::command]
pub async fn apply_group(app: AppHandle, ids: Vec<String>, label: String) -> ZResult<GroupResult> {
  let state = app.state::<AppState>();
  let settings = load_settings(&state);

  // Restore point up-front for the whole group.
  let restore_point_created = ensure_restore_point(&state, false, settings.restore_point_risky).unwrap_or(false);

  let mut results = Vec::new();
  let mut auto_backup_id: Option<String> = None;
  {
    // One automatic backup capturing all targets before the group runs.
    let catalog = build_catalog();
    let selected: Vec<&Tweak> = ids.iter().filter_map(|id| find_tweak(&catalog, id).ok()).collect();
    if let Ok(store) = state.backups.lock() {
      if let Ok(entries) = store.capture_entries(&selected) {
        if let Ok(snap) = store.create(&format!("Before: {label}"), true, entries) {
          auto_backup_id = Some(snap.id);
        }
      }
    }
  }

  for id in &ids {
    match execute_tweak(&state, id, "apply") {
      Ok(mut r) => {
        r.skipped = false;
        results.push(r);
      }
      Err(e) => results.push(TweakResult {
        tweak_id: id.clone(),
        success: false,
        skipped: false,
        error: Some(e.message),
        details: e.details,
        restart_required: false,
        steps: vec![],
      }),
    }
  }

  let group = GroupResult { label: label.clone(), results: results.clone(), restore_point_created };
  log_group(&state, "apply", &label, &results);
  if let Ok(journal) = state.journal.lock() {
    let _ = journal.record(&label, auto_backup_id, results);
  }
  Ok(group)
}

// ---------------------------------------------------------------- profiles

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
  pub id: String,
  pub name: String,
  pub description: String,
  pub tweak_ids: Vec<String>,
  pub builtin: bool,
}

#[tauri::command]
pub async fn list_profiles(state: Ctx<'_>) -> ZResult<Vec<Profile>> {
  let _ = &state;
  let mut out = vec![
    Profile {
      id: "profile.gaming".into(),
      name: "Gaming".into(),
      description: "Game Mode on, Game Bar/DVR off, responsive input, GPU scheduling.".into(),
      tweak_ids: vec![
        "game.game-mode-on".into(),
        "game.disable-game-bar".into(),
        "game.disable-game-dvr".into(),
        "game.mouse-accurate".into(),
        "game.keyboard-delay".into(),
        "perf.menu-delay".into(),
        "perf.system-responsiveness".into(),
      ],
      builtin: true,
    },
    Profile {
      id: "profile.low-latency".into(),
      name: "Low Latency".into(),
      description: "Multimedia priorities and responsiveness for lower input latency.".into(),
      tweak_ids: vec![
        "perf.system-responsiveness".into(),
        "perf.games-task-priority".into(),
        "game.mouse-hover-fast".into(),
        "gpu.monitor-latency".into(),
      ],
      builtin: true,
    },
    Profile {
      id: "profile.balanced".into(),
      name: "Balanced".into(),
      description: "Safe everyday optimizations: telemetry off, suggested apps off, DNS cache tweaks.".into(),
      tweak_ids: vec![
        "win.telemetry-minimal".into(),
        "win.advertising-id".into(),
        "win.suggested-apps".into(),
        "perf.menu-delay".into(),
        "net.dns-cache-timers".into(),
      ],
      builtin: true,
    },
    Profile {
      id: "profile.streaming".into(),
      name: "Streaming".into(),
      description: "Keeps Game Bar/capture available for broadcasting, trims background noise instead.".into(),
      tweak_ids: vec![
        "win.notifications-off".into(),
        "perf.background-apps-off".into(),
        "perf.system-responsiveness".into(),
      ],
      builtin: true,
    },
  ];
  // Custom profiles: stored in profiles.json next to settings.
  if let Some(dir) = crate::APP_PATHS.get() {
    if let Ok(stored) = std::fs::read_to_string(dir.root.join("profiles.json")) {
      if let Ok(list) = serde_json::from_str::<Vec<Profile>>(&stored) {
        out.extend(list);
      }
    }
  }
  Ok(out)
}

fn custom_path() -> PathBuf {
  crate::APP_PATHS
    .get()
    .map(|p| p.root.join("profiles.json"))
    .unwrap_or_else(|| PathBuf::from("profiles.json"))
}

#[tauri::command]
pub async fn save_custom_profile(name: String, tweak_ids: Vec<String>) -> ZResult<()> {
  let path = custom_path();
  let mut list: Vec<Profile> = std::fs::read_to_string(&path)
    .ok()
    .and_then(|t| serde_json::from_str(&t).ok())
    .unwrap_or_default();
  list.retain(|p| p.name != name);
  list.push(Profile {
    id: format!("profile.custom-{}", name.to_lowercase().replace(' ', "-")),
    name,
    description: "Custom profile".into(),
    tweak_ids,
    builtin: false,
  });
  std::fs::write(&path, serde_json::to_string_pretty(&list)?)?;
  Ok(())
}

#[tauri::command]
pub async fn delete_custom_profile(name: String) -> ZResult<()> {
  let path = custom_path();
  let mut list: Vec<Profile> = std::fs::read_to_string(&path)
    .ok()
    .and_then(|t| serde_json::from_str(&t).ok())
    .unwrap_or_default();
  list.retain(|p| p.name != name);
  std::fs::write(&path, serde_json::to_string_pretty(&list)?)?;
  Ok(())
}

// ---------------------------------------------------------------- backups

#[tauri::command]
pub async fn create_backup(state: Ctx<'_>, label: String) -> ZResult<backup::Snapshot> {
  let catalog = build_catalog();
  let refs: Vec<&Tweak> = catalog.iter().collect();
  let store = state.backups.lock().map_err(|_| ZenouError::state("Backups unavailable."))?;
  let entries: Vec<SnapshotEntry> = store.capture_entries(&refs)?;
  let snap = store.create(&label, false, entries)?;
  drop(store);
  if let Ok(log) = state.log.lock() {
    log.append(&LogEntry {
      timestamp: chrono::Utc::now(),
      action: "backup".into(),
      tweak_id: None,
      tweak_name: None,
      result: "success".into(),
      message: format!("Created backup \"{label}\" with {} entries.", snap.entries.len()),
      error_details: None,
    });
  }
  Ok(snap)
}

#[tauri::command]
pub async fn list_backups(state: Ctx<'_>) -> ZResult<Vec<backup::Snapshot>> {
  let store = state.backups.lock().map_err(|_| ZenouError::state("Backups unavailable."))?;
  store.list()
}

#[tauri::command]
pub async fn delete_backup(state: Ctx<'_>, id: String) -> ZResult<()> {
  let store = state.backups.lock().map_err(|_| ZenouError::state("Backups unavailable."))?;
  store.delete(&id)
}

#[tauri::command]
pub async fn restore_backup(state: Ctx<'_>, id: String) -> ZResult<GroupResult> {
  let snapshot = {
    let store = state.backends_lock_hack()?;
    store.get(&id)?
  };
  let mut results = Vec::new();
  {
    let store = state.backends_lock_hack()?;
    drop(store);
  }
  for entry in &snapshot.entries {
    // Restore each entry's recorded values.
    let mut steps = Vec::new();
    for v in &entry.values {
      let outcome = registry::restore(v).map(|_| types::StepOutcome {
        location: format!("{}\\{}", v.hive, v.path),
        ok: true,
        error: None,
      });
      match outcome {
        Ok(step) => steps.push(step),
        Err(e) => steps.push(types::StepOutcome { location: format!("{}\\{}", v.hive, v.path), ok: false, error: Some(e.message) }),
      }
    }
    let success = steps.iter().all(|s| s.ok);
    results.push(TweakResult {
      tweak_id: entry.tweak_id.clone(),
      success,
      skipped: false,
      error: if success { None } else { Some("Some values could not be restored.".into()) },
      details: Some(crate::commands::format_steps_public(&steps)),
      restart_required: false,
      steps,
    });
  }
  let group = GroupResult { label: format!("Restore backup \"{}\"", snapshot.label), results: results.clone(), restore_point_created: false };
  log_group(&state, "restore", &group.label, &results);
  if let Ok(journal) = state.journal.lock() {
    let _ = journal.record(&group.label, Some(snapshot.id.clone()), results);
  }
  Ok(group)
}

// ---------------------------------------------------------------- changes

#[tauri::command]
pub async fn list_changes(state: Ctx<'_>) -> ZResult<Vec<ChangeSetSummary>> {
  let journal = state.journal.lock().map_err(|_| ZenouError::state("Journal unavailable."))?;
  journal.list()
}

#[tauri::command]
pub async fn get_change(state: Ctx<'_>, id: String) -> ZResult<ChangeSet> {
  let journal = state.journal.lock().map_err(|_| ZenouError::state("Journal unavailable."))?;
  journal.get(&id)
}

/// Restore all tweaks touched by a change-set (from its recorded backup).
#[tauri::command]
pub async fn restore_change_set(app: AppHandle, id: String) -> ZResult<GroupResult> {
  let state = app.state::<AppState>();
  let change = {
    let journal = state.journal.lock().map_err(|_| ZenouError::state("Journal unavailable."))?;
    journal.get(&id)?
  };
  let backup_id = change.backup_id.clone().ok_or_else(|| {
    ZenouError::state("This change was not paired with a backup, so it cannot be restored automatically. Restore the individual tweaks instead.")
  })?;
  let store = state.backends_lock_hack()?;
  let snapshot = store.get(&backup_id)?;
  drop(store);
  let mut results = Vec::new();
  for entry in &snapshot.entries {
    let mut steps = Vec::new();
    for v in &entry.values {
      match registry::restore(v) {
        Ok(()) => steps.push(types::StepOutcome { location: format!("{}\\{}", v.hive, v.path), ok: true, error: None }),
        Err(e) => steps.push(types::StepOutcome { location: format!("{}\\{}", v.hive, v.path), ok: false, error: Some(e.message) }),
      }
    }
    let success = steps.iter().all(|s| s.ok);
    results.push(TweakResult {
      tweak_id: entry.tweak_id.clone(),
      success,
      skipped: false,
      error: if success { None } else { Some("Some values could not be restored.".into()) },
      details: None,
      restart_required: false,
      steps,
    });
  }
  let group = GroupResult { label: format!("Restore change \"{}\"", change.label), results: results.clone(), restore_point_created: false };
  log_group(&state, "restore", &group.label, &results);
  Ok(group)
}

// ---------------------------------------------------------------- logs

#[tauri::command]
pub async fn read_logs(state: Ctx<'_>) -> ZResult<Vec<LogEntry>> {
  let log = state.log.lock().map_err(|_| ZenouError::state("Log unavailable."))?;
  log.read()
}

#[tauri::command]
pub async fn clear_logs(state: Ctx<'_>) -> ZResult<()> {
  let log = state.log.lock().map_err(|_| ZenouError::state("Log unavailable."))?;
  log.clear()
}

#[tauri::command]
pub async fn export_logs(state: Ctx<'_>, app: AppHandle, target: String) -> ZResult<String> {
  let log = state.log.lock().map_err(|_| ZenouError::state("Log unavailable."))?;
  let path = PathBuf::from(&target);
  // Guard: only write where the user picked via dialog (dialog returns absolute path).
  if !path.is_absolute() {
    return Err(ZenouError::state("Choose a save location through the save dialog."));
  }
  let _ = app;
  log.export_to(&path)
}

#[tauri::command]
pub async fn pick_save_file(app: AppHandle, file_name: String) -> ZResult<Option<String>> {
  let (tx, rx) = std::sync::mpsc::channel();
  app
    .dialog()
    .file()
    .set_file_name(&file_name)
    .save_file(move |path| {
      let _ = tx.send(path.map(|p| p.to_string()));
    });
  rx.recv()
    .map_err(|_| ZenouError::state("The save dialog was closed."))
}

// ---------------------------------------------------------------- settings

#[tauri::command]
pub async fn get_settings(state: Ctx<'_>) -> ZResult<AppSettings> {
  let store = state.settings.lock().map_err(|_| ZenouError::state("Settings unavailable."))?;
  store.load()
}

#[tauri::command]
pub async fn set_settings(app: AppHandle, settings: AppSettings) -> ZResult<()> {
  let state = app.state::<AppState>();
  let store = state.settings.lock().map_err(|_| ZenouError::state("Settings unavailable."))?;
  store.save(&settings)?;
  drop(store);
  // Keep autostart in sync (plugin manages the registry Run entry itself).
  {
    use tauri_plugin_autostart::ManagerExt;
    let auto = app.autolaunch();
    if settings.start_with_windows {
      let _ = auto.enable();
    } else {
      let _ = auto.disable();
    }
  }
  Ok(())
}

#[tauri::command]
pub async fn toggle_favorite(state: Ctx<'_>, id: String) -> ZResult<Vec<String>> {
  let store = state.settings.lock().map_err(|_| ZenouError::state("Settings unavailable."))?;
  let mut settings = store.load()?;
  if settings.favorites.contains(&id) {
    settings.favorites.retain(|f| f != &id);
  } else {
    settings.favorites.push(id);
  }
  store.save(&settings)?;
  Ok(settings.favorites)
}

// ---------------------------------------------------------------- cleanup

#[tauri::command]
pub async fn cleanup_previews() -> Vec<crate::cleanup::CleanupPreview> {
  vec![
    crate::cleanup::preview_user_temp(),
    crate::cleanup::preview_windows_temp(),
    crate::cleanup::preview_prefetch(),
    crate::cleanup::preview_windows_update_cache(),
  ]
}

#[tauri::command]
pub async fn run_cleanup(state: Ctx<'_>, id: String) -> ZResult<(u64, u64, u64)> {
  let result = match id.as_str() {
    "cleanup.user-temp" => crate::cleanup::run_user_temp_cleanup(),
    "cleanup.windows-temp" => crate::cleanup::run_windows_temp_cleanup(),
    "cleanup.prefetch" => crate::cleanup::run_prefetch_cleanup(),
    "cleanup.wu-cache" => crate::cleanup::run_windows_update_cache_cleanup(),
    other => return Err(ZenouError::state(format!("Unknown cleanup operation \"{other}\"."))),
  };
  if let Ok(log) = state.log.lock() {
    match &result {
      Ok((files, _, _)) => log.append(&LogEntry {
        timestamp: chrono::Utc::now(),
        action: "cleanup".into(),
        tweak_id: Some(id.clone()),
        tweak_name: Some(id.clone()),
        result: "success".into(),
        message: format!("{id}: {files} items removed."),
        error_details: None,
      }),
      Err(e) => log.append(&LogEntry {
        timestamp: chrono::Utc::now(),
        action: "cleanup".into(),
        tweak_id: Some(id.clone()),
        tweak_name: Some(id.clone()),
        result: "failed".into(),
        message: e.message.clone(),
        error_details: e.details.clone(),
      }),
    }
  }
  result
}

// ---------------------------------------------------------------- restore point

#[tauri::command]
pub async fn create_restore_point(state: Ctx<'_>) -> ZResult<String> {
  restore_point::create_restore_point("Zenou Tweaks — manual")?;
  if let Ok(log) = state.log.lock() {
    log.append(&LogEntry {
      timestamp: chrono::Utc::now(),
      action: "system".into(),
      tweak_id: None,
      tweak_name: None,
      result: "success".into(),
      message: "Created a Windows restore point.".into(),
      error_details: None,
    });
  }
  let _ = state;
  Ok("Windows restore point created.".into())
}

// ---------------------------------------------------------------- elevation

/// Ask Windows to start a new elevated instance of Zenou Tweaks (UAC prompt).
/// The current window stays open; if the user approves, the elevated instance
/// takes over and this one can be closed by the user.
#[tauri::command]
pub async fn request_elevation() -> ZResult<()> {
  let exe = std::env::current_exe()
    .map_err(|_| ZenouError::state("Could not locate the running application."))?;
  let exe_str = exe.to_string_lossy().to_string();
  crate::winapi::run_elevated(&exe_str, "")
}

impl AppState {
  /// Small internal helper: lock the backup store or fail with a state error.
  fn backends_lock_hack(&self) -> ZResult<std::sync::MutexGuard<'_, BackupStore>> {
    self
      .backups
      .lock()
      .map_err(|_| ZenouError::state("Backups are unavailable."))
  }
}

pub fn format_steps_public(steps: &[types::StepOutcome]) -> String {
  format_steps(steps)
}

#[allow(dead_code)]
fn unused(_c: catalog_ext::CapturedValue) {}
