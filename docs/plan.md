# Ice Audio Manager: Implementation Plan

> **Status: APPROVED.** D-01 to D-14 signed off by the owner on 2026-11-22 (see the Decision Log in
> [planning-discussion.md](planning-discussion.md)). Stack: **Rust + Tauri 2 + Svelte 5, TOML + CSS** (D-01 to D-04).
> D-15 (memory policy), D-16 (device identity) and D-17 (spatial sound) were also signed off on 2026-11-22.
> The owner deferred the first public release to [Phase 6](#phase-6-hardening-and-first-public-release).
> Coding rules live in [code-practices.md](code-practices.md). **How each module works: [docs/modules/](modules/README.md)** (read first, update with every PR).

## Guiding constraints

1. **Lightweight:** budgets below are hard gates. The Phase 0–3 measurements stand for now; the full benchmark suite runs
   again once, in the **Phase 6 testing pass** (owner decision, 2026-11-23). Meanwhile the design rules in code-practices §2
   keep the app within them.
2. **Customizable:** every visual and behavioural knob lives in config/theme, not in code.
3. **Trustworthy:** no LL hooks, no admin, no packers, no silent network, open source at release, signed.
4. **Windows 11 only** (D-08): use native APIs directly. No cross-platform abstractions "just in case".

### Performance budgets (gates)

| Metric | Target | Hard limit | How measured |
|---|---|---|---|
| Idle CPU (no activity) | ~0.0% | < 0.2% avg over 60 s | Release build, 60 s sample after settling |
| Idle RAM, default profile (whole process tree, Settings closed) | **≤ 100 MB** (owner, Q-06) | **150 MB** (owner allowance, D-15) | Private Bytes summed over the host + all app-owned WebView2 processes (dedicated user-data folder, `GetProcessInfos`). Since 2026-11-24 the default (`Instant`) keeps the WebView awake at idle |
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
  - [ ] By hand: compare with Spotify on a few genres (EDM, hip-hop, acoustic, a quiet podcast) on both monitors
  - [ ] **D-26 (open in planning-discussion.md): real bass/treble spectrum bands and bar dynamics.** The meter today gives
        only a peak per channel (no spectrum), so the bars follow beats, not pitch. D-26 compares an upgraded multi-envelope
        wave shaper against WASAPI loopback capture + a 14-band IIR biquad filter bank or FFT on the meter thread while
        visible, paired with per-band adaptive gain, frequency-graded springs, and wave coupling
- [ ] Secondary QA: Chromium/Edge/Firefox media sessions, **including YouTube Music** (the website, and the app installed
      from Chrome/Edge). Owner, 2026-11-23: Spotify and YouTube Music are the supported music services (see Backlog)
- [x] Expanded card empty state when nothing plays: a clear hint (e.g. "Nothing playing. Start Spotify or YouTube Music"),
      not a blank card (NotchBox reference, D-20 review). *`ExpandedEmptyCard`: "Nothing playing", the current device, and
      labelled Spotify / YouTube Music launch buttons*
- [ ] **D-18 (accepted 2026-11-23): owner-picked notch devices, preferences out of the notch**
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

**Exit criteria:** track changes animate within 300 ms of the event. Visualizer costs < 1% CPU while visible and 0% while hidden.

---

## Notch interaction and placement (D-19, D-20; accepted 2026-11-23)

Runs between Phase 3 and Phase 4. Item 1 is a bug fix that comes first. D-21 Part B (C0) applies: every resource cap stays.

**D-19 · the idle pill (do first):**
- [x] **Bug fix** (Gemini, `native_dot.rs`): the idle "MIC OFF" label is unreachable while volume is known
      (`if let Some(v) = state.volume … else if state.mic_muted`), and `MediaChanged` is ignored
- [ ] `ice_core::IdleContent { glyph, label, tone }`: one rule for both pills, unit-tested (Claude). The design owner sets
      the states: mic muted, speaker muted, media playing, resting
- [ ] Hit-testable native pill (Gemini): drop `WS_EX_TRANSPARENT`, `WM_MOUSEACTIVATE → MA_NOACTIVATE`.
  - Click-first: native pressed state within one frame. Hover-to-open is opt-in. Middle-click = mute.
    *Done in `native_dot/input.rs`: pressed paints in 0.35 ms p50; the pill reports `PillInput::{Click, MiddleClick, HoverOpen}`
    through `set_input_handler` and the shell wires them (Codex). Still open: the by-hand focus check (QA F4,
    `cargo test -p ice-win real_click -- --ignored`) and the wheel.*
  - The wheel ships only after testing Windows' "scroll inactive windows" setting on and off.
- [x] Shell wiring (Codex): pill click wakes and expands the notch; in Lite, a click opens the tray quick-switch menu.
      *Done 2026-11-24 (Claude): `app/src-tauri/src/show_notch.rs` (`wire_native_pill`); middle click = mute; hover-to-open
      (opt-in) does the same as a click; nothing happens over a fullscreen game; `tray::open_quick_menu` extracted*
- [x] "Show notch" hotkey, unbound by default (Claude: binding + engine; Codex: tray/Settings entry). *Done 2026-11-24
      (Claude): config `show_notch_hotkey` (schema v7, unique with every other chord), `Command::SetShowNotchHotkey`,
      `EngineEvent::ShowNotch` → `show_notch::show` (toggles the pinned notch; Lite → quick-switch menu); Settings →
      Shortcuts → Notch*
- [ ] By hand: hotkey in Instant / Balanced (asleep) / Lite / over a game; pill click and middle click in Balanced and Lite
- [x] Native pill renderer (Gemini): spike GDI glyphs into the existing DIB first, then Direct2D/DirectWrite only if needed.
      Its RAM budget in every idle profile where it shows (Lite, Balanced < 30/40 MB; Instant no longer suspends) is checked in the
      Phase 6 testing pass. Size and
      radius come from the same tokens as the WebView pill. *GDI (`native_dot/glyphs.rs`, zero D2D/DWrite DLLs) passed
      every 60 s idle gate: Lite `1.69 MB` WS, Balanced `3.82 MB` WS, Instant suspended `8.58 MB` WS; matched `116×28`
      (`r=14`) and `36×18` (`r=9`) tokens. Wiring `IdleContent` follows Claude's merge.*
- [ ] Side-by-side visual QA of the native and WebView idle pills (all states, light/dark, 100 %/150 %, both monitors)

**D-20 · placement (owner ruling: both methods, drag & snap by default):**
- [ ] `ice_core::anchor_rect(anchor, work_area, size)` and six anchors (top/bottom × left/centre/right), unit-tested with
      taskbar positions and DPI (Claude). It replaces both copies of the top-centre math (`overlay_win`, `native_dot`)
- [ ] Config (Claude, schema bump + migration test): `notch.anchor` (default top-centre) and `notch.placement`
      = `drag_snap` (default) | `fixed`, plus the engine commands to set them
- [ ] Drag & snap with our own pointer capture (`SetCapture` + `SetWindowPos(SWP_NOACTIVATE)`), never the system move loop.
      WebView notch: Codex. Native pill: Gemini.
  - Click vs drag via `SM_CXDRAG`/`SM_CYDRAG`. Esc cancels.
  - Anchor slots shown on every monitor while dragging. Release springs to the nearest slot; reduce motion = instant.
  - Dropping on another monitor re-pins it there.
- [ ] **Release gate:** during and after a drag the foreground app keeps focus and keyboard input, on both hosts, across
      150 %/100 % monitors
- [ ] Bottom anchors (Gemini): the notch sits above the taskbar (work area) and expands upwards (flip `.pill-slot`,
      clip-path, glow origin)
- [ ] Anchor list in the tray and Settings → Notch, which works in both placement modes (Codex)
- [ ] R1 "attached vs floating" style (Gemini, Phase 4 Appearance): asymmetric corners via an SVG path and the native SDF
- [ ] R2 optional modules (Phase 4 Activities): a disabled module runs no timer, subscription or IPC

**D-21.1 (accepted 2026-11-23):**
- [x] L1 · Idle linger in config, per profile (default 15 s, validated range), replacing `IDLE_SUSPEND_LINGER_MS`
      (Claude: config + schema migration; Gemini: the UI reads it from the snapshot). *Done 2026-11-24 (Claude): one value,
      `idle_linger_secs` (schema v6, 5–600 s, out-of-range edits rejected), because only Balanced suspends now (Instant stays
      awake, Lite has no WebView); `Command::SetIdleLinger`; the scheduler reads `settings.idleLingerSecs`; Settings →
      General shows "Sleep the notch after" (5 s – 5 min) while Balanced is selected*
- [ ] L2 · Event-driven hit-testing for the WebView notch, replacing the cursor poll (Codex).
  - Spike on the real host first: moving edges during spring morphs, mixed DPI, glow/shadow not clipped.
  - If it can't follow the springs cleanly, narrow when the poll runs instead.
- [x] L3 · Session pin on the expanded notch: "keep open until unpinned" (Gemini UI, Codex shell). It shows a pin mark and
      clears on session lock or sleep. Gaming mode still releases it
  - [x] Pin button with an on state in the notch side rail (`toggle_expand`); gaming mode resets `expanded`
  - [x] Clear the pin on session lock and sleep (Claude, 2026-11-24): `ice_win::session_watch` (hidden top-level window,
        `WTSRegisterSessionNotification` + `WM_POWERBROADCAST`, no polling) → `app/src-tauri/src/session.rs`
  - [ ] By hand: pin, lock (Win+L) and unlock → collapsed; pin, sleep and wake → collapsed (QA checklist)
- [ ] L6 · Live cost readout in Settings → General: RAM and CPU of the process tree, sampled only while that page is open
      (Codex, via `ice_win::proc_metrics`)

---

## Music services, lyrics, look and ambient motion (D-22, D-23, D-25)

Items marked *(needs Settings)* wait for the Phase 4 Settings window. The rest can start now.

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
- Full design spec: [`docs/lyrics.md`](lyrics.md) (bottom pill/card extension, active-word emphasis, Options A & B + Option C constraints, offline Japanese Romaji, Settings → Privacy & Storage max cache size, and phased implementation path).
- [ ] Settings → Privacy: lists every network feature, its destination and the data sent, with one "all network features
      off" switch (Codex)
- [ ] Rust LRCLIB client (Claude), written from LRCLIB's docs (code-practices §1.8):
  - allowlisted host; title/artist/album/duration only;
  - a one-track lookup first, then optional automatic lookups;
  - results bound to the requesting track, late responses dropped;
  - "no match" preferred over uncertain timing;
  - local cache, cleared when turned off.
- [ ] Lyrics sub-view in the expanded card (Gemini): synced lines interpolated from GSMTC position, ticking only while
      visible and playing; V6 styling (bright current line, fading neighbours)

**D-23 · Look of the expanded notch:**
- [ ] 2–3 colour art palette and average luminance in `ice_core::media`, next to today's accent (Claude)
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
- [ ] **144 Hz measurement gate before shipping:** 60 s on the 144 Hz monitor (process-tree CPU, GPU engine use, private
      working set vs. static backdrop). If process-tree CPU rises by `> +0.1 %` over static, fall back to a ~30 fps
      JavaScript `transform` driver (`C4`)

---

## Notch refresh (owner request, 2026-11-24)

The owner found the expanded card cluttered, the shadow broken, the visualizer bars ugly, and `Instant` sometimes falling back
to the static native pill. Done by Claude; module docs: [app-ui-notch](modules/app-ui-notch.md), [app-shell](modules/app-shell.md).

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
- [ ] By hand: skip tracks in Spotify and YouTube Music; the title should change within ~0.3 s (`--verbose` logs "now playing")
- [x] **"Now playing pill" setting** (owner, 2026-11-24; config schema v5 `appearance.nowPlaying`, Settings → Notch,
      tray submenu): **Brief, then small** (default: the 2.4 s pop-up, then the idle pill) · **While playing** (stays as the
      cover-and-controls pill while media plays; pausing shrinks it; Balanced doesn't idle-suspend while it shows) · **Off**
      (no pop-up and no notch wake on a new track; the idle pill just shows the title)
- [ ] By hand: each of the three modes with Spotify (skip, pause, resume) in Instant and Balanced
- [x] **Waveform scrubber replaces the now-playing pill's buttons** (owner: hovering expands the notch before the buttons
      can be reached). `TrackActivity` shows a 14-bar live waveform (`VisualizerBars variant="wide"`, `spreadLevels`) lit in
      the art accent up to the played position, dim after it; a calm resting waveform while paused or without a meter.
      Position is interpolated locally and re-rendered only when the next bar lights up (~every duration / 14)
- [ ] By hand: the waveform on light/dark desktops, a live stream (no duration = all lit) and pause/resume
- [ ] Visual QA of the new card on both monitors at 100 % / 150 %, light/dark desktops, all 4 themes (owner)
- [ ] Phase 6: measure Instant idle with the WebView awake (expected ≈ 77 MB private WS from Phase 0.2, inside the ≤ 100 MB
      target) and the cursor poll's idle CPU, which now runs for as long as Instant shows the notch. D-21.1 L2
      (event-driven hit-testing) would remove that poll

---

## Phase 4: Settings & customization

**Features:** F19–F23.

- [x] Settings window: created on demand, destroyed on close; Mica backdrop; light/dark (Claude, `phase-4/settings-window`;
      [module doc](modules/app-ui-settings.md))
- [ ] Pages: General (incl. **Performance profile**) · Devices (aliases, hide, favourites, icons) · Hotkeys (recorder via `RegisterHotKey` test-bind) ·
      Notch (geometry, idle mode, pinned monitor) · Activities (toggles, durations, priorities) · Appearance (theme, accent, springs) · About (version)
  - [x] General (launch at sign-in, performance profile, default roles, show disconnected) · Devices (notch order, star, alias,
        spatial, hide/show) · Shortcuts (toggle pairs, device shortcuts, recorder, knob gestures) · Notch (idle style, display map) ·
        Appearance (theme and animation with live previews) · About (version, trust promises)
  - [ ] Activities page (needs config for module switches, D-20 R2 / D-22), notch anchor (D-20), favourites cycle hotkey
        (config has `cycle_hotkey` but no command yet), live cost readout (D-21.1 L6)
    - [x] Activities page with the D-22 music service switches; Shortcuts → Notch has the "show notch" hotkey (2026-11-24)
    - [ ] Still open: D-20 R2 module switches, notch anchor, favourites cycle hotkey command, live cost readout
- [ ] Theme system: bundled `Ice` (default, portfolio palette), `Midnight`, `Fluent`, `Minimal`. User themes = CSS files in
      `%APPDATA%\IceAudioManager\themes\` overriding documented `--ice-*` tokens, **validated before applying**, with **live preview**
- [ ] Strict CSP: a theme with remote `url()`/`@import` produces zero network requests (verified)
- [x] Documented theme API (`docs/theming.md`): every token, its default, an example theme
- [ ] Import/export config + theme bundle (`.zip`)
- [ ] Every setting applies live without restart (true for every setting the window has today)

**Exit criteria:** a user can build a theme without reading source code. Closing Settings returns memory to the idle budget
(verified after 10× open/close, in the Phase 6 testing pass).

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

**Testing pass (owner decision, 2026-11-23).** Performance, memory and security testing happens **once, here**, not in
every feature phase. Phases 1–5 focus on features and UI/UX.
- [ ] **Performance and RAM:**
  - re-run the Phase 0.2 benchmark recipe across every profile and state (Instant, Balanced, Lite, gaming, Settings 10×
    open/close), with the plan's budget table as the pass/fail line;
  - latency: event → feedback, wake, switch;
  - an 8-hour soak (WebView working set, handles, threads).
- [ ] **Visualizer, native pill and drag:** CPU while visible and 0 % while hidden; native pill RAM in every idle profile;
      pin/awake costs if D-21.1 adds them; release exe, installer and UI bundle sizes.
- [ ] **Security:**
  - code-practices §1 audit (no hooks, no injection, no admin, no silent network, guarded undocumented APIs);
  - `cargo audit` + `npm audit`;
  - CSP and the `iceart://` handler;
  - Tauri capabilities per window (least privilege);
  - config and theme input validation (fuzz the TOML and CSS validators).
- [ ] Fix what the pass finds before the release items below.

- [ ] Microsoft Store individual developer registration (free, with identity verification) and a separate MSIX packaging step.
      Tauri's standard Windows installers are NSIS and WiX. Smoke-test Core Audio, GSMTC, tray recovery,
      settings paths, and `StartupTask` in the packaged build
- [x] Audit the Git history for secrets: the repo has been public since 2025-02-21. Checked 2026-11-23: `.env` only ever held
      `PYTHONPATH`, and a credential-pattern scan of the full history (keys, tokens, passwords, private keys) found
      nothing. Re-run before the first release
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
- **Spotify and YouTube Music** (owner, 2026-11-23: both will be supported):
  - *Level 1, works today without extra code:* both play through Windows' media sessions (GSMTC): Spotify desktop
    (the primary QA target, Q-09), and YouTube Music in Chrome/Edge or as an installed app. Covered by the Phase 3
    secondary QA. No network, no sign-in.
  - *Level 2, a future decision:* richer features (like/save, queue, lyrics, playlists) need each service's web API,
    which means network calls and a sign-in. §1.5 allows only the opt-in update check today, so this needs a new decision
    before any code: opt-in per service, OAuth tokens in Windows Credential Manager, no background traffic, a documented
    privacy note. YouTube Music has no official public API. Unofficial ones risk breaking its terms, so Level 2 may be
    Spotify-only.
  - **D-22 (closed 2026-11-23):** Level 1 + a "Music services" widget and opt-in LRCLIB lyrics are planned (section "Music
    services, lyrics and look"). YouTube Music playback inside the app was rejected. Spotify "Level 2" (like, queue,
    playlists) still needs its own decision under the new §1.5.
- **Ideas from BitChord / Echo-Music** (owner references, 2026-11-23; undecided, each needs an F-ID or decision before code):
  - *Sleep timer:* pause the current media session after N minutes or at the end of the track (GSMTC; no network).
  - *Output format readout:* the default device's sample rate and bit depth in the expanded notch (the endpoint's mix
    format). Pairs with "sample-rate quick toggles" above.
  - *Word-synced lyrics animation:* only if D-22 adopts lyrics.
  - *Discord Rich Presence:* local IPC to Discord, but it shares what you play, so it would be opt-in with a privacy note.
  - Not for us: offline downloads, ad-free playback via private APIs, analytics/crash reporting (§1.5).
- **Ideas from Auralys** (Android, closed source; owner screenshot 2026-11-23; undecided):
  - *Share card:* render the current track (art, title, artist, theme colours) to a PNG on the clipboard or in a file.
    Local only, no upload.
  - *UI style as a setting* (Editorial vs Glass), separate from colour themes. Related to D-23.
  - Not for us: "listen together" rooms. They need a server and real-time sync, outside D-05 and §1.5.

## Cleanup backlog

Known debt under [code-practices §11](code-practices.md#11-code-hygiene-duplication-cleanliness-bloat). Each item is fixed in its
own `refactor:` PR by the path's owner, or sooner if a feature needs to grow the file. Sizes from 2026-11-23, excluding tests.

| Item | Owner | Size / issue | Suggested split |
|---|---|---|---|
| `crates/ice-win/src/native_dot.rs` | Gemini | 1706 lines (limit ~600) | window/message loop · GDI drawing · layout/geometry · theme colours |
| `app/src-tauri/src/main.rs` | Codex | 1414 lines; `#[allow(too_many_lines)]` at the setup fn | app setup · notch window lifecycle · commands · profiles/gaming mode |
| `app/ui/src/lib/notch/scheduler.ts` | Gemini | 621 lines (2026-11-24), over the limit | Split the merge rules and the media/track handling out (the now-playing view already lives in `nowPlaying.ts`) |
| Repeated `#[allow(clippy::cast_possible_truncation, cast_sign_loss)]` (native_dot ×5, audio_engine, media) | Claude + Gemini | Same `f64` → integer conversion repeated without a reason | One checked conversion helper in `ice-win`; remaining suppressions become `#[expect(…, reason)]` |

## Dependency map

```
Phase 0 ─► Phase 1 ─► Phase 2 ─► Phase 3 ─► Phase 4 ─► Phase 5 ─► Phase 6 (first public release)
```
