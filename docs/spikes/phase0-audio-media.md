# Phase 0.3 / 0.4 findings: audio, spatial sound, media (Claude)

> Measured 2026-11-22 on the owner's PC (Windows 11 build 26200, Ryzen 5 3600X, debug build of the spikes, un-elevated).
> Code: [`crates/ice-win`](../../crates/ice-win) (reusable) + [`spikes/audio`](../../spikes/audio) / [`spikes/media`](../../spikes/media) (throwaway CLIs).

## Summary

| Question | Result |
|---|---|
| Enumerate endpoints incl. ghosts, with identity fields | ✅ `ice_win::endpoints` |
| Switch default per role via `IPolicyConfig` | ✅ All 3 roles, **~142 ms for 3 calls** (~47 ms each), confirmed by re-reading defaults |
| Endpoint notifications (`IMMNotificationClient`) | ✅ Registered; events go to a channel. Hot-plug behaviour still needs the owner (see "Still to do") |
| Volume change events without a keyboard hook (`IAudioEndpointVolumeCallback`) | ✅ Registered; follows the default device when it changes |
| Peak meter (`IAudioMeterInformation`) | ✅ Reads at ~30 Hz (0.000 while silent, as expected) |
| `PKEY_AudioEndpoint_StableId` | ✅ **Found**: `{1DA5D803-D492-4EDD-8C23-E0C0FFEE7F0E},12` (see below) |
| Spatial sound read/write (D-17) | ✅ **Documented WinRT API**, works **un-elevated** from an unpackaged exe |
| Media / now playing (GSMTC) with Spotify | ✅ Manager in ~25 ms, session read in ~21 ms, album art stream available (~109 KB) |
| Footprint of the Rust audio core (release `watch`: notifications + volume callback, idle) | **1.8 MB private, 10.8 MB working set, 0 ms CPU over 3 s.** The `Lite` profile (D-15) has lots of headroom |
| Binary size (release, LTO, stripped) | audio-spike 403 KB, media-spike 166 KB |

## D-16: device identity

**The bug is visible on the owner's machine right now.** Windows currently names the headset
**"Headphones (2- Arctis Nova 7)"**. The `2-` prefix is how Windows disambiguates a re-installed instance of a device
it has seen before (e.g. after plugging the dongle into a different USB port). The endpoint ID (`…{2608ca0f-…}`)
did *not* change this time, but **anything matching by name** ("Headphones (Arctis Nova 7)") breaks. The legacy
Python app matched names by substring across *all* endpoints including ghosts. That explains the dead hotkey.

**Stable ID:** the windows crate (0.62) and SDK 10.0.26100 headers don't define `PKEY_AudioEndpoint_StableId`.
`audio-spike stable` probed every string property of every active endpoint and kept those that
`IMMDeviceEnumerator::GetDevice` resolves back to the same endpoint. Exactly one key qualifies, on every device:

```
{1DA5D803-D492-4EDD-8C23-E0C0FFEE7F0E},12  =  {0.0.0.00000000}.{001.{473DEC6D-…}}   (Arctis render)
```

Same fmtid as the other `PKEY_AudioEndpoint_*` keys, `VT_LPWSTR`, accepted by `GetDevice`. That matches Microsoft's
description. It's now `ice_win::props::STABLE_ID`, and `EndpointInfo.stable_id` is populated.

**Identity fields available per endpoint** (all populated on this machine):
endpoint ID · stable ID · friendly name · endpoint name ("Headphones") · interface name ("2- Arctis Nova 7") ·
devnode (`{1}.USB\VID_1038&PID_22A1&MI_00\…`) → hardware key `USB\VID_1038&PID_22A1&MI_00` · container ID · form factor
(Arctis render = 3 Headphones, Realtek = 1 Speakers, monitors = 9 SPDIF/HDMI, Arctis mic = 4).

**Resolver implication:** never rely on `interface_name`/`friendly_name` equality (the `2-` prefix changes them).
Match order: endpoint ID → stable ID → hardware key + container + form factor + endpoint name ("Headphones"),
with name only as a weak tie-breaker after stripping a leading `N- ` prefix.

## D-17: spatial sound

`Windows.Media.Audio.SpatialAudioDeviceConfiguration` (WinRT, documented, Windows 10 1903+) **does everything we need**:
read active/selected format, list formats, set the format. No undocumented API is required. This corrects the
D-17 premise.

| Test | Result |
|---|---|
| Read Arctis | supported, **active = selected = Dolby Atmos for Headphones** |
| Read others | active = selected = Off (Windows reports Off as the **null GUID**) |
| Set Windows Sonic on the unused *Steam Streaming Speakers* | `Succeeded` in **37 ms**, read-back confirmed |
| Set Off with an empty string | **`AccessDenied`**. Off must be passed as the **null GUID** |
| Set Off with the null GUID | `Succeeded` in **25 ms**, read-back confirmed (test device restored) |
| Arctis set to Atmos when it's already Atmos | skipped by read-before-write |

Caveat: `IsSpatialAudioFormatSupported` returned the same 5 formats for every endpoint (it seems to report installed
providers, not per-device licence state). Treat a `LicenseNotValidForAudioEndpoint` / `NotSupportedOnAudioEndpoint`
status on write as the real answer, and re-read after writing (already in D-17's amendments).

## Media (Phase 0.4)

`GlobalSystemMediaTransportControlsSessionManager` with Spotify desktop: 1 session (`Spotify.exe`), title/artist/album,
playback status, thumbnail stream 108,946 bytes. Phase 3 should use the manager/session **events** (not the spike's 4 Hz poll)
and decode/resize art in Rust (D-03 `iceart://`).

## Still to do (needs the owner, ~5 minutes)

The reconnect cases can't be simulated from software. With the Arctis connected, run:

```powershell
cargo run -p audio-spike -- watch --log spikes/audio/out/churn.jsonl
```

…then: (1) unplug/replug the dongle in the **same** port, (2) a **different** port, (3) power-cycle the headset,
(4) sleep/resume the PC. Press Ctrl+C and share `spikes/audio/out/churn.jsonl` (it stays local and is gitignored).
That shows which of endpoint ID / stable ID / container / name change in each case, and finalises the D-16 resolver weights.

## F31 knob test (owner, 2026-11-22)

`audio-spike hotkeys 60` with the owner's keyboard volume knob:

- **Ctrl/Shift/Alt + turn all reach `RegisterHotKey`** as `<Mod>+VolumeUp/VolumeDown`. No hook needed, so F31 is feasible.
- Plain turns didn't fire, so they keep changing Windows' volume as before (D-07 A).
- **Fast spins send a detent every 13–40 ms** (slow turns 150–600 ms). Design consequences:
  - **Ctrl+turn (cycle device)** must not switch per detent. Spinning moves a highlight through a device list in the notch,
    and the switch commits ~400 ms after the last detent, Alt+Tab style. One switch per gesture.
  - **Shift+turn (1% steps)** and **Alt+turn (mic volume)** act per detent, coalesced into one volume write per frame.
- `Ctrl+VolumeMute` (knob press) did not fire. Pending the owner: whether the knob press was tried, and what it sends.
