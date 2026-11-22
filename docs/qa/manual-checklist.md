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
