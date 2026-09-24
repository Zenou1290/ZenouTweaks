//! Typed Windows registry access with pre-capture support.

use serde::{Deserialize, Serialize};
use winreg::enums::{
  HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, HKEY_USERS,
  KEY_READ, KEY_SET_VALUE, RegType,
};
use winreg::HKEY;
use winreg::{RegKey, RegValue};

use crate::error::{ZenouError, ZResult};
use crate::types::SavedValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hive {
  #[serde(rename = "HKCU")]
  CurrentUser,
  #[serde(rename = "HKLM")]
  LocalMachine,
  #[serde(rename = "HKU")]
  Users,
  #[serde(rename = "HKCR")]
  ClassesRoot,
  #[serde(rename = "HKCC")]
  CurrentConfig,
}

impl Hive {
  pub fn parse(s: &str) -> ZResult<Hive> {
    match s.to_ascii_uppercase().as_str() {
      "HKCU" | "HKEY_CURRENT_USER" => Ok(Hive::CurrentUser),
      "HKLM" | "HKEY_LOCAL_MACHINE" => Ok(Hive::LocalMachine),
      "HKU" | "HKEY_USERS" => Ok(Hive::Users),
      "HKCR" | "HKEY_CLASSES_ROOT" => Ok(Hive::ClassesRoot),
      "HKCC" | "HKEY_CURRENT_CONFIG" => Ok(Hive::CurrentConfig),
      other => Err(ZenouError::registry(
        "The tweak targets an unsupported registry location and was blocked.",
        Some(format!("unknown hive: {other}")),
      )),
    }
  }

  pub fn as_predef(self) -> HKEY {
    match self {
      Hive::CurrentUser => HKEY_CURRENT_USER,
      Hive::LocalMachine => HKEY_LOCAL_MACHINE,
      Hive::Users => HKEY_USERS,
      Hive::ClassesRoot => HKEY_CLASSES_ROOT,
      Hive::CurrentConfig => HKEY_CURRENT_CONFIG,
    }
  }

  pub fn as_str(self) -> &'static str {
    match self {
      Hive::CurrentUser => "HKCU",
      Hive::LocalMachine => "HKLM",
      Hive::Users => "HKU",
      Hive::ClassesRoot => "HKCR",
      Hive::CurrentConfig => "HKCC",
    }
  }
}

/// Open a key for reading values AND enumerating subkeys.
/// KEY_READ includes KEY_QUERY_VALUE | KEY_ENUMERATE_SUB_KEYS; with only
/// KEY_QUERY_VALUE, RegEnumKeyEx fails forever and winreg's EnumKeys iterator
/// never advances (infinite loop).
pub fn open_key_ro(hive: Hive, path: &str) -> ZResult<RegKey> {
  RegKey::predef(hive.as_predef())
    .open_subkey_with_flags(path, KEY_READ)
    .map_err(Into::into)
}

pub fn open_key_wo(hive: Hive, path: &str) -> ZResult<RegKey> {
  RegKey::predef(hive.as_predef())
    .open_subkey_with_flags(path, KEY_SET_VALUE)
    .map_err(Into::into)
}

/// Open (creating parents as needed) the key that directly contains `value_name`.
pub fn open_parent_for_set(hive: Hive, path: &str) -> ZResult<RegKey> {
  match RegKey::predef(hive.as_predef()).open_subkey_with_flags(path, KEY_SET_VALUE) {
    Ok(key) => Ok(key),
    Err(_) => {
      let predef = RegKey::predef(hive.as_predef());
      predef
        .create_subkey(path)
        .map(|(k, _)| k)
        .map_err(|e| ZenouError::registry("Could not create the registry location.", Some(format!("{e}"))))
    }
  }
}

pub fn read_raw(hive: Hive, path: &str, name: &str) -> ZResult<Option<RegValue>> {
  match open_key_ro(hive, path) {
    Err(e) if e.code == "registry" && e.message.contains("does not exist") => Ok(None),
    Err(e) if is_missing_key(&e) => Ok(None),
    Err(e) => Err(e),
    Ok(key) => Ok(key.get_raw_value(name).ok()),
  }
}

fn is_missing_key(e: &ZenouError) -> bool {
  e.details
    .as_deref()
    .map(|d| d.contains("code: 2") || d.contains("cannot find the file") || d.to_lowercase().contains("not found"))
    .unwrap_or(false)
}

fn encode_value(value: &RegValue) -> (String, Vec<u8>) {
  let t = match value.vtype {
    RegType::REG_SZ => "REG_SZ",
    RegType::REG_EXPAND_SZ => "REG_EXPAND_SZ",
    RegType::REG_BINARY => "REG_BINARY",
    RegType::REG_DWORD => "REG_DWORD",
    RegType::REG_DWORD_BIG_ENDIAN => "REG_DWORD_BIG_ENDIAN",
    RegType::REG_LINK => "REG_LINK",
    RegType::REG_MULTI_SZ => "REG_MULTI_SZ",
    RegType::REG_QWORD => "REG_QWORD",
    _ => "REG_NONE",
  };
  (t.to_string(), value.bytes.clone())
}

fn decode_value(reg_type: &str, data: &[u8]) -> ZResult<RegValue> {
  let vtype = match reg_type {
    "REG_SZ" => RegType::REG_SZ,
    "REG_EXPAND_SZ" => RegType::REG_EXPAND_SZ,
    "REG_BINARY" => RegType::REG_BINARY,
    "REG_DWORD" => RegType::REG_DWORD,
    "REG_DWORD_BIG_ENDIAN" => RegType::REG_DWORD_BIG_ENDIAN,
    "REG_LINK" => RegType::REG_LINK,
    "REG_MULTI_SZ" => RegType::REG_MULTI_SZ,
    "REG_QWORD" => RegType::REG_QWORD,
    _ => RegType::REG_NONE,
  };
  Ok(RegValue { vtype, bytes: data.to_vec() })
}

/// Capture the current value of a registry value; absent values are recorded as ABSENT
/// so restore can re-delete them.
pub fn capture(hive: Hive, path: &str, name: &str) -> ZResult<SavedValue> {
  let raw = read_raw(hive, path, name)?;
  Ok(match raw {
    Some(v) => {
      let (t, data) = encode_value(&v);
      SavedValue { hive: hive.as_str().into(), path: path.into(), name: name.into(), reg_type: t, data }
    }
    None => SavedValue {
      hive: hive.as_str().into(),
      path: path.into(),
      name: name.into(),
      reg_type: "ABSENT".into(),
      data: vec![],
    },
  })
}

/// Restore a previously captured value. ABSENT => delete the value if it now exists.
pub fn restore(saved: &SavedValue) -> ZResult<()> {
  let hive = Hive::parse(&saved.hive)?;
  if saved.reg_type == "ABSENT" {
    delete_value(hive, &saved.path, &saved.name)?;
    return Ok(());
  }
  let value = decode_value(&saved.reg_type, &saved.data)?;
  let key = open_parent_for_set(hive, &saved.path)?;
  key
    .set_raw_value(&saved.name, &value)
    .map_err(|e| ZenouError::registry("Could not restore the original registry value.", Some(format!("{e}"))))
}

pub fn delete_value(hive: Hive, path: &str, name: &str) -> ZResult<()> {
  match open_key_wo(hive, path) {
    Ok(key) => key.delete_value(name).map_err(|e| {
      if e.kind() == std::io::ErrorKind::NotFound {
        // Nothing to delete: the desired end state is already true.
        ZenouError::new("registry-absent", "value already absent", None)
      } else {
        ZenouError::registry("Could not remove the registry value.", Some(format!("{e}")))
      }
    }),
    Err(e) if is_missing_key(&e) => Ok(()), // key gone => value gone
    Err(e) => Err(e),
  }
}

/// Write a REG_DWORD, capturing nothing (engine handles capture).
pub fn set_dword(hive: Hive, path: &str, name: &str, data: u32) -> ZResult<()> {
  let key = open_parent_for_set(hive, path)?;
  key
    .set_raw_value(name, &RegValue { vtype: RegType::REG_DWORD, bytes: data.to_le_bytes().to_vec() })
    .map_err(|e| ZenouError::registry("Could not write the registry value.", Some(format!("{e}"))))
}

pub fn set_sz(hive: Hive, path: &str, name: &str, data: &str) -> ZResult<()> {
  let key = open_parent_for_set(hive, path)?;
  key
    .set_raw_value(name, &RegValue { vtype: RegType::REG_SZ, bytes: to_utf16le(data) })
    .map_err(|e| ZenouError::registry("Could not write the registry value.", Some(format!("{e}"))))
}

pub fn set_binary(hive: Hive, path: &str, name: &str, data: &[u8]) -> ZResult<()> {
  let key = open_parent_for_set(hive, path)?;
  key
    .set_raw_value(name, &RegValue { vtype: RegType::REG_BINARY, bytes: data.to_vec() })
    .map_err(|e| ZenouError::registry("Could not write the registry value.", Some(format!("{e}"))))
}

fn to_utf16le(s: &str) -> Vec<u8> {
  s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
}

/// Human-readable rendering of a captured/preview value for the UI.
pub fn render_saved(saved: &SavedValue) -> String {
  if saved.reg_type == "ABSENT" {
    return "not set".into();
  }
  match saved.reg_type.as_str() {
    "REG_DWORD" if saved.data.len() == 4 => {
      let v = u32::from_le_bytes([saved.data[0], saved.data[1], saved.data[2], saved.data[3]]);
      format!("{v} (0x{v:08x})")
    }
    "REG_QWORD" if saved.data.len() == 8 => {
      let v = u64::from_le_bytes(saved.data[..8].try_into().unwrap());
      format!("{v}")
    }
    "REG_SZ" | "REG_EXPAND_SZ" => decode_utf16le(&saved.data).unwrap_or_else(|| "(binary)".into()),
    "REG_BINARY" => {
      if saved.data.len() <= 16 {
        saved.data.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
      } else {
        format!("{} bytes", saved.data.len())
      }
    }
    _ => format!("{} bytes", saved.data.len()),
  }
}

pub fn decode_utf16le(bytes: &[u8]) -> Option<String> {
  let units: Vec<u16> = bytes
    .chunks_exact(2)
    .map(|c| u16::from_le_bytes([c[0], c[1]]))
    .take_while(|&u| u != 0)
    .collect();
  String::from_utf16(&units).ok()
}

#[cfg(test)]
pub fn parse_hive_shim(s: &str) -> ZResult<Hive> {
  Hive::parse(s)
}
