# Manual QA checklist (hardware-dependent behaviour)

Automated tests cover pure logic (`cargo test`). Everything here depends on real devices, Windows, or timing, so a person runs it.
**Run the relevant sections before merging a branch that touches them, and the whole list before each phase closes.**

- **Who:** the owner, on the reference machine (below). Agents run the sections that don't need hands on hardware.
- **How to record:** copy the "Run log" template at the bottom into a new section and fill in ✅ / ❌ / n/a plus notes.
- **A ❌ blocks the phase exit.** Open a fix branch, and link it in the run log.

## Reference machine (owner)

Windows 11 25H2 (build 26200) · Ryzen 5 3600X · RTX 3050 · 16 GB · `DISPLAY1` 1920×1080 @ 144 Hz (primary), `DISPLAY2` 1920×1080 @ 75 Hz, 100% scale ·
**Headphones (Arctis Nova 7)** (USB dongle, Dolby Atmos for Headphones) · **Speakers (Realtek(R) Audio)** · ASUS VP249 / LG FULL HD (HDMI/DP audio) ·
Steam Streaming virtual devices · keyboard with a **volume knob** (press = mute) · Spotify desktop.

## Setup

From the repo folder, in **cmd or PowerShell** (not Git Bash). One command per line; the block has no comments, so it
pastes safely into cmd:

```
cargo run -p audio-spike -- make-config
cargo run -p audio-spike -- engine spikes/audio/out/config.toml 600
```

1. `make-config` writes your setup: Arctis ↔ Speakers on **Ctrl+Alt+F12**, Atmos on the Arctis, knob gestures on.
2. `engine … 600` prints every engine event for 10 minutes.

These only build the audio tools. The UI build (`npm --prefix app/ui ci`, then
`npm --prefix app/ui run build`, as two separate commands) is only needed to build the **whole** workspace.

Keep the `engine` window visible. "Expect" lines refer to what it prints. In later phases the tray and notch must show the same thing.

---

## A. Switching (D-05 F02, D-16)

| ID | Steps | Expect |
|---|---|---|
| A1 | Arctis is default. Press **Ctrl+Alt+F12** | `SWITCHED → Speakers (Realtek(R) Audio) (Default)`; sound moves to the speakers within ~150 ms |
| A2 | Press **Ctrl+Alt+F12** again | `SWITCHED → Headphones (Arctis Nova 7) (Default) [DolbyAtmosHeadphones]` |
| A3 | Press it 6× quickly | Every press alternates; no crash, no stuck state, the final device matches the last line |
| A4 | In Windows Sound settings, set **LG FULL HD** as default. Press Ctrl+Alt+F12 | Goes to the **Arctis** (primary) when a third device took over, not to the speakers |
| A5 | Turn the headset off, then press Ctrl+Alt+F12 from the speakers | `Notice … NotConnected { name: "Headphones (Arctis Nova 7)" }`; speakers stay default (nothing silently fails) |
| A6 | Set roles to `both` for the pair in `config.toml` (`roles = "both"` under `[[toggle_pair]]`), toggle | Communications default follows too (check Sound settings › More sound settings › Communications) |

## B. Reconnect and identity (D-16)

| ID | Steps | Expect |
|---|---|---|
| B1 | Headset power off → wait 10 s → on | `DeviceLeft` ×2 (render + mic) then `DeviceArrived` ×2; Windows restores the Arctis; volume line follows the current device |
| B2 | Unplug dongle → replug **same** port | As B1. Then Ctrl+Alt+F12 still toggles (endpoint ID unchanged) |
| B3 | Unplug dongle → plug into a **different** USB port | Arrival may show a new endpoint ID / "2-"/"3-" name. **Ctrl+Alt+F12 still reaches the headset** (`SWITCHED … [re-linked]` the first time), and `config.toml` now holds the new IDs |
| B4 | After B3, toggle again | Plain `SWITCHED` (no re-link): the saved binding was updated once, after a confirmed switch |
| B5 | Sleep the PC 30 s → wake | Engine still running; devices refresh; hotkeys still work |
| B6 | Turn a monitor off/on (HDMI/DP audio endpoint disappears/returns) | Arrival/departure for that monitor only; no effect on the Arctis ↔ Speakers pair |
| B7 | Restart the Windows audio service (admin PowerShell: `Restart-Service audiosrv -Force`) | Engine recovers or reports clearly; **known gap until Phase 6 crash-resilience** (record the behaviour) |

## C. Spatial sound (D-17)

| ID | Steps | Expect |
|---|---|---|
| C1 | Switch to the Arctis (A2) | `[DolbyAtmosHeadphones]` on the `SWITCHED` line; Windows quick settings › Spatial sound shows Dolby Atmos for Headphones |
| C2 | In Windows, set the Arctis spatial sound to **Off**, then switch away and back with Ctrl+Alt+F12 | Atmos is re-applied and confirmed (`[DolbyAtmosHeadphones]`) |
| C3 | Play music in Spotify on the Arctis and toggle twice | **No audible glitch** when Atmos is already on (read-before-write skips the write) |
| C4 | `cargo run -p audio-spike -- spatial` | Arctis active = Dolby Atmos; speakers = Off |
| C5 | (If available) a device without the Dolby licence: set its preference to `dolbyAtmosHeadphones` and switch to it | `Notice … SpatialUnavailable` with a reason; the switch itself still happened |

## D. Volume, knob and hotkeys (F31, D-07)

| ID | Steps | Expect |
|---|---|---|
| D1 | Turn the knob (no modifier) | Windows' own volume and flyout; engine prints `VolumeChanged` in 2% steps |
| D2 | **Shift** + turn | 1% steps |
| D3 | **Alt** + turn | Microphone volume changes (check Sound settings › Input); playback volume unchanged |
| D4 | **Ctrl** + one quick spin, then rest | Exactly **one** `SWITCHED` ~0.4 s after the knob stops, in the spin direction; never to the device already in use |
| D5 | Press the knob | Windows mute toggles; engine prints `muted: true/false` |
| D6 | Fast spin (Shift) | No lag build-up: volume follows the knob, final value matches Windows |
| D7 | Run `cargo run -p audio-spike -- hotkeys 60` in a second window **before** starting the engine | Engine prints `HotkeyConflict` notices for the chords the spike owns (on subscribe); everything else still works |

## E. Config hot reload (code-practices §7)

| ID | Steps | Expect |
|---|---|---|
| E1 | While the engine runs, add `alias = "Arctis"` on the line right after `spatial = "dolbyAtmosHeadphones"` (before `[device.device]`) and save | Within ~0.5 s the device list shows "Arctis"; no restart |
| E2 | Change the pair hotkey to `Ctrl+Alt+F11`, save | F11 now toggles; F12 no longer does |
| E3 | Save a broken file (e.g. delete a `]`) | One `ConfigInvalid` notice with line/column; the old config keeps working; **the file is not renamed** |
| E4 | Fix the file | Applies silently; no repeated notices |
| E5 | Stop the engine, break the file, start the engine | `ConfigReplaced` notice; the broken file is kept as `config.invalid-<time>.toml`; defaults in use |

## F. Trust rules (code-practices §1). Run before every phase exit

| ID | Check | Expect |
|---|---|---|
| F1 | `dumpbin /imports target\release\<exe> \| findstr /i "SetWindowsHookEx RegisterRawInputDevices"` (VS Developer prompt) | **No output** (no keyboard/mouse hooks) |
| F2 | Task Manager › Details › the app › right-click › Properties | Runs as the user (not elevated); no UAC prompt at start |
| F3 | Resource Monitor › Network (or `Get-NetTCPConnection -OwningProcess <pid>`) while idle 2 min | **No connections** (D-11) |
| F4 | Type in Notepad while switching/turning the knob | Focus never leaves Notepad (applies to the notch from Phase 2) |

## G. Footprint (plan.md budgets)

| ID | Check | Expect |
|---|---|---|
| G1 | Engine idle 60 s: Task Manager › Details, add columns "Memory (private working set)" and "CPU" | Engine-only: a few MB, 0% CPU. With the notch (Phase 2): D-15.1 budgets (≤ 15 MB suspended, ≤ 100 MB visible) |
| G2 | Hotkey → audible switch | Feels instant (< 150 ms) |

## H. Later phases (fill in as features land)

- **Tray (Codex, Phase 1):** left-click menu lists the same devices with checkmarks; picking one = A1 behaviour; Explorer restart (`taskkill /f /im explorer.exe & start explorer`) recreates the icon.
- **Notch (Phase 2):** pinned to the chosen monitor; hidden over fullscreen games on that monitor; springs identical on 144 Hz and 75 Hz; click-through outside the pill.
- **Media (Phase 3):** Spotify track change animates within 300 ms; art and controls correct.

---

## Run log template

```markdown
### Run: YYYY-MM-DD · <branch/commit> · <who>
| ID | Result | Notes |
|---|---|---|
| A1 | ✅ | |
| B3 | ❌ | describe what happened; link the fix branch |
```

## Run log

### Run: 2026-11-22 · `main` @ engine (`f24b9b5` → `abec8c4`) · owner + Claude
| ID | Result | Notes |
|---|---|---|
| A1 / A2 / A3 | ✅ | Toggle both ways; Atmos confirmed on every switch to the Arctis |
| A4 | ✅ (observed) | Windows moved Communications to the ASUS monitor on unplug; resolver logic covered by tests |
| B1 | ✅ | Both endpoints left and returned with the same IDs; Atmos kept |
| B3 | ❌ → fixed | Different USB port: Windows gave the Arctis new endpoint IDs **and a new container** (`8EC256AB…` → `F94FF25C…`, USB without serial), and reset spatial sound to Off. The resolver treated a container mismatch as "another headset" → `NotConnected`. Fixed: hardware key anchors, container only breaks ties. `audio-spike resolve` on the owner's config now gives `Relinked` for the pair, the Atmos preference and the favourite. **Re-run after `697fdc7`: ✅ owner confirmed it works** |
| B4 | ✅ | After the first re-linked switch the saved IDs are current (plain `SWITCHED` from then on) |
| B5 | not yet run | Sleep/wake |
| D1 / D2 / D5 | ✅ | Plain 2%, Shift 1%, knob press = mute |
| D4 | ✅ (after fix) | Re-switched to the current device with 2 favourites; fixed in `f24b9b5`; owner confirmed the fix works |
| A3 (repeat) | ✅ | 2nd run: ~10 rapid toggles 260 ms apart, all confirmed, Atmos each time |
| E1 / E3 / E4 | ✅ | Live test: alias applied, broken file → one notice, file not renamed |

### Run: 2026-11-22 · `phase-1/real-engine` @ `127e709` · Codex

Windows tray app with a separate test config. The native menu integration test exercised toggle-pair creation, the spatial preference, and the knob check item with the fake engine. The Windows smoke checks below used the real engine inside the tray process and inspected its event log.

| ID | Result | Notes |
|---|---|---|
| A1–A6 | Pending hands-on | Confirm audible switching, rapid alternation, the third-device fallback, the missing-headset notification, and Communications switching from the tray. |
| B1–B7 | Pending hands-on | Power-cycle, USB reconnect and re-link, sleep/wake, monitor changes, and audio-service restart still need the reference machine run. |
| D1–D6 | Pending hands-on | Confirm the physical knob modifiers, mute, and spin response against the tray volume readings. |
| D7 | App log verified | The audio spike held Ctrl+Alt+F12. The tray app's initial snapshot contained `HotkeyConflict` for the chord. Visual menu check pending. |
| E1 | App log verified | `DevicesChanged` showed the Arctis alias without restart in about 0.55 s, including the log polling interval. Visual menu check pending. |
| E2 | App log verified in part | `SettingsChanged` exposed Ctrl+Alt+F11 on the pair and Windows let the spike register the released F12 chord. An audible switch with F11 still needs a hands-on check. |
| E3 | App log and file verified | `ConfigInvalid` included line and column. The corrupted file stayed in place during reload. Visual menu check pending. |
| E4 | App log verified | Restoring the file emitted `NoticesChanged` with an empty list and a total of one `ConfigInvalid` notice for the broken save. Visual clearing check pending. |
| E5 | App log and file verified | Startup reported `ConfigReplaced` in the initial snapshot. The original contents were preserved in `config.invalid-*.toml`. Visual menu check pending. |

### Run: 2026-11-23 · `phase-2/idle-and-qa` · Gemini

#### 1. Q-10 Default Idle State Settlement (`MiniPill` vs `Dot` vs `Hidden`)
Compared `MiniPill` (`mini_pill` / `IdleStyle::MiniPill`), `Dot` (`dot` / `IdleStyle::Dot`), and `Hidden` (`hidden` / `IdleStyle::Hidden`) over dark (`#0E1116` portfolio dark, `#000000` pure black) and light (`#F6F7F9` portfolio light, `#FFFFFF` pure white) desktop backgrounds at `100%` (`1.0×`, `96 DPI`) and `150%` (`1.5×`, `144 DPI`) scaling:

| Criterion | `MiniPill` (**Settled Default**) | `Dot` (Option kept) | `Hidden` (Option kept) |
|---|---|---|---|
| **Geometry (`100%` / `150%`)** | WebView: `116×28` (`174×42` phys) device idle, `132×28` mic-muted, `196×28` now-playing (`216×28` mic-muted). Native (`TrySuspend`/`Lite`): `92×18` (`138×27` phys). | WebView: `36×18` (`54×27` phys), `42×18` mic-muted, `44×18` now-playing. Native: `38×16` (`57×24` phys). | WebView: `96×10` invisible top-center hover sensor (`opacity: 0`). Native: hidden (`SW_HIDE`) on idle; wakes on event. |
| **Dark desktop (`#0E1116` / `#000000`)** | Reads clearly as a Dynamic-Island top-center anchor framed by the 1 px `--ice-rim` specular highlight without crowding titlebars. | Blends into dark IDE/browser titlebars; looks like a tiny status speck rather than an interactive island. | Invisible until an audio/media event arrives or the pointer hovers top-center for `120 ms`. |
| **Light desktop (`#F6F7F9` / `#FFFFFF`)** | Composited obsidian-ice surface (`#24272B` over `#FFFFFF`) + drop shadow gives a clean, balanced capsule silhouette and `≥ 4.54:1` text contrast. | Small `36×18` capsule over white browser tab strips resembles a stray webcam/dead-pixel artifact. | Zero visual footprint when idle. |
| **At-a-glance status (`Arctis ↔ Speakers`)** | Shows active short device (`Arctis Nova 7` / `Speakers`) + live volume (`40%` / `Mute`) or compact now-playing track; native pill shows status mark + live volume `%` / `MUTE`. | Shows only the accent dot (or slashed-mic glyph when muted); requires hover or keypress to see active device or volume `%`. | None while idle. |
| **Hover-expand & wheel ergonomics** | Comfortable `116×28` (`92×18` native) top-center target for `120 ms` hover-intent expand, wheel volume (`±4%`), `Shift+wheel` device switch, and middle-click mute. | Narrow `36–38 px` hit target makes quick mouse-wheel volume adjustments fussy at `100%` (`1920×1080`). | Top-edge hover sensor (`96×10`) expands cleanly after `120 ms` hover-intent delay. |

- **Settlement:** **`MiniPill` (`mini_pill` / `IdleStyle::MiniPill`)** is the default idle state in both the WebView notch (`DEFAULT_IDLE_MODE = 'mini_pill'`) and the native wake/`Lite` window (`IdleStyle::default() == IdleStyle::MiniPill`). `Dot` and `Hidden` remain live-selectable options (`Pill` / `Dot` / `Hide` in the expanded card and `NativeDotHandle::set_idle_style`).

#### 2. Visual QA Pass: Activities, Media Card, Monitors (`144 Hz` & `75 Hz`), Scaling (`100%` & `150%`), and WCAG Contrast

| Area / Check | Result | Notes & Measurements |
|---|---|---|
| **Monitors (`DISPLAY1` 144 Hz & `DISPLAY2` 75 Hz)** | ✅ | Verified on `\\.\DISPLAY1` (`1920×1080 @ 144 Hz`, primary) and `\\.\DISPLAY2` (`1920×1080 @ 75 Hz`, secondary). `overlay_win::move_top_center` repositions in `1.07 ms` p50 / `1.82 ms` p95 with zero focus steal (`focus_preserved_on_monitor_move: true`). |
| **Spring cadence (`144 Hz` vs `75 Hz`, `100%` & `150%`)** | ✅ | `SpringGroup` (`Snappy`, `Smooth`, `Bouncy` `8.04%` overshoot, `Off` instant) step response and mid-flight retargets match within `< 1.5%` across `75 Hz`, `144 Hz`, and `240 Hz`. SVG `<rect rx ry>` (`vector-effect="non-scaling-stroke"`) and `clip-path: inset(... round ...)` keep circular capsule ends distortion-free at both `100%` (`1.0×`) and `150%` (`1.5×`). |
| **Idle states (`IdlePill.svelte` & `native_dot.rs`)** | ✅ (fixed) | Verified `MiniPill` (device + `%`, compact now-playing art + `title · artist` + play/pause glyph), `Dot`, and `Hidden`. **Fix applied:** `native_dot.rs` `PillMode::Idle` previously drew a plain `2.3 px` circle that only changed colour (`accent` → `danger`) on `mic_muted`. Replaced with a scaled SDF **slashed-mic glyph** (`mic_icon_cov`) in both `MiniPill` and `Dot` modes (satisfying D-12 non-color shape rule) plus live volume `%`/`MUTE` text in `MiniPill`. |
| **Volume & Mic Volume (`VolumeActivity.svelte`, `236×38`)** | ✅ | Repeat volume ticks merge without re-triggering entry micro-pulse; `0%`/`100%` boundary rubber-band nudge (`±2 px`) fires on repeat boundary ticks; `Alt+knob` (`micVolumeChanged`) renders the `Mic` channel pill + mic glyph; mute swaps to slashed glyph and `--ice-danger`. |
| **Device Switch (`SwitchActivity.svelte`, `296×44`)** | ✅ (fixed) | Outgoing device slides left/blurs out while confirmed target slides in; `Re-linked` caption renders on `relinked: true`; `Connected` caption renders when merged with `DeviceArrived` (`≤ 1500 ms`); `Atmos` badge appears only after confirmed `spatialApplied`. **Fix applied:** anchored `.prev-name` to `top: 0` so the outgoing device label never overlaps the two-line `Re-linked` / `Connected` sub-caption during the `220 ms` crossfade. |
| **Plug / Unplug (`PlugActivity.svelte`, `268×44`)** | ✅ | `deviceArrived` renders `--ice-ok` emerald rim + `Connected` badge; `deviceLeft` renders `--ice-warn` amber rim + `Removed` badge + `plug-out` icon. |
| **Mic Mute (`MicActivity.svelte`, `204×38`)** | ✅ | Renders `Mic Muted` (`--ice-danger`) / `Mic Live` (`--ice-ok`) with one entrance pulse in animated presets, then settles to the static muted-mic glyph badge on idle (`0%` idle GPU). |
| **Track Change (`TrackActivity.svelte`, `296×48`)** | ✅ | Fixed left `32×32` `AlbumArtTile` (`iceart://fake-art-0..2` PNGs with `art.accent` tile fallback) cross-scales (`0.88 → 1.0`), center title/artist slides vertically with blur crossfade, right transport controls (`previous`, `playPause`, `next`) stay completely stationary outside `{#key activity.trackSeq}`, and ambient radial glow crossfades (`400 ms`) to `art.accent`. |
| **Expanded Media Card (`ExpandedNotch.svelte`, `384×178`)** | ✅ (fixed) | Renders `34×34` art tile, title, artist, inline spatial tag, transport buttons (`previous`, `playPause`, `next`), `Pin`/`Pinned`, `positionMs`/`durationMs` scrubber (`0:52 / 3:31`), volume bar + `-/+`, playback device chips (`Arctis Nova 7`, `Speakers`), mic mute button, monitor button, Settings button, and footer selectors (`Pill`/`Dot`/`Hide`, presets, themes). **Fix applied:** registered `media_command` (`Command::Media { action }`) and `set_idle_style` in `app/src-tauri/src/main.rs` so transport buttons and idle-style switches work end-to-end in the live Tauri shell. |
| **WCAG 2.2 Text Contrast over Light & Dark Desktops** | ✅ (fixed) | Audited all 7 foreground tokens (`--ice-fg`, `--ice-fg-muted`, `--ice-accent`, `--ice-accent-strong`, `--ice-ok`, `--ice-warn`, `--ice-danger`) across all 4 themes (`Ice`, `Midnight`, `Fluent`, `Minimal`) composited over dark (`#0E1116`, `#000000`) and light (`#F6F7F9`, `#FFFFFF`) desktops, including at the top-center radial glow peak. **Fix applied:** `Fluent`'s `rgba(32, 32, 32, 0.88)` backdrop and full-opacity `.accent-glow` previously allowed `--ice-fg-muted` (`3.11:1`–`4.45:1`) and `--ice-danger` (`2.67:1`–`3.82:1`) to drop below `4.5:1` over `#FFFFFF`. Tuned `Fluent` to `rgba(24, 24, 27, 0.93)` / `rgba(36, 36, 40, 0.94)` (`base.css`, `native_dot.rs`, `docs/theming.md`) and calibrated `.accent-glow` opacity to `0.28` (`activeGlowCss` alpha `0.24`). Every token now achieves **`≥ 4.54:1` WCAG AA** (`--ice-fg` is **`11.58:1–18.53:1` WCAG AAA**) over both light and dark desktops. |
| **D-15.1 Footprint & Trust Gates (`F1–F4`, `G1`)** | ✅ | 60 s settled release benchmark (`app/out/bench-p2.json`): `5_notch_try_suspended` = **`6.00 MB` Private WS** (`≤ 15 MB` budget, `0.004%` CPU); `3_notch_visible_warm` = **`80.45 MB` Private WS** (`≤ 100 MB` budget); `6_notch_destroyed_native_dot` = **`3.61 MB` Private WS** (`0.002%` CPU); `7_after_10x_settings` = **`70.30 MB` Private WS**; `TrySuspend` resume latency `21.89 ms` p50 / `28.49 ms` p95. |

### Run: 2026-11-23 · `phase-3/visualizer` · Gemini
| ID / Check | Result | Notes |
|---|---|---|
| **Visualizer logic & springs (`visualizer.test.ts`)** | ✅ | 19/19 UI unit tests pass (`spring.test.ts`, `scheduler.test.ts`, `visualizer.test.ts`): 4–6 bar channel spreading, `peak` fallback when `channels` is empty, perceptual curve (`0.42` gamma), fast-attack (`>=75%` in `45 ms`) / slow-decay (`>15%` at `150 ms`, full settle `< 1 s`), exact `75 Hz` / `144 Hz` / `240 Hz` invariance (`< 1e-8` delta), and `Off` instant jump with `0` frames. |
| **Compact & Expanded layout (`100%` / `150%`, light & dark)** | ✅ (CDP) / Pending 144/75 Hz HW | Verified via headless Chromium CDP against `app/ui/dist` over `#0E1116` dark and `#F6F7F9` light backgrounds at `1.0×` (`100%`) and `1.5×` (`150%`) DPR. Physical `144 Hz` / `75 Hz` monitor visual check and live `--fake-engine` `audio://meter/v1` verification remain pending (`phase-3/meter-shell` not yet merged; current test host has dual `1920×1080 @ 60 Hz` `HP Z22i` displays). |
| **DevTools perf trace & idle loop stop** | ✅ | CDP `Performance.getMetrics` + `Tracing` at 30 Hz meter feed: compact **`0.435%`** whole-machine CPU (`239.8 µs` avg rAF, `0 ms` layout), expanded **`0.493%`** CPU (`147.1 µs` avg rAF, `0 ms` layout). **`0` rAF calls & `0` active intervals** when settled on silence, paused (`status: 'paused'`), hidden (`suspended: true`), and `Off` reduce motion. Pre-merge mock fallback stops cleanly on pause/hide. |

### Run: 2026-11-23 · `phase-3/notch-d18` · Gemini
| ID / Check | Result | Notes |
|---|---|---|
| **D-18 device row & picker (`devices.test.ts`, `23/23` UI tests)** | ✅ | Unit-tested favourites order, playback alias labels, non-favourite default prepended first, disconnected favourites hidden unless `showDisconnected` is on, hidden exclusion, no-favourites fallback (`current` + `togglePairs[0]` + `Choose devices…`), 4-chip cap + `More…`, and `savedId` order preservation when toggling favourites. |
| **Live WebView2 E2E (`--fake-engine --show-notch`)** | ✅ | Verified `set_visualizer(true/false)` + `audio://meter/v1` live bar animation (`2.522%` single-core `TaskDuration` ≈ `0.21%` whole-machine), `More…` picker `switch_to_device` + `set_notch_devices` updating `favouritesPlayback`, pause stopping bars (`0` mounted), and production `!DEV` error hiding bars without mock fallback. |
| **Live Real Engine & 144/75 Hz physical QA (`--show-notch`)** | ⚠️ Partial (RDP host) | Real-engine app launched cleanly with D-18 fallback row (`Remote Audio`, `Choose devices…`), `Mic On`, `Settings`, `0` bars. Current session runs in RDP (`Remote Audio`, `media: undefined`, 1 virtual `60 Hz` display), so live Spotify/real-audio bar verification and physical `144 Hz` / `75 Hz` monitor visual check remain for the owner's local reference displays. CDP `100%`/`150%` light/dark screenshots (`#0E1116` & `#F6F7F9`) verified visually. |

### Run · 2026-11-23 · F17 shell meter

With the real Windows engine, `--verbose`, and a temporary notch-side IPC probe, the engine logged `meter stopped: no subscribers` 31 ms after hide/TrySuspend, 32 ms after the fullscreen fixture triggered gaming mode on the notch monitor, and 63 ms after switching to Lite. Windows thread descriptions showed `ice-meter` and `ice-notch-meter` while visible and neither after each transition.
