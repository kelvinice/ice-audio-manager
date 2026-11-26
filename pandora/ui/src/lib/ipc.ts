import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  EngineSnapshot,
  EqState,
  HomeFeed,
  IceConfigPatch,
  IceStatus,
  LibraryState,
  MeterFrame,
  PandoraLyrics,
  SearchResponse,
  SpatialMode,
  Track,
} from "./types";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function isTauriEnv(): boolean {
  return inTauri;
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) {
    throw new Error(`Tauri runtime unavailable for ${cmd}`);
  }
  return invoke<T>(cmd, args);
}

export async function getProxyPort(): Promise<number> {
  if (!inTauri) return 0;
  return call<number>("get_proxy_port");
}

export function ytmHome(): Promise<HomeFeed> {
  if (!inTauri) {
    return Promise.resolve({ quickPicks: [], trending: [], offlineFallback: true, error: null });
  }
  return call<HomeFeed>("ytm_home");
}

export function ytmSearch(query: string, filter = "songs"): Promise<SearchResponse> {
  if (!inTauri) {
    return Promise.resolve({ query, tracks: [], error: "Preview mode (outside Tauri)" });
  }
  return call<SearchResponse>("ytm_search", { query, filter });
}

export function ytmRelated(videoId: string): Promise<Track[]> {
  if (!inTauri) return Promise.resolve([]);
  return call<Track[]>("ytm_related", { videoId });
}

export function fetchLyrics(
  title: string,
  artist: string,
  album: string,
  durationMs: number | null,
  lrcOverride: string | null = null,
): Promise<PandoraLyrics> {
  if (!inTauri) {
    return Promise.resolve({
      trackKey: `${title}-${artist}`,
      status: "not_found",
      source: null,
      lines: [],
      plainText: null,
      error: null,
    });
  }
  return call<PandoraLyrics>("fetch_lyrics", {
    title,
    artist,
    album,
    durationMs,
    lrcOverride,
  });
}

export function getLibrary(): Promise<LibraryState> {
  if (!inTauri) {
    return Promise.resolve({
      liked: [],
      playlists: [],
      history: [],
      localFolders: [],
      localTracks: [],
      tracksIndex: {},
      cachedIds: [],
      cacheBytes: 0,
      eq: {
        enabled: true,
        preset: "Flat",
        bands: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        normalize: false,
        skipSilence: false,
        playbackRate: 1,
      },
      iceModePref: "auto",
    });
  }
  return call<LibraryState>("get_library");
}

export const toggleLikeTrack = (track: Track) => call<LibraryState>("toggle_like", { track });
export const recordHistoryTrack = (track: Track) => call<LibraryState>("record_history", { track });
export const createPlaylist = (name: string) => call<LibraryState>("create_playlist", { name });
export const deletePlaylist = (playlistId: string) => call<LibraryState>("delete_playlist", { playlistId });
export const addToPlaylist = (playlistId: string, track: Track) =>
  call<LibraryState>("add_to_playlist", { playlistId, track });
export const removeFromPlaylist = (playlistId: string, trackId: string) =>
  call<LibraryState>("remove_from_playlist", { playlistId, trackId });
export const saveEqState = (eq: EqState) => call<LibraryState>("save_eq", { eq });
export const addLocalFolder = (path: string) => call<LibraryState>("add_local_folder", { path });
export const removeLocalFolder = (path: string) => call<LibraryState>("remove_local_folder", { path });
export const rescanLocalFolders = () => call<LibraryState>("rescan_local");
export const downloadTrackOffline = (track: Track) => call<LibraryState>("download_track", { track });
export const deleteCachedTrack = (trackId: string) => call<LibraryState>("delete_cached_track", { trackId });
export const clearOfflineCache = () => call<LibraryState>("clear_offline_cache");

export const getIceStatus = () => call<IceStatus>("ice_status");
export const getIceSnapshot = () => call<EngineSnapshot>("ice_snapshot");
export const setIceModePref = (pref: "auto" | "embedded" | "connected") =>
  call<IceStatus>("ice_set_mode_pref", { pref });
export const switchIceDevice = (deviceId: string) => call<void>("ice_switch_device", { deviceId });
export const setIceVolume = (percent: number) => call<void>("ice_set_volume", { percent });
export const toggleIceMute = () => call<void>("ice_toggle_mute");
export const toggleIceMicMute = () => call<void>("ice_toggle_mic_mute");
export const setIceSpatial = (deviceId: string, mode: SpatialMode) =>
  call<void>("ice_set_spatial", { deviceId, mode });
export const patchIceSettings = (patch: IceConfigPatch) => call<EngineSnapshot>("ice_patch_settings", { patch });
export const setIceMeterActive = (active: boolean) => call<void>("ice_set_meter_active", { active });
export const launchIceStandalone = () => call<IceStatus>("ice_launch_standalone");
export const stopIceStandalone = () => call<IceStatus>("ice_stop_standalone");

export async function onIceStatus(cb: (status: IceStatus) => void): Promise<UnlistenFn> {
  if (!inTauri) return () => {};
  return listen<IceStatus>("ice://status/v1", (e) => cb(e.payload));
}

export async function onIceSnapshot(cb: (snap: EngineSnapshot) => void): Promise<UnlistenFn> {
  if (!inTauri) return () => {};
  return listen<EngineSnapshot>("ice://snapshot/v1", (e) => cb(e.payload));
}

export async function onIceMeter(cb: (frame: MeterFrame) => void): Promise<UnlistenFn> {
  if (!inTauri) return () => {};
  return listen<MeterFrame>("ice://meter/v1", (e) => cb(e.payload));
}
