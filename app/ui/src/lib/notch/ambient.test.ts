import assert from 'node:assert/strict';
import test from 'node:test';
import {
  AMBIENT_TEMPO_MAX,
  AMBIENT_TEMPO_MIN,
  BREATH_STEP,
  BreathEnvelope,
  ambientTempoRate,
} from './ambient.ts';

test('the lamp follows the tempo gently and stays inside its range', () => {
  assert.equal(ambientTempoRate(1), 1);
  assert.ok(ambientTempoRate(1.65) > 1 && ambientTempoRate(1.65) <= AMBIENT_TEMPO_MAX, 'fast songs: livelier');
  assert.ok(ambientTempoRate(0.58) < 1 && ambientTempoRate(0.58) >= AMBIENT_TEMPO_MIN, 'ballads: calmer');
  assert.equal(ambientTempoRate(9), AMBIENT_TEMPO_MAX);
  assert.equal(ambientTempoRate(0), AMBIENT_TEMPO_MIN);
  assert.equal(ambientTempoRate(Number.NaN), 1);
});

test('a kick swells the breath at once, then it exhales slowly', () => {
  const b = new BreathEnvelope();
  assert.equal(b.update(0, 0), 0);
  assert.equal(b.update(0.9, 33), 0.9, 'instant swell');
  const after100 = b.update(0, 133);
  const after400 = b.update(0, 433);
  assert.ok(after100 < 0.9 && after100 > 0.6, `still high after 100 ms: ${after100}`);
  assert.ok(after400 < after100 && after400 > 0.1, `exhaling after 400 ms: ${after400}`);
  assert.equal(b.update(0, 3433), 0, 'at rest after a few seconds');
});

test('weak kicks are ignored and values come in steps', () => {
  const b = new BreathEnvelope();
  assert.equal(b.update(0.05, 0), 0, 'below the kick threshold');
  const v = b.update(0.537, 33);
  assert.ok(Math.abs(v / BREATH_STEP - Math.round(v / BREATH_STEP)) < 1e-9, `quantised: ${v}`);
  b.reset();
  assert.equal(b.update(0, 66), 0, 'reset = rest');
});
