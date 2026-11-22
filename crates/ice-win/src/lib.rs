//! All Windows FFI for Ice Audio Manager.
//!
//! This is the **only** crate allowed to contain `unsafe` (code-practices §3). Every `unsafe`
//! block carries a `SAFETY:` comment, and only safe APIs are exported.
//!
//! COM threading: callers create one [`com::ComGuard`] (MTA) on the thread that owns these objects.
//! COM callbacks never block. They only push [`ice_core::event::AudioEvent`]s into a channel.

pub mod com;
pub mod endpoints;
pub mod fullscreen;
pub mod media;
pub mod meter;
pub mod notify;
pub mod policy;
pub mod props;
pub mod spatial;
pub mod volume;

pub use windows::core::{Error, Result};
