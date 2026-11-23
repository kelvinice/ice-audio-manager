import assert from 'node:assert/strict';
import test from 'node:test';
import type { MeterFrame } from '../ipc/engine.ts';
import { SPRING_PRESETS } from '../motion/spring.ts';
import {
  barScaleY,
  DEFAULT_BAR_COUNT,
  deriveVisualizerSpringParams,
  mapFrameToBars,
  MIN_BAR_SCALE,
  perceptualLevel,
  SILENT_THRESHOLD,
  VisualizerBank,
} from './visualizer.ts';

function stepBankFor(bank: VisualizerBank, hz: number, totalSec: number): boolean {
  const dt = 1 / hz;
  const steps = Math.round(totalSec * hz);
  let moving = false;
  for (let i = 0; i < steps; i += 1) {
    moving = bank.step(dt);
  }
  return moving;
}

test('mapFrameToBars spreads channels across 4–6 bars and falls back to peak when channels are empty', () => {
  const emptyChannels: MeterFrame = { peak: 0.64, channels: [] };
  for (const count of [4, 5, 6]) {
    const bars = mapFrameToBars(emptyChannels, count);
    assert.equal(bars.length, count);
    assert.ok(bars.every((v) => v > 0.5 && v <= 1));
  }

  const leftOnly: MeterFrame = { peak: 0.8, channels: [0.8, 0] };
  const leftBars = mapFrameToBars(leftOnly, 5);
  assert.ok(leftBars[0] > 0.6);
  assert.equal(leftBars[4], 0);

  const rightOnly: MeterFrame = { peak: 0.8, channels: [0, 0.8] };
  const rightBars = mapFrameToBars(rightOnly, 5);
  assert.equal(rightBars[0], 0);
  assert.ok(rightBars[4] > 0.6);

  // Surround endpoint with stereo L/R active and rear channels silent shouldn't leave right bars dead.
  const surroundStereo: MeterFrame = { peak: 0.7, channels: [0.6, 0.7, 0, 0, 0, 0] };
  const surroundBars = mapFrameToBars(surroundStereo, 5);
  assert.ok(surroundBars[0] > 0.5);
  assert.ok(surroundBars[4] > 0.5);
});

test('perceptual curve lifts quiet music while silence and NaN stay zero', () => {
  assert.equal(perceptualLevel(0), 0);
  assert.equal(perceptualLevel(SILENT_THRESHOLD * 0.5), 0);
  assert.equal(perceptualLevel(Number.NaN), 0);

  const quietBar = perceptualLevel(0.02);
  assert.ok(quietBar > 0.15, `expected quiet 0.02 amplitude to produce visible bar, got ${quietBar}`);
  assert.equal(perceptualLevel(1.5), 1);

  const silent = mapFrameToBars({ peak: 0.0002, channels: [0, 0.0004] }, DEFAULT_BAR_COUNT);
  assert.deepEqual(silent, [0, 0, 0, 0, 0]);
  assert.equal(barScaleY(0), MIN_BAR_SCALE);
  assert.equal(barScaleY(1), 1);
});

test('fast attack rises sharply while slow decay glides smoothly and stops loop on settle', () => {
  const bank = new VisualizerBank(5, 'smooth');
  const hot: MeterFrame = { peak: 0.9, channels: [0.9, 0.9] };
  assert.equal(bank.setFrame(hot), true);

  const targets = mapFrameToBars(hot, 5);
  stepBankFor(bank, 144, 0.045);
  const afterAttack = bank.values();
  assert.ok(
    afterAttack[2] >= targets[2] * 0.75,
    `expected center bar to reach >=75% within 45 ms, got ${afterAttack[2]} vs target ${targets[2]}`,
  );

  // Settle at target, then drop to silence and check slow decay.
  stepBankFor(bank, 144, 0.4);
  const peakValues = bank.values();
  bank.setFrame({ peak: 0, channels: [] });

  stepBankFor(bank, 144, 0.15);
  const midDecay = bank.values();
  assert.ok(
    midDecay[2] > peakValues[2] * 0.15,
    `expected slow decay still >15% at 150 ms, got ${midDecay[2]} from ${peakValues[2]}`,
  );

  const stillMoving = stepBankFor(bank, 144, 0.85);
  assert.equal(stillMoving, false);
  assert.equal(bank.isSettled(), true);
  assert.deepEqual(bank.values(), [0, 0, 0, 0, 0]);
  assert.equal(bank.step(1 / 144), false);
});

test('visualizer springs are derived from config presets and remain frame-rate independent at 75 and 144 Hz', () => {
  for (const preset of ['snappy', 'smooth', 'bouncy'] as const) {
    const attack = deriveVisualizerSpringParams(preset, 'attack');
    const decay = deriveVisualizerSpringParams(preset, 'decay');
    assert.ok(attack.stiffness > SPRING_PRESETS[preset].stiffness);
    assert.ok(decay.stiffness < SPRING_PRESETS[preset].stiffness);
    assert.equal(attack.instant, undefined);
  }

  const runBankTo = (bank: VisualizerBank, hz: number, elapsedSec: number): void => {
    const dt = 1 / hz;
    let t = 0;
    while (t + dt <= elapsedSec + 1e-12) {
      bank.step(dt);
      t += dt;
    }
    const rem = elapsedSec - t;
    if (rem > 1e-12) {
      bank.step(rem);
    }
  };

  const runSequence = (hz: number) => {
    const bank = new VisualizerBank(5, 'bouncy');
    bank.setFrame({ peak: 0.85, channels: [0.8, 0.85] });
    runBankTo(bank, hz, 0.06);
    bank.setFrame({ peak: 0.15, channels: [0.1, 0.15] });
    runBankTo(bank, hz, 0.12);
    return bank.values();
  };

  const v75 = runSequence(75);
  const v144 = runSequence(144);
  const v240 = runSequence(240);
  for (let i = 0; i < v75.length; i += 1) {
    const diff75_144 = Math.abs(v75[i] - v144[i]);
    const diff144_240 = Math.abs(v144[i] - v240[i]);
    assert.ok(diff75_144 < 1e-8, `75 Hz vs 144 Hz mismatch on bar ${i}: ${diff75_144}`);
    assert.ok(diff144_240 < 1e-8, `144 Hz vs 240 Hz mismatch on bar ${i}: ${diff144_240}`);
  }
  // Exact integer-step timestamp match: t = 1/3 s = 25 steps @ 75 Hz = 48 steps @ 144 Hz.
  const exact75 = new VisualizerBank(5, 'smooth');
  const exact144 = new VisualizerBank(5, 'smooth');
  exact75.setFrame({ peak: 0.9, channels: [0.9, 0.4] });
  exact144.setFrame({ peak: 0.9, channels: [0.9, 0.4] });
  stepBankFor(exact75, 75, 1 / 3);
  stepBankFor(exact144, 144, 1 / 3);
  for (let i = 0; i < 5; i += 1) {
    assert.ok(Math.abs(exact75.values()[i] - exact144.values()[i]) < 1e-8);
  }
});

test('reduce motion / off preset jumps immediately with zero animation frames', () => {
  const bank = new VisualizerBank(5, 'smooth');
  bank.setPreset('smooth', true);
  const frame: MeterFrame = { peak: 0.72, channels: [0.72, 0.36] };
  assert.equal(bank.setFrame(frame), false);
  assert.equal(bank.isSettled(), true);
  assert.deepEqual(bank.values(), mapFrameToBars(frame, 5));
  assert.equal(bank.step(1 / 144), false);

  bank.setFrame({ peak: 0, channels: [] });
  assert.equal(bank.isSettled(), true);
  assert.deepEqual(bank.values(), [0, 0, 0, 0, 0]);
});
