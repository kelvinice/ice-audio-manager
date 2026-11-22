//! Events produced by the Windows layer and consumed by the app core.
//!
//! COM callbacks must not block: they only convert data into these values and push them
//! into a channel (see `docs/code-practices.md` §4).

use serde::{Deserialize, Serialize};

use crate::device::{DeviceState, Flow, Role};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AudioEvent {
    DeviceAdded {
        id: String,
    },
    DeviceRemoved {
        id: String,
    },
    StateChanged {
        id: String,
        state: DeviceState,
    },
    DefaultChanged {
        flow: Flow,
        role: Role,
        id: Option<String>,
    },
    PropertyChanged {
        id: String,
        key: String,
    },
    /// Master volume of an endpoint changed (from any source: keys, mixer, other apps).
    Volume {
        scalar: f32,
        muted: bool,
    },
}
