//! User configuration, `config.toml` schema v1 (D-04).
//!
//! Location: `%APPDATA%\IceAudioManager\config.toml` (chosen by the app). Every field has a default, so a
//! missing or partial file is valid. Unknown future `schema_version`s are rejected rather than
//! misread, and a bad file is backed up and replaced by defaults (code-practices §7).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::{RoleSet, SpatialMode};
use crate::hotkey::Chord;
use crate::resolve::DeviceRef;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    /// Roles set by switches that don't specify their own.
    pub default_roles: RoleSet,
    /// Device hotkeys: press → switch to that device.
    #[serde(rename = "binding", skip_serializing_if = "Vec::is_empty")]
    pub bindings: Vec<Binding>,
    /// Toggle pairs, e.g. Speakers ↔ Arctis (the owner's main workflow, D-16).
    #[serde(rename = "toggle_pair", skip_serializing_if = "Vec::is_empty")]
    pub toggle_pairs: Vec<TogglePair>,
    /// Per-device behaviour applied on switch (spatial sound, D-17).
    #[serde(rename = "device", skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<DevicePrefs>,
    pub favourites: Favourites,
    pub knob: KnobGestures,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            default_roles: RoleSet::Default,
            bindings: Vec::new(),
            toggle_pairs: Vec::new(),
            devices: Vec::new(),
            favourites: Favourites::default(),
            knob: KnobGestures::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub id: String,
    pub hotkey: Option<Chord>,
    pub device: DeviceRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roles: Option<RoleSet>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TogglePair {
    pub id: String,
    pub hotkey: Option<Chord>,
    /// Chosen when a third device has taken over (D-16 amendment).
    pub primary: DeviceRef,
    pub secondary: DeviceRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roles: Option<RoleSet>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevicePrefs {
    pub device: DeviceRef,
    /// Applied after a confirmed switch to this device (read-before-write, D-17).
    #[serde(default)]
    pub spatial: SpatialMode,
    /// Shown instead of Windows' name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Favourites {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub playback: Vec<DeviceRef>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recording: Vec<DeviceRef>,
    /// Hotkey for "next favourite playback device".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle_hotkey: Option<Chord>,
}

/// F31: modified knob turns (`RegisterHotKey` on Mod+VolumeUp/Down; plain turns stay Windows').
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct KnobGestures {
    pub enabled: bool,
    /// Ctrl+turn: browse favourites in the notch, commit after the knob rests (Alt+Tab style).
    pub cycle_device_modifier: Option<String>,
    /// Shift+turn: fine volume steps.
    pub fine_volume_modifier: Option<String>,
    pub fine_step_percent: u8,
    /// Alt+turn: microphone volume.
    pub mic_volume_modifier: Option<String>,
    /// Quiet time after the last detent before a device browse commits.
    pub commit_after_ms: u32,
}

impl Default for KnobGestures {
    fn default() -> Self {
        Self {
            enabled: false,
            cycle_device_modifier: Some("Ctrl".into()),
            fine_volume_modifier: Some("Shift".into()),
            fine_step_percent: 1,
            mic_volume_modifier: Some("Alt".into()),
            commit_after_ms: 400,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(String),
    /// Written by a newer app version; never guess at its meaning.
    TooNew(u32),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "config I/O error: {e}"),
            Self::Parse(e) => write!(f, "config is invalid: {e}"),
            Self::TooNew(v) => write!(f, "config schema {v} is newer than this app supports ({SCHEMA_VERSION})"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Parses TOML, applying defaults and migrations.
    ///
    /// # Errors
    /// Invalid TOML, unknown fields, bad hotkeys, or a schema newer than [`SCHEMA_VERSION`].
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        let cfg: Self = toml::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))?;
        match cfg.schema_version {
            v if v > SCHEMA_VERSION => Err(ConfigError::TooNew(v)),
            // 0 = file written before versioning; nothing to migrate yet.
            _ => Ok(Self { schema_version: SCHEMA_VERSION, ..cfg }),
        }
    }

    /// Serialises to commented-free, stable TOML.
    ///
    /// # Panics
    /// Never in practice: every field is TOML-representable.
    #[must_use]
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("config is always serialisable")
    }

    /// The device preferences entry for a device, if any (matched by the resolver, so it survives re-links).
    #[must_use]
    pub fn prefs_for(&self, endpoint: &crate::device::EndpointInfo) -> Option<&DevicePrefs> {
        let one = std::slice::from_ref(endpoint);
        self.devices.iter().find(|p| crate::resolve::resolve(&p.device, one).target().is_some())
    }
}

/// Loads and saves the config file.
pub struct ConfigStore {
    path: PathBuf,
}

/// What [`ConfigStore::load`] did.
#[derive(Debug)]
pub enum Loaded {
    Ok(Config),
    /// No file yet: defaults.
    Missing(Config),
    /// The file was unusable; it was renamed to `backup` and defaults are used. Tell the user.
    Replaced {
        config: Config,
        backup: PathBuf,
        error: String,
    },
}

impl Loaded {
    #[must_use]
    pub fn config(&self) -> &Config {
        match self {
            Self::Ok(c) | Self::Missing(c) | Self::Replaced { config: c, .. } => c,
        }
    }
}

impl ConfigStore {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Never fails: a broken file is backed up and replaced by defaults (the app keeps running).
    #[must_use]
    pub fn load(&self) -> Loaded {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Loaded::Missing(Config::default()),
            Err(e) => return self.replace_broken(&e.to_string()),
        };
        match Config::from_toml(&text) {
            Ok(c) => Loaded::Ok(c),
            Err(e) => self.replace_broken(&e.to_string()),
        }
    }

    fn replace_broken(&self, error: &str) -> Loaded {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let backup = self.path.with_extension(format!("invalid-{stamp}.toml"));
        let _ = std::fs::rename(&self.path, &backup);
        Loaded::Replaced { config: Config::default(), backup, error: error.to_string() }
    }

    /// Non-destructive read for hot reload: never renames the file (the user may be mid-edit).
    /// `Ok(None)` if the file doesn't exist.
    ///
    /// # Errors
    /// I/O or parse errors, so the caller can keep the last valid config and tell the user.
    pub fn read(&self) -> Result<Option<Config>, ConfigError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Config::from_toml(&text).map(Some),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ConfigError::Io(e)),
        }
    }

    /// Atomic save: write a temp file next to the target, then rename over it.
    ///
    /// # Errors
    /// I/O failures (folder not writable, disk full).
    pub fn save(&self, cfg: &Config) -> Result<(), ConfigError> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(ConfigError::Io)?;
        }
        let tmp = self.path.with_extension("toml.tmp");
        std::fs::write(&tmp, cfg.to_toml()).map_err(ConfigError::Io)?;
        std::fs::rename(&tmp, &self.path).map_err(ConfigError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::Flow;
    use crate::resolve::MatchMode;

    fn dref(id: &str, name: &str) -> DeviceRef {
        DeviceRef {
            endpoint_id: id.into(),
            stable_id: None,
            flow: Flow::Render,
            display_name: name.into(),
            endpoint_name: Some("Headphones".into()),
            interface_name: Some("Arctis Nova 7".into()),
            hardware_key: Some(r"USB\VID_1038&PID_22A1&MI_00".into()),
            container_id: Some("{8EC256AB}".into()),
            form_factor: Some(3),
            match_mode: MatchMode::SameModel,
        }
    }

    fn owner_config() -> Config {
        Config {
            toggle_pairs: vec![TogglePair {
                id: "main".into(),
                hotkey: Some("Ctrl+Alt+F12".parse().unwrap()),
                primary: dref("arctis", "Headphones (Arctis Nova 7)"),
                secondary: dref("realtek", "Speakers (Realtek(R) Audio)"),
                roles: None,
            }],
            devices: vec![DevicePrefs {
                device: dref("arctis", "Headphones (Arctis Nova 7)"),
                spatial: SpatialMode::DolbyAtmosHeadphones,
                alias: None,
                hidden: false,
            }],
            ..Config::default()
        }
    }

    #[test]
    fn empty_file_is_all_defaults() {
        assert_eq!(Config::from_toml("").unwrap(), Config::default());
    }

    #[test]
    fn round_trips_the_owner_setup() {
        let cfg = owner_config();
        let text = cfg.to_toml();
        assert!(text.contains("[[toggle_pair]]") && text.contains(r#"hotkey = "Ctrl+Alt+F12""#), "{text}");
        assert!(text.contains(r#"spatial = "dolbyAtmosHeadphones""#), "{text}");
        assert_eq!(Config::from_toml(&text).unwrap(), cfg);
    }

    #[test]
    fn rejects_newer_schema_bad_hotkeys_and_typos() {
        assert!(matches!(Config::from_toml("schema_version = 99"), Err(ConfigError::TooNew(99))));
        let bad_key = "[[binding]]\nid='x'\nhotkey='VolumeUp'\n[binding.device]\nendpoint_id='a'\nflow='render'\ndisplay_name='A'";
        assert!(matches!(Config::from_toml(bad_key), Err(ConfigError::Parse(_))));
        assert!(matches!(Config::from_toml("defualt_roles = 'both'"), Err(ConfigError::Parse(_))));
    }

    #[test]
    fn store_saves_atomically_and_backs_up_broken_files() {
        let dir = std::env::temp_dir().join(format!("ice-cfg-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = ConfigStore::new(dir.join("config.toml"));
        assert!(matches!(store.load(), Loaded::Missing(_)));
        store.save(&owner_config()).unwrap();
        assert!(matches!(store.load(), Loaded::Ok(c) if c == owner_config()));
        std::fs::write(store.path(), "this is [ not toml").unwrap();
        match store.load() {
            Loaded::Replaced { config, backup, .. } => {
                assert_eq!(config, Config::default());
                assert!(backup.exists() && !store.path().exists());
            }
            other => panic!("expected Replaced, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hot_reload_read_never_renames_a_broken_file() {
        let dir = std::env::temp_dir().join(format!("ice-cfg-read-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = ConfigStore::new(dir.join("config.toml"));
        assert!(matches!(store.read(), Ok(None)));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(store.path(), "half-typed = [").unwrap();
        assert!(matches!(store.read(), Err(ConfigError::Parse(_))));
        assert!(store.path().exists(), "the user's file must stay where it is");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
