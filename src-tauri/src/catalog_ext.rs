//! Additional tweak-building support: dynamic edits, tasks and services handling.
//!
//! State verification reads the authoritative registry directly (Services' Start
//! values, Tasks' dynamic vault entries) — the same source schtasks/net write —
//! so `check_state` never spawns a subprocess. Mutations still go through
//! schtasks/net (documented, supported interfaces) with captured originals.

use std::sync::Arc;

use crate::engine::{RegEdit, RegEditKind, Tweak};
use crate::error::{ZenouError, ZResult};
use crate::ops;
use crate::types::{SavedValue, StepOutcome};

/// A tweak that applies to scheduled tasks. Original enable/disable state is
/// captured per task before mutation.
pub struct TaskTweak {
  pub tweak: Tweak,
  pub tasks: Vec<String>,
}

/// A tweak that changes service start types. Original Start values are
/// captured per service before mutation.
pub struct ServiceTweak {
  pub tweak: Tweak,
  pub services: Vec<String>,
}

thread_local! {
  static TASK_CAPTURES: std::cell::RefCell<Vec<(String, bool)>> = const { std::cell::RefCell::new(Vec::new()) };
  static SERVICE_CAPTURES: std::cell::RefCell<Vec<(String, Option<u32>)>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub const TELEMETRY_TASKS: &[&str] = &[
  r"\Microsoft\Windows\Application Experience\Microsoft Compatibility Appraiser",
  r"\Microsoft\Windows\Application Experience\ProgramDataUpdater",
  r"\Microsoft\Windows\Application Experience\StartupAppTask",
  r"\Microsoft\Windows\Application Experience\PcaPatchDbTask",
  r"\Microsoft\Windows\Customer Experience Improvement Program\Consolidator",
  r"\Microsoft\Windows\Customer Experience Improvement Program\UsbCeip",
  r"\Microsoft\Windows\Customer Experience Improvement Program\BthSQM",
  r"\Microsoft\Windows\Customer Experience Improvement Program\KernelCeipTask",
  r"\Microsoft\Windows\Customer Experience Improvement Program\Uploader",
  r"\Microsoft\Windows\DiskDiagnostic\Microsoft-Windows-DiskDiagnosticDataCollector",
  r"\Microsoft\Windows\DiskDiagnostic\Microsoft-Windows-DiskDiagnosticResolver",
  r"\Microsoft\Windows\Maintenance\WinSAT",
  r"\Microsoft\Windows\Windows Error Reporting\QueueReporting",
  r"\Microsoft\Windows\Autochk\Proxy",
  r"\Microsoft\Windows\NetTrace\GatherNetworkInfo",
  r"\Microsoft\Windows\PI\Sqm-Tasks",
  r"\Microsoft\Windows\Power Efficiency Diagnostics\AnalyzeSystem",
  r"\Microsoft\Windows\DiskFootprint\Diagnostics",
  r"\Microsoft\Windows\Shell\FamilySafetyMonitor",
  r"\Microsoft\Windows\Shell\FamilySafetyRefresh",
  r"\Microsoft\Windows\Shell\FamilySafetyUpload",
  r"\Microsoft\Windows\CloudExperienceHost\CreateObjectTask",
  r"\Microsoft\Windows\Device Information\Device",
  r"\Microsoft\Windows\Device Information\Device User",
  r"\Microsoft\Windows\Feedback\Siuf\DmClient",
  r"\Microsoft\Windows\Feedback\Siuf\DmClientOnScenarioDownload",
  r"\Microsoft\Windows\Diagnosis\RecommendedTroubleshootingScanner",
  r"\Microsoft\Windows\Diagnosis\Scheduled",
  r"\Microsoft\Windows\Maps\MapsToastTask",
  r"\Microsoft\Windows\Maps\MapsUpdateTask",
  r"\Microsoft\Windows\RetailDemo\CleanupOfflineContent",
];

pub const UNNECESSARY_SERVICES: &[&str] = &[
  "RemoteRegistry",
  "RetailDemo",
  "WalletService",
  "PhoneSvc",
  "Fax",
  "WMPNetworkSvc",
  "WerSvc",
  "MapsBroker",
  "lfsvc",
  "SharedAccess",
  "RemoteAccess",
  "WpcMonSvc",
  "SEMgrSvc",
  "SNMPTRAP",
  "AJRouter",
  "scardSvr",
  "Phone-Svc",
  "SmsRouter",
];

pub const NVIDIA_TELEMETRY_TASKS: &[&str] = &[
  r"NvTmRep_CrashReport1_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
  r"NvTmRep_CrashReport2_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
  r"NvTmRep_CrashReport3_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
  r"NvTmRep_CrashReport4_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
  r"NvDriverUpdateCheckDaily_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
  r"NVIDIA GeForce Experience SelfUpdate_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
  r"NvTmMon_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
];

// --------------------------------------------------------------------------
// Hook builders
// --------------------------------------------------------------------------

/// (apply, verify, restore) hook triple shared by group tweaks.
pub type HookTriple = (
  Arc<dyn Fn() -> ZResult<()> + Send + Sync>,
  Arc<dyn Fn() -> ZResult<bool> + Send + Sync>,
  Arc<dyn Fn() -> ZResult<()> + Send + Sync>,
);

/// Build hooks for a task-disabling tweak. Apply/restore mutate via schtasks
/// (captured originals); verify reads the registry vault (instant).
pub fn task_disable_hooks(
  tasks: Vec<String>,
) -> HookTriple {
  let apply_tasks = tasks.clone();
  let verify_tasks = tasks.clone();
  let restore_tasks = tasks;

  let apply = Arc::new(move || -> ZResult<()> {
    let mut captured = Vec::new();
    for task in &apply_tasks {
      // Snapshot state when available; fall back to a per-task schtasks query.
      let enabled = match ops::disabled_tasks_snapshot() {
        Some(set) => {
          let full = if task.starts_with('\\') { task.clone() } else { format!("\\{task}") };
          !set.contains(&full)
        }
        None => ops::task_state(task).ok().flatten().unwrap_or(true),
      };
      if !enabled {
        captured.push((task.clone(), false));
        continue;
      }
      captured.push((task.clone(), true));
      match ops::set_task_enabled(task, false) {
        Ok(()) => {}
        // Task not present on this system: skip gracefully.
        Err(e) if e.code == "task-absent" => {}
        Err(e) => {
          // Roll back already-disabled tasks before failing.
          for (t, was_enabled) in &captured {
            if *was_enabled {
              let _ = ops::set_task_enabled(t, true);
            }
          }
          return Err(e);
        }
      }
    }
    TASK_CAPTURES.with(|c| *c.borrow_mut() = captured);
    Ok(())
  });

  let verify = Arc::new(move || -> ZResult<bool> {
    // One cached snapshot covers every task (single subprocess when cold).
    let Some(disabled) = ops::disabled_tasks_snapshot() else {
      return Ok(false); // cannot determine -> not verified
    };
    for task in &verify_tasks {
      let full = if task.starts_with('\\') { task.clone() } else { format!("\\{task}") };
      if !disabled.contains(&full) {
        return Ok(false); // present and not disabled -> not applied
      }
    }
    Ok(true)
  });

  let restore = Arc::new(move || -> ZResult<()> {
    let captured = TASK_CAPTURES.with(|c| std::mem::take(&mut *c.borrow_mut()));
    let mut last_err = None;
    // Prefer persisted captures if present.
    let pairs: Vec<(String, bool)> = if captured.is_empty() {
      load_task_captures(&restore_tasks)
    } else {
      captured
    };
    for (task, was_enabled) in pairs {
      if was_enabled {
        if let Err(e) = ops::set_task_enabled(&task, true) {
          if e.code != "task-absent" {
            last_err = Some(e);
          }
        }
      }
    }
    match last_err {
      Some(e) => Err(e),
      None => Ok(()),
    }
  });

  (apply, verify, restore)
}

pub fn service_disable_hooks(
  services: Vec<String>,
) -> HookTriple {
  let apply_services = services.clone();
  let verify_services = services.clone();
  let restore_services = services;

  let apply = Arc::new(move || -> ZResult<()> {
    let mut captured: Vec<(String, Option<u32>)> = Vec::new();
    for svc in &apply_services {
      let original = ops::service_start_value(svc).unwrap_or(None);
      captured.push((svc.clone(), original));
      if original == Some(4) {
        continue; // already disabled
      }
      // Service may not exist (WpcMonSvc on Home, Fax, etc.) — skip gracefully.
      if original.is_none() {
        continue;
      }
      match ops::set_service_start(svc, ops::ServiceStart::Disabled) {
        Ok(()) => {}
        Err(e) => {
          for (s, orig) in &captured {
            if let Some(v) = orig {
              if *v != 4 {
                let _ = ops::set_service_start(s, ops::ServiceStart::from_start_value(*v));
              }
            }
          }
          return Err(e);
        }
      }
    }
    SERVICE_CAPTURES.with(|c| *c.borrow_mut() = captured);
    Ok(())
  });

  let verify = Arc::new(move || -> ZResult<bool> {
    // Registry-only (Services\...\Start) — instant, no subprocess.
    for svc in &verify_services {
      match ops::service_start_value(svc) {
        Ok(Some(4)) | Ok(None) => {}
        Ok(Some(_)) => return Ok(false),
        Err(_) => return Ok(false),
      }
    }
    Ok(true)
  });

  let restore = Arc::new(move || -> ZResult<()> {
    let captured = SERVICE_CAPTURES.with(|c| std::mem::take(&mut *c.borrow_mut()));
    let pairs: Vec<(String, Option<u32>)> = if captured.is_empty() {
      load_service_captures(&restore_services)
    } else {
      captured
    };
    let mut last_err = None;
    for (svc, original) in pairs {
      if let Some(v) = original {
        if v != 4 {
          if let Err(e) = ops::set_service_start(&svc, ops::ServiceStart::from_start_value(v)) {
            last_err = Some(e);
          }
        }
      }
    }
    match last_err {
      Some(e) => Err(e),
      None => Ok(()),
    }
  });

  (apply, verify, restore)
}

fn captures_file() -> Option<std::path::PathBuf> {
  crate::APP_PATHS.get().map(|p| p.root.join("captures.json"))
}

fn load_task_captures(tasks: &[String]) -> Vec<(String, bool)> {
  let Some(path) = captures_file() else { return Vec::new() };
  let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
  let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return Vec::new() };
  let mut out = Vec::new();
  for t in tasks {
    let entry = v["tasks"].get(t);
    let enabled = entry.and_then(|e| e.as_bool()).unwrap_or(true);
    out.push((t.clone(), enabled));
  }
  out
}

fn load_service_captures(services: &[String]) -> Vec<(String, Option<u32>)> {
  let Some(path) = captures_file() else { return Vec::new() };
  let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
  let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return Vec::new() };
  let mut out = Vec::new();
  for s in services {
    let entry = v["services"].get(s);
    let val = entry.and_then(|e| e.as_u64()).map(|x| x as u32);
    out.push((s.clone(), val));
  }
  out
}

/// Persist captures for restore-after-restart scenarios.
pub fn persist_captures(task_pairs: &[(String, bool)], service_pairs: &[(String, Option<u32>)]) -> ZResult<()> {
  let Some(path) = captures_file() else {
    return Err(ZenouError::state("Application paths are not initialized."));
  };
  let mut tasks = serde_json::Map::new();
  for (t, enabled) in task_pairs {
    tasks.insert(t.clone(), serde_json::Value::Bool(*enabled));
  }
  let mut services = serde_json::Map::new();
  for (s, val) in service_pairs {
    if let Some(v) = val {
      services.insert(s.clone(), serde_json::Value::from(*v as u64));
    }
  }
  let doc = serde_json::json!({ "tasks": tasks, "services": services });
  std::fs::write(path, serde_json::to_string_pretty(&doc)?)?;
  Ok(())
}

/// Persist captures from the thread-local state after a task/service apply.
pub fn persist_pending_captures() -> ZResult<()> {
  let tasks = TASK_CAPTURES.with(|c| c.borrow().clone());
  let services = SERVICE_CAPTURES.with(|c| c.borrow().clone());
  if tasks.is_empty() && services.is_empty() {
    return Ok(());
  }
  persist_captures(&tasks, &services)
}

/// Attach task/service hooks to a tweak by id (called at catalog build time).
pub fn attach_group_hooks(tweak: &mut Tweak, kind: &str) {
  match kind {
    "win.telemetry-tasks" => {
      let (a, v, r) = task_disable_hooks(TELEMETRY_TASKS.iter().map(|s| s.to_string()).collect());
      tweak.extra_apply = Some(a);
      tweak.extra_verify = Some(v);
      tweak.extra_restore = Some(r);
    }
    "win.unnecessary-services" => {
      let (a, v, r) = service_disable_hooks(UNNECESSARY_SERVICES.iter().map(|s| s.to_string()).collect());
      tweak.extra_apply = Some(a);
      tweak.extra_verify = Some(v);
      tweak.extra_restore = Some(r);
    }
    "gpu.nvidia-telemetry" => {
      let (a, v, r) = task_disable_hooks(NVIDIA_TELEMETRY_TASKS.iter().map(|s| s.to_string()).collect());
      tweak.extra_apply = Some(a);
      tweak.extra_verify = Some(v);
      tweak.extra_restore = Some(r);
    }
    _ => {}
  }
}

/// Step outcome helper used by the cleanup module for file operations.
pub fn file_step(location: &str, result: ZResult<usize>) -> (StepOutcome, usize) {
  match result {
    Ok(count) => (StepOutcome { location: location.into(), ok: true, error: None }, count),
    Err(e) => (StepOutcome { location: location.into(), ok: false, error: Some(e.message) }, 0),
  }
}

/// SavedValue re-export for the cleanup module.
pub type CapturedValue = SavedValue;

use crate::registry::Hive;

#[allow(dead_code)]
pub fn make_edit(hive: Hive, path: &str, name: &str, kind: RegEditKind) -> RegEdit {
  RegEdit { hive, path: path.into(), name: name.into(), kind }
}
