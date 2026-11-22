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

    /// Every saved device reference: bindings, both sides of toggle pairs, device preferences, favourites.
    pub fn device_refs_mut(&mut self) -> impl Iterator<Item = &mut DeviceRef> {
        self.bindings
            .iter_mut()
            .map(|b| &mut b.device)
            .chain(self.toggle_pairs.iter_mut().flat_map(|p| [&mut p.primary, &mut p.secondary]))
            .chain(self.devices.iter_mut().map(|d| &mut d.device))
            .chain(self.favourites.playback.iter_mut())
            .chain(self.favourites.recording.iter_mut())
    }

    /// After a confirmed switch to a re-created endpoint, points *every* reference to the old endpoint at the
    /// new one (review ENG-10), keeping each reference's display name and match mode. Returns how many changed.
    pub fn relink_all(&mut self, old_endpoint_id: &str, target: &crate::device::EndpointInfo) -> usize {
        let mut n = 0;
        for r in self.device_refs_mut() {
            if r.endpoint_id.eq_ignore_ascii_case(old_endpoint_id) && r.flow == target.flow {
                *r = r.relinked_to(target);
                n += 1;
            }
        }
        n
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
    /// The file was unusable; defaults are used. Tell the user. `backup` is where the broken file went,
    /// or `None` if it couldn't be moved (it's then left in place untouched).
    Replaced {
        config: Config,
        backup: Option<PathBuf>,
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
        // Millisecond stamp: quick restarts mustn't collide (review ENG-04).
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
        let target = self.path.with_extension(format!("invalid-{stamp}.toml"));
        let backup = std::fs::rename(&self.path, &target).ok().map(|()| target);
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

    /// The file's current text, for detecting external edits before an automatic save. `None` if missing.
    ///
    /// # Errors
    /// I/O errors other than "not found".
    pub fn read_text(&self) -> std::io::Result<Option<String>> {
        match std::fs::read_to_string(&self.path) {
            Ok(t) => Ok(Some(t)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Atomic save: write and flush a temp file next to the target, then rename over it. Windows briefly
    /// locks files that editors or antivirus have open, so the rename is retried; the temp file is
    /// removed if it never lands (review ENG-04). Returns the text written.
    ///
    /// # Errors
    /// I/O failures (folder not writable, disk full, file locked for longer than the retries).
    pub fn save(&self, cfg: &Config) -> Result<String, ConfigError> {
        use std::io::Write as _;
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(ConfigError::Io)?;
        }
        let text = cfg.to_toml();
        let tmp = self.path.with_extension("toml.tmp");
        let written = std::fs::File::create(&tmp).and_then(|mut f| {
            f.write_all(text.as_bytes())?;
            f.sync_all()
        });
        if let Err(e) = written {
            let _ = std::fs::remove_file(&tmp);
            return Err(ConfigError::Io(e));
        }
        let mut last = None;
        for attempt in 0..5 {
            match std::fs::rename(&tmp, &self.path) {
                Ok(()) => return Ok(text),
                Err(e) => last = Some(e),
            }
            std::thread::sleep(std::time::Duration::from_millis(20 * (attempt + 1)));
        }
        let _ = std::fs::remove_file(&tmp);
        Err(ConfigError::Io(last.unwrap_or_else(|| std::io::Error::other("rename failed"))))
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
                assert!(backup.is_some_and(|b| b.exists()) && !store.path().exists());
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

    #[test]
    fn relink_all_updates_every_reference_to_the_old_endpoint() {
        let mut cfg = owner_config();
        cfg.favourites.playback.push(dref("arctis", "Headphones (Arctis Nova 7)"));
        let new = crate::device::EndpointInfo {
            id: "arctis-new".into(),
            flow: Flow::Render,
            state: crate::device::DeviceState::Active,
            friendly_name: Some("Headphones (2- Arctis Nova 7)".into()),
            endpoint_name: Some("Headphones".into()),
            interface_name: Some("2- Arctis Nova 7".into()),
            devnode: None,
            hardware_key: Some(r"USB\VID_1038&PID_22A1&MI_00".into()),
            container_id: Some("{8EC256AB}".into()),
            form_factor: Some(3),
            stable_id: None,
            default_for: vec![],
        };
        // Pair primary + device prefs + favourite point at "arctis" (matched case-insensitively); the speakers don't.
        assert_eq!(cfg.relink_all("ARCTIS", &new), 3);
        assert_eq!(cfg.toggle_pairs[0].primary.endpoint_id, "arctis-new");
        assert_eq!(cfg.toggle_pairs[0].primary.display_name, "Headphones (Arctis Nova 7)");
        assert_eq!(cfg.toggle_pairs[0].primary.interface_name.as_deref(), Some("Arctis Nova 7"));
        assert_eq!(cfg.devices[0].device.endpoint_id, "arctis-new");
        assert_eq!(cfg.favourites.playback[0].endpoint_id, "arctis-new");
        assert_eq!(cfg.toggle_pairs[0].secondary.endpoint_id, "realtek");
    }
}
