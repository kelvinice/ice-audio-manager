//! Phase 0.3 audio spike. Throwaway CLI; the reusable code lives in `ice-win`.
//!
//! ```text
//! audio-spike list [--all]                 endpoints (+ ghosts with --all), defaults, identity fields
//! audio-spike props <device>               dump every property of one endpoint
//! audio-spike stable                       probe which property is PKEY_AudioEndpoint_StableId
//! audio-spike watch [--seconds N] [--log F] endpoint + volume events; JSONL churn log (default out/churn.jsonl)
//! audio-spike switch <device> [default|comms|all]   set default, then re-read and report what changed
//! audio-spike peak [seconds]               peak meter of the default playback device at ~30 Hz
//! audio-spike spatial [device]             spatial sound status (all active playback devices if omitted)
//! audio-spike spatial-set <device> <off|sonic|atmos|atmos-ht|atmos-spk|dts-hx|dtsx-ultra|dtsx-ht>
//! audio-spike hotkeys [seconds]            F31 knob test: registers Ctrl/Shift/Alt + volume keys, prints presses
//! audio-spike make-config [path]           write the owner's setup (Arctis ↔ Speakers on Ctrl+Alt+F12, Atmos, knob)
//! audio-spike engine [path] [seconds]      run the real engine with that config and print its events
//! audio-spike resolve [path]              read-only: how each saved device in the config resolves right now
//! audio-spike settings-test                set up a toggle pair through engine commands on a throwaway config (no switching)
//! ```
//! Config path defaults to `spikes/audio/out/config.toml` (gitignored).
//! `<device>` = an endpoint ID, or a case-insensitive part of the name of an *Active* endpoint.

use std::io::Write as _;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ice_core::device::{DeviceState, EndpointInfo, Flow, Role};
use ice_core::event::AudioEvent;
use ice_win::com::ComGuard;
use ice_win::endpoints::Endpoints;
use ice_win::spatial::{self, SpatialFormat};
use ice_win::{meter, notify, policy, props, volume};

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> AnyResult<()> {
    let _com = ComGuard::init_mta()?;
    let eps = Endpoints::new()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).map(String::as_str);
    match arg(0).unwrap_or("list") {
        "list" => list(&eps, args.iter().any(|a| a == "--all")),
        "props" => dump_props(&eps, arg(1).ok_or("usage: props <device>")?),
        "stable" => probe_stable(&eps),
        "watch" => {
            let secs = flag_value(&args, "--seconds").and_then(|s| s.parse().ok());
            let log = flag_value(&args, "--log").unwrap_or("spikes/audio/out/churn.jsonl");
            watch(&eps, secs, log)
        }
        "switch" => switch(&eps, arg(1).ok_or("usage: switch <device> [default|comms|all]")?, arg(2).unwrap_or("all")),
        "peak" => peak(&eps, arg(1).and_then(|s| s.parse().ok()).unwrap_or(5)),
        "spatial" => spatial_status(&eps, arg(1)),
        "spatial-set" => spatial_set(
            &eps,
            arg(1).ok_or("usage: spatial-set <device> <format>")?,
            arg(2).ok_or("usage: spatial-set <device> <format>")?,
        ),
        "hotkeys" => hotkeys(arg(1).and_then(|s| s.parse().ok()).unwrap_or(30)),
        "make-config" => make_config(&eps, arg(1).unwrap_or(DEFAULT_CONFIG)),
        "resolve" => resolve_saved(&eps, arg(1).unwrap_or(DEFAULT_CONFIG)),
        "settings-test" => settings_test(),
        "engine" => run_engine(arg(1).unwrap_or(DEFAULT_CONFIG), arg(2).and_then(|s| s.parse().ok()).unwrap_or(120)),
        other => Err(format!("unknown command `{other}` (see the header of spikes/audio/src/main.rs)").into()),
    }
}

const DEFAULT_CONFIG: &str = "spikes/audio/out/config.toml";

/// Exercises the settings commands on the real engine with a throwaway config. Switches nothing.
fn settings_test() -> AnyResult<()> {
    use ice_core::config::ConfigStore;
    use ice_core::engine::{Command, Engine, EngineEvent};
    use ice_win::audio_engine::AudioEngine;
    let path = "spikes/audio/out/settings-test.toml";
    let _ = std::fs::remove_file(path);
    let engine = AudioEngine::start(ConfigStore::new(path))?;
    let events = engine.subscribe();
    let snap = engine.snapshot();
    let find = |needle: &str| snap.playback.iter().find(|d| d.name.contains(needle)).map(|d| d.id.clone());
    let (arctis, speakers) = (find("Arctis").ok_or("no Arctis")?, find("Realtek").ok_or("no Realtek")?);
    let wait = |label: &str| {
        while let Ok(env) = events.recv_timeout(Duration::from_secs(3)) {
            match env.event {
                EngineEvent::SettingsChanged { settings } => {
                    println!(
                        "{label}: SettingsChanged -> {} pair(s), {} device pref(s)",
                        settings.toggle_pairs.len(),
                        settings.devices.len()
                    );
                    for p in &settings.toggle_pairs {
                        println!(
                            "    {} [{}]: {} ↔ {}",
                            p.id,
                            p.hotkey.as_deref().unwrap_or("-"),
                            p.primary.name,
                            p.secondary.name
                        );
                    }
                    return;
                }
                EngineEvent::Notice { notice } => {
                    println!("{label}: {notice:?}");
                    return;
                }
                _ => {}
            }
        }
        println!("{label}: (no event)");
    };
    engine.send(Command::SetTogglePair {
        pair_id: "main".into(),
        primary_id: arctis.clone(),
        secondary_id: speakers.clone(),
        hotkey: Some("Ctrl+Alt+F10".into()),
        roles: None,
    });
    wait("set pair");
    engine.send(Command::SetDevicePrefs {
        device_id: arctis.clone(),
        spatial: Some(ice_core::engine::SpatialMode::DolbyAtmosHeadphones),
        alias: None,
        hidden: None,
    });
    wait("atmos pref");
    engine.send(Command::SetBinding {
        binding_id: "x".into(),
        device_id: speakers,
        hotkey: Some("Ctrl+Alt+F10".into()),
        roles: None,
    });
    wait("duplicate hotkey");
    engine.send(Command::SetBinding {
        binding_id: "x".into(),
        device_id: arctis,
        hotkey: Some("VolumeUp".into()),
        roles: None,
    });
    wait("plain volume key");
    drop(engine);
    println!(
        "
{path}:
{}",
        std::fs::read_to_string(path)?
    );
    Ok(())
}

/// Read-only diagnostic: resolves every saved reference against the live endpoints. Changes nothing.
fn resolve_saved(eps: &Endpoints, path: &str) -> AnyResult<()> {
    use ice_core::config::Config;
    use ice_core::resolve::{Resolution, resolve};
    let cfg = Config::from_toml(&std::fs::read_to_string(path)?)?;
    let live = eps.list(true)?;
    let mut refs: Vec<(String, ice_core::resolve::DeviceRef)> = Vec::new();
    for p in &cfg.toggle_pairs {
        refs.push((format!("pair {} primary", p.id), p.primary.clone()));
        refs.push((format!("pair {} secondary", p.id), p.secondary.clone()));
    }
    refs.extend(cfg.bindings.iter().map(|b| (format!("binding {}", b.id), b.device.clone())));
    refs.extend(cfg.devices.iter().map(|d| ("device prefs".to_string(), d.device.clone())));
    refs.extend(cfg.favourites.playback.iter().map(|r| ("favourite".to_string(), r.clone())));
    for (what, r) in refs {
        let res = match resolve(&r, &live) {
            Resolution::Exact(e) => format!("Exact            → {}", e.id),
            Resolution::StableId(e) => format!("StableId (moved) → {}", e.id),
            Resolution::Relinked(e) => format!("Relinked (moved) → {}", e.id),
            Resolution::NeedsConfirmation(e) => format!("ask the user     → {}?", e.id),
            Resolution::Ambiguous => "Ambiguous (picker)".to_string(),
            Resolution::NotConnected => "NOT CONNECTED".to_string(),
        };
        println!("{what:<22} {:<32} {res}", r.display_name);
    }
    Ok(())
}

/// Writes the owner's workflow as a config: toggle pair Arctis (primary) ↔ Realtek speakers on Ctrl+Alt+F12,
/// Dolby Atmos applied when switching to the Arctis, both as favourites, knob gestures on.
fn make_config(eps: &Endpoints, path: &str) -> AnyResult<()> {
    use ice_core::config::{Config, ConfigStore, DevicePrefs, Favourites, KnobGestures, TogglePair};
    use ice_core::engine::SpatialMode;
    use ice_core::resolve::{DeviceRef, MatchMode};
    let find = |needle: &str| -> AnyResult<EndpointInfo> {
        eps.list(false)?
            .into_iter()
            .find(|e| e.flow == Flow::Render && e.friendly_name.as_deref().unwrap_or_default().contains(needle))
            .ok_or_else(|| format!("no active playback device containing `{needle}`").into())
    };
    let arctis = DeviceRef::from_endpoint(&find("Arctis")?, MatchMode::SameModel);
    let speakers = DeviceRef::from_endpoint(&find("Realtek")?, MatchMode::SameModel);
    let cfg = Config {
        toggle_pairs: vec![TogglePair {
            id: "main".into(),
            hotkey: Some("Ctrl+Alt+F12".parse()?),
            primary: arctis.clone(),
            secondary: speakers.clone(),
            roles: None,
        }],
        devices: vec![DevicePrefs {
            device: arctis.clone(),
            spatial: SpatialMode::DolbyAtmosHeadphones,
            alias: None,
            hidden: false,
        }],
        favourites: Favourites { playback: vec![arctis, speakers], ..Favourites::default() },
        knob: KnobGestures { enabled: true, ..KnobGestures::default() },
        ..Config::default()
    };
    ConfigStore::new(path).save(&cfg)?;
    println!("wrote {path}:\n\n{}", cfg.to_toml());
    Ok(())
}

fn run_engine(path: &str, secs: u64) -> AnyResult<()> {
    use ice_core::config::ConfigStore;
    use ice_core::engine::{Engine, EngineEvent};
    use ice_win::audio_engine::AudioEngine;
    let started = Instant::now();
    let engine = AudioEngine::start(ConfigStore::new(path))?;
    let events = engine.subscribe();
    let snap = engine.snapshot();
    println!("engine ready in {:?} (revision {})", started.elapsed(), snap.revision);
    for d in &snap.playback {
        println!(
            "  {} {}{}{}",
            if d.is_default { "▶" } else { " " },
            d.name,
            d.spatial.map(|s| format!("  [{s:?}]")).unwrap_or_default(),
            if d.is_virtual { "  (virtual)" } else { "" }
        );
    }
    println!("volume {:?}, mic muted {:?}, mic volume {:?}", snap.volume, snap.mic_muted, snap.mic_volume);
    for n in &snap.notices {
        println!("  ! {n:?}");
    }
    println!(
        "\nfor {secs} s: press Ctrl+Alt+F12 (toggle), Ctrl+knob (browse, commits when it rests), Shift+knob (1% steps),"
    );
    println!("Alt+knob (mic volume), unplug/replug the headset…");
    let t0 = Instant::now();
    while let Some(left) = Duration::from_secs(secs).checked_sub(t0.elapsed()) {
        let Ok(env) = events.recv_timeout(left) else { continue };
        let what = match &env.event {
            EngineEvent::Switched { device, roles, relinked, spatial_applied } => format!(
                "SWITCHED → {} ({roles:?}){}{}",
                device.name,
                if *relinked { " [re-linked]" } else { "" },
                spatial_applied.map(|s| format!(" [{s:?}]")).unwrap_or_default()
            ),
            EngineEvent::DevicesChanged { playback, .. } => format!(
                "devices: {}",
                playback
                    .iter()
                    .map(|d| if d.is_default { format!("▶{}", d.name) } else { d.name.clone() })
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
            other => format!("{other:?}"),
        };
        println!("{:>8} ms  r{:<4} {what}", t0.elapsed().as_millis(), env.revision);
    }
    Ok(())
}

/// F31 check: do knob turns with a modifier held reach `RegisterHotKey`? Plain turns are *not* registered,
/// so they keep changing Windows' volume as usual.
fn hotkeys(secs: u64) -> AnyResult<()> {
    use ice_core::hotkey::Chord;
    let chords = [
        "Ctrl+VolumeUp",
        "Ctrl+VolumeDown",
        "Shift+VolumeUp",
        "Shift+VolumeDown",
        "Alt+VolumeUp",
        "Alt+VolumeDown",
        "Ctrl+VolumeMute",
        "Ctrl+Alt+F12",
    ];
    let (tx, rx) = mpsc::channel();
    let svc = ice_win::hotkeys::HotkeyService::start(move |id| {
        let _ = tx.send(id);
    })?;
    let mut parsed = Vec::new();
    for (i, s) in chords.iter().enumerate() {
        let chord: Chord = s.parse()?;
        let id = i32::try_from(i).unwrap_or(0) + 1;
        match svc.register(id, chord, true) {
            Ok(()) => println!("registered  {chord}"),
            Err(e) => println!("FAILED      {e}"),
        }
        parsed.push(chord);
    }
    println!(
        "\nfor {secs} s: turn/press the knob while holding Ctrl, Shift or Alt; also try plain turns (should NOT print)"
    );
    let start = Instant::now();
    while let Some(left) = Duration::from_secs(secs).checked_sub(start.elapsed()) {
        if let Ok(id) = rx.recv_timeout(left) {
            let name = usize::try_from(id - 1)
                .ok()
                .and_then(|i| parsed.get(i))
                .map_or_else(|| "?".into(), ToString::to_string);
            println!("{:>8} ms  {name}", start.elapsed().as_millis());
        }
    }
    Ok(())
}

fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn unix_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis())
}

/// Resolves `<device>` to one endpoint: exact ID, else a unique name match among Active endpoints.
fn resolve(eps: &Endpoints, query: &str) -> AnyResult<EndpointInfo> {
    let all = eps.list(true)?;
    if let Some(e) = all.iter().find(|e| e.id.eq_ignore_ascii_case(query)) {
        return Ok(e.clone());
    }
    let q = query.to_lowercase();
    let hits: Vec<_> = all
        .into_iter()
        .filter(|e| e.state == DeviceState::Active)
        .filter(|e| e.friendly_name.as_deref().unwrap_or_default().to_lowercase().contains(&q))
        .collect();
    match hits.len() {
        1 => Ok(hits.into_iter().next().unwrap_or_else(|| unreachable!())),
        0 => Err(format!("no Active endpoint matches `{query}`").into()),
        _ => Err(format!(
            "`{query}` is ambiguous: {}",
            hits.iter().map(|e| e.friendly_name.clone().unwrap_or_default()).collect::<Vec<_>>().join(" | ")
        )
        .into()),
    }
}

fn roles_label(roles: &[Role]) -> String {
    roles
        .iter()
        .map(|r| match r {
            Role::Console => "C",
            Role::Multimedia => "M",
            Role::Communications => "X",
        })
        .collect()
}

fn list(eps: &Endpoints, all: bool) -> AnyResult<()> {
    let mut v = eps.list(all)?;
    v.sort_by(|a, b| (a.flow as u8, &a.friendly_name).cmp(&(b.flow as u8, &b.friendly_name)));
    println!(
        "{:<7} {:<10} {:<3} {:<46} {:<32} {:<9} {:>2}  ID",
        "FLOW", "STATE", "DEF", "NAME", "HARDWARE KEY", "CONTAINER", "FF"
    );
    for e in &v {
        println!(
            "{:<7} {:<10} {:<3} {:<46} {:<32} {:<9} {:>2}  {}",
            format!("{:?}", e.flow),
            format!("{:?}", e.state),
            roles_label(&e.default_for),
            trunc(e.friendly_name.as_deref().unwrap_or("?"), 46),
            trunc(e.hardware_key.as_deref().unwrap_or("-"), 32),
            e.container_id.as_deref().map_or("-", |c| c.trim_start_matches('{').get(..8).unwrap_or(c)),
            e.form_factor.map_or("-".into(), |f| f.to_string()),
            e.id
        );
    }
    println!("\n{} endpoints. DEF: C=console M=multimedia X=communications. FF = EndpointFormFactor.", v.len());
    Ok(())
}

fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n - 1).collect::<String>()) }
}

fn dump_props(eps: &Endpoints, query: &str) -> AnyResult<()> {
    let e = resolve(eps, query)?;
    println!("{} ({})", e.friendly_name.as_deref().unwrap_or("?"), e.id);
    let store = ice_win::endpoints::open_store(&eps.device(&e.id)?)?;
    for (key, value) in props::dump(&store) {
        println!("{:<46} {}", props::key_to_string(&key), value.as_deref().unwrap_or("<non-string>"));
    }
    Ok(())
}

fn probe_stable(eps: &Endpoints) -> AnyResult<()> {
    for e in eps.list(false)? {
        let hits = eps.probe_stable_id(&eps.device(&e.id)?);
        println!("{}", e.friendly_name.as_deref().unwrap_or("?"));
        if hits.is_empty() {
            println!("    (no other string property resolves back to this endpoint)");
        }
        for (key, value) in hits {
            println!("    {}  =  {}", props::key_to_string(&key), value);
        }
    }
    Ok(())
}

fn switch(eps: &Endpoints, query: &str, roles: &str) -> AnyResult<()> {
    let target = resolve(eps, query)?;
    if target.state != DeviceState::Active {
        return Err(format!("refusing to switch to a {:?} endpoint", target.state).into());
    }
    let roles: &[Role] = match roles {
        "default" => &Role::DEFAULT,
        "comms" => &[Role::Communications],
        _ => &Role::ALL,
    };
    let started = Instant::now();
    let results = policy::set_default(&target.id, roles)?;
    let elapsed = started.elapsed();
    let after = eps.defaults();
    println!("target: {} ({})", target.friendly_name.as_deref().unwrap_or("?"), target.id);
    for (role, res) in results {
        let now = after.iter().find(|(f, r, _)| *f == target.flow && *r == role).map(|(_, _, id)| id.as_str());
        println!(
            "  {role:?}: call {} | default is now target: {}",
            res.map_or_else(|e| format!("FAILED ({e})"), |()| "ok".into()),
            now == Some(target.id.as_str())
        );
    }
    println!("  SetDefaultEndpoint calls took {elapsed:?}");
    Ok(())
}

fn peak(eps: &Endpoints, secs: u64) -> AnyResult<()> {
    let dev = eps.default_device(Flow::Render, Role::Multimedia)?;
    let m = meter::Meter::new(&dev)?;
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        let p = m.peak()?;
        let bars = (p * 40.0).round() as usize;
        print!("\r{:>5.3} {:<40}", p, "█".repeat(bars));
        std::io::stdout().flush()?;
        std::thread::sleep(Duration::from_millis(33));
    }
    println!();
    Ok(())
}

fn watch(eps: &Endpoints, secs: Option<u64>, log_path: &str) -> AnyResult<()> {
    if let Some(dir) = std::path::Path::new(log_path).parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut log = std::fs::OpenOptions::new().create(true).append(true).open(log_path)?;
    let snapshot = |log: &mut std::fs::File, reason: &str| -> AnyResult<()> {
        let line = serde_json::json!({ "ts": unix_ms(), "snapshot": reason, "endpoints": eps.list(true)? });
        writeln!(log, "{line}")?;
        Ok(())
    };
    snapshot(&mut log, "start")?;

    let (tx, rx) = mpsc::channel();
    let sink = |tx: mpsc::Sender<AudioEvent>| {
        move |e| {
            let _ = tx.send(e);
        }
    };
    let _reg = notify::register(eps.raw(), sink(tx.clone()))?;
    let mut vol_watch =
        volume::VolumeWatch::new(&eps.default_device(Flow::Render, Role::Multimedia)?, sink(tx.clone()))?;
    let start = Instant::now();
    println!("watching (endpoint + default-device volume events); log: {log_path}");
    println!("try: volume keys, unplug/replug the headset dongle, power-cycle the headset, switch default device…");

    loop {
        let timeout = match secs {
            Some(s) => match Duration::from_secs(s).checked_sub(start.elapsed()) {
                Some(left) => left,
                None => break,
            },
            None => Duration::from_secs(3600),
        };
        let Ok(ev) = rx.recv_timeout(timeout) else { continue };
        let t = start.elapsed().as_millis();
        let name = |id: &str| {
            eps.device(id).ok().and_then(|d| eps.info(&d).ok()).and_then(|i| i.friendly_name).unwrap_or_default()
        };
        match &ev {
            AudioEvent::Volume { scalar, muted } => {
                println!("{t:>8} ms  volume {:>3.0}%{}", scalar * 100.0, if *muted { " (muted)" } else { "" })
            }
            AudioEvent::DefaultChanged { flow, role, id } => {
                println!(
                    "{t:>8} ms  default {flow:?}/{role:?} -> {}",
                    id.as_deref().map(name).unwrap_or_else(|| "<none>".into())
                );
                // Follow the new default device's volume (the notch shows the *current* device).
                if *flow == Flow::Render
                    && *role == Role::Multimedia
                    && let Ok(dev) = eps.default_device(Flow::Render, Role::Multimedia)
                    && let Ok(w) = volume::VolumeWatch::new(&dev, sink(tx.clone()))
                {
                    vol_watch = w;
                }
            }
            AudioEvent::PropertyChanged { .. } => {} // very chatty; logged to the file only
            AudioEvent::DeviceAdded { id } | AudioEvent::DeviceRemoved { id } | AudioEvent::StateChanged { id, .. } => {
                println!("{t:>8} ms  {ev:?}  [{}]", name(id));
            }
        }
        writeln!(log, "{}", serde_json::json!({ "ts": unix_ms(), "event": ev }))?;
        if matches!(
            ev,
            AudioEvent::DeviceAdded { .. } | AudioEvent::DeviceRemoved { .. } | AudioEvent::StateChanged { .. }
        ) {
            snapshot(&mut log, "after-device-change")?;
        }
    }
    drop(vol_watch);
    Ok(())
}

fn print_spatial(e: &EndpointInfo) {
    let name = e.friendly_name.as_deref().unwrap_or("?");
    match spatial::status(&e.id) {
        Ok(s) => {
            let fmt = |v: &Option<(Option<SpatialFormat>, String)>| match v {
                None => "Off".to_string(),
                Some((Some(f), _)) => f.label().to_string(),
                Some((None, raw)) => format!("unknown provider {raw}"),
            };
            println!("{name}");
            println!("    supported: {} | active: {} | selected: {}", s.supported, fmt(&s.active), fmt(&s.default));
            println!("    available: {}", s.available.iter().map(|f| f.label()).collect::<Vec<_>>().join(", "));
        }
        Err(err) => println!("{name}\n    error: {err}"),
    }
}

fn spatial_status(eps: &Endpoints, query: Option<&str>) -> AnyResult<()> {
    match query {
        Some(q) => print_spatial(&resolve(eps, q)?),
        None => eps.list(false)?.iter().filter(|e| e.flow == Flow::Render).for_each(print_spatial),
    }
    Ok(())
}

fn spatial_set(eps: &Endpoints, query: &str, format: &str) -> AnyResult<()> {
    let e = resolve(eps, query)?;
    let wanted = if format.eq_ignore_ascii_case("off") {
        None
    } else {
        Some(SpatialFormat::parse(format).ok_or_else(|| format!("unknown format `{format}`"))?)
    };
    let before = spatial::status(&e.id)?;
    let current = before.default.as_ref().and_then(|(f, _)| *f);
    if before.default.is_none() == wanted.is_none() && current == wanted {
        println!("already set; skipping the write (read-before-write rule, D-17)");
        return Ok(());
    }
    let started = Instant::now();
    let status = spatial::set_default(&e.id, wanted)?;
    println!("SetDefaultSpatialAudioFormatAsync -> {status} in {:?}", started.elapsed());
    print_spatial(&e);
    Ok(())
}
