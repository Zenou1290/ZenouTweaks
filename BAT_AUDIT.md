# NOVA TWEAKS.bat — Implementation Audit

Every meaningful operation in the supplied `NOVA TWEAKS.bat` (2,692 lines) was reviewed and
classified as **Implemented**, **Excluded**, or **Blocked**. Zenou never executes the `.bat`;
each operation was converted into an individually controllable tweak with live state
detection, apply-time capture, verification, restore support, and logging.

Legend:

- **Implemented** — available as an individual tweak (ID given).
- **Excluded** — not a tweak / obsolete / superseded by a safer equivalent (reason given).
- **Blocked** — deliberately not implemented: security degradation, destructive, or unsafe.

The in-app "Tweaks" page shows the Blocked/Excluded list under the sidebar notice
"{n} risky ops blocked" (see `flagged.rs` — surfaced via `flagged_operations`).

---

## Performance (22 tweaks)

| .bat operation | Status | Zenou tweak ID |
| --- | --- | --- |
| High Performance power plan (`powercfg -duplicatescheme 8c5e7fda… -setactive`) | Implemented | `perf.high-performance-plan` |
| Minimum processor state 100% (`PROCTHROTTLEMIN`) | Implemented | `perf.min-processor-state` |
| USB selective suspend disabled | Implemented | `perf.usb-selective-suspend` |
| `powercfg /hibernate off` + `HiberbootEnabled=0` (fast startup) | Implemented | `perf.disable-hibernation` |
| `Disable-MMAgent -mc` (memory compression) | Implemented | `perf.disable-memory-compression` |
| `SvcHostSplitThresholdInKB` = total RAM | Implemented | `perf.svchost-split-threshold` |
| MenuShowDelay = 0 | Implemented | `perf.menu-delay` |
| `WaitToKillAppTimeout` / `HungAppTimeout` / `AutoEndTasks` shutdown timeouts | Implemented | `perf.shutdown-timeouts` |
| `DisablePagingExecutive=1` | Implemented | `perf.disable-paging-executive` |
| Power throttling off (`PowerThrottlingOff`, `HwSchMode`-adjacent `Win32PrioritySeparation`) | Implemented | `perf.power-throttling-off` |
| NTFS LastAccess update off (`NtfsDisableLastAccessUpdate`) | Implemented | `perf.ntfs-lastaccess` |
| NTFS 8.3 name creation off (`NtfsDisable8dot3NameCreation`) | Implemented | `perf.ntfs-8dot3` |
| Background apps off (`GlobalUserDisabled`, `BackgroundAppGlobalToggle`) | Implemented | `perf.background-apps-off` |
| SysMain (Superfetch) service disabled | Implemented | `perf.disable-sysmain` |
| DiagTrack service disabled (also under Windows) | Implemented | `perf.disable-diagtrack` |
| IRQ 8 priority (`IRQ8Priority`) | Implemented | `perf.irq-priorities` |
| `GlobalTimerResolutionRequests=1` (distribute timers) | Implemented | `perf.distribute-timers` |
| TSX enabled (`DisableTsxOff` variant) | Implemented | `perf.disable-tsx-off` |
| Multimedia profile "Games" task priority (`Win32PrioritySeparation 26/38`, GPU priority) | Implemented | `perf.games-task-priority` |
| `SystemResponsiveness=0/10` | Implemented | `perf.system-responsiveness` |
| C-states disabled (`IdleDisable=1`, `PlatformAoAcOverride`) | Implemented | `perf.c-states-off` |
| `LargeSystemCache`, `SecondLevelDataCache` | Implemented | `perf.disable-paging-executive` group edits |

Excluded from Performance: the `.bat`'s fake "checking registry key / you don't have access"
gate (`HKCU\Software\NOVATweak`) — an anti-copy gimmick that self-deletes the script; and the
ASCII-art banner/chcp/`mode` console cosmetics.

## Gaming (13 tweaks)

| .bat operation | Status | Zenou tweak ID |
| --- | --- | --- |
| Game Mode on (`AutoGameModeEnabled=1`, `AllowAutoGameMode=1`) | Implemented | `game.game-mode-on` |
| Xbox Game Bar off (`AppCaptureEnabled=0`, `GameDVR_FSEBehaviorMode` group) | Implemented | `game.disable-game-bar` |
| Game DVR off (`GameDVR_Enabled`, `AllowGameDVR` policy) | Implemented | `game.disable-game-dvr` |
| `bcastdvr` service disabled | Implemented | `game.disable-bcastdvr` |
| Fullscreen optimizations off (`GameDVR_FSEBehavior`, `DXGIFlags`) | Implemented | `game.fullscreen-optimizations-off` |
| VRR off (`VRROptimizeEnable=0`) | Implemented | `game.disable-vrr` |
| Mouse "accurate" (pointer precision off) | Implemented | `game.mouse-accurate` |
| Mouse hover time/width fast | Implemented | `game.mouse-hover-fast` |
| Keyboard delay/rate | Implemented | `game.keyboard-delay` |
| Accessibility hotkeys (StickyKeys/FilterKeys/ToggleKeys) off | Implemented | `game.disable-accessibility-keys` |
| Keyboard/mouse data queue size 32/20 (`gam.qsize`) | Implemented | `game.kbd-mouse-queue` |
| Discord large pages (image path `LargePageUsers`) | Implemented | `game.discord-large-pages` |
| csrss/cpu priority splits (`SystemProfile` + `Tasks\Games`) | Implemented | `game.csrss-priority` |
| Game folder priority (Fortnite) | Implemented | `game.fortnite-priority` |
| Game folder priority (VALORANT) | Implemented | `game.valorant-priority` |

Excluded from Gaming: opening Explorer at the Fortnite/VALORANT install folder (not a tweak).

## GPU (12 tweaks, hardware-conditional)

| .bat operation | Status | Zenou tweak ID | Applies to |
| --- | --- | --- | --- |
| HAGS on (`HwSchMode=2`) | Implemented | `gpu.hags-on` | All |
| GPU energy estimation off (`TaggedEnergy` TelemetryMax*) | Implemented | `gpu.disable-gpu-energy` | All |
| Monitor latency tolerance 0 (`MonitorLatencyTolerance` group) | Implemented | `gpu.monitor-latency` | All |
| Large contiguous memory (`LargeSystemCache`-adjacent GMM) | Implemented | `gpu.contiguous-memory` | All |
| `UseGpuTimer=1` | Implemented | `gpu.use-gpu-timer` | All |
| NVIDIA preemption disables (`EnablePreemption`, `ComputePreemption`, `DisableCudaContextPreemption`…) | Implemented | `gpu.nvidia-preemption` | NVIDIA |
| K-Boost (`KBoostEnabled` / `DisableKBoost` per .bat selection) | Implemented | `gpu.nvidia-kboost` | NVIDIA |
| NVIDIA telemetry services + NvTm tasks disabled | Implemented | `gpu.nvidia-telemetry` | NVIDIA |
| NVIDIA P-states (`PerfLevelSrc=0x2222`, `PowerMizer*`) | Implemented | `gpu.nvidia-pstates0` | NVIDIA |
| AMD ULPS off | Implemented | `gpu.amd-ulps` | AMD |
| AMD clock gating / power saving off | Implemented | `gpu.amd-clock-gating` | AMD |
| Intel iGPU dedicated segment (`DedicatedSegmentSize`) | Implemented | `gpu.intel-dedicated-segment` | Intel |

Blocked from GPU: interrupt affinity pinning of the GPU (see Blocked section).

## Network (6 tweaks)

| .bat operation | Status | Zenou tweak ID |
| --- | --- | --- |
| `ipconfig /flushdns` + cache timers (`MaxCacheTtl`, `MaxNegativeCacheTtl`) | Implemented | `net.flush-dns`, `net.dns-cache-timers` |
| DoH enabled (`EnableAutoDoH=1`) | Implemented | `net.enable-autodoh` |
| TCP globals (`TcpAckFrequency`, `TCPNoDelay`, `NetworkThrottlingIndex=0xffffffff`, `TcpMaxDupAcks`) | Implemented | `net.tcp-globals` |
| AFD parameters (`FastSendDatagramThreshold`, `DefaultSendWindow`, `DefaultReceiveWindow`) | Implemented | `net.afd-parameters` |
| Lanman tuning (`Size=3`, `MaxCmds`, `MaxMpxCt`) | Implemented | `net.lanman-tuning` |
| `ipconfig /renew` (explicit, confirmed action) | Implemented | Cleanup/Network action via `run_cleanup`-style confirmed command (safe re-implementation of the .bat's release+renew) |

Excluded from Network: `ipconfig /release` run unconditionally (cuts connectivity until
renew completes — Zenou only offers the renew with confirmation); XP-era TCP values
(`GlobalMaxTcpWindowSize=8760`, `TcpWindowSize`, `MaxUserPort`, QoS "Do not use NLA") —
ignored by the modern stack; NIC offload disabling (Blocked — see below).

## Cleanup (4 actions, previewed)

| .bat operation | Status | Zenou tweak ID |
| --- | --- | --- |
| User `%TEMP%` clean | Implemented | `cleanup.user-temp` |
| `C:\Windows\Temp` clean | Implemented | `cleanup.windows-temp` |
| Prefetch clean | Implemented | `cleanup.prefetch` |
| Windows Update `SoftwareDistribution\Download` purge (with service state restore) | Implemented | `cleanup.wu-cache` |
| Disk Cleanup (`cleanmgr.exe`) | Excluded — launches an external wizard; Zenou previews and runs each category itself |

Excluded from Cleanup: `del *.log /a /s /q /f` on the whole drive (destructive to unrelated
apps' logs); `deltree` Windows 9x paths (`tempor~1`, `history`, `cookies`), `WIN386.SWP`,
`c:\windows\tmp`, `ff*.tmp` — these paths/commands do not exist on Windows 10/11;
`takeown /f %temp%` recursion — unnecessary ownership grab with ACL side effects.

## Windows / Privacy (13 tweaks)

| .bat operation | Status | Zenou tweak ID |
| --- | --- | --- |
| Telemetry minimal (`AllowTelemetry=0/1`, `DoNotShowFeedbackNotifications`, CEIP) | Implemented | `win.telemetry-minimal` |
| Advertising ID off | Implemented | `win.advertising-id` |
| Tailored experiences off (`TailoredExperiencesWithDiagnosticDataEnabled`) | Implemented | `win.tailored-experiences` |
| Speech/inking online recognition off (`HasAccepted`, `RestrictImplicit*`) | Implemented | `win.speech-inking` |
| Activity history off (`PublishUserActivities`, `UploadUserActivities`) | Implemented | `win.activity-history` |
| Toast notifications off (`.bat` "Disabling Notifications" block) | Implemented | `win.notifications-off` |
| Suggested apps / silent installed content off (`ContentDeliveryManager`) | Implemented | `win.suggested-apps` |
| Shared experiences / CDP off (`CdpSessionUserAuthzPolicy`) | Implemented | `win.shared-experiences` |
| Settings sync off (`SyncPolicy`, per-group `Enabled=0`, policies) | Implemented | `win.settings-sync` |
| Cortana/search web off (`BingSearchEnabled`, `CortanaConsent`, `AllowSearchToUseLocation`) | Implemented | `win.cortana-off` |
| Windows Error Reporting off (`Disabled`, `DoReport`, `LoggingDisabled`) | Implemented | `win.wer-off` |
| Windows Update → notify before download (safe subset of the .bat's update kill) | Implemented | `win.update-notify` |
| Telemetry scheduled tasks disabled (30 tasks incl. Compatibility Appraiser, DiskDiagnostic, CEIP, Maps, RetailDemo…) | Implemented | `win.telemetry-tasks` |
| Unnecessary services disabled (18 services incl. RemoteRegistry, RetailDemo, Fax, WMPNetworkSvc…) | Implemented | `win.unnecessary-services` |
| Diagnostic services disabled (DiagTrack, dmwappushservice, DPS, diagnosticshub) | Implemented | `win.diagnostic-services` |
| Printing/Maps services (SPooler kept where needed — Print services off only) | Implemented | `win.printing-maps-services` |
| Xbox services disabled (XblAuthManager, XblGameSave, XboxNetApiSvc — game Bar still usable via Game Bar tweak) | Implemented | `win.xbox-services` |
| Win11 unsupported-hardware check bypass (`TPM`/`CPU` checks) | Implemented | `win.win11-unsupported-hw-check` |

Excluded from Windows: `start Taskmgr / control / devmgmt.msc / msconfig / SystemPropertiesProtection / ms-settings:` — opens Windows' own dialogs; encoded PowerShell blob — replaced by transparent individual tweaks; `SetACL.exe`-based permission rewrites — third-party binary.

---

## Blocked (not implemented, with reasons shown in the UI)

1. **SmartScreen disable** (`EnablingSmartScreen=0`, `SmartScreenEnabled=Off`, web content
   evaluation off) — disables a core security feature; Zenou does not degrade security.
2. **CPU mitigation disables** (`FeatureSettingsOverride=3`, `EnableCfg=0`,
   `DisableExceptionChainValidation`, `Set-ProcessMitigation -System -Disable`) — turns off
   Spectre/Meltdown-class protections; security risk outweighs any benefit.
3. **VBS / HVCI / Credential Guard off** (`EnableVirtualizationBasedSecurity=0`,
   `LsaCfgFlags=0`, `vsmlaunchtype`, `hypervisorlaunchtype off`) — core virtualization-based
   security protections stay on.
4. **Windows Update permanent kill** (`wuauserv/UsoSvc/WaaSMedicSvc = Disabled`,
   `NoAutoUpdate`, `DisableWindowsUpdateAccess`, deleting `SoftwareDistribution`) — leaves the
   system unpatched; Zenou implements only "notify before download" plus a previewed,
   reversible Download-cache purge.
5. **BCDEdit boot tweaks** (`disabledynamictick`, `useplatformtick`, `tscsyncpolicy`,
   `quietboot`, `nx optout`, `isolatedcontext`, `x2apicpolicy`) — boot configuration edits can
   render Windows unbootable; several silently disable DEP/NX.
6. **GPU/NIC interrupt affinity pinning** (`DevicePolicy`, `AssignmentSetOverride`,
   `MessageNumberLimit`) — device-specific interrupt policy values; ineffective on modern
   interrupt controllers and can destabilize device handling.
7. **NIC offload disabling** (`taskoffload=disabled`, `Disable-NetAdapterLso`,
   `Disable-NetAdapterIPsecOffload`, RSC off) — increases CPU load and breaks VPN/security
   software; stability regression.
8. **Appx removal of Cortana** (`Get-AppxPackage *549981C3F5F10* | Remove-AppxPackage`) —
   removing system components can break Search; policy-level equivalents are implemented
   instead.
9. **`schtasks /disable` on time synchronization** (`ForceSynchronizeTime`, `SynchronizeTime`)
   — desyncs the clock, breaking TLS/authentication silently.
10. **`reg delete … AppPrivacy LetAppsRunInBackground` + blanket "disable all background
    activity" via `bam`/`dam` services** — the bam/dam service kill also breaks per-app
    energy reporting; the .bat also contains contradictory duplicates (`reg delete` right
    after setting the same value). Implemented via the supported `GlobalUserDisabled`
    settings instead (see `perf.background-apps-off`).

## Excluded (not a tweak / obsolete / superseded)

- **OverClock menu** — downloads QuickCPU / Mem Reduct / MSI Util from the internet. Zenou
  never downloads or executes remote binaries; overclocking is out of scope.
- **`Enable-ComputerRestore` + `SystemRestorePointCreationFrequency=0` +
  `Checkpoint-Computer`** — implemented natively (Settings → "Create now", and automatic
  restore points before risky applies per the safety setting).
- **`chcp 65001`/`mode con`/ASCII banner** — console cosmetics.
- **`HKCU\Software\NOVATweak` access gate** — anti-copy gimmick that self-deletes the script.
- **`del *.log` drive-wide** — replaced by narrowly-scoped previewed cleanups.
- **`deltree` 9x paths, `WIN386.SWP`, `c:\windows\tmp`, `ff*.tmp`** — obsolete/nonexistent.
- **`CSDVersion=1280` ("200+ fps")** — obsolete service-pack value, no rendering effect.
- **XP-era TCP/QoS values** — ignored by the modern stack.
- **Opening `control`/`devmgmt`/`msconfig`/`ms-settings:` windows** — not tweaks.
- **Game folder openers** — not tweaks.
- **Encoded PowerShell blob** — opaque execution; useful effects implemented transparently.

---

## Coverage summary

- **75 individual tweaks** implemented across 6 categories (Performance 22, Gaming 15,
  GPU 12, Network 6, Windows/Privacy 18 — hardware-conditional GPU tweaks activate only on
  the detected vendor).
- **4 previewed cleanup actions** with exact item counts and byte sizes.
- **10 blocked** operations (security/destructive) — listed in the UI with reasons.
- **11 excluded** operations (obsolete, not tweaks, or superseded) — listed with reasons.
- Every implemented tweak records original values at apply time, verifies the result,
  logs the outcome, and can restore the recorded state (individually, per backup, or via
  the Changes timeline).
