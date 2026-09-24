//! Windows restore point creation via Checkpoint-Computer.

use crate::error::{ZenouError, ZResult};
use crate::winapi::run_powershell;

const RESTORE_POINT_INTERVAL_KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore";
const CREATE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// Create a system restore point. Returns Ok(()) on success; a friendly error
/// otherwise (System Restore disabled is reported clearly).
pub fn create_restore_point(description: &str) -> ZResult<()> {
  // Ensure SystemRestorePointCreationFrequency=0 so Checkpoint-Computer
  // is allowed even when one was created recently (same approach as the .bat).
  let _ = crate::registry::set_dword(
    crate::registry::Hive::LocalMachine,
    RESTORE_POINT_INTERVAL_KEY,
    "SystemRestorePointCreationFrequency",
    0,
  );

  let script = format!(
    "Checkpoint-Computer -Description \"{}\" -RestorePointType MODIFY_SETTINGS",
    description.replace('"', "'")
  );
  let out = run_powershell(&script, CREATE_TIMEOUT)?;
  if out.exit_code != 0 {
    let err = out.stderr_str();
    if err.to_lowercase().contains("not enabled") || err.to_lowercase().contains("disabled") {
      return Err(ZenouError::state(
        "System Restore is disabled on this computer, so a restore point could not be created. You can enable it in Windows (System Properties > System Protection) or continue without one.",
      ));
    }
    return Err(ZenouError::command(
      "Windows could not create a restore point.",
      Some(out.combined()),
    ));
  }
  Ok(())
}

/// Whether System Restore is enabled for the system drive (best-effort check).
pub fn system_restore_enabled() -> bool {
  let out = run_powershell(
    "try { (Get-ComputerRestorePoint -ErrorAction Stop | Measure-Object).Count -ge 0 } catch { $false }",
    std::time::Duration::from_secs(20),
  );
  out.map(|o| o.exit_code == 0 && o.stdout_str().trim().eq_ignore_ascii_case("true")).unwrap_or(false)
}
