# Plan archive: finished work

> Moved out of [plan.md](../plan.md) on 2026-11-25 to keep the live plan short (owner request). Text is verbatim; each
> section keeps its original heading, and items keep the sub-heading they were listed under. Newer finished items are
> appended below their section as they are ticked.

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
- ✅ 0.3 audio + spatial spike and ✅ 0.4 media spike (Claude): see [docs/spikes/phase0-audio-media.md](../spikes/phase0-audio-media.md).
  ID churn recorded by the owner: power-cycle and same-port replug keep the IDs; a **different port** gives new endpoint,
  stable and container IDs (fixed in the resolver, QA B3/B4 ✅). Sleep/resume (QA B5) still to run.
- ✅ 0.7 (Claude): `legacy-python-v0` tag, Python removed, `.env` untracked, MIT `LICENSE`, Rust workspace (`ice-core`, `ice-win`, spikes),
  CI (fmt, clippy `-D warnings`, test, release build). The Tauri `app/` scaffold and typed IPC codegen came with Codex's `app-shell`.
- ✅ 0.2 overlay spike + measurements (Gemini): `spikes/overlay` + [docs/spikes/phase0-overlay.md](../spikes/phase0-overlay.md).
  60 s release numbers: native dot `2.20 MB` Private Bytes (`0.000%` CPU, `0.02 ms` p95); warm/hidden WebView2 `149.84–150.39 MB` Private Bytes (`77.10–76.79 MB` Private WS); `TrySuspend` `148.01 MB` Private Bytes / **`6.44 MB` Private WS** (`27.24 ms` p95 resume); cold WebView2 spawn **`715.44 ms` p95**; 0 leaked PIDs on destroy. The strict D-15 gate pointed at Option C; **D-15.1 then chose A** (WebView notch + `TrySuspend`, budgets in private working set), with C as the fallback.
- ✅ D-16 resolver (Claude): `ice_core::resolve` (`DeviceRef`, `resolve`, `toggle`), 12 tests built from the owner's real endpoints.
- ✅ Engine contract (Claude, `phase-1/engine-api`): `ice_core::engine`: `Command`, `EngineSnapshot`, `Envelope { revision, event }`,
  `EngineEvent`, `Notice`, and the `Engine` trait. **Codex and Gemini: build against `ice_core::engine::fake::FakeEngine`** (owner's devices,
  no Windows calls) until `phase-1/engine` merges. Enable `ice-core/ts` in the app to export TypeScript bindings (camelCase, `type`-tagged).
- ✅ Settings commands (Claude, `phase-1/config-commands`): `SetTogglePair`, `SetBinding`, `SetFavourites`, `SetDevicePrefs`, `SetKnobGestures`, `SetDefaultRoles` (+ removes), validated in `Config::apply_setting`, saved through the safe save path; `EngineSnapshot.settings` + `SettingsChanged`; `Notice::SettingRejected`. The tray can set up the owner's toggle pair without editing TOML.
- ✅ 0.5 fullscreen spike (Codex): see [dual-monitor results](../spikes/phase0-fullscreen.md). Phase 2 still needs to cover
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

**Status (2026-11-22):** feature-complete in `main`; the exit criteria (3 days of daily use, QA A–G) are the owner's to run.
- [x] `ice-win::audio`: endpoint model (id, stable id, names, flow, state, form factor, hardware key, container, default per role, virtual badge)
- [x] **`DeviceRef` + resolver (D-16):** endpoint ID → stable ID → hardware + logical-endpoint match among **Active** endpoints;
      re-link persisted after a confirmed switch; ambiguous → picker (`ChooseDevice`); missing → `NotConnected`. Tested with the
      owner's real endpoints, incl. the new-USB-port case (QA B3/B4 ✅)
- [x] **Toggle-pair hotkey** (Speakers ↔ Arctis) with one-click setup from the tray ("Toggle with the current device…")
- [x] Ghost endpoints hidden by default
- [x] "Show disconnected devices" toggle (AudioSwitcher v1 parity) *(verified 2026-11-25: Settings → General)*
  - [x] Engine (Claude, `phase-1/show-disconnected`): `show_disconnected` (config schema v3, v2 migration tested),
        `Command::SetShowDisconnected`, `settings.showDisconnected`, and `EngineSnapshot.disconnected` / `DevicesChanged.disconnected`.
        Only *Unplugged* endpoints are listed, never ghosts or disabled ones. Hidden devices are skipped. They're never switch
        targets (`SwitchTo` → `NotConnected`). The fake engine lists one unplugged front-panel jack
  - [x] Tray (Codex, finished by Claude): a "Show disconnected devices" check item, and greyed, non-clickable entries for `snapshot.disconnected`
- [x] `ice-win::audio`: device change notifications → engine events (sliding, capped debounce)
- [x] `ice-win::policy`: switch default. `Default` = Console + Multimedia; `Both` = all three roles. Defaults are re-read after
      every switch; partial failures are reported per role
- [x] `ice-win::hotkeys`: `RegisterHotKey` on a dedicated message-loop thread (thread-level, no hook); conflicts reported as notices
- [x] `ice-core::config`: TOML schema v1, validation, atomic save with lock retries, **event-driven hot reload**, invalid edits keep
      the last valid config + notice, external edits adopted before automatic saves
- [x] Settings commands so the tray configures pairs, hotkeys, favourites, per-device spatial/alias/hidden and knob gestures without TOML
- [x] Tray (Codex): left-click quick-switch menu, right-click full menu, tooltip with the current device, notices and the device picker.
      (Scroll-on-icon was cut by D-05.F05.)
- [x] Favourites + cycle hotkey; hide devices; aliases
- [x] Knob gestures (F31): Ctrl+turn = next device per gesture, Shift+turn = 1% steps, Alt+turn = mic volume
- [x] Autostart (launch at sign-in), `--start-minimized`, single instance (Codex)
- [x] Logging: file logs in `%LOCALAPPDATA%\IceAudioManager\logs`, `--verbose` (Codex)
- [x] Engine start-up recovery: tray stays up and retries if the audio service isn't ready; event stream resubscribes (Codex, SHELL-01/02)
- [x] CI: fmt, clippy, tests, UI check/test/build on `windows-latest`. Unsigned **portable zip** artifact for private use
- [x] Manual QA checklist: [docs/qa/manual-checklist.md](../qa/manual-checklist.md) with a run log

## Phase 2: Notch v1 (volume, switch, mic)

**Status (2026-11-23):** Phase 2 is feature-complete and visually QA'd in `main`.
- [x] Notch window: **exactly one** transparent window, moved between monitors (`overlay_win::move_top_center`), DPI-aware
- [x] **Performance profiles (D-15.1):** `Instant` (WebView warm **and awake at idle**: it never falls back to the native pill,
      owner 2026-11-24, see "Notch refresh") · `Balanced` (created on demand, `TrySuspend` when idle) ·
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
      of text over light and dark desktops ([manual-checklist.md](../qa/manual-checklist.md))

## Phase 3: Media (now playing)

**Progress (2026-11-22, Claude, `phase-3/media-core`):** engine-independent groundwork is merged.
- [x] `ice-win::media`: GSMTC manager, current-session tracking, timeline, playback status, transport commands (event-driven)
- [x] Media contract in the engine API, with a fake implementation
- [x] Real engine wiring: dedicated media worker thread, coalesced signals, bounded WinRT waits (MED fixes)
- [x] Scoped `iceart://<id>` handler in the app shell: engine PNGs only, no paths/queries/other methods, in the production CSP (Codex)
- [x] Album art: downscale (≤ 128 px) + content hash (incl. size) + readable accent on every built-in dark surface; size/time
      bounds, EXIF orientation, straight alpha
- [x] Activities: **Track change** and compact **Now playing** idle state (Gemini, against the fake engine)
- [x] Expanded notch: media card (art, title, artist, scrubber, prev/play/next) (Gemini, against the fake engine). Real-engine data
      now comes from the real engine too
- [x] Expanded card empty state when nothing plays: a clear hint (e.g. "Nothing playing. Start Spotify or YouTube Music"),
      not a blank card (NotchBox reference, D-20 review). *`ExpandedEmptyCard`: "Nothing playing", the current device, and
      labelled Spotify / YouTube Music launch buttons*
- [x] **D-18 (accepted 2026-11-23): owner-picked notch devices, preferences out of the notch**
  - [x] Tray (Codex, finished by Claude): a **"Show in notch"** check item per playback device (= favourite, via `SetFavourites`, in order), and the
        submenus **Theme**, **Animation** and **Idle notch** next to Performance profile / Pinned monitor. These three are now
        saved (`[appearance]`, config schema v4, `Command::SetTheme/SetPreset/SetIdleStyle`) and survive restarts. The WebView
        and the native dot follow them, with the theme mapped by the shell so Phase 4 user themes can plug in. Editing
        favourites keeps a disconnected favourite in its place (`SavedDevice.savedId`)
  - [x] Notch (Gemini): the device row lists `settings.favouritesPlayback` in order. The current default is shown first if it isn't a
        favourite. With no favourites: current device + the other side of the toggle pair and no picker button (owner, 2026-11-23: favourites are set from the tray's "Show in notch"), never
        "all devices". Alias-aware chips wide enough to read, at most 4. Remove the theme, spring, idle-style and
        monitor pickers from the expanded card. *Owner, 2026-11-24: no "More…" button or in-notch picker either; Settings →
        Devices is the only place to choose and order them (the notch's `set_notch_devices` command was removed)*
  - [x] Settings → Devices: reorderable list (Phase 4; Claude, `phase-4/settings-window`)

**Progress (2026-11-22, Claude, `phase-3/media-core`):** engine-independent groundwork is merged.
- `ice_core::media`: `NowPlaying`, `PlaybackStatus`, `ArtInfo`, dominant-colour extraction and `readable_accent`
  (WCAG AA ≥ 4.5:1 on the Ice surface; colourless art falls back to the brand accent). Unit-tested.
- `ice_win::media`: event-driven `MediaWatcher` (session/properties/playback/timeline events, no polling), transport
  commands, and an art pipeline using Windows' own imaging API: decode, Fant downscale to ≤ 128 px, content-hash id
  for `iceart://`, accent, PNG.
- Measured with Spotify: session read ~5 ms, timeline event → read ~4 ms, art pipeline 34–60 ms (debug build).
- Gemini's [media review](../reviews/media-review.md) (MED-01–MED-10) is fixed (`phase-3/media-engine`; resolution table in the review).
- Real engine (`phase-3/media-engine`, Claude): `ice_win::media_service` runs GSMTC on its own `ice-media` MTA thread, coalesces
  signals (dirty mask + 60 ms settle), bounds every WinRT wait (500 ms per call, 2 s per art read), and feeds
  `snapshot.media` / `MediaChanged` / `Command::Media`; `Engine::art` serves the last 8 PNGs. Measured with Spotify: first
  `MediaChanged` ~100 ms after engine start, art read ~31 ms.
- Media contract (`phase-3/media-api`, Claude): `EngineSnapshot.media`, `EngineEvent::MediaChanged`, `Command::Media { action }`,
  `Engine::art(id)`; the fake engine simulates three Spotify tracks with art so UI work can start.

- [x] Visualizer: `IAudioMeterInformation` peaks at ≤ 30 Hz, subscribed **only while visible**, rendered as 4–6 bars with fast-attack/slow-decay springs
      (CPU while visible/hidden is re-measured in the Phase 6 testing pass)
  - [x] Engine (Claude, `phase-3/meter-api` + `phase-3/meter-engine`): `Engine::meter()` returns a `MeterSubscription` of
        `MeterFrame { peak, channels }` (0.0–1.0, `engine.ts`). Levels bypass the snapshot and revisions. Only visible changes are
        sent, silence once. The `ice-meter` thread runs only while a subscription is alive and re-binds on default-device changes.
        The fake engine sends stereo levels while its track plays. The thread stopping after the last drop is tested on real Windows.
        Measured (release, music playing, 10 s): **0.013% CPU**, 206 frames (≈ 21/s after change filtering)
  - [x] Shell (Codex): the notch calls `set_visualizer({ active })` to request or release `audio://meter/v1` frames.
        The shell forwards off the main thread and drops the subscription on hide, `TrySuspend`, WebView release or crash,
        gaming mode, and Lite. A new document must request frames again.
  - [x] Notch (Gemini): 4–6 bars from `channels` (fall back to `peak`), fast-attack/slow-decay springs, no animation loop
        once frames stop and the bars have settled
  - [x] Bars look fix (Claude, 2026-11-24): `clip-path` instead of `scaleY` (round caps stay round), whole-pixel width/gap
  - [x] Beat-driven dynamics (Claude, 2026-11-24; owner: "not dynamic enough, only twitching; lower when silent; feel the
        beat go up and down"). `lib/notch/beat.ts` `BeatShaper` replaces `BarShaper`: per-channel dB tracking of the
        track's body level and recent peak, so bars measure how far a frame sits *above* the body (beats go high, bars
        fall between hits, loud and quiet tracks alike); each detected beat throws a new pattern; centre bars swing with
        the body, outer bars flick on fast rises. The bank animates 14 independent bars (wide waveform = all of them,
        5-bar variants pick 5). Faster fall (decay ×2.4, was ×0.65), resting height 0.1 (was 0.18), peaks brighten.
        Tested with simulated 120 BPM kicks on loud and quiet tracks (`beat.test.ts`)

**D-26 (accepted 2026-11-24): real bass/treble spectrum bands and bar dynamics (`A2 + B1 + B2 + B4a`, `A1` fallback):**
  - [x] Trust & privacy rule: add §1.9 audio-stream rule in `docs/code-practices.md`, config `visualizer_spectrum` (schema v10, default `true`), `Command::SetVisualizerSpectrum`, and Settings → Privacy disclosure + meter-only fallback switch
  - [x] `ice-core`: pure 4th-order 14-band log-spaced `f32` IIR biquad `SpectrumAnalyzer` (`45 Hz – 14 kHz`, sample-rate-aware RBJ coefficients, `reset()`), separate optional `MeterFrame.bands` participating in `MeterFrame::differs` + TS codegen (`engine.ts`)
  - [x] `ice-win`: non-blocking shared-mode WASAPI render loopback (`AUDCLNT_STREAMFLAGS_LOOPBACK`, `≥ 100 ms` buffer) drained once per existing 30 Hz `ice-meter` tick, automatic `IAudioMeterInformation` fallback (and DRM/silent-loopback detection), filter/band state discarded on endpoint rebind or fallback
  - [x] Notch UI (`beat.ts`, `visualizer.ts`, `VisualizerBars.svelte`): `B1` per-band AGC + pink tilt gated on audible band energy, low-band (`0–2`) beat onset accent, `A1` multi-envelope travelling-wave fallback when `bands` is absent, `B2` frequency-graded `(k, c, m)` springs, `B4a` symmetric centre-bass 5-bar badge + L→R `bass → treble` 14-bar wide scrubber, clean settle/stop on silence

## Notch interaction and placement (D-19, D-20; accepted 2026-11-23)

**D-19 · the idle pill (do first):**
- [x] **Bug fix** (Gemini, `native_dot.rs`): the idle "MIC OFF" label is unreachable while volume is known
      (`if let Some(v) = state.volume … else if state.mic_muted`), and `MediaChanged` is ignored
- [x] Hit-testable native pill (Gemini): drop `WS_EX_TRANSPARENT`, `WM_MOUSEACTIVATE → MA_NOACTIVATE`.
  - Click-first: native pressed state within one frame. Hover-to-open is opt-in. Middle-click = mute.
    *Done in `native_dot/input.rs`: pressed paints in 0.35 ms p50; the pill reports `PillInput::{Click, MiddleClick, HoverOpen}`
    through `set_input_handler` and the shell wires them (Codex).*
  - The wheel (Claude, 2026-11-25): `PillInput::Wheel { notches, shift }` (touchpad deltas add up to whole notches),
    shell: ±4 % per notch, Shift switches device, like the WebView notch (`WHEEL_VOLUME_STEP` twin). Checked through
    the running app's shell and engine: two notches 40 % → 48 %, Shift + one notch switched the output. With Windows'
    "Scroll inactive windows when hovering" off, the wheel goes to the focused app and the pill does nothing (harmless)
- [x] Shell wiring (Codex): pill click wakes and expands the notch; in Lite, a click opens the tray quick-switch menu.
      *Done 2026-11-24 (Claude): `app/src-tauri/src/show_notch.rs` (`wire_native_pill`); middle click = mute; hover-to-open
      (opt-in) does the same as a click; nothing happens over a fullscreen game; `tray::open_quick_menu` extracted*
- [x] "Show notch" hotkey, unbound by default (Claude: binding + engine; Codex: tray/Settings entry). *Done 2026-11-24
      (Claude): config `show_notch_hotkey` (schema v7, unique with every other chord), `Command::SetShowNotchHotkey`,
      `EngineEvent::ShowNotch` → `show_notch::show` (toggles the pinned notch; Lite → quick-switch menu); Settings →
      Shortcuts → Notch*
- [x] Native pill renderer (Gemini): spike GDI glyphs into the existing DIB first, then Direct2D/DirectWrite only if needed.
      Its RAM budget in every idle profile where it shows (Lite, Balanced < 30/40 MB; Instant no longer suspends) is checked in the
      Phase 6 testing pass. Size and
      radius come from the same tokens as the WebView pill. *GDI (`native_dot/glyphs.rs`, zero D2D/DWrite DLLs) passed
      every 60 s idle gate: Lite `1.69 MB` WS, Balanced `3.82 MB` WS, Instant suspended `8.58 MB` WS; matched `116×28`
      (`r=14`) and `36×18` (`r=9`) tokens. Wiring `IdleContent` follows Claude's merge.*

**D-20 · placement (owner ruling: both methods, drag & snap by default):** *Resumed 2026-11-25 (owner: "continue …
- [x] Placement rule and anchors (Claude, 2026-11-25): `ice_core::placement` (`NotchAnchor`, `anchor_origin`, tests with
      taskbars on every side, a second monitor at 150 %, a narrow screen). Each anchor fixes the notch's **centre** in the
      monitor's **work area**; side anchors reserve the full 648 px canvas so the widest card stays on screen; both hosts
      centre their own box on it (`overlay_win::notch_origin`), which replaced both copies of the top-centre math and
      stops covering a top taskbar. **Top three anchors only**: bottom anchors are added with the upward-growing layout
      below, so no setting exists that does nothing. Checked on the running app: WebView `(0,0)–(648,292)` top left,
      `(1272,0)–(1920,292)` top right after a hot edit; native pill centred at 324 / 1596, the same centres
- [x] Config `notch_anchor` (schema v15 + migration test), `Command::SetNotchAnchor`. `notch.placement` (`drag_snap` |
      `fixed`) is added with drag & snap, not before
- [x] Anchor list in the tray ("Notch position": monitors, then the anchors) and Settings → Notch → Display ("Position")
- [x] R1 "attached vs floating" style: **already done** as Edge Nook (flush, ears) vs Floating (`appearance.notchStyle`,
      schema v11), on both hosts
- [x] Flexible markless drag & drop with our own pointer capture (`SetCapture` + `SetWindowPos(SWP_NOACTIVATE)`), never the system move loop.
      WebView notch (`start_notch_drag`, `drag.ts`, `controller.svelte.ts`): Gemini; Native pill (`native_dot/input.rs`, `drag.rs`): Claude & Gemini.
  - [x] Click vs drag via `SM_CXDRAG`/`SM_CYDRAG` (native) + `DRAG_THRESHOLD_PX` (UI). Esc cancels.
  - [x] Continuous horizontal placement (`notch_x_ratio` in schema v17) anywhere between Left and Right without visual slot/ghost marks, clamped to monitor bounds (`NOTCH_CANVAS_W`), with silent 14 px magnet at Left/Centre/Right and compositor-only settle + accent rim flash (`placed_seq` / `flash_placed`). Reduce motion = instant.
  - [x] Dropping on another monitor re-pins it there; Settings → Notch & tray **"Drag & place"** (`NotchPlacement`, schema v16).
- [x] Bottom anchors for **both Floating and Edge Nook** (`BottomLeft`/`BottomCenter`/`BottomRight`): Floating sits `6 px` above the taskbar (`CANVAS_BOTTOM_OVERHANG`); Edge Nook docks pixel-flush on top of the taskbar with inverted concave ears (`sd_nook_notch` mirrored + SVG `buildNookFillPath(..., bottom = true)`) and expands upwards.
- [x] Drag & drop effects (owner, 2026-11-26): a held notch lifts into a full capsule and lands with an impact; jelly
      landing graduated to Settings → Notch → Display; landing ripple and carry glow stay Experimental

**D-21.1 (accepted 2026-11-23):**
- [x] L1 · Idle linger in config, per profile (default 15 s, validated range), replacing `IDLE_SUSPEND_LINGER_MS`
      (Claude: config + schema migration; Gemini: the UI reads it from the snapshot). *Done 2026-11-24 (Claude): one value,
      `idle_linger_secs` (schema v6, 5–600 s, out-of-range edits rejected), because only Balanced suspends now (Instant stays
      awake, Lite has no WebView); `Command::SetIdleLinger`; the scheduler reads `settings.idleLingerSecs`; Settings →
      General shows "Sleep the notch after" (5 s – 5 min) while Balanced is selected*

- [x] L3 · Session pin on the expanded notch: "keep open until unpinned" (Gemini UI, Codex shell). It shows a pin mark and
      clears on session lock or sleep. Gaming mode still releases it
  - [x] Pin button with an on state in the notch side rail (`toggle_expand`); gaming mode resets `expanded`
  - [x] Clear the pin on session lock and sleep (Claude, 2026-11-24): `ice_win::session_watch` (hidden top-level window,
        `WTSRegisterSessionNotification` + `WM_POWERBROADCAST`, no polling) → `app/src-tauri/src/session.rs`

## Music services, lyrics, look and ambient motion (D-22, D-23, D-25)

**D-22 · Music services widget (owner: enabled by default):**
- [x] "Music services" widget in the expanded notch and the empty "nothing playing" state (Gemini UI).
  - Quick-launch buttons (`MusicServicesWidget.svelte`, `backdrop.ts#resolveMusicServices`) for Spotify and YouTube Music: icons
    in the side rail while media plays, labelled buttons in the empty state (`open_music_service`).
- [x] Shell side (Codex): find and launch the installed Spotify / YouTube Music web apps; fall back to the URL.
- [x] One switch per service in Settings → Activities (Spotify, YouTube Music), both on by default; config fields with defaults
      (Claude) *(the switches need Settings; the config can land first)*. *Done 2026-11-24 (Claude): `[music_services]`
      (schema v7, `ice_core::media::MusicServices`), `Command::SetMusicService`, new Settings → Activities page;
      `resolveMusicServices` reads `settings.musicServices`*

**D-22 · Lyrics (opt-in, LRCLIB + local `.lrc` + Romaji + Storage cache) *(needs Settings)*:**
- [x] Settings → Privacy: lists every network feature, its destination and the data sent, with one "all network features
      off" switch (Codex). *Built by Gemini, 2026-11-24 (`PrivacyPage.svelte`)*
- [x] Rust LRCLIB client (Claude), written from LRCLIB's docs (code-practices §1.8). *Built by Gemini, 2026-11-24
      (`ice-win/src/lyrics/lrclib.rs`):*
  - allowlisted host; title/artist/album/duration only;
  - a one-track lookup first, then optional automatic lookups;
  - results bound to the requesting track, late responses dropped;
  - "no match" preferred over uncertain timing;
  - local cache, cleared when turned off.

**D-23 · Look of the expanded notch:**
- [x] 2–3 colour art palette and average luminance in `ice_core::media`, next to today's accent (Claude, 2026-11-25:
      `art_palette` → `ArtInfo.palette` (≤ 4 real colours) + `luminance_pct`; the wash uses only the cover's colours)
- [x] Backdrops per theme (Gemini): V2 gradient wash by default; V1 blurred art as an opt-in (`--ice-art-backdrop`) with a
      luminance-clamped scrim (`--ice-scrim`). Ice/Midnight/Fluent/Minimal each get one treatment; reads `art.palette` +
      `art.luminance` when present and derives a 2nd wash stop + luminance from `art.accent` in the meantime. New tokens in `docs/theming.md`
- [x] V3 translucent controls and V4 cleanliness pass on the 384×178 card, keeping D-18's device row visible. Before/after
      screenshots verified at `100%` and `150%` across light/dark desktops and all 4 themes (Gemini). *Superseded by the
      2026-11-24 relayout below*

**D-25 · Ambient motion on the expanded card (accepted 2026-11-24):**
- [x] Move `.stage`'s `filter: drop-shadow(...)` (`NotchApp.svelte`) off the animated subtree onto a static sibling `.pill-shadow`
      element so drifting layers and springs don't re-filter the stage every frame (Gemini)
- [x] **A1 + B1:** Split `.art-wash` (`NotchBackdrop.svelte`) into 2–3 oversized soft radial-gradient layers (no `filter`,
      sized to the `392×164` expanded card) drifting on compositor-only CSS `transform` (`translate3d` / `scale`) with
      coprime periods (`22 s` / `1.41×` / `1.73×`), keeping the contrast scrim anchored still and the animation phase unkeyed
      from `art.id` so track changes, pins, and resumes never snap the colours (Gemini)
- [x] **C1 + C2 lifecycle:** animate only on the expanded card while visible and media is playing (`resolveAmbientMotionState`:
      `animation-play-state: paused` when paused so colours freeze in place; `animation: none` when collapsed, hidden,
      suspended, under reduce motion / preset `Off`, or in `Minimal` / `--ice-ambient-period: 0s`) (Gemini)
- [x] **D1 + D3 + C3 setting and token:** add `--ice-ambient-period` to `base.css` and `docs/theming.md` (`0s` in `Minimal`,
      `28s` in `Midnight`, `22s` in `Ice`/`Fluent`), `appearance.movingBackdrop` (schema v8, default `true`),
      `Command::SetMovingBackdrop`, and the `"Moving background"` switch in Settings → Appearance (Gemini)
- [x] **Lava-lamp refresh** (Claude, 2026-11-25, owner: "do the 1 to 5 improvements"):
  1. palette cross-fades between songs (registered `<color>` orb colours, 1.4 s) instead of snapping;
  2. closed orbital loops (each orb circles its own spot, never ping-pongs);
  3. calmer: 30 s loops (38 s Midnight), tempo nudges only 0.85–1.2× (was up to 1.65× on 9 s half-cycles);
  4. bass breath: the wash swells ≤ 6 % on low-band kicks and exhales over ~0.4 s (one compositor transform, only
     while playing, visible, not reduced motion);
  5. readability: orb colours capped by luminance (not HSL lightness) + `lighten` blending. Measured on a bright
     cover over 8 frames: worst muted-text contrast 1.24:1 → 3.23:1, primary text ≈ 7:1

- [x] Lyrics sub-view in the expanded card (Gemini): synced lines interpolated from GSMTC position, ticking only while
      visible and playing; V6 styling (bright current line, fading neighbours)
  - [x] 2026-11-24 (Claude): two-row music pill; line changes scroll (pill and card); generated Romaji separated from
        paired subtitles and no longer cached; Romaji particles and word breaks; Romaji follows the singing; long lines
        shrink or slide instead of losing their last characters (lyrics.md §8)

## Notch refresh (owner request, 2026-11-24)

- [x] **Relayout** (`ExpandedNotch`, 392 × 164, or 392 × 144 with nothing playing): main column = media card (58 px art, title,
      artist, transport, scrubber) + volume row + device chips; a **side rail** (`NotchSideRail`) holds Pin, Mic, Spotify /
      YouTube Music (while media plays) and Settings, so the player stays clean
- [x] **Volume bar is a slider:** click or drag (`set_volume` → `Command::SetVolume`, throttled to 40 ms), arrow keys; the −/+ buttons are gone
- [x] **"More…" and the in-notch device picker removed** (Settings → Devices owns favourites and order)
- [x] **Shadow fix:** the 420 × 190 canvas left 6 px under the expanded card for an 8 px offset + 18 px blur shadow, so the
      window edge cut it. Canvas is now 440 × 210 (twin constants in `geometry.ts` and `main.rs`) and `--ice-drop-shadow` is
      `0 6px 16px`; the rule is in `theming.md`
- [x] **Instant never falls back to the native pill:** `Placement::suspends_on_idle` (Balanced only) gates `request_idle_suspend`
      in the shell and the UI; switching to Instant wakes a suspended notch; switching away re-arms the idle linger
- [x] **Visualizer bars** (see Phase 3)
- [x] New tokens/icons: `--ice-radius-md` (12 px), `settings` icon
- [x] **Song-change delay fix** (owner report: the notch and native pill kept the previous song for ~5 s). Spotify can fire
      "properties changed" before the new title is readable, and only properties events re-read the title. `media_service`
      now re-reads the track at +300 ms / +1 s / +2.5 s after each track read (emits only on change, never at idle), treats a
      timeline duration jump as a new track, and gives the first read's cover 400 ms so a slow cover can't hold back the
      title (the re-reads pick it up)
- [x] **"Now playing pill" setting** (owner, 2026-11-24; config schema v5 `appearance.nowPlaying`, Settings → Notch,
      tray submenu): **Brief, then small** (default: the 2.4 s pop-up, then the idle pill) · **While playing** (stays as the
      cover-and-controls pill while media plays; pausing shrinks it; Balanced doesn't idle-suspend while it shows) · **Off**
      (no pop-up and no notch wake on a new track; the idle pill just shows the title)
- [x] **Waveform scrubber replaces the now-playing pill's buttons** (owner: hovering expands the notch before the buttons
      can be reached). `TrackActivity` shows a 14-bar live waveform (`VisualizerBars variant="wide"`, `spreadLevels`) lit in
      the art accent up to the played position, dim after it; a calm resting waveform while paused or without a meter.
      Position is interpolated locally and re-rendered only when the next bar lights up (~every duration / 14)

## Edge Nook track (priority from 2026-11-25)

**Done:**
- [x] Lyrics in the Edge column (Claude, 2026-11-25; [lyrics.md §8 follow-up](../lyrics.md#8-fixed-2026-11-24-the-last-characters-of-a-line-were-cut-off)):
      the `fitLine` CSS had been dropped by Svelte (no fades, long centred lines lost their first characters) and the
      active word's lift was clipped at the row edge. Fixed for pill, card and nook. In the nook, the current line now
      **wraps** (balanced, ≤ 3 rows) instead of shrinking or sliding, and the rows are placed around its measured height

**Open, fixes (Edge-specific, do first):**
- [x] **Hover intent at the top edge** (Claude, 2026-11-26): `NotchStyleStrategy.hoverIntentMs` (Floating 120 ms, Edge
      Nook 220 ms), restarted while the pointer sweeps past faster than 0.5 px/ms (`drag.ts` `isPassingThrough`). The
      owner's feel check stays open in plan.md
- [x] **Startup hit rect** (Claude, 2026-11-26): `startup_hit_rect` in `main.rs` reads `native_dot::idle_body_box`
      (one source, honours bottom anchors) instead of the hand-copied `116 + 16 × 30`
- [x] **Hit rect vs ears** (Claude, 2026-11-26): `nookStyleStrategy.computeHitRect` covers the body only, so clicks
      under the transparent concave ears reach the app below

**Open, polish:**
- [x] Top bar: the device name repeats the active device chip below. *Done in the Edge Nook card refresh (2026-11-25): the
      header names the player (`mediaSource`) and the spatial badge*

**Open discussion, for later (owner, 2026-11-25):**
- [x] **D-27 · Hibernate mode, first cut** (Claude, 2026-11-25, owner asked for it with Claude's recommendations): plain
      black notch + red mic-muted dot, no pop-ups, click-through, hidden from screen captures while on (switchable),
      Settings → Hibernate and the tray's "Hibernate notch"
- [x] Hibernate button (moon) in the Edge Nook card's toolbar (Claude, 2026-11-25); also unpins the card. The Floating
      side rail is full (5 buttons max), so it waits for the Floating pass
- [x] Edge Nook card refresh (Claude, 2026-11-25, owner: "modernize/improve the UI/UX"): header names the player
      (Spotify / YouTube Music / Chrome…) instead of repeating the device; launchers and toggles split, toggles in one
      capsule; 64 px cover with the scrubber beside it and the transport centred under it; fading lyrics divider; no
      duplicate device name in the empty card. Checked in a headless-Edge harness (dark/light, JA/EN, empty)
- [x] Hibernate follow-up (Claude, 2026-11-25, owner: "separate setting, never start hibernated, the static notch
      doesn't show, let me hide it completely"): **root cause** of the missing plain notch: `--ice-hibernate-*` sat in
      the Minimal theme's block, so on every other theme the fill was transparent (now `:root`, plus a test that no
      theme declares a token `:root` lacks). Hibernate is session state (every start is awake, a hot reload keeps the
      live value); schema v13 `hibernateLook` = plain notch / hide the notch; own Settings page "Hibernate"; Balanced
      hands over to the native pill 1 s after hibernating; hibernating from the tray or Settings unpins the card too.
      Checked on the running app (scratch `APPDATA`): starts awake from a hibernated file; plain notch drawn (WebView on
      Instant, native pill on Balanced); hidden look shows nothing on both; waking restores the idle pill and its input
- [x] **D-28 · Volume vs track progress** ([entry](decisions-closed.md#d-28--volume-vs-track-progress-in-the-notch-what-the-user-sees-and-touches-added-2026-11-25-owner-request-neutral-entry-under-04)):
      built 2026-11-25 on the owner's ruling (A2 + B1 + B2 + C1, chip fallback amendment). Both cards: song block on
      top (the only accent bar, 4 px, times at both ends), output strip below (device chips + small grey volume, or a
      volume chip that opens the slider when the chips need the room); a click on the volume only steps ±2 %; the volume
      pop-up (WebView and native) leads with the %. Checked on the running app: two chips → inline slider; a click near
      the top of the rail at 40 % → 42 %
- [x] Lyrics out of sync (owner, 2026-11-25): **per-device lyrics delay** (schema v14 `[[device]] lyrics_delay_ms`,
      `Command::SetLyricsDelay`, Settings → Devices → a playback device → Lyrics delay). Auto = 200 ms on Bluetooth
      (`BTHENUM`/`BTHLE`, an estimate: Windows has no API for the link's delay), 0 elsewhere; −1000…+2000 ms. The lyric
      clock is the player's position minus the playing output's delay (`lyricClockMs`). Word timing stays estimated
      unless the LRC has word stamps

**Fixed (2026-11-25):**
- [x] Clicks right around the pill and the music pill were swallowed (Claude): the cursor poll scaled the hit rect with a
      stored monitor scale that can go stale (topology/DPI change, other monitor); it now uses the window's own size,
      re-applies click-through every tick and resets it when it stops; the UI resends the rect after a wake or failed send
- [x] **Root cause found and fixed** (Claude, 2026-11-25, owner: "the notch area still cannot be clicked, in pill mode
      and hibernate"): click-through set only `WS_EX_TRANSPARENT`, which Windows ignores for mouse routing on a
      non-layered window, so the whole 648 × 292 canvas blocked clicks (it never worked; the Phase 0.2 check only tested
      that the call returned OK). Now tao's `set_ignore_cursor_events` (`TRANSPARENT | LAYERED`, kept across tao's style
      rewrites) and `.focusable(false)`. Verified on the running app: three points beside the pill → window below
      (old build: notch)

## Phase 4: Settings & customization

**Features:** F19–F23.
- [x] Settings window: created on demand, destroyed on close; Mica backdrop; light/dark (Claude, `phase-4/settings-window`;
      [module doc](../modules/app-ui-settings.md))
- [x] Documented theme API (`docs/theming.md`): every token, its default, an example theme
- [ ] Pages: General (incl. **Performance profile**) · Devices (aliases, hide, favourites, icons) · Hotkeys (recorder via `RegisterHotKey` test-bind) ·
      Notch (geometry, idle mode, pinned monitor) · Activities (toggles, durations, priorities) · Appearance (theme, accent, springs) · About (version)
  - [x] General (launch at sign-in, performance profile, default roles, show disconnected) · Devices (notch order, star, alias,
        spatial, hide/show) · Shortcuts (toggle pairs, device shortcuts, recorder, knob gestures) · Notch (idle style, display map) ·
        Appearance (theme and animation with live previews) · About (version, trust promises)
    - [x] Activities page with the D-22 music service switches; Shortcuts → Notch has the "show notch" hotkey (2026-11-24)
    - [x] Notch → Display → Position (D-20 anchors, 2026-11-25); Notch → Hibernate page; Devices → Lyrics delay
    - [x] Experimental page (2026-11-25); on 2026-11-26 the owner graduated estimated word highlight (now Activities →
          Lyric highlight: Word by word, the default, or Row by row; schema v22), the wavy progress line (Appearance →
          Song progress) and jelly landing (Notch → Display) out of it

## Phase 6: Hardening and first public release

**Testing pass (owner decision, 2026-11-23).** Performance, memory and security testing happens **once, here**, not in
- [x] Audit the Git history for secrets: the repo has been public since 2025-02-21. Checked 2026-11-23: `.env` only ever held
      `PYTHONPATH`, and a credential-pattern scan of the full history (keys, tokens, passwords, private keys) found
      nothing. Re-run before the first release

## Backlog (unscheduled)

- **Spotify and YouTube Music**, D-22 (closed 2026-11-23): Level 1 + a "Music services" widget and opt-in LRCLIB lyrics
  are planned (section "Music services, lyrics and look"). YouTube Music playback inside the app was rejected. Spotify
  "Level 2" (like, queue, playlists) still needs its own decision under the new §1.5. *Built since; the open Level 2 note
  stays in plan.md.*
- **Ideas from BitChord / Echo-Music:** *Word-synced lyrics animation:* only if D-22 adopts lyrics. *Built with D-22
  (word highlight, then Word by word / Row by row on 2026-11-26).*
