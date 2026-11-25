// Auto-generated from Rust IPC types via `cargo test -p ice-audio-manager export_ts_bindings`.
// Do not edit manually.

import type { IdleStyle, NotchAnchor, NotchStyle, SpringPreset, ThemeKind } from './engine';
export type { IdleStyle, NotchAnchor, NotchStyle, SpringPreset, ThemeKind } from './engine';

export type ActivityKind = "idle" | "volume" | "switch" | "mic_mute" | "track";

export type MonitorSummary = { index: number, id: string,
/**
 * Persistent display path; `Command::SetPinnedMonitor` takes this, never `id`.
 */
displayPath: string,
/**
 * Monitor's own name (EDID), e.g. "LG FULL HD".
 */
name: string, isPrimary: boolean,
/**
 * Desktop position in physical pixels, for drawing the monitor layout.
 */
left: number, top: number, width: number, height: number, scaleFactor: number, refreshHz: number, };

export type HitRect = { x: number, y: number, width: number, height: number, };

export type NotchSnapshot = { revision: number, eventSeq: number, placedSeq: number, activity: ActivityKind, volumePercent: number, muted: boolean, micMuted: boolean, activeDevice: string, targetDevice: string, spatialBadge: string | null, preset: SpringPreset, systemReduceMotion: boolean, theme: ThemeKind, idleStyle: IdleStyle, notchStyle: NotchStyle,
/**
 * D-20: the anchor in use (Edge Nook maps a bottom anchor to the top); a bottom one makes the notch grow upwards.
 */
anchor: NotchAnchor, expanded: boolean, suspended: boolean, hiddenForBench: boolean, activeMonitor: number, monitors: Array<MonitorSummary>, };

export type NotchDelta = { "kind": "state", snapshot: NotchSnapshot, };
