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
  /** Shelves in YouTube Music's order: song rows (Quick picks) and playlist / album rows. */
  shelves: HomeShelf[];
  /** Every song across the shelves. */
  quickPicks: Track[];
  offlineFallback: boolean;
  error: string | null;
}

export interface HomeShelf {
  title: string;
  strapline: string | null;
  tracks: Track[];
  collections: CollectionCard[];
}

export type CollectionKind = "playlist" | "album" | "artist";

export interface CollectionCard {
  browseId: string;
  kind: CollectionKind;
  title: string;
  subtitle: string;
  artworkUrl: string | null;
}

export interface CollectionPage extends CollectionCard {
  tracks: Track[];
}

export interface SearchResponse {
  query: string;
  tracks: Track[];
  error: string | null;
}

export interface ParametricBand {
  filterType: string;
  frequency: number;
  gain: number;
  q: number;
}

export interface ParametricEqProfile {
  name: string;
  preamp: number;
  bands: ParametricBand[];
}

export interface EqState {
  enabled: boolean;
  preset: string;
  bands: number[];
  normalize: boolean;
  skipSilence: boolean;
  playbackRate: number;
  parametric?: ParametricEqProfile | null;
}

export interface RecognizedTrack {
  id: string;
  title: string;
  artist: string;
  album: string | null;
  artworkUrl: string | null;
  genre: string | null;
  releaseDate: string | null;
  label: string | null;
  lyricsSnippet: string | null;
  shazamUrl: string | null;
  matchedTrack: Track | null;
  source: "system" | "mic" | string;
  recognizedAtMs: number;
}

export interface RecognitionProgress {
  state: "listening" | "matching" | "done" | "cancelled" | "error";
  elapsedMs: number;
  level: number;
}

export type PandoraTheme = "midnight" | "ice" | "iceLight" | "daylight" | "ember" | "system";

export interface PandoraSettings {
  theme: PandoraTheme;
  /** Buttons and accents follow the playing cover instead of the theme (Rust `artwork_accents`). */
  artworkAccents: boolean;
  animatedCanvas: boolean;
  showRomaji: boolean;
  autoQueueRelated: boolean;
  defaultVolume: number;
  mediaSessionEnabled: boolean;
  autoLaunchIceNotch: boolean;
  showIceCardInSidebar: boolean;
  wavyProgress: boolean;
  spinningCover: boolean;
  closeToTray: boolean;
  /** Disk budget for cached artwork in MiB; 0 keeps artwork in memory only. */
  artworkCacheMb: number;
  lyricsStyle: LyricsStyle;
  backdropMotion: BackdropMotion;
  experimental: ExperimentalSettings;
}

/** Settings → Experimental (Rust `ExperimentalSettings`), all off by default. */
export interface ExperimentalSettings {
  /** Songs fade in, fade out over their last seconds, and skips dip instead of cutting. */
  smoothTransitions: boolean;
  /** Now Playing's cover pulses on bass kicks. */
  beatPulse: boolean;
  /** The sleep timer fades the music out over its last 30 seconds. */
  gentleSleep: boolean;
}

/**
 * Now Playing's animated background (Rust `BackdropMotion`): the blurred cover under a faint lava lamp (default), slow
 * circles, or Ice's full lava lamp.
 */
export type BackdropMotion = "blend" | "calm" | "lively";

/** Now Playing lyric styles (Rust `LyricsStyle`). */
export type LyricsStyle = "default" | "fisheye" | "rain";

export const DEFAULT_PANDORA_SETTINGS: PandoraSettings = {
  theme: "midnight",
  artworkAccents: false,
  animatedCanvas: true,
  showRomaji: true,
  autoQueueRelated: true,
  defaultVolume: 0.85,
  mediaSessionEnabled: true,
  autoLaunchIceNotch: false,
  showIceCardInSidebar: true,
  wavyProgress: true,
  spinningCover: false,
  closeToTray: false,
  artworkCacheMb: 256,
  lyricsStyle: "default",
  backdropMotion: "blend",
  experimental: { smoothTransitions: false, beatPulse: false, gentleSleep: false },
};

export interface AppPaths {
  libraryPath: string;
  cacheDir: string;
  artworkDir: string;
  iceConfigPath: string;
  iceBridgePath: string;
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
  settings: PandoraSettings;
  recognitionHistory: RecognizedTrack[];
}

export interface IceStatus {
  mode: "embedded" | "connected" | "fake" | "unavailable";
  standaloneRunning: boolean;
  standaloneExe: string | null;
  configPath: string;
  bridgeConnected: boolean;
  bridgePort: number | null;
  bridgePid: number | null;
  notchExpanded: boolean | null;
  iceVersion: string | null;
  bridgeInfoPath: string;
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

export interface PlaylistImportResult {
  library: LibraryState;
  playlistId: string;
  playlistName: string;
  source: "ytmusic" | "spotify" | "text" | string;
  totalTracks: number;
  importedTracks: number;
  unmatchedTracks: string[];
  note: string | null;
}

export interface PlaylistImportProgress {
  stage: "fetching" | "matching" | "done" | "cancelled" | "error";
  source: string;
  completed: number;
  total: number;
  matched: number;
  currentTrack: string | null;
}
