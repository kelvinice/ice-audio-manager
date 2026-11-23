# Ice Audio Manager: Implementation Plan

> **Status: APPROVED.** D-01 to D-14 signed off by the owner on 2026-11-22 (see the Decision Log in
> [planning-discussion.md](planning-discussion.md)). Stack: **Rust + Tauri 2 + Svelte 5, TOML + CSS** (D-01 to D-04).
> D-15 (memory policy), D-16 (device identity) and D-17 (spatial sound) were also signed off on 2026-11-22.
> The owner deferred the first public release to [Phase 6](#phase-6-hardening-and-first-public-release).
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

**Phase 0 status: ✅ complete (2026-11-22).** All of 0.1–0.7 are done; the notes below are kept as the record.

**Progress (2026-11-22):**
- ✅ 0.1 owner sign-off of D-01–D-17; ✅ 0.6 D-15.1 closed on the measurements (A: WebView notch + `TrySuspend`).
- ✅ 0.3 audio + spatial spike and ✅ 0.4 media spike (Claude): see [docs/spikes/phase0-audio-media.md](spikes/phase0-audio-media.md).
  ID churn recorded by the owner: power-cycle and same-port replug keep the IDs; a **different port** gives new endpoint,
  stable and container IDs (fixed in the resolver, QA B3/B4 ✅). Sleep/resume (QA B5) still to run.
- ✅ 0.7 (Claude): `legacy-python-v0` tag, Python removed, `.env` untracked, MIT `LICENSE`, Rust workspace (`ice-core`, `ice-win`, spikes),
  CI (fmt, clippy `-D warnings`, test, release build). The Tauri `app/` scaffold and typed IPC codegen came with Codex's `app-shell`.
- ✅ 0.2 overlay spike + measurements (Gemini): `spikes/overlay` + [docs/spikes/phase0-overlay.md](spikes/phase0-overlay.md).
  60 s release numbers: native dot `2.20 MB` Private Bytes (`0.000%` CPU, `0.02 ms` p95); warm/hidden WebView2 `149.84–150.39 MB` Private Bytes (`77.10–76.79 MB` Private WS); `TrySuspend` `148.01 MB` Private Bytes / **`6.44 MB` Private WS** (`27.24 ms` p95 resume); cold WebView2 spawn **`715.44 ms` p95**; 0 leaked PIDs on destroy. The strict D-15 gate pointed at Option C; **D-15.1 then chose A** (WebView notch + `TrySuspend`, budgets in private working set), with C as the fallback.
- ✅ D-16 resolver (Claude): `ice_core::resolve` (`DeviceRef`, `resolve`, `toggle`), 12 tests built from the owner's real endpoints.
- ✅ Engine contract (Claude, `phase-1/engine-api`): `ice_core::engine`: `Command`, `EngineSnapshot`, `Envelope { revision, event }`,
  `EngineEvent`, `Notice`, and the `Engine` trait. **Codex and Gemini: build against `ice_core::engine::fake::FakeEngine`** (owner's devices,
  no Windows calls) until `phase-1/engine` merges. Enable `ice-core/ts` in the app to export TypeScript bindings (camelCase, `type`-tagged).
- ✅ Settings commands (Claude, `phase-1/config-commands`): `SetTogglePair`, `SetBinding`, `SetFavourites`, `SetDevicePrefs`, `SetKnobGestures`, `SetDefaultRoles` (+ removes), validated in `Config::apply_setting`, saved through the safe save path; `EngineSnapshot.settings` + `SettingsChanged`; `Notice::SettingRejected`. The tray can set up the owner's toggle pair without editing TOML.
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

**Status (2026-11-22):** feature-complete in `main`; the exit criteria (3 days of daily use, QA A–G) are the owner's to run.

- [x] `ice-win::audio`: endpoint model (id, stable id, names, flow, state, form factor, hardware key, container, default per role, virtual badge)
- [x] **`DeviceRef` + resolver (D-16):** endpoint ID → stable ID → hardware + logical-endpoint match among **Active** endpoints;
      re-link persisted after a confirmed switch; ambiguous → picker (`ChooseDevice`); missing → `NotConnected`. Tested with the
      owner's real endpoints, incl. the new-USB-port case (QA B3/B4 ✅)
- [x] **Toggle-pair hotkey** (Speakers ↔ Arctis) with one-click setup from the tray ("Toggle with the current device…")
- [x] Ghost endpoints hidden by default
- [ ] "Show disconnected devices" toggle (AudioSwitcher v1 parity)
- [x] `ice-win::audio`: device change notifications → engine events (sliding, capped debounce)
- [x] `ice-win::policy`: switch default. `Default` = Console + Multimedia; `Both` = all three roles. Defaults are re-read after
      every switch; partial failures are reported per role
- [x] `ice-win::hotkeys`: `RegisterHotKey` on a dedicated message-loop thread (thread-level, no hook); conflicts reported as notices
- [x] `ice-core::config`: TOML schema v1, validation, atomic save with lock retries, **event-driven hot reload**, invalid edits keep
      the last valid config + notice, external edits adopted before automatic saves
- [ ] Explicit portable mode (`portable.flag` next to the exe)
- [x] Settings commands so the tray configures pairs, hotkeys, favourites, per-device spatial/alias/hidden and knob gestures without TOML
- [x] Tray (Codex): left-click quick-switch menu, right-click full menu, tooltip with the current device, notices and the device picker.
      (Scroll-on-icon was cut by D-05.F05.)
- [x] Favourites + cycle hotkey; hide devices; aliases
- [x] Knob gestures (F31): Ctrl+turn = next device per gesture, Shift+turn = 1% steps, Alt+turn = mic volume
- [x] Autostart (launch at sign-in), `--start-minimized`, single instance (Codex)
- [x] Logging: file logs in `%LOCALAPPDATA%\IceAudioManager\logs`, `--verbose` (Codex)
- [x] Engine start-up recovery: tray stays up and retries if the audio service isn't ready; event stream resubscribes (Codex, SHELL-01/02)
- [x] CI: fmt, clippy, tests, UI check/test/build on `windows-latest`. Unsigned **portable zip** artifact for private use
- [x] Manual QA checklist: [docs/qa/manual-checklist.md](qa/manual-checklist.md) with a run log
- [ ] **Exit:** owner's 3 days of daily use + QA sections A–G through the tray (B5 sleep/resume still to run)

**Exit criteria:** owner uses it as the daily switcher for 3 days with no crash. Budgets are met.
No hook DLLs or LL hooks are visible in Process Explorer. **The Speakers ↔ Arctis hotkey keeps working after unplugging and
replugging the dongle in a different USB port, power-cycling the headset, restarting SteelSeries GG, and sleep/resume.**

---

## Phase 2: Notch v1 (volume, switch, mic)

**Goal:** add the notch to the private daily-use build (D-06). **Features:** F08–F14.

**Status (2026-11-23):** Phase 2 is feature-complete and visually QA'd in `main`.

- [x] Notch window: **exactly one** transparent window, moved between monitors (`overlay_win::move_top_center`), DPI-aware
- [x] **Performance profiles (D-15.1):** `Instant` (WebView warm, `TrySuspend` when idle) · `Balanced` (created on demand, suspends) ·
      `Lite` (no notch WebView; native dot + tray only). Saved in config (schema v2), chosen from the tray
- [x] **Native idle dot/pill** drawn when the WebView is suspended/released: live volume %, mute, mic volume and theme colours (Gemini)
- [x] **Gaming mode:** automatic while a fullscreen app covers the notch's monitor (`FullscreenWatcher`, event-driven); the WebView
      is released and a generation guard stops events from recreating it. A manual toggle isn't built
- [x] Activity scheduler: priorities, durations, merge rules, snapshot + delta with monotonic revisions. Lives in `app/ui`
      (`scheduler.ts`, tested) rather than `ice-core`
- [x] Activities: **Volume**, **Mic volume**, **Device switch** (with "re-linked" caption and the Atmos badge only after Windows
      confirms), **Plug/unplug**, **Mic mute**
- [x] Interactions: hover-intent expand, scroll = volume, Shift+scroll = switch device, middle-click = mute (`NotchApp.svelte`)
- [x] Placement: **pinned monitor is the default** (first run pins the primary), saved by display path, chosen from the tray;
      falls back while it's disconnected
- [x] Idle state: **default = small visible mini pill** (settled over dot after light/dark desktop comparison at 100%/150%, Q-10);
      `dot` and `hidden` remain options
- [x] Springs: custom `(k, c, m)` module; Snappy / Smooth / Bouncy (~8% overshoot) / Off (instant); identical at 75/144/240 Hz (tested)
- [x] **Spatial sound on switch (D-17):** per-device preference applied after a confirmed switch (read-before-write, read-back),
      re-applied after a re-link, Atmos badge, `SpatialUnavailable` notice; set from the tray's "Spatial sound" submenu
- [x] Visual QA across all activities and the media card on 144 Hz and 75 Hz monitors at 100%/150% scaling; WCAG AA contrast check
      of text over light and dark desktops ([manual-checklist.md](qa/manual-checklist.md))

**Exit criteria:** hardware volume keys, notch scroll and hotkeys all animate correctly. No focus stealing. Hidden over fullscreen
games on the pinned monitor. Budgets met in every profile.

---

## Phase 3: Media (now playing)

**Features:** F15–F17 and F18 (without the mixer). **Primary QA target: Spotify desktop** (Q-09).

**Progress (2026-11-22, Claude, `phase-3/media-core`):** engine-independent groundwork is merged.
- `ice_core::media`: `NowPlaying`, `PlaybackStatus`, `ArtInfo`, dominant-colour extraction and `readable_accent`
  (WCAG AA ≥ 4.5:1 on the Ice surface; colourless art falls back to the brand accent). Unit-tested.
- `ice_win::media`: event-driven `MediaWatcher` (session/properties/playback/timeline events, no polling), transport
  commands, and an art pipeline using Windows' own imaging API: decode, Fant downscale to ≤ 128 px, content-hash id
  for `iceart://`, accent, PNG.
- Measured with Spotify: session read ~5 ms, timeline event → read ~4 ms, art pipeline 34–60 ms (debug build).
- Gemini's [media review](reviews/media-review.md) (MED-01–MED-10) is fixed (`phase-3/media-engine`; resolution table in the review).
- Real engine (`phase-3/media-engine`, Claude): `ice_win::media_service` runs GSMTC on its own `ice-media` MTA thread, coalesces
  signals (dirty mask + 60 ms settle), bounds every WinRT wait (500 ms per call, 2 s per art read), and feeds
  `snapshot.media` / `MediaChanged` / `Command::Media`; `Engine::art` serves the last 8 PNGs. Measured with Spotify: first
  `MediaChanged` ~100 ms after engine start, art read ~31 ms.
- Media contract (`phase-3/media-api`, Claude): `EngineSnapshot.media`, `EngineEvent::MediaChanged`, `Command::Media { action }`,
  `Engine::art(id)`; the fake engine simulates three Spotify tracks with art so UI work can start.

- [x] `ice-win::media`: GSMTC manager, current-session tracking, timeline, playback status, transport commands (event-driven)
- [x] Media contract in the engine API, with a fake implementation
- [x] Real engine wiring: dedicated media worker thread, coalesced signals, bounded WinRT waits (MED fixes)
- [x] Scoped `iceart://<id>` handler in the app shell: engine PNGs only, no paths/queries/other methods, in the production CSP (Codex)
- [x] Album art: downscale (≤ 128 px) + content hash (incl. size) + readable accent on every built-in dark surface; size/time
      bounds, EXIF orientation, straight alpha
- [x] Activities: **Track change** and compact **Now playing** idle state (Gemini, against the fake engine)
- [x] Expanded notch: media card (art, title, artist, scrubber, prev/play/next) (Gemini, against the fake engine). Real-engine data
      now comes from the real engine too
- [ ] Visualizer: `IAudioMeterInformation` peaks at ≤ 30 Hz, subscribed **only while visible**, rendered as 4–6 bars with fast-attack/slow-decay springs
  - [x] Engine (Claude, `phase-3/meter-api` + `phase-3/meter-engine`): `Engine::meter()` returns a `MeterSubscription` of
        `MeterFrame { peak, channels }` (0.0–1.0, `engine.ts`). Levels bypass the snapshot and revisions. Only visible changes are
        sent, silence once. The `ice-meter` thread runs only while a subscription is alive and re-binds on default-device changes.
        The fake engine sends stereo levels while its track plays. The thread stopping after the last drop is tested on real Windows.
        Measured (release, music playing, 10 s): **0.013% CPU**, 206 frames (≈ 21/s after change filtering)
  - [ ] Shell (Codex): hold a subscription only while the notch is visible *and* the visualizer is shown. **Drop it on hide,
        `TrySuspend`, WebView release and gaming mode**, without relying on the UI to ask. Forward frames off the main thread
        to the notch as `audio://meter/v1`, and add a Tauri command for the UI to turn it on/off
  - [ ] Notch (Gemini): 4–6 bars from `channels` (fall back to `peak`), fast-attack/slow-decay springs, no animation loop
        once frames stop and the bars have settled
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
- [x] Documented theme API (`docs/theming.md`): every token, its default, an example theme
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

## Phase 6: Hardening and first public release

**Goal:** harden and publish the app after Phases 1–5 and private daily use. Start Store onboarding, public signing, and distribution in this phase.

- [ ] Microsoft Store individual developer registration (free, with identity verification) and a separate MSIX packaging step.
      Tauri's standard Windows installers are NSIS and WiX. Smoke-test Core Audio, GSMTC, tray recovery,
      settings paths, and `StartupTask` in the packaged build
- [ ] Audit the release tree and Git history before publishing the source under MIT (Q-02)
- [ ] Signing per D-09: Store MSIX first; **SignPath** for GitHub artifacts once eligible (its terms require an OSS project with a prior release).
      Sign the exe inside the portable zip as well as the NSIS installer
- [ ] Packaging per D-10: MSIX, per-user NSIS installer (no admin), portable zip
- [ ] Release workflow: tag → CI build → sign → SHA256SUMS (after signing) → GitHub artifact attestation (public repo) → draft release
- [ ] Optional **notify-only** update check: off by default, never downloads (D-11). About page shows the version.
      Trace the release build to verify zero app-initiated network requests by default
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

## Cleanup backlog

Known debt under [code-practices §11](code-practices.md#11-code-hygiene-duplication-cleanliness-bloat). Each item is fixed in its
own `refactor:` PR by the path's owner, or sooner if a feature needs to grow the file. Sizes from 2026-11-23, excluding tests.

| Item | Owner | Size / issue | Suggested split |
|---|---|---|---|
| `crates/ice-win/src/native_dot.rs` | Gemini | 1706 lines (limit ~600) | window/message loop · GDI drawing · layout/geometry · theme colours |
| `app/src-tauri/src/main.rs` | Codex | 1414 lines; `#[allow(too_many_lines)]` at the setup fn | app setup · notch window lifecycle · commands · profiles/gaming mode |
| `crates/ice-win/src/audio_engine.rs` | Claude | 1087 lines | audio thread loop · switch/confirm/re-link · volume/mic · spatial |
| `app/ui/src/notch/NotchApp.svelte` | Gemini | 687 lines (component limit ~300) | interactions (scroll/hover/click) and IPC sync out of the component |
| `app/ui/src/notch/activities/ExpandedNotch.svelte` | Gemini | 485 lines | Shrinks with D-18 Part B (pickers removed). Split the media card and the device row |
| `app/ui/src/lib/notch/scheduler.ts` | Gemini | 604 lines, at the limit | Watch it. Split the merge rules out if it grows |
| Repeated `#[allow(clippy::cast_possible_truncation, cast_sign_loss)]` (native_dot ×5, audio_engine, media) | Claude + Gemini | Same `f64` → integer conversion repeated without a reason | One checked conversion helper in `ice-win`; remaining suppressions become `#[expect(…, reason)]` |

## Dependency map

```
Phase 0 ─► Phase 1 ─► Phase 2 ─► Phase 3 ─► Phase 4 ─► Phase 5 ─► Phase 6 (first public release)
```
