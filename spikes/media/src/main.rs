//! Phase 0.4 / Phase 3 media spike. Throwaway CLI; the reusable code lives in `ice_win::media`.
//!
//! ```text
//! media-spike list                 every GSMTC session + which one Windows calls current
//! media-spike art                  current album art: decode/downscale/encode time, size, accent colour
//! media-spike watch [seconds]      event-driven: prints track/status/timeline changes and how fast they were read
//! media-spike toggle               play/pause the current session (only run when you mean it)
//! ```

use std::sync::mpsc;
use std::time::{Duration, Instant};

use ice_core::engine::MediaAction;
use ice_core::media::NowPlaying;
use ice_win::com::ComGuard;
use ice_win::media::{self, MediaSignal, MediaWatcher};

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn line(n: &NowPlaying) -> String {
    let time = match (n.position_ms, n.duration_ms) {
        (Some(p), Some(d)) => format!(" {}:{:02}/{}:{:02}", p / 60_000, p / 1000 % 60, d / 60_000, d / 1000 % 60),
        _ => String::new(),
    };
    format!("[{:?}] {} — {} ({}){time} | {}", n.status, n.artist, n.title, n.album, n.app_id)
}

fn run() -> AnyResult<()> {
    let _com = ComGuard::init_mta()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let started = Instant::now();
    let mgr = media::manager()?;
    println!("session manager ready in {:?}", started.elapsed());
    match args.first().map_or("list", String::as_str) {
        "list" => {
            let t = Instant::now();
            let all = media::sessions(&mgr)?;
            println!("{} session(s), read in {:?}", all.len(), t.elapsed());
            for n in &all {
                println!("  {}", line(n));
            }
            match media::now_playing(&mgr)? {
                Some(n) => println!("current: {}", line(&n)),
                None => println!("current: <none>"),
            }
        }
        "art" => {
            let t = Instant::now();
            match media::current_art(&mgr, 128)? {
                Some(a) => println!(
                    "art {}x{} id {} accent {} | PNG {} bytes | decode+scale+accent+encode {:?}",
                    a.info.width,
                    a.info.height,
                    a.info.id,
                    a.info.accent.hex(),
                    a.png.len(),
                    t.elapsed()
                ),
                None => println!("no art"),
            }
        }
        "watch" => {
            let secs: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(60);
            let (tx, rx) = mpsc::channel();
            let mut watcher = MediaWatcher::start(mgr, move |s| {
                let _ = tx.send((s, Instant::now()));
            })?;
            println!("watching for {secs} s: change tracks, pause/play, seek in Spotify…");
            let start = Instant::now();
            let mut last_art = None;
            while let Some(left) = Duration::from_secs(secs).checked_sub(start.elapsed()) {
                let Ok((signal, at)) = rx.recv_timeout(left) else { continue };
                if matches!(signal, MediaSignal::CurrentSessionChanged | MediaSignal::SessionsChanged)
                    && !watcher.follow_current()
                {
                    continue;
                }
                let now = media::now_playing(watcher.manager()).ok().flatten();
                let read = at.elapsed();
                let mut extra = String::new();
                if matches!(signal, MediaSignal::PropertiesChanged | MediaSignal::CurrentSessionChanged) {
                    let t = Instant::now();
                    if let Ok(Some(a)) = media::current_art(watcher.manager(), 128)
                        && last_art.as_ref() != Some(&a.info.id)
                    {
                        extra = format!(" | new art {} accent {} ({:?})", a.info.id, a.info.accent.hex(), t.elapsed());
                        last_art = Some(a.info.id);
                    }
                }
                println!(
                    "{:>8} ms  {signal:?} (read {read:?}): {}{extra}",
                    start.elapsed().as_millis(),
                    now.as_ref().map_or("<none>".into(), line)
                );
            }
        }
        "toggle" => println!("toggle accepted: {}", media::send(&mgr, MediaAction::PlayPause)?),
        other => return Err(format!("unknown command `{other}`").into()),
    }
    Ok(())
}
