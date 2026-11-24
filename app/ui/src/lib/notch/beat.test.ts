import assert from 'node:assert/strict';
import test from 'node:test';
import type { MeterFrame } from '../ipc/engine.ts';
import { BEAT_BAR_COUNT, BeatShaper, beatHash, pickBars } from './beat.ts';

const FRAME_MS = 33; // the engine's meter cadence (30 Hz)
const frame = (l: number, r = l): MeterFrame => ({ peak: Math.max(l, r), channels: [l, r] });
const avg = (xs: number[]) => xs.reduce((s, v) => s + v, 0) / xs.length;

/** Feeds `frames` at 30 Hz and returns every output (copied). */
function run(shaper: BeatShaper, frames: MeterFrame[], startMs = 0): number[][] {
  return frames.map((f, i) => [...shaper.shape(f, startMs + i * FRAME_MS)]);
}

/** A kick on every 15th frame (2 beats/s = 120 BPM) over a quieter bed; loud masters barely dip between hits. */
function beatPattern(seconds: number, hit: number, bed: number): MeterFrame[] {
  return Array.from({ length: Math.round(seconds * 30) }, (_, i) => frame(i % 15 === 0 ? hit : bed));
}

test('silence puts every bar at rest', () => {
  const s = new BeatShaper();
  const out = run(s, [frame(0), frame(0.0004)]);
  assert.ok(out.every((bars) => bars.length === BEAT_BAR_COUNT && bars.every((v) => v === 0)));
});

test('a steady loud signal settles low instead of pinning the bars at the top', () => {
  const s = new BeatShaper();
  const out = run(s, Array.from({ length: 90 }, () => frame(0.9)));
  const settled = out.at(-1)!;
  assert.ok(Math.max(...settled) < 0.35, `steady tone settles low: ${settled.map((v) => v.toFixed(2))}`);
});

test('beats throw the bars high and they drop between hits, for loud and quiet tracks alike', () => {
  for (const [hit, bed] of [
    [0.95, 0.6], // loud master: only ~4 dB of dip between kicks
    [0.08, 0.03], // quiet acoustic track
  ]) {
    const s = new BeatShaper();
    const out = run(s, beatPattern(4, hit, bed));
    const last2s = out.slice(-60);
    const onHits = last2s.filter((_, i) => (out.length - 60 + i) % 15 === 0).map((b) => Math.max(...b));
    const between = last2s.filter((_, i) => (out.length - 60 + i) % 15 === 7).map((b) => Math.max(...b));
    assert.ok(avg(onHits) > 0.6, `hits reach high (${hit}/${bed}): ${onHits.map((v) => v.toFixed(2))}`);
    assert.ok(avg(between) < 0.3, `bars fall between hits (${hit}/${bed}): ${between.map((v) => v.toFixed(2))}`);
  }
});

test('each beat throws a different pattern across the bars', () => {
  const s = new BeatShaper();
  const out = run(s, beatPattern(2, 0.9, 0.4));
  const hits = out.filter((_, i) => i % 15 === 0 && i > 0);
  const shape = (b: number[]) => b.map((v) => v.toFixed(2)).join(',');
  assert.ok(new Set(hits.map(shape)).size >= hits.length - 1, 'beats don\'t repeat one skyline');
  assert.ok(hits.every((b) => Math.max(...b) - Math.min(...b) > 0.15), 'bars differ within one hit');
});

test('left-only sound lifts the left bars', () => {
  const s = new BeatShaper();
  const frames = Array.from({ length: 60 }, (_, i) => frame(i % 15 === 0 ? 0.9 : 0.2, 0.0002));
  const hit = run(s, frames).filter((_, i) => i % 15 === 0).at(-1)!;
  assert.ok(avg(hit.slice(0, 4)) > avg(hit.slice(-4)) + 0.2, `left > right: ${hit.map((v) => v.toFixed(2))}`);
});

test('pickBars takes independent bars evenly, and beatHash is deterministic', () => {
  const bank = Array.from({ length: BEAT_BAR_COUNT }, (_, i) => i / (BEAT_BAR_COUNT - 1));
  assert.deepEqual(pickBars(bank, 5).map((v) => v.toFixed(2)), ['0.00', '0.23', '0.54', '0.77', '1.00']);
  assert.equal(beatHash(3, 4), beatHash(3, 4));
  assert.ok(beatHash(3, 4) >= 0 && beatHash(3, 4) < 1);
});
