# Phase 0.2 findings: overlay, springs, and memory policy (Gemini)

> Measured 2026-11-22 on the owner's PC (Windows 11, Ryzen 5 3600X, RTX 3050, 16 GB RAM, `\\.\DISPLAY1` 1920×1080 @ 144 Hz primary + `\\.\DISPLAY2` 1920×1080 @ 75 Hz secondary at 100% scale, LTO release build).
> Code: [`crates/ice-win`](../../crates/ice-win) (`monitors`, `overlay_win`, `native_dot`, `proc_metrics`, `webview2`) + [`spikes/overlay`](../../spikes/overlay).

## Run from PowerShell

```powershell
# UI checks, 75/144/240 Hz spring unit tests, and production dist build
Set-Location spikes\overlay\ui
npm run check
npm test
npm run build

# Release benchmark (60 s settle-and-sample across all 7 memory/CPU states)
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
Set-Location ..\..\..
cargo build --release -p overlay-spike
.\target\release\overlay-spike.exe --bench --sample-secs 60 --out spikes\overlay\out\bench-60s.json

# Interactive tray mode (right-click tray icon to test events, presets, monitor switch, Settings)
.\target\release\overlay-spike.exe
```

Release binary size (`target/release/overlay-spike.exe`, LTO): **8,841,728 bytes (8.43 MiB)** (target `< 10 MB`).

## 7-state memory and CPU (measured, 60 s settled)

Each state settles for 3 s, then samples `PrivateUsage` (Private Bytes), `PrivateWorkingSetSize` (Private WS), `WorkingSetSize`, and per-PID CPU time (`GetProcessTimes` over alive `STILL_ACTIVE` processes returned by `ICoreWebView2Environment14::GetProcessInfos`) over **60 s**.

| State | Procs (wv2) | Private Bytes (MB) | Private WS (MB) | Working Set (MB) | Avg CPU % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `1_native_host_only` | 1 (0) | **2.11** | **1.58** | 11.34 | **0.000%** |
| `2_native_dot_only` | 1 (0) | **2.20** | **1.64** | 11.48 | **0.000%** |
| `3_notch_visible_warm` | 6 (5) | **149.84** | **77.10** | 346.15 | **0.059%** |
| `4_notch_hidden` | 6 (5) | **150.39** | **76.79** | 347.25 | **0.078%** |
| `5_notch_try_suspended` | 6 (5) | **148.01** | **6.44** | 32.34 | **0.009%** |
| `6_notch_destroyed_native_dot` | 1 (0) | **5.56** | **3.76** | 26.62 | **0.000%** |
| `7_after_10x_settings` | 6 (5) | **167.49** | **74.25** | 343.54 | **0.054%** |

### Warm WebView2 per-process breakdown (`3_notch_visible_warm` vs `5_notch_try_suspended`)

| Process kind | Warm Private Bytes (MB) | Warm Private WS (MB) | Warm Working Set (MB) | `TrySuspend` Private Bytes (MB) | `TrySuspend` Private WS (MB) |
| --- | ---: | ---: | ---: | ---: | ---: |
| `host` (`overlay-spike.exe`) | 5.73 | 4.22 | 26.59 | 5.79 | 3.91 |
| `wv2-gpu` | 61.04 | 19.86 | 65.58 | 60.68 | 0.66 |
| `wv2-browser` | 35.20 | 25.94 | 129.31 | 35.50 | 1.49 |
| `wv2-renderer` | 29.47 | 17.96 | 65.54 | 27.93 | 0.21 |
| `wv2-utility` (×2 combined) | 18.40 | 9.13 | 59.13 | 18.11 | 0.16 |

Key teardown observations:

- **Native dot overhead:** `+0.09 MB` Private Bytes (`2.11 → 2.20 MB`), `0.000%` idle CPU.
- **Full WebView2 release (`State 6`):** Destroying the notch window exits all 5 `msedgewebview2.exe` processes (`remaining known_wv2_pids=0`), returning total Private Bytes to **5.56 MB** and `0.000%` CPU.
- **10× lazy Settings open/destroy cycles (`State 7`):** Private WS stayed flat (`77.10 → 74.25 MB`), while `wv2-gpu` committed Private Bytes rose from `61.04` to `82.27 MB` (total `149.84 → 167.49 MB`). Isolating Settings in a separate WebView2 user-data folder is worth doing if Settings uses WebView2 alongside a resident WebView2 notch.

## Latency and window behaviour (measured)

| Metric | Samples | p50 (ms) | p95 (ms) | max (ms) | Budget / check |
| --- | ---: | ---: | ---: | ---: | --- |
| Native dot pulse (`native_dot_pulse_ms`) | 10 | **0.02** | **0.02** | 0.02 | Target `< 50 ms` first feedback |
| `Balanced` cold dot react (`balanced_cold_dot_react_ms`) | 5 | **0.02** | **0.06** | 0.06 | Immediate native pill response |
| Warm event → 2× rAF render (`warm_event_to_render_ms`) | 10 | **10.22** | **34.17** | 34.17 | `< 50 ms` |
| Monitor move 144 Hz ↔ 75 Hz (`monitor_move_ms`) | 4 | **1.08** | **1.39** | 1.39 | Single HWND `SetWindowPos` |
| `TrySuspend` resume + render (`try_suspend_resume_ms`) | 3 | **18.17** | **27.24** | 27.24 | `TrySuspend` returned `true` |
| `Balanced` cold WebView2 spawn (`balanced_cold_wv2_paint_ms`) | 5 | **604.55** | **715.44** | 715.44 | **FAILS `≤ 250 ms` D-15 p95 gate** (and > 500 ms diagnostic cap; initial spawn: `1010.22 ms`) |

Window flags & input routing checks (all passed in release benchmark):

- **Zero focus stealing:** `focus_preserved_on_spawn = true`, `focus_preserved_on_activity = true`, `focus_preserved_on_monitor_move = true` (`WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST` + `WM_MOUSEACTIVATE → MA_NOACTIVATE` subclass in `ice_win::overlay_win`).
- **Click-through toggling:** `click_through_toggled_cleanly = true` (`WS_EX_TRANSPARENT` toggled on/off/on via cursor-proximity poll around the UI's `HitRect`).
- **Reduced motion:** `SPI_GETCLIENTAREAANIMATION` read cleanly (`false` on test system).

## Spring physics verification (`spring.test.ts`, 5/5 passing)

- Exact closed-form analytical `(stiffness k, damping c, mass m)` oscillator in [`spikes/overlay/ui/src/lib/motion/spring.ts`](../../spikes/overlay/ui/src/lib/motion/spring.ts) animates SVG `<rect rx ry>` + `clip-path: inset(...)` with zero layout-property animation.
- `Bouncy` retuned per Codex's D-13 check (`k=300, c=21.7, m=1.0`): verified **7.5%–8.5%** (~8%) step overshoot (`bouncy` test).
- `Snappy` (`k=420, c=34, m=1.0`) and `Smooth` (`k=280, c=31, m=1.0`) verified `< 2.0%` and `< 0.25%` overshoot respectively.
- Identical position & velocity (`< 1e-8` delta) across **75 Hz, 144 Hz, and 240 Hz**, including mid-flight velocity-preserving retargeting.
- `Off` preset (`instant: true`) updates immediately with zero animation frames; settled springs auto-sleep (`isSettled()` returns rAF to 0%).

## D-15 gate analysis & verdict

Evaluating the 60 s settled release numbers against the signed-off **D-15** decision gates (`plan.md` / `planning-discussion.md`):

1. **Gate 1 (`Instant` idle Private Bytes ≤ 100 MB):** **FAILS.** Warm (`149.84 MB`), hidden (`150.39 MB`), `TrySuspend` (`148.01 MB`), and post-Settings (`167.49 MB`) Private Bytes all exceed `100 MB`.
2. **Gate 2 (`Instant` ≤ 150 MB Private Bytes AND `Balanced` cold WebView2 p95 ≤ 250 ms):** **FAILS.**
   - `Balanced` cold WebView2 spawn (`604.55 ms p50` / **`715.44 ms p95`**) exceeds the `≤ 250 ms` p95 gate by `2.86×` (even on warm cache with an empty minimal Svelte pill), creating a visible two-stage pop-in behind the native pill.
   - `Instant` Private Bytes sit right on/above the `150 MB` limit (`150.39 MB` hidden, `167.49 MB` after 10× Settings cycles).
3. **Gate 3 (Else → Option C: Native Direct2D / DirectComposition notch + lazy WebView2 Settings only):** **TRIGGERED (strict D-15 verdict = Option C).**
   - A native Rust notch (`Direct2D` + `DirectComposition`) avoids keeping 5 WebView2 processes resident (~`2.2–5.6 MB` Private Bytes measured for native host + dot, `0.000%` idle CPU, `0.02 ms` p95 response, native 144/75 Hz frame pacing) while keeping Tauri/WebView2 strictly for lazy on-demand Settings (`0` WebView2 processes whenever Settings is closed).

### Owner decision point before Phase 2 (`Private Bytes` vs `Private Working Set`)

Because the 60 s `TrySuspend` sample surfaced an important metric nuance, the owner has one explicit sign-off choice before Phase 2 notch implementation starts:

| Path | Memory profile (measured) | Wake latency (measured) | Trade-off |
| --- | --- | --- | --- |
| **C. Native Direct2D / DirectComposition notch** *(strict D-15 gate winner)* | Native host + dot measured at **2.20–5.56 MB Private Bytes** (`0.000%` CPU); lazy WebView2 for Settings only | **< 1 ms** (`0.02 ms` native dot), zero pop-in | Notch themes become token-only (`--ice-*` tokens parsed into Rust primitives, no arbitrary user CSS DOM layouts); higher Phase 2 implementation effort |
| **A. Keep Tauri WebView2 notch (`Instant` + `TrySuspend`)** *(only if owner relaxes D-15 metric to Private Working Set)* | **6.44 MB Private WS** (`0.009%` CPU) suspended / **77.10 MB Private WS** warm, **but 148–167 MB committed Private Bytes**; `Lite` tears down to `5.56 MB` | **18.17 ms p50 / 27.24 ms p95** `TrySuspend` resume | Preserves full user CSS themes (`--ice-*` + CSS files) and Svelte UI, at the cost of ~150 MB virtual commit (`Private Bytes`) outside `Lite`/gaming mode |

## Manual interactive QA still recommended

While automated 60 s checks proved `WS_EX_NOACTIVATE`, `WS_EX_TRANSPARENT` toggling, and 0 leaked WebView2 PIDs on destroy, interactively confirm during shell wiring:

- Live mouse-wheel / click pass-through directly underneath and inside the `HitRect` on a text editor / browser at **100% and 150% DPI** across 144 Hz + 75 Hz displays.
- Visual inspection of spring morphs across `144 Hz` (`DISPLAY1`) and `75 Hz` (`DISPLAY2`) plus first-event wake behavior after Windows sleep/resume.
