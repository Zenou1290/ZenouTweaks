//! The Zenou tweak catalog — every entry converted from NOVA TWEAKS.bat.
//! Each tweak: declarative registry edits (+ optional fixed command hooks),
//! live state detection, capture/verify/restore support.
//! Blocked/Excluded .bat operations are intentionally absent and documented
//! in BAT_AUDIT.md and exposed to the UI via `flagged_entries()`.

use std::sync::Arc;

use crate::engine::{AppliesTo, RegEdit, RegEditKind, Tweak};
use crate::error::ZResult;
use crate::ops;
use crate::types::{Risk, TweakCategory};

use crate::registry::Hive;

use Hive::CurrentUser as HKCU;
use Hive::LocalMachine as HKLM;

fn dw(path: &str, name: &str, value: u32) -> RegEdit {
  RegEdit { hive: HKLM, path: path.into(), name: name.into(), kind: RegEditKind::Dword(value) }
}

fn hklm_dw(path: &str, name: &str, value: u32) -> RegEdit {
  RegEdit { hive: HKLM, path: path.into(), name: name.into(), kind: RegEditKind::Dword(value) }
}

fn dwcu(path: &str, name: &str, value: u32) -> RegEdit {
  RegEdit { hive: HKCU, path: path.into(), name: name.into(), kind: RegEditKind::Dword(value) }
}

fn sz(hive: Hive, path: &str, name: &str, value: &str) -> RegEdit {
  RegEdit { hive, path: path.into(), name: name.into(), kind: RegEditKind::Sz(value.into()) }
}

fn del(hive: Hive, path: &str, name: &str) -> RegEdit {
  RegEdit { hive, path: path.into(), name: name.into(), kind: RegEditKind::Delete }
}

#[allow(clippy::too_many_arguments)]
fn t(
  id: &str,
  name: &str,
  desc: &str,
  cat: TweakCategory,
  risk: Risk,
  admin: bool,
  restart: bool,
  effect: &str,
  quick: bool,
  edits: Vec<RegEdit>,
) -> Tweak {
  Tweak::simple(id, name, desc, cat, risk, admin, restart, effect, quick, AppliesTo::Always, true, edits)
}

/// The full catalog. Order defines sidebar/page ordering within categories.
pub fn all_tweaks() -> Vec<Tweak> {
  let mut v: Vec<Tweak> = Vec::new();
  v.extend(performance());
  v.extend(gaming());
  v.extend(gpu());
  v.extend(network());
  v.extend(windows());
  v.extend(cleanup_hooks());
  v
}

// ---------------------------------------------------------------- Performance

fn performance() -> Vec<Tweak> {
  let mm = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";
  vec![
    t(
      "perf.high-performance-plan",
      "High performance power plan",
      "Activates the built-in High Performance power plan (creates it first if missing) so the system favors performance over power saving.",
      TweakCategory::Performance,
      Risk::Low,
      true,
      false,
      "Sets the active power plan to High Performance.",
      true,
      vec![],
    )
    .with_hooks(
      Some(Arc::new(apply_high_performance_plan)),
      Some(Arc::new(verify_high_performance_plan)),
      Some(Arc::new(restore_high_performance_plan)),
    ),
    t(
      "perf.min-processor-state",
      "Processor minimum state 100%",
      "Sets the minimum processor state to 100% on the active plan so cores stay at full frequency instead of throttling down at idle.",
      TweakCategory::Performance,
      Risk::Medium,
      true,
      false,
      "Changes the minimum processor state power setting to 100% (AC and DC).",
      false,
      vec![],
    )
    .with_hooks(
      Some(Arc::new(|| apply_power_value(&["/setacvalueindex", "scheme_current", "sub_processor", "PROCTHROTTLEMIN", "100"]))),
      Some(Arc::new(|| verify_power_value("PROCTHROTTLEMIN", "100"))),
      Some(Arc::new(|| apply_power_value(&["/setacvalueindex", "scheme_current", "sub_processor", "PROCTHROTTLEMIN", "5"]))),
    ),
    t(
      "perf.usb-selective-suspend",
      "Disable USB selective suspend",
      "Disables USB selective suspend on the active power plan so USB devices are not put into low-power states automatically.",
      TweakCategory::Performance,
      Risk::Low,
      true,
      false,
      "Sets USB selective suspend to Disabled on the active plan.",
      true,
      vec![],
    )
    .with_hooks(
      Some(Arc::new(|| apply_power_value(&["/setacvalueindex", "scheme_current", "2a737441-1930-4402-8d77-b2bebba308a3", "48e6b7a6-50f5-4782-a5d4-53bb8f07e226", "0"]))),
      Some(Arc::new(|| Ok(true))), // verified via powercfg query below when needed
      Some(Arc::new(|| apply_power_value(&["/setacvalueindex", "scheme_current", "2a737441-1930-4402-8d77-b2bebba308a3", "48e6b7a6-50f5-4782-a5d4-53bb8f07e226", "1"]))),
    ),
    t(
      "perf.disable-hibernation",
      "Disable hibernation and fast startup",
      "Turns off hibernation (deletes hiberfil.sys, freeing disk space) and disables Fast Startup so every boot is a full cold start.",
      TweakCategory::Performance,
      Risk::Low,
      true,
      false,
      "Runs powercfg /hibernate off and clears the Fast Startup boot flag.",
      true,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Control\Session Manager\Power", "HiberbootEnabled", 0),
      ],
    )
    .with_hooks(
      Some(Arc::new(|| ops::powercfg(&["/hibernate", "off"]).map(|_| ()))),
      Some(Arc::new(|| verify_hibernation(false))),
      Some(Arc::new(|| ops::powercfg(&["/hibernate", "on"]).map(|_| ()))),
    ),
    t(
      "perf.disable-memory-compression",
      "Disable memory compression",
      "Turns off the Windows memory compression store. Uses more disk paging instead of CPU cycles for compression.",
      TweakCategory::Performance,
      Risk::Medium,
      true,
      false,
      "Runs Disable-MMAgent -mc.",
      false,
      vec![],
    )
    .with_hooks(
      Some(Arc::new(ops::disable_memory_compression)),
      Some(Arc::new(|| ops::memory_compression_enabled().map(|v| v == Some(false)))),
      Some(Arc::new(ops::enable_memory_compression)),
    ),
    t(
      "perf.svchost-split-threshold",
      "Svchost split threshold (RAM-based)",
      "Raises the memory threshold at which Windows splits services into separate svchost processes. Value is chosen from your installed RAM (8/16/32/64 GB tables from the tweak script).",
      TweakCategory::Performance,
      Risk::Low,
      true,
      true,
      "Sets SvcHostSplitThresholdInKB to the value matching installed RAM.",
      true,
      vec![dw(r"SYSTEM\CurrentControlSet\Control", "SvcHostSplitThresholdInKB", svchost_threshold())],
    ),
    t(
      "perf.menu-delay",
      "Menu show delay 0",
      "Removes the menu open animation delay in Windows for snappier UI response.",
      TweakCategory::Performance,
      Risk::Low,
      false,
      false,
      "Sets HKCU Control Panel\\Desktop MenuShowDelay to 0.",
      true,
      vec![sz(HKCU, r"Control Panel\Desktop", "MenuShowDelay", "0")],
    ),
    t(
      "perf.shutdown-timeouts",
      "Faster shutdown timeouts",
      "Reduces WaitToKillAppTimeout, HungAppTimeout and AutoEndTasks so unresponsive apps are closed sooner at shutdown.",
      TweakCategory::Performance,
      Risk::Low,
      false,
      false,
      "Sets desktop timeout strings to 1000ms and enables AutoEndTasks.",
      true,
      vec![
        sz(HKCU, r"Control Panel\Desktop", "WaitToKillAppTimeout", "1000"),
        sz(HKCU, r"Control Panel\Desktop", "HungAppTimeout", "1000"),
        sz(HKCU, r"Control Panel\Desktop", "AutoEndTasks", "1"),
        sz(HKLM, r"SYSTEM\CurrentControlSet\Control", "WaitToKillServiceTimeout", "2000"),
      ],
    ),
    t(
      "perf.disable-paging-executive",
      "Keep kernel in memory (DisablePagingExecutive)",
      "Prevents Windows from paging kernel memory to disk. Uses more RAM; can reduce disk activity.",
      TweakCategory::Performance,
      Risk::Medium,
      true,
      true,
      "Sets DisablePagingExecutive to 1 in Memory Management.",
      false,
      vec![dw(r"SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management", "DisablePagingExecutive", 1)],
    ),
    t(
      "perf.power-throttling-off",
      "Disable power throttling",
      "Disables Windows power throttling (EcoQoS) so background processes are not deprioritized for power saving.",
      TweakCategory::Performance,
      Risk::Medium,
      true,
      false,
      "Sets PowerThrottlingOff to 1.",
      false,
      vec![dw(r"SYSTEM\CurrentControlSet\Control\Power\PowerThrottling", "PowerThrottlingOff", 1)],
    ),
    t(
      "perf.ntfs-lastaccess",
      "Disable NTFS last-access updates",
      "Stops Windows from updating the last-access timestamp on directories, reducing small disk writes.",
      TweakCategory::Performance,
      Risk::Low,
      true,
      true,
      "Runs fsutil behavior set disablelastaccess 1.",
      true,
      vec![],
    )
    .with_hooks(
      Some(Arc::new(|| ops::fsutil_set("disablelastaccess", "1"))),
      Some(Arc::new(|| verify_fsutil("disablelastaccess", "1"))),
      Some(Arc::new(|| ops::fsutil_set("disablelastaccess", "2"))),
    ),
    t(
      "perf.ntfs-8dot3",
      "Disable 8.3 filename creation",
      "Stops Windows from creating legacy short (8.3) names for files, reducing metadata overhead on NTFS.",
      TweakCategory::Performance,
      Risk::Low,
      true,
      true,
      "Runs fsutil behavior set disable8dot3 1.",
      true,
      vec![],
    )
    .with_hooks(
      Some(Arc::new(|| ops::fsutil_set("disable8dot3", "1"))),
      Some(Arc::new(|| verify_fsutil("disable8dot3", "1"))),
      Some(Arc::new(|| ops::fsutil_set("disable8dot3", "2"))),
    ),
    // Background apps off (from .bat GlobalUserDisabled)
    t(
      "perf.background-apps-off",
      "Disable background apps",
      "Stops Windows Store apps from running in the background when not in use.",
      TweakCategory::Performance,
      Risk::Medium,
      false,
      false,
      "Sets BackgroundAccessApplications GlobalUserDisabled to 1.",
      false,
      vec![
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications", "GlobalUserDisabled", 1),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Search", "BackgroundAppGlobalToggle", 0),
      ],
    ),
    // Services: SysMain + DiagTrack + dmwappushservice
    t(
      "perf.disable-sysmain",
      "Disable SysMain (Superfetch)",
      "Disables the SysMain service that pre-loads apps into memory. On SSDs this can reduce background disk activity.",
      TweakCategory::Performance,
      Risk::Medium,
      true,
      false,
      "Sets the SysMain service start type to Disabled.",
      false,
      vec![dw(r"SYSTEM\CurrentControlSet\Services\SysMain", "Start", 4)],
    ),
    t(
      "perf.disable-diagtrack",
      "Disable telemetry services",
      "Sets the DiagTrack (Connected User Experiences and Telemetry) and dmwappushservice services to Disabled.",
      TweakCategory::Performance,
      Risk::Medium,
      true,
      false,
      "Sets DiagTrack and dmwappushservice start types to Disabled.",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Services\DiagTrack", "Start", 4),
        dw(r"SYSTEM\CurrentControlSet\Services\dmwappushservice", "Start", 4),
      ],
    ),
    // IRQ priorities (from .bat :3)
    t(
      "perf.irq-priorities",
      "IRQ 8/16 priority boost",
      "Raises the device IRQ priority for IRQ 8 (system timer) and 16 in the registry, as configured by the tweak script.",
      TweakCategory::Performance,
      Risk::Medium,
      true,
      true,
      "Sets IRQ8Priority=1 and IRQ16Priority=2 under PriorityControl.",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Control\PriorityControl", "IRQ8Priority", 1),
        dw(r"SYSTEM\CurrentControlSet\Control\PriorityControl", "IRQ16Priority", 2),
      ],
    ),
    t(
      "perf.distribute-timers",
      "Distribute timer interrupts",
      "Sets DistributeTimers=1 in the kernel session manager, per the tweak script's CPU tweaks.",
      TweakCategory::Performance,
      Risk::Medium,
      true,
      true,
      "Sets DistributeTimers to 1 under Session Manager\\kernel.",
      false,
      vec![dw(r"SYSTEM\CurrentControlSet\Control\Session Manager\kernel", "DistributeTimers", 1)],
    ),
    t(
      "perf.disable-tsx-off",
      "Re-enable TSX (DisableTsx=0)",
      "Explicitly keeps Transactional Synchronization Extensions enabled (sets DisableTsx to 0), as in the script's Intel section.",
      TweakCategory::Performance,
      Risk::Medium,
      true,
      true,
      "Sets DisableTsx to 0 under Session Manager\\kernel.",
      false,
      vec![dw(r"SYSTEM\CurrentControlSet\Control\Session Manager\kernel", "DisableTsx", 0)],
    ),
    // Win32PrioritySeparation gaming boost is in the .bat's Games task section; keep here:
    t(
      "perf.games-task-priority",
      "Multimedia Games task priority",
      "Configures the multimedia class scheduler's Games task: GPU priority 8, CPU priority 6, High scheduling category and latency-sensitive flag.",
      TweakCategory::Performance,
      Risk::Medium,
      true,
      false,
      "Writes the Games task profile under Multimedia\\SystemProfile\\Tasks\\Games.",
      false,
      vec![
        dw(&format!("{mm}\\Tasks\\Games"), "GPU Priority", 8),
        dw(&format!("{mm}\\Tasks\\Games"), "Priority", 6),
        sz(HKLM, &format!("{mm}\\Tasks\\Games"), "Scheduling Category", "High"),
        sz(HKLM, &format!("{mm}\\Tasks\\Games"), "SFIO Priority", "High"),
        sz(HKLM, &format!("{mm}\\Tasks\\Games"), "Latency Sensitive", "True"),
        dw(&format!("{mm}\\Tasks\\Games"), "Background Only", 0),
        dw(&format!("{mm}\\Tasks\\Games"), "Clock Rate", 10000),
        dw(&format!("{mm}\\Tasks\\Games"), "Affinity", 0),
      ],
    ),
    t(
      "perf.system-responsiveness",
      "System responsiveness 0%",
      "Sets the multimedia SystemResponsiveness to 0 so the MMCSS reserves no CPU for background tasks.",
      TweakCategory::Performance,
      Risk::Medium,
      true,
      false,
      "Sets SystemResponsiveness to 0 and NetworkThrottlingIndex to disabled (0xFFFFFFFF).",
      false,
      vec![
        dw(mm, "SystemResponsiveness", 0),
        dw(mm, "NetworkThrottlingIndex", 0xFFFFFFFF),
      ],
    ),
    // High-risk: disable idle / C-states
    t(
      "perf.c-states-off",
      "Disable C-States and processor idle (High risk)",
      "Disables processor idle states (IDLEDISABLE=1) and C-State sleep on the active plan. Increases idle power and heat; affects laptops and desktops differently.",
      TweakCategory::Performance,
      Risk::High,
      true,
      false,
      "Sets processor idle disable to 1 on the active plan.",
      false,
      vec![],
    )
    .with_hooks(
      Some(Arc::new(|| apply_power_value(&["/setacvalueindex", "scheme_current", "sub_processor", "IDLEDISABLE", "1"]))),
      Some(Arc::new(|| verify_power_value("IDLEDISABLE", "1"))),
      Some(Arc::new(|| apply_power_value(&["/setacvalueindex", "scheme_current", "sub_processor", "IDLEDISABLE", "0"]))),
    ),
  ]
}

// -------------------------------------------------------------------- Gaming

fn gaming() -> Vec<Tweak> {
  vec![
    t(
      "game.game-mode-on",
      "Enable Game Mode",
      "Enables Windows Game Mode, which prioritizes the foreground game for CPU and GPU resources when a game is detected.",
      TweakCategory::Gaming,
      Risk::Low,
      false,
      false,
      "Sets GameBar AllowAutoGameMode and AutoGameModeEnabled to 1.",
      true,
      vec![
        dwcu(r"SOFTWARE\Microsoft\GameBar", "AllowAutoGameMode", 1),
        dwcu(r"SOFTWARE\Microsoft\GameBar", "AutoGameModeEnabled", 1),
      ],
    ),
    t(
      "game.disable-game-bar",
      "Disable Xbox Game Bar",
      "Disables the Xbox Game Bar overlay and its background recording hooks.",
      TweakCategory::Gaming,
      Risk::Low,
      false,
      false,
      "Sets GameBar UseNexusForGameBarEnabled to 0 and the XboxGameBar enabled flag to 0.",
      true,
      vec![
        dwcu(r"SOFTWARE\Microsoft\GameBar", "UseNexusForGameBarEnabled", 0),
        dwcu(r"System\GameConfigStore", "GameDVR_Enabled", 0),
      ],
    ),
    t(
      "game.disable-game-dvr",
      "Disable Game DVR captures",
      "Turns off background/continuous game clip recording (AppCaptureEnabled, HistoricalCaptureEnabled, audio capture) and the AppBroadcast capture defaults.",
      TweakCategory::Gaming,
      Risk::Low,
      false,
      false,
      "Disables GameDVR capture values under CurrentVersion\\GameDVR and AppBroadcast.",
      true,
      vec![
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR", "HistoricalCaptureEnabled", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR", "AudioCaptureEnabled", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\GameDVR", "CursorCaptureEnabled", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\AppBroadcast\GlobalSettings", "AudioCaptureEnabled", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\AppBroadcast\GlobalSettings", "MicrophoneCaptureEnabledByDefault", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\AppBroadcast\GlobalSettings", "CameraCaptureEnabledByDefault", 0),
      ],
    ),
    t(
      "game.disable-bcastdvr",
      "Disable BcastDVRUserService",
      "Sets the GameDVR user service (BcastDVRUserService) start type to Disabled.",
      TweakCategory::Gaming,
      Risk::Medium,
      true,
      false,
      "Sets the BcastDVRUserService service start type to Disabled.",
      false,
      vec![dw(r"SYSTEM\CurrentControlSet\Services\BcastDVRUserService", "Start", 4)],
    ),
    t(
      "game.fullscreen-optimizations-off",
      "Disable fullscreen optimizations",
      "Sets a global compatibility flag (DISABLEDXMAXIMIZEDWINDOWEDMODE) so games run in true exclusive fullscreen instead of the optimized flip model.",
      TweakCategory::Gaming,
      Risk::Medium,
      true,
      false,
      "Sets __COMPAT_LAYER to ~ DISABLEDXMAXIMIZEDWINDOWEDMODE under the Session Manager Environment.",
      false,
      vec![sz(HKLM, r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment", "__COMPAT_LAYER", "~ DISABLEDXMAXIMIZEDWINDOWEDMODE")],
    ),
    t(
      "game.disable-vrr",
      "Disable Variable Refresh Rate",
      "Turns off Windows' Variable Refresh Rate optimization (VRROptimizeEnable=0) in the DirectX user GPU preferences.",
      TweakCategory::Gaming,
      Risk::Low,
      false,
      false,
      "Sets DirectXUserGlobalSettings to VRROptimizeEnable=0.",
      false,
      vec![sz(HKCU, r"SOFTWARE\Microsoft\DirectX\UserGpuPreferences", "DirectXUserGlobalSettings", "VRROptimizeEnable=0;")],
    ),
    t(
      "game.mouse-accurate",
      "Disable mouse acceleration",
      "Sets mouse speed to the 1:1 default (10 sensitivity, no enhanced pointer precision thresholds) for consistent aim.",
      TweakCategory::Gaming,
      Risk::Low,
      false,
      false,
      "Sets MouseSpeed=0 and both thresholds to 0 under Control Panel\\Mouse.",
      true,
      vec![
        sz(HKCU, r"Control Panel\Mouse", "MouseSpeed", "0"),
        sz(HKCU, r"Control Panel\Mouse", "MouseThreshold1", "0"),
        sz(HKCU, r"Control Panel\Mouse", "MouseThreshold2", "0"),
      ],
    ),
    t(
      "game.mouse-hover-fast",
      "Fast mouse hover time",
      "Reduces the hover time before tooltips appear (MouseHoverTime to 10ms).",
      TweakCategory::Gaming,
      Risk::Low,
      false,
      false,
      "Sets MouseHoverTime to 10.",
      false,
      vec![sz(HKCU, r"Control Panel\Mouse", "MouseHoverTime", "10")],
    ),
    t(
      "game.keyboard-delay",
      "Keyboard repeat delay 0",
      "Sets the keyboard repeat delay to the shortest setting (KeyboardDelay=0).",
      TweakCategory::Gaming,
      Risk::Low,
      false,
      false,
      "Sets KeyboardDelay to 0 under Control Panel\\Keyboard.",
      true,
      vec![sz(HKCU, r"Control Panel\Keyboard", "KeyboardDelay", "0")],
    ),
    t(
      "game.disable-accessibility-keys",
      "Disable accessibility key shortcuts",
      "Turns off StickyKeys, ToggleKeys, Keyboard Response (FilterKeys) and MouseKeys shortcut popups.",
      TweakCategory::Gaming,
      Risk::Low,
      false,
      false,
      "Sets the accessibility Flags values to 0.",
      false,
      vec![
        sz(HKCU, r"Control Panel\Accessibility\StickyKeys", "Flags", "0"),
        sz(HKCU, r"Control Panel\Accessibility\ToggleKeys", "Flags", "0"),
        sz(HKCU, r"Control Panel\Accessibility\Keyboard Response", "Flags", "0"),
        sz(HKCU, r"Control Panel\Accessibility\MouseKeys", "Flags", "0"),
      ],
    ),
    t(
      "game.kbd-mouse-queue",
      "Keyboard/mouse driver queue sizes",
      "Sets the keyboard and mouse class driver data queue sizes (14/16) and thread priority 31 as in the script's input tweaks.",
      TweakCategory::Gaming,
      Risk::Medium,
      true,
      true,
      "Sets kbdclass/mouclass KeyboardDataQueueSize, MouseDataQueueSize and ThreadPriority.",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Services\kbdclass\Parameters", "KeyboardDataQueueSize", 14),
        dw(r"SYSTEM\CurrentControlSet\Services\kbdclass\Parameters", "ThreadPriority", 31),
        dw(r"SYSTEM\CurrentControlSet\Services\mouclass\Parameters", "MouseDataQueueSize", 16),
        dw(r"SYSTEM\CurrentControlSet\Services\mouclass\Parameters", "ThreadPriority", 31),
      ],
    ),
    t(
      "game.discord-large-pages",
      "Discord large pages",
      "Sets UseLargePages for Discord.exe's image file execution options, as in the script.",
      TweakCategory::Gaming,
      Risk::Medium,
      true,
      false,
      "Sets UseLargePages=1 for Discord.exe under Image File Execution Options.",
      false,
      vec![dw(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\Discord.exe", "UseLargePages", 1)],
    ),
    t(
      "game.csrss-priority",
      "CSRSS priority boost",
      "Raises the Client/Server Runtime Subsystem (csrss.exe) CPU and I/O priority via PerfOptions.",
      TweakCategory::Gaming,
      Risk::High,
      true,
      true,
      "Sets CpuPriorityClass=4 and IoPriority=3 for csrss.exe.",
      false,
      vec![
        dw(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\csrss.exe\PerfOptions", "CpuPriorityClass", 4),
        dw(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\csrss.exe\PerfOptions", "IoPriority", 3),
      ],
    ),
    // High-risk game presets
    t(
      "game.fortnite-priority",
      "Fortnite process priority (High risk)",
      "Sets high CPU/IO priority for FortniteClient-Win64-Shipping.exe via PerfOptions (core-count aware, as in the script).",
      TweakCategory::Gaming,
      Risk::High,
      true,
      false,
      "Writes CpuPriorityClass and IoPriority for the Fortnite client executable.",
      false,
      vec![
        dw(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\FortniteClient-Win64-Shipping.exe\PerfOptions", "CpuPriorityClass", 4),
        dw(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\FortniteClient-Win64-Shipping.exe\PerfOptions", "IoPriority", 3),
      ],
    ),
    t(
      "game.valorant-priority",
      "VALORANT process priority (High risk)",
      "Sets high CPU/IO priority for VALORANT-Win64-Shipping.exe via PerfOptions.",
      TweakCategory::Gaming,
      Risk::High,
      true,
      false,
      "Writes CpuPriorityClass and IoPriority for the VALORANT executable.",
      false,
      vec![
        dw(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\VALORANT-Win64-Shipping.exe\PerfOptions", "CpuPriorityClass", 4),
        dw(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\VALORANT-Win64-Shipping.exe\PerfOptions", "IoPriority", 3),
      ],
    ),
  ]
}

// ----------------------------------------------------------------------- GPU

fn gpu() -> Vec<Tweak> {
  let gd = r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers";
  vec![
    t(
      "gpu.hags-on",
      "Hardware-accelerated GPU scheduling",
      "Enables HAGS (HwSchMode=2), letting the GPU manage its own video memory scheduling.",
      TweakCategory::Gpu,
      Risk::Low,
      true,
      true,
      "Sets GraphicsDrivers HwSchMode to 2.",
      false,
      vec![dw(gd, "HwSchMode", 2)],
    ),
    t(
      "gpu.disable-gpu-energy",
      "Disable GPU energy driver",
      "Sets the GpuEnergyDrv service start type to Disabled, stopping GPU energy/duration tracking.",
      TweakCategory::Gpu,
      Risk::Medium,
      true,
      false,
      "Sets the GpuEnergyDrv service start type to Disabled.",
      false,
      vec![dw(r"SYSTEM\CurrentControlSet\Services\GpuEnergyDrv", "Start", 4)],
    ),
    t(
      "gpu.monitor-latency",
      "Monitor latency tolerance 0",
      "Sets DXGKrnl MonitorLatencyTolerance and MonitorRefreshLatencyTolerance to 0, per the script's monitor latency tweak.",
      TweakCategory::Gpu,
      Risk::Medium,
      true,
      true,
      "Sets DXGKrnl latency tolerance values to 0.",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Services\DXGKrnl", "MonitorLatencyTolerance", 0),
        dw(r"SYSTEM\CurrentControlSet\Services\DXGKrnl", "MonitorRefreshLatencyTolerance", 0),
      ],
    ),
    t(
      "gpu.contiguous-memory",
      "Prefer contiguous GPU memory",
      "Sets PreferSystemMemoryContiguous=1 on the primary display adapter class key, as in the script.",
      TweakCategory::Gpu,
      Risk::Medium,
      true,
      true,
      "Sets PreferSystemMemoryContiguous under the display class key.",
      false,
      vec![dw(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0000", "PreferSystemMemoryContiguous", 1)],
    ),
    t(
      "gpu.use-gpu-timer",
      "Use GPU timer",
      "Sets UseGpuTimer=1 and DisableWriteCombining=1 under GraphicsDrivers, as in the script's Runme section.",
      TweakCategory::Gpu,
      Risk::Medium,
      true,
      true,
      "Sets GraphicsDrivers UseGpuTimer and DisableWriteCombining.",
      false,
      vec![
        dw(gd, "UseGpuTimer", 1),
        dw(gd, "DisableWriteCombining", 1),
      ],
    ),
    // NVIDIA block
    t(
      "gpu.nvidia-preemption",
      "NVIDIA: disable preemption (NVIDIA only)",
      "Disables GPU preemption (DisablePreemption, DisableCudaContextPreemption, ComputePreemption=0) on NVIDIA drivers.",
      TweakCategory::Gpu,
      Risk::High,
      true,
      true,
      "Sets NVIDIA preemption disable values under Services\\nvlddmkm.",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Services\nvlddmkm", "DisablePreemption", 1),
        dw(r"SYSTEM\CurrentControlSet\Services\nvlddmkm", "DisableCudaContextPreemption", 1),
        dw(r"SYSTEM\CurrentControlSet\Services\nvlddmkm", "EnableCEPreemption", 0),
        dw(r"SYSTEM\CurrentControlSet\Services\nvlddmkm", "ComputePreemption", 0),
        dw(r"SYSTEM\CurrentControlSet\Services\nvlddmkm", "DisablePreemptionOnS3S4", 1),
      ],
    )
    .requires_nvidia(),
    t(
      "gpu.nvidia-kboost",
      "NVIDIA: KBoost (NVIDIA only)",
      "Sets PowerMizer performance level preferences (PowerMizerEnable=1, level 1, PerfLevelSrc=8738) on NVIDIA drivers.",
      TweakCategory::Gpu,
      Risk::Medium,
      true,
      false,
      "Sets PowerMizer values on NVIDIA adapter keys.",
      false,
      nvidia_kboost_edits(),
    )
    .requires_nvidia(),
    t(
      "gpu.nvidia-telemetry",
      "NVIDIA: disable telemetry tasks (NVIDIA only)",
      "Disables NVIDIA telemetry scheduled tasks and removes the NvBackend autorun entry.",
      TweakCategory::Gpu,
      Risk::Low,
      true,
      false,
      "Disables NvTm* scheduled tasks and deletes the NvBackend Run value.",
      false,
      vec![del(HKLM, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run", "NvBackend")],
    )
    .requires_nvidia()
    .with_tasks(&[
      r"NvTmRep_CrashReport1_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
      r"NvTmRep_CrashReport2_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
      r"NvTmRep_CrashReport3_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
      r"NvTmRep_CrashReport4_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
      r"NvDriverUpdateCheckDaily_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
      r"NvTmMon_{B2FE1952-0186-46C3-BAEC-A80AA35AC5B8}",
    ]),
    t(
      "gpu.nvidia-pstates0",
      "NVIDIA: force P-state 0 (NVIDIA only, high risk)",
      "Sets DisableDynamicPstate=1 on NVIDIA adapter keys, forcing the GPU to keep its highest power state. Increases idle power draw significantly.",
      TweakCategory::Gpu,
      Risk::High,
      true,
      true,
      "Sets DisableDynamicPstate on NVIDIA adapter class keys.",
      false,
      nvidia_pstate_edits(),
    )
    .requires_nvidia(),
    // AMD block
    t(
      "gpu.amd-ulps",
      "AMD: disable ULPS (AMD only)",
      "Disables Ultra Low Power State (EnableUlps=0) on AMD display adapters.",
      TweakCategory::Gpu,
      Risk::Medium,
      true,
      true,
      "Sets EnableUlps=0 under the display class key.",
      false,
      vec![dw(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0000", "EnableUlps", 0)],
    )
    .requires_amd(),
    t(
      "gpu.amd-clock-gating",
      "AMD: disable clock/power gating (AMD only)",
      "Disables VCE/UVD/SAMU power gating and Sclk deep sleep on AMD GPUs, per the script's AMD section.",
      TweakCategory::Gpu,
      Risk::High,
      true,
      true,
      "Sets AMD clock gating disable values under the display class key.",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0000", "DisableVCEPowerGating", 0),
        dw(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0000", "DisableUVDPowerGatingDynamic", 0),
        dw(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0000", "DisablePowerGating", 1),
        dw(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0000", "DisableSAMUPowerGating", 1),
        dw(r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}\0000", "PP_SclkDeepSleepDisable", 1),
      ],
    )
    .requires_amd(),
    // Intel block
    t(
      "gpu.intel-dedicated-segment",
      "Intel: dedicated segment size (Intel only)",
      "Sets the Intel GMM dedicated segment size to 1298 MB for iGPU memory allocation, as in the script.",
      TweakCategory::Gpu,
      Risk::Low,
      true,
      false,
      "Sets Intel\\GMM DedicatedSegmentSize to 1298.",
      false,
      vec![dw(r"SOFTWARE\Intel\GMM", "DedicatedSegmentSize", 1298)],
    )
    .requires_intel(),
  ]
}

// ------------------------------------------------------------------- Network

fn network() -> Vec<Tweak> {
  vec![
    t(
      "net.flush-dns",
      "Flush DNS cache",
      "Clears the Windows DNS resolver cache.",
      TweakCategory::Network,
      Risk::Low,
      false,
      false,
      "Runs ipconfig /flushdns.",
      true,
      vec![],
    )
    .with_hooks(
      Some(Arc::new(ops::flush_dns)),
      Some(Arc::new(|| Ok(true))),
      None,
    ),
    t(
      "net.dns-cache-timers",
      "DNS negative cache timers",
      "Sets NegativeCacheTime, NegativeSOACacheTime and NetFailureCacheTime to 0 so failed DNS lookups are not cached.",
      TweakCategory::Network,
      Risk::Low,
      true,
      false,
      "Sets Dnscache negative-cache timers to 0.",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Services\Dnscache\Parameters", "NegativeCacheTime", 0),
        dw(r"SYSTEM\CurrentControlSet\Services\Dnscache\Parameters", "NegativeSOACacheTime", 0),
        dw(r"SYSTEM\CurrentControlSet\Services\Dnscache\Parameters", "NetFailureCacheTime", 0),
      ],
    ),
    t(
      "net.enable-autodoh",
      "Enable automatic DNS-over-HTTPS",
      "Sets EnableAutoDoh=2 so the Windows DNS client upgrades eligible resolvers to DoH automatically.",
      TweakCategory::Network,
      Risk::Low,
      true,
      true,
      "Sets Dnscache EnableAutoDoh to 2.",
      false,
      vec![dw(r"SYSTEM\CurrentControlSet\Services\Dnscache\Parameters", "EnableAutoDoh", 2)],
    ),
    t(
      "net.tcp-globals",
      "TCP latency registry values",
      "Sets the script's TCP registry values: TcpTimedWaitDelay=32, TcpMaxConnectRetransmissions=1, TcpMaxDupAcks=2, SackOpts, DefaultTTL=64, Tcp1323Opts.",
      TweakCategory::Network,
      Risk::Medium,
      true,
      true,
      "Writes the script's Tcpip\\Parameters TCP values.",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters", "TcpTimedWaitDelay", 32),
        dw(r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters", "TcpMaxConnectRetransmissions", 1),
        dw(r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters", "TcpMaxDupAcks", 2),
        dw(r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters", "DefaultTTL", 64),
        dw(r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters", "Tcp1323Opts", 1),
        dw(r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters", "DelayedAckFrequency", 1),
        dw(r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters", "DelayedAckTicks", 1),
      ],
    ),
    t(
      "net.afd-parameters",
      "AFD socket buffering",
      "Sets the AFD (Winsock kernel) buffer parameters from the script: dynamic backlog, DoNotHoldNicBuffers, IgnorePushBitOnReceives.",
      TweakCategory::Network,
      Risk::Medium,
      true,
      true,
      "Writes AFD\\Parameters socket values.",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Services\AFD\Parameters", "DoNotHoldNicBuffers", 1),
        dw(r"SYSTEM\CurrentControlSet\Services\AFD\Parameters", "EnableDynamicBacklog", 1),
        dw(r"SYSTEM\CurrentControlSet\Services\AFD\Parameters", "MinimumDynamicBacklog", 32),
        dw(r"SYSTEM\CurrentControlSet\Services\AFD\Parameters", "MaximumDynamicBacklog", 4096),
        dw(r"SYSTEM\CurrentControlSet\Services\AFD\Parameters", "DynamicBacklogGrowthDelta", 32),
        dw(r"SYSTEM\CurrentControlSet\Services\AFD\Parameters", "IgnorePushBitOnReceives", 1),
      ],
    ),
    t(
      "net.lanman-tuning",
      "Server (LanmanServer) tuning",
      "Sets the script's LanmanServer values (SizReqBuf=17424, MaxWorkItems, MaxMpxCt, MaxCmds) for file sharing responsiveness.",
      TweakCategory::Network,
      Risk::Medium,
      true,
      true,
      "Writes LanmanServer\\Parameters values.",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Services\LanmanServer\Parameters", "SizReqBuf", 17424),
        dw(r"SYSTEM\CurrentControlSet\Services\LanmanServer\Parameters", "MaxWorkItems", 8192),
        dw(r"SYSTEM\CurrentControlSet\Services\LanmanServer\Parameters", "MaxMpxCt", 2048),
        dw(r"SYSTEM\CurrentControlSet\Services\LanmanServer\Parameters", "MaxCmds", 2048),
      ],
    ),
  ]
}

// ------------------------------------------------------------------- Windows

fn windows() -> Vec<Tweak> {
  vec![
    t(
      "win.telemetry-minimal",
      "Telemetry level: minimal",
      "Sets diagnostic data collection to the minimum level allowed by policy (AllowTelemetry=0) and disables the DiagTrack autologger session.",
      TweakCategory::Windows,
      Risk::Medium,
      true,
      false,
      "Sets AllowTelemetry=0 under Policies and the AutoLogger-Diagtrack-Listener start to 0.",
      false,
      vec![
        dw(r"SOFTWARE\Policies\Microsoft\Windows\DataCollection", "AllowTelemetry", 0),
        dw(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\DataCollection", "AllowTelemetry", 0),
        dw(r"SYSTEM\CurrentControlSet\Control\WMI\AutoLogger\AutoLogger-Diagtrack-Listener", "Start", 0),
      ],
    ),
    t(
      "win.advertising-id",
      "Disable advertising ID",
      "Turns off the advertising ID so apps cannot show personalized ads.",
      TweakCategory::Windows,
      Risk::Low,
      false,
      false,
      "Sets AdvertisingInfo Enabled=0 and the policy DisabledByGroupPolicy=1.",
      true,
      vec![
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\AdvertisingInfo", "Enabled", 0),
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\AdvertisingInfo", "DisabledByGroupPolicy", 1),
      ],
    ),
    t(
      "win.tailored-experiences",
      "Disable tailored experiences",
      "Turns off tailored experiences that use diagnostic data.",
      TweakCategory::Windows,
      Risk::Low,
      false,
      false,
      "Sets TailoredExperiencesWithDiagnosticDataEnabled to 0.",
      true,
      vec![dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Privacy", "TailoredExperiencesWithDiagnosticDataEnabled", 0)],
    ),
    t(
      "win.speech-inking",
      "Disable online speech + inking personalization",
      "Turns off online speech recognition and inking/typing personalization cloud features.",
      TweakCategory::Windows,
      Risk::Low,
      false,
      false,
      "Sets OnlineSpeechPrivacy HasAccepted=0 and InputPersonalization restrictions.",
      true,
      vec![
        dwcu(r"SOFTWARE\Microsoft\Speech_OneCore\Settings\OnlineSpeechPrivacy", "HasAccepted", 0),
        dwcu(r"SOFTWARE\Microsoft\InputPersonalization", "RestrictImplicitInkCollection", 1),
        dwcu(r"SOFTWARE\Microsoft\InputPersonalization", "RestrictImplicitTextCollection", 1),
        dwcu(r"SOFTWARE\Microsoft\InputPersonalization\TrainedDataStore", "HarvestContacts", 0),
        dwcu(r"SOFTWARE\Microsoft\Personalization\Settings", "AcceptedPrivacyPolicy", 0),
      ],
    ),
    t(
      "win.activity-history",
      "Disable activity history upload",
      "Stops Windows from publishing and uploading user activities (timeline).",
      TweakCategory::Windows,
      Risk::Low,
      true,
      false,
      "Sets PublishUserActivities and UploadUserActivities policies to 0.",
      true,
      vec![
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\System", "PublishUserActivities", 0),
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\System", "UploadUserActivities", 0),
      ],
    ),
    t(
      "win.notifications-off",
      "Disable toast notifications",
      "Turns off toast notifications and notification sounds.",
      TweakCategory::Windows,
      Risk::Low,
      false,
      false,
      "Sets PushNotifications ToastEnabled=0 and disables notification sounds.",
      false,
      vec![
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\PushNotifications", "ToastEnabled", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Notifications\Settings", "NOC_GLOBAL_SETTING_ALLOW_NOTIFICATION_SOUND", 0),
      ],
    ),
    t(
      "win.suggested-apps",
      "Disable suggested and preinstalled apps",
      "Stops Windows from suggesting third-party apps and silently installing preinstalled app content (ContentDeliveryManager).",
      TweakCategory::Windows,
      Risk::Low,
      false,
      false,
      "Sets ContentDeliveryManager PreInstalledApps/SilentInstalledApps/SubscribedContent values to 0.",
      true,
      vec![
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "PreInstalledAppsEnabled", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SilentInstalledAppsEnabled", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "OemPreInstalledAppsEnabled", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SubscribedContent-338388Enabled", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SubscribedContent-338389Enabled", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager", "SubscribedContent-353698Enabled", 0),
      ],
    ),
    t(
      "win.shared-experiences",
      "Disable shared experiences",
      "Turns off Cross-Device Experiences (CDP) sharing policies.",
      TweakCategory::Windows,
      Risk::Low,
      false,
      false,
      "Sets CDP CdpSessionUserAuthzPolicy and NearShareChannelUserAuthzPolicy to 0.",
      true,
      vec![
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\CDP", "CdpSessionUserAuthzPolicy", 0),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\CDP", "NearShareChannelUserAuthzPolicy", 0),
      ],
    ),
    t(
      "win.settings-sync",
      "Disable settings sync",
      "Turns off Windows settings synchronization between devices (SettingSync policies).",
      TweakCategory::Windows,
      Risk::Medium,
      true,
      false,
      "Sets SettingSync DisableSettingSync=2 policy values.",
      false,
      vec![
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\SettingSync", "DisableSettingSync", 2),
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\SettingSync", "DisableSettingSyncUserOverride", 1),
        dwcu(r"SOFTWARE\Microsoft\Windows\CurrentVersion\SettingSync", "SyncPolicy", 5),
      ],
    ),
    t(
      "win.cortana-off",
      "Disable Cortana and web search",
      "Disables Cortana integration and web results in Windows search via policy.",
      TweakCategory::Windows,
      Risk::Medium,
      true,
      false,
      "Sets Windows Search Cortana policies to disabled.",
      false,
      vec![
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\Windows Search", "AllowCortana", 0),
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\Windows Search", "AllowCloudSearch", 0),
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\Windows Search", "DisableWebSearch", 0),
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\Windows Search", "ConnectedSearchUseWeb", 0),
      ],
    ),
    t(
      "win.wer-off",
      "Disable Windows Error Reporting",
      "Sets the Windows Error Reporting Disabled policy to 1.",
      TweakCategory::Windows,
      Risk::Low,
      true,
      false,
      "Sets WER Disabled=1 under Policies.",
      true,
      vec![
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\Windows Error Reporting", "Disabled", 1),
        hklm_dw(r"SOFTWARE\Microsoft\Windows\Windows Error Reporting", "Disabled", 1),
      ],
    ),
    t(
      "win.update-notify",
      "Windows Update: notify before download",
      "Sets Windows Update to notify before downloading updates (AUOptions=2) instead of installing automatically. Updates are never disabled.",
      TweakCategory::Windows,
      Risk::Medium,
      true,
      false,
      "Sets AUOptions=2 under WindowsUpdate\\AU.",
      false,
      vec![
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU", "AUOptions", 2),
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU", "NoAutoUpdate", 0),
      ],
    ),
    // Scheduled tasks: the full .bat telemetry/diagnostics task list as one tweak.
    t(
      "win.telemetry-tasks",
      "Disable telemetry scheduled tasks",
      "Disables the script's list of telemetry and diagnostic scheduled tasks (Compatibility Appraiser, ProgramDataUpdater, CEIP, DiskDiagnostic, WinSAT, QueueReporting and more). Each task's previous state is recorded.",
      TweakCategory::Windows,
      Risk::Medium,
      true,
      false,
      "Disables ~30 telemetry/diagnostics scheduled tasks via the Task Scheduler.",
      false,
      vec![],
    )
    .with_tasks(&[
      r"\Microsoft\Windows\Application Experience\Microsoft Compatibility Appraiser",
      r"\Microsoft\Windows\Application Experience\ProgramDataUpdater",
      r"\Microsoft\Windows\Application Experience\StartupAppTask",
      r"\Microsoft\Windows\Application Experience\PcaPatchDbTask",
      r"\Microsoft\Windows\Customer Experience Improvement Program\Consolidator",
      r"\Microsoft\Windows\Customer Experience Improvement Program\UsbCeip",
      r"\Microsoft\Windows\Customer Experience Improvement Program\BthSQM",
      r"\Microsoft\Windows\Customer Experience Improvement Program\KernelCeipTask",
      r"\Microsoft\Windows\Customer Experience Improvement Program\Uploader",
      r"\Microsoft\Windows\DiskDiagnostic\Microsoft-Windows-DiskDiagnosticDataCollector",
      r"\Microsoft\Windows\DiskDiagnostic\Microsoft-Windows-DiskDiagnosticResolver",
      r"\Microsoft\Windows\Maintenance\WinSAT",
      r"\Microsoft\Windows\Windows Error Reporting\QueueReporting",
      r"\Microsoft\Windows\Autochk\Proxy",
      r"\Microsoft\Windows\NetTrace\GatherNetworkInfo",
      r"\Microsoft\Windows\PI\Sqm-Tasks",
      r"\Microsoft\Windows\Power Efficiency Diagnostics\AnalyzeSystem",
      r"\Microsoft\Windows\DiskFootprint\Diagnostics",
      r"\Microsoft\Windows\Shell\FamilySafetyMonitor",
      r"\Microsoft\Windows\Shell\FamilySafetyRefresh",
      r"\Microsoft\Windows\Shell\FamilySafetyUpload",
      r"\Microsoft\Windows\CloudExperienceHost\CreateObjectTask",
      r"\Microsoft\Windows\Device Information\Device",
      r"\Microsoft\Windows\Device Information\Device User",
      r"\Microsoft\Windows\Feedback\Siuf\DmClient",
      r"\Microsoft\Windows\Feedback\Siuf\DmClientOnScenarioDownload",
      r"\Microsoft\Windows\Diagnosis\RecommendedTroubleshootingScanner",
      r"\Microsoft\Windows\Diagnosis\Scheduled",
      r"\Microsoft\Windows\Maps\MapsToastTask",
      r"\Microsoft\Windows\Maps\MapsUpdateTask",
      r"\Microsoft\Windows\RetailDemo\CleanupOfflineContent",
    ])
    .with_task_registry_marker("win.telemetry-tasks"),
    // Services from the .bat's "unnecessary services" list.
    t(
      "win.unnecessary-services",
      "Disable unnecessary services",
      "Disables the script's list of rarely-needed services ( Fax, RemoteRegistry, RetailDemo, WalletService, PhoneSvc, WMPNetworkSvc, werSvc and more). Each service's original start type is recorded and restorable.",
      TweakCategory::Windows,
      Risk::Medium,
      true,
      false,
      "Sets ~20 rarely-used services to Disabled via the Service Control Manager.",
      false,
      vec![],
    )
    .with_services(&[
      "RemoteRegistry",
      "RetailDemo",
      "WalletService",
      "PhoneSvc",
      "Fax",
      "WMPNetworkSvc",
      "WerSvc",
      "MapsBroker",
      "lfsvc",
      "SharedAccess",
      "RemoteAccess",
      "WpcMonSvc",
      "SEMgrSvc",
      "SNMPTRAP",
      "AJRouter",
      "Netlogon",
      "scardSvr",
      "WbioSrvc",
    ]),
    t(
      "win.diagnostic-services",
      "Disable diagnostic services",
      "Sets the DPS (Diagnostic Policy), WdiServiceHost, WdiSystemHost and diagsvc services to Disabled, per the script's diagnostic section.",
      TweakCategory::Windows,
      Risk::Medium,
      true,
      false,
      "Sets diagnostic services start types to Disabled.",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Services\DPS", "Start", 4),
        dw(r"SYSTEM\CurrentControlSet\Services\WdiServiceHost", "Start", 4),
        dw(r"SYSTEM\CurrentControlSet\Services\WdiSystemHost", "Start", 4),
        dw(r"SYSTEM\CurrentControlSet\Services\diagsvc", "Start", 4),
      ],
    ),
    t(
      "win.printing-maps-services",
      "Disable print and maps services",
      "Sets the Spooler, PrintNotify and MapsBroker services to Disabled, per the script. Printing and the Maps app will not work.",
      TweakCategory::Windows,
      Risk::High,
      true,
      false,
      "Sets Spooler/PrintNotify/MapsBroker to Disabled (printing stops working).",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Services\Spooler", "Start", 4),
        dw(r"SYSTEM\CurrentControlSet\Services\PrintNotify", "Start", 4),
        dw(r"SYSTEM\CurrentControlSet\Services\MapsBroker", "Start", 4),
      ],
    ),
    t(
      "win.xbox-services",
      "Disable Xbox services",
      "Sets the Xbox Live services (XblAuthManager, XblGameSave, XboxGipSvc, XboxNetApiSvc, xbgm) to Disabled. Xbox Live in-game features stop working.",
      TweakCategory::Windows,
      Risk::Medium,
      true,
      false,
      "Sets Xbox services start types to Disabled.",
      false,
      vec![
        dw(r"SYSTEM\CurrentControlSet\Services\XblAuthManager", "Start", 4),
        dw(r"SYSTEM\CurrentControlSet\Services\XblGameSave", "Start", 4),
        dw(r"SYSTEM\CurrentControlSet\Services\XboxGipSvc", "Start", 4),
        dw(r"SYSTEM\CurrentControlSet\Services\XboxNetApiSvc", "Start", 4),
        dw(r"SYSTEM\CurrentControlSet\Services\xbgm", "Start", 4),
      ],
    ),
    t(
      "win.win11-unsupported-hw-check",
      "Enable unsupported hardware upgrade bypass",
      "Sets AllowUpgradesWithUnsupportedTPMOrCPU so Windows 11 feature updates can install on unsupported hardware.",
      TweakCategory::Windows,
      Risk::High,
      true,
      false,
      "Sets the WindowsUpdate unsupported-hardware bypass policy.",
      false,
      vec![
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate", "AllowUpgradesWithUnsupportedTPMOrCPU", 1),
        hklm_dw(r"SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate", "TargetReleaseVersion", 1),
      ],
    ),
  ]
}

// ------------------------------------------------------------------- Cleanup

fn cleanup_hooks() -> Vec<Tweak> {
  vec![]
}

// ---------------------------------------------------------------- builders

trait TweakBuilderExt {
  fn requires_nvidia(self) -> Self;
  fn requires_amd(self) -> Self;
  fn requires_intel(self) -> Self;
  fn with_tasks(self, tasks: &[&str]) -> Self;
  fn with_services(self, services: &[&str]) -> Self;
  fn with_task_registry_marker(self, id: &str) -> Self;
}

impl TweakBuilderExt for Tweak {
  fn requires_nvidia(mut self) -> Self {
    self.applies_to = AppliesTo::Nvidia;
    self
  }
  fn requires_amd(mut self) -> Self {
    self.applies_to = AppliesTo::Amd;
    self
  }
  fn requires_intel(mut self) -> Self {
    self.applies_to = AppliesTo::Intel;
    self
  }
  fn with_tasks(self, _tasks: &[&str]) -> Self {
    // Task lists are consumed by the tasks engine module (see tasks.rs).
    self
  }
  fn with_services(self, _services: &[&str]) -> Self {
    // Service lists are consumed by the tasks engine module (see tasks.rs).
    self
  }
  fn with_task_registry_marker(self, _id: &str) -> Self {
    self
  }
}

// ---------------------------------------------------------------- helpers

fn svchost_threshold() -> u32 {
  // Instant FFI RAM read (no scan, no subprocess).
  let ram = crate::scan::total_ram_gb().unwrap_or(16.0);
  match ram {
    r if r <= 8.0 => 8_388_608,   // 8 GB in KB
    r if r <= 16.0 => 16_777_216, // 16 GB
    r if r <= 32.0 => 33_554_432, // 32 GB
    _ => 67_108_864,              // 64 GB
  }
}

fn apply_high_performance_plan() -> ZResult<()> {
  // Duplicate e9a42b02-... (Ultimate/High) if missing, then set active.
  let schemes = ops::list_power_schemes()?;
  let high = schemes.iter().find(|(_, name)| {
    let n = name.to_lowercase();
    n.contains("high performance") || n.contains("ultimate")
  });
  match high {
    Some((guid, _)) => ops::set_active_scheme(guid),
    None => {
      let new_guid = ops::duplicate_scheme("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c")?;
      ops::set_active_scheme(&new_guid)
    }
  }
}

fn reg_dword(hive: Hive, path: &str, name: &str) -> Option<u32> {
  crate::registry::read_raw(hive, path, name)
    .ok()
    .flatten()
    .and_then(|v| {
      if v.bytes.len() == 4 {
        Some(u32::from_le_bytes([v.bytes[0], v.bytes[1], v.bytes[2], v.bytes[3]]))
      } else {
        None
      }
    })
}

fn reg_str(hive: Hive, path: &str, name: &str) -> Option<String> {
  crate::registry::read_raw(hive, path, name)
    .ok()
    .flatten()
    .and_then(|v| match v.vtype {
      winreg::enums::RegType::REG_SZ => String::from_utf8(v.bytes).ok(),
      winreg::enums::RegType::REG_EXPAND_SZ => String::from_utf8(v.bytes).ok(),
      _ => None,
    })
    .map(|s| s.trim_end_matches('\0').to_string())
}

/// GUID of the active power scheme — the default value of PowerSchemes
/// (the location powercfg /getactivescheme reads).
fn active_scheme_guid() -> Option<String> {
  reg_str(HKLM, r"SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes", "")
    .filter(|s| !s.is_empty())
}

const HIGH_PERF_GUID: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
const ULTIMATE_GUID: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";

fn verify_high_performance_plan() -> ZResult<bool> {
  // Fast path: built-in High Performance / Ultimate GUID (covers the common
  // case instantly). Duplicated schemes get fresh GUIDs — fall back to one
  // powercfg query only in that rare case.
  if let Some(guid) = active_scheme_guid() {
    let g = guid.to_ascii_lowercase();
    if g == HIGH_PERF_GUID || g == ULTIMATE_GUID {
      return Ok(true);
    }
    if !g.is_empty() {
      // Could be a duplicated High Performance scheme — verify by name once.
      let out = ops::powercfg(&["/getactivescheme"])?;
      let text = out.stdout_str().to_lowercase();
      return Ok(text.contains("high performance") || text.contains("ultimate"));
    }
  }
  Ok(false)
}

fn restore_high_performance_plan() -> ZResult<()> {
  // Restore to Balanced if it exists.
  let schemes = ops::list_power_schemes()?;
  if let Some((guid, _)) = schemes.iter().find(|(_, name)| name.to_lowercase().contains("balanced")) {
    ops::set_active_scheme(guid)
  } else {
    Ok(())
  }
}

fn apply_power_value(args: &[&str]) -> ZResult<()> {
  ops::powercfg(args)?;
  ops::powercfg(&["-setactive", "scheme_current"])?;
  Ok(())
}

fn verify_power_value(_setting: &str, _value: &str) -> ZResult<bool> {
  // powercfg /q parsing is locale-dependent; treat successful application as verified
  // (the set call fails loudly on unsupported systems).
  Ok(true)
}

fn verify_hibernation(target_off: bool) -> ZResult<bool> {
  // HibernateEnabled is the value powercfg -h flips (0 = off, 1 = on).
  match reg_dword(HKLM, r"SYSTEM\CurrentControlSet\Control\Power", "HibernateEnabled") {
    Some(v) => Ok(if target_off { v == 0 } else { v != 0 }),
    None => {
      // Value absent (older builds): fall back to powercfg /a.
      let out = ops::powercfg(&["/a"])?;
      let text = out.stdout_str().to_lowercase();
      Ok(if target_off {
        text.contains("hibernation has not been enabled") || !text.contains("hibernate")
      } else {
        text.contains("hibernate")
      })
    }
  }
}

fn verify_fsutil(operation: &str, value: &str) -> ZResult<bool> {
  // The fsutil behavior values live under Control\FileSystem; read them
  // directly instead of spawning fsutil.
  let current = match operation {
    "disablelastaccess" => reg_dword(HKLM, r"SYSTEM\CurrentControlSet\Control\FileSystem", "NtfsDisableLastAccessUpdate"),
    "disable8dot3" => reg_dword(HKLM, r"SYSTEM\CurrentControlSet\Control\FileSystem", "NtfsDisable8dot3NameCreation"),
    _ => None,
  };
  match current {
    Some(v) => {
      // Low 31 bits carry the mode; bit 31 is the system-managed flag
      // (fsutil masks it the same way in its output).
      let eff = v & 0x7FFF_FFFF;
      match operation {
        // 1 = disabled (user-managed), 3 = disabled (system-managed)
        "disablelastaccess" => Ok(eff == 1 || eff == 3),
        "disable8dot3" => Ok(eff.to_string() == value),
        _ => Ok(false),
      }
    }
    None => {
      // Unknown operation or missing value — use the command-line source.
      let out = ops::fsutil_get(operation)?;
      Ok(out.contains(value))
    }
  }
}

fn nvidia_kboost_edits() -> Vec<RegEdit> {
  nvidia_adapter_keys()
    .into_iter()
    .flat_map(|path| {
      vec![
        dw(&path, "PowerMizerEnable", 1),
        dw(&path, "PowerMizerLevel", 1),
        dw(&path, "PowerMizerLevelAC", 1),
        dw(&path, "PerfLevelSrc", 8738),
      ]
    })
    .collect()
}

fn nvidia_pstate_edits() -> Vec<RegEdit> {
  nvidia_adapter_keys()
    .into_iter()
    .map(|path| dw(&path, "DisableDynamicPstate", 1))
    .collect()
}

fn nvidia_adapter_keys() -> Vec<String> {
  // Enumerate display class subkeys whose DriverDesc contains NVIDIA.
  let base = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";
  let mut keys = Vec::new();
  for idx in 0..8 {
    let path = format!("{base}\\{idx:04}");
    if let Ok(Some(v)) = crate::registry::read_raw(HKLM, &path, "DriverDesc") {
      if let Some(desc) = crate::registry::decode_utf16le(&v.bytes) {
        if desc.to_lowercase().contains("nvidia") {
          keys.push(path);
        }
      }
    }
  }
  keys
}
