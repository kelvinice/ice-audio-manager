# Ice Audio Manager: Planning Discussion

> **Status:** D-01–D-26 and D-28 closed (entries in [archive/decisions-closed.md](archive/decisions-closed.md)); D-27 partly open · **Opened:** 2026-11-22 · **Moderator:** the owner (kelvinice)
> **Participants:** Claude (Anthropic), Gemini (Antigravity), Codex (OpenAI)
>
> Companion docs: [plan.md](plan.md) (phased implementation plan) ·
> [code-practices.md](code-practices.md) (coding standards)

This is where the three agents debate and vote on how to rebuild Ice Audio Manager.
Nothing here is final until it appears in the **Decision Log** at the bottom.

---

## 0. How this discussion works

### 0.1 Rules for agents

1. **Write only in your own sections.** Each decision has a `Positions` block with one
   sub-heading per agent. Do not edit another agent's text; quote it and reply in yours.
2. **Back claims with evidence.** If you cite a number (RAM, binary size, fps), say whether
   you measured it, sourced it (link), or estimated it. Estimates are fine but must be labelled.
3. **Vote by score** on every option of every decision. See [§0.2 Voting](#02-voting). From D-19 on, votes are
   **sealed until all three are in**: see [§0.4 Blind voting](#04-blind-voting-owner-decision-2026-11-23-applies-from-d-19-on).
4. **Keep reasons short in the table** (one line). Longer arguments go in your Positions section.
5. **Need something?** Add it to [§6 Requests to the owner](#3-requests-to-the-owner)
   (clone a repo, install a tool, answer a question, generate an asset).
6. When a decision closes, the moderator (or Claude, if asked) copies the result to the
   **Decision Log** and updates `plan.md` / `code-practices.md` to match.

### 0.2 Voting

We use **score voting**. Each agent scores **every** option of a decision, not just its favourite.

| Score         | Meaning                                                                                  |
| ------------- | ---------------------------------------------------------------------------------------- |
| **+2**  | Strongly prefer: I'd argue for this                                                      |
| **+1**  | Good: happy with it                                                                      |
| **0**   | Neutral, or abstain (outside my expertise). Still counts as a vote                       |
| **−1** | Prefer not, but I can live with it                                                       |
| **−2** | **Block:** I think this is a real mistake. **A concrete reason is required** |

**Decision types.** Each vote table is labelled with its type:

| Type                | Table shape           | Passes when                                                                                                                                           |
| ------------------- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pick one**  | One column per option | The option with the **highest total** wins, provided it has no unresolved −2                                                                   |
| **Pick many** | One column per option | **Every** option with total **≥ +2** and no unresolved −2 is adopted (used where options combine, e.g. signing routes, package formats) |
| **Proposal**  | One `Score` column  | Passes if total **≥ +2** and no unresolved −2. Otherwise it goes back for revision                                                            |

**Rules:**

1. **Quorum:** a decision closes only when all three agents have scored every option.
   You may change your scores until the decision closes.
2. **Blocks (−2):** a −2 opens **one rebuttal round**. The other agents answer the objection in
   their Positions sections. The blocker then either withdraws (changes the score) or keeps the block,
   and **the owner rules**.
3. **Ties:** the owner decides.
4. **The owner** doesn't score, but can add options at any time, break ties, rule on blocks,
   veto any result, and closes each decision.
5. **Late options:** if an option is added after voting started, every agent must score it before the decision closes.
6. **Recording:** when a decision closes, the result and its total (e.g. `A (+5)`) go into the Decision Log.

**Example (pick one):**

| Agent           | Option X     | Option Y | Option Z           |
| --------------- | ------------ | -------- | ------------------ |
| Agent 1         | +2           | +1       | −2 *(reason…)* |
| Agent 2         | +1           | +2       | −1                |
| Agent 3         | +2           | 0        | −1                |
| **Total** | **+5** | +3       | −4                |

X wins. Z's block needs no rebuttal because Z lost on points anyway.

### 0.3 Sessions

Sessions S1–S5 and every later session are closed; their entries are in the
[archive](archive/decisions-closed.md). A new decision gets a `D-xx` entry under [Open decisions](#2-open-decisions)
and moves to the archive once the owner closes it. Its result stays in the [Decision Log](#4-decision-log).

### 0.4 Blind voting (owner decision, 2026-11-23; applies from D-19 on)

**Why:** almost every decision through D-18 closed at +6 with identical scores. The owner noticed the agents echoing each
other. The causes are structural: the proposer published its scores and verdicts inside the entry ("not recommended",
"listed to be scored down"), everyone else read them before voting, and nothing asked anyone to disagree.

1. **Neutral entries.** Whoever opens a decision writes the problem, the facts (labelled measured / sourced / estimated,
   §0.1) and the options, each with its costs and risks. **No recommendation, no verdict words, no scores and no Position
   in the entry.** Rule constraints are stated as facts ("polls while shown; §2 requires a stop condition"), not as
   conclusions.
2. **Sealed votes.** Each agent writes its scores and Position in `docs/votes/D-XX/<agent>.md` (see
   [docs/votes/README.md](votes/README.md)), commits it on its own branch and merges it. **Until the reveal, don't open
   another agent's vote file for that decision** (nor its git history or diff). The proposer is bound the same way and may
   seal its vote at proposal time.
3. **Every vote file contains:**
   - a score for every option, with a one-line reason;
   - **the strongest argument against your own top choice**;
   - **one new option or amendment**, or one sentence on why none is needed;
   - **evidence you checked yourself**: a file/line, a measurement, a test or a link. Restating the entry isn't evidence.
4. **Reveal.** When all three files exist, any agent copies the scores into the entry's vote tables and adds a short summary
   of the Positions, with links to the files. Then **one revision round**: an agent may change a score with a reason, and
   the table marks it "changed after reveal".
5. **The Decision Log records the split**, not just the total, e.g. `P2 (+5: Claude +2, Gemini +2, Codex +1)`. It also
   notes scores changed after the reveal, and votes cast before this rule (D-19 to D-21 are marked *anchored*).
6. **Late options** are scored sealed too, in an addendum to your vote file.

---

## 1. Context

### 1.1 Goal (from the owner)

Remake Ice Audio Manager into an audio manager that is **as great and as cool as possible**:

- Fast audio device switching (like AudioSwitcher)
- A **Dynamic-Island-style notch** with animations for volume changes, track changes, and
  device switches (like DynamicWin)
- **As lightweight and simple as possible**, while being **as customizable as possible**
- **Windows only**
- Must **not** end up looking like malware (DynamicWin is widely suspected; see §1.3)

### 1.2 References (local, in `docs/reference/`, gitignored)

| Ref                        | What it is                                                                                                    | Language / license                                                        | What we take from it                                                                                                                                                                                                                  |
| -------------------------- | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **AudioSwitcher**    | .NET library over Windows Core Audio (MMDevice API): devices, volume, sessions, peak meters, `PolicyConfig` | C# · **Ms-PL**                                                     | The **API map**: which COM interfaces to use, the threading pitfalls (see `ComInteropTimeoutException`, `Threading/`), session handling, `PeakValueTimer`                                                                 |
| **AudioSwitcher v1** | The classic WinForms tray switcher (screenshots in `Screenshots/`)                                          | C# · **Ms-PL**                                                     | The **feature baseline**: tray quick-switch menu, per-device hotkeys, favourites cycling, "dual switch mode" (set both default + communications), show disabled/disconnected devices, close to tray, start minimized, autostart |
| **DynamicWin**       | Dynamic Island for Windows: media controller, BT battery, file tray, calendar                                 | C# (FenUI, Skia) · **CC BY-SA 4.0** · **no source in repo** | **Concept only**: the pill at the top centre, activity system, "spring notches", scroll-to-switch views. We copy **no assets or code** (there is none, and the GIF is CC BY-SA)                                           |

> If code is ported from AudioSwitcher, Ms-PL requires keeping its copyright, patent,
> trademark and attribution notices in the ported files. Prefer re-deriving from the
> documented Windows APIs over line-by-line porting.

**Added 2026-11-22 (R-01 to R-04, shallow clones):**

| Ref                                            | What it is                                                                      | Language / license                                            | What we take from it                                                                                                                                                                                                                                                    |
| ---------------------------------------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **EarTrumpet**                           | The best-known Win11 per-app volume mixer (tray flyout)                         | C# · **MIT** (with a short list of excluded companies) | Per-app sessions, per-app routing (`IAudioPolicyConfigFactory`), tray icon handling (`UI/Helpers/ShellNotifyIcon.cs`), **hook-free tray-icon scrolling** via raw input registered only while the cursor is over the icon (`Interop/Helpers/InputHelper.cs`) |
| **ModernFlyouts**                        | Replacement for the native volume/media flyouts (archived, last commit 2024-11) | C# · **MIT**                                           | Prior art for D-07 B (suppressing the native flyout) and the GSMTC media card                                                                                                                                                                                           |
| **boring.notch**                         | macOS notch app                                                                 | Swift · **GPL-3.0**                                    | **Motion study only.** Watching it is fine; copying code or assets is not (GPL would apply to our app)                                                                                                                                                            |
| **windows-rs** (`crates/samples` only) | Official Rust bindings samples                                                  | Rust · MIT/Apache-2.0                                        | COM/WinRT patterns (`#[implement]` callbacks, async WinRT)                                                                                                                                                                                                            |

**Added 2026-11-23 (owner references; screenshots in `docs/reference/screens/`):**

| Ref | What it is | Language / license | What we take from it |
|---|---|---|---|
| **Auralis (Microsoft Store widget)** | Compact media widget for Windows: art, title, scrubber, transport, a side rail (pin, power, modules, volume), colour themes, "nine anchors" with a vertical card on side edges | Unknown (Store page not readable; publisher unconfirmed) | **Screenshots only.** Layout, theming and anchor ideas: D-20 review and late option A2b |
| **Auralis (GitHub, `matiaszanolli/Auralis`)** | A *different* product with the same name: Python + Electron music library with audio "mastering" | Python/JS · **AGPL-3.0** + commercial | Nothing directly. Cloned to `docs/reference/Auralis` to check. **Look, don't copy** (AGPL) |
| **NotchBox** | macOS notch app: media card, date/calendar | Unknown (App Store) · screenshot only | The empty-state wording for "nothing playing" |
| **FluentFlyout** (`unchihugo/FluentFlyout`) | Open-source Windows media flyout + taskbar widget with a visualizer | Not cloned; licence not checked | The closest Windows peer. Clone if we need prior art for the taskbar/visualizer |
| **BitChord** (`kushagrasinghx/BitChord`) | Android YouTube Music client (Kotlin), plus a Go backend and a web app: InnerTube playback, lossless "module sources", Spotify playlist import, word-synced lyrics from 12 providers, Discord Rich Presence, scrobbling, sleep timer, "stats for nerds" | Kotlin/Go/TS · **GPL-3.0** | **Ideas only** (GPL): lyrics UX, the provider fallback chain, small audio-manager features (sleep timer, output format readout). Cloned to `docs/reference/BitChord` |
| **Echo-Music** (`EchoMusicApp/Echo-Music`) | Android YouTube Music client (Kotlin): InnerTube playback, offline downloads, synced lyrics (LRCLIB, KuGou, BetterLyrics, SimpMusic, …), Shazam-style recognition, Genius "Music DNA" | Kotlin · **GPL-3.0** | **Ideas only** (GPL): how it queries LRCLIB (`lrclib/.../LrcLib.kt`: search, no key, duration ±2 s → ±5 s). Note it ships Firebase Analytics/Crashlytics, which our §1.5 forbids. Cloned to `docs/reference/Echo-Music` |
| **Auralys** (Android, v4.1.0) | Android music app: "listen together" rooms with a session code (real-time shared playback), two switchable UI styles ("Editorial" and "Liquid Glass"), song share cards for stories | Unknown · **no public source found** | **Screenshot only** (`docs/reference/screens/auralys-android-promo-v4.1.0.png`). Style as a user choice, separate from colour themes (D-23). Listen-together needs a server, which is outside D-05 and §1.5 |

*§1.3 (why DynamicWin is called malware) and §1.4 (the replaced Python codebase) are in the [archive](archive/decisions-closed.md#context-history).*

### 1.5 Owner constraints (from §7.1 answers)

| Topic                  | Owner answer                                                                 | What it means for us                                                                                                                                                                                                                                                                                        |
| ---------------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows version (Q-01) | **Win11 only**                                                         | D-08 closed. Mica, Segoe Fluent Icons, Segoe UI Variable, and Win11 DWM corners can be assumed                                                                                                                                                                                                              |
| Open source (Q-02)     | Private use first; open-source if it costs little                            | See Claude's recommendation below                                                                                                                                                                                                                                                                           |
| Rewrite (Q-03)         | Anything can change, even the name                                           | Phase 0.7 can delete the Python app                                                                                                                                                                                                                                                                         |
| Budget (Q-04)          | **$0**, has a Microsoft account                                        | Only Store (free individual registration, verify) and SignPath (needs OSS) remain viable signing routes                                                                                                                                                                                                     |
| Name (Q-05)            | Keep **Ice Audio Manager**; likes the **snowflake**              | Snowflake mark = app/tray icon basis                                                                                                                                                                                                                                                                        |
| Toolchain (Q-08)       | Rust installed                                                               | Verified by Claude: `rustc 1.100.0`, `stable-x86_64-pc-windows-msvc`, VS 2022 C++ tools present                                                                                                                                                                                                         |
| Idle RAM (Q-06)        | **> 100 MB is too heavy**; lower still when unused or gaming           | Hard limit 100 MB for the whole process tree. Add a **gaming/unused mode** that releases the notch WebView entirely (see plan.md F13)                                                                                                                                                                 |
| Hardware (Q-07)        | Desktop PC, **2 monitors**, RTX 3050, Ryzen 5 3600X, 16 GB             | Multi-monitor is the main case, not an edge case. Notch must be **pinnable to one specific screen** (F14 "fixed" becomes the primary mode)                                                                                                                                                            |
| Media apps (Q-09)      | **Spotify**                                                            | Spotify desktop is the primary GSMTC QA target in Phase 3                                                                                                                                                                                                                                                   |
| Headset (Q-12)         | No Sonar; switches **Headphones (Arctis Nova 7) ↔ Speakers (Realtek)**; wants **spatial sound** on the headset | Primary toggle-pair test case (D-16). Spatial sound on switch is D-17 |
| Input habits (2026-11-22) | Keyboard-first, with a **volume knob** on the keyboard; sometimes changes volume with the **mouse** (on another PC) | Knob turns arrive as normal volume keys, so the notch animates with no hook. Proposed **F31** knob + modifier gestures. Mouse: notch scroll (F12) covers it; the owner's mouse use is a data point if F05 (tray scroll, cut) is ever revisited |
| Idle state (Q-10)      | Keep all idle modes as options; **default = small (dot or mini pill)** | Default is visible, not hidden. This overrides the "`hidden` (default)" wording in Gemini's F21 note. Dot vs mini pill is picked from Codex mockups in Phase 2                                                                                                                                            |
| Displays (Q-11)        | **144 Hz primary, 75 Hz secondary**, not OLED                          | Springs must be `dt`-independent (Gemini's D-13.1 already covers this) and feel identical when the notch moves between 144 Hz and 75 Hz screens. Animation target = display refresh rate, not a fixed 60 fps. No OLED burn-in constraint for the owner, but keep static indicators subtle for other users |

**Claude's recommendation for Q-02:** keep the repo **private while it's for personal use**.
Binaries you build yourself never carry the "downloaded from the internet" mark (Mark-of-the-Web),
so **SmartScreen never triggers on your own builds and nothing needs signing**. Go public under
**MIT** (already named in README) at the first public release, now Phase 6 (owner, 2026-11-22). At that point the cost to
you is only that the code is visible. It also gets you SignPath's free signing and a portfolio
piece. Until then, D-09/D-10 votes describe the *eventual* release path and block nothing.

### 1.6 Brand (from the owner's portfolio, kelvinice.github.io)

The owner's personal brand already has an ice identity: a **white snowflake on a blue rounded square**
mark, and these CSS tokens (from the site's stylesheet):

| Token                                    | Light                                                       | Dark                      |
| ---------------------------------------- | ----------------------------------------------------------- | ------------------------- |
| `--accent`                             | `#026aa2`                                                 | `#6bc5ff`               |
| `--accent-strong`                      | `#015580`                                                 | `#9ad6ff`               |
| `--accent-soft`                        | `#e6f3fb`                                                 | `#13283c`               |
| `--bg` / `--surface`                 | `#f6f7f9` / `#ffffff`                                   | `#0e1116` / `#171d27` |
| `--ink` / `--muted`                  | `#11161f` / `#4b5668`                                   | `#edf1f7` / `#98a4b8` |
| `--line`                               | `#d6dde6`                                                 | `#263041`               |
| `--ok` / `--signal` (warning/danger) | `#15803d` / `#c94624`                                   | `#4ade80` / `#f07148` |
| Radii                                    | 4 / 6 / 8 / 12 / 16 / 24 px                                 | same                      |
| Fonts                                    | IBM Plex Sans, IBM Plex Mono, Bricolage Grotesque (display) | same                      |

Proposal: the `Ice` theme's defaults derive from these tokens, so the app and the owner's portfolio
feel like one family. See D-12.

---

## 2. Open decisions

Closed entries (D-01–D-26, D-28) are in [archive/decisions-closed.md](archive/decisions-closed.md).

### D-27 · Hibernate mode: a plain notch for meetings and screen sharing *(added 2026-11-25, owner request; neutral entry under §0.4; open, for later)*

**Owner request (2026-11-25):** *"Hibernate mode. The reason is serious time / meeting time: we don't want to be
distracted, or want someone to see our last played song when we share the screen. Make it look like just a normal notch
that has a way to wake up again."* The owner asked for this to be **opened for later**, not built now.

**Facts: what the notch shows today when nothing is happening (checked in the code 2026-11-25):**
- The idle pill (WebView `IdlePill.svelte`, native `crates/ice-win/src/native_dot.rs`) shows volume, mute, mic state and,
  while media plays, the track (art/title or a visualizer). Even with the now-playing pill set to **Off**
  (`appearance.now_playing`, `crates/ice-core/src/appearance.rs`), the idle pill still shows the title.
- A track change can pop up the music pill (art, title, artist, waveform) and, if lyrics are on, the current line
  (`TrackActivity.svelte`, `LyricsExtension.svelte`). Hovering the pill opens the expanded card (art, title, lyrics).
- `idle_style = hidden` (`IdleStyle::Hidden`) hides the idle pill, but activities still pop up on events (volume, track).
- There is no "do not disturb" state, no hotkey for it, and nothing that hides the notch from screen capture.
- **Screen capture:** Windows has a documented, user-mode API, `SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)`
  (Windows 10 2004+), that leaves a window out of screen captures (desktop duplication, Windows.Graphics.Capture, BitBlt)
  while it stays visible to the user. It is not used in the code today (`grep DisplayAffinity` = 0 hits). Untested here:
  how Teams, Zoom, Meet in Chrome/Edge, OBS and Remote Desktop treat it. Each must be checked on the owner's PC before
  we rely on it.
- **Detecting a meeting/share:** there is no public Windows API that reports "the screen is being shared".
  `SHQueryUserNotificationState` (already used in `fullscreen.rs`) reports presentation mode and full-screen apps only.
  Windows 11's "Do not disturb" state has no documented read API. Automatic entry would therefore be a guess. Any
  detection must stay within code-practices §1 (no hooks, no injection, no reading other apps' windows or content).

**Options.**

*Part A · How the notch looks while hibernating (pick one).*
- **A1 · Plain notch:** the idle silhouette only, in a neutral dark fill: no text, glyphs, art, accent colour, visualizer
  or lyrics. On Edge Nook it reads as a hardware notch (flush top, ears); on Floating, a small dark pill. Same shape on the
  WebView and the native pill.
- **A2 · Plain notch + mic dot:** A1 plus one small red dot when the microphone is muted (useful in a meeting, shows no
  media). Nothing else.
- **A3 · Fully hidden:** nothing on screen; wake only by hotkey or tray.

*Part B · What is suppressed (pick one).*
- **B1 · All content:** no activities at all (volume, switch, mic, track, lyrics pop-ups), no hover-expand. Volume keys
  still change the volume; the notch just doesn't show it.
- **B2 · Media only:** track/lyrics/art/visualizer/now-playing are suppressed; volume, mic and device-switch pop-ups still
  show, without any track info.

*Part C · Screen-capture privacy (independent, pick any).*
- **C1 · Exclude from capture while hibernating:** set `WDA_EXCLUDEFROMCAPTURE` on the notch and native pill windows
  while hibernate is on, so a screen share doesn't show the notch at all, even if it wakes.
- **C2 · Exclude from capture always** (a Settings → Privacy switch, off by default).
- **C3 · None:** rely on the plain look only.

*Part D · Entering and waking (pick any).*
- **D1 · Manual:** tray item "Hibernate notch", a hotkey (`RegisterHotKey`, unbound by default) and a Settings switch.
- **D2 · Wake gesture:** a deliberate action on the plain notch wakes it: click (not hover), or a long hover (e.g. 600 ms).
  Waking shows the normal notch; whether it re-hibernates by itself is Part E.
- **D3 · Auto-enter (best effort):** enter while Windows reports presentation mode (`SHQueryUserNotificationState`).
  Never by reading other apps' windows or processes.
- **D4 · Auto-enter while a chosen app runs** (e.g. `Teams.exe`, `Zoom.exe`), reusing the Phase 5 "rules" engine
  (process start/stop, no injection). Waits for Phase 5.

*Part E · Leaving (pick one).*
- **E1 · Until turned off** (persists across restarts, like a setting).
- **E2 · For this session:** cleared on sign-out/restart; also offered as timed (30 min / 1 h / 2 h) from the tray.
- **E3 · Wake is temporary:** after a D2 wake, the notch goes back to hibernate after the idle linger.

**Cost and fit (for voters to check):**
- Hibernate can also **release resources**: in every profile (even Instant) the WebView can `TrySuspend` and the native
  pill can draw the plain shape (Lite-level RAM, < 25 MB target) while hibernating.
- Config: one `[hibernate]` table (schema bump + migration test), `Command::SetHibernate`, and an `EngineEvent` so the tray,
  native pill and UI agree. The native pill needs an `IdleContent::Plain` state (ties in with D-19's `IdleContent`).
- `WDA_EXCLUDEFROMCAPTURE` is a documented `SetWindowDisplayAffinity` call in `ice-win` (one `unsafe` block with a
  `// SAFETY:` note). It fails safely on older builds, and we only support Windows 11 (D-08).
- Edge Nook first (owner, 2026-11-25): the plain look is designed and QA'd on Edge Nook, then Floating.

**Questions to the owner:**
1. In a meeting, do you still want to see mic-mute and volume feedback (A2/B2), or nothing at all (A1/B1)?
2. Should the notch be invisible to screen shares (C1/C2), or is the plain look enough?
3. Wake with a click, a long hover, or only the hotkey/tray?

**Votes:** not opened yet. Opened when the owner schedules it (after the Edge Nook track is stable).

**Update (owner, 2026-11-25): built now with Claude's recommendations** ("add it in Settings with your recommendations;
users can still wake it from the tray icon menu"). Shipped: **A2** (plain black silhouette + red mic-muted dot), **B1**
(no activities, no hover-expand; the plain notch is fully click-through), **C1** (out of screen captures while
hibernating, its own switch, on by default), **D1** without the hotkey (Settings → Notch → Hibernate, tray "Hibernate
notch"), **E1** (saved, survives restarts). Not built: a hotkey, timed hibernate (E2), presentation-mode auto-entry (D3),
per-app rules (D4), releasing the WebView while hibernating. The owner can still revise any part; a vote is only needed
for the open ones.

**Revision (owner, 2026-11-25):** E1 → **E2 without the timer**: every start is awake (session state; a hot reload keeps
the live value). Hibernate has its own Settings page, and A3 was added as a choice next to A2 ("Plain notch" / "Hide the
notch"). Balanced hands the hibernating notch to the native pill after 1 s.

---

## 3. Requests to the owner

Questions Q-01–Q-12 are answered and resource requests R-01–R-05 are done; both tables are in the
[archive](archive/decisions-closed.md#answered-requests). Their answers live on in §1.5.

### 3.1 Asset requests for Codex (after D-12 closes)

- App icon (SVG master → `.ico` 16/20/24/32/48/64/256), light & dark taskbar variants.
  **Base it on the owner's snowflake mark** (white six-arm snowflake on a `#026aa2` rounded square, see §1.6). Refine it
  for 16 px legibility rather than inventing a new symbol
- Tray icon states: default, muted, mic-muted, switching (monochrome, 16/20/24/32 px)
- Device-type glyphs not in Segoe Fluent Icons (e.g. HDMI/monitor speaker, USB DAC, virtual cable)
- Notch mockups for each activity (idle, volume, switch, mic-mute, now-playing, expanded) at 100% & 150% scale
- Store listing (Phase 6): 1:1 logo, hero banner, 4 screenshots
- *(optional)* soft "switch" chime (`.wav`, < 30 KB), disabled by default

---

## 4. Decision Log

Record the result with its total, e.g. `A Tauri (+5)`, and note any block and how it was resolved.
A decision passes when all three agents have voted and the rules produce a result. The owner then closes it (or vetoes it).
Full entries of every closed decision: [archive/decisions-closed.md](archive/decisions-closed.md). **D-01 to D-17 were signed off by the owner on 2026-11-22** ("I accept the roles, I also accept the decisions"). **D-18 was accepted by the owner on 2026-11-23.**

| ID   | Decision                     | Result (total)                                                                                             | Status / closed on              | Notes                                                                                                          |
| ---- | ---------------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| D-01 | Language / UI tech           | **A: Rust + Tauri 2 (+6)**; fallback D native notch (+3)                                             | **Closed 2026-11-22 (owner)** | Gated by Phase 0.2 measurements + D-15                                                                         |
| D-02 | UI framework                 | **Svelte 5 + strict TS (+6)**                                                                        | **Closed 2026-11-22 (owner)** | Custom `(k, c, m)` spring module, not `svelte/motion`                                                      |
| D-03 | Architecture / layout        | **Workspace as proposed (+6)**                                                                       | **Closed 2026-11-22 (owner)** | + typed IPC codegen, snapshot/delta with revisions, `iceart://`, event names `…/v1` (Tauri rejects `@`) |
| D-04 | Config & theme format        | **TOML + CSS (+6)**                                                                                  | **Closed 2026-11-22 (owner)** | Strict CSP, validated `--ice-*` tokens, keep last valid config on bad reload                                 |
| D-05 | Feature inventory & phases   | **Accepted as listed**, with F05 tray scroll cut (+3), F18 mixer → Phase 5 (+6), F21 clock cut (+5) | **Closed 2026-11-22 (owner)** | + Codex's F02 role semantics and F13/F14 pinned-display rules                                                  |
| D-06 | MVP cut                      | **Option 1: Phases 1–2 (+6)**                                                                       | **Closed 2026-11-22 (owner)** | Phases 1–2 are for private daily use. The owner moved the first public release to Phase 6                                               |
| D-07 | Native volume flyout         | **A: Coexist (+6)**                                                                                  | **Closed 2026-11-22 (owner)** | F29 stays research; ModernFlyouts uses an LL hook, so it can't be copied                                       |
| D-08 | Windows versions             | **Win11 only** (owner)                                                                               | 2026-11-22                      | Owner decision via Q-01                                                                                        |
| D-09 | Code signing                 | **Store MSIX (+6) + SignPath (+5)**                                                                  | **Closed 2026-11-22 (owner)** | Only matters at public release; private builds need no signing                                                 |
| D-10 | Package format               | **MSIX (+6) + per-user NSIS (+5) + portable zip (+5)**                                               | **Closed 2026-11-22 (owner)** | Portable zip first, for private use                                                                            |
| D-11 | Updates & network            | **Opt-in notify-only check (+5)**                                                                    | **Closed 2026-11-22 (owner)** | Off by default; zero background requests, zero telemetry; verified by network trace                            |
| D-12 | Visual style                 | **Ice default (+6)** + Midnight, Fluent, Minimal bundled                                             | **Closed 2026-11-22 (owner)** | Portfolio palette, snowflake icon, tinted (not blurred) notch, contrast-checked accents                        |
| D-13 | Animation system             | **Springs + D-13.1 motion specs (+6 / +6)**                                                          | **Closed 2026-11-22 (owner)** | Off = instant (no fade); retune Bouncy; QA on 144 Hz + 75 Hz                                                   |
| D-14 | Team roles                   | **As proposed (+6)**                                                                                 | **Closed 2026-11-22 (owner)** |                                                                                                                |
| D-15 | Memory policy & idle modes   | **Profiles Instant/Balanced/Lite, measured gates, 150 MB cap (+5)** | **Closed 2026-11-22 (owner)** | p95 ≤ 250 ms; native pill shows the value ≤ 50 ms |
| D-15.1 | Notch technology (after measurements) | **A: WebView notch + TrySuspend (+6)**; C fallback (+1) | **Closed 2026-11-22 (agents, owner-delegated)** | Budgets in private working set: idle (suspended) ≤ 15 MB, visible ≤ 100 MB; Private Bytes ceiling 200 MB; Settings in its own WebView2 data folder |
| D-16 | Device identity & reconnects | **DeviceRef + Active-only resolver + toggle pair, Phase 1 (+5)** | **Closed 2026-11-22 (owner)** | + StableId, logical-endpoint check, confirm-then-persist |
| D-17 | Spatial sound on switch      | **0.3 spike → experimental per-device action in Phase 2 (+6)** | **Closed 2026-11-22 (owner)** | Read-before-write, un-elevated, Settings deep-link fallback |
| D-18 | Notch device list & settings placement | **A1 favourites drive the notch device row + Part B preferences move to tray → Settings** (Claude +2, Gemini +2, Codex voted) | **Closed 2026-11-23 (owner)** | Codex's vote row lands with its branch. Tasks in plan.md Phase 3 |
| D-19 | Native idle pill | A1 +6 (+2/+2/+2), A2 +3 (+1/+1/+1), A3 −6 (−2/−2/−2), A4 −3 (−1/−1/−1). Part B +3 (+1/+1/+1; Claude changed after reveal from +2) | **Closed 2026-11-23 (owner)** | Accepted with the amendments: A1 click-first (+ drag per D-20), A2 hotkey, B1 `IdleContent`, B2 renderer passing the RAM gate in every idle profile (GDI first), B3 QA. Fix the "MIC OFF" bug now |
| D-20 | Placement and modules | **P1 +5 (+2/+1/+2)** leads P2 +4 (+1/+2/+1), P3 −3. A2 +6 (+2/+2/+2). A2b −3 (−1/−1/−1). R1 +3, R2 +5 (+2/+2/+1), R3 −3, R4 −6 (−2/−2/−2) | **Closed 2026-11-23 (owner ruling)** | Owner: build both P1 and P2; Settings → Notch "Drag & snap" (default) or "Fixed position". Drag via own pointer capture, focus/keyboard release gate. A2 adopted, A2b rejected, R1+R2 adopted, R3+R4 rejected |
| D-21 | User modes and resource caps | Part A 0 (0/+1/−1): **fails as written**. **C0 +6 (+2/+2/+2)**, C1 +1 (0/+1/0), C2 −6 | **Part B closed 2026-11-23: C0**; Part A → D-21.1 | Every cap stays. Rich bundle failed (0); its pieces are re-scored in D-21.1 |
| D-21.1 | What replaces the Rich profile | L1 +5 (+2/+2/+1), L2 +5 (+2/+1/+2), L3 +3 (+1/+1/+1), L4 +1 (+1/+1/−1), L5 −3 (−1/−1/−1), L6 +3 (+1/+1/+1), in Claude/Gemini/Codex order | **Closed 2026-11-23 (owner)** | Accepted: L1 linger in config, L2 event-driven hit-testing (real-host spike first), L3 session pin (clears on lock/sleep), L6 cost readout (sampled only while the page is open). Rejected: L4, L5 |
| D-22 | YouTube Music playback, lyrics, network use | N0 +1 (−1/+1/+1), N1 +6 (+2/+2/+2), N2 −5 (−1/−2/−2), B1 +6 (+2/+2/+2), B2 −3 (−1/−1/−1), B3 −6 (−2/−2/−2), K0 −1 (−1/0/0), K1 +5 (+2/+2/+1), K2 −2 (0/−1/−1), K3 −6 (−2/−2/−2) | **Closed 2026-11-23 (owner)** | N1 adopted (§1.5 rewritten); B1 as a "Music services" widget (Spotify + YouTube Music), on by default, switchable per service; K1 opt-in with all amendments |
| D-23 | Visual direction: art backdrop, translucency, cleanliness | V1 +3 (+1/+1/+1; Claude changed after reveal), V2 +5 (+2/+2/+1; Claude changed after reveal), V3 +4 (+1/+2/+1), V4 +6 (+2/+2/+2), V5 −5 (−1/−2/−2), V6 +3 (+1/+1/+1) | **Closed 2026-11-23 (owner)** | V2 default, V1 opt-in per theme with luminance scrim, one backdrop per theme, V3, V4 (D-18 row visible), V6 with lyrics; V5 rejected |
| D-24 | Project licence: MIT or GPL-3.0 | G0 +4 (+1/+2/+1), G1 −3 (−1/−1/−1), G2 +5 (+2/+1/+2) | **Closed 2026-11-23 (owner)** | G2: stay MIT; licence changes only on the owner's explicit acceptance; references read, never copied (code-practices §1.8) |
| D-25 | Ambient motion: moving gradient backdrop | A0 −1 (−1/−1/+1), **A1 +6 (+2/+2/+2)**, A2 −3, A3 −3, A4 −2 (0/−1/−1). **B1 +6**, B2 −3 (−1/−1/−1; Claude changed after reveal from +1), B3 −5 (−1/−2/−2, Gemini block). **C1 +6, C2 +6, C3 +6**, C4 −2 (0/−1/−1), C5 −3. **D1 +5 (+1/+2/+2)**, D2 −1 (0/0/−1), D3 +4 (+2/+1/+1) | **Closed 2026-11-24 (owner)** | Accepted: A1 on the expanded card only, while playing, stopped when hidden, with an off switch and one tuned speed (`--ice-ambient-period`, `0s` in Minimal). Amendments: compositor teardown, scrim anchored, shadow filter off the subtree, 144 Hz measurement gate with a 30 fps fallback. Moved to plan.md |
| D-26 | Visualizer spectrum bands and bar dynamics | **A2 +6 (+2/+2/+2)** with **A1 +3** fallback; **B1 +4 (+2/+2/0)**, **B2 +5 (+1/+2/+2)**, **B4a +5 (+1/+2/+2)**; B3 −3 (−1/−1/−1; Gemini changed after reveal from +2); B4b +1 (0/0/+1) | **Closed 2026-11-24 (owner)** | Accepted with all amendments: 4th-order `f32` IIR bands on existing 30 Hz `ice-meter` tick, separate `MeterFrame.bands` in `differs`, rebind reset, low-band beat accent, `B1` audible-energy gate, §1 audio-stream rule, Settings → Privacy disclosure + meter-only switch, Win11 mic-indicator & CPU QA gates. Moved to plan.md |
| D-27 | Hibernate mode: plain notch for meetings / screen sharing | Not voted; owner asked for Claude's recommendations | **Built 2026-11-25 (owner request); revised the same day (always start awake, plain or hidden look); hotkey, timer, auto-entry open** | Options A1–A3 look, B1–B2 suppression, C1–C3 capture exclusion (`WDA_EXCLUDEFROMCAPTURE`), D1–D4 entry/wake, E1–E3 exit; 3 owner questions |
| D-28 | Volume vs track progress in the notch | Not voted to the end; owner ruled on Claude's sealed recommendation | **Closed 2026-11-25 (owner ruling), built the same day** | A2 song block + output strip (chip fallback), B1 two looks, B2 no jump on click, C1 % leads the pop-up. Not built: A1, A3 default, A4, B3–B5. Entry: [archive](archive/decisions-closed.md#d-28--volume-vs-track-progress-in-the-notch-what-the-user-sees-and-touches-added-2026-11-25-owner-request-neutral-entry-under-04) |
