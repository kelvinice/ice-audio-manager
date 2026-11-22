# Ice Audio Manager: Code Practices

> **Status: APPROVED** (owner sign-off 2026-11-22). Stack: Rust + Tauri 2 + Svelte 5
> (D-01 to D-04 in [planning-discussion.md](planning-discussion.md)). If Phase 0 triggers the D-01 fallback
> (native notch), §5 applies only to the Settings window. These rules apply to **every** agent and human contributor.

---

## 1. Non-negotiables ("don't look like malware")

These are hard rules. A PR that breaks one is rejected, whatever else it does.

1. **No low-level input hooks.** No `SetWindowsHookEx(WH_KEYBOARD_LL / WH_MOUSE_LL)`, no raw-input keylogging.
   Global hotkeys use `RegisterHotKey` only. Volume/media changes are observed via Core Audio/GSMTC callbacks.
2. **No DLL injection, no code injection, no patching other processes**, including Explorer/ShellExperienceHost.
3. **No admin.** Manifest is `asInvoker`. Write only to `%APPDATA%`, `%LOCALAPPDATA%`, `HKCU`.
4. **No packers or obfuscators** (UPX, PyInstaller-style onefile, ConfuserEx, etc.).
5. **No silent network.** No telemetry. The only network call allowed is the opt-in "check for update" (D-11),
   which may only *notify*. Never download or execute code.
6. **No undocumented API without a guard.** `IPolicyConfig`, `IAudioPolicyConfigFactory`, and shell window hacks must be
   isolated in `ice-win`, feature-detected at runtime, and fail gracefully with a log line, never a crash.
7. **No secrets in the repo.** `.env` is for local dev only and must not hold credentials. Signing keys live only in CI secrets.

## 2. Lightweight rules

- **Event-driven, never polling**, except: the cursor poll for click-through (only while the cursor is near the notch, ≤ 30 Hz)
  and the peak meter (only while the visualizer is visible, ≤ 30 Hz). Every timer must have a documented stop condition.
- **Hidden means idle:** when the notch is hidden, stop animation loops and timers.
- **Lazy windows:** the settings window is created on open and destroyed on close.
- **The WebView is a resource, not a given (D-15):** the core, tray, hotkeys and the native idle dot must work with **no
  WebView alive**. Never put logic in the UI that the app needs while the WebView is released (`Balanced`/`Lite`/gaming mode).
  A recreated WebView rebuilds itself from the snapshot and never replays expired activities.
- **Dependency budget:** each new crate or npm package needs a one-line justification in the PR description
  ("why not std / existing dep?"). Prefer `std`, `windows`, `serde`. No UI component libraries.
- **Heavy work in Rust, not JS:** image decoding/resizing, colour extraction, sorting device lists.
- Check budgets from [plan.md](plan.md#performance-budgets-gates) before merging any phase-closing PR.

## 3. Architecture rules

- **Layering (enforced by crate boundaries):**
  `ice-core` (pure logic, no Windows deps) ← `ice-win` (all FFI) ← `app/src-tauri` (wiring) ↔ `app/ui` (presentation).
  `ice-core` must never depend on `ice-win` or Tauri.
- **Only `ice-win` may contain `unsafe`.** Other crates use `#![forbid(unsafe_code)]`.
- **Device identity is a `DeviceRef` (D-16):** the endpoint ID first, plus a hardware fingerprint for re-linking. Never use indices or
  friendly names as keys. **Resolve only against Active endpoints.** Windows keeps stale NotPresent endpoints with identical names.
  Never fail silently: an unresolved or ambiguous device always produces user-visible feedback.
- **Single source of truth:** backend state lives in Rust. The UI is a projection of events plus a snapshot on load.
  The UI never caches authoritative state beyond what it renders.
- **Everything configurable goes through `ice-core::config`.** No magic numbers for durations, sizes, colours, or springs
  in UI code. Those come from config (behaviour) or CSS variables (appearance).

## 4. Rust practices **[stack]**

- Edition 2024, stable MSVC toolchain pinned in `rust-toolchain.toml` (owner machine: rustc 1.100.0, VS 2022 C++ tools).
- Build from PowerShell or a VS Developer prompt, **not Git Bash**. Git's `/usr/bin/link.exe` sits on the Git Bash PATH and
  can shadow the MSVC linker.
- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must pass. Enable `clippy::pedantic` selectively in `ice-core`.
- **Errors:** `thiserror` enums in library crates, `anyhow` only in the app binary. No `unwrap()`/`expect()`
  outside tests and `main` startup, except with a comment proving it can't fail.
- **`unsafe`:** every block has a `// SAFETY:` comment. Wrap raw COM in small safe types (RAII, `Drop` releases),
  and expose only safe APIs from `ice-win`.
- **COM threading:**
  - All Core Audio work runs on a dedicated **MTA audio thread** that owns the COM objects.
  - COM callbacks (`IMMNotificationClient`, `IAudioEndpointVolumeCallback`, session events) **must not block and must not
    call back into the same COM objects synchronously**. They copy data into a channel and return immediately
    (Microsoft documents deadlocks otherwise).
  - Re-acquire endpoint interfaces after `AUDCLNT_E_DEVICE_INVALIDATED` or a device-removed notification.
- **Async:** keep it minimal. Tauri brings its own runtime, so don't add a second one. Prefer channels + dedicated threads for Windows
  message loops (hotkeys, hidden windows) and COM.
- **Logging:** `tracing` with structured fields (`device_id`, `role`, `activity`). No `println!`. Never log track titles at
  info level (privacy). Use debug only.
- **Tests:** `ice-core` is fully unit-tested (scheduler, config migration, rules). `ice-win` exposes traits
  (`AudioBackend`, `MediaBackend`) with fake implementations for tests. Hardware paths are covered by the manual QA checklist.

## 5. Frontend practices (Svelte 5 + TS) **[stack]**

- TypeScript `strict: true`. No `any` without a `// reason:` comment.
- Svelte 5 runes (`$state`, `$derived`, `$effect`). Keep components small: one activity = one component under `ui/src/notch/activities/`.
- **IPC is typed end-to-end.** Command and event payload types are generated from Rust (e.g. `tauri-specta` or `ts-rs`,
  decided in Phase 0) into `ui/src/lib/ipc/`. Never hand-write payload types.
- **Animation rules:**
  - Springs only, from our own `(stiffness, damping, mass)` module in `ui/src/lib/motion/` (pure TS, injected clock,
    time-based so it's frame-rate independent, unit-tested at 75/144/240 Hz `dt`; thin Svelte adapter). **Not** `svelte/motion`,
    which has no mass. Parameters come from config presets.
  - Animate `transform`, `opacity`, `filter`, `clip-path` only. Never `width/height/top/left` (layout thrash).
    The island's size morph drives an **SVG `<rect rx ry>`** (`vector-effect="non-scaling-stroke"`) + `clip-path: inset(… round …)`
    from the spring's `(w, h, r)`. Don't `scale()` the pill: it distorts corners and 1 px rims (D-13.1).
  - Honour the reduce-motion flag from the backend (Windows setting), not only `prefers-reduced-motion`. **Off = instant**:
    geometry and content update immediately, with no fade.
  - No animation loop runs while the notch is hidden.
- **Styling & theming:**
  - All visual values are CSS custom properties defined in `ui/src/themes/base.css` with the `--ice-` prefix
    (e.g. `--ice-island-bg`, `--ice-island-radius`, `--ice-accent`, `--ice-blur`).
  - Components may only use theme variables, never literal colours or radii.
  - Any new variable must be added to `docs/theming.md` in the same PR.
- **Accessibility:** settings UI is fully keyboard navigable with visible focus rings and ARIA labels on icon buttons.

## 6. Tauri shell practices **[stack]**

- Capabilities/permissions are least-privilege: each window gets only the commands it uses. No `shell` plugin.
  No filesystem plugin scope beyond the app config dir.
- CSP enabled. No remote URLs loaded. No `eval`.
- Commands are thin: validate input, call `ice-core`/`ice-win`, return typed results. No business logic in command handlers.
- Event names are namespaced and versioned with Tauri-legal characters: `audio://device-changed/v1`, `notch://activity/v1`
  (Tauri rejects `@`).
- **Snapshot + delta protocol (D-03):** every snapshot and delta carries the same monotonic `revision`. On window creation or
  wake: register the listener first, buffer deltas, fetch the snapshot, apply only newer revisions in order, and re-snapshot on a gap.
  High-frequency values (volume, peaks) are coalesced. On queue overflow, re-snapshot instead of replaying.
- Custom protocols (`iceart://`) serve only cached image IDs. Verify their origin in the production CSP.

## 7. Config & data

- `config.toml` has `schema_version`. Every schema change ships a migration and a test that migrates the previous version's fixture.
- Writes are atomic (write temp → `ReplaceFileW`/rename). Invalid config on load means keep running with defaults,
  back up the bad file as `config.invalid-<timestamp>.toml`, and show a notification.
- Hot reload: file watcher → validate → apply diff → emit `config://changed@1`.

## 8. Git & collaboration (multi-agent)

- **Branches:** `main` is always buildable and only changes through the owner's merges. **Every task branches from the latest
  `main`** as `phase-<n>/<short-name>` (e.g. `phase-0/overlay-spike`, `phase-2/volume-activity`). Never branch from another
  agent's unmerged branch. If you need unmerged work, wait for it to merge, or coordinate with its owner first.
- **One working folder per agent (git worktrees).** All agents work on the same PC, so never switch branches in a folder
  another agent is using. That's how the earlier save conflicts happened. The owner's folder stays on `main`. Each agent works in its own worktree:
  ```powershell
  git worktree add ..\ice-wt-gemini -b phase-0/overlay-spike main   # Gemini
  git worktree add ..\ice-wt-codex  -b phase-0/fullscreen-spike main # Codex
  git worktree add ..\ice-wt-claude -b phase-1/device-resolver main  # Claude
  ```
  Remove a worktree after its branch merges (`git worktree remove ..\ice-wt-gemini`). `target/` and `node_modules/` are per worktree.
- **Shared docs** (`docs/planning-discussion.md`, `plan.md`, this file): edit them on your task branch like code, keeping to your own
  sections. If two branches touch the same table, the second to merge rebases and resolves it. Never overwrite another agent's text.
- **Staying current:** before opening a PR, `git fetch` (if a remote is used) and rebase on `main` (`git rebase main`). Re-run fmt, clippy and tests.
- **Commits:** Conventional Commits (`feat(notch): …`, `fix(audio): …`, `docs: …`, `chore(ci): …`), authored as the owner's git user.
  **No agent `Co-Authored-By` trailers or other AI attribution lines** (owner decision, 2026-11-22). Say which agent did the work
  in the PR description instead.
- **PRs:** small (aim < 400 changed lines excluding generated files). The description includes: what/why, linked feature IDs (F-xx)
  and decision IDs (D-xx), new deps with justification, budget impact if any, screenshots/GIF for UI.
- **Review:** at least one *other* agent reviews before the owner merges. Reviewers check §1 first.
- **Ownership:** follow D-14. Editing another owner's area is fine with a heads-up in the PR. Avoid two agents working on the same files at once.
- **Decisions:** anything that changes architecture or a closed decision needs a new entry in
  `planning-discussion.md` (or an ADR in `docs/decisions/NNNN-title.md`) **before** the code PR.

## 9. Assets

- SVG is the source of truth in `assets/src/`. Raster/ICO outputs are generated by a script in `assets/build` and committed
  only under `assets/dist/` (or generated in CI, decided in Phase 0).
- Icons: monochrome, drawn on a 16 px grid, readable at 16 px, light + dark variants for tray.
- Prefer system **Segoe Fluent Icons** glyphs (referenced, not bundled) over custom assets.
- Every generated asset records its prompt/tool in `assets/CREDITS.md`. Nothing copied from DynamicWin or other apps.

## 10. Definition of Done (every PR)

- [ ] Builds and passes fmt/lint/tests in CI
- [ ] No rule in §1 violated; no new polling without a stop condition
- [ ] New config fields have defaults, validation, docs; new CSS variables documented
- [ ] Works at 100% and 150% display scaling; light and dark
- [ ] Manual QA steps listed in the PR for hardware-dependent behaviour
- [ ] Docs updated (`plan.md` checkboxes ticked, README if user-facing)
