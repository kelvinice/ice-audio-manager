use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    /// `"ytmusic" | "local" | "demo"`
    pub source: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: Option<u64>,
    pub artwork_url: Option<String>,
    pub local_path: Option<String>,
    #[serde(default)]
    pub cached: bool,
    #[serde(default)]
    pub liked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub track_ids: Vec<String>,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeFeed {
    /// Shelves in the order YouTube Music returns them ("Quick picks", editorial rows, new releases, ...).
    pub shelves: Vec<HomeShelf>,
    /// Every playable song across the shelves, in order (hero pick and "play all").
    pub quick_picks: Vec<Track>,
    pub offline_fallback: bool,
    pub error: Option<String>,
}

/// One titled row of the home feed. A shelf holds songs, collections (playlists / albums), or both.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeShelf {
    pub title: String,
    /// Small label above the title ("TOP TUNES RIGHT NOW").
    pub strapline: Option<String>,
    pub tracks: Vec<Track>,
    pub collections: Vec<CollectionCard>,
}

/// A playlist, album or artist tile that opens with `ytm_collection`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionCard {
    pub browse_id: String,
    /// `"playlist" | "album" | "artist"`
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub artwork_url: Option<String>,
}

/// A collection opened from the feed: its header plus playable tracks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionPage {
    pub browse_id: String,
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub artwork_url: Option<String>,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub query: String,
    pub tracks: Vec<Track>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParametricBand {
    /// `"PK" | "LSC" | "HSC" | "LPQ" | "HPQ"`
    pub filter_type: String,
    pub frequency: f32,
    pub gain: f32,
    pub q: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParametricEqProfile {
    pub name: String,
    pub preamp: f32,
    pub bands: Vec<ParametricBand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqState {
    pub enabled: bool,
    pub preset: String,
    pub bands: [f32; 10],
    pub normalize: bool,
    pub skip_silence: bool,
    pub playback_rate: f32,
    #[serde(default)]
    pub parametric: Option<ParametricEqProfile>,
}

impl Default for EqState {
    fn default() -> Self {
        Self {
            enabled: true,
            preset: "Flat".into(),
            bands: [0.0; 10],
            normalize: false,
            skip_silence: false,
            playback_rate: 1.0,
            parametric: None,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecognizedTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub artwork_url: Option<String>,
    pub genre: Option<String>,
    pub release_date: Option<String>,
    pub label: Option<String>,
    pub lyrics_snippet: Option<String>,
    pub shazam_url: Option<String>,
    pub matched_track: Option<Track>,
    pub source: String,
    pub recognized_at_ms: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PandoraTheme {
    Daylight,
    Ember,
    System,
    #[default]
    #[serde(other)]
    Midnight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PandoraSettings {
    pub theme: PandoraTheme,
    pub dynamic_theme: bool,
    pub animated_canvas: bool,
    pub show_romaji: bool,
    pub auto_queue_related: bool,
    pub default_volume: f32,
    pub media_session_enabled: bool,
    pub auto_launch_ice_notch: bool,
    pub show_ice_card_in_sidebar: bool,
    #[serde(default = "default_true")]
    pub wavy_progress: bool,
    #[serde(default)]
    pub spinning_cover: bool,
    #[serde(default)]
    pub close_to_tray: bool,
    /// Disk budget for cached artwork, in MiB (0 keeps artwork in memory only).
    #[serde(default = "default_artwork_cache_mb")]
    pub artwork_cache_mb: u32,
}

fn default_artwork_cache_mb() -> u32 {
    256
}


impl Default for PandoraSettings {
    fn default() -> Self {
        Self {
            theme: PandoraTheme::default(),
            dynamic_theme: true,
            animated_canvas: true,
            show_romaji: true,
            auto_queue_related: true,
            default_volume: 0.85,
            media_session_enabled: true,
            auto_launch_ice_notch: false,
            show_ice_card_in_sidebar: true,
            wavy_progress: true,
            spinning_cover: false,
            close_to_tray: false,
            artwork_cache_mb: default_artwork_cache_mb(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPaths {
    pub library_path: String,
    pub cache_dir: String,
    pub artwork_dir: String,
    pub ice_config_path: String,
    pub ice_bridge_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryState {
    pub liked: Vec<Track>,
    pub playlists: Vec<Playlist>,
    pub history: Vec<Track>,
    pub local_folders: Vec<String>,
    pub local_tracks: Vec<Track>,
    pub tracks_index: std::collections::HashMap<String, Track>,
    pub cached_ids: Vec<String>,
    pub cache_bytes: u64,
    pub eq: EqState,
    /// `"auto" | "embedded" | "connected"`
    pub ice_mode_pref: String,
    #[serde(default)]
    pub settings: PandoraSettings,
    #[serde(default)]
    pub recognition_history: Vec<RecognizedTrack>,
}

impl Default for LibraryState {
    fn default() -> Self {
        Self {
            liked: Vec::new(),
            playlists: Vec::new(),
            history: Vec::new(),
            local_folders: Vec::new(),
            local_tracks: Vec::new(),
            tracks_index: std::collections::HashMap::new(),
            cached_ids: Vec::new(),
            cache_bytes: 0,
            eq: EqState::default(),
            ice_mode_pref: "auto".into(),
            settings: PandoraSettings::default(),
            recognition_history: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IceStatus {
    /// `"embedded" | "connected" | "fake" | "unavailable"`
    pub mode: String,
    pub standalone_running: bool,
    pub standalone_exe: Option<String>,
    pub config_path: String,
    pub bridge_connected: bool,
    pub bridge_port: Option<u16>,
    pub bridge_pid: Option<u32>,
    pub notch_expanded: Option<bool>,
    pub ice_version: Option<String>,
    pub bridge_info_path: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IceConfigPatch {
    pub theme: Option<String>,
    pub notch_style: Option<String>,
    pub hibernate: Option<bool>,
    pub moving_backdrop: Option<bool>,
    pub call_mode_enabled: Option<bool>,
    pub lyrics_enabled: Option<bool>,
    pub lyrics_online_lookup: Option<bool>,
    pub lyrics_romaji: Option<bool>,
    pub lyrics_estimated_word_sync: Option<bool>,
    pub visualizer_spectrum: Option<bool>,
    pub performance_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PandoraLyricWord {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PandoraLyricLine {
    pub time_ms: u64,
    pub end_ms: Option<u64>,
    pub text: String,
    pub subtitle: Option<String>,
    pub romaji: Option<String>,
    pub words: Vec<PandoraLyricWord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PandoraLyrics {
    pub track_key: String,
    /// `"idle" | "loading" | "synced" | "plain" | "instrumental" | "not_found" | "error"`
    pub status: String,
    pub source: Option<String>,
    pub lines: Vec<PandoraLyricLine>,
    pub plain_text: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistImportResult {
    pub library: LibraryState,
    pub playlist_id: String,
    pub playlist_name: String,
    /// `"ytmusic" | "spotify" | "text"`
    pub source: String,
    pub total_tracks: usize,
    pub imported_tracks: usize,
    pub unmatched_tracks: Vec<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistImportProgress {
    /// `"fetching" | "matching" | "done" | "cancelled" | "error"`
    pub stage: String,
    pub source: String,
    pub completed: usize,
    pub total: usize,
    pub matched: usize,
    pub current_track: Option<String>,
}
