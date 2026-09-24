//! Native Windows helpers: elevation, process launch, PowerShell MMAgent.

use windows::core::PCWSTR;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::error::{ZenouError, ZResult};

fn to_wide(s: &str) -> Vec<u16> {
  s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Whether the current process token has administrator elevation.
pub fn is_elevated() -> bool {
  unsafe {
    let mut token = HANDLE::default();
    if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
      return false;
    }
    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut ret_len = 0u32;
    let ok = GetTokenInformation(
      token,
      TokenElevation,
      Some(&mut elevation as *mut _ as *mut _),
      std::mem::size_of::<TOKEN_ELEVATION>() as u32,
      &mut ret_len,
    );
    let _ = windows::Win32::Foundation::CloseHandle(token);
    ok.is_ok() && elevation.TokenIsElevated != 0
  }
}

/// Launch a fixed executable (never frontend-controlled) requesting admin elevation.
/// Returns Err with code "cancelled" when the user declines the UAC prompt.
pub fn run_elevated(exe: &str, args: &str) -> ZResult<()> {
  let exe_w = to_wide(exe);
  let args_w = to_wide(args);
  let verb = to_wide("runas");
  let dir = to_wide(std::env::temp_dir().to_string_lossy().as_ref());
  unsafe {
    let result = ShellExecuteW(
      None,
      PCWSTR(verb.as_ptr()),
      PCWSTR(exe_w.as_ptr()),
      PCWSTR(args_w.as_ptr()),
      PCWSTR(dir.as_ptr()),
      SW_SHOWNORMAL,
    );
    // ShellExecuteW returns a value > 32 on success.
    if result.0 as usize <= 32 {
      let code = result.0 as usize;
      // SE_ERR_ACCESSDENIED (5) is what UAC cancellation produces.
      if code == 5 {
        return Err(ZenouError::cancelled(
          "Administrator permission was not granted, so this action was not performed.",
        ));
      }
      return Err(ZenouError::permission(
        "Windows could not start the elevated helper for this action.",
        Some(format!("ShellExecuteW returned {code}")),
      ));
    }
  }
  Ok(())
}

/// Run a helper process with the given fixed arguments, capturing output.
pub fn run_captured(exe: &str, args: &[&str], timeout: std::time::Duration) -> ZResult<Output> {
  use std::process::{Command, Stdio};
  #[cfg(windows)]
  use std::os::windows::process::CommandExt;
  let mut cmd = Command::new(exe);
  cmd
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .stdin(Stdio::null());
  #[cfg(windows)]
  cmd.creation_flags(CREATE_NO_WINDOW);
  let child = cmd
    .spawn()
    .map_err(|e| ZenouError::command(format!("Could not start {exe}."), Some(format!("{e}"))))?;
  wait_with_timeout(child, timeout)
}

/// Wait for a child up to `timeout`, capturing stdout/stderr without deadlock.
///
/// Pipe reads run on helper threads. On timeout the whole process tree is
/// force-killed (taskkill /T), which closes every pipe handle, which lets the
/// readers finish — so the joins below cannot block indefinitely.
fn wait_with_timeout(mut child: std::process::Child, timeout: std::time::Duration) -> ZResult<Output> {
  use std::io::Read;
  use std::os::windows::process::CommandExt;
  use std::time::Duration;
  let start = std::time::Instant::now();
  let mut out_pipe = child.stdout.take();
  let mut err_pipe = child.stderr.take();
  let t_out = std::thread::spawn(move || {
    let mut buf = Vec::new();
    if let Some(p) = out_pipe.as_mut() {
      let _ = p.read_to_end(&mut buf);
    }
    buf
  });
  let t_err = std::thread::spawn(move || {
    let mut buf = Vec::new();
    if let Some(p) = err_pipe.as_mut() {
      let _ = p.read_to_end(&mut buf);
    }
    buf
  });
  let status = loop {
    match child
      .try_wait()
      .map_err(|e| ZenouError::command("The system command failed.", Some(format!("{e}"))))?
    {
      Some(status) => break status,
      None if start.elapsed() > timeout => {
        // Kill the entire tree so no descendant keeps the pipes open.
        let pid = child.id();
        let _ = std::process::Command::new("taskkill.exe")
          .args(["/F", "/T", "/PID", &pid.to_string()])
          .creation_flags(CREATE_NO_WINDOW)
          .output();
        let _ = child.kill();
        let _ = child.wait();
        // All pipe writers are dead now; give the readers a bounded window.
        join_reader(t_out, Duration::from_secs(2));
        join_reader(t_err, Duration::from_secs(2));
        return Err(ZenouError::command(
          "The system command took too long to respond and was stopped.",
          Some(format!("timed out after {timeout:?}")),
        ));
      }
      None => std::thread::sleep(std::time::Duration::from_millis(25)),
    }
  };
  let out = t_out.join().unwrap_or_default();
  let err = t_err.join().unwrap_or_default();
  Ok(Output { exit_code: status.code().unwrap_or(-1), stdout: out, stderr: err })
}

/// Join a reader thread with a deadline. The reader normally finishes as soon
/// as its pipe closes (all writers dead); if something pathological keeps a
/// handle open, we abandon the thread rather than hang the caller.
fn join_reader(t: std::thread::JoinHandle<Vec<u8>>, limit: std::time::Duration) {
  use std::sync::atomic::{AtomicBool, Ordering};
  use std::sync::Arc;
  let done = Arc::new(AtomicBool::new(false));
  let flag = Arc::clone(&done);
  let waiter = std::thread::spawn(move || {
    let _ = t.join();
    flag.store(true, Ordering::Release);
  });
  let deadline = std::time::Instant::now() + limit;
  while !done.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
    std::thread::sleep(std::time::Duration::from_millis(10));
  }
  drop(waiter); // detached (with the reader inside) if the deadline passed
}

#[derive(Debug, Clone)]
pub struct Output {
  pub exit_code: i32,
  pub stdout: Vec<u8>,
  pub stderr: Vec<u8>,
}

impl Output {
  pub fn stdout_str(&self) -> String {
    String::from_utf8_lossy(&self.stdout).to_string()
  }
  pub fn stderr_str(&self) -> String {
    String::from_utf8_lossy(&self.stderr).to_string()
  }
  pub fn combined(&self) -> String {
    format!("exit: {}\nstdout:\n{}\nstderr:\n{}", self.exit_code, self.stdout_str(), self.stderr_str())
  }
}

/// Run a fixed PowerShell snippet (whitelisted, no user input) and return its combined output.
pub fn run_powershell(script: &str, timeout: std::time::Duration) -> ZResult<Output> {
  run_captured(
    "powershell.exe",
    &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script],
    timeout,
  )
}

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wide_strings_are_null_terminated() {
    let w = to_wide("abc");
    assert_eq!(w, vec![97, 98, 99, 0]);
  }
}
