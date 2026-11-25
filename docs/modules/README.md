# Module map

> **Read this before opening source.** Each file here is a short recap of one module: what it owns, its public surface,
> how data flows through it, and the traps. It exists to cut the tokens spent re-reading code. Last verified against `main`
> on **2026-11-23**.

## How to use (every agent)

1. Find your task in the table below and read **only** that module doc, plus [data-flows.md](data-flows.md) if the change
   crosses a layer.
2. Open source files only for the function you are changing. The docs give file names and symbol names so you can go
   straight there with a search.
3. If the doc and the code disagree, the **code wins**. Fix the doc in the same PR (see below).

## Keeping them current (every PR)

Part of the Definition of Done ([code-practices §10](../code-practices.md#10-definition-of-done-every-pr)):

- If your change adds, removes or renames a file, public type, command, event, config field, thread, timer or `--ice-*`
  token, or changes a flow, **edit the affected module doc in the same PR**. Add a line to its *Change log*.
- Edit only the module docs for the paths you touched. Never rewrite another module's doc from memory. Read the code first.
- Keep it a recap, not a copy: names, one-line purposes, invariants and traps. No code listings longer than 5 lines, no
  history (git has it), no plans (that is `plan.md`). Delete text that is no longer true.
- Update the `Verified` date only when you actually re-checked the whole doc against the code.
- A new module (new crate, new `ice-win` file group, new UI folder) gets its own doc from the [template](#template) and a row
  in the table below.

## Index

| Doc | Covers | Paths | Owner (D-14) |
|---|---|---|---|
| [ice-core](ice-core.md) | Pure logic and the engine contract: config, device identity, commands/events, media maths, meter, call-mode ducking | `crates/ice-core/**` | Claude |
| [ice-win-audio](ice-win-audio.md) | Real engine on Core Audio: switching, volume, hotkeys, spatial sound, config reload, call mode | `crates/ice-win/src/{audio_engine*,call_mode*,endpoints,notify,volume,policy,spatial,props,hotkeys,file_watch,meter,com}.rs` | Claude |
| [ice-win-media](ice-win-media.md) | Now playing via GSMTC, album art, the `ice-media` thread | `crates/ice-win/src/{media,media_service}.rs` | Claude |
| [ice-win-display](ice-win-display.md) | Overlay window styles, monitors, fullscreen watcher, WebView2 helpers, process metrics, autostart, single instance, music-service launcher | `crates/ice-win/src/{overlay_win,monitors,fullscreen,webview2,proc_metrics,desktop,autostart,music_service}.rs` | Gemini / Codex |
| [ice-win-native-pill](ice-win-native-pill.md) | The native idle pill (no WebView): window, drawing, input | `crates/ice-win/src/native_dot*` | Gemini |
| [app-shell](app-shell.md) | Tauri host: runtime state, windows, commands, engine bridge, placement, meter, `iceart://`, capabilities | `app/src-tauri/**` (except tray) | Codex |
| [app-tray](app-tray.md) | Tray menus, prompts and notices | `app/src-tauri/src/tray*` | Codex |
| [app-ui-notch](app-ui-notch.md) | Notch WebView: controller, scheduler, springs, geometry, activities, themes | `app/ui/**` (except Settings) | Gemini |
| [app-ui-settings](app-ui-settings.md) | Settings window: pages, controls, previews, settings client, `--ice-set-*` tokens | `app/ui/src/settings/**`, `app/ui/src/lib/settings/**` | Gemini |
| [data-flows](data-flows.md) | End-to-end traces: hotkey switch, volume key, media, wake/suspend, settings change | cross-layer | all |

## Layers at a glance

```
app/ui (Svelte, presentation)  ◄─ Tauri events/commands ─►  app/src-tauri (wiring, windows, tray)
                                                                  │
                                                           ice-win (all unsafe, Windows APIs)
                                                                  │
                                                           ice-core (pure logic, contract types)
```

Rules that matter for every module: only `ice-win` has `unsafe`; `ice-core` never depends on Windows or Tauri; the core,
tray, hotkeys and native pill must work with **no WebView alive** (D-15). Quick guide: [AGENTS.md](../../AGENTS.md) · Full rules: [code-practices](../code-practices.md).

## Commands

| Task | Command |
|---|---|
| Everything CI runs | `build check` (from `build.cmd`) |
| Rust only | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` |
| TS bindings | `cargo test -p ice-audio-manager export_ts_bindings` (rewrites `app/ui/src/lib/ipc/engine.ts` only when content changes) |
| UI | in `app/ui`: `npm run check`, `npm test`, `npm run build` |
| Run with fake devices | app exe with `--fake-engine` (also `--start-minimized`, `--verbose`, `--show-notch`) |

Build from PowerShell or a VS Developer prompt, not Git Bash (code-practices §4).

## Template

```markdown
# <module name>

> Paths: … · Owner: … · Verified: YYYY-MM-DD

## Purpose
One paragraph: what it owns and what it must never do.

## Files
| File | What it does |

## Public surface
Types / functions other modules use, with one line each.

## How it works
Threads, timers (with stop conditions), data flow, invariants.

## Traps
Non-obvious things that already cost someone time (cite D-xx / review IDs).

## Tests
Where the tests are and what they cover.

## Change log
- YYYY-MM-DD <agent>: one line per PR that changed the module's behaviour or surface.
```
