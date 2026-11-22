// Auto-generated from the ice-core engine contract.
// Do not edit manually.

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
 * Volume of the default playback device.
 */
volume: VolumeState | null,
/**
 * Mute state of the default communications microphone.
 */
micMuted: boolean | null, };

export type Command = { "type": "switchTo", deviceId: string, roles: RoleSet, } | { "type": "switchBinding", bindingId: string, } | { "type": "togglePair", pairId: string, } | { "type": "cycleFavourites", flow: Flow, step: number, } | { "type": "setVolume", percent: number, } | { "type": "stepVolume", delta: number, } | { "type": "toggleMute" } | { "type": "toggleMicMute" } | { "type": "setSpatial", deviceId: string, mode: SpatialMode, } | { "type": "resolveChoice", requestId: number, deviceId: string | null, } | { "type": "refresh" };

export type Notice = { "type": "notConnected", name: string, } | { "type": "chooseDevice", requestId: number, name: string, candidates: Array<DeviceSummary>, } | { "type": "switchFailed", name: string, failedRoles: Array<Role>, } | { "type": "spatialUnavailable", name: string, mode: SpatialMode, reason: string, } | { "type": "hotkeyConflict", bindingId: string, keys: string, };

export type EngineEvent = { "type": "devicesChanged", playback: Array<DeviceSummary>, recording: Array<DeviceSummary>, } | { "type": "volumeChanged", volume: VolumeState, } | { "type": "micMuteChanged", muted: boolean, } | { "type": "switched", device: DeviceSummary, roles: RoleSet,
/**
 * The binding followed the device to a new endpoint ID (shown as a caption, one notice).
 */
relinked: boolean,
/**
 * Set when a spatial format was applied and read back.
 */
spatialApplied: SpatialMode | null, } | { "type": "deviceArrived", device: DeviceSummary, } | { "type": "deviceLeft", device: DeviceSummary, } | { "type": "notice", notice: Notice, };

export type Envelope = { revision: number, event: EngineEvent, };
