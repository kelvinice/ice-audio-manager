//! D-30: per-app audio sessions on every active playback device (`IAudioSessionManager2`): which process owns each
//! one, whether it is playing, its peak level and its volume. Documented user-mode Core Audio only; no hooks, no
//! injection (§1). New sessions and state changes arrive as COM callbacks that only call the caller's sink (no
//! blocking, no calls back into Core Audio); the owner then re-scans on its own thread.

use std::collections::HashMap;
use std::sync::Arc;

use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::{
    AudioSessionDisconnectReason, AudioSessionState, AudioSessionStateActive, IAudioSessionControl,
    IAudioSessionControl2, IAudioSessionEvents, IAudioSessionEvents_Impl, IAudioSessionManager2,
    IAudioSessionNotification, IAudioSessionNotification_Impl, ISimpleAudioVolume,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoTaskMemFree};
use windows::core::{BOOL, GUID, Interface, PCWSTR, Ref, Result, implement};

use crate::endpoints::Endpoints;
use crate::proc_metrics::process_exe_name;

/// Runs on Core Audio's threads: must only signal (send to a channel), never block.
pub type Sink = Arc<dyn Fn() + Send + Sync>;

/// Tags our own volume writes, so anything that listens can tell them from the user's.
const CALL_MODE_CONTEXT: GUID = GUID::from_u128(0x6a1f_3c2e_8d4b_4e7a_9c35_d30c_a11d_0c4e);

#[implement(IAudioSessionNotification)]
struct Created {
    sink: Sink,
}

impl IAudioSessionNotification_Impl for Created_Impl {
    fn OnSessionCreated(&self, _new: Ref<IAudioSessionControl>) -> Result<()> {
        (self.sink)();
        Ok(())
    }
}

#[implement(IAudioSessionEvents)]
struct StateEvents {
    sink: Sink,
}

impl IAudioSessionEvents_Impl for StateEvents_Impl {
    fn OnDisplayNameChanged(&self, _name: &PCWSTR, _ctx: *const GUID) -> Result<()> {
        Ok(())
    }
    fn OnIconPathChanged(&self, _path: &PCWSTR, _ctx: *const GUID) -> Result<()> {
        Ok(())
    }
    fn OnSimpleVolumeChanged(&self, _volume: f32, _mute: BOOL, _ctx: *const GUID) -> Result<()> {
        Ok(())
    }
    fn OnChannelVolumeChanged(
        &self,
        _count: u32,
        _volumes: *const f32,
        _changed: u32,
        _ctx: *const GUID,
    ) -> Result<()> {
        Ok(())
    }
    fn OnGroupingParamChanged(&self, _param: *const GUID, _ctx: *const GUID) -> Result<()> {
        Ok(())
    }
    fn OnStateChanged(&self, _state: AudioSessionState) -> Result<()> {
        (self.sink)();
        Ok(())
    }
    fn OnSessionDisconnected(&self, _reason: AudioSessionDisconnectReason) -> Result<()> {
        (self.sink)();
        Ok(())
    }
}

/// One app's audio session on one playback device.
pub struct Session {
    /// Windows' session instance identifier: unique per session, stable while it lives.
    pub key: String,
    /// MMDevice endpoint ID this session belongs to.
    pub endpoint_id: String,
    /// Executable file name of the owning process, e.g. `Discord.exe`.
    pub exe: String,
    control: IAudioSessionControl2,
}

impl Session {
    /// The app is playing through this session right now.
    #[must_use]
    pub fn is_active(&self) -> bool {
        // SAFETY: plain getter on a valid session control.
        unsafe { self.control.GetState() }.is_ok_and(|s| s == AudioSessionStateActive)
    }

    /// The session's current peak (0.0–1.0); 0 if it can't be read. Levels only, never audio samples (§1.9).
    #[must_use]
    pub fn peak(&self) -> f32 {
        let Ok(meter) = self.control.cast::<IAudioMeterInformation>() else { return 0.0 };
        // SAFETY: plain getter on the session's own meter interface.
        unsafe { meter.GetPeakValue() }.unwrap_or(0.0)
    }

    /// The app's volume slider in the Windows Volume Mixer (0.0–1.0).
    #[must_use]
    pub fn volume(&self) -> Option<f32> {
        let volume = self.control.cast::<ISimpleAudioVolume>().ok()?;
        // SAFETY: plain getter on the session's own volume interface.
        unsafe { volume.GetMasterVolume() }.ok()
    }

    /// Sets the app's volume slider; `false` if the session is gone.
    pub fn set_volume(&self, level: f32) -> bool {
        let Ok(volume) = self.control.cast::<ISimpleAudioVolume>() else { return false };
        let context = CALL_MODE_CONTEXT;
        // SAFETY: documented setter; `context` lives on the stack for the whole synchronous call.
        unsafe { volume.SetMasterVolume(level.clamp(0.0, 1.0), &raw const context) }.is_ok()
    }
}

struct EndpointManager {
    id: String,
    manager: IAudioSessionManager2,
    created: IAudioSessionNotification,
    meter: Option<crate::meter::Meter>,
}

/// The session managers of every active playback device with our "session created" notification registered, plus
/// state-change notifications on the sessions passed to [`SessionWatch::watch`]. Unregisters everything on drop.
pub struct SessionWatch {
    managers: Vec<EndpointManager>,
    watched: Vec<(String, IAudioSessionControl, IAudioSessionEvents)>,
    exe_names: HashMap<u32, Option<String>>,
    sink: Sink,
}

impl SessionWatch {
    /// Registers on every active playback device. Devices that refuse are skipped (logged at debug).
    pub fn new(eps: &Endpoints, sink: Sink) -> Self {
        let mut managers = Vec::new();
        for info in eps.list(false).unwrap_or_default() {
            if info.flow != ice_core::device::Flow::Render {
                continue;
            }
            match register_manager(eps, &info.id, &sink) {
                Ok(entry) => managers.push(entry),
                Err(e) => tracing::debug!(error = %e, "call mode: no session manager for a playback device"),
            }
        }
        Self { managers, watched: Vec::new(), exe_names: HashMap::new(), sink }
    }

    /// Current master peak (0.0–1.0) of the render endpoint `endpoint_id`, or `0.0` if unavailable.
    #[must_use]
    pub fn endpoint_peak(&self, endpoint_id: &str) -> f32 {
        self.managers
            .iter()
            .find(|ep| ep.id == endpoint_id)
            .and_then(|ep| ep.meter.as_ref())
            .and_then(|m| m.peak().ok())
            .unwrap_or(0.0)
    }

    /// Every app session on every watched device (system sounds and our own process excluded).
    pub fn sessions(&mut self) -> Vec<Session> {
        let own = std::process::id();
        let mut out = Vec::new();
        let mut live_pids = Vec::new();
        for ep in &self.managers {
            // SAFETY: plain enumeration on a valid manager; the enumerator is a snapshot owned by us.
            let Ok(list) = (unsafe { ep.manager.GetSessionEnumerator() }) else { continue };
            // SAFETY: plain getter on a valid enumerator.
            let count = unsafe { list.GetCount() }.unwrap_or(0);
            for i in 0..count {
                // SAFETY: `i` is within the enumerator's count.
                let Ok(control) = (unsafe { list.GetSession(i) }) else { continue };
                let Ok(control) = control.cast::<IAudioSessionControl2>() else { continue };
                // SAFETY: plain getters on a valid session control.
                let (system, pid) = unsafe { (control.IsSystemSoundsSession(), control.GetProcessId()) };
                let Ok(pid) = pid else { continue };
                // `IsSystemSoundsSession` returns S_OK (0) for the system sounds session, S_FALSE (1) otherwise.
                if system.0 == 0 || pid == 0 || pid == own {
                    continue;
                }
                live_pids.push(pid);
                let Some(exe) = self.exe_names.entry(pid).or_insert_with(|| process_exe_name(pid)).clone() else {
                    continue;
                };
                if let Some(key) = instance_id(&control) {
                    out.push(Session { key, endpoint_id: ep.id.clone(), exe, control });
                }
            }
        }
        self.exe_names.retain(|pid, _| live_pids.contains(pid));
        out
    }

    /// Notifies the sink when this session starts or stops playing, or goes away. Idempotent per session.
    pub fn watch(&mut self, session: &Session) {
        if self.watched.iter().any(|(key, ..)| *key == session.key) {
            return;
        }
        let Ok(control) = session.control.cast::<IAudioSessionControl>() else { return };
        let events: IAudioSessionEvents = StateEvents { sink: Arc::clone(&self.sink) }.into();
        // SAFETY: `events` is kept alive in `watched` until it is unregistered in `retain_watched` or `Drop`.
        if unsafe { control.RegisterAudioSessionNotification(&events) }.is_ok() {
            self.watched.push((session.key.clone(), control, events));
        }
    }

    /// Stops watching sessions whose key isn't in `keep` (they ended, or their app left the call list).
    pub fn retain_watched(&mut self, keep: &[&str]) {
        self.watched.retain(|(key, control, events)| {
            let kept = keep.contains(&key.as_str());
            if !kept {
                // SAFETY: unregisters the same events object registered in `watch`.
                let _ = unsafe { control.UnregisterAudioSessionNotification(events) };
            }
            kept
        });
    }
}

impl Drop for SessionWatch {
    fn drop(&mut self) {
        self.retain_watched(&[]);
        for ep in &self.managers {
            // SAFETY: unregisters the same notification registered in `register_manager`.
            let _ = unsafe { ep.manager.UnregisterSessionNotification(&ep.created) };
        }
    }
}

fn register_manager(eps: &Endpoints, device_id: &str, sink: &Sink) -> Result<EndpointManager> {
    let device = eps.device(device_id)?;
    let meter = crate::meter::Meter::new(&device).ok();
    // SAFETY: documented activation of the session manager on a valid endpoint.
    let manager: IAudioSessionManager2 = unsafe { device.Activate(CLSCTX_ALL, None)? };
    // Windows only starts sending "session created" after the first enumeration (documented on
    // `RegisterSessionNotification`).
    // SAFETY: plain enumeration on a valid manager.
    let _ = unsafe { manager.GetSessionEnumerator()? };
    let created: IAudioSessionNotification = Created { sink: Arc::clone(sink) }.into();
    // SAFETY: `created` is kept alive with the manager until `SessionWatch::drop` unregisters it.
    unsafe { manager.RegisterSessionNotification(&created)? };
    Ok(EndpointManager { id: device_id.to_string(), manager, created, meter })
}

fn instance_id(control: &IAudioSessionControl2) -> Option<String> {
    // SAFETY: returns a CoTaskMemAlloc'ed string that we copy and then free exactly once.
    unsafe {
        let raw = control.GetSessionInstanceIdentifier().ok()?;
        if raw.is_null() {
            return None;
        }
        let id = raw.to_string().ok();
        CoTaskMemFree(Some(raw.0.cast_const().cast()));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::com::ComGuard;

    /// Real host, read-only: lists every app session with its owner, state, peak and volume. Changes nothing.
    /// `cargo test -p ice-win real_sessions -- --ignored --nocapture`
    #[test]
    #[ignore = "needs the real audio service"]
    fn real_sessions_list() {
        use ice_core::device::Flow;
        let _com = ComGuard::init_mta().unwrap();
        let eps = Endpoints::new().unwrap();
        for info in eps.list(false).unwrap_or_default() {
            if info.flow != Flow::Render {
                continue;
            }
            let Ok(dev) = eps.device(&info.id) else { continue };
            let ep_vol = crate::volume::get(&dev).ok();
            let ep_peak = crate::meter::Meter::new(&dev).and_then(|m| m.peak()).unwrap_or(-1.0);
            // SAFETY: documented activation of IAudioSessionManager2 on a valid endpoint.
            let Ok(mgr) = (unsafe { dev.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) }) else {
                continue;
            };
            // SAFETY: plain enumeration on a valid manager.
            let Ok(list) = (unsafe { mgr.GetSessionEnumerator() }) else { continue };
            // SAFETY: plain getter on a valid enumerator.
            let count = unsafe { list.GetCount() }.unwrap_or(0);
            if count == 0 {
                continue;
            }
            println!(
                "=== DEVICE {} ep_vol={ep_vol:?} ep_peak={ep_peak:.4} sessions={count} ===",
                info.friendly_name.as_deref().unwrap_or(&info.id)
            );
            for i in 0..count {
                // SAFETY: `i` is within `count`.
                let Ok(ctl) = (unsafe { list.GetSession(i) }) else { continue };
                let Ok(ctl2) = ctl.cast::<IAudioSessionControl2>() else { continue };
                // SAFETY: plain getters on a valid session control.
                let pid = unsafe { ctl2.GetProcessId() }.unwrap_or(0);
                // SAFETY: plain getter on a valid session control.
                let sys = unsafe { ctl2.IsSystemSoundsSession() }.0 == 0;
                let exe =
                    if sys { "<system>".into() } else { process_exe_name(pid).unwrap_or_else(|| format!("pid:{pid}")) };
                // SAFETY: `GetSessionIdentifier` returns a CoTaskMemAlloc'ed string freed with CoTaskMemFree.
                let sid = unsafe {
                    ctl2.GetSessionIdentifier()
                        .ok()
                        .map(|p| {
                            let s = p.to_string().unwrap_or_default();
                            CoTaskMemFree(Some(p.0.cast_const().cast()));
                            s
                        })
                        .unwrap_or_default()
                };
                // SAFETY: plain getter on a valid session control.
                let active = unsafe { ctl2.GetState() }.is_ok_and(|s| s == AudioSessionStateActive);
                // SAFETY: plain getter on a valid session meter interface.
                let peak =
                    ctl2.cast::<IAudioMeterInformation>().and_then(|m| unsafe { m.GetPeakValue() }).unwrap_or(-1.0);
                // SAFETY: plain getter on a valid session volume interface.
                let vol = ctl2.cast::<ISimpleAudioVolume>().and_then(|v| unsafe { v.GetMasterVolume() }).ok();
                println!("  [{i}] {exe:<24} pid={pid:<6} active={active:<5} peak={peak:.4} vol={vol:?} sid={sid}");
            }
        }
        let mut watch = SessionWatch::new(&eps, Arc::new(|| {}));
        let sessions = watch.sessions();
        for s in &sessions {
            watch.watch(s);
        }
        assert_eq!(watch.watched.len(), sessions.len(), "every session accepts a state notification");
        let default_dev = eps.default_device(Flow::Render, ice_core::device::Role::Multimedia).ok();
        let meter = default_dev.as_ref().and_then(|d| crate::meter::Meter::new(d).ok());
        for step in 0..10 {
            let ep_p = meter.as_ref().and_then(|m| m.peak().ok()).unwrap_or(-1.0);
            let active_peaks: Vec<String> =
                sessions.iter().filter(|s| s.is_active()).map(|s| format!("{}={:.4}", s.exe, s.peak())).collect();
            println!("sample[{step}]: ep_peak={ep_p:.4} sessions=[{}]", active_peaks.join(", "));
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
    }

    /// Real host, read-only: while the app runs, logs Discord's peak and every music-session volume change it sees
    /// for 15 s (what the running call mode writes). `cargo test -p ice-win real_watch_dip -- --ignored --nocapture`
    #[test]
    #[ignore = "needs the real audio service"]
    fn real_watch_dip() {
        use ice_core::call_mode::{CALL_TICK, CallModeConfig};
        let _com = ComGuard::init_mta().unwrap();
        let eps = Endpoints::new().unwrap();
        let config = CallModeConfig::default();
        let mut watch = SessionWatch::new(&eps, Arc::new(|| {}));
        let start = std::time::Instant::now();
        let mut last: Vec<(String, Option<f32>)> = Vec::new();
        while start.elapsed() < std::time::Duration::from_secs(15) {
            let sessions = watch.sessions();
            let peak = sessions.iter().filter(|s| config.is_call_app(&s.exe) && s.is_active()).map(Session::peak);
            let peak = peak.fold(0.0_f32, f32::max);
            let now: Vec<(String, Option<f32>)> =
                sessions.iter().filter(|s| !config.is_call_app(&s.exe)).map(|s| (s.exe.clone(), s.volume())).collect();
            if now != last {
                println!("{:>6} ms call={peak:.3} {now:?}", start.elapsed().as_millis());
                last = now;
            }
            std::thread::sleep(CALL_TICK);
        }
    }

    /// Real host, read-only: counts session notifications (what wakes `ice-call`) over 15 s with Discord watched, and
    /// prints Discord's peak once a second. `cargo test -p ice-win real_notify_rate -- --ignored --nocapture`
    #[test]
    #[ignore = "needs the real audio service"]
    fn real_notify_rate() {
        use ice_core::call_mode::CallModeConfig;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let _com = ComGuard::init_mta().unwrap();
        let eps = Endpoints::new().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&count);
        let mut watch = SessionWatch::new(
            &eps,
            Arc::new(move || {
                counter.fetch_add(1, Ordering::Relaxed);
            }),
        );
        let config = CallModeConfig::default();
        let calls: Vec<Session> = watch.sessions().into_iter().filter(|s| config.is_call_app(&s.exe)).collect();
        for s in &calls {
            watch.watch(s);
        }
        for second in 1..=15 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let peak = calls.iter().filter(|s| s.is_active()).map(Session::peak).fold(0.0_f32, f32::max);
            let active = calls.iter().filter(|s| s.is_active()).count();
            println!(
                "{second:>2}s notifications={} active_calls={active} peak={peak:.3}",
                count.load(Ordering::Relaxed)
            );
        }
    }

    /// Real host, WRITES Spotify's mixer level for ~4 s, then puts it back: does Spotify re-write its own session
    /// volume after someone else sets it? `cargo test -p ice-win real_spotify_echo -- --ignored --nocapture`
    #[test]
    #[ignore = "changes Spotify's volume briefly"]
    fn real_spotify_echo() {
        let _com = ComGuard::init_mta().unwrap();
        let eps = Endpoints::new().unwrap();
        let mut watch = SessionWatch::new(&eps, Arc::new(|| {}));
        let spotify: Vec<Session> =
            watch.sessions().into_iter().filter(|s| s.exe.eq_ignore_ascii_case("Spotify.exe")).collect();
        println!("Spotify sessions: {}", spotify.len());
        let original: Vec<Option<f32>> = spotify.iter().map(Session::volume).collect();
        println!("original: {original:?}");
        for target in [0.9_f32, 0.533, 0.066, 0.0, 0.25] {
            for s in &spotify {
                s.set_volume(target);
            }
            let mut seen = Vec::new();
            for _ in 0..18 {
                std::thread::sleep(std::time::Duration::from_millis(33));
                let v: Vec<Option<f32>> = spotify.iter().map(Session::volume).collect();
                if seen.last() != Some(&v) {
                    seen.push(v);
                }
            }
            println!("set {target:.3} -> read {seen:?}");
        }
        for (s, v) in spotify.iter().zip(&original) {
            if let Some(v) = v {
                s.set_volume(*v);
            }
        }
        println!("restored: {:?}", spotify.iter().map(Session::volume).collect::<Vec<_>>());
    }

    /// Real host, read-only: samples the active call sessions' peaks for 10 s at the call-mode tick and replays them
    /// through the `Ducker` (no volume is changed). `cargo test -p ice-win real_call_peaks -- --ignored --nocapture`
    #[test]
    #[ignore = "needs the real audio service"]
    fn real_call_peaks() {
        use ice_core::call_mode::{CALL_TICK, CallModeConfig, Ducker};
        const TICKS: usize = 150;
        let _com = ComGuard::init_mta().unwrap();
        let eps = Endpoints::new().unwrap();
        let config = CallModeConfig::default();
        let mut watch = SessionWatch::new(&eps, Arc::new(|| {}));
        let calls: Vec<Session> = watch.sessions().into_iter().filter(|s| config.is_call_app(&s.exe)).collect();
        let mut ducker = Ducker::new(&config);
        let (mut peaks, mut lowered) = (Vec::new(), 0);
        for _ in 0..TICKS {
            let peak = calls.iter().filter(|s| s.is_active()).map(Session::peak).fold(0.0_f32, f32::max);
            peaks.push(peak);
            if ducker.step(peak, CALL_TICK) < 1.0 {
                lowered += 1;
            }
            std::thread::sleep(CALL_TICK);
        }
        peaks.sort_by(f32::total_cmp);
        let at = |q: usize| peaks[(TICKS - 1) * q / 100];
        println!("peaks min={:.4} median={:.4} p90={:.4} max={:.4}", at(0), at(50), at(90), at(100));
        println!("lowered on {lowered}/{TICKS} ticks, ending at gain {:.2}", ducker.gain());
    }
}
