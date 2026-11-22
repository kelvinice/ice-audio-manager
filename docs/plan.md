# Ice Audio Manager: Implementation Plan

> **Status: APPROVED.** D-01 to D-14 signed off by the owner on 2026-11-22 (see the Decision Log in
> [planning-discussion.md](planning-discussion.md)). Stack: **Rust + Tauri 2 + Svelte 5, TOML + CSS** (D-01 to D-04).
> D-15 (memory policy), D-16 (device identity) and D-17 (spatial sound) were also signed off on 2026-11-22.
> Coding rules live in [code-practices.md](code-practices.md).

## Guiding constraints

1. **Lightweight:** budgets below are hard gates, measured every phase, not aspirations.
2. **Customizable:** every visual and behavioural knob lives in config/theme, not in code.
3. **Trustworthy:** no LL hooks, no admin, no packers, no silent network, open source at release, signed.
4. **Windows 11 only** (D-08): use native APIs directly. No cross-platform abstractions "just in case".

### Performance budgets (gates)

| Metric | Target | Hard limit | How measured |
|---|---|---|---|
| Idle CPU (no activity) | ~0.0% | < 0.2% avg over 60 s | Release build, 60 s sample after settling |
| Idle RAM, default profile (whole process tree, Settings closed) | **≤ 100 MB** (owner, Q-06) | **150 MB** (owner allowance, D-15) | Private Bytes summed over the host + all app-owned WebView2 processes (dedicated user-data folder, `GetProcessInfos`) |
| Idle RAM, `Balanced` profile (WebView released, native dot) | < 30 MB | < 40 MB | Same |
| RAM, `Lite` / gaming mode | < 25 MB | < 40 MB | Same, while a fullscreen game runs |
| Event → first visible feedback | < 1 frame (native dot reacts) | 50 ms | Log timestamps + screen capture |
| Event → full notch visible (`Balanced`, cold WebView) | ≤ 250 ms | 500 ms | Same |
| Notch animation | Display refresh rate (owner: **144 Hz** primary, **75 Hz** secondary) | Never below 60 fps; no dropped frames at 150% scaling | WebView2 DevTools perf trace on both monitors |
| Cold start → tray ready | < 1 s | < 2 s | Log timestamps |
| Device switch latency (hotkey → default changed) | < 150 ms | < 400 ms | Log timestamps |
| Installer size | < 10 MB | < 20 MB | CI artifact |

---

## Phase 0: Spikes (de-risk before building)

**Goal:** owner signs off D-01 to D-14, and the riskiest assumptions are proven with throwaway code in `spikes/`.

| # | Task | Owner (D-14) | Output |
|---|---|---|---|
| 0.1 | Owner signs off D-01 to D-14 in the Decision Log | Owner | Decision Log closed |
| 0.2 | **Overlay + memory spike** (details below) | Gemini + Codex | `spikes/overlay/` + a measurement table that **decides D-15** |
| 0.3 | **Core Audio spike:** enumerate endpoints, `IMMNotificationClient`, `IAudioEndpointVolumeCallback`, `IPolicyConfig::SetDefaultEndpoint` per role (re-read defaults after each switch), `IAudioMeterInformation` peak read. **Reproduce the reconnect bug (D-16):** Arctis dongle replug (same port + different port), headset power-cycle, SteelSeries GG/Sonar restart, sleep/resume. Log which endpoint IDs and fingerprint fields change. **Spatial sound research (D-17):** find the user-mode mechanism to read/list/set an endpoint's spatial format (Off / Windows Sonic / Dolby Atmos for Headphones), verified against the Windows Sound settings UI; no third-party binaries | Claude | `spikes/audio/` CLI printing events + an ID-churn log + a spatial get/set proof (or a documented "not feasible") |
| 0.4 | **Media spike:** GSMTC session list, current track, thumbnail stream, play/pause/next, **tested with Spotify desktop** | Claude | `spikes/media/` |
| 0.5 | **Fullscreen spike:** foreground-window bounds vs target monitor rect, with `SHQueryUserNotificationState` as a hint only (it sends no start/stop notification). Test borderless and exclusive fullscreen on either monitor and rapid Alt+Tab | Codex | Notes + snippet |
| 0.6 | Compare results with the budgets. Record go/no-go on D-01 and the D-15 default profile | All | Tables in the Decision Log |
| 0.7 | Tag `legacy-python-v0`, remove the Python app, scaffold the workspace (D-03) with typed IPC codegen. **Untrack `.env`** (`git rm --cached`, add to `.gitignore`), add an MIT `LICENSE` file | Claude | Buildable skeleton + CI green |

**Progress (2026-11-22):**
- ✅ 0.3 audio + spatial spike and ✅ 0.4 media spike (Claude): see [docs/spikes/phase0-audio-media.md](spikes/phase0-audio-media.md).
  Remaining: the owner runs `audio-spike watch` while replugging the Arctis (5 min) to record ID churn.
- ✅ 0.7 (Claude): `legacy-python-v0` tag, Python removed, `.env` untracked, MIT `LICENSE`, Rust workspace (`ice-core`, `ice-win`, spikes),
  CI (fmt, clippy `-D warnings`, test, release build). **Not yet:** Tauri `app/` scaffold + typed IPC codegen. These belong with the 0.2
  overlay spike (Gemini + Codex), which creates `app/`.
- ✅ 0.2 overlay spike code (Gemini) merged: `spikes/overlay` (Tauri + Svelte, typed IPC export, spring module, native dot,
  process metrics). **Missing:** the measurement write-up (`docs/spikes/phase0-overlay.md`) that picks the D-15 default profile.
- ✅ D-16 resolver (Claude): `ice_core::resolve` (`DeviceRef`, `resolve`, `toggle`), 12 tests built from the owner's real endpoints.
- ✅ Engine contract (Claude, `phase-1/engine-api`): `ice_core::engine`: `Command`, `EngineSnapshot`, `Envelope { revision, event }`,
  `EngineEvent`, `Notice`, and the `Engine` trait. **Codex and Gemini: build against `ice_core::engine::fake::FakeEngine`** (owner's devices,
  no Windows calls) until `phase-1/engine` merges. Enable `ice-core/ts` in the app to export TypeScript bindings (camelCase, `type`-tagged).
- ✅ 0.5 fullscreen spike (Codex): see [dual-monitor results](spikes/phase0-fullscreen.md). Phase 2 still needs to cover
  exclusive-mode transitions that can be missed by window notifications.

**0.2 details (Codex's measurement recipe):**
- Window: transparent, topmost, non-activating (`WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`, `WM_MOUSEACTIVATE → MA_NOACTIVATE`), no taskbar entry, one spring-animated pill.
- Click-through: toggle `set_ignore_cursor_events` from a bounded cursor poll near the pill. Verify that re-entry is detected while
  the window ignores input, that clicks and wheel outside the pill reach the app underneath, that controls inside work on the
  first click, and that typing in the foreground app is never interrupted. If polling loses clicks, compare against a shaped native host.
- Memory states to report: native tray-only · visible mini pill (WebView warm) · hidden · `TrySuspend`ed · WebView destroyed +
  native dot · after 10× Settings open/close. Report Private Bytes, private working set and average CPU separately.
- Latency: event → first visible feedback and → full notch, for warm and cold WebView, on both monitors, including mixed DPI and resume from sleep.
- Spring check: same step + mid-flight retarget at 75 Hz and 144 Hz.

**Exit criteria:** D-01 to D-14 signed off, and the D-15 default profile chosen from measured numbers. Switching works for all
3 roles. Volume events arrive from hardware keys **without any keyboard hook**.

---

## Parallel work: tracks, branches and file ownership (from 2026-11-22)

Three agents work at once, each in **its own worktree** branched from the latest `main` (code-practices §8).

| Track | Agent | Branches, in order | Depends on |
|---|---|---|---|
| **A: Engine** | Claude | `phase-1/engine-api` (small, merge first: commands, events, snapshot types in `ice-core`) → `phase-1/engine` (TOML config, `RegisterHotKey` hotkeys, audio thread: events → resolver → switch → confirm → re-link) | — |
| **B: App shell** | Codex | `phase-1/app-shell` (**`git mv spikes/overlay app`**, tray quick-switch menu, single instance, autostart abstraction, logging; codes against `engine-api`, using a fake engine until `engine` merges) → `phase-1/assets` (snowflake app + tray icons) | `engine-api` |
| **C: Notch UI** | Gemini | `phase-0/overlay-report` (D-15 numbers, docs only, start now) → `phase-2/notch-activities` (volume/switch/mic activities against a **mock event source**, native-dot theming) | `app-shell` merged (for the `app/` move) |

**Rules that keep the tracks from colliding:**
- **One owner per path.** Claude: `crates/ice-core/**`, the audio/hotkey/config/engine modules in `crates/ice-win`, `spikes/audio`, `spikes/media`.
  Codex: `app/src-tauri/**`, `crates/ice-win/src/{fullscreen,autostart}.rs`, `assets/**`, release/packaging CI.
  Gemini: `app/ui/**`, `crates/ice-win/src/{native_dot,overlay_win,monitors,webview2,proc_metrics}.rs`, `docs/theming.md`.
- **Shared hotspots:** root `Cargo.toml`, `Cargo.lock`, `crates/ice-win/Cargo.toml`, `crates/ice-win/src/lib.rs`, `ci.yml`, docs tables. Keep edits to
  one line each (one `pub mod`, one feature), rebase on `main` right before merging, and resolve conflicts as the union of both sides.
- **The `spikes/overlay` → `app/` move happens once, in Codex's `app-shell`,** merged early. Gemini starts `notch-activities` after that merge.
- **Contracts first:** cross-track interfaces (engine API, IPC types) merge as their own small branch before code that uses them.
- **Generated files** (e.g. `ui/src/lib/ipc/types.ts`) are only rewritten when their content changes, so test runs never dirty a worktree.

---

## Phase 1: Engine + tray switcher (parity with AudioSwitcher v1)

**Goal:** a daily-drivable, tray-only switcher that is already better than v1. **Features:** F01–F07.

- [ ] `ice-win::audio`: endpoint model (id, name, alias, flow, state, form factor/icon, is_default per role, virtual badge)
- [ ] **`DeviceRef` + resolver (D-16):** endpoint ID first; else a unique fingerprint match among **Active** endpoints → re-link and
      persist + notify; ambiguous → picker; missing → "not connected" message. Unit-tested in `ice-core` with fixtures built from the
      owner's real endpoint list (duplicate ghosts, Sonar virtual devices, BT hands-free pairs)
- [ ] **Toggle-pair hotkey** (Speakers ↔ Arctis) with one-click setup. This is the owner's primary workflow
- [ ] Ghost endpoints hidden by default; "Show disconnected" toggle
- [ ] `ice-win::audio`: device change notifications → `ice-core` events (debounced, coalesced)
- [ ] `ice-win::policy`: switch default. **`Default` = Console + Multimedia; `Both` = all three roles** for the selected flow.
      Treat each role change as observable. If one fails or the endpoint disappears mid-switch, re-read defaults and report which roles changed
- [ ] `ice-win::hotkeys`: `RegisterHotKey` on a hidden message window; conflict detection + user-facing error
- [ ] `ice-core::config`: TOML schema v1, load/validate/migrate, atomic save with external-edit conflict detection, file-watch reload
      (bad reload → keep last valid config + show error), explicit portable mode
- [ ] Tray: left-click quick-switch menu (outputs, separator, inputs, checkmarks), right-click full menu, tooltip shows current device.
      (Scroll-on-icon was cut by D-05.F05.)
- [ ] Favourites + cycle hotkey; hide devices; aliases; show disabled/disconnected toggles
- [ ] Autostart behind an `ice-win` abstraction: HKCU Run (unpackaged) / `StartupTask` (MSIX). Start minimized, single instance
- [ ] Logging: rotating file logs in `%LOCALAPPDATA%`, `--verbose` flag
- [ ] CI: fmt, clippy, tests, build on `windows-latest`. Unsigned **portable zip** artifact for private use
- [ ] Manual QA checklist (USB headset hot-plug, Bluetooth connect/disconnect, monitor sleep, duplicate device names)

**Exit criteria:** owner uses it as the daily switcher for 3 days with no crash. Budgets are met.
No hook DLLs or LL hooks are visible in Process Explorer. **The Speakers ↔ Arctis hotkey keeps working after unplugging and
replugging the dongle in a different USB port, power-cycling the headset, restarting SteelSeries GG, and sleep/resume.**

---

## Phase 2: Notch v1 (volume, switch, mic)

**Goal:** the "wow" moment. **Features:** F08–F14. **Content of the first public alpha (D-06).**

- [ ] Notch window: **exactly one** transparent window, moved between monitors with `SetWindowPos`, DPI-aware (per-monitor v2)
- [ ] **Performance profiles (D-15):** `Instant` (WebView always warm) · `Balanced` (WebView created on activity, released after
      a linger, default 60 s) · `Lite` (no notch WebView; native dot + tray only). Default per the Phase 0.2 result
- [ ] **Native idle dot/pill** (Direct2D/GDI layered window, ~1–2 MB est.) drawn whenever the WebView is released. It reacts within
      one frame to any event, and the WebView takes over seamlessly once ready
- [ ] **Gaming mode:** auto while a fullscreen app runs on the notch's monitor (or manual). The notch WebView stays released and
      **events do not recreate it** until gaming mode ends
- [ ] Activity Scheduler in `ice-core`: priorities, min/max durations, merge rules (repeat volume ticks extend one activity;
      a plug event merges with the resulting switch to the same device), snapshot + delta with monotonic revisions
- [ ] Activities: **Volume** (percent and mute glyph update instantly, only the fill springs, no entry pulse on repeats),
      **Device switch** (reveal the confirmed new endpoint), **Plug/unplug**, **Mic mute** (distinct mic glyph, not colour alone;
      one entrance pulse, then a static indicator)
- [ ] Interactions: hover-intent expand (120 ms), 250 ms leave grace, scroll = volume, Shift+scroll = switch device, middle-click = mute
- [ ] Placement: **pinned monitor is the default** (owner has 2 screens), remembered by display path. While it's disconnected, fall
      back to primary and return when it reconnects. Primary / cursor modes as options
- [ ] Idle state: **default = small visible notch** (dot or mini pill, picked from mockups over light and dark content at 100%/150%) (Q-10).
      `hidden` remains an option (clock idle mode was cut by D-05.F21)
- [ ] Springs: custom `(k, c, m)` module; presets Snappy / Smooth / Bouncy (retuned to ~8% overshoot) / Off (**instant, no fade**).
      Windows "Animation effects" off forces Off. Identical feel at 144 Hz and 75 Hz
- [ ] **Spatial sound on switch (D-17, experimental, only if the 0.3 spike succeeded):** per-device "On switch" action
      (keep / Off / Windows Sonic / Dolby Atmos / DTS), re-applied after a D-16 re-link; "Atmos" badge in the switch activity;
      clear message + fallback to `keep` when a format isn't available (e.g. Dolby Access not set up)
- [ ] Visual QA against Codex mockups at 100%/125%/150%/200% scaling; WCAG contrast check of text over light and dark desktops

**Exit criteria:** hardware volume keys, notch scroll and hotkeys all animate correctly. No focus stealing. Hidden over fullscreen
games on the pinned monitor. Budgets met in every profile.

### Alpha release (between Phase 2 and Phase 3)

- [ ] Owner uses the private portable build daily first
- [ ] Microsoft Store individual developer registration (free; identity verification) and a separate MSIX packaging step
      (Tauri's own outputs are NSIS/WiX). Smoke-test Core Audio, GSMTC, tray recovery, settings paths and `StartupTask` in the packaged build
- [ ] Audit the release tree and git history before the repo goes public; make it public under MIT (Q-02)
- [ ] Network trace of the release build: zero app-initiated requests (D-11)

---

## Phase 3: Media (now playing)

**Features:** F15–F17 and F18 (without the mixer). **Primary QA target: Spotify desktop** (Q-09).

- [ ] `ice-win::media`: GSMTC manager, session tracking (pick the "current" session sensibly when several play), timeline, playback status
- [ ] Album art pipeline: thumbnail stream → downscale in Rust (128 px) → cache by hash → served via a scoped `iceart://<id>` handler.
      Extract the accent in Rust and clamp it for contrast. Fall back to the brand accent when art is unavailable
- [ ] Activities: **Track change** (art + labels crossfade in fixed slots; controls never move; accent glow transition), compact **Now playing** idle state
- [ ] Expanded notch: media card (art, title, artist, scrubber, prev/play/next) + device switcher row. Hold expansion while the pointer moves into controls
- [ ] Visualizer: `IAudioMeterInformation` peaks at ≤ 30 Hz, subscribed **only while visible**, rendered as 4–6 bars with fast-attack/slow-decay springs
- [ ] Secondary QA: Chromium/Edge/Firefox media sessions

**Exit criteria:** track changes animate within 300 ms of the event. Visualizer costs < 1% CPU while visible and 0% while hidden.

---

## Phase 4: Settings & customization

**Features:** F19–F23.

- [ ] Settings window: created on demand, destroyed on close; Mica backdrop; light/dark
- [ ] Pages: General (incl. **Performance profile**) · Devices (aliases, hide, favourites, icons) · Hotkeys (recorder via `RegisterHotKey` test-bind) ·
      Notch (geometry, idle mode, pinned monitor) · Activities (toggles, durations, priorities) · Appearance (theme, accent, springs) · About (version)
- [ ] Theme system: bundled `Ice` (default, portfolio palette), `Midnight`, `Fluent`, `Minimal`. User themes = CSS files in
      `%APPDATA%\IceAudioManager\themes\` overriding documented `--ice-*` tokens, **validated before applying**, with **live preview**
- [ ] Strict CSP: a theme with remote `url()`/`@import` produces zero network requests (verified)
- [ ] Documented theme API (`docs/theming.md`): every token, its default, an example theme
- [ ] Import/export config + theme bundle (`.zip`)
- [ ] Every setting applies live without restart

**Exit criteria:** a user can build a theme without reading source code. Closing Settings returns memory to the idle budget
(verified after 10× open/close).

---

## Phase 5: Power features

**Features:** F24–F27.

- [ ] Per-app volume mixer (`IAudioSessionManager2`, `ISimpleAudioVolume`, session events) in the **expanded notch** (moved here by D-05.F18) + settings
- [ ] **Experimental** per-app output routing (`IAudioPolicyConfigFactory`, build-specific GUIDs, behind a flag, graceful failure)
- [ ] Profiles (device set + volumes + mic state) with hotkeys
- [ ] Rules: "when `process.exe` starts/focuses → apply profile" (event-driven, or low-rate polling; **no injection**)
- [ ] `icectl` CLI + `iceaudio://` URI handler (switch, volume, mute, profile) for Stream Deck / AutoHotkey

**Exit criteria:** profiles and rules work across reboot. Per-app routing fails safely on unsupported builds.

---

## Phase 6: Hardening, distribution & 1.0

- [ ] Signing per D-09: Store MSIX first; **SignPath** for GitHub artifacts once eligible (its terms require an OSS project with a prior release).
      Sign the exe inside the portable zip as well as the NSIS installer
- [ ] Packaging per D-10: MSIX, per-user NSIS installer (no admin), portable zip
- [ ] Release workflow: tag → CI build → sign → SHA256SUMS (after signing) → GitHub artifact attestation (public repo) → draft release
- [ ] Optional **notify-only** update check: off by default, never downloads (D-11). About page shows the version
- [ ] Release checklist: VirusTotal scan of final artifacts (record results); submit any false positive to Microsoft
- [ ] winget manifest PR
- [ ] Crash-resilience: COM call timeouts, audio-thread watchdog, recover from `AUDCLNT_E_DEVICE_INVALIDATED`, re-create tray on `TaskbarCreated`
- [ ] Accessibility: keyboard navigation in settings, screen-reader names, high-contrast fallback
- [ ] Docs: README with GIFs, theming guide, privacy statement (incl. what Windows/WebView2 do on their own), FAQ ("Is this malware?")

**Exit criteria:** signed 1.0 in the Store and on GitHub. A fresh Win11 install via the Store shows no SmartScreen warning;
0 or explained detections on VirusTotal.

---

## Backlog (unscheduled)

- F28 Bluetooth headset battery in notch
- F29 Optional native-flyout suppression (research only; never via injection or LL hooks)
- F05 tray-icon scrolling (cut by vote; EarTrumpet shows a hook-free raw-input approach if it's ever revisited)
- Equalizer / APO integration (probably out of scope)
- Spatial audio / sample-rate quick toggles
- Localization

## Dependency map

```
Phase 0 ─► Phase 1 ─► Phase 2 ─► Alpha ─► Phase 3 ─► Phase 4 ─► Phase 5 ─► Phase 6
                          └── Store registration + MSIX step can start in parallel after Phase 1
```
