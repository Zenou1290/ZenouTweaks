//! Startup performance instrumentation: times the real cold-start path
//! (scan + per-tweak state checks) and asserts budget compliance.
//!
//! These tests run the actual catalog/scan code (integration over real registry
//! and subprocess hooks), so they only run when ZENOU_STARTUP_BENCH=1 is set.

use std::time::{Duration, Instant};

fn bench_enabled() -> bool {
  std::env::var("ZENOU_STARTUP_BENCH").is_ok()
}

fn wall<F: FnOnce() -> T, T>(f: F) -> (T, Duration) {
  let start = Instant::now();
  (f(), start.elapsed())
}

#[test]
fn cold_start_within_budget() {
  if !bench_enabled() {
    eprintln!("skipped: set ZENOU_STARTUP_BENCH=1 to run (hits real registry/subprocesses)");
    return;
  }

  // ---- scan (first call does real work) ----
  let (_scan, scan_time) = wall(|| crate::scan::scan_system().expect("scan should succeed"));
  eprintln!("scan_system: {:?}", scan_time);
  // Even the slow path (PowerShell fallback) is capped by subprocess timeouts;
  // the FFI path should be far below this. Generous ceiling catches regressions.
  assert!(scan_time < Duration::from_secs(10), "scan took {:?}", scan_time);

  // ---- second scan must be served from cache ----
  let (_s2, cached_time) = wall(|| crate::scan::scan_system().expect("cached scan"));
  eprintln!("scan_system (cached): {:?}", cached_time);
  assert!(cached_time < Duration::from_millis(50), "cached scan took {:?}", cached_time);

  // ---- full catalog state checks (the list_tweaks body), per-tweak breakdown ----
  let catalog = crate::catalog::all_tweaks();
  let n = catalog.len();
  let check_time_start = Instant::now();
  let mut slow = Vec::new();
  let mut total = Duration::ZERO;
  for t in &catalog {
    let (state, dt) = wall(|| {
      std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.check_state()))
        .unwrap_or(crate::types::TweakState::Unknown)
    });
    total += dt;
    if dt > Duration::from_millis(200) {
      slow.push(format!("  {:?} {} -> {:?}", dt, t.meta.id, state));
    }
  }
  let check_time = check_time_start.elapsed();
  for line in &slow {
    eprintln!("{}", line);
  }
  eprintln!("check_state x{} (serial): sum={:?} wall={:?}", n, total, check_time);
  // Documented budget: a few seconds cold. Generous ceiling catches regressions.
  assert!(check_time < Duration::from_secs(60), "state checks took {:?}", check_time);

  // ---- second pass (warm) should be comparable — no unbounded growth ----
  let catalog2 = crate::catalog::all_tweaks();
  let (_states2, warm_time) = wall(|| {
    catalog2
      .iter()
      .map(|t| std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.check_state())).unwrap_or(crate::types::TweakState::Unknown))
      .collect::<Vec<_>>()
  });
  eprintln!("check_state x{} (warm): {:?}", n, warm_time);
  assert!(warm_time < Duration::from_secs(60), "warm state checks took {:?}", warm_time);
}

/// Concurrent scan stampede: many threads racing scan_system (as happens when
/// the frontend calls scan while tweak builders resolve hardware conditions)
/// must all get the same snapshot, and no caller may deadlock.
#[test]
fn concurrent_scan_is_consistent() {
  if !bench_enabled() {
    eprintln!("skipped: set ZENOU_STARTUP_BENCH=1 to run (hits real registry)");
    return;
  }
  // Warm the cache once so the race is over the fast path.
  let baseline = crate::scan::scan_system().expect("warm scan");
  let mut handles = Vec::new();
  for i in 0..16 {
    handles.push(std::thread::spawn(move || {
      let scan = crate::scan::scan_system().expect("concurrent scan");
      (i, scan.windows_version, scan.cpu_name, scan.ram_gb)
    }));
  }
  for h in handles {
    let (i, version, cpu, ram) = h.join().expect("worker panicked");
    assert_eq!(version, baseline.windows_version, "thread {i} saw different OS");
    assert_eq!(cpu, baseline.cpu_name, "thread {i} saw different CPU");
    assert_eq!(ram, baseline.ram_gb, "thread {i} saw different RAM");
  }
}

/// Cross-validates the PowerShell task snapshot against schtasks ground truth:
/// for every telemetry task that exists, the snapshot-based classification
/// (disabled vs enabled) must match what schtasks reports.
#[test]
fn task_snapshot_matches_schtasks() {
  if !bench_enabled() {
    eprintln!("skipped: set ZENOU_STARTUP_BENCH=1 to run (spawns schtasks)");
    return;
  }
  let mut compared = 0;
  let mut unresolved = 0;
  let mut mismatches = Vec::new();
  for task in crate::catalog_ext::TELEMETRY_TASKS {
    let schtasks_state = crate::ops::task_state(task).ok().flatten(); // ground truth
    let Some(enabled_per_schtasks) = schtasks_state else {
      continue; // task not on this machine
    };
    compared += 1;
    // Snapshot probe: verify() == Ok(true) means "disabled or absent".
    let probe = vault_probe(task);
    let Some(probe_all_disabled) = probe else {
      unresolved += 1;
      continue;
    };
    let expected = !enabled_per_schtasks; // disabled per schtasks?
    // A single-task probe returning true means disabled-or-absent; the task
    // exists per schtasks, so true must mean disabled here.
    if probe_all_disabled != expected {
      mismatches.push(format!(
        "{}: schtasks_disabled={} snapshot_disabled={}",
        task, expected, probe_all_disabled
      ));
    }
  }
  eprintln!(
    "snapshot cross-check: {} compared, {} unresolved, {} mismatches",
    compared,
    unresolved,
    mismatches.len()
  );
  for m in &mismatches {
    eprintln!("MISMATCH {}", m);
  }
  assert!(mismatches.is_empty(), "snapshot disagrees with schtasks");
  assert_eq!(unresolved, 0, "snapshot could not classify existing tasks");
  assert!(compared > 0, "no tasks compared — test environment has none");
}

fn vault_probe(task: &str) -> Option<bool> {
  // Build a one-task hook set and read its verify outcome (no mutation).
  let (_apply, verify, _restore) =
    crate::catalog_ext::task_disable_hooks(vec![task.to_string()]);
  verify().ok()
}
