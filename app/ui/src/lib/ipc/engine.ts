// Auto-generated from the ice-core engine contract.
// Do not edit manually.

export type SpringPreset = "bouncy" | "smooth" | "snappy" | "off";

export type ThemeKind = "ice" | "ice_light" | "midnight" | "fluent" | "minimal";

export type IdleStyle = "dot" | "mini_pill" | "hidden";

export type NowPlayingPill = "brief" | "while_playing" | "off";

export type Appearance = { theme: ThemeKind, preset: SpringPreset, idleStyle: IdleStyle, nowPlaying: NowPlayingPill,
/**
 * D-25: slow ambient drift of the art wash on the expanded card while playing.
 */
movingBackdrop: boolean, };

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
spatial: SpatialMode | null, };

export type VolumeState = {
/**
 * 0–100, rounded from Windows' scalar.
 */
percent: number, muted: boolean, };

export type MusicServiceId = "spotify" | "youtube_music";

export type MusicServices = { spotify: boolean, youtubeMusic: boolean, };

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
accent: Rgb, };

export type NowPlaying = {
/**
 * Source app, e.g. `Spotify.exe`.
 */
appId: string, title: string, artist: string, album: string, status: PlaybackStatus, positionMs: bigint | null, durationMs: bigint | null, art: ArtInfo | null, };

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

export type DevicePrefsView = { device: SavedDevice, spatial: SpatialMode, alias: string | null, hidden: boolean, };

export type PerformanceProfile = "instant" | "balanced" | "lite";

export type SettingsView = { appearance: Appearance, performanceProfile: PerformanceProfile, pinnedMonitor: string | null, defaultRoles: RoleSet, togglePairs: Array<TogglePairView>, bindings: Array<BindingView>, favouritesPlayback: Array<SavedDevice>, favouritesRecording: Array<SavedDevice>, devices: Array<DevicePrefsView>, knobGestures: boolean, showDisconnected: boolean,
/**
 * Seconds before `Balanced` suspends the idle notch (`config::IDLE_LINGER_SECS`).
 */
idleLingerSecs: number,
/**
 * D-22 launch buttons in the notch.
 */
musicServices: MusicServices,
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
media: NowPlaying | null, };

export type Command = { "type": "switchTo", deviceId: string, roles: RoleSet, } | { "type": "switchBinding", bindingId: string, } | { "type": "togglePair", pairId: string, } | { "type": "cycleFavourites", flow: Flow, step: number, } | { "type": "setVolume", percent: number, } | { "type": "stepVolume", delta: number, } | { "type": "toggleMute" } | { "type": "toggleMicMute" } | { "type": "media", action: MediaAction, } | { "type": "setSpatial", deviceId: string, mode: SpatialMode, } | { "type": "resolveChoice", requestId: number, deviceId: string | null, } | { "type": "refresh" } | { "type": "setTogglePair", pairId: string, primaryId: string, secondaryId: string, hotkey: string | null, roles: RoleSet | null, } | { "type": "removeTogglePair", pairId: string, } | { "type": "setBinding", bindingId: string, deviceId: string, hotkey: string | null, roles: RoleSet | null, } | { "type": "removeBinding", bindingId: string, } | { "type": "setFavourites", flow: Flow, deviceIds: Array<string>, } | { "type": "setDevicePrefs", deviceId: string, spatial: SpatialMode | null, alias: string | null, hidden: boolean | null, } | { "type": "setTheme", theme: ThemeKind, } | { "type": "setPreset", preset: SpringPreset, } | { "type": "setIdleStyle", style: IdleStyle, } | { "type": "setNowPlayingPill", mode: NowPlayingPill, } | { "type": "setMovingBackdrop", enabled: boolean, } | { "type": "setPerformanceProfile", profile: PerformanceProfile, } | { "type": "setPinnedMonitor", displayPath: string, } | { "type": "setKnobGestures", enabled: boolean, } | { "type": "setShowDisconnected", enabled: boolean, } | { "type": "setIdleLinger", secs: number, } | { "type": "setShowNotchHotkey", hotkey: string | null, } | { "type": "setMusicService", service: MusicServiceId, enabled: boolean, } | { "type": "setDefaultRoles", roles: RoleSet, };

export type Notice = { "type": "notConnected", name: string, } | { "type": "chooseDevice", requestId: number, name: string, candidates: Array<DeviceSummary>, } | { "type": "switchFailed", name: string, failedRoles: Array<Role>, } | { "type": "spatialUnavailable", name: string, mode: SpatialMode, reason: string, } | { "type": "hotkeyConflict", bindingId: string, keys: string, } | { "type": "configReplaced", backup: string, error: string, } | { "type": "configInvalid", error: string, } | { "type": "configSaveFailed", error: string, } | { "type": "settingRejected", error: string, };

export type EngineEvent = { "type": "devicesChanged", playback: Array<DeviceSummary>, recording: Array<DeviceSummary>,
/**
 * Mirrors `EngineSnapshot::disconnected` (empty while the setting is off).
 */
disconnected: Array<DeviceSummary>, } | { "type": "volumeChanged", volume: VolumeState, } | { "type": "micMuteChanged", muted: boolean, } | { "type": "micVolumeChanged", percent: number, } | { "type": "mediaChanged", media: NowPlaying | null, } | { "type": "switched", device: DeviceSummary, roles: RoleSet,
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
channels: Array<number>, };
