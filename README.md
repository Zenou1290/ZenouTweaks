# Zenou Tweaks

A clean, lightweight **Windows optimization utility** built with **Tauri 2 + Rust +
React + TypeScript + Vite**.

Zenou Tweaks is a **vibecoded project**, developed with AI-assisted coding alongside
hands-on testing, debugging, refinement, and system-level experimentation. The goal is
to turn complex Windows tweaks into a simple, transparent, and reversible experience.

Zenou converts the operations found in `NOVA TWEAKS.bat` into individually controllable
tweaks with live state detection, apply-time value capture, verification, logging, and full
restore support. It never executes the `.bat`, never downloads anything, and never makes
a system change silently.

## Highlights

* **Zenou Scan** — real system detection: Windows edition/build, CPU, GPU + vendor/driver,
  RAM, storage, network adapters, uptime, elevation status.
* **75 tweaks** across Performance, Gaming, GPU, Network, and Windows/Privacy — each with
  current state, risk level, admin/restart requirements, before/after preview, apply and
  restore.
* **Hardware awareness** — NVIDIA/AMD/Intel tweaks only appear for the detected GPU vendor.
* **Quick Optimize** — previews the exact list of applicable tweaks, lets you deselect any,
  never applies risky tweaks automatically.
* **Profiles** — Gaming, Balanced, Low Latency, Streaming: plain collections of individual
  tweaks, fully reviewable before applying.
* **Safety stack** — optional Windows restore point before risky applies, automatic backups
  of every touched value, step-level rollback when a multi-step tweak fails midway, and
  verification before "Applied" is ever shown.
* **Changes timeline** — every modification grouped into reviewable change sets with
  per-tweak restore and "Restore all".
* **Backup & Restore** — manual and automatic snapshots stored in
  `%APPDATA%\ZenouTweaks\backups`.
* **Logs** — timestamped operation history with search, filtering, export, and clear.
* **Blocked ops** — the `.bat`'s dangerous operations (SmartScreen, CPU mitigations, VBS,
  Windows Update kill, BCDEdit, …) are deliberately blocked and listed in the app with
  reasons. Full audit: [BAT_AUDIT.md](BAT_AUDIT.md).

## Building

Prerequisites: Node 18+, Rust (MSVC toolchain), Windows 10/11.

```bash
npm install
npm run tauri dev      # run the desktop app in development
npm run tauri build    # produce the NSIS installer + portable exe
```

Frontend-only checks:

```bash
npm run check-types    # tsc
npm test               # vitest
npm run build          # tsc + vite production build
```

Rust checks (in `src-tauri`):

```bash
cargo check
cargo test
```

## Privacy

Zenou Tweaks is fully local and offline. No accounts, no telemetry, no analytics, no
downloads. Settings and backups live in `%APPDATA%\ZenouTweaks`.

## Layout

```text
src/                  React frontend
  components/         shared UI (tweak rows, dialogs, badges…)
  pages/              one file per navigation page
  lib/                typed invoke layer + utilities
  store/              zustand app state
  types/              types mirroring the Rust API
src-tauri/
  src/
    commands.rs       Tauri command handlers
    engine.rs         tweak engine (state, apply/verify/rollback)
    catalog.rs        tweak table (converted from the .bat)
    catalog_ext.rs    service/task group tweaks
    ops.rs             whitelisted system operation wrappers
    scan.rs            hardware/system detection
    backup.rs          snapshot store
    journal.rs         change-set persistence
    flagged.rs         blocked/excluded .bat operations
```
