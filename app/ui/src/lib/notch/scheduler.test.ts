import assert from 'node:assert/strict';
import test from 'node:test';
import type { DeviceSummary, EngineSnapshot } from '../ipc/engine.ts';
import {
  IDLE_SUSPEND_LINGER_MS,
  NotchActivityScheduler,
  SWITCH_DURATION_MS,
  VOLUME_DURATION_MS,
  type ActivityView,
} from './scheduler.ts';

function expectActivity<K extends ActivityView['kind']>(
  sched: NotchActivityScheduler,
  kind: K,
): Extract<ActivityView, { kind: K }> {
  assert.equal(sched.state.activity.kind, kind);
  return sched.state.activity as Extract<ActivityView, { kind: K }>;
}

function sampleDevice(overrides: Partial<DeviceSummary> = {}): DeviceSummary {
  return {
    id: 'fake-arctis',
    name: 'Headphones (Arctis Nova 7)',
    flow: 'render',
    state: 'active',
    isDefault: true,
    isDefaultCommunications: true,
    formFactor: 3,
    isVirtual: false,
    spatial: 'dolbyAtmosHeadphones',
    ...overrides,
  };
}

function sampleSnapshot(): EngineSnapshot {
  return {
    revision: 1,
    playback: [
      sampleDevice(),
      sampleDevice({
        id: 'fake-realtek',
        name: 'Speakers (Realtek(R) Audio)',
        isDefault: false,
        isDefaultCommunications: false,
        formFactor: 1,
        spatial: 'off',
      }),
    ],
    recording: [
      sampleDevice({
        id: 'fake-arctis-mic',
        name: 'Microphone (Arctis Nova 7)',
        flow: 'capture',
        formFactor: 4,
        spatial: null,
      }),
    ],
    volume: { percent: 40, muted: false },
    micMuted: false,
    micVolume: 80,
    notices: [],
    settings: {
      defaultRoles: 'default',
      togglePairs: [],
      bindings: [],
      favouritesPlayback: [],
      favouritesRecording: [],
      devices: [],
      knobGestures: false,
    },
  };
}

test('initial snapshot keeps idle state and D-03 revision guard rejects stale or gapped envelopes', () => {
  const sched = new NotchActivityScheduler(1000);
  sched.applySnapshot(sampleSnapshot(), 1000);
  assert.equal(sched.state.activity.kind, 'idle');
  assert.equal(sched.state.revision, 1);

  const stale = sched.applyEnvelope(
    { revision: 1, event: { type: 'volumeChanged', volume: { percent: 50, muted: false } } },
    1050,
  );
  assert.equal(stale.status, 'ignored_stale');
  assert.equal(sched.state.activity.kind, 'idle');

  const gap = sched.applyEnvelope(
    { revision: 4, event: { type: 'volumeChanged', volume: { percent: 50, muted: false } } },
    1060,
  );
  assert.equal(gap.status, 'needs_resnapshot');
});

test('repeat volume ticks merge without re-triggering entry pulse and detect 0%/100% bounds', () => {
  const sched = new NotchActivityScheduler(0);
  sched.applySnapshot(sampleSnapshot(), 0);

  const r1 = sched.applyEnvelope(
    { revision: 2, event: { type: 'volumeChanged', volume: { percent: 95, muted: false } } },
    100,
  );
  assert.equal(r1.status, 'applied');
  const v1 = expectActivity(sched, 'volume');
  const firstEntrySeq = v1.entrySeq;

  const r2 = sched.applyEnvelope(
    { revision: 3, event: { type: 'volumeChanged', volume: { percent: 100, muted: false } } },
    200,
  );
  assert.equal(r2.status, 'applied');
  const v2 = expectActivity(sched, 'volume');
  assert.equal(v2.entrySeq, firstEntrySeq, 'repeat tick must not replay entry pulse');
  assert.equal(v2.boundBump, 0);

  sched.applyEnvelope(
    { revision: 4, event: { type: 'volumeChanged', volume: { percent: 100, muted: false } } },
    260,
  );
  const v3 = expectActivity(sched, 'volume');
  assert.equal(v3.boundBump, 1);
  assert.equal(v3.expiresAtMs, 260 + VOLUME_DURATION_MS);
});

test('micVolumeChanged uses mic channel and preserves micMuted state', () => {
  const sched = new NotchActivityScheduler(0);
  sched.applySnapshot(sampleSnapshot(), 0);
  sched.applyEnvelope({ revision: 2, event: { type: 'micMuteChanged', muted: true } }, 100);
  sched.applyEnvelope({ revision: 3, event: { type: 'micVolumeChanged', percent: 62 } }, 200);
  const v = expectActivity(sched, 'volume');
  assert.equal(v.channel, 'mic');
  assert.equal(v.percent, 62);
  assert.equal(v.muted, true);
  assert.equal(sched.state.micMuted, true);
});

test('deviceArrived and switched for same device merge into a single switch notification', () => {
  const sched = new NotchActivityScheduler(0);
  sched.applySnapshot(sampleSnapshot(), 0);

  const usbDac = sampleDevice({
    id: 'fake-usb-dac',
    name: 'Headphones (USB DAC)',
    isDefault: false,
    isDefaultCommunications: false,
    spatial: 'windowsSonic',
  });

  sched.applyEnvelope({ revision: 2, event: { type: 'deviceArrived', device: usbDac } }, 500);
  const p = expectActivity(sched, 'plug');
  assert.equal(p.direction, 'arrived');

  sched.applyEnvelope(
    {
      revision: 3,
      event: {
        type: 'switched',
        device: usbDac,
        roles: 'default',
        relinked: true,
        spatialApplied: 'dolbyAtmosHeadphones',
      },
    },
    900,
  );
  const sw = expectActivity(sched, 'switch');
  assert.equal(sw.mergedPlug, true);
  assert.equal(sw.relinked, true);
  assert.equal(sw.spatialBadge, 'Atmos');
  assert.equal(sw.previousDevice, 'Headphones (Arctis Nova 7)');
  assert.equal(sw.expiresAtMs, 900 + SWITCH_DURATION_MS);

  // Reverse order (switched first, deviceArrived right after) preserves switch card and marks mergedPlug.
  const sched2 = new NotchActivityScheduler(0);
  sched2.applySnapshot(sampleSnapshot(), 0);
  sched2.applyEnvelope(
    {
      revision: 2,
      event: { type: 'switched', device: usbDac, roles: 'default', relinked: false, spatialApplied: null },
    },
    500,
  );
  sched2.applyEnvelope({ revision: 3, event: { type: 'deviceArrived', device: usbDac } }, 700);
  const sw2 = expectActivity(sched2, 'switch');
  assert.equal(sw2.mergedPlug, true);
});

test('deviceLeft removes endpoint and triggers left plug activity', () => {
  const sched = new NotchActivityScheduler(0);
  sched.applySnapshot(sampleSnapshot(), 0);
  const removed = sampleDevice({ state: 'not_present', isDefault: false, isDefaultCommunications: false, spatial: null });
  sched.applyEnvelope({ revision: 2, event: { type: 'deviceLeft', device: removed } }, 400);
  assert.equal(sched.state.playback.some((d) => d.id === removed.id), false);
  const left = expectActivity(sched, 'plug');
  assert.equal(left.direction, 'left');
});

test('micMuteChanged leaves persistent static indicator on idle and schedules TrySuspend linger', () => {
  const sched = new NotchActivityScheduler(0);
  sched.applySnapshot(sampleSnapshot(), 0);
  assert.equal(sched.tick(IDLE_SUSPEND_LINGER_MS, false).shouldRequestSuspend, true);
  assert.equal(sched.tick(IDLE_SUSPEND_LINGER_MS + 1000, false).shouldRequestSuspend, false);

  sched.applyEnvelope({ revision: 2, event: { type: 'micMuteChanged', muted: true } }, 20_000);
  assert.equal(sched.state.activity.kind, 'mic');
  assert.equal(sched.state.micMuted, true);

  // While hovered/expanded, holdOpen blocks auto-dismiss and suspend.
  assert.equal(sched.tick(25_000, true).dismissedToIdle, false);
  assert.equal(sched.nextTickDelayMs(25_000, true), null);

  // Once released past expiry, dismisses to idle (keeping micMuted) and triggers suspend after linger.
  const exp = sched.tick(25_000, false);
  assert.equal(exp.dismissedToIdle, true);
  assert.equal(exp.shouldRequestSuspend, false);
  assert.equal(sched.state.activity.kind, 'idle');
  assert.equal(sched.state.micMuted, true);
  assert.equal(sched.nextTickDelayMs(25_000, false), IDLE_SUSPEND_LINGER_MS);

  const sus = sched.tick(25_000 + IDLE_SUSPEND_LINGER_MS, false);
  assert.equal(sus.shouldRequestSuspend, true);
});
