//! Phase 0.4 media spike. Throwaway CLI; the reusable code lives in `ice_win::media`.
//!
//! ```text
//! media-spike list                 every GSMTC session + which one Windows calls current
//! media-spike watch [seconds]      print track/status changes with timestamps (polls at 4 Hz; Phase 3 uses events)
//! media-spike toggle               play/pause the current session (only run when you mean it)
//! ```

use std::time::{Duration, Instant};

use ice_win::com::ComGuard;
use ice_win::media::{self, NowPlaying};

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn line(n: &NowPlaying) -> String {
    format!(
        "[{}] {} — {} ({}) | app: {} | art: {}",
        n.status,
        n.artist,
        n.title,
        n.album,
        n.app,
        n.thumbnail_bytes.map_or("none".into(), |b| format!("{b} bytes"))
    )
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
            match media::current(&mgr)? {
                Some(n) => println!("current: {}", line(&n)),
                None => println!("current: <none>"),
            }
        }
        "watch" => {
            let secs: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(30);
            let mut last: Option<NowPlaying> = None;
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(secs) {
                let now = media::current(&mgr).ok().flatten();
                if now != last {
                    println!("{:>8} ms  {}", start.elapsed().as_millis(), now.as_ref().map_or("<none>".into(), line));
                    last = now;
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        }
        "toggle" => println!("toggle accepted: {}", media::toggle_play_pause(&mgr)?),
        other => return Err(format!("unknown command `{other}`").into()),
    }
    Ok(())
}
