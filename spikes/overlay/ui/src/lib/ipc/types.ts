// Auto-generated from Rust IPC types via `cargo test -p overlay-spike export_ts_bindings`.
// Do not edit manually.

export type SpringPreset = "snappy" | "smooth" | "bouncy" | "off";

export type ThemeKind = "ice" | "midnight" | "fluent" | "minimal";

export type ActivityKind = "idle" | "volume" | "switch" | "mic_mute";

export type MonitorSummary = { index: number, id: string, isPrimary: boolean, width: number, height: number, scaleFactor: number, refreshHz: number, };

export type HitRect = { x: number, y: number, width: number, height: number, };

export type NotchSnapshot = { revision: bigint, eventSeq: bigint, activity: ActivityKind, volumePercent: number, muted: boolean, micMuted: boolean, activeDevice: string, targetDevice: string, spatialBadge: string | null, preset: SpringPreset, systemReduceMotion: boolean, theme: ThemeKind, expanded: boolean, hiddenForBench: boolean, activeMonitor: number, monitors: Array<MonitorSummary>, };

export type NotchDelta = { "kind": "state", snapshot: NotchSnapshot, };
