//! Whitelisted system operation wrappers. Every function uses fixed arguments —
//! no frontend-supplied command text, no arbitrary shell.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{ZenouError, ZResult};
use crate::winapi::{run_captured, run_powershell, Output};

const TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStart {
  Boot,
  System,
  Auto,
  Demand,
  Disabled,
}

impl ServiceStart {
  pub fn to_start_value(self) -> u32 {
    match self {
      ServiceStart::Boot => 0,
      ServiceStart::System => 1,
      ServiceStart::Auto => 2,
      ServiceStart::Demand => 3,
      ServiceStart::Disabled => 4,
    }
  }

  pub fn from_start_value(v: u32) -> ServiceStart {
    match v {
      0 => ServiceStart::Boot,
      1 => ServiceStart::System,
      2 => ServiceStart::Auto,
      3 => ServiceStart::Demand,
      _ => ServiceStart::Disabled,
    }
  }

  pub fn label(self) -> &'static str {
    match self {
      ServiceStart::Boot => "Boot",
      ServiceStart::System => "System",
      ServiceStart::Auto => "Automatic",
      ServiceStart::Demand => "Manual",
      ServiceStart::Disabled => "Disabled",
    }
  }
}

/// Read a service's current Start value from the registry (services live in HKLM).
pub fn service_start_value(service: &str) -> ZResult<Option<u32>> {
  let path = format!("SYSTEM\\CurrentControlSet\\Services\\{service}");
  match crate::registry::read_raw(crate::registry::Hive::LocalMachine, &path, "Start")? {
    Some(v) if v.vtype == winreg::enums::RegType::REG_DWORD && v.bytes.len() == 4 => {
      Ok(Some(u32::from_le_bytes([v.bytes[0], v.bytes[1], v.bytes[2], v.bytes[3]])))
    }
    Some(_) => Ok(None),
    None => Ok(None),
  }
}

/// Change a service's start type via the Service Control Manager (native API path).
pub fn set_service_start(service: &str, start: ServiceStart) -> ZResult<()> {
  let out = run_captured(
    "net.exe",
    &["config", service, "start=", match start {
      ServiceStart::Boot => "boot",
      ServiceStart::System => "system",
      ServiceStart::Auto => "auto",
      ServiceStart::Demand => "demand",
      ServiceStart::Disabled => "disabled",
    }],
    TIMEOUT,
  )?;
  if out.exit_code != 0 {
    return Err(service_change_error(service, &out));
  }
  Ok(())
}

fn service_change_error(service: &str, out: &Output) -> ZenouError {
  let err = out.stderr_str();
  let friendly = if err.contains("1060") || err.to_lowercase().contains("does not exist") {
    format!("The service \"{service}\" is not installed on this system.")
  } else if err.contains("5") {
    "Administrator permission is required to change this service. Approve the permission prompt and try again.".to_string()
  } else {
    format!("Windows refused to change the service \"{service}\".")
  };
  ZenouError::command(friendly, Some(out.combined()))
}

/// Enable or disable a scheduled task via schtasks.
pub fn set_task_enabled(task_path: &str, enabled: bool) -> ZResult<()> {
  invalidate_task_snapshot(); // state is about to change
  let flag = if enabled { "/ENABLE" } else { "/DISABLE" };
  let out = run_captured("schtasks.exe", &["/Change", "/TN", task_path, flag], TIMEOUT)?;
  if out.exit_code != 0 {
    let err = out.stderr_str();
    if err.to_lowercase().contains("does not exist") || err.contains("ERROR: The system cannot find") {
      return Err(ZenouError::new(
        "task-absent",
        format!("The scheduled task \"{task_path}\" is not present on this system."),
        Some(out.combined()),
      ));
    }
    return Err(ZenouError::command(
      format!("Windows refused to change the scheduled task \"{task_path}\"."),
      Some(out.combined()),
    ));
  }
  Ok(())
}

/// Query whether a scheduled task exists (schtasks /Query).
pub fn task_exists(task_path: &str) -> bool {
  run_captured("schtasks.exe", &["/Query", "/TN", task_path], TIMEOUT)
    .map(|o| o.exit_code == 0)
    .unwrap_or(false)
}

/// One cached snapshot of every scheduled task and its state, taken with a
/// single PowerShell call (Get-ScheduledTask). Replaces one schtasks query
/// per task on the startup state-check path; cleared on set_task_enabled.
#[derive(Clone)]
struct TaskEntry {
  name: String,   // TaskPath + TaskName (leading backslash included)
  disabled: bool,
}

static TASKS_CACHE: std::sync::Mutex<Option<Option<Vec<TaskEntry>>>> =
  std::sync::Mutex::new(None);

fn invalidate_task_snapshot() {
  if let Ok(mut guard) = TASKS_CACHE.lock() {
    *guard = None;
  }
}

fn task_snapshot() -> Option<Vec<TaskEntry>> {
  if let Ok(guard) = TASKS_CACHE.lock() {
    if let Some(cached) = guard.as_ref() {
      return cached.clone();
    }
  }
  let out = run_powershell(
    "Get-ScheduledTask | ForEach-Object { \"$($_.TaskPath)$($_.TaskName)|$($_.State)\" }",
    Duration::from_secs(30),
  )
  .ok()?;
  if out.exit_code != 0 {
    if let Ok(mut guard) = TASKS_CACHE.lock() {
      *guard = Some(None);
    }
    return None;
  }
  let entries: Vec<TaskEntry> = out
    .stdout_str()
    .lines()
    .filter_map(|l| {
      let l = l.trim().trim_end_matches('\0');
      let (name, state) = l.rsplit_once('|')?;
      Some(TaskEntry {
        name: name.to_string(),
        disabled: state.eq_ignore_ascii_case("Disabled"),
      })
    })
    .collect();
  let snapshot = Some(entries);
  if let Ok(mut guard) = TASKS_CACHE.lock() {
    *guard = Some(snapshot.clone());
  }
  snapshot
}


/// Names of every disabled task. None = snapshot unavailable.
pub fn disabled_tasks_snapshot() -> Option<std::collections::HashSet<String>> {
  Some(
    task_snapshot()?
      .into_iter()
      .filter(|e| e.disabled)
      .map(|e| e.name)
      .collect(),
  )
}

/// Whether a task exists, from the cached snapshot (no extra subprocess).
pub fn task_present(task_path: &str) -> Option<bool> {
  let full = if task_path.starts_with('\\') {
    task_path.to_string()
  } else {
    format!("\\{task_path}")
  };
  Some(task_snapshot()?.iter().any(|e| e.name == full))
}

pub fn task_state(task_path: &str) -> ZResult<Option<bool>> {
  if !task_exists(task_path) {
    return Ok(None);
  }
  let out = run_captured("schtasks.exe", &["/Query", "/TN", task_path, "/FO", "LIST", "/V"], TIMEOUT)?;
  if out.exit_code != 0 {
    return Err(ZenouError::command(
      format!("Could not read the state of scheduled task \"{task_path}\"."),
      Some(out.combined()),
    ));
  }
  let text = out.stdout_str();
  // "Scheduled Task State" appears as "Disabled" or "Enabled".
  let state = text
    .lines()
    .find(|l| l.contains("Scheduled Task State"))
    .map(|l| l.to_lowercase().contains("enabled"));
  Ok(state)
}

/// Power configuration (powercfg). All argument sets are fixed by the engine.
pub fn powercfg(args: &[&str]) -> ZResult<Output> {
  let out = run_captured("powercfg.exe", args, TIMEOUT)?;
  if out.exit_code != 0 {
    return Err(ZenouError::command(
      "Windows (powercfg) refused the power configuration change.",
      Some(out.combined()),
    ));
  }
  Ok(out)
}

/// List power schemes as (guid, name).
pub fn list_power_schemes() -> ZResult<Vec<(String, String)>> {
  let out = powercfg(&["/list"])?;
  let mut schemes = Vec::new();
  for line in out.stdout_str().lines() {
    if let Some(idx) = line.find("GUID") {
      // Format: "  Power Scheme GUID: xxxx  (Name)"
      if let Some(rest) = line.get(idx + 5..) {
        let guid = rest.split_whitespace().next().unwrap_or("").to_string();
        if let Some(open) = line.rfind('(') {
          if let Some(close) = line.rfind(')') {
            if close > open {
              schemes.push((guid, line[open + 1..close].to_string()));
            }
          }
        }
      }
    }
  }
  Ok(schemes)
}

/// Duplicate a scheme and return its new GUID by diffing scheme lists.
pub fn duplicate_scheme(source_guid: &str) -> ZResult<String> {
  let before = list_power_schemes()?;
  powercfg(&["-duplicatescheme", source_guid])?;
  let after = list_power_schemes()?;
  after
    .iter()
    .find(|(guid, _)| !before.iter().any(|(g, _)| g == guid))
    .map(|(guid, _)| guid.clone())
    .ok_or_else(|| ZenouError::state("The power plan was created but could not be identified."))
}

/// Make a scheme active.
pub fn set_active_scheme(guid: &str) -> ZResult<()> {
  powercfg(&["-setactive", guid]).map(|_| ())
}

/// The currently active scheme GUID.
pub fn active_scheme() -> ZResult<Option<String>> {
  let out = powercfg(&["/getactivescheme"])?;
  if out.exit_code != 0 {
    return Ok(None);
  }
  let text = out.stdout_str();
  Ok(text
    .lines()
    .find(|l| l.contains("GUID"))
    .and_then(|l| {
      let idx = l.find("GUID:")?;
      l.get(idx + 5..)?.split_whitespace().next().map(|s| s.to_string())
    }))
}

/// Delete a scheme by GUID.
pub fn delete_scheme(guid: &str) -> ZResult<()> {
  powercfg(&["-delete", guid]).map(|_| ())
}

/// fsutil behavior setters (fixed operations only).
pub fn fsutil_set(operation: &str, value: &str) -> ZResult<()> {
  let out = run_captured("fsutil.exe", &["behavior", "set", operation, value], TIMEOUT)?;
  if out.exit_code != 0 {
    return Err(ZenouError::command(
      "Windows (fsutil) refused the file system configuration change.",
      Some(out.combined()),
    ));
  }
  Ok(())
}

pub fn fsutil_get(operation: &str) -> ZResult<String> {
  let out = run_captured("fsutil.exe", &["behavior", "query", operation], TIMEOUT)?;
  Ok(out.stdout_str())
}

/// Netsh fixed operations for TCP globals.
pub fn netsh_tcp_global(setting: &str, value: &str) -> ZResult<()> {
  let out = run_captured(
    "netsh.exe",
    &["int", "tcp", "set", "global", setting, "=", value],
    TIMEOUT,
  )?;
  if out.exit_code != 0 {
    return Err(ZenouError::command(
      "Windows (netsh) refused the network configuration change.",
      Some(out.combined()),
    ));
  }
  Ok(())
}

/// Disable Windows memory compression via MMAgent (fixed operation).
pub fn disable_memory_compression() -> ZResult<()> {
  let out = run_powershell("Disable-MMAgent -mc", Duration::from_secs(60))?;
  if out.exit_code != 0 {
    return Err(ZenouError::command(
      "Windows refused to disable memory compression.",
      Some(out.combined()),
    ));
  }
  invalidate_memory_compression_cache(); // re-read on next verify
  Ok(())
}

pub fn enable_memory_compression() -> ZResult<()> {
  let out = run_powershell("Enable-MMAgent -mc", Duration::from_secs(60))?;
  if out.exit_code != 0 {
    return Err(ZenouError::command(
      "Windows refused to enable memory compression.",
      Some(out.combined()),
    ));
  }
  invalidate_memory_compression_cache(); // re-read on next verify
  Ok(())
}

/// Outer None = not cached yet; inner None = queried and undeterminable.
static MEM_COMPRESSION_CACHE: std::sync::Mutex<Option<Option<bool>>> =
  std::sync::Mutex::new(None);

fn invalidate_memory_compression_cache() {
  if let Ok(mut guard) = MEM_COMPRESSION_CACHE.lock() {
    *guard = None;
  }
}

/// Whether memory compression is enabled. Cached until a mutation changes it —
/// this is the only subprocess call on the startup state-check path, and the
/// value only changes through disable/enable above (which clear the cache so
/// post-apply verification re-reads fresh state).
pub fn memory_compression_enabled() -> ZResult<Option<bool>> {
  if let Ok(guard) = MEM_COMPRESSION_CACHE.lock() {
    if let Some(cached) = *guard {
      return Ok(cached);
    }
  }
  let out = run_powershell("(Get-MMAgent).MemoryCompression", Duration::from_secs(60))?;
  let value = if out.exit_code != 0 {
    None
  } else {
    Some(out.stdout_str().trim().eq_ignore_ascii_case("true"))
  };
  if let Ok(mut guard) = MEM_COMPRESSION_CACHE.lock() {
    *guard = Some(value);
  }
  Ok(value)
}

/// Flush the DNS resolver cache.
pub fn flush_dns() -> ZResult<()> {
  let out = run_captured("ipconfig.exe", &["/flushdns"], TIMEOUT)?;
  if out.exit_code != 0 {
    return Err(ZenouError::command(
      "The DNS cache could not be flushed.",
      Some(out.combined()),
    ));
  }
  Ok(())
}

/// Stop or start a service now (SCM), leaving its start type untouched.
pub fn stop_service_now(service: &str) -> ZResult<()> {
  let out = run_captured("net.exe", &["stop", service, "/y"], TIMEOUT)?;
  let err = out.stderr_str();
  if out.exit_code != 0 && !err.to_lowercase().contains("not started") && !err.to_lowercase().contains("has not been started") {
    return Err(service_change_error(service, &out));
  }
  Ok(())
}

pub fn start_service_now(service: &str) -> ZResult<()> {
  let out = run_captured("net.exe", &["start", service], TIMEOUT)?;
  let err = out.stderr_str();
  if out.exit_code != 0 && !err.to_lowercase().contains("already") {
    return Err(service_change_error(service, &out));
  }
  Ok(())
}
