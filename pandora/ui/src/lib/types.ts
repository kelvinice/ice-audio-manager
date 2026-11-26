export type TrackSource = "ytmusic" | "local" | "demo";

export interface Track {
  id: string;
  source: TrackSource;
  title: string;
  artist: string;
  album: string;
  durationMs: number | null;
  artworkUrl: string | null;
  localPath: string | null;
  cached: boolean;
  liked: boolean;
  // Optional client-side fields for bundled offline demo tracks
  demoDataUrl?: string;
  demoLrc?: string;
}

export interface Playlist {
  id: string;
  name: string;
  trackIds: string[];
  createdAtMs: number;
}

export interface HomeFeed {
  quickPicks: Track[];
  trending: Track[];
  offlineFallback: boolean;
  error: string | null;
}

export interface SearchResponse {
  query: string;
  tracks: Track[];
  error: string | null;
}

export interface EqState {
  enabled: boolean;
  preset: string;
  bands: number[];
  normalize: boolean;
  skipSilence: boolean;
  playbackRate: number;
}

export interface LibraryState {
  liked: Track[];
  playlists: Playlist[];
  history: Track[];
  localFolders: string[];
  localTracks: Track[];
  tracksIndex: Record<string, Track>;
  cachedIds: string[];
  cacheBytes: number;
  eq: EqState;
  iceModePref: "auto" | "embedded" | "connected";
}

export interface IceStatus {
  mode: "embedded" | "connected" | "fake" | "unavailable";
  standaloneRunning: boolean;
  standaloneExe: string | null;
  configPath: string;
  error: string | null;
}

export type SpatialMode =
  | "keep"
  | "off"
  | "windowsSonic"
  | "dolbyAtmosHeadphones"
  | "dolbyAtmosHomeTheater"
  | "dolbyAtmosSpeakers"
  | "dtsHeadphoneX"
  | "dtsXUltra"
  | "dtsXHomeTheater";

export interface DeviceSummary {
  id: string;
  name: string;
  flow: "render" | "capture";
  state: string;
  isDefault: boolean;
  isDefaultCommunications: boolean;
  formFactor: number | null;
  isVirtual: boolean;
  spatial: SpatialMode | null;
  lyricsDelayMs: number;
}

export interface VolumeState {
  percent: number;
  muted: boolean;
}

export interface IceSettingsView {
  appearance: {
    theme: string;
    preset: string;
    idleStyle: string;
    notchStyle: string;
    nowPlaying: string;
    movingBackdrop: boolean;
    hibernate: boolean;
  };
  performanceProfile: string;
  lyrics: {
    enabled: boolean;
    onlineLookup: boolean;
    romaji: boolean;
    estimatedWordSync: boolean;
  };
  visualizerSpectrum: boolean;
  callMode: {
    enabled: boolean;
    duckPercent: number;
    holdSecs: number;
  };
}

export interface EngineSnapshot {
  revision: number;
  playback: DeviceSummary[];
  recording: DeviceSummary[];
  disconnected: DeviceSummary[];
  volume: VolumeState | null;
  micMuted: boolean | null;
  micVolume: number | null;
  settings: IceSettingsView;
}

export interface MeterFrame {
  peak: number;
  channels: number[];
  bands?: number[] | null;
}

export interface IceConfigPatch {
  theme?: string;
  notchStyle?: string;
  hibernate?: boolean;
  movingBackdrop?: boolean;
  callModeEnabled?: boolean;
  lyricsEnabled?: boolean;
  lyricsOnlineLookup?: boolean;
  lyricsRomaji?: boolean;
  lyricsEstimatedWordSync?: boolean;
  visualizerSpectrum?: boolean;
  performanceProfile?: string;
}

export interface PandoraLyricWord {
  startMs: number;
  endMs: number;
  text: string;
}

export interface PandoraLyricLine {
  timeMs: number;
  endMs: number | null;
  text: string;
  subtitle: string | null;
  romaji: string | null;
  words: PandoraLyricWord[];
}

export interface PandoraLyrics {
  trackKey: string;
  status: "idle" | "loading" | "synced" | "plain" | "instrumental" | "not_found" | "error";
  source: string | null;
  lines: PandoraLyricLine[];
  plainText: string | null;
  error: string | null;
}
