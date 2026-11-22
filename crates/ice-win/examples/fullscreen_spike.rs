//! Read-only Phase 0.5 diagnostic. Build and run from PowerShell.

#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

use ice_win::fullscreen::{Snapshot, snapshot, watch_for};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => match snapshot()? {
            Some(state) => print_snapshot(&state),
            None => println!("foreground sample inconclusive"),
        },
        [mode, seconds] if mode == "watch" => {
            let start = Instant::now();
            let mut last = None;
            watch_for(Duration::from_secs(seconds.parse()?), |current| {
                if last.as_ref() != Some(&current) {
                    println!("+{} ms", start.elapsed().as_millis());
                    print_snapshot(&current);
                    last = Some(current);
                }
            })?;
        }
        _ => return Err("usage: fullscreen_spike [watch <seconds>]".into()),
    }
    Ok(())
}

fn print_snapshot(state: &Snapshot) {
    println!("foreground frame: {:?}", state.foreground_frame);
    for display in &state.displays {
        let primary = if display.primary { " (primary)" } else { "" };
        println!("{}{primary} {:?}: fullscreen={}", display.name, display.bounds, display.covered,);
    }
    println!("shell D3D fullscreen hint: {:?}", state.shell_reports_d3d_fullscreen);
}
