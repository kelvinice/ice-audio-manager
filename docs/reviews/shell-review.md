# Phase 1 tray shell review (`phase-1/real-engine`)

> **Reviewer:** Claude · **Commit:** `bd42d6e` (Codex), merged to `main` 2026-11-22 · **Scope:** review only

## Verdict: merged. No blocking findings

Checked: fmt, clippy `-D warnings`, `cargo test --workspace` (44 + 10 + 3), UI check + 11 tests, from a clean worktree.

| Rule | Status | Notes |
|---|---|---|
| §1 no hooks / injection / admin / network | ✅ | Tray balloons via `Shell_NotifyIconW(NIM_MODIFY, NIF_INFO)` with `NIIF_RESPECT_QUIET_TIME`; no processes, registry writes or network in the tray code |
| §4 threading | ✅ | `engine.send` is non-blocking; engine events are marshalled with `run_on_main_thread`; `unwrap()` only in tests |
| Tray GUID pitfall | ✅ | GUID derived from a hash of the exe path, so debug/release/worktree builds don't collide (Windows binds a tray GUID to one path) |
| Real engine by default | ✅ | `%APPDATA%\IceAudioManager\config.toml`; `--fake-engine` and bench mode keep the fake |

## Non-blocking notes (Codex, when convenient)

| ID | Severity | Note |
|---|---|---|
| SHELL-01 | Low | `engine_bridge::watch` ends silently if its receiver disconnects. The engine drops a subscriber that falls 1024 events behind (ENG-12), and then the tray stops updating. Re-subscribe and re-read the snapshot when the loop ends while the app is still running |
| SHELL-02 | Medium | If `AudioEngine::start` fails (e.g. audio service not up yet at sign-in), the whole app exits. Prefer: start the tray anyway, show the error in the menu, retry the engine every few seconds (bounded), or fall back to the fake engine with a clear "audio unavailable" state |
| SHELL-03 | Cosmetic | `handle_prompt_action` refreshes the tooltip from a snapshot taken right after `send`, before the engine has processed the command; the following event corrects it. Either drop that refresh or let the event do it |
