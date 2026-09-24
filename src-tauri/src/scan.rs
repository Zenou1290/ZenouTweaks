//! Zenou Scan: real system and hardware information — one-shot, cached, no polling.
//!
//! Everything is resolved via direct Win32 calls and the registry (instant). PowerShell is
//! only ever used as a last-resort fallback, and the full scan result is cached for the
//! process lifetime so repeated calls (frontend refresh, hardware checks, tweak builders)
//! are free.

use serde::Serialize;
use crate::error::ZResult;
use crate::registry::{self, Hive};
use crate::winapi;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
  pub model: String,
  pub size_gb: Option<u64>,
  pub drive_letter: Option<String>,
  pub total_gb: Option<u64>,
  pub free_gb: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
  pub name: String,
  pub vendor: GpuVendor,
  pub driver_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuVendor {
  Nvidia,
  Amd,
  Intel,
  Other,
}

impl GpuVendor {
  pub fn detect(name: &str) -> GpuVendor {
    let n = name.to_lowercase();
    if n.contains("nvidia") || n.contains("geforce") || n.contains("quadro") || n.contains("rtx") || n.contains("gtx") {
      GpuVendor::Nvidia
    } else if n.contains("amd") || n.contains("radeon") || n.contains("rx ") || n.contains("vega") {
      GpuVendor::Amd
    } else if n.contains("intel") || n.contains("iris") || n.contains("uhd") || n.contains("hd graphics") {
      GpuVendor::Intel
    } else {
      GpuVendor::Other
    }
  }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetAdapterInfo {
  pub name: String,
  pub description: String,
  pub interface_guid: String,
  pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemScan {
  pub windows_edition: String,
  pub windows_version: String,
  pub windows_build: String,
  pub architecture: String,
  pub cpu_name: String,
  pub cpu_cores: u32,
  pub cpu_threads: u32,
  pub cpu_vendor: String,
  pub ram_gb: Option<f64>,
  pub gpus: Vec<GpuInfo>,
  pub storage: Vec<DiskInfo>,
  pub adapters: Vec<NetAdapterInfo>,
  pub uptime: String,
  pub is_elevated: bool,
  pub hostname: String,
  pub username: String,
}

fn reg_sz(hive: Hive, path: &str, name: &str) -> Option<String> {
  registry::read_raw(hive, path, name)
    .ok()
    .flatten()
    .and_then(|v| registry::decode_utf16le(&v.bytes))
}

fn reg_dword(hive: Hive, path: &str, name: &str) -> Option<u32> {
  registry::read_raw(hive, path, name).ok().flatten().and_then(|v| {
    if v.bytes.len() == 4 {
      Some(u32::from_le_bytes([v.bytes[0], v.bytes[1], v.bytes[2], v.bytes[3]]))
    } else {
      None
    }
  })
}

static SCAN_CACHE: std::sync::RwLock<Option<SystemScan>> = std::sync::RwLock::new(None);

/// Serializes cold scans so concurrent callers (frontend + tweak builders)
/// share one scan instead of stampeding hardware queries. The first caller
/// does the work; the rest then hit the warm cache.
static SCAN_INFLIGHT: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Cached scan. The first call does the real work; every later call (frontend refresh,
/// hardware-conditional tweak checks, tweak builders) returns the cached snapshot.
pub fn scan_system() -> ZResult<SystemScan> {
  if let Some(scan) = SCAN_CACHE
    .read()
    .ok()
    .and_then(|guard| guard.clone())
  {
    return Ok(scan);
  }
  // A poisoned lock here only means another scan panicked mid-mutation; the
  // scan itself is read-only hardware inspection, so proceed regardless.
  let _gate = SCAN_INFLIGHT.lock().unwrap_or_else(|e| e.into_inner());
  // Re-check: another thread may have populated the cache while we waited.
  if let Some(scan) = SCAN_CACHE
    .read()
    .ok()
    .and_then(|guard| guard.clone())
  {
    return Ok(scan);
  }
  let scan = scan_system_uncached()?;
  if let Ok(mut guard) = SCAN_CACHE.write() {
    *guard = Some(scan.clone());
  }
  Ok(scan)
}

/// Drop the cache so the next scan_system() re-reads hardware (manual "Rescan").
pub fn invalidate_scan_cache() {
  if let Ok(mut guard) = SCAN_CACHE.write() {
    *guard = None;
  }
}

fn scan_system_uncached() -> ZResult<SystemScan> {
  // ---- Windows identity (real registry values) ----
  let cv = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
  let edition = reg_sz(Hive::LocalMachine, cv, "ProductName").unwrap_or_else(|| "Windows".into());
  let display_version = reg_sz(Hive::LocalMachine, cv, "DisplayVersion").unwrap_or_default();
  let release_id = reg_sz(Hive::LocalMachine, cv, "ReleaseId").unwrap_or_default();
  let version = if !display_version.is_empty() { display_version } else { release_id };
  let build_ubr = reg_dword(Hive::LocalMachine, cv, "UBR").unwrap_or(0);
  let build = reg_sz(Hive::LocalMachine, cv, "CurrentBuildNumber").unwrap_or_default();
  let windows_build = if build_ubr > 0 { format!("{build}.{build_ubr}") } else { build };

  let env_arch = if cfg!(target_arch = "aarch64") {
    "ARM64"
  } else {
    "x64"
  };
  let architecture = format!("{}-bit", env_arch.trim_end_matches("-bit"));

  // ---- CPU ----
  let identifier = reg_sz(Hive::LocalMachine, r"HARDWARE\DESCRIPTION\System\CentralProcessor\0", "Identifier").unwrap_or_default();
  let vendor_string = reg_sz(Hive::LocalMachine, r"HARDWARE\DESCRIPTION\System\CentralProcessor\0", "VendorIdentifier").unwrap_or_default();
  let cpu_name = reg_sz(Hive::LocalMachine, r"HARDWARE\DESCRIPTION\System\CentralProcessor\0", "ProcessorNameString").unwrap_or_default();
  let cpu_vendor = if vendor_string.contains("GenuineIntel") {
    "Intel"
  } else if vendor_string.contains("AuthenticAMD") {
    "AMD"
  } else {
    "Unknown"
  }
  .to_string();

  let (cpu_cores, cpu_threads) = read_cpu_counts(&identifier);

  // ---- RAM: instant via GlobalMemoryStatusEx (raw FFI; PowerShell only as fallback) ----
  let ram_gb = read_total_ram_gb();

  // ---- GPUs ----
  let gpus = read_gpus();

  // ---- Storage: logical volumes (instant FFI) + physical disk models (fast fallback) ----
  let storage = read_storage();

  // ---- Network adapters (from the same registry keys the .bat enumerates) ----
  let adapters = read_adapters();

  // ---- Uptime via GetTickCount64 ----
  let uptime = format_uptime(unsafe_uptime_seconds());

  let hostname = hostname();
  let username = username();

  Ok(SystemScan {
    windows_edition: edition,
    windows_version: version,
    windows_build,
    architecture,
    cpu_name,
    cpu_cores,
    cpu_threads,
    cpu_vendor,
    ram_gb,
    gpus,
    storage,
    adapters,
    uptime,
    is_elevated: winapi::is_elevated(),
    hostname,
    username,
  })
}

fn read_cpu_counts(identifier: &str) -> (u32, u32) {
  // Threads: count of CentralProcessor\N keys is logical processors.
  // Bound the count (512 is far beyond any current Windows machine) instead of
  // breaking at the first gap — key indices need not be contiguous.
  let mut threads = 0u32;
  for i in 0..512u32 {
    let path = format!(r"HARDWARE\DESCRIPTION\System\CentralProcessor\{i}");
    if open_key_exists(Hive::LocalMachine, &path) {
      threads += 1;
    }
  }
  if threads == 0 {
    threads = 1;
  }
  let _ = identifier;
  let cores = std::env::var("NUMBER_OF_PROCESSORS")
    .ok()
    .and_then(|v| v.parse::<u32>().ok())
    .unwrap_or(threads)
    .max(1);
  let cores = cores.min(threads);
  (cores, threads)
}

/// True when a registry key exists (cheap KEY_READ open, no enumeration).
fn open_key_exists(hive: Hive, path: &str) -> bool {
  registry::open_key_ro(hive, path).is_ok()
}

/// Total physical RAM in GB via GlobalMemoryStatusEx (instant, no subprocess).
/// Total physical RAM in GB — instant (FFI + cache), no full scan required.
pub fn total_ram_gb() -> Option<f64> {
  read_total_ram_gb()
}

fn read_total_ram_gb() -> Option<f64> {
  #[link(name = "kernel32")]
  extern "system" {
    fn GlobalMemoryStatusEx(lpBuffer: *mut MemoryStatusEx) -> i32;
  }
  #[repr(C)]
  struct MemoryStatusEx {
    dw_length: u32,
    dw_memory_load: u32,
    ull_total_phys: u64,
    ull_avail_phys: u64,
    ull_total_page_file: u64,
    ull_avail_page_file: u64,
    ull_total_virtual: u64,
    ull_avail_virtual: u64,
    ull_avail_extended_virtual: u64,
  }
  let mut ms = MemoryStatusEx {
    dw_length: std::mem::size_of::<MemoryStatusEx>() as u32,
    dw_memory_load: 0,
    ull_total_phys: 0,
    ull_avail_phys: 0,
    ull_total_page_file: 0,
    ull_avail_page_file: 0,
    ull_total_virtual: 0,
    ull_avail_virtual: 0,
    ull_avail_extended_virtual: 0,
  };
  let ok = unsafe { GlobalMemoryStatusEx(&mut ms) };
  if ok == 0 || ms.ull_total_phys == 0 {
    // Last-resort fallback (cached result, one short subprocess).
    return winapi::run_powershell(
      "[math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory/1GB,1)",
      std::time::Duration::from_secs(20),
    )
    .ok()
    .and_then(|out| out.stdout_str().trim().parse::<f64>().ok());
  }
  Some((ms.ull_total_phys as f64 / 1024.0 / 1024.0 / 1024.0 * 10.0).round() / 10.0)
}

fn read_gpus() -> Vec<GpuInfo> {
  let mut gpus = Vec::new();
  let base = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";
  for idx in 0..8 {
    let path = format!("{base}\\{idx:04}");
    let desc = match reg_sz(Hive::LocalMachine, &path, "DriverDesc") {
      Some(d) => d,
      None => continue,
    };
    if desc.is_empty() || desc.to_lowercase().contains("virtual") {
      continue;
    }
    let driver = reg_sz(Hive::LocalMachine, &path, "DriverVersion");
    let vendor = GpuVendor::detect(&desc);
    gpus.push(GpuInfo { name: desc, vendor, driver_version: driver });
  }
  gpus
}

fn read_storage() -> Vec<DiskInfo> {
  let mut disks = Vec::new();
  // Logical volumes with real free/total space (GetDiskFreeSpaceExW — instant).
  // Drive letters come from GetLogicalDrives (never touched on the filesystem),
  // and only fixed local disks are queried: a mapped network drive or an empty
  // optical drive can stall Path-based existence checks for tens of seconds.
  for l in fixed_drive_letters() {
    let root = format!("{l}:\\");
    if let Some((total, free)) = drive_space(&root) {
      disks.push(DiskInfo {
        model: format!("{l}: Volume"),
        size_gb: Some(total),
        drive_letter: Some(format!("{l}:")),
        total_gb: Some(total),
        free_gb: Some(free),
      });
    }
  }
  // Physical disk models (real hardware names) — enumerates the fixed disk
  // registry locations; no subprocess needed on any mainstream Windows build.
  for disk in query_physical_disks() {
    disks.push(disk);
  }
  disks
}

/// Drive letters of local fixed disks via Win32 bitmasks — no filesystem calls,
/// so removable/mapped/empty drives can never stall the scan.
fn fixed_drive_letters() -> Vec<char> {
  #[link(name = "kernel32")]
  extern "system" {
    fn GetLogicalDrives() -> u32;
    fn GetDriveTypeW(lpRootPathName: *const u16) -> u32;
  }
  let mask = unsafe { GetLogicalDrives() };
  let mut out = Vec::new();
  for i in 0..26u32 {
    if mask & (1 << i) == 0 {
      continue;
    }
    let letter = (b'A' + i as u8) as char;
    let root: Vec<u16> = format!("{letter}:\\")
      .encode_utf16()
      .chain(std::iter::once(0))
      .collect();
    // GetDriveTypeW: 3 = DRIVE_FIXED. Skip removable (2), remote (4), cdrom (5).
    if unsafe { GetDriveTypeW(root.as_ptr()) } == 3 {
      out.push(letter);
    }
  }
  out
}

/// Physical disk models via the disk class registry (instant). Falls back to a
/// single PowerShell CIM query only if the registry enumeration finds nothing.
fn query_physical_disks() -> Vec<DiskInfo> {
  let mut disks = Vec::new();
  let base = r"SYSTEM\CurrentControlSet\Enum\SCSI";
  if let Ok(enum_key) = registry::open_key_ro(Hive::LocalMachine, base) {
    // Subkeys are like Disk&Ven_...&Prod_.... Take a bounded prefix: machines
    // expose a handful of disk groups, and one unusable subkey must never be
    // able to stall the whole scan.
    let disk_ids: Vec<String> = enum_key.enum_keys().take(32).collect::<Result<_, _>>().unwrap_or_default();
    for disk_id in disk_ids {
      let Ok(id_key) = registry::open_key_ro(Hive::LocalMachine, &format!("{base}\\{disk_id}")) else {
        continue;
      };
      // Serial subkeys (usually one per group); bounded per group.
      let serials: Vec<String> = id_key.enum_keys().take(16).collect::<Result<_, _>>().unwrap_or_default();
      for serial in serials {
        let dev_path = format!("{base}\\{disk_id}\\{serial}");
        let model = reg_sz(Hive::LocalMachine, &dev_path, "FriendlyName")
          .or_else(|| reg_sz(Hive::LocalMachine, &dev_path, "DeviceDesc"));
        if let Some(model) = model {
          if model.to_lowercase().contains("virtual") || disks.iter().any(|d: &DiskInfo| d.model == model) {
            continue;
          }
          disks.push(DiskInfo {
            model,
            size_gb: None,
            drive_letter: None,
            total_gb: None,
            free_gb: None,
          });
        }
      }
    }
  }
  if !disks.is_empty() {
    return disks;
  }
  // Fallback: one short whitelisted PowerShell query.
  let script = "Get-CimInstance Win32_DiskDrive | ForEach-Object { \"$($_.Model)|$([math]::Round($_.Size/1GB,0))\" }";
  let out = match winapi::run_powershell(script, std::time::Duration::from_secs(20)) {
    Ok(o) if o.exit_code == 0 => o,
    _ => return Vec::new(),
  };
  out
    .stdout_str()
    .lines()
    .take(64)
    .filter_map(|l| {
      let mut parts = l.trim().split('|');
      let model = parts.next()?.trim().to_string();
      let size = parts.next().and_then(|s| s.trim().parse::<u64>().ok());
      Some(DiskInfo {
        model,
        size_gb: size,
        drive_letter: None,
        total_gb: size,
        free_gb: None,
      })
    })
    .collect()
}

/// Free/total space for a drive root via winapi GetDiskFreeSpaceExW (raw FFI, no deps).
fn drive_space(root: &str) -> Option<(u64, u64)> {
  #[link(name = "kernel32")]
  extern "system" {
    fn GetDiskFreeSpaceExW(
      lpDirectoryName: *const u16,
      lpFreeBytesAvailableToCaller: *mut u64,
      lpTotalNumberOfBytes: *mut u64,
      lpTotalNumberOfFreeBytes: *mut u64,
    ) -> i32;
  }
  let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
  let mut total = 0u64;
  let mut free = 0u64;
  let mut avail = 0u64;
  let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut avail, &mut total, &mut free) };
  if ok != 0 {
    Some((total / (1024 * 1024 * 1024), free / (1024 * 1024 * 1024)))
  } else {
    None
  }
}

fn read_adapters() -> Vec<NetAdapterInfo> {
  // Enumerate the actual subkeys of NetworkCards (bounded) instead of guessing
  // index numbers — drivers can leave gaps, and gaps previously hid adapters.
  let mut adapters = Vec::new();
  let cards_key = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\NetworkCards";
  let Ok(cards) = registry::open_key_ro(Hive::LocalMachine, cards_key) else {
    return adapters;
  };
  let indexes: Vec<String> = cards
    .enum_keys()
    .take(32)
    .collect::<Result<_, _>>()
    .unwrap_or_default();
  for idx in indexes {
    let path = format!("{cards_key}\\{idx}");
    let service = match reg_sz(Hive::LocalMachine, &path, "ServiceName") {
      Some(s) => s,
      None => continue,
    };
    let description = reg_sz(Hive::LocalMachine, &path, "Description").unwrap_or_default();
    // Map ServiceName (connection GUID) to the friendly connection name via the
    // Network Connections registry.
    let conn_path = format!(
      r"SYSTEM\CurrentControlSet\Control\Network\{{4D36E972-E325-11CE-BFC1-08002BE10318}}\{service}\\connection"
    );
    let name = reg_sz(Hive::LocalMachine, &conn_path, "Name")
      .unwrap_or_else(|| description.clone());
    let enabled = registry::read_raw(Hive::LocalMachine, &format!(r"SYSTEM\CurrentControlSet\Services\{service}"), "Start")
      .ok()
      .flatten();
    let status = match enabled {
      Some(_) => "Available".to_string(),
      None => "Unknown".to_string(),
    };
    adapters.push(NetAdapterInfo { name, description, interface_guid: service, status });
  }
  adapters
}

fn unsafe_uptime_seconds() -> u64 {
  #[link(name = "kernel32")]
  extern "system" {
    fn GetTickCount64() -> u64;
  }
  unsafe { GetTickCount64() / 1000 }
}

fn format_uptime(secs: u64) -> String {
  let days = secs / 86400;
  let hours = (secs % 86400) / 3600;
  let mins = (secs % 3600) / 60;
  if days > 0 {
    format!("{days}d {hours}h {mins}m")
  } else if hours > 0 {
    format!("{hours}h {mins}m")
  } else {
    format!("{mins}m")
  }
}

fn hostname() -> String {
  std::env::var("COMPUTERNAME").unwrap_or_default()
}

fn username() -> String {
  std::env::var("USERNAME").unwrap_or_default()
}

/// GPU vendor of the primary adapter — reads the GPU registry class directly
/// (cached, no full re-scan).
pub fn primary_gpu_vendor() -> Option<GpuVendor> {
  read_gpus().first().map(|g| g.vendor)
}
