//! Shared data types for Zenou Tweaks.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
  Low,
  Medium,
  High,
}

impl Risk {
  pub fn rank(&self) -> u8 {
    match self {
      Risk::Low => 0,
      Risk::Medium => 1,
      Risk::High => 2,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TweakCategory {
  Performance,
  Gaming,
  Gpu,
  Network,
  Cleanup,
  Windows,
}

impl TweakCategory {
  pub const ALL: [TweakCategory; 6] = [
    TweakCategory::Performance,
    TweakCategory::Gaming,
    TweakCategory::Gpu,
    TweakCategory::Network,
    TweakCategory::Cleanup,
    TweakCategory::Windows,
  ];

  pub fn label(&self) -> &'static str {
    match self {
      TweakCategory::Performance => "Performance",
      TweakCategory::Gaming => "Gaming",
      TweakCategory::Gpu => "GPU",
      TweakCategory::Network => "Network",
      TweakCategory::Cleanup => "Cleanup",
      TweakCategory::Windows => "Windows",
    }
  }
}

/// Live, machine-detected tweak state. Never guessed from history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TweakState {
  /// Not applied; the tweak can be applied.
  Available,
  /// The tweak's target state is currently in effect.
  Applied,
  /// Part of the tweak's target state is in effect (multi-part tweak).
  Partial,
  /// The tweak does not apply to this system (hardware/OS mismatch).
  Unsupported,
  /// State could not be determined.
  Unknown,
}

/// Transient operation state used by the UI while an action runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationKind {
  Applying,
  Restoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TweakMeta {
  pub id: String,
  pub name: String,
  pub description: String,
  pub category: TweakCategory,
  pub risk: Risk,
  pub requires_admin: bool,
  pub restart_required: bool,
  /// Human explanation of expected effect (what changes — never performance promises).
  pub effect: String,
  /// Low-risk-and-safe tweaks are eligible for Quick Optimize.
  pub quick_optimize: bool,
}

/// One concrete change a tweak makes, used for previews and details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePreview {
  /// System location, e.g. a registry path or a command target.
  pub location: String,
  pub value_name: Option<String>,
  /// What is there now ("not set" when absent).
  pub current: String,
  /// What will be written.
  pub new: String,
}

/// Recorded original value of a registry value (for restore).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedValue {
  pub hive: String,
  pub path: String,
  pub name: String,
  /// Registry type name: REG_SZ, REG_DWORD, REG_BINARY, REG_QWORD... or "ABSENT".
  pub reg_type: String,
  /// Raw bytes payload ( DWORD little-endian 4 bytes, QWORD 8, SZ/EXPAND_SZ utf16le ).
  pub data: Vec<u8>,
}

/// A single step inside a tweak's execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepOutcome {
  pub location: String,
  pub ok: bool,
  pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TweakResult {
  pub tweak_id: String,
  pub success: bool,
  pub skipped: bool,
  /// Failure reason (human) when success == false.
  pub error: Option<String>,
  /// Raw technical detail for the "View details" expander.
  pub details: Option<String>,
  pub restart_required: bool,
  pub steps: Vec<StepOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupResult {
  pub label: String,
  pub results: Vec<TweakResult>,
  pub restore_point_created: bool,
}

impl GroupResult {
  pub fn success_count(&self) -> usize {
    self.results.iter().filter(|r| r.success && !r.skipped).count()
  }
  pub fn failed_count(&self) -> usize {
    self.results.iter().filter(|r| !r.success && !r.skipped).count()
  }
  pub fn skipped_count(&self) -> usize {
    self.results.iter().filter(|r| r.skipped).count()
  }
}
