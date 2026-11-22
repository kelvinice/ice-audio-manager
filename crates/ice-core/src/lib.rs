//! Pure domain types and logic for Ice Audio Manager.
//!
//! This crate must never depend on Windows APIs or Tauri, so it stays unit-testable
//! and reusable by a future native notch or CLI (see `docs/planning-discussion.md`, D-03).
#![forbid(unsafe_code)]

pub mod device;
pub mod engine;
pub mod event;
pub mod hotkey;
pub mod resolve;
