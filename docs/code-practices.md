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
5. **No silent network.** No telemetry, no analytics, no crash reporting. Network use is allowed only for **opt-in,
   per-feature** functions (D-22 N1, owner 2026-11-23): the update check (D-11, notify only) and features accepted by a
   decision, such as lyrics lookup.
   - **Each is off by default,** with its own switch. Its destination and the exact data sent are listed in Settings →
     Privacy, and one switch turns all of them off.
   - **It sends only what the feature needs,** with no background traffic while it's off. Results are cached locally, and
     the cache is cleared when the feature is turned off.
   - **Requests are made from Rust only,** to an allowlisted host. The WebView CSP (§6) never gains a remote origin.
   - **Never download or execute code.**
6. **No undocumented API without a guard.** `IPolicyConfig`, `IAudioPolicyConfigFactory`, and shell window hacks must be
   isolated in `ice-win`, feature-detected at runtime, and fail gracefully with a log line, never a crash.
7. **No secrets in the repo.** `.env` is for local dev only and must not hold credentials. Signing keys live only in CI secrets.
8. **Reference, never copy (owner decision, 2026-11-23; D-24).** The project is **MIT**.
   - Other projects (`docs/reference/`, GitHub, any licence) may be read to learn *how* they solve something, including their
     implementation.
   - Our code is written by us, in our own structure and names. **Never copy, translate line by line (e.g. Kotlin → Rust) or
     closely paraphrase another project's code.**
   - Name the reference that informed a design in the PR description.
   - **Changing the licence (e.g. to GPL-3.0) happens only when the owner explicitly accepts it** for a named component, with
     the import record from D-24 (upstream revision, exact licence grant, files, upkeep cost). Until then, no GPL/AGPL code
     enters the repo.

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
- **Budgets are verified once, in the Phase 6 testing pass** (owner decision, 2026-11-23), not per PR or per phase. The
  Phase 0–3 measurements stand until then. The design rules above (event-driven, hidden means idle, lazy windows) still
  apply to every PR. They're what keeps the app inside the budgets between measurements.

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
- **Minimal, non-redundant testing (owner decision, 2026-11-23).** The focus until Phase 6 is features and UI/UX.
  - **One focused test per new behaviour,** at the lowest layer that owns it (pure logic in `ice-core` or a pure TS module).
    Don't test the same behaviour again in another layer.
  - **No performance or RAM benchmarks, soak tests or security audits in feature work.** Those run once, in the Phase 6
    testing pass (plan.md). Don't re-run the benchmark suite to re-confirm known numbers.
  - **UI/UX checks stay part of feature work:** how it looks and behaves at 100 %/150 %, light/dark, on both monitors, with
    a screenshot or a short note in the PR. Interaction details (focus never stolen, clicks land where expected) count as
    UX, not performance.
  - **Manual QA:** run only the checklist sections the change touches.

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
- **Merging (owner decision, 2026-11-22):** when a task or phase is done and fmt, clippy and tests pass, the agent that did it
  **merges its branch into `main` itself** (rebase on `main` first, then `git merge --ff-only`). It then deletes the branch and removes its worktree.
  Ask another agent for a review first when the change touches §1 rules, shared docs tables, or another agent's area. Reviewers check §1 first.
- **Independent reviews (owner decision, 2026-11-23):** the same rule as blind voting
  ([planning-discussion.md §0.4](planning-discussion.md)).
  - When two agents review the same change, each writes its findings before reading the other's review.
  - A review lists concrete findings (file:line, what's wrong, how it fails), or the specific checks it ran that found
    nothing. "Looks good, agree with X" isn't a review.
  - The author's own summary isn't evidence: reviewers read the diff and run the code or tests themselves.
- **Module docs (owner request, 2026-11-23):** [docs/modules/](modules/README.md) recaps how each module works so agents don't re-read source.
  Read the relevant module doc **before** the code, and update it in the same PR as the change (§10). The code wins if they disagree.
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
- [ ] Hardware-dependent behaviour checked against [docs/qa/manual-checklist.md](qa/manual-checklist.md) (only the sections this change touches), results added to its run log
- [ ] Tests are minimal and not redundant (§4). No benchmark, soak or security re-runs: those belong to the Phase 6 testing pass
- [ ] No third copy of the same logic, no unexplained lint suppression, no dead or commented-out code, no new dependency or
      config knob without a reason (§11)
- [ ] Docs updated (`plan.md` checkboxes ticked, README if user-facing)
- [ ] **Module docs updated** ([docs/modules/](modules/README.md)): every module doc for a path you touched is still true, with a
      one-line *Change log* entry (new/renamed file, public type, command, event, config field, thread, timer or token; changed flow)

## 11. Code hygiene: duplication, cleanliness, bloat

> Added 2026-11-23 at the owner's request. **Priority:** §1 > correctness > performance budgets > §11. The rules here are
> defaults, not absolutes: any of them may be broken with a one-line reason **at the site** (a comment or `reason = "…"`).
> A reason in the PR description alone is not enough, because the next reader only sees the code.

### 11.1 Duplication

- **Duplicated knowledge is never allowed, not even twice.** A fact has exactly one home, and everything else reads it from
  there: config defaults and limits live in `ice-core::config`, IPC payload types are generated (§5), visual values are `--ice-*` tokens
  in `base.css`, and durations and springs come from config presets. A copied constant, validation rule or enum mapping is a bug
  waiting to happen, even if the code around it differs.
- **Duplicated code follows the rule of three.** A second similar block is fine, because two copies don't yet show what the shared
  shape is. When you write the third, extract it in the same PR.
- **Similar is not the same.** Two blocks that look alike but change for different reasons (e.g. the switch-confirm path and the
  re-link path) stay separate. Extract only when a change to one would always have to be made to the other.
- **Shared code lives in the lowest layer that can own it** (§3): pure logic in `ice-core`, Windows/COM wrappers in `ice-win`
  (one helper for COM init, `PROPVARIANT` reads, HSTRING conversion and checked `f64` → pixel casts), and UI helpers in `app/ui/src/lib/`.
  Never move code *up* a layer just to share it, and never create a `utils`/`common`/`helpers` grab-bag. Name the module after what it does.
- **Don't copy from another owner's area.** If you need something that lives in another agent's files, give a heads-up and move it
  to the right shared place (§8 ownership), rather than forking a private copy.

**Allowed duplication (exceptions):**

| Case | Why | Condition |
|---|---|---|
| **Native dot ↔ WebView** presentation logic (volume %, mute/mic glyph choice, theme colour mapping) | D-15: the native dot must work with no WebView alive, so Rust can't call the TS version | Each copy has a `// Twin: <path>` comment pointing at the other, and both are tested with the same input cases |
| **Tests** | Readable tests beat clever ones (DAMP over DRY): each test should show its own fixture | Extract a test helper only when the setup is noise *and* is used by 3+ tests |
| **`unsafe` FFI blocks** | A macro or generic that hides several `unsafe` calls also hides their `SAFETY` reasoning | Each block keeps its own `// SAFETY:` comment. Wrap repeated *safe* parts, not the `unsafe` itself |
| **Across a layer boundary** | Sharing would need a forbidden dependency edge (e.g. `ice-core` → `ice-win`) | Short, and it carries a `// Twin:` comment |
| **`spikes/`** | Throwaway code (Phase 0) | Never imported by `crates/` or `app/`. Promoting spike code means rewriting it to these rules |
| **Generated files** (`ui/src/lib/ipc/types.ts`, `assets/dist/`) | Produced by tools | Not reviewed for duplication. Fix the generator, never the output |

### 11.2 Cleanliness

- **Names say what, and carry units:** `fade_ms`, `width_px`, `volume_pct` (0–100) vs `volume_scalar` (0.0–1.0), `peak_db`.
  Core Audio mixes scalars and percentages, and a missing unit is how off-by-100 bugs happen.
- **Size limits (soft; they trigger a split or a reason):**

  | Unit | Limit | Counted as |
  |---|---|---|
  | Function / Svelte `<script>` block | ~60 lines, nesting ≤ 3 levels | Excluding comments and blank lines |
  | Source file | ~600 lines | Excluding `#[cfg(test)]` modules and `*.test.ts` |
  | Svelte component | ~300 lines | Whole file. One activity = one component (§5) |

  `clippy::too_many_lines` and `too_many_arguments` count as hitting the limit. Suppressing them needs a reason (below).
- **Lint suppressions are explained and self-expiring.** In new code, use `#[expect(lint, reason = "…")]`, not `#[allow]`, so the
  compiler flags the suppression once it's no longer needed. TS: `// @ts-expect-error <reason>`, never `@ts-ignore`; `any` already
  needs `// reason:` (§5). Existing `#[allow]`s are converted when their function is next touched.
- **Comments explain *why*, not *what*.** Cite decision, feature and QA IDs (`D-16`, `F31`, `QA B4`) so the reasoning can be traced.
  Delete comments that only repeat the code.
- **No dead code.** No commented-out code (Git keeps history), no `#[allow(dead_code)]` without a reason, and no unused exports, props, CSS
  classes or `--ice-*` tokens. Delete them when you notice them.
- **TODOs are owned:** `// TODO(<agent>, <F-xx|D-xx|plan item>): …`. A TODO without an owner and a tracked item is rejected.
  Anything bigger than a line goes into `plan.md`, not into a comment.
- **One pattern per job.** Follow the pattern the module already uses (error type, channel style, store shape). If you
  introduce a better one, migrate the old uses in the same PR or a follow-up `refactor:` PR named in the description. Never leave both patterns
  indefinitely.
- **Formatting is the tools' job** (`cargo fmt`, the Svelte/TS compiler). Reviews don't debate style the tools already settle.

### 11.3 Bloat

- **Build for the current phase (YAGNI).** No code, config field, trait, generic parameter or option for a feature that isn't in
  `plan.md`'s current or next phase. "Might be useful later" isn't a reason, because Git can bring it back.
- **An abstraction must pay for itself:** a new trait, generic, macro or builder needs **≥ 2 real users**. The test seams in §4
  (`Engine`, `AudioBackend`, `MediaBackend`) qualify because the fake is a real user. A trait with one impl and no fake is a struct.
- **New knobs need a request.** Guiding constraint 2 says knobs live in config, not that every number becomes a knob. A new config
  field or `--ice-*` token needs an F-xx, a D-xx or an owner request. Otherwise it's a named constant in the module that owns it.
- **Deleting is part of the feature.** When a feature is cut or replaced (e.g. D-05.F05, D-18 Part B), remove its code, config
  fields (with a schema migration, §7), tokens, docs and tests in the same PR. Never leave it hidden behind a flag.
- **Experimental flags have an exit:** each one (e.g. per-app routing, F25) names the condition for removing the flag, either by
  promoting the feature or by deleting it.
- **Dependencies and features are pruned, not only gated.** The §2 dependency budget covers additions, and removal matters too: drop a
  crate, npm package or `windows` crate feature in the same PR that stops using it.
- **Size is checked in the Phase 6 testing pass** (release exe, installer, UI bundle). Until then, the one-line dependency
  justification (§2) is enough. Installer and RAM gates stay in [plan.md](plan.md#performance-budgets-gates).

### 11.4 How it is enforced

- **Automatic:** fmt, clippy `-D warnings` (incl. `too_many_lines`), `tsc --noEmit`, tests. A rule that a tool can check
  should eventually be checked by a tool. Propose the lint in a small PR.
- **Author:** runs through the §10 checklist before opening the PR. **Reviewer** (when §8 requires one) checks §1 first, then §11.
  A §11 finding blocks a merge only when it's a third copy, duplicated knowledge, dead code, or an unexplained suppression. Anything
  else is a suggestion the author may decline with a reason.
- **Refactors are their own PRs:** behaviour-preserving, branch `phase-<n>/refactor-<name>`, `refactor(<scope>): …` commits, tests
  green before and after, and no feature changes mixed in. Don't refactor unrelated code inside a feature PR. If a feature needs
  to grow a file that's over the limit, split the file first in a `refactor:` PR, then add the feature.
- **Known debt is tracked**, not hidden: a file or module over the limits, or a taken shortcut, gets a line in
  [plan.md → Cleanup backlog](plan.md#cleanup-backlog) with its owner. Shortcuts also get
  `// DEBT(<agent>): <what> — <exit condition>` at the site.
- **Every phase ends with a cleanup pass** by each owner over their own paths: dead code, over-limit files, third copies, unused deps and
  `windows` features, stale `#[allow]`s. It is part of the phase's exit criteria, and the backlog shrinks or each remaining item is re-justified.
