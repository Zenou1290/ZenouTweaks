//! Application error type producing user-friendly messages.

use serde::Serialize;

pub type ZResult<T> = Result<T, ZenouError>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZenouError {
  pub code: String,
  pub message: String,
  pub details: Option<String>,
}

impl std::fmt::Display for ZenouError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl std::error::Error for ZenouError {}

impl ZenouError {
  pub fn new(code: &str, message: impl Into<String>, details: Option<String>) -> Self {
    Self { code: code.to_string(), message: message.into(), details }
  }

  pub fn io(message: impl Into<String>) -> Self {
    Self::new("io", message, None)
  }

  pub fn registry(message: impl Into<String>, details: Option<String>) -> Self {
    Self::new("registry", message, details)
  }

  pub fn command(message: impl Into<String>, details: Option<String>) -> Self {
    Self::new("command", message, details)
  }

  pub fn permission(message: impl Into<String>, details: Option<String>) -> Self {
    Self::new("permission", message, details)
  }

  pub fn unsupported(message: impl Into<String>) -> Self {
    Self::new("unsupported", message, None)
  }

  pub fn cancelled(message: impl Into<String>) -> Self {
    Self::new("cancelled", message, None)
  }

  pub fn state(message: impl Into<String>) -> Self {
    Self::new("state", message, None)
  }
}

impl From<std::io::Error> for ZenouError {
  fn from(e: std::io::Error) -> Self {
    let code = e.kind();
    let friendly = match code {
      std::io::ErrorKind::PermissionDenied => {
        "Administrator permission is required for this action. Approve the permission prompt (or run Zenou Tweaks as administrator) and try again."
      }
      std::io::ErrorKind::NotFound => "A required file or folder could not be found.",
      _ => "An unexpected error occurred while accessing the system.",
    };
    Self::new(
      "io",
      friendly,
      Some(format!("{code:?}: {e}")),
    )
  }
}



impl From<serde_json::Error> for ZenouError {
  fn from(e: serde_json::Error) -> Self {
    ZenouError::io("Could not read or write application data.").set_details(format!("{e}"))
  }
}
