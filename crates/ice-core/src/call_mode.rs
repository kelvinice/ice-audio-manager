//! D-30 call mode (owner, 2026-11-25): lower the music while people talk on a call, and bring it back after a quiet
//! stretch. Pure logic only: the settings, which apps count as calls, matching the now-playing app to an audio
//! session's process, and the [`Ducker`] that turns call-audio peaks into a music gain. The Windows side (sessions,
//! peaks, volumes) lives in `ice-win::call_mode`.

use std::ops::RangeInclusive;
use std::time::Duration;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// Music level while people talk, as a % of the music's own level. 0 = silent; never above 90 % (no audible dip).
pub const CALL_DUCK_PERCENT: RangeInclusive<u8> = 0..=90;
pub const CALL_DUCK_PERCENT_DEFAULT: u8 = 30;
/// Quiet seconds after the last speech before the music comes back.
pub const CALL_HOLD_SECS: RangeInclusive<u8> = 1..=15;
pub const CALL_HOLD_SECS_DEFAULT: u8 = 3;
/// At most this many call apps, each name at most [`CALL_APP_NAME_MAX`] characters.
pub const CALL_APPS_MAX: usize = 16;
pub const CALL_APP_NAME_MAX: usize = 64;
/// How often the call app's level is read, only while one of its audio sessions is active (~15 Hz, §2).
pub const CALL_TICK: Duration = Duration::from_millis(66);
/// Speech must last this long before the music dips, so a click or a cough doesn't trigger it.
pub const CALL_ATTACK: Duration = Duration::from_millis(120);
/// Time for a full-scale fade down (music dips quickly) and back up (it returns gently).
pub const CALL_FADE_DOWN: Duration = Duration::from_millis(150);
pub const CALL_FADE_UP: Duration = Duration::from_millis(1000);
/// A late tick never moves the fade further than this (sleep/resume, a stalled thread).
const MAX_STEP: Duration = Duration::from_millis(250);

/// How loud call audio must be to count as talking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
pub enum CallSensitivity {
    /// Only clear speech (ignores quiet background noise in the call).
    Low,
    #[default]
    Normal,
    /// Also soft voices.
    High,
}

impl CallSensitivity {
    /// Peak (0.0–1.0) of the call app's session at or above which it counts as talking. Estimates for voice
    /// through Discord's noise suppression: speech peaks ~0.05–0.5, suppressed silence is ~0.
    #[must_use]
    pub const fn threshold(self) -> f32 {
        match self {
            Self::Low => 0.05,
            Self::Normal => 0.02,
            Self::High => 0.008,
        }
    }
}

/// `[call_mode]` in `config.toml` (schema v18).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct CallModeConfig {
    /// Off by default: the app never changes another app's volume unless asked to.
    pub enabled: bool,
    /// Executable names whose audio counts as a call (`Discord.exe`; `.exe` optional, any case).
    pub apps: Vec<String>,
    /// Music level while people talk, % of its own level ([`CALL_DUCK_PERCENT`]).
    pub duck_percent: u8,
    /// Quiet seconds before the music comes back ([`CALL_HOLD_SECS`]).
    pub hold_secs: u8,
    pub sensitivity: CallSensitivity,
}

impl Default for CallModeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            apps: default_call_apps(),
            duck_percent: CALL_DUCK_PERCENT_DEFAULT,
            hold_secs: CALL_HOLD_SECS_DEFAULT,
            sensitivity: CallSensitivity::Normal,
        }
    }
}

/// Discord's desktop builds (stable, PTB, Canary). Teams, Zoom and others can be added in Settings.
#[must_use]
pub fn default_call_apps() -> Vec<String> {
    ["Discord.exe", "DiscordPTB.exe", "DiscordCanary.exe"].map(String::from).to_vec()
}

impl CallModeConfig {
    /// # Errors
    /// A user-facing reason when a value is out of range or an app name is invalid.
    pub fn validate(&self) -> Result<(), String> {
        if !CALL_DUCK_PERCENT.contains(&self.duck_percent) {
            return Err(format!(
                "The music level while talking must be {}–{} %.",
                CALL_DUCK_PERCENT.start(),
                CALL_DUCK_PERCENT.end()
            ));
        }
        if !CALL_HOLD_SECS.contains(&self.hold_secs) {
            return Err(format!(
                "The wait before the music comes back must be {}–{} seconds.",
                CALL_HOLD_SECS.start(),
                CALL_HOLD_SECS.end()
            ));
        }
        normalize_apps(&self.apps).map(|_| ())
    }

    /// Whether audio from this process (an exe file name such as `Discord.exe`) counts as a call.
    #[must_use]
    pub fn is_call_app(&self, exe: &str) -> bool {
        let stem = exe_stem(exe);
        !stem.is_empty() && self.apps.iter().any(|a| exe_stem(a) == stem)
    }

    /// The music's gain while people talk (0.0–0.9).
    #[must_use]
    pub fn duck_gain(&self) -> f32 {
        f32::from(self.duck_percent.min(*CALL_DUCK_PERCENT.end())) / 100.0
    }

    #[must_use]
    pub fn hold(&self) -> Duration {
        Duration::from_secs(u64::from(self.hold_secs))
    }
}

/// Trims, checks and de-duplicates call app names (case-insensitive, `.exe` optional), keeping their order.
///
/// # Errors
/// A user-facing reason for an empty, too long, path-like or control-character name, or too many apps.
pub fn normalize_apps(apps: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for raw in apps {
        let name = raw.trim();
        if name.is_empty() {
            continue;
        }
        if name.chars().count() > CALL_APP_NAME_MAX {
            return Err(format!("\"{name}\" is too long for an app name."));
        }
        if name.chars().any(|c| c.is_control() || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')) {
            return Err(format!("Use the app's file name, like Discord.exe, not \"{name}\"."));
        }
        if !out.iter().any(|o| exe_stem(o) == exe_stem(name)) {
            out.push(name.to_string());
        }
    }
    if out.len() > CALL_APPS_MAX {
        return Err(format!("Add at most {CALL_APPS_MAX} call apps."));
    }
    Ok(out)
}

/// `C:\…\Discord.exe` → `discord`: the file name, lower-case, without `.exe`.
#[must_use]
pub fn exe_stem(name: &str) -> String {
    let file = name.rsplit(['\\', '/']).next().unwrap_or(name).trim();
    let lower = file.to_ascii_lowercase();
    lower.strip_suffix(".exe").unwrap_or(&lower).to_string()
}

/// Whether a GSMTC now-playing `app_id` (`Spotify.exe`, `Chrome._crx_…`, `MSEdge`) is the process `exe`. Apps
/// with an AppUserModelID that doesn't name their exe (Firefox, Media Player) never match, so nothing is lowered.
#[must_use]
pub fn app_id_matches_exe(app_id: &str, exe: &str) -> bool {
    let stem = exe_stem(exe);
    if stem.is_empty() {
        return false;
    }
    let id = exe_stem(app_id);
    let before = |sep: char| id.split(sep).next().unwrap_or_default().to_string();
    id == stem || before('.') == stem || before('!') == stem || before('_') == stem
}

/// Once talking, the call stays "talking" while its peak is at least this share of the threshold (hysteresis), so a
/// level wobbling around the threshold can't flip the music up and down. Replaces an adaptive background estimate
/// (2026-11-25) that re-armed on any single quiet tick and made lively call audio pump the music.
pub const CALL_RELEASE_RATIO: f32 = 0.5;
/// Multiplier and floor margin over the endpoint loopback bleed when cancelling music bleed from a call session's
/// peak meter. Certain Windows audio drivers / virtual mixers reflect the post-master-volume endpoint mix into
/// capture-capable apps' render session meters (measured: `ep_peak = 0.073`, quiet `Discord.exe = 0.078`, real
/// speech `0.839`).
pub const LOOPBACK_BLEED_SCALE: f32 = 1.25;
pub const LOOPBACK_BLEED_MARGIN: f32 = 0.01;
const BLEED_ACTIVE_FLOOR: f32 = 0.005;

/// Cancels endpoint loopback bleed out of a call app's session peak when the now-playing music app is actively
/// producing output on that same endpoint. Returns the residual speech peak (`0.0..=1.0`).
#[must_use]
pub fn clean_call_peak(session_peak: f32, endpoint_peak: f32, music_output_peak: f32) -> f32 {
    let sanitize = |v: f32| if v.is_finite() { v.max(0.0) } else { 0.0 };
    let session = sanitize(session_peak);
    let endpoint = sanitize(endpoint_peak);
    let music = sanitize(music_output_peak);
    if music <= BLEED_ACTIVE_FLOOR || endpoint <= BLEED_ACTIVE_FLOOR {
        return session;
    }
    let bleed = endpoint.min(music);
    let ceiling = bleed * LOOPBACK_BLEED_SCALE + LOOPBACK_BLEED_MARGIN;
    (session - ceiling).max(0.0)
}

/// Turns the call app's peak level into the music's gain: lowered after [`CALL_ATTACK`] above the sensitivity
/// threshold, held while the call stays above [`CALL_RELEASE_RATIO`] × that threshold, and faded back after `hold` of
/// quiet. Pure and time-driven by the caller (`step(peak, dt)`).
#[derive(Debug, Clone)]
pub struct Ducker {
    threshold: f32,
    floor: f32,
    hold: Duration,
    gain: f32,
    talking: bool,
    loud_for: Duration,
    quiet_for: Duration,
}

impl Ducker {
    #[must_use]
    pub fn new(config: &CallModeConfig) -> Self {
        Self {
            threshold: config.sensitivity.threshold(),
            floor: config.duck_gain(),
            hold: config.hold(),
            gain: 1.0,
            talking: false,
            loud_for: Duration::ZERO,
            quiet_for: Duration::ZERO,
        }
    }

    /// New settings take effect from the next step; the current fade carries on from where it is.
    pub fn configure(&mut self, config: &CallModeConfig) {
        self.threshold = config.sensitivity.threshold();
        self.floor = config.duck_gain();
        self.hold = config.hold();
    }

    /// Feeds the loudest call-audio peak (0.0–1.0) since the last step, `dt` later. Returns the music gain (0.0–1.0).
    pub fn step(&mut self, peak: f32, dt: Duration) -> f32 {
        let dt = dt.min(MAX_STEP);
        let peak = if peak.is_finite() { peak.max(0.0) } else { 0.0 };
        let threshold = if self.talking { self.threshold * CALL_RELEASE_RATIO } else { self.threshold };
        if peak >= threshold {
            self.loud_for += dt;
            self.quiet_for = Duration::ZERO;
            // If speech resumes mid-fade-up, re-duck immediately instead of waiting for attack while volume swells.
            if self.loud_for >= CALL_ATTACK || (!self.talking && self.gain < 1.0) {
                self.talking = true;
            }
        } else {
            self.loud_for = Duration::ZERO;
            if self.talking {
                self.quiet_for += dt;
                if self.quiet_for >= self.hold {
                    self.talking = false;
                    self.quiet_for = Duration::ZERO;
                }
            }
        }
        let target = if self.talking { self.floor } else { 1.0 };
        let fade = if target < self.gain { CALL_FADE_DOWN } else { CALL_FADE_UP };
        let max_move = dt.as_secs_f32() / fade.as_secs_f32();
        self.gain = if (target - self.gain).abs() <= max_move {
            target
        } else {
            self.gain + max_move.copysign(target - self.gain)
        };
        self.gain
    }

    #[must_use]
    pub const fn gain(&self) -> f32 {
        self.gain
    }

    /// People are talking (the music is going down or staying down), as opposed to holding or fading back.
    #[must_use]
    pub const fn talking(&self) -> bool {
        self.talking
    }

    /// Nothing is lowered and nothing is pending: the caller may stop ticking.
    #[must_use]
    pub fn is_resting(&self) -> bool {
        !self.talking && self.loud_for.is_zero() && self.gain >= 1.0
    }

    /// Back to full level at once (call mode turned off, app quitting).
    pub fn reset(&mut self) {
        self.gain = 1.0;
        self.talking = false;
        self.loud_for = Duration::ZERO;
        self.quiet_for = Duration::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TICK: Duration = CALL_TICK;

    fn run(d: &mut Ducker, peak: f32, ticks: usize) -> f32 {
        (0..ticks).fold(d.gain(), |_, _| d.step(peak, TICK))
    }

    fn on() -> CallModeConfig {
        CallModeConfig { enabled: true, ..CallModeConfig::default() }
    }

    #[test]
    fn speech_lowers_the_music_and_quiet_brings_it_back_after_the_hold() {
        let mut d = Ducker::new(&on());
        assert!(d.is_resting());
        // One loud tick (a click) is shorter than the attack: nothing happens.
        assert!((d.step(0.3, TICK) - 1.0).abs() < f32::EPSILON);
        assert!((d.step(0.0, TICK) - 1.0).abs() < f32::EPSILON);
        // Sustained speech: reaches the 30 % floor within the attack + fade-down.
        let g = run(&mut d, 0.3, 6);
        assert!((g - 0.3).abs() < 1e-6, "ducked to {g}");
        // Quiet shorter than the 3 s hold: still lowered.
        let g = run(&mut d, 0.0, 40);
        assert!((g - 0.3).abs() < 1e-6, "still ducked at {g}");
        // After the hold, it fades up over ~0.7 s and rests.
        let g = run(&mut d, 0.0, 6 + 12);
        assert!((g - 1.0).abs() < 1e-6, "restored to {g}");
        assert!(d.is_resting());
    }

    #[test]
    fn talking_again_during_the_hold_keeps_it_lowered() {
        let mut d = Ducker::new(&on());
        run(&mut d, 0.3, 6);
        run(&mut d, 0.0, 30); // 2 s quiet
        run(&mut d, 0.3, 2); // speaks again
        let g = run(&mut d, 0.0, 30); // another 2 s: the hold restarted
        assert!((g - 0.3).abs() < 1e-6);
    }

    #[test]
    fn a_level_wobbling_around_the_threshold_never_pumps() {
        // Low sensitivity (0.05), 1 s hold, silent: the owner's settings when the glitch was reported.
        let cfg = CallModeConfig { sensitivity: CallSensitivity::Low, hold_secs: 1, duck_percent: 0, ..on() };
        let mut d = Ducker::new(&cfg);
        run(&mut d, 0.2, 4);
        assert!(d.talking());
        // 20 s of call audio wobbling 0.03–0.06 (below the 0.05 threshold most of the time, above half of it).
        for i in 0..300 {
            let g = d.step(if i % 3 == 0 { 0.06 } else { 0.03 }, TICK);
            assert!(g.abs() < 1e-6, "the music came back at tick {i} (gain {g})");
        }
        // Real quiet still brings it back after the hold.
        assert!((run(&mut d, 0.0, 16 + 16) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn long_talk_with_pauses_between_words_stays_lowered() {
        let mut d = Ducker::new(&on());
        // 30 s of speech: 4 loud ticks, 2 near-silent ones (gaps between words).
        for _ in 0..75 {
            run(&mut d, 0.2, 4);
            run(&mut d, 0.002, 2);
        }
        assert!((d.gain() - 0.3).abs() < 1e-6, "a long talk stays lowered, at {}", d.gain());
    }

    #[test]
    fn sensitivity_sets_the_threshold() {
        let low = CallModeConfig { sensitivity: CallSensitivity::Low, ..on() };
        let mut d = Ducker::new(&low);
        assert!((run(&mut d, 0.03, 10) - 1.0).abs() < f32::EPSILON, "0.03 is below Low");
        let high = CallModeConfig { sensitivity: CallSensitivity::High, ..on() };
        let mut d = Ducker::new(&high);
        assert!(run(&mut d, 0.01, 10) < 1.0, "0.01 is above High");
    }

    #[test]
    fn a_late_tick_never_jumps_the_fade() {
        let mut d = Ducker::new(&on());
        run(&mut d, 0.3, 6);
        // A 10 s gap (sleep) counts as one 250 ms step: still lowered, not restored in one go.
        assert!(d.step(0.0, Duration::from_secs(10)) < 1.0);
        assert!(d.step(f32::NAN, TICK) < 1.0, "NaN counts as quiet");
    }

    #[test]
    fn validates_ranges_and_app_names() {
        assert!(on().validate().is_ok());
        assert!(CallModeConfig { duck_percent: 95, ..on() }.validate().is_err());
        assert!(CallModeConfig { hold_secs: 0, ..on() }.validate().is_err());
        assert!(CallModeConfig { apps: vec!["C:\\x\\Discord.exe".into()], ..on() }.validate().is_err());
        let apps = vec![" Zoom ".into(), "zoom.exe".into(), String::new(), "ms-teams.exe".into()];
        assert_eq!(normalize_apps(&apps), Ok(vec!["Zoom".to_string(), "ms-teams.exe".to_string()]));
        let many: Vec<String> = (0..=CALL_APPS_MAX).map(|i| format!("app{i}.exe")).collect();
        assert!(normalize_apps(&many).is_err());
    }

    #[test]
    fn call_apps_match_by_exe_stem() {
        let c = on();
        assert!(c.is_call_app("Discord.exe"));
        assert!(c.is_call_app("C:\\Users\\x\\AppData\\Local\\Discord\\app-1.0\\discord.EXE"));
        assert!(c.is_call_app("DiscordCanary.exe"));
        assert!(!c.is_call_app("Spotify.exe"));
        assert!(!c.is_call_app(""));
    }

    #[test]
    fn now_playing_ids_match_their_process() {
        assert!(app_id_matches_exe("Spotify.exe", "Spotify.exe"));
        assert!(app_id_matches_exe("Chrome._crx_cinhimbnkkaeohfgghhklpknlkffjgod", "chrome.exe"));
        assert!(app_id_matches_exe("MSEdge", "msedge.exe"));
        assert!(!app_id_matches_exe("308046B0AF4A39CB", "firefox.exe"));
        assert!(!app_id_matches_exe(
            "Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic",
            "Microsoft.Media.Player.exe"
        ));
        assert!(!app_id_matches_exe("Spotify.exe", ""));
    }

    #[test]
    fn loopback_bleed_is_cancelled_without_hiding_speech() {
        // Quiet Discord reporting endpoint bleed (~0.078 while endpoint peak is 0.073 and Spotify plays 0.873).
        assert!((clean_call_peak(0.078, 0.073, 0.873) - 0.0).abs() < f32::EPSILON);
        // Real speech on top of the same music bleed stays strongly above any sensitivity threshold.
        let speech = clean_call_peak(0.839, 0.073, 0.873);
        assert!(speech > 0.7, "expected clear speech residual, got {speech}");
        // Soft speech above the bleed ceiling also passes Low sensitivity (0.05).
        let soft = clean_call_peak(0.18, 0.073, 0.873);
        assert!(soft > CallSensitivity::Low.threshold(), "expected soft speech > 0.05, got {soft}");
        // When music is ducked to 0 or paused, raw call peaks pass untouched.
        assert!((clean_call_peak(0.06, 0.06, 0.0) - 0.06).abs() < f32::EPSILON);
        assert!((clean_call_peak(0.06, 0.0, 0.8) - 0.06).abs() < f32::EPSILON);
    }

    #[test]
    fn loopback_bleed_feedback_loop_never_oscillates() {
        let cfg = CallModeConfig { sensitivity: CallSensitivity::Low, hold_secs: 1, duck_percent: 0, ..on() };
        let mut d = Ducker::new(&cfg);
        for tick in 0..150 {
            let music_out = 0.873 * d.gain();
            let ep_peak = 0.073 * d.gain();
            let discord_bleed = 0.078 * d.gain();
            let cleaned = clean_call_peak(discord_bleed, ep_peak, music_out);
            let gain = d.step(cleaned, TICK);
            assert!((gain - 1.0).abs() < f32::EPSILON, "oscillated at tick {tick}: gain={gain}");
        }
    }

    #[test]
    fn resuming_speech_during_fade_up_re_ducks_immediately() {
        let cfg = CallModeConfig { hold_secs: 1, ..on() };
        let mut d = Ducker::new(&cfg);
        run(&mut d, 0.3, 6);
        assert!(d.talking());
        // Hold expires and fade-up begins.
        run(&mut d, 0.0, 18);
        assert!(!d.talking() && d.gain() < 1.0);
        // Single tick of speech re-engages immediately without waiting for full attack.
        let before = d.gain();
        let after = d.step(0.2, TICK);
        assert!(d.talking());
        assert!(after <= before, "expected immediate re-ducking: before={before} after={after}");
    }
}
