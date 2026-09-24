//! Unit tests: registry round-trips, state detection, rollback on failure.

use crate::engine::{RegEdit, RegEditKind, Tweak};
use crate::error::ZResult;
use crate::registry::{self, Hive};
use crate::types::{Risk, TweakCategory};

const TEST_ROOT: &str = "SOFTWARE\\ZenouTweaksTest";

/// Unique scratch sub-path per test (tests run in parallel).
fn test_path(name: &str) -> String {
  format!("{TEST_ROOT}\\{name}")
}

fn cleanup_test_key(name: &str) {
  let predef = winreg::RegKey::predef(Hive::CurrentUser.as_predef());
  let _ = predef.delete_subkey_all(&test_path(name));
}

fn test_edit(path: &str, value: u32) -> RegEdit {
  RegEdit {
    hive: Hive::CurrentUser,
    path: path.into(),
    name: "TestValue".into(),
    kind: RegEditKind::Dword(value),
  }
}

fn make_tweak(path: &str, edits: Vec<RegEdit>) -> Tweak {
  Tweak::simple(
    "test.tweak",
    "Test tweak",
    "Internal test tweak",
    TweakCategory::Performance,
    Risk::Low,
    false,
    false,
    "Writes a scratch registry value.",
    false,
    crate::engine::AppliesTo::Always,
    true,
    edits,
  )
  .with_path_tag(path)
}

#[test]
fn apply_verify_restore_round_trip() -> ZResult<()> {
  let name = "roundtrip";
  let path = test_path(name);
  cleanup_test_key(name);
  // Ensure some original value exists.
  registry::set_dword(Hive::CurrentUser, &path, "TestValue", 111)?;

  let t = make_tweak(&path, vec![test_edit(&path, 222)]);
  // State should be Available (target 222 not present).
  assert_eq!(t.check_state(), crate::types::TweakState::Available);

  // Capture originals then apply.
  let saved = t.capture_all()?;
  let outcomes = t.apply();
  assert!(outcomes.iter().all(|o| o.ok), "apply outcomes should succeed");
  assert_eq!(t.check_state(), crate::types::TweakState::Applied);

  // Restore should return the original 111.
  let restore_outcomes = t.restore(&saved);
  assert!(restore_outcomes.iter().all(|o| o.ok));
  let raw = registry::read_raw(Hive::CurrentUser, &path, "TestValue")?;
  let val = raw.map(|v| u32::from_le_bytes([v.bytes[0], v.bytes[1], v.bytes[2], v.bytes[3]]));
  assert_eq!(val, Some(111));
  cleanup_test_key(name);
  Ok(())
}

#[test]
fn capture_absent_and_restore_deletes() -> ZResult<()> {
  let name = "absent";
  let path = test_path(name);
  cleanup_test_key(name);
  // No original value — capture must record ABSENT.
  let t = make_tweak(&path, vec![test_edit(&path, 5)]);
  let saved = t.capture_all()?;
  assert_eq!(saved[0].reg_type, "ABSENT");

  // Apply (creates the value), then restore must delete it again.
  let outcomes = t.apply();
  assert!(outcomes.iter().all(|o| o.ok));
  assert!(registry::read_raw(Hive::CurrentUser, &path, "TestValue")?.is_some());

  let restore_outcomes = t.restore(&saved);
  assert!(restore_outcomes.iter().all(|o| o.ok), "restore of ABSENT should delete");
  assert!(registry::read_raw(Hive::CurrentUser, &path, "TestValue")?.is_none());
  cleanup_test_key(name);
  Ok(())
}

#[test]
fn failed_step_rolls_back_completed_steps() -> ZResult<()> {
  let name = "rollback";
  let path = test_path(name);
  cleanup_test_key(name);
  registry::set_dword(Hive::CurrentUser, &path, "GoodValue", 1)?;

  // First edit succeeds, then a command step fails -> both must roll back.
  let edits = vec![RegEdit {
    hive: Hive::CurrentUser,
    path: path.clone(),
    name: "GoodValue".into(),
    kind: RegEditKind::Dword(9),
  }];
  let t = make_tweak(&path, edits).with_failing_hook();
  let outcomes = t.apply();
  assert!(!outcomes.iter().all(|o| o.ok), "command step should fail");
  // The registry edit must have been rolled back to its original value.
  let raw = registry::read_raw(Hive::CurrentUser, &path, "GoodValue")?;
  let val = raw.map(|v| u32::from_le_bytes([v.bytes[0], v.bytes[1], v.bytes[2], v.bytes[3]]));
  assert_eq!(val, Some(1), "rollback should restore the original value");
  cleanup_test_key(name);
  Ok(())
}

#[test]
fn hive_parsing() -> ZResult<()> {
  assert!(matches!(registry::Hive::parse("HKCU")?, Hive::CurrentUser));
  assert!(matches!(registry::Hive::parse("hkey_local_machine")?, Hive::LocalMachine));
  assert!(registry::Hive::parse("bogus").is_err());
  Ok(())
}

#[test]
fn render_saved_dword() {
  let saved = crate::types::SavedValue {
    hive: "HKCU".into(),
    path: "X".into(),
    name: "Y".into(),
    reg_type: "REG_DWORD".into(),
    data: 255u32.to_le_bytes().to_vec(),
  };
  assert_eq!(registry::render_saved(&saved), "255 (0x000000ff)");
  let absent = crate::types::SavedValue {
    hive: "HKCU".into(),
    path: "X".into(),
    name: "Y".into(),
    reg_type: "ABSENT".into(),
    data: vec![],
  };
  assert_eq!(registry::render_saved(&absent), "not set");
}

#[test]
fn catalog_builds_and_has_unique_ids() {
  let catalog = crate::build_catalog();
  assert!(catalog.len() >= 40, "catalog should be substantial");
  let mut ids: Vec<&str> = catalog.iter().map(|t| t.meta.id.as_str()).collect();
  ids.sort_unstable();
  let before = ids.len();
  ids.dedup();
  assert_eq!(before, ids.len(), "tweak ids must be unique");
  // Every Always-applicable tweak must have steps: registry edits, task/service hooks,
  // or command hooks. Hardware-conditional tweaks (NVIDIA/AMD/Intel) may legitimately
  // resolve to zero edits when that hardware is absent.
  for t in &catalog {
    if t.applies_to != crate::engine::AppliesTo::Always {
      continue;
    }
    let has_steps = !t.group.edits.is_empty()
      || t.extra_apply.is_some()
      || t.extra_restore.is_some();
    assert!(has_steps, "tweak {} has no steps", t.meta.id);
  }
}
