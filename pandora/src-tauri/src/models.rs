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
    pub quick_picks: Vec<Track>,
    pub trending: Vec<Track>,
    pub offline_fallback: bool,
    pub error: Option<String>,
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
pub struct EqState {
    pub enabled: bool,
    pub preset: String,
    pub bands: [f32; 10],
    pub normalize: bool,
    pub skip_silence: bool,
    pub playback_rate: f32,
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
        }
    }
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
