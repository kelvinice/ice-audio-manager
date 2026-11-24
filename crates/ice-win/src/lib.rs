//! All Windows FFI for Ice Audio Manager.
//!
//! This is the **only** crate allowed to contain `unsafe` (code-practices §3). Every `unsafe`
//! block carries a `SAFETY:` comment, and only safe APIs are exported.
//!
//! COM threading: callers create one [`com::ComGuard`] (MTA) on the thread that owns these objects.
//! COM callbacks never block. They only push [`ice_core::event::AudioEvent`]s into a channel.

pub mod audio_engine;
pub mod autostart;
pub mod com;
pub mod desktop;
pub mod endpoints;
pub mod file_watch;
pub mod fullscreen;
pub mod hotkeys;
pub mod media;
pub mod media_service;
pub mod meter;
pub mod monitors;
pub mod music_service;
pub mod native_dot;
pub mod notify;
pub mod overlay_win;
pub mod policy;
pub mod proc_metrics;
pub mod props;
pub mod session_watch;
pub mod spatial;
pub mod volume;
#[cfg(feature = "webview2")]
pub mod webview2;

pub use windows::core::{Error, Result};
