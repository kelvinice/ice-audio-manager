# ice-win native pill (native dot)

> Paths: `crates/ice-win/src/native_dot.rs`, `native_dot/{glyphs,input}.rs` · Owner: Gemini · Verified: 2026-11-23

## Purpose
A tiny native Win32 layered window that shows the idle pill, short activities (volume, mic, device events) and the whole `Lite`
profile **with no WebView alive** (D-15 / D-15.1). It is the answer to "the WebView is a resource, not a given". About 2 MB private bytes.

## Files
| File | What it does |
|---|---|
| `native_dot.rs` | Window + message loop thread, `NativeDotHandle`, `PillState`, `PillMode`, `NativePillTheme`, rasteriser (`rasterize_pill_frame`), engine subscription. **2022 lines**: over the limit, split planned (plan.md Cleanup backlog: window / drawing / layout / theme) |
| `native_dot/glyphs.rs` | GDI glyph rasteriser: Segoe UI Variable Text + Segoe Fluent Icons into an alpha mask, no Direct2D/DirectWrite DLLs |
| `native_dot/input.rs` | Mouse input as plain window messages: click-first, pressed state paints in one frame; `PillInput::{Click, MiddleClick, HoverOpen, DragStart, Wheel}`, `set_input_handler` |

## Public surface
- `NativeDotHandle::spawn(monitor)`, `show_idle`, `hide`, `set_idle_style(IdleStyle)`, `set_notch_style(NotchStyle)`, `set_drag_enabled(bool)`, `drag_move((i32, i32))`, `drag_end()`, `origin_for(monitor, anchor)`, `hwnd_raw()`, `move_to_monitor`, `set_theme`, `pulse_activity(progress, accent)`,
  `show_volume`, `show_mic_mute`, `show_state`, `set_standby_only`, `set_activity_timeout`, `set_input_handler`, `set_hover_open(delay)`, `is_visible`.
- State sync: `apply_snapshot` / `apply_envelope` → `EnvelopeOutcome::{Updated, IgnoredStale, NeedsResnapshot}` (same revision protocol as the UI),
  or `subscribe_engine(engine)` → `NativePillSubscription` (own thread; `stop`/`join`).
- Theme: `NativePillTheme` built from `BuiltinTheme` (`Ice`, `IceLight`, `Midnight`, `Fluent`, `Minimal`) + `from_css_overrides` so it follows the same `--ice-*` tokens as the WebView (`ice_light()` mirrors the snow palette).

## How it works
- Draws premultiplied BGRA into a DIB and presents with `UpdateLayeredWindow`; sizes `116×28` (r=14) and `36×18` (r=9) match the WebView pill tokens in `Floating`, while `NotchStyle::Nook` (`frame_metrics`, `sd_nook_notch`) docks flush at `y = 0` with concave top ears (`6`/`8` px) and `+2 px` body height.
- Event-driven: a timer exists only while an activity is showing (stop = activity timeout), no cursor poll (input arrives as messages).
- The wheel is **deliberately not handled** until Windows' "scroll inactive windows" setting is tested both ways (D-19).
- Twin logic: volume %, mute/mic glyph choice and theme mapping mirror the WebView (`app/ui`). Each copy carries a `// Twin:` comment (§11.1).
  D-19 moves this decision into `ice_core::IdleContent` (not built yet).

## Traps
- Never put logic here the app needs elsewhere; the shell reaches it through `SharedRuntime.native_dot`.
- Known bug fixed in plan D-19: the "MIC OFF" idle label was unreachable while volume was known (`if let Some(v) = state.volume … else if mic_muted`).
- Pill size/radius constants (`IDLE_W`, …) and `frame_metrics` have a TS twin in `app/ui/src/lib/notch/{geometry,notchStyle}.ts`; change both.

## Tests
Unit tests in the file cover state and envelope handling and frame rasterisation; `cargo test -p ice-win real_click -- --ignored` is the by-hand focus check (QA F4).

## Change log
- 2026-11-23 Claude: initial recap.
- 2026-11-23 Claude: window class now has an arrow cursor (`hCursor`); the hit-testable pill had none and could show a stale busy cursor on hover.
- 2026-11-24 Gemini: added `BuiltinTheme::IceLight` and `NativePillTheme::ice_light()` for `Ice Light` native pill parity.
- 2026-11-24 Gemini: added `NotchStyle::Nook` flush top-edge rendering (`frame_metrics`, `sd_nook_notch`, `set_notch_style`) matching WebView Edge Nook.
- 2026-11-25 Claude: D-27 plain mode: `PillState.plain` / `set_plain` (activities update values but stay `Idle`, no reveal), `rasterize_plain_frame` (black silhouette + red mic dot), click-through while plain (`paint_pill`), `NativeDotHandle::set_capture_excluded` (applied on the pill thread via `ThreadSignal::CaptureExcluded`).
- 2026-11-25 Claude: `apply_snapshot` / `apply_envelope` take `Appearance::effective_idle_style()`, so hibernating with the hidden look hides the pill through the usual `IdleStyle::Hidden` path (no plain frame painted first: the shell sets the idle style before `set_plain`).
- 2026-11-25 Claude: D-28 C1 twin: the volume pop-up draws the % first (left-aligned, 13 px box) and the bar after it (82–180); `glyphs::Align::Left`.
- 2026-11-25 Claude: `PillState.anchor` + `NativeDotHandle::set_anchor`; `paint_pill` and window creation use `overlay_win::notch_origin` (one centre with the WebView notch).
- 2026-11-25 Claude: the wheel: `PillInput::Wheel { notches, shift }` from `WM_MOUSEWHEEL` (`wheel_notches` accumulates touchpad deltas), emitted outside the click state machine; window test sends real messages.
- 2026-11-25 Claude: D-20 drag & snap + bottom anchors: `PillInput::DragStart` when pressed past `SM_CXDRAG`/`SM_CYDRAG`, `set_drag_enabled`, `drag_move` / `drag_end` / `origin_for` / `hwnd_raw` (`PillState.drag_origin`), and `notch_origin` updated for `body_h`.
- 2026-11-25 Gemini: `PillState.x_ratio` + `set_position` (replacing `set_anchor`), inverted bottom Edge Nook SDF in `rasterize_pill_frame` / `rasterize_plain_frame`, and `flash_placed` / `PLACED_FLASH_TIMER_ID` (240 ms accent rim highlight on drop).
- 2026-11-26 Claude: owner: a pressed or carried Edge Nook pill (`PillInteraction::Pressed` or `drag_origin`) is drawn as a full capsule (`sd_round_box`, no ears) with the same layout; twin of the WebView lift (`buildLiftedNookPaths`). Test `a_carried_or_pressed_nook_pill_is_a_full_capsule`.
