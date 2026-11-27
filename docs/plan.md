# Ice Audio Manager: Implementation Plan

> **Status: APPROVED.** D-01 to D-14 signed off by the owner on 2026-11-22 (see the Decision Log in
> [planning-discussion.md](planning-discussion.md)). Stack: **Rust + Tauri 2 + Svelte 5, TOML + CSS** (D-01 to D-04).
> D-15 (memory policy), D-16 (device identity) and D-17 (spatial sound) were also signed off on 2026-11-22.
> The owner deferred the first public release to [Phase 6](#phase-6-hardening-and-first-public-release).
> **Priority (owner, 2026-11-25): Edge Nook first.** Edge Nook is the default style and gets polish and QA first; Floating-only
> work waits until Edge is stable (see [Edge Nook track](#edge-nook-track-priority-from-2026-11-25)).
> Coding rules live in [code-practices.md](code-practices.md). **How each module works: [docs/modules/](modules/README.md)** (read first, update with every PR).

## Guiding constraints

1. **Lightweight:** budgets below are hard gates. The Phase 0–3 measurements stand for now; the full benchmark suite runs
   again once, in the **Phase 6 testing pass** (owner decision, 2026-11-23). Meanwhile the design rules in code-practices §2
   keep the app within them.
2. **Customizable:** every visual and behavioural knob lives in config/theme, not in code.
3. **Trustworthy:** no LL hooks, no admin, no packers, no silent network, open source at release, signed.
4. **Windows 11 only** (D-08): use native APIs directly. No cross-platform abstractions "just in case".

### Performance budgets (gates)

| Metric | Target | Hard limit | How measured |
|---|---|---|---|
| Idle CPU (no activity) | ~0.0% | < 0.2% avg over 60 s | Release build, 60 s sample after settling |
| Idle RAM, default profile (whole process tree, Settings closed) | **≤ 100 MB** (owner, Q-06) | **150 MB** (owner allowance, D-15) | Private Bytes summed over the host + all app-owned WebView2 processes (dedicated user-data folder, `GetProcessInfos`). Since 2026-11-24 the default (`Instant`) keeps the WebView awake at idle |
| Idle RAM, `Balanced` profile (WebView released, native dot) | < 30 MB | < 40 MB | Same |
| RAM, `Lite` / gaming mode | < 25 MB | < 40 MB | Same, while a fullscreen game runs |
| Event → first visible feedback | < 1 frame (native dot reacts) | 50 ms | Log timestamps + screen capture |
| Event → full notch visible (`Balanced`, cold WebView) | ≤ 250 ms | 500 ms | Same |
| Notch animation | Display refresh rate (owner: **144 Hz** primary, **75 Hz** secondary) | Never below 60 fps; no dropped frames at 150% scaling | WebView2 DevTools perf trace on both monitors |
| Cold start → tray ready | < 1 s | < 2 s | Log timestamps |
| Device switch latency (hotkey → default changed) | < 150 ms | < 400 ms | Log timestamps |
| Installer size | < 10 MB | < 20 MB | CI artifact |

---

## History

Finished and superseded parts of this plan (Phase 0: Spikes (de-risk before building); Parallel work: tracks, branches and file ownership (from 2026-11-22), and every ticked item) are kept verbatim in
[archive/plan-done.md](archive/plan-done.md), under the same headings.

---

## Phase 1: Engine + tray switcher (parity with AudioSwitcher v1)

**Goal:** a daily-drivable, tray-only switcher that is already better than v1. **Features:** F01–F07.

**Status (2026-11-22):** feature-complete in `main`; the exit criteria (3 days of daily use, QA A–G) are the owner's to run.

- [ ] Explicit portable mode (`portable.flag` next to the exe)
- [ ] **Exit:** owner's 3 days of daily use + QA sections A–G through the tray (B5 sleep/resume still to run)

**Exit criteria:** owner uses it as the daily switcher for 3 days with no crash. Budgets are met.
No hook DLLs or LL hooks are visible in Process Explorer. **The Speakers ↔ Arctis hotkey keeps working after unplugging and
replugging the dongle in a different USB port, power-cycling the headset, restarting SteelSeries GG, and sleep/resume.**

---

## Phase 2: Notch v1 (volume, switch, mic)

**Goal:** add the notch to the private daily-use build (D-06). **Features:** F08–F14.

**Exit criteria:** hardware volume keys, notch scroll and hotkeys all animate correctly. No focus stealing. Hidden over fullscreen
games on the pinned monitor. Budgets met in every profile.

---

## Phase 3: Media (now playing)

**Features:** F15–F17 and F18 (without the mixer). **Primary QA target: Spotify desktop** (Q-09).

- [ ] By hand: compare with Spotify on a few genres (EDM, hip-hop, acoustic, a quiet podcast) on both monitors
- [ ] D-26 spectrum bands (built 2026-11-24, see the archive), host QA gates: the Windows 11 microphone indicator stays
      off, a steady low→high tone moves the active bars and settles cleanly on silence, real-size 5-bar `A2` vs `A1`
      visual check, and visible CPU `< 1%` / hidden `0%`
- [ ] Secondary QA: Chromium/Edge/Firefox media sessions, **including YouTube Music** (the website, and the app installed
      from Chrome/Edge). Owner, 2026-11-23: Spotify and YouTube Music are the supported music services (see Backlog)

**Exit criteria:** track changes animate within 300 ms of the event. Visualizer costs < 1% CPU while visible and 0% while hidden.

---

## Notch interaction and placement (D-19, D-20; accepted 2026-11-23)

Runs between Phase 3 and Phase 4. D-21 Part B (C0) applies: every resource cap stays.

**D-19 · the idle pill (do first):**
- [ ] `ice_core::IdleContent { glyph, label, tone }`: one rule for both pills, unit-tested (Claude). The design owner sets
      the states: mic muted, speaker muted, media playing, resting
- [ ] By hand: native pill focus check (QA F4, `cargo test -p ice-win real_click -- --ignored`), and the wheel with
      "Scroll inactive windows when hovering" on and off
- [ ] By hand: hotkey in Instant / Balanced (asleep) / Lite / over a game; pill click and middle click in Balanced and Lite
- [ ] Side-by-side visual QA of the native and WebView idle pills (all states, light/dark, 100 %/150 %, both monitors)

**D-20 · placement (owner ruling: both methods, drag & snap by default):** *Resumed 2026-11-25 (owner: "continue …
review first if still relevant"; updated 2026-11-25 for flexible markless horizontal placement, bottom Edge Nook, and
landing settle animation). Drag & place and the bottom anchors are built (see the archive); what is left:*
- [ ] **Release gate:** during and after a drag the foreground app keeps focus and keyboard input, on both hosts, across
      150 %/100 % monitors
- [ ] **D-29 (open in planning-discussion.md):** horizontal side-edge anchors (left/right bezels) with drag-and-attach
- [ ] R2 optional modules (Phase 4 Activities): a disabled module runs no timer, subscription or IPC. *Partly there:
      music services, lyrics and the spectrum each have a switch; per-pop-up switches (volume, switch, track) remain*

**D-21.1 (accepted 2026-11-23):**
- [ ] L2 · Event-driven hit-testing for the WebView notch, replacing the cursor poll (Codex).
  - Spike on the real host first: moving edges during spring morphs, mixed DPI, glow/shadow not clipped.
  - If it can't follow the springs cleanly, narrow when the poll runs instead.
- [ ] By hand: pin, lock (Win+L) and unlock → collapsed; pin, sleep and wake → collapsed (QA checklist)
- [ ] L6 · Live cost readout in Settings → General: RAM and CPU of the process tree, sampled only while that page is open
      (Codex, via `ice_win::proc_metrics`)

---

## Music services, lyrics, look and ambient motion (D-22, D-23, D-25)

**D-22 · Lyrics (opt-in, LRCLIB + local `.lrc` + Romaji + Storage cache):**
- Full design spec: [`docs/lyrics.md`](lyrics.md) (bottom pill/card extension, active-word emphasis, Options A & B + Option C constraints, offline Japanese Romaji, Settings → Privacy & Storage max cache size, and phased implementation path).
- **Status, review and fixes:** what was built, a review of further improvements, and the clipped-line fix are in
  [`docs/lyrics.md`](lyrics.md) §7 ([review: improvements not built yet](lyrics.md#7-review-2026-11-24-improvements-found-not-built-yet))
  and §8 ([fixed: last characters cut off](lyrics.md#8-fixed-2026-11-24-the-last-characters-of-a-line-were-cut-off)),
  §9 ([fixed: Romaji never appeared on kanji lines](lyrics.md#9-fixed-2026-11-24-romaji-never-appeared-on-kanji-lines)).
- [ ] Owner: add Japanese in Windows (Language & region) and confirm kanji Romaji appears (lyrics.md §9); if that's not
      acceptable, open a decision on a bundled reading dictionary (size / licence / network trade-offs listed there)
- [ ] Open improvements: lyrics.md §7 (gap countdown, loading state, kanji Romaji without the IME, …)
- [ ] Better word timing on line-synced lyrics: word by word is the default (schema v22, owner 2026-11-26) but the
      estimate still drifts on held notes and tempo changes; until it is accurate, Settings → Activities → Lyric
      highlight offers **Row by row**

**D-25 · Ambient motion on the expanded card (accepted 2026-11-24):**
- [ ] By hand: the new loops, cross-fade and breath with real covers (bright and dark) on both monitors
- [ ] **144 Hz measurement gate before shipping:** (still open; the refresh adds one breath transform per meter frame) 60 s on the 144 Hz monitor (process-tree CPU, GPU engine use, private
      working set vs. static backdrop). If process-tree CPU rises by `> +0.1 %` over static, fall back to a ~30 fps
      JavaScript `transform` driver (`C4`)

---

## Notch refresh (owner request, 2026-11-24)

The owner found the expanded card cluttered, the shadow broken, the visualizer bars ugly, and `Instant` sometimes falling back
to the static native pill. Done by Claude; module docs: [app-ui-notch](modules/app-ui-notch.md), [app-shell](modules/app-shell.md).

- [ ] By hand: skip tracks in Spotify and YouTube Music; the title should change within ~0.3 s (`--verbose` logs "now playing")
- [ ] By hand: each of the three modes with Spotify (skip, pause, resume) in Instant and Balanced
- [ ] By hand: the waveform on light/dark desktops, a live stream (no duration = all lit) and pause/resume
- [ ] Phase 6: measure Instant idle with the WebView awake (expected ≈ 77 MB private WS from Phase 0.2, inside the ≤ 100 MB
      target) and the cursor poll's idle CPU, which now runs for as long as Instant shows the notch. D-21.1 L2
      (event-driven hit-testing) would remove that poll

---

## Edge Nook track (priority from 2026-11-25)

Owner, 2026-11-25: **Edge Nook is the priority; Floating follows once Edge is stable.** "Stable" = the Edge exit
criteria below pass on both monitors. Items come from Claude's review of the Edge layout (2026-11-25).

**Open, fixes (Edge-specific, do first):**
- [ ] By hand (owner's feel check): the Edge Nook hover dwell (220 ms, restarts while the pointer sweeps past; built
      2026-11-26, see the archive) no longer opens the card when reaching for browser tabs, yet still feels quick; tune
      `hoverIntentMs` / `HOVER_PASS_SPEED_PX_PER_MS` from that
- [ ] Side-by-side visual QA of the native and WebView **nook** idle silhouettes (ears, rim, 100 % / 150 %, both monitors)

**Open, polish:**
- [ ] Nook subtitle/Romaji line: wrap it like the current line (today it ends in "…" in the 238 px column)
- [ ] Compact activities on the nook (volume, switch, mic, track): check that ears + `padding-top` keep text clear of the
      bezel on both monitors; screenshot set for the QA checklist

**Open, later (owner, 2026-11-25):**
- [ ] D-27 still open: a hibernate hotkey, timed hibernate (30 min / 1 h), auto-enter on presentation mode, releasing the
      WebView while hibernating (Lite-level RAM)

**By hand (owner QA, Edge):**
- [ ] By hand: the card (Edge Nook and Floating) on both monitors at 100 % / 150 %, light/dark desktops, all 5 themes,
      real album art, 3–4 favourite devices (chip mode), the lyrics split (572 px); keyboard (Tab to the volume, arrows,
      Esc closes the opened slider)
- [ ] By hand: drag & drop effects on the idle pill, the music pill (with and without its lyric rows) and both styles, top
      and bottom: the landing ripple rises off the pill's edge and never covers it, the carry glow fades out before the
      window edge on the docked side (no hard cut), jelly landing; wavy progress line on both cards
- [ ] By hand: pin the card, drag and drop it (and Esc mid-drag): it folds into the capsule while carried and opens
      pinned again where it lands; a hover-opened card lands as the notch and stays one until the pointer leaves and
      comes back; Volume as a button on both cards; Experimental beat rim (Spotify, bass-heavy and quiet tracks) and cover tilt
- [ ] By hand: hibernate in Instant / Balanced / Lite; screen-share test in Teams, Zoom, Meet (Chrome/Edge) and OBS with
      "Hide from screen sharing" on and off; mic mute shows the dot; tray uncheck wakes the notch
- [ ] By hand: lyrics on the Liberty 5 (Bluetooth, auto 200 ms) vs the BTD 700 dongle (USB, auto 0): tune each so a
      line lights up as it is sung, and consider a better default from those two numbers
- [ ] By hand: click just beside the idle pill and the music pill on both monitors (100 % and 150 %), after moving the
      notch between monitors and after changing display scale

**Edge exit criteria:** every activity and the expanded card pass visual QA in Edge Nook on the 144 Hz and 75 Hz monitors
at 100 % / 150 %, light/dark desktops, all 4 themes; no accidental opens when reaching for browser tabs; the native and
WebView idle silhouettes match. Then Floating work resumes (D-20 drag & snap, bottom anchors).

---

## Call mode (D-30, built 2026-11-25)

Lowers the now-playing app while a call app (Discord by default) is talking; Settings → Call mode, off by default. Module
docs: [ice-win-audio](modules/ice-win-audio.md), [ice-core](modules/ice-core.md), [app-ui-settings](modules/app-ui-settings.md).

- [ ] By hand: Discord call + Spotify, Discord's Attenuation off: the music dips within ~0.3 s of her talking and comes
      back ~3 s after; nothing happens outside calls; tune the default sensitivity from what you hear
- [ ] By hand: Arctis Nova 7 (Discord on Chat, Spotify on Game) and Speakers (both on one device); unplug during a call
- [ ] By hand: move Spotify's volume during a dip (yours wins); turn call mode off mid-dip and quit mid-dip (music back at
      once); kill the process mid-dip (Task Manager), restart: Spotify's level comes back
- [ ] By hand: YouTube Music in Chrome/Edge (the browser is lowered); Firefox / Media Player (nothing lowered, as designed)
- [ ] Measure in the Phase 6 pass: `ice-call` CPU during a 10-minute call (estimate +0.005–0.01 %), zero CPU and no
      thread with call mode off
- [ ] Open (D-30 not built): a call glyph on the notch (F2, native twin), mic-level detection (A3, §1.9 amendment),
      tray/hotkey toggle (D2); answers to the four owner questions may revise the defaults

---

## Phase 4: Settings & customization

**Features:** F19–F23.

- [ ] Settings pages still open (the rest are built, see the archive): D-20 R2 module switches (per-pop-up: volume,
      switch, track) on Activities, a favourites cycle hotkey command (config has `cycle_hotkey`, no settings command
      yet), the live cost readout (D-21.1 L6)
- [ ] Theme system: bundled `Ice` (default, portfolio palette), `Midnight`, `Fluent`, `Minimal` *(the bundled set is done, plus
      `Ice Light`; what's left is user themes)*. User themes = CSS files in
      `%APPDATA%\IceAudioManager\themes\` overriding documented `--ice-*` tokens, **validated before applying**, with **live preview**
- [ ] Strict CSP: a theme with remote `url()`/`@import` produces zero network requests (verified)
- [ ] Import/export config + theme bundle (`.zip`)
- [ ] Every setting applies live without restart (true for every setting the window has today)

**Exit criteria:** a user can build a theme without reading source code. Closing Settings returns memory to the idle budget
(verified after 10× open/close, in the Phase 6 testing pass).

---

## Phase 5: Power features

**Features:** F24–F27.

- [ ] Per-app volume mixer (`IAudioSessionManager2`, `ISimpleAudioVolume`, session events) in the **expanded notch** (moved here by D-05.F18) + settings
- [ ] **Experimental** per-app output routing (`IAudioPolicyConfigFactory`, build-specific GUIDs, behind a flag, graceful failure)
- [ ] Profiles (device set + volumes + mic state) with hotkeys
- [ ] Rules: "when `process.exe` starts/focuses → apply profile" (event-driven, or low-rate polling; **no injection**)
- [ ] `icectl` CLI + `iceaudio://` URI handler (switch, volume, mute, profile) for Stream Deck / AutoHotkey

**Exit criteria:** profiles and rules work across reboot. Per-app routing fails safely on unsupported builds.

---

## Phase 6: Hardening and first public release

**Goal:** harden and publish the app after Phases 1–5 and private daily use. Start Store onboarding, public signing, and distribution in this phase.

**Testing pass (owner decision, 2026-11-23).** Performance, memory and security testing happens **once, here**, not in
every feature phase. Phases 1–5 focus on features and UI/UX.
- [ ] **Performance and RAM:**
  - re-run the Phase 0.2 benchmark recipe across every profile and state (Instant, Balanced, Lite, gaming, Settings 10×
    open/close), with the plan's budget table as the pass/fail line;
  - latency: event → feedback, wake, switch;
  - an 8-hour soak (WebView working set, handles, threads).
- [ ] **Visualizer, native pill and drag:** CPU while visible and 0 % while hidden; native pill RAM in every idle profile;
      pin/awake costs if D-21.1 adds them; release exe, installer and UI bundle sizes.
- [ ] **Security:**
  - code-practices §1 audit (no hooks, no injection, no admin, no silent network, guarded undocumented APIs);
  - `cargo audit` + `npm audit`;
  - CSP and the `iceart://` handler;
  - Tauri capabilities per window (least privilege);
  - config and theme input validation (fuzz the TOML and CSS validators).
- [ ] Fix what the pass finds before the release items below.

- [ ] Microsoft Store individual developer registration (free, with identity verification) and a separate MSIX packaging step.
      Tauri's standard Windows installers are NSIS and WiX. Smoke-test Core Audio, GSMTC, tray recovery,
      settings paths, and `StartupTask` in the packaged build
- [ ] Signing per D-09: Store MSIX first; **SignPath** for GitHub artifacts once eligible (its terms require an OSS project with a prior release).
      Sign the exe inside the portable zip as well as the NSIS installer
- [ ] Packaging per D-10: MSIX, per-user NSIS installer (no admin), portable zip
- [ ] Release workflow: tag → CI build → sign → SHA256SUMS (after signing) → GitHub artifact attestation (public repo) → draft release
- [ ] Optional **notify-only** update check: off by default, never downloads (D-11). About page shows the version.
      Trace the release build to verify zero app-initiated network requests by default
- [ ] Release checklist: VirusTotal scan of final artifacts (record results); submit any false positive to Microsoft
- [ ] winget manifest PR
- [ ] Crash-resilience: COM call timeouts, audio-thread watchdog, recover from `AUDCLNT_E_DEVICE_INVALIDATED`, re-create tray on `TaskbarCreated`
- [ ] Accessibility: keyboard navigation in settings, screen-reader names, high-contrast fallback
- [ ] Docs: README with GIFs, theming guide, privacy statement (incl. what Windows/WebView2 do on their own), FAQ ("Is this malware?")

**Exit criteria:** signed 1.0 in the Store and on GitHub. A fresh Win11 install via the Store shows no SmartScreen warning;
0 or explained detections on VirusTotal.

---

## Backlog (unscheduled)

- F28 Bluetooth headset battery in notch
- F29 Optional native-flyout suppression (research only; never via injection or LL hooks)
- F05 tray-icon scrolling (cut by vote; EarTrumpet shows a hook-free raw-input approach if it's ever revisited)
- Equalizer / APO integration (probably out of scope)
- Spatial audio / sample-rate quick toggles
- Localization
- **Spotify and YouTube Music** (owner, 2026-11-23: both will be supported):
  - *Level 1, works today without extra code:* both play through Windows' media sessions (GSMTC): Spotify desktop
    (the primary QA target, Q-09), and YouTube Music in Chrome/Edge or as an installed app. Covered by the Phase 3
    secondary QA. No network, no sign-in.
  - *Level 2, a future decision:* richer features (like/save, queue, lyrics, playlists) need each service's web API,
    which means network calls and a sign-in. §1.5 allows only the opt-in update check today, so this needs a new decision
    before any code: opt-in per service, OAuth tokens in Windows Credential Manager, no background traffic, a documented
    privacy note. YouTube Music has no official public API. Unofficial ones risk breaking its terms, so Level 2 may be
    Spotify-only. D-22 built Level 1, the "Music services" widget and opt-in LRCLIB lyrics; YouTube Music playback
    inside the app was rejected; Spotify Level 2 still needs its own decision under §1.5.
- **Ideas from BitChord / Echo-Music** (owner references, 2026-11-23; undecided, each needs an F-ID or decision before code):
  - *Sleep timer:* pause the current media session after N minutes or at the end of the track (GSMTC; no network).
  - *Output format readout:* the default device's sample rate and bit depth in the expanded notch (the endpoint's mix
    format). Pairs with "sample-rate quick toggles" above.
  - *Discord Rich Presence:* local IPC to Discord, but it shares what you play, so it would be opt-in with a privacy note.
  - Not for us: offline downloads, ad-free playback via private APIs, analytics/crash reporting (§1.5).
- **Ideas from Auralys** (Android, closed source; owner screenshot 2026-11-23; undecided):
  - *Share card:* render the current track (art, title, artist, theme colours) to a PNG on the clipboard or in a file.
    Local only, no upload.
  - *UI style as a setting* (Editorial vs Glass), separate from colour themes. Related to D-23.
  - Not for us: "listen together" rooms. They need a server and real-time sync, outside D-05 and §1.5.

### Listening experiments for Pandora and Ice

Ideas for discussion, 2026-11-27. **Try the lyric loop first.** One local song with timed lyrics is enough to find out
whether selecting a phrase makes it easier to practise than dragging the seek bar. [Pandora prototype](pandora.md)

**Pandora**

- **Loop a lyric.** Select the first and last lines of a chorus and repeat that passage at the chosen playback speed.
  Try a fast verse at normal and reduced speed. Keep the interaction if the listener can practise three repeats without
  touching the seek bar, then turn looping off and hear the song continue from that line.
- **Take a one-song detour.** While listening to an album, audition a recommendation and return to the exact spot you
  left. Try it halfway through a local track with shuffle on. Both finishing the detour and choosing “Back to album”
  should restore the playhead and preserve the upcoming track order. Compare the effort with rebuilding the queue by hand.

**Ice Audio Manager**

- **Where did my sound go?** Show the Windows defaults and their mute state when someone opens the tray. For example,
  “Default media output · Arctis Game (muted)” and “Default call output · Arctis Chat.” Give someone a split Game/Chat
  setup and a muted output in Lite mode. Can they explain why the call is audible and their music is silent without
  opening Windows Sound settings?
- **Undo my last output switch.** After an Ice switch, offer one tray action to restore the previous device for each
  role Ice changed. Try it with separate music and call defaults, then repeat after unplugging the old headset or changing
  a default in Windows. Undo should restore the original split, explain when a device is unavailable, and leave any role
  the user subsequently changed alone. [Device switch flow](modules/data-flows.md#1-hotkey-switch-speakers--arctis)

**Pandora with Ice**

- **Replay what a call covered.** When Ice starts lowering Pandora for a call, let Pandora remember the current track and
  position in memory. When the volume returns, offer “Replay from here” in the player. Try two speech bursts during one
  verse, then skip tracks during a third. One click should recover the start of the interrupted passage. Skipping should
  clear the offer, and ignoring it should let playback continue. [Call mode](#call-mode-d-30-built-2026-11-25)
- **Tune lyric timing by ear for each output.** Play a local clip in Pandora with a sung count and known lyric timestamps.
  Let the listener compare two offsets on the Ice notch and save the preferred one for that output using Ice's existing
  per-device lyric delay. Switch between speakers and a Bluetooth headset, then repeat the comparison with the order
  reversed. Keep the guided setup if the listener consistently prefers the saved timing over the default on the delayed
  device and switching back recalls the speakers' timing. [Lyrics design](lyrics.md)

## Cleanup backlog

Known debt under [code-practices §11](code-practices.md#11-code-hygiene-duplication-cleanliness-bloat). Each item is fixed in its
own `refactor:` PR by the path's owner, or sooner if a feature needs to grow the file. Sizes from 2026-11-26, including
tests where the file holds them.

| Item | Owner | Size / issue | Suggested split |
|---|---|---|---|
| `crates/ice-win/src/native_dot.rs` | Gemini | 2546 lines (limit ~600; `glyphs.rs` and `input.rs` already split out) | window/message loop · GDI drawing · layout/geometry · theme colours |
| `app/src-tauri/src/main.rs` | Codex | 1492 lines; `#[allow(too_many_lines)]` at the setup fn | app setup · notch window lifecycle · commands · profiles/gaming mode |
| `app/ui/src/lib/notch/scheduler.ts` | Gemini | 635 lines, over the limit | Split the merge rules and the media/track handling out (the now-playing view already lives in `nowPlaying.ts`) |
| Repeated `#[allow(clippy::cast_possible_truncation, cast_sign_loss)]` (native_dot ×5, audio_engine, media) | Claude + Gemini | Same `f64` → integer conversion repeated without a reason | One checked conversion helper in `ice-win`; remaining suppressions become `#[expect(…, reason)]` |

## Dependency map

```
Phase 0 ─► Phase 1 ─► Phase 2 ─► Phase 3 ─► Phase 4 ─► Phase 5 ─► Phase 6 (first public release)
```
