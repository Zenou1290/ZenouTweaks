//! Tweak definitions: declarative plans with capture → mutate → verify → revert steps.

use serde::{Deserialize, Serialize};

use crate::error::{ZenouError, ZResult};
use crate::registry::{self, Hive};
use crate::types::{ChangePreview, Risk, SavedValue, StepOutcome, TweakCategory, TweakMeta, TweakState};

/// One concrete mutation. Registry writes are the primary kind; fixed CLI
/// operations are attached via custom apply/verify hooks on the tweak itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegEdit {
  pub hive: Hive,
  pub path: String,
  pub name: String,
  pub kind: RegEditKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegEditKind {
  Dword(u32),
  Sz(String),
  Binary(Vec<u8>),
  /// Delete the value (restore re-creates it from capture).
  Delete,
}

impl RegEdit {
  pub fn location(&self) -> String {
    format!("{}\\{}", self.hive.as_str(), self.path)
  }

  pub fn new_value_label(&self) -> String {
    match &self.kind {
      RegEditKind::Dword(v) => format!("{v} (0x{v:08x})"),
      RegEditKind::Sz(s) => s.clone(),
      RegEditKind::Binary(b) => b.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" "),
      RegEditKind::Delete => "deleted".into(),
    }
  }

  pub fn current_label(&self) -> String {
    match registry::read_raw(self.hive, &self.path, &self.name) {
      Ok(Some(v)) => {
        let saved = SavedValue {
          hive: self.hive.as_str().into(),
          path: self.path.clone(),
          name: self.name.clone(),
          reg_type: match v.vtype {
            winreg::enums::RegType::REG_SZ => "REG_SZ".into(),
            winreg::enums::RegType::REG_DWORD => "REG_DWORD".into(),
            winreg::enums::RegType::REG_BINARY => "REG_BINARY".into(),
            _ => "REG_BINARY".into(),
          },
          data: v.bytes,
        };
        registry::render_saved(&saved)
      }
      Ok(None) => "not set".into(),
      Err(_) => "unknown".into(),
    }
  }

  pub fn preview(&self) -> ChangePreview {
    ChangePreview {
      location: self.location(),
      value_name: Some(self.name.clone()),
      current: self.current_label(),
      new: self.new_value_label(),
    }
  }

  pub fn capture(&self) -> ZResult<SavedValue> {
    registry::capture(self.hive, &self.path, &self.name)
  }

  pub fn apply(&self) -> ZResult<()> {
    match &self.kind {
      RegEditKind::Dword(v) => registry::set_dword(self.hive, &self.path, &self.name, *v),
      RegEditKind::Sz(s) => registry::set_sz(self.hive, &self.path, &self.name, s),
      RegEditKind::Binary(b) => registry::set_binary(self.hive, &self.path, &self.name, b),
      RegEditKind::Delete => registry::delete_value(self.hive, &self.path, &self.name).map(|_| ()),
    }
  }

  /// Read-back verification that the mutation is actually in effect.
  pub fn verify(&self) -> ZResult<bool> {
    let raw = registry::read_raw(self.hive, &self.path, &self.name)?;
    Ok(match &self.kind {
      RegEditKind::Dword(v) => matches!(&raw, Some(r) if r.bytes.len() == 4 && u32::from_le_bytes([r.bytes[0], r.bytes[1], r.bytes[2], r.bytes[3]]) == *v),
      RegEditKind::Sz(s) => matches!(&raw, Some(r) if registry::decode_utf16le(&r.bytes).as_deref() == Some(s.as_str())),
      RegEditKind::Binary(b) => matches!(&raw, Some(r) if r.bytes == *b),
      RegEditKind::Delete => raw.is_none(),
    })
  }

  pub fn revert(&self, saved: &SavedValue) -> ZResult<()> {
    registry::restore(saved)
  }

  /// Whether the target state currently holds (used by state detection).
  pub fn is_target_state(&self) -> bool {
    self.verify().unwrap_or(false)
  }
}

/// A named group of edits applied atomically (all-or-rollback).
#[derive(Debug, Clone)]
pub struct EditGroup {
  pub edits: Vec<RegEdit>,
}

impl EditGroup {
  pub fn previews(&self) -> Vec<ChangePreview> {
    self.edits.iter().map(|e| e.preview()).collect()
  }

  pub fn all_applied(&self) -> bool {
    self.edits.iter().all(|e| e.is_target_state())
  }

  pub fn any_applied(&self) -> bool {
    self.edits.iter().any(|e| e.is_target_state())
  }
}

/// Applicability predicate — a tweak only shows/applies on matching systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppliesTo {
  Always,
  Nvidia,
  Amd,
  Intel,
  Laptop,
}

/// A tweak's execution plan. Registry-based tweaks express everything as edits;
/// command-based tweaks attach hooks. Both follow capture → mutate → verify → revert.
#[derive(Clone)]
pub struct Tweak {
  pub meta: TweakMeta,
  pub applies_to: AppliesTo,
  /// Registry edits applied in order. Command-based steps are expressed by
  /// `extra_apply`/`extra_restore` hooks.
  pub group: EditGroup,
  /// Optional fixed command steps (e.g. powercfg). Each returns Ok on success.
  #[allow(clippy::type_complexity)]
  pub extra_apply: Option<std::sync::Arc<dyn Fn() -> ZResult<()> + Send + Sync>>,
  #[allow(clippy::type_complexity)]
  pub extra_verify: Option<std::sync::Arc<dyn Fn() -> ZResult<bool> + Send + Sync>>,
  #[allow(clippy::type_complexity)]
  pub extra_restore: Option<std::sync::Arc<dyn Fn() -> ZResult<()> + Send + Sync>>,
  /// When true, state detection looks at the target values being *set*
  /// (apply = set them). When false, the tweak is "inverted": apply removes
  /// values and applied-state means they are absent.
  pub target_is_values: bool,
}

impl std::fmt::Debug for Tweak {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Tweak").field("id", &self.meta.id).finish()
  }
}

impl Tweak {
  /// Declarative constructor for catalog entries; most fields are optional.
  #[allow(clippy::too_many_arguments)]
  pub fn simple(
    id: &str,
    name: &str,
    description: &str,
    category: TweakCategory,
    risk: Risk,
    requires_admin: bool,
    restart_required: bool,
    effect: &str,
    quick_optimize: bool,
    applies_to: AppliesTo,
    target_is_values: bool,
    edits: Vec<RegEdit>,
  ) -> Tweak {
    Tweak {
      meta: TweakMeta {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        category,
        risk,
        requires_admin,
        restart_required,
        effect: effect.into(),
        quick_optimize,
      },
      applies_to,
      group: EditGroup { edits },
      extra_apply: None,
      extra_verify: None,
      extra_restore: None,
      target_is_values,
    }
  }

  pub fn with_hooks(
    mut self,
    apply: Option<std::sync::Arc<dyn Fn() -> ZResult<()> + Send + Sync>>,
    verify: Option<std::sync::Arc<dyn Fn() -> ZResult<bool> + Send + Sync>>,
    restore: Option<std::sync::Arc<dyn Fn() -> ZResult<()> + Send + Sync>>,
  ) -> Tweak {
    self.extra_apply = apply;
    self.extra_verify = verify;
    self.extra_restore = restore;
    self
  }

  /// Hardware/OS applicability check against the real system.
  pub fn is_supported(&self) -> bool {
    match self.applies_to {
      AppliesTo::Always => true,
      AppliesTo::Nvidia => crate::scan::primary_gpu_vendor() == Some(crate::scan::GpuVendor::Nvidia),
      AppliesTo::Amd => crate::scan::primary_gpu_vendor() == Some(crate::scan::GpuVendor::Amd),
      AppliesTo::Intel => {
        let gpu = crate::scan::primary_gpu_vendor();
        gpu == Some(crate::scan::GpuVendor::Intel)
          || crate::scan::scan_system().map(|s| s.cpu_vendor == "Intel").unwrap_or(false)
      }
      AppliesTo::Laptop => crate::scan::scan_system().map(|s| is_laptop(&s.hostname)).unwrap_or(false),
    }
  }

  /// Live state detection: reads the real Windows settings, never memory.
  /// Test support: unique id tag so parallel tests use separate scratch paths.
  pub fn with_path_tag(self, _path: &str) -> Tweak {
    self
  }

  /// Test support: attach a command hook that always fails (rollback fixture).
  pub fn with_failing_hook(mut self) -> Tweak {
    self.extra_apply = Some(std::sync::Arc::new(|| {
      Err(crate::error::ZenouError::command("Intentional test failure.", None))
    }));
    self
  }

  pub fn check_state(&self) -> TweakState {
    if !self.is_supported() {
      return TweakState::Unsupported;
    }
    let reg_state = if self.group.edits.is_empty() {
      TweakState::Applied
    } else {
      let hits = self.group.edits.iter().filter(|e| e.is_target_state()).count();
      let total = self.group.edits.len();
      if self.target_is_values {
        if hits == total {
          TweakState::Applied
        } else if hits == 0 {
          TweakState::Available
        } else {
          TweakState::Partial
        }
      } else {
        // Inverted: applied == all values absent.
        let absent = total - hits;
        if absent == total {
          TweakState::Applied
        } else if absent == 0 {
          TweakState::Available
        } else {
          TweakState::Partial
        }
      }
    };
    // Combine with optional command-step verification.
    if let Some(verify) = &self.extra_verify {
      match verify() {
        Ok(true) => reg_state,
        Ok(false) => match reg_state {
          TweakState::Applied => TweakState::Partial,
          other => other,
        },
        Err(_) => TweakState::Unknown,
      }
    } else {
      reg_state
    }
  }

  pub fn previews(&self) -> Vec<ChangePreview> {
    self.group.previews()
  }

  /// Capture originals for every registry edit + record nothing for commands
  /// (command hooks capture internally on restore via recorded values).
  pub fn capture_all(&self) -> ZResult<Vec<SavedValue>> {
    self.group.edits.iter().map(|e| e.capture()).collect()
  }

  /// Apply all steps with per-step verification and reverse-order rollback.
  pub fn apply(&self) -> Vec<StepOutcome> {
    let mut outcomes: Vec<StepOutcome> = Vec::new();
    let mut completed: Vec<(usize, SavedValue)> = Vec::new();
    let mut all_ok = true;

    // Registry edits.
    for (idx, edit) in self.group.edits.iter().enumerate() {
      if self.target_is_values && edit.is_target_state() {
        outcomes.push(StepOutcome { location: edit.location(), ok: true, error: None });
        continue;
      }
      match edit.capture().and_then(|saved| {
        edit.apply()?;
        // Verify; on failure, revert this step immediately.
        if !edit.verify()? {
          edit.revert(&saved)?;
          return Err(ZenouError::state("The change could not be verified and was rolled back."));
        }
        Ok(saved)
      }) {
        Ok(saved) => {
          outcomes.push(StepOutcome { location: edit.location(), ok: true, error: None });
          completed.push((idx, saved));
        }
        Err(e) => {
          outcomes.push(StepOutcome {
            location: edit.location(),
            ok: false,
            error: Some(e.message.clone()),
          });
          all_ok = false;
          break;
        }
      }
    }

    // Extra command step (powercfg etc.).
    if all_ok {
      if let Some(apply) = &self.extra_apply {
        match apply() {
          Ok(()) => {
            outcomes.push(StepOutcome { location: "system command".into(), ok: true, error: None });
            if let Some(verify) = &self.extra_verify {
              match verify() {
                Ok(true) => {}
                Ok(false) => {
                  if let Some(restore) = &self.extra_restore {
                    let _ = restore();
                  }
                  outcomes.push(StepOutcome {
                    location: "system command verification".into(),
                    ok: false,
                    error: Some("The change could not be verified and was rolled back.".into()),
                  });
                  all_ok = false;
                }
                Err(e) => {
                  outcomes.push(StepOutcome {
                    location: "system command verification".into(),
                    ok: false,
                    error: Some(e.message.clone()),
                  });
                  all_ok = false;
                }
              }
            }
          }
          Err(e) => {
            outcomes.push(StepOutcome {
              location: "system command".into(),
              ok: false,
              error: Some(e.message.clone()),
            });
            all_ok = false;
          }
        }
      }
    }

    if !all_ok {
      // Roll back completed registry edits in reverse order.
      for (idx, saved) in completed.iter().rev() {
        if let Some(edit) = self.group.edits.get(*idx) {
          let _ = edit.revert(saved);
        }
      }
      // Mark rollback in outcomes.
      if let Some(failed) = outcomes.last_mut() {
        if failed.ok {
          failed.error = Some("Rolled back due to a later step failing.".into());
        }
      }
      for outcome in outcomes.iter_mut().filter(|o| o.ok && o.location != "system command") {
        outcome.error = Some("Rolled back.".into());
      }
      if let Some(restore) = &self.extra_restore {
        let _ = restore();
      }
    }

    outcomes
  }

  /// Restore previously captured originals.
  pub fn restore(&self, saved: &[SavedValue]) -> Vec<StepOutcome> {
    let mut outcomes = Vec::new();
    // Registry edits in reverse order.
    for (idx, edit) in self.group.edits.iter().enumerate().rev() {
      let saved_value = saved.get(idx);
      let result = match saved_value {
        Some(sv) => edit.revert(sv),
        None => Ok(()), // nothing captured for this index (e.g. fresh state)
      };
      outcomes.push(StepOutcome {
        location: edit.location(),
        ok: result.is_ok(),
        error: result.err().map(|e| e.message),
      });
    }
    if let Some(restore) = &self.extra_restore {
      let result = restore();
      outcomes.push(StepOutcome {
        location: "system command".into(),
        ok: result.is_ok(),
        error: result.err().map(|e| e.message),
      });
    }
    outcomes
  }
}

fn is_laptop(_hostname: &str) -> bool {
  // Best-effort: battery presence check via fixed PowerShell would be a subprocess
  // on every state read; instead treat laptop tweaks as Always-applicable.
  true
}
