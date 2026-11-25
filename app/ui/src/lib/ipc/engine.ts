// Auto-generated from the ice-core engine contract.
// Do not edit manually.

export type SpringPreset = "bouncy" | "smooth" | "snappy" | "off";

export type ThemeKind = "ice" | "ice_light" | "midnight" | "fluent" | "minimal";

export type IdleStyle = "dot" | "mini_pill" | "hidden";

export type NotchStyle = "floating" | "nook";

export type NowPlayingPill = "brief" | "while_playing" | "off";

export type HibernateLook = "plain" | "hidden";

export type NotchAnchor = "top_left" | "top_center" | "top_right" | "bottom_left" | "bottom_center" | "bottom_right";

export type NotchPlacement = "drag_snap" | "fixed";

export type Appearance = { theme: ThemeKind, preset: SpringPreset, idleStyle: IdleStyle, notchStyle: NotchStyle, nowPlaying: NowPlayingPill,
/**
 * D-25: slow ambient drift of the art wash on the expanded card while playing.
 */
movingBackdrop: boolean,
/**
 * D-27 hibernate: the notch shows no track, art, lyrics or pop-ups and is click-through, for meetings and screen
 * sharing. Session state: every start begins awake (`ConfigStore::load`), and a hot reload keeps the live value.
 */
hibernate: boolean,
/**
 * D-27: what the notch looks like while hibernating (plain notch, or nothing at all).
 */
hibernateLook: HibernateLook,
/**
 * D-27: while hibernating, leave the notch windows out of screen captures (`WDA_EXCLUDEFROMCAPTURE`).
 */
hibernateHideFromCapture: boolean, };

export type Flow = "render" | "capture";

export type Role = "console" | "multimedia" | "communications";

export type DeviceState = "active" | "disabled" | "not_present" | "unplugged" | "unknown";

export type RoleSet = "default" | "communications" | "both";

export type SpatialMode = "keep" | "off" | "windowsSonic" | "dolbyAtmosHeadphones" | "dolbyAtmosHomeTheater" | "dolbyAtmosSpeakers" | "dtsHeadphoneX" | "dtsXUltra" | "dtsXHomeTheater";

export type DeviceSummary = {
/**
 * Endpoint ID. Use it for `SwitchTo`; bindings use `DeviceRef`s held by the engine.
 */
id: string,
/**
 * Display name without Windows' `N- ` re-install prefix, e.g. "Headphones (Arctis Nova 7)".
 */
name: string, flow: Flow, state: DeviceState,
/**
 * Default for Console + Multimedia.
 */
isDefault: boolean,
/**
 * Default for Communications.
 */
isDefaultCommunications: boolean,
/**
 * `EndpointFormFactor` (3 = headphones, 1 = speakers, 4 = microphone, 9 = digital/HDMI…), for the icon.
 */
formFactor: number | null,
/**
 * Software device (Steam Streaming, Sonar, VB-Cable…): shown with a "virtual" badge.
 */
isVirtual: boolean,
/**
 * Active spatial format, when known (`None` for capture devices or unknown).
 */
spatial: SpatialMode | null,
/**
 * How many ms later lyrics show while this device plays (the user's value or the automatic one; 0 for capture).
 */
lyricsDelayMs: number, };

export type VolumeState = {
/**
 * 0–100, rounded from Windows' scalar.
 */
percent: number, muted: boolean, };

export type MusicServiceId = "spotify" | "youtube_music";

export type MusicServices = { spotify: boolean, youtubeMusic: boolean, };

export type LyricsConfig = {
/**
 * Master toggle for displaying lyrics on the notch (off by default).
 */
enabled: boolean,
/**
 * Option A (LRCLIB `https://lrclib.net`): strictly opt-in from Settings → Privacy (§1.5).
 */
onlineLookup: boolean,
/**
 * Attach the compact bottom lyrics extension under the music pill.
 */
showOnPill: boolean,
/**
 * Attach the 3-line bottom lyrics extension under the expanded card.
 */
showOnCard: boolean,
/**
 * Highlight the active word across each synced line (Enhanced LRC, or estimated cadence when enabled).
 */
wordHighlight: boolean,
/**
 * Experimental: estimate word-by-word highlight timing for line-synced LRC files without word timestamps.
 */
estimatedWordSync: boolean,
/**
 * Generate an offline Romaji subtitle line underneath Japanese lyric lines.
 */
romaji: boolean,
/**
 * Maximum managed lyrics cache size on disk, in MiB ([`LYRICS_CACHE_MAX_MB`]).
 */
cacheMaxMb: number, };

export type CallModeConfig = {
/**
 * Off by default: the app never changes another app's volume unless asked to.
 */
enabled: boolean,
/**
 * Executable names whose audio counts as a call (`Discord.exe`; `.exe` optional, any case).
 */
apps: Array<string>,
/**
 * Music level while people talk, % of its own level ([`CALL_DUCK_PERCENT`]).
 */
duckPercent: number,
/**
 * Quiet seconds before the music comes back ([`CALL_HOLD_SECS`]).
 */
holdSecs: number, sensitivity: CallSensitivity, };

export type CallSensitivity = "low" | "normal" | "high";

export type LyricsSource = "local" | "cache" | "lrclib";

export type LyricsSyncKind = "word" | "line" | "plain";

export type LyricsStatus = "disabled" | "idle" | "loading" | "ready" | "notFound";

export type LyricWord = { startMs: number, endMs: number, text: string, };

export type LyricLine = { startMs: number, endMs: number | null, text: string,
/**
 * A paired line from the lyrics source itself (same timestamp in the `.lrc`): a translation or the source's
 * own Romaji. Shown whenever present.
 */
subtitle: string | null,
/**
 * Romaji generated on this PC (`ice_win::lyrics::romaji`) for Japanese lines without a `subtitle`. Shown only
 * while `LyricsConfig::romaji` is on, and never written to the disk cache, so generator fixes reach every song.
 */
romaji: string | null, words: Array<LyricWord>, };

export type LyricsDocument = { trackKey: string, source: LyricsSource, syncKind: LyricsSyncKind, instrumental: boolean, lines: Array<LyricLine>, plainText: string | null, };

export type LyricsState = { status: LyricsStatus, trackKey: string | null, document: LyricsDocument | null, };

export type LyricsStorageStats = { cacheBytes: number, cacheFiles: number, customFiles: number, cacheMaxMb: number, customDir: string,
/**
 * Whether Windows can read kanji for Romaji here (needs the Japanese language installed); when false, only
 * kana lines get Romaji. Probed when the Settings window asks, never in the background.
 */
romajiReadsKanji: boolean, };

export type MediaAction = "playPause" | "play" | "pause" | "next" | "previous" | { "seek": { positionMs: number, } };

export type PlaybackStatus = "playing" | "paused" | "stopped" | "changing" | "opened" | "closed";

export type Rgb = { r: number, g: number, b: number, };

export type ArtInfo = {
/**
 * Content hash of the downscaled image; stable across re-reads of the same art.
 */
id: string, width: number, height: number,
/**
 * Readable accent colour derived from the art.
 */
accent: Rgb,
/**
 * Colours that are really in the art, largest area first, at most [`PALETTE_SIZE`] (owner, 2026-11-25: the
 * backdrop should follow the cover, not hue-rotated guesses). Empty only for fully transparent art.
 */
palette: Array<Rgb>,
/**
 * The cover's average WCAG luminance, 0–100 % (the backdrop scrim strengthens over bright covers).
 */
luminancePct: number, };

export type NowPlaying = {
/**
 * Source app, e.g. `Spotify.exe`.
 */
appId: string, title: string, artist: string, album: string, status: PlaybackStatus, positionMs: bigint | null, durationMs: bigint | null,
/**
 * Unix epoch timestamp (ms) when `position_ms` was measured (`LastUpdatedTime`), used to interpolate
 * smooth live playback time without polling GSMTC.
 */
positionUpdatedMs?: number | null, art: ArtInfo | null, };

export type SavedDevice = {
/**
 * Name it was saved under, e.g. "Headphones (Arctis Nova 7)".
 */
name: string, flow: Flow,
/**
 * Original endpoint ID, including while disconnected. Used to retain ordered favourites.
 */
savedId: string,
/**
 * The live endpoint it resolves to right now (`None` = not connected or needs a choice).
 */
deviceId: string | null, };

export type TogglePairView = { id: string, hotkey: string | null, primary: SavedDevice, secondary: SavedDevice, roles: RoleSet | null, };

export type BindingView = { id: string, hotkey: string | null, device: SavedDevice, roles: RoleSet | null, };

export type DevicePrefsView = { device: SavedDevice, spatial: SpatialMode, alias: string | null, hidden: boolean,
/**
 * The user's lyric delay for this device; `None` = automatic (`config::auto_lyrics_delay_ms`).
 */
lyricsDelayMs: number | null, };

export type PerformanceProfile = "instant" | "balanced" | "lite";

export type SettingsView = { appearance: Appearance, performanceProfile: PerformanceProfile, pinnedMonitor: string | null,
/**
 * D-20: where on the pinned monitor's top or bottom edge the notch sits.
 */
notchAnchor: NotchAnchor,
/**
 * D-20: optional continuous horizontal offset (`0.0..=1.0`) between Left and Right; `None` = preset `notch_anchor`.
 */
notchXRatio: number | null,
/**
 * D-20: drag to move along the monitor edge or fixed.
 */
notchPlacement: NotchPlacement, defaultRoles: RoleSet, togglePairs: Array<TogglePairView>, bindings: Array<BindingView>, favouritesPlayback: Array<SavedDevice>, favouritesRecording: Array<SavedDevice>, devices: Array<DevicePrefsView>, knobGestures: boolean, showDisconnected: boolean,
/**
 * Seconds before `Balanced` suspends the idle notch (`config::IDLE_LINGER_SECS`).
 */
idleLingerSecs: number,
/**
 * D-22 launch buttons in the notch.
 */
musicServices: MusicServices,
/**
 * D-22 synced lyrics, Privacy opt-in, Romaji, and Storage cache ceiling.
 */
lyrics: LyricsConfig,
/**
 * D-26: live 14-band spectrum bars via local WASAPI loopback (`false` = peak-meter-only fallback).
 */
visualizerSpectrum: boolean,
/**
 * D-30 call mode settings.
 */
callMode: CallModeConfig,
/**
 * D-19 "show notch" hotkey, if bound.
 */
showNotchHotkey: string | null, };

export type EngineSnapshot = { revision: number,
/**
 * Active playback devices (ghosts excluded, D-16).
 */
playback: Array<DeviceSummary>,
/**
 * Active recording devices.
 */
recording: Array<DeviceSummary>,
/**
 * Unplugged endpoints of both flows (`state: unplugged`), only while `settings.show_disconnected` is on.
 * For display only (greyed); they can't be switched to. Hidden devices and ghosts are never listed.
 */
disconnected: Array<DeviceSummary>,
/**
 * Volume of the default playback device.
 */
volume: VolumeState | null,
/**
 * Mute state of the default communications microphone.
 */
micMuted: boolean | null,
/**
 * Volume (0–100) of the default communications microphone.
 */
micVolume: number | null,
/**
 * Problems that stay true until fixed (hotkey conflicts, config problems). A UI shows these after
 * (re)subscribing, instead of relying on replayed events (review ENG-08).
 */
notices: Array<Notice>,
/**
 * The user's settings, resolved against the live devices.
 */
settings: SettingsView,
/**
 * What's playing (Windows' current media session), `None` when nothing is.
 */
media: NowPlaying | null,
/**
 * Synced lyrics state for the current track (disabled by default; D-22).
 */
lyrics: LyricsState, };

export type Command = { "type": "switchTo", deviceId: string, roles: RoleSet, } | { "type": "switchBinding", bindingId: string, } | { "type": "togglePair", pairId: string, } | { "type": "cycleFavourites", flow: Flow, step: number, } | { "type": "setVolume", percent: number, } | { "type": "stepVolume", delta: number, } | { "type": "toggleMute" } | { "type": "toggleMicMute" } | { "type": "media", action: MediaAction, } | { "type": "setSpatial", deviceId: string, mode: SpatialMode, } | { "type": "resolveChoice", requestId: number, deviceId: string | null, } | { "type": "refresh" } | { "type": "setTogglePair", pairId: string, primaryId: string, secondaryId: string, hotkey: string | null, roles: RoleSet | null, } | { "type": "removeTogglePair", pairId: string, } | { "type": "setBinding", bindingId: string, deviceId: string, hotkey: string | null, roles: RoleSet | null, } | { "type": "removeBinding", bindingId: string, } | { "type": "setFavourites", flow: Flow, deviceIds: Array<string>, } | { "type": "setDevicePrefs", deviceId: string, spatial: SpatialMode | null, alias: string | null, hidden: boolean | null, } | { "type": "setLyricsDelay", deviceId: string, delayMs: number | null, } | { "type": "setTheme", theme: ThemeKind, } | { "type": "setPreset", preset: SpringPreset, } | { "type": "setIdleStyle", style: IdleStyle, } | { "type": "setNotchStyle", style: NotchStyle, } | { "type": "setNowPlayingPill", mode: NowPlayingPill, } | { "type": "setMovingBackdrop", enabled: boolean, } | { "type": "setHibernate", enabled: boolean, } | { "type": "setHibernateLook", look: HibernateLook, } | { "type": "setHibernateHideFromCapture", enabled: boolean, } | { "type": "setPerformanceProfile", profile: PerformanceProfile, } | { "type": "setNotchAnchor", anchor: NotchAnchor, } | { "type": "setNotchPosition", anchor: NotchAnchor, xRatio: number | null, } | { "type": "setNotchPlacement", placement: NotchPlacement, } | { "type": "setPinnedMonitor", displayPath: string, } | { "type": "setKnobGestures", enabled: boolean, } | { "type": "setShowDisconnected", enabled: boolean, } | { "type": "setIdleLinger", secs: number, } | { "type": "setShowNotchHotkey", hotkey: string | null, } | { "type": "setMusicService", service: MusicServiceId, enabled: boolean, } | { "type": "setLyricsEnabled", enabled: boolean, } | { "type": "setLyricsOnlineLookup", enabled: boolean, } | { "type": "setLyricsDisplay", showOnPill: boolean | null, showOnCard: boolean | null, wordHighlight: boolean | null, romaji: boolean | null, } | { "type": "setLyricsEstimatedWordSync", enabled: boolean, } | { "type": "setLyricsCacheMaxMb", maxMb: number, } | { "type": "disableAllNetworkFeatures" } | { "type": "setVisualizerSpectrum", enabled: boolean, } | { "type": "setCallMode", enabled: boolean | null, apps: Array<string> | null, duckPercent: number | null, holdSecs: number | null, sensitivity: CallSensitivity | null, } | { "type": "setDefaultRoles", roles: RoleSet, };

export type Notice = { "type": "notConnected", name: string, } | { "type": "chooseDevice", requestId: number, name: string, candidates: Array<DeviceSummary>, } | { "type": "switchFailed", name: string, failedRoles: Array<Role>, } | { "type": "spatialUnavailable", name: string, mode: SpatialMode, reason: string, } | { "type": "hotkeyConflict", bindingId: string, keys: string, } | { "type": "configReplaced", backup: string, error: string, } | { "type": "configInvalid", error: string, } | { "type": "configSaveFailed", error: string, } | { "type": "settingRejected", error: string, };

export type EngineEvent = { "type": "devicesChanged", playback: Array<DeviceSummary>, recording: Array<DeviceSummary>,
/**
 * Mirrors `EngineSnapshot::disconnected` (empty while the setting is off).
 */
disconnected: Array<DeviceSummary>, } | { "type": "volumeChanged", volume: VolumeState, } | { "type": "micMuteChanged", muted: boolean, } | { "type": "micVolumeChanged", percent: number, } | { "type": "mediaChanged", media: NowPlaying | null, } | { "type": "lyricsChanged", lyrics: LyricsState, } | { "type": "switched", device: DeviceSummary, roles: RoleSet,
/**
 * The binding followed the device to a new endpoint ID (shown as a caption, one notice).
 */
relinked: boolean,
/**
 * Set when a spatial format was applied and read back.
 */
spatialApplied: SpatialMode | null, } | { "type": "deviceArrived", device: DeviceSummary, } | { "type": "deviceLeft", device: DeviceSummary, } | { "type": "notice", notice: Notice, } | { "type": "noticesChanged", notices: Array<Notice>, } | { "type": "settingsChanged", settings: SettingsView, } | { "type": "showNotch" };

export type Envelope = { revision: number, event: EngineEvent, };

export type MeterFrame = {
/**
 * Highest peak across channels, 0.0–1.0 linear amplitude, as Windows reports it.
 */
peak: number,
/**
 * Per-channel peaks (0.0–1.0) in the endpoint's channel order (front left, front right, …).
 * Empty when Windows doesn't report channels.
 */
channels: Array<number>,
/**
 * Optional 14-band log-spaced spectrum levels (0.0–1.0, ~45 Hz – 14 kHz) from local WASAPI loopback (D-26).
 * `None` when using peak-meter-only fallback (`IAudioMeterInformation` or Privacy toggle off).
 */
bands?: Array<number>, };
