import type {
  DeviceSummary,
  EngineEvent,
  EngineSnapshot,
  Envelope,
  Notice,
  RoleSet,
  SpatialMode,
  VolumeState,
} from '../ipc/engine.ts';

export const VOLUME_DURATION_MS = 1600;
export const SWITCH_DURATION_MS = 2200;
export const PLUG_DURATION_MS = 2000;
export const MIC_DURATION_MS = 1600;
export const PLUG_SWITCH_MERGE_WINDOW_MS = 1500;
export const IDLE_SUSPEND_LINGER_MS = 15_000;

export type ActivityView =
  | { kind: 'idle' }
  | {
      kind: 'volume';
      channel: 'playback' | 'mic';
      percent: number;
      muted: boolean;
      deviceName: string;
      formFactor: number | null;
      entrySeq: number;
      boundBump: -1 | 0 | 1;
      bumpSeq: number;
      expiresAtMs: number;
    }
  | {
      kind: 'switch';
      previousDevice: string | null;
      device: DeviceSummary;
      relinked: boolean;
      mergedPlug: boolean;
      spatialBadge: string | null;
      switchSeq: number;
      expiresAtMs: number;
    }
  | {
      kind: 'plug';
      direction: 'arrived' | 'left';
      device: DeviceSummary;
      plugSeq: number;
      expiresAtMs: number;
    }
  | {
      kind: 'mic';
      muted: boolean;
      deviceName: string;
      entranceSeq: number;
      expiresAtMs: number;
    };

export interface NotchSchedulerState {
  revision: number;
  playback: DeviceSummary[];
  recording: DeviceSummary[];
  volume: VolumeState | null;
  micMuted: boolean | null;
  micVolume: number | null;
  notices: Notice[];
  activeDevice: DeviceSummary | null;
  targetDevice: DeviceSummary | null;
  activeMic: DeviceSummary | null;
  activity: ActivityView;
  idleSinceMs: number | null;
  suspendDispatched: boolean;
  lastArrivedDevice: { id: string; atMs: number } | null;
}

export type EnvelopeResult = {
  status: 'applied' | 'ignored_stale' | 'needs_resnapshot';
  wokeActivity: boolean;
};

export function formatSpatialBadge(mode: SpatialMode | null): string | null {
  switch (mode) {
    case 'windowsSonic':
      return 'Sonic';
    case 'dolbyAtmosHeadphones':
    case 'dolbyAtmosHomeTheater':
    case 'dolbyAtmosSpeakers':
      return 'Atmos';
    case 'dtsHeadphoneX':
    case 'dtsXUltra':
    case 'dtsXHomeTheater':
      return 'DTS';
    case 'keep':
    case 'off':
    case null:
      return null;
  }
}

export function shortDeviceName(name: string): string {
  const trimmed = name.trim();
  if (!trimmed) return 'Audio';
  const paren = trimmed.indexOf('(');
  if (paren > 0) {
    const inner = trimmed.slice(paren + 1, trimmed.lastIndexOf(')')).trim();
    if (trimmed.startsWith('Headphones (') || trimmed.startsWith('Microphone (')) {
      return inner || trimmed;
    }
    return trimmed.slice(0, paren).trim();
  }
  return trimmed;
}

function findActivePlayback(list: DeviceSummary[]): DeviceSummary | null {
  return list.find((d) => d.isDefault) ?? list[0] ?? null;
}

function findNextTarget(list: DeviceSummary[]): DeviceSummary | null {
  return list.find((d) => !d.isDefault) ?? null;
}

function findActiveMic(list: DeviceSummary[]): DeviceSummary | null {
  return list.find((d) => d.isDefaultCommunications) ?? list.find((d) => d.isDefault) ?? list[0] ?? null;
}

function updateSwitchDefaults(list: DeviceSummary[], switched: DeviceSummary, roles: RoleSet): DeviceSummary[] {
  let found = false;
  const updated = list.map((item) => {
    const isTarget = item.id === switched.id;
    if (isTarget) {
      found = true;
    }
    const isDefault = roles === 'communications' ? item.isDefault : isTarget;
    const isDefaultCommunications = roles === 'default' ? item.isDefaultCommunications : isTarget;
    if (isTarget) {
      return { ...switched, isDefault, isDefaultCommunications };
    }
    return { ...item, isDefault, isDefaultCommunications };
  });
  if (found) {
    return updated;
  }
  return [
    ...updated,
    {
      ...switched,
      isDefault: roles === 'communications' ? switched.isDefault : true,
      isDefaultCommunications: roles === 'default' ? switched.isDefaultCommunications : true,
    },
  ];
}

function upsertArrivedDevice(list: DeviceSummary[], arrived: DeviceSummary): DeviceSummary[] {
  const prev = list.find((d) => d.id === arrived.id);
  const merged = prev
    ? {
        ...arrived,
        isDefault: arrived.isDefault || prev.isDefault,
        isDefaultCommunications: arrived.isDefaultCommunications || prev.isDefaultCommunications,
      }
    : arrived;
  return [...list.filter((d) => d.id !== arrived.id), merged];
}

export class NotchActivityScheduler {
  state: NotchSchedulerState;
  private seqCounter = 0;

  constructor(nowMs = 0) {
    this.state = {
      revision: 0,
      playback: [],
      recording: [],
      volume: null,
      micMuted: null,
      micVolume: null,
      notices: [],
      activeDevice: null,
      targetDevice: null,
      activeMic: null,
      activity: { kind: 'idle' },
      idleSinceMs: nowMs,
      suspendDispatched: false,
      lastArrivedDevice: null,
    };
  }

  /** Synchronizes authoritative state from an EngineSnapshot without inventing expired activities. */
  applySnapshot(snapshot: EngineSnapshot, nowMs: number): void {
    const activeDevice = findActivePlayback(snapshot.playback);
    const targetDevice = findNextTarget(snapshot.playback);
    const activeMic = findActiveMic(snapshot.recording);
    this.state = {
      ...this.state,
      revision: snapshot.revision,
      playback: snapshot.playback,
      recording: snapshot.recording,
      volume: snapshot.volume,
      micMuted: snapshot.micMuted,
      micVolume: snapshot.micVolume,
      notices: snapshot.notices,
      activeDevice,
      targetDevice,
      activeMic,
      idleSinceMs: this.state.activity.kind === 'idle' ? (this.state.idleSinceMs ?? nowMs) : null,
    };
  }

  /** Applies one monotonic delta Envelope per D-03 rules. */
  applyEnvelope(envelope: Envelope, nowMs: number): EnvelopeResult {
    if (envelope.revision <= this.state.revision) {
      return { status: 'ignored_stale', wokeActivity: false };
    }
    if (this.state.revision > 0 && envelope.revision > this.state.revision + 1) {
      return { status: 'needs_resnapshot', wokeActivity: false };
    }

    this.state.revision = envelope.revision;
    const ev: EngineEvent = envelope.event;

    switch (ev.type) {
      case 'volumeChanged': {
        this.state.volume = ev.volume;
        const pct = Math.max(0, Math.min(100, ev.volume.percent));
        const cur = this.state.activity;
        const sameChannel = cur.kind === 'volume' && cur.channel === 'playback';
        let boundBump: -1 | 0 | 1 = 0;
        let bumpSeq = sameChannel ? cur.bumpSeq : 0;
        if (sameChannel && cur.percent === 0 && pct === 0) {
          boundBump = -1;
          bumpSeq += 1;
        } else if (sameChannel && cur.percent === 100 && pct === 100) {
          boundBump = 1;
          bumpSeq += 1;
        }
        const entrySeq = sameChannel ? cur.entrySeq : ++this.seqCounter;
        this.state.activity = {
          kind: 'volume',
          channel: 'playback',
          percent: pct,
          muted: ev.volume.muted,
          deviceName: this.state.activeDevice?.name ?? 'Playback',
          formFactor: this.state.activeDevice?.formFactor ?? null,
          entrySeq,
          boundBump,
          bumpSeq,
          expiresAtMs: nowMs + VOLUME_DURATION_MS,
        };
        this.state.idleSinceMs = null;
        this.state.suspendDispatched = false;
        return { status: 'applied', wokeActivity: true };
      }

      case 'micVolumeChanged': {
        const pct = Math.max(0, Math.min(100, ev.percent));
        this.state.micVolume = pct;
        const cur = this.state.activity;
        const sameChannel = cur.kind === 'volume' && cur.channel === 'mic';
        let boundBump: -1 | 0 | 1 = 0;
        let bumpSeq = sameChannel ? cur.bumpSeq : 0;
        if (sameChannel && cur.percent === 0 && pct === 0) {
          boundBump = -1;
          bumpSeq += 1;
        } else if (sameChannel && cur.percent === 100 && pct === 100) {
          boundBump = 1;
          bumpSeq += 1;
        }
        const entrySeq = sameChannel ? cur.entrySeq : ++this.seqCounter;
        this.state.activity = {
          kind: 'volume',
          channel: 'mic',
          percent: pct,
          muted: this.state.micMuted ?? false,
          deviceName: this.state.activeMic?.name ?? 'Microphone',
          formFactor: this.state.activeMic?.formFactor ?? 4,
          entrySeq,
          boundBump,
          bumpSeq,
          expiresAtMs: nowMs + VOLUME_DURATION_MS,
        };
        this.state.idleSinceMs = null;
        this.state.suspendDispatched = false;
        return { status: 'applied', wokeActivity: true };
      }

      case 'micMuteChanged': {
        this.state.micMuted = ev.muted;
        this.state.activity = {
          kind: 'mic',
          muted: ev.muted,
          deviceName: this.state.activeMic?.name ?? 'Microphone',
          entranceSeq: ++this.seqCounter,
          expiresAtMs: nowMs + MIC_DURATION_MS,
        };
        this.state.idleSinceMs = null;
        this.state.suspendDispatched = false;
        return { status: 'applied', wokeActivity: true };
      }

      case 'switched': {
        const prevName = this.state.activeDevice?.name ?? null;
        if (ev.device.flow === 'render') {
          this.state.playback = updateSwitchDefaults(this.state.playback, ev.device, ev.roles);
          this.state.activeDevice = findActivePlayback(this.state.playback);
          this.state.targetDevice = findNextTarget(this.state.playback);
        } else {
          this.state.recording = updateSwitchDefaults(this.state.recording, ev.device, ev.roles);
          this.state.activeMic = findActiveMic(this.state.recording);
        }

        const recentArrive =
          this.state.lastArrivedDevice !== null &&
          this.state.lastArrivedDevice.id === ev.device.id &&
          nowMs - this.state.lastArrivedDevice.atMs <= PLUG_SWITCH_MERGE_WINDOW_MS;
        const activePlugArrive =
          this.state.activity.kind === 'plug' &&
          this.state.activity.direction === 'arrived' &&
          this.state.activity.device.id === ev.device.id;
        const mergedPlug = recentArrive || activePlugArrive;
        if (mergedPlug) {
          this.state.lastArrivedDevice = null;
        }

        const effectiveDevice =
          (ev.device.flow === 'render' ? this.state.activeDevice : this.state.activeMic) ?? ev.device;
        this.state.activity = {
          kind: 'switch',
          previousDevice: prevName && prevName !== ev.device.name ? prevName : null,
          device: effectiveDevice,
          relinked: ev.relinked,
          mergedPlug,
          spatialBadge: formatSpatialBadge(ev.spatialApplied ?? ev.device.spatial),
          switchSeq: ++this.seqCounter,
          expiresAtMs: nowMs + SWITCH_DURATION_MS,
        };
        this.state.idleSinceMs = null;
        this.state.suspendDispatched = false;
        return { status: 'applied', wokeActivity: true };
      }

      case 'deviceArrived': {
        if (ev.device.flow === 'render') {
          this.state.playback = upsertArrivedDevice(this.state.playback, ev.device);
          this.state.activeDevice = findActivePlayback(this.state.playback);
          this.state.targetDevice = findNextTarget(this.state.playback);
        } else {
          this.state.recording = upsertArrivedDevice(this.state.recording, ev.device);
          this.state.activeMic = findActiveMic(this.state.recording);
        }
        this.state.lastArrivedDevice = { id: ev.device.id, atMs: nowMs };

        // Codex D-13 merge rule: if switched to the same device just arrived, merge cleanly into the switch pill.
        if (this.state.activity.kind === 'switch' && this.state.activity.device.id === ev.device.id) {
          this.state.activity = {
            ...this.state.activity,
            mergedPlug: true,
            expiresAtMs: nowMs + SWITCH_DURATION_MS,
          };
        } else {
          this.state.activity = {
            kind: 'plug',
            direction: 'arrived',
            device: ev.device,
            plugSeq: ++this.seqCounter,
            expiresAtMs: nowMs + PLUG_DURATION_MS,
          };
        }
        this.state.idleSinceMs = null;
        this.state.suspendDispatched = false;
        return { status: 'applied', wokeActivity: true };
      }

      case 'deviceLeft': {
        if (ev.device.flow === 'render') {
          this.state.playback = this.state.playback.filter((d) => d.id !== ev.device.id);
          this.state.activeDevice = findActivePlayback(this.state.playback);
          this.state.targetDevice = findNextTarget(this.state.playback);
        } else {
          this.state.recording = this.state.recording.filter((d) => d.id !== ev.device.id);
          this.state.activeMic = findActiveMic(this.state.recording);
        }
        if (this.state.lastArrivedDevice?.id === ev.device.id) {
          this.state.lastArrivedDevice = null;
        }
        this.state.activity = {
          kind: 'plug',
          direction: 'left',
          device: ev.device,
          plugSeq: ++this.seqCounter,
          expiresAtMs: nowMs + PLUG_DURATION_MS,
        };
        this.state.idleSinceMs = null;
        this.state.suspendDispatched = false;
        return { status: 'applied', wokeActivity: true };
      }

      case 'devicesChanged': {
        this.state.playback = ev.playback;
        this.state.recording = ev.recording;
        this.state.activeDevice = findActivePlayback(ev.playback);
        this.state.targetDevice = findNextTarget(ev.playback);
        this.state.activeMic = findActiveMic(ev.recording);
        return { status: 'applied', wokeActivity: false };
      }

      case 'noticesChanged': {
        this.state.notices = ev.notices;
        return { status: 'applied', wokeActivity: false };
      }

      case 'notice':
      case 'settingsChanged': {
        return { status: 'applied', wokeActivity: false };
      }
    }
  }

  /** Marks manual hover/pointer interaction so auto-dismiss and idle suspend wait cleanly. */
  markInteraction(nowMs: number): void {
    this.state.suspendDispatched = false;
    if (this.state.activity.kind === 'idle') {
      this.state.idleSinceMs = nowMs;
    }
  }

  /** Advances expiry/idle state; caller schedules one-shot timeouts via `nextTickDelayMs`. */
  tick(nowMs: number, holdOpen: boolean): { dismissedToIdle: boolean; shouldRequestSuspend: boolean } {
    if (holdOpen) {
      this.state.suspendDispatched = false;
      if (this.state.activity.kind === 'idle') {
        this.state.idleSinceMs = nowMs;
      }
      return { dismissedToIdle: false, shouldRequestSuspend: false };
    }

    let dismissedToIdle = false;
    if (this.state.activity.kind !== 'idle' && nowMs >= this.state.activity.expiresAtMs) {
      this.state.activity = { kind: 'idle' };
      this.state.idleSinceMs = nowMs;
      this.state.suspendDispatched = false;
      dismissedToIdle = true;
    }

    const shouldRequestSuspend =
      this.state.activity.kind === 'idle' &&
      !this.state.suspendDispatched &&
      this.state.idleSinceMs !== null &&
      nowMs - this.state.idleSinceMs >= IDLE_SUSPEND_LINGER_MS;

    if (shouldRequestSuspend) {
      this.state.suspendDispatched = true;
    }
    return { dismissedToIdle, shouldRequestSuspend };
  }

  /** Computes the exact delay in ms until the next state transition (or `null` when nothing is pending). */
  nextTickDelayMs(nowMs: number, holdOpen: boolean): number | null {
    if (holdOpen) {
      return null;
    }
    if (this.state.activity.kind !== 'idle') {
      return Math.max(0, this.state.activity.expiresAtMs - nowMs);
    }
    if (!this.state.suspendDispatched && this.state.idleSinceMs !== null) {
      return Math.max(0, this.state.idleSinceMs + IDLE_SUSPEND_LINGER_MS - nowMs);
    }
    return null;
  }
}
