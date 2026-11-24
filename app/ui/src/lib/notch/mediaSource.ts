/**
 * Names the app a media session comes from (`NowPlaying.appId`), for the Edge Nook header ("Spotify", "YouTube Music",
 * "Chrome"…). Pure; no network. `appId` is GSMTC's source id: an exe name (`Spotify.exe`) or an AppUserModelID
 * (`Chrome._crx_…`, `Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic`).
 */
export type MediaSourceIcon = 'spotify' | 'youtube-music' | 'music';

export interface MediaSource {
  label: string;
  icon: MediaSourceIcon;
}

const KNOWN: { test: RegExp; source: MediaSource }[] = [
  { test: /spotify/i, source: { label: 'Spotify', icon: 'spotify' } },
  { test: /music\.youtube|youtube ?music|ytm/i, source: { label: 'YouTube Music', icon: 'youtube-music' } },
  { test: /youtube/i, source: { label: 'YouTube', icon: 'youtube-music' } },
  { test: /zunemusic|media ?player/i, source: { label: 'Media Player', icon: 'music' } },
  { test: /msedge|microsoftedge/i, source: { label: 'Edge', icon: 'music' } },
  { test: /chrome/i, source: { label: 'Chrome', icon: 'music' } },
  { test: /firefox/i, source: { label: 'Firefox', icon: 'music' } },
];

export function mediaSource(appId: string | null | undefined): MediaSource {
  const id = (appId ?? '').trim();
  for (const known of KNOWN) {
    if (known.test.test(id)) return known.source;
  }
  // Unknown app: the last path/AUMID segment without `.exe`, e.g. `C:\…\foobar2000.exe` → "foobar2000".
  const last = id.split(/[\\/!]/).filter(Boolean).pop() ?? '';
  const name = last.replace(/\.exe$/i, '').replace(/_[a-z0-9]{13}$/i, '');
  return { label: name.length > 0 && name.length <= 24 ? name : 'Media', icon: 'music' };
}
