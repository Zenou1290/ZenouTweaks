//! Blocked and Excluded `.bat` operations — surfaced in the UI with reasons.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FlagKind {
  /// Deliberately not implemented: security degradation, destructive, or unsafe.
  Blocked,
  /// Not a tweak / obsolete / ineffective / superseded by a safer implementation.
  Excluded,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlaggedEntry {
  pub kind: FlagKind,
  /// The .bat operation, summarized.
  pub operation: String,
  pub reason: String,
}

/// Every meaningful non-implemented operation from NOVA TWEAKS.bat, with reasons.
pub fn flagged_entries() -> Vec<FlaggedEntry> {
  vec![
    FlaggedEntry {
      kind: FlagKind::Blocked,
      operation: "Disable SmartScreen (EnablingSmartScreen=0, SmartScreenEnabled=Off, web content evaluation off)".into(),
      reason: "Disables a core Windows security feature that warns about untrusted downloads and apps.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Blocked,
      operation: "Disable CPU mitigations (FeatureSettings=1, FeatureSettingsOverride=3, EnableCfg=0, KernelSEHOPEnabled=0, DisableExceptionChainValidation=1, ProtectionMode=0, Set-ProcessMitigation -System -Disable ...)".into(),
      reason: "Disables Spectre/Meltdown and related CPU vulnerability mitigations. Security risk outweighs any measured benefit; Zenou does not modify security mitigations.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Blocked,
      operation: "Disable VBS/HVCI/DeviceGuard (EnableVirtualizationBasedSecurity=0, HypervisorEnforcedCodeIntegrity=0, LsaCfgFlags=0, vsmlaunchtype Off, hypervisorlaunchtype Off)".into(),
      reason: "Turns off virtualization-based security and credential guard. Core security protections are not modified.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Blocked,
      operation: "Permanently disable Windows Update (wuauserv/UsoSvc/WaaSMedicSvc/BITS start=4, NoAutoUpdate=1, DisableWindowsUpdateAccess=1, DoNotConnectToWindowsUpdateInternetLocations)".into(),
      reason: "Permanently disabling updates leaves the system unpatched. Zenou offers only the safe 'notify before download' option instead.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Blocked,
      operation: "BCDEdit boot tweaks (disabledynamictick, useplatformtick, tscsyncpolicy, bootux, quietboot, nx optout/alwaysoff, allowedinmemorysettings, isolatedcontext, vsmlaunchtype, x2apicpolicy...)".into(),
      reason: "Modifies boot configuration data; several entries also disable DEP/NX. Errors here can make Windows unbootable and some options are ignored on modern UEFI systems.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Blocked,
      operation: "GPU interrupt affinity pinning (DevicePolicy / AssignmentSetOverride on PCI devices)".into(),
      reason: "Pins device interrupts to specific cores; ineffective on modern interrupt controllers and can destabilize device interrupt handling.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Blocked,
      operation: "NIC offload disables (taskoffload=disabled, Disable-NetAdapterLso, Disable-NetAdapterIPsecOffload, RSC off)".into(),
      reason: "Disabling TCP offloads increases CPU usage and breaks some VPN/security software; treated as a security/stability regression.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Excluded,
      operation: "OverClock menu — download QuickCPU, Mem Reduct, MSI Util (Invoke-WebRequest from coderbag/GitHub/MediaFire)".into(),
      reason: "Zenou never downloads or runs third-party installers. Overclocking is out of scope for a system tweaks utility.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Excluded,
      operation: "ipconfig /release (run unconditionally before /renew)".into(),
      reason: "Cuts network connectivity until renewal completes; replaced by an explicit 'Renew network adapters' action with confirmation.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Excluded,
      operation: "del *.log /a /s /q /f on C:\\ (delete every .log on the drive)".into(),
      reason: "Deletes arbitrary application and system logs across the entire drive, including logs belonging to other software. Implemented instead as a narrowly-scoped, previewed cleanup.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Excluded,
      operation: "deltree C:\\windows\\tempor~1, c:\\windows\\history, c:\\windows\\cookies, del C:\\WIN386.SWP".into(),
      reason: "Windows 9x-era paths and commands that do not exist on Windows 10/11 (deltree was removed after Windows Me).".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Excluded,
      operation: "Set CSDVersion=1280/4096 ('200+ fps' tweak)".into(),
      reason: "Obsolete: CSDVersion described Windows NT service pack levels and does not influence rendering or frame rates; corrupting it misreports the OS version.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Excluded,
      operation: "SetACL.exe permission rewrites on USB device keys".into(),
      reason: "Depends on a third-party binary that is not part of Windows. The registry changes it guarded are implemented directly where safe.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Excluded,
      operation: "Tcpip QoS 'Do not use NLA' + XP-era TCP values (GlobalMaxTcpWindowSize=8760, TcpWindowSize, MaxUserPort)".into(),
      reason: "Legacy values ignored by the modern TCP stack; several are documented as deprecated since Windows Vista.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Excluded,
      operation: "Open msconfig/devmgmt/control/Taskmgr/SystemPropertiesProtection windows".into(),
      reason: "Not a tweak — opens Windows dialogs. Zenou shows the same underlying settings itself.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Excluded,
      operation: "Game folder openers for Fortnite/VALORANT (start Explorer at the install path)".into(),
      reason: "Not a tweak; opens a file location. The underlying priority tweaks are implemented individually.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Excluded,
      operation: "time synchronization tasks disabled (ForceSynchronizeTime, SynchronizeTime)".into(),
      reason: "Would silently desync the system clock, breaking TLS and authentication. Kept enabled deliberately.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Blocked,
      operation: "Uninstall Cortana appx (Get-AppxPackage *549981C3F5F10* | Remove-AppxPackage)".into(),
      reason: "Removing system components can break Windows Search updates. Search/Cortana policies are implemented without removing the package.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Blocked,
      operation: "Delete Windows Update files + re-create SoftwareDistribution, kill SystemSettings.exe".into(),
      reason: "Destructive to the update component. Implemented instead as a safe, previewed purge of only the Download cache with service state restore.".into(),
    },
    FlaggedEntry {
      kind: FlagKind::Excluded,
      operation: "Encoded PowerShell blob (base64 -encodedCommand) reconfiguring USB/NIC power management".into(),
      reason: "Opaque obfuscated command execution is never acceptable; its useful effects (USB/NIC power settings) are implemented transparently.".into(),
    },
  ]
}
