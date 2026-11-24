# app tray

> Paths: `app/src-tauri/src/tray.rs`, `tray/preferences.rs` · Owner: Codex · Verified: 2026-11-23

## Purpose
The always-available UI: the notification-area icon, its left-click quick-switch menu, its right-click full menu, the tooltip, and the
prompts/notices (device picker, hotkey conflicts, config problems). It works with no WebView alive and is the main settings surface until
the Phase 4 Settings window exists. It only builds menus from `EngineSnapshot` and sends `Command`s; no audio logic.

## Files
| File | What it does |
|---|---|
| `tray.rs` | `build`, `close`, `refresh_tooltip`, `show_notice`; menu construction (`quick_menu`, `append_devices`), menu-id ↔ `Command` mapping (`COMMAND_PREFIX` = `audio/`, `CHOICE_PREFIX` = `choice/`), `Prompts` (pending device pickers and feedback), `notify` balloons |
| `tray/preferences.rs` | "Show in notch" check item per playback device (= favourite, via `SetFavourites`, order kept), `edit_favourite` (re-reads favourites at click time because a reload may have reordered them), `appearance_menus` (`Theme` incl. `Ice Light`, `Animation` with `Bouncy` first, `Idle notch`, `Now playing pill`), `choices` |

## What the menu offers
Playback devices (switch; per-device **Show in notch**, spatial sound submenu with `SPATIAL_OPTIONS`) · playback volume line · recording devices ·
communications mic line · **Toggle pairs** (and "Toggle with the current device…" one-click setup) · **Show disconnected devices** ·
**Knob gestures** · **Theme / Animation / Idle notch / Now playing pill** submenus (saved in `[appearance]`) · **Performance profile** · **Pinned monitor** · notice/feedback entries
(Dismiss, Choose device for X, Cancel) · "Audio unavailable" while the engine is starting.

## How it works
- A menu item id encodes a serialised `Command` (`command_id` / `checked_command`); a click is decoded and sent with `EngineSlot::send`.
- `ChooseDevice` notices become a picker submenu; an answer sends `ResolveChoice`, Cancel sends `device_id: None`.
- Persistent notices (`snapshot.notices`) appear as status lines via `persistent_notice_text`; one-off notices use `desktop::notify_tray` balloons.
- Disconnected devices are greyed, non-clickable entries from `snapshot.disconnected`.
- In `Lite`, a click on the native pill opens this quick menu (D-19, shell wiring still open).
- Left-click scroll on the icon was cut (D-05.F05).

## Traps
- Menu text is user/device data: pass it through `menu_text` (escapes `&` mnemonics).
- A new `Command` that the tray should expose needs the id mapping and a snapshot-driven check state; don't cache state in the tray.
- `tray.rs` is 580 lines. Split before adding more submenus (§11.2).

## Tests
Unit tests in `tray.rs` and `preferences.rs` cover id round-trips and favourite edits. Manual: checklist sections A, C, D.

## Change log
- 2026-11-23 Claude: initial recap.
- 2026-11-23 Claude: "Launch at sign-in" reads the registry through `startup.rs` each time the menu opens (Settings can change it too).
- 2026-11-24 Claude: "Now playing pill" submenu (Brief, then small (default) / While playing / Off).
- 2026-11-24 Claude: left click goes through `open_quick_menu` (also used by the native pill and the show-notch hotkey in Lite).
- 2026-11-24 Gemini: `tray/preferences.rs` adds `Ice Light` to the `Theme` submenu and puts `Bouncy` first in `Animation`.
