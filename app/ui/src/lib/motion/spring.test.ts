import assert from 'node:assert/strict';
import test from 'node:test';
import { SPRING_PRESETS, Spring1D, type SpringParams } from './spring.ts';

function stepForDuration(initial: number, target: number, params: SpringParams, hz: number, totalSec: number) {
  const s = new Spring1D(initial, params);
  s.setTarget(target);
  const dt = 1 / hz;
  const steps = Math.round(totalSec * hz);
  for (let i = 0; i < steps; i++) {
    s.step(dt);
  }
  return { value: s.value, velocity: s.velocity, settled: s.isSettled() };
}

test('bouncy preset produces ~8% overshoot (7.5%..8.5%)', () => {
  const s = new Spring1D(0, SPRING_PRESETS.bouncy);
  s.setTarget(100);
  let peak = 0;
  for (let i = 0; i < 144; i++) {
    s.step(1 / 144);
    if (s.value > peak) peak = s.value;
  }
  const overshootPct = peak - 100;
  assert.ok(overshootPct >= 7.5 && overshootPct <= 8.5, `expected ~8% overshoot, got ${overshootPct.toFixed(2)}%`);
});

test('snappy has minimal overshoot and smooth is nearly critically damped', () => {
  const stepPeak = (params: SpringParams) => {
    const s = new Spring1D(0, params);
    s.setTarget(100);
    let peak = 0;
    for (let i = 0; i < 240; i++) {
      s.step(1 / 240);
      if (s.value > peak) peak = s.value;
    }
    return peak - 100;
  };
  assert.ok(stepPeak(SPRING_PRESETS.snappy) < 2.0);
  assert.ok(stepPeak(SPRING_PRESETS.smooth) < 0.25);
});

test('frame-rate invariance across 75 Hz, 144 Hz, and 240 Hz', () => {
  // Check exact elapsed instants divisible across sample intervals using common durations (0.2s = 15@75Hz, 48@240Hz; 0.4s = 30@75Hz, 96@240Hz; and 1s exact across all 3)
  for (const preset of [SPRING_PRESETS.snappy, SPRING_PRESETS.smooth, SPRING_PRESETS.bouncy]) {
    const a75 = stepForDuration(0, 200, preset, 75, 0.2);
    const a240 = stepForDuration(0, 200, preset, 240, 0.2);
    assert.ok(Math.abs(a75.value - a240.value) < 1e-8, `75 vs 240 value delta: ${Math.abs(a75.value - a240.value)}`);
    assert.ok(Math.abs(a75.velocity - a240.velocity) < 1e-8);

    // Also test 144 Hz vs 75 Hz via exact timestamp match at t = 1/3 s (25 steps @ 75Hz vs 48 steps @ 144Hz)
    const b75 = stepForDuration(0, 200, preset, 75, 1 / 3);
    const b144 = stepForDuration(0, 200, preset, 144, 1 / 3);
    assert.ok(Math.abs(b75.value - b144.value) < 1e-8, `75 vs 144 value delta: ${Math.abs(b75.value - b144.value)}`);
    assert.ok(Math.abs(b75.velocity - b144.velocity) < 1e-8);
  }
});

test('mid-flight retargeting preserves velocity identically at 75 Hz, 144 Hz, and 240 Hz', () => {
  const runTo = (targetElapsed: number, stepHz: number, s: Spring1D) => {
    const dt = 1 / stepHz;
    let t = 0;
    while (t + dt <= targetElapsed + 1e-12) {
      s.step(dt);
      t += dt;
    }
    const rem = targetElapsed - t;
    if (rem > 1e-12) s.step(rem);
  };

  const runRetarget = (hz: number) => {
    const s = new Spring1D(84, SPRING_PRESETS.bouncy);
    s.setTarget(284);
    runTo(0.08, hz, s);
    const vAtSwitch = s.velocity;
    assert.ok(vAtSwitch > 100, 'spring should be mid-flight at 80 ms');
    s.setTarget(140);
    assert.equal(s.velocity, vAtSwitch, 'retarget must not reset velocity');
    runTo(0.09, hz, s);
    return s.value;
  };
  const v75 = runRetarget(75);
  const v144 = runRetarget(144);
  const v240 = runRetarget(240);
  assert.ok(Math.abs(v75 - v144) < 1e-8, `75 vs 144 retarget delta: ${Math.abs(v75 - v144)}`);
  assert.ok(Math.abs(v144 - v240) < 1e-8, `144 vs 240 retarget delta: ${Math.abs(v144 - v240)}`);
});

test('off preset snaps immediately and auto-sleep stops animation loop', () => {
  const off = new Spring1D(10, SPRING_PRESETS.off);
  off.setTarget(300);
  assert.equal(off.value, 300);
  assert.equal(off.velocity, 0);
  assert.equal(off.isSettled(), true);
  assert.equal(off.step(1 / 144), false);

  const settledCheck = stepForDuration(0, 100, SPRING_PRESETS.smooth, 144, 1.0);
  assert.equal(settledCheck.settled, true);
  assert.equal(settledCheck.value, 100);
  assert.equal(settledCheck.velocity, 0);
});
