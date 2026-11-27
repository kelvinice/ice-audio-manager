# Reference recap: VIVI Music

> **Read on 2026-11-26** by Claude, from a shallow clone at commit `e41b101` (2026-11-23) in
> `docs/reference/vivi-music/` (gitignored, never committed). Owner request: what is in it, what Ice could adopt, and
> which ideas are greenlight or grey area for us.

## In one paragraph

VIVI Music (`vivizzz007/vivi-music`) is an **Android YouTube Music client** in Kotlin (Jetpack Compose, Material 3,
ExoPlayer/Media3). It descends from Metrolist (and before it InnerTune/OuterTune). It plays YouTube Music itself
(ad-free), and adds synced lyrics from about 15 sources, an equalizer, downloads, Discord presence, Last.fm, Shazam-style
recognition, "Listen Together" rooms, animated album "canvas" videos and an in-app updater. Its look is the part closest
to Ice: colours from the cover, animated backdrops, expressive sliders. It is a media **player**; Ice is an audio
**manager** that never plays audio itself. So many of its features don't map onto Ice at all, and many that do rely on
private APIs.

## Licence: ideas only, no code

- **GPL-3.0** (`LICENSE`). Its `rules.md` adds conditions on reuse: attribution in the source **and in the app's UI**
  for the JioSaavn code, and copies must also be GPL-3.0.
- Some files go further and forbid any reuse: `jiosaavn/…/SaavnService.kt` ("copying, modification, redistribution, or
  reuse … strictly prohibited") and several `kizzy` files (`KizzyRPC.kt`, `HeartBeat.kt`, `Ready.kt`, `RpcImage.kt`,
  `Ext.kt`).
- For Ice (MIT, D-24) that means **look, don't copy** (code-practices §1.8): every idea below is to be built from
  documented Windows APIs, or from public format specs, in our own code. The PR cites this file as the reference.
- Its `AGENTS.md` is written for agents working on VIVI's own repo; nothing in it applies here.

## What's inside

| Area | Where | What it does |
|---|---|---|
| Playback | `app/…/playback/` (`MusicService`, `SleepTimer`, `SilenceDetectorAudioProcessor`, queues) | ExoPlayer service, queues, sleep timer (minutes or end of song), skip long silences, crossfade, normalisation, tempo/pitch |
| Streaming sources | `innertube/` (107 files), `utils/` (`CipherDeobfuscator`, `PoTokenWebView`, `Sabr…`, `BotDetectionMitigator`), `jiosaavn/` | YouTube Music internals and stream deciphering; JioSaavn as a fallback source (DES-decrypted media URLs) |
| Lyrics | `app/…/lyrics/` (16 providers), `lyricsProvider/` (`TTMLParser`, KuGou, LrcLib, Unison, BetterLyrics, YouLyPlus, Paxsenix) | User-ordered provider chain (`LyricsProviderOrder`), TTML word timing with duet/background-vocal agents, KuGou KRC, Genius as plain-text fallback |
| Lyrics extras | `LyricsRomanizationHelper`, `LyricsTranslationHelper`, `api/` (`DeepLService`, `MistralService`, `OpenRouter…`) | Romanization for 15+ scripts **via Google Translate online**; AI translation with the user's DeepL/Mistral/OpenRouter key |
| Look | `Motionartwork/`, `artistvideo/`, `spotify/SpotifyCanvasProvider`, `ui/` (224 files) | Animated covers and artist backgrounds from Apple Music / Tidal / Spotify, cover-coloured theme, squiggly slider, rotating cover, turntable widget |
| Audio devices | `vivimusic/AudioDeviceBottomSheet`, `ViviVolumeSlider`, `MusicService.onAudioDevicesAdded` | In-place output switcher (stable order: speaker → wired → Bluetooth), segmented volume with hold-to-repeat ± buttons, **resume when a Bluetooth headset connects** |
| Equalizer | `eq/` (`ParametricEQParser`, `BiquadFilter`, `CustomEqualizerAudioProcessor`) | 10+ band parametric EQ on its own player; imports AutoEq `ParametricEQ.txt` (Equalizer APO format) |
| Social / accounts | `kizzy/`, `utils/DiscordRPC`, `lastfm/`, `utils/ScrobbleManager`, `listentogether/` | Discord presence over Discord's **gateway with the user's account token**; Last.fm scrobbling (delay %, minimum length, offline queue); "Listen Together" rooms through VIVI's servers |
| Recognition | `recognition/`, `shazamkit/`, `app/src/main/cpp/vibrafp` | Records the **microphone** for 10 s, builds a Shazam-compatible fingerprint, queries Shazam's API |
| Upkeep | `vivimusic/vivimusicupdater` + `UpdateDownloadWorker`, `github/` (announcements, star prompt), `utils/AutoBackupHelper`, `CrashHandler`, `DiagnosticLogCollector` | OTA updater that downloads APKs; announcements fetched from GitHub; weekly + pre-update backups (keeps 5); local crash logs; a user-triggered diagnostics bundle |
| Settings | `constants/PreferenceKeys.kt` (266 keys) | Also: setup wizard, UI density scale, custom fonts, proxy, data saver, "Wrapped" year review, block screenshots |

## Already covered in Ice

Cover colours and motion backdrop (D-23, D-25) · word-highlighted synced lyrics with Romaji and a size-capped cache
(D-22, `docs/lyrics.md`) · now-playing on the notch from Windows media sessions (no playback of our own) · out of screen
captures (D-27 C1, vs. VIVI's "disable screenshots") · local-only data, zero telemetry (§1.5).

## Greenlight: fits our rules, local, no network

Each one still needs an F-ID or a decision entry before code (plan.md backlog rule). Effort is estimated: S ≈ a day,
M ≈ a few days.

| # | Idea | VIVI reference | How it would work in Ice | Effort |
|---|---|---|---|---|
| G1 | **Resume when a headset connects** (and, as its pair, pause when it leaves) | `MusicService.onAudioDevicesAdded` | On `DeviceArrived` / default-device change to a chosen output, send GSMTC Play if media is paused; on removal, Pause. Per device, off by default (Settings → Devices). Event-driven, no polling | S |
| G2 | **Sleep timer** (N minutes, or end of the current track) | `playback/SleepTimer.kt` | GSMTC Pause from the engine; tray + notch toolbar; end-of-track uses the timeline we already get. Already in the backlog (BitChord idea) | S |
| G3 | **Output format badge** (sample rate / bit depth) | `ShowAudioQualityBadge` | Read the endpoint's mix format; show it on the device chip or card header. Already in the backlog ("output format readout") | S |
| G4 | **Volume ± buttons with hold-to-repeat** | `vivimusic/ViviVolumeSlider.kt` | Add −/+ caps to the expanded volume slider in the output strip (`OutputStrip`), stepping `VOLUME_CLICK_STEP`, repeating while held | S |
| G5 | **Local TTML lyrics** (word timing, duets, background vocals) | `lyricsProvider/…/TTMLParser.kt` | Parse `.ttml` (W3C TTML, Apple-style `ttm:agent`) in our local lyrics folder, written from the spec; style duet singers and background vocals on the Edge Nook ladder | M |
| G6 | **Offline romanization for more scripts**: Korean (Revised Romanization is pure rules), Cyrillic (transliteration tables) | `LyricsRomanizeKorean`, `…Cyrillic…` (VIVI does it online) | Same pure-function approach as our Kana → Hepburn; no network. Chinese pinyin needs a dictionary → grey (size) | M |
| G7 | **Lyrics display options**: text alignment, animation style | `LyricsTextPosition`, `LyricsAnimationStyle` | Settings → Activities; only `--ice-lyric-*` token and class changes | S |
| G8 | **Wavy progress bar while playing** | `SquigglySlider` | The seek bar's played part becomes a gently moving wave (one `transform` on a repeating SVG wave), frozen when paused, hidden or reduce motion. Experimental first | S |
| G9 | **Spinning cover** while playing | `RotatingThumbnail`, `TurntableWidgetReceiver` | Rotate the round art in the music pill (`transform`, stops on pause/hide). Experimental | S |
| G10 | **Config backup before every schema migration** (keep the 5 newest) + Export/Import | `utils/AutoBackupHelper.kt` | `ConfigStore` copies `config.toml` before migrating to a new `SCHEMA_VERSION`; Export/Import is already a Phase 4 item | S |
| G11 | **Local crash log + "Export diagnostics"** | `utils/CrashHandler.kt`, `DiagnosticLogCollector.kt` | Panic hook writes a crash file under `%LOCALAPPDATA%`; a Settings → About button zips logs + config for the user to attach. Never uploaded; no titles above `debug` (§1.5). Fits Phase 6 crash resilience | M |
| G12 | **First-run setup** (toggle pair, notch style, hotkeys) | `SetupWizardCompleted` | A short first-run page in the Settings window; skippable | M |

## Grey area: possible, but needs an owner decision, an opt-in, or a check first

| # | Idea | VIVI reference | Why it's grey for Ice | What would make it OK |
|---|---|---|---|---|
| Y1 | **Last.fm scrobbling** | `lastfm/`, `utils/ScrobbleManager.kt` | Network + account + API key; §1.5 allows only the opt-in update check (and LRCLIB under D-22) | A D-22-style decision: off by default, token in Credential Manager, no background traffic beyond scrobbles, privacy note |
| Y2 | **Lyric translation** with the user's own DeepL / AI key | `LyricsTranslationHelper`, `api/` | Sends lyric text to a third party; keys to store | Opt-in per provider, user's own key in Credential Manager, a clear disclosure in Privacy |
| Y3 | **Discord Rich Presence** through Discord's documented **local IPC** (not VIVI's account-token gateway) | `utils/DiscordRPC.kt` (template idea `{song_name}`, `{artist_name}`) | Shares what you play; already in the backlog as opt-in | Local named-pipe IPC with our own app ID, off by default, a privacy note |
| Y4 | **More lyrics sources** (Unison, BetterLyrics, KuGou, YouLyPlus, Genius…) | `lyrics/` providers + `LyricsProviderOrder` | Each is a new network destination; several are unofficial or have unclear terms | Per-source terms check, then the D-22 rules (opt-in, cache); a provider-order setting |
| Y5 | **Parametric EQ / AutoEq import** per headset | `eq/ParametricEQParser.kt` | Ice doesn't own the audio stream; system-wide EQ needs an APO driver, and installing or editing one needs admin rights (§1.2) | Only as "export an Equalizer APO config file" for users who already run APO; otherwise park (plan.md already says "probably out of scope") |
| Y6 | **Local listening history + yearly recap card** | listen history, `ShowWrappedCard` | Stores track titles on disk (media PII); new data to protect and clear | Opt-in, local only, retention + "Clear history" in Storage; pairs with the Auralys "share card" backlog idea |
| Y7 | **Chinese pinyin** romanization | `LyricsRomanizeChinese` | Needs a character dictionary (MBs); licence and size to check | A decision on a bundled dictionary, like the kanji question in `docs/lyrics.md` §9 |
| Y8 | **Notch size / density setting** | `DensityScale` | Fixed canvas (648 × 292) and native-pill twin sizes would both need to scale | A geometry spike first (both hosts, 100 / 150 %) |
| Y9 | **"What's new" after an update** | `changelogscreen.kt` | Fine if the notes ship inside the app; not if fetched (D-11, §1.5) | Bundle the changelog with the release; show once |

## Beyond Ice: the experimental Pandora app

The former redline list was removed on 2026-11-27 at the owner's request. Ideas that don't fit inside Ice itself
(in-app YouTube Music playback, song recognition, playlist import, audio DSP) are now explored in **Pandora**, the
experimental companion player in `pandora/` ([pandora.md](../pandora.md)). Ice stays standalone and keeps its own rules
(§1.5 zero silent network, §1.9 microphone); Pandora talks to it only through the optional loopback bridge. VIVI's
code is still never copied into either app (GPL-3.0; §1.8 reference-never-copy).

**Not applicable** (Ice never plays the audio itself): crossfade, normalisation, tempo/pitch, skip-silence, queues,
downloads, data saver, proxy, SponsorBlock segment skipping.

## Suggested next step

If the owner wants a short list, the best value for the least risk is **G1** (resume/pause on headset connect, which
fits the Arctis ↔ Speakers workflow), **G2** (sleep timer) and **G3** (output format badge). Each is small and
event-driven, and each needs only an F-ID. **G5** (local TTML) and **G6** (Korean/Cyrillic romanization) extend
lyrics work that is already built. Grey items need a `D-xx` entry before any code.
