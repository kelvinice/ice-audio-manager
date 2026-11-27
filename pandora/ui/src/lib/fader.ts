/**
 * Experimental fades (Settings → Experimental), two gain stages after the analyser, so the spectrum keeps reading the
 * song while it fades:
 * - `transition`: songs fade in, fade out over their last seconds, and a skip dips out quickly instead of cutting.
 * - `sleep`: the sleep timer lowers the music over its last stretch.
 * Both sit at 1 (no effect) unless a fade is running. Ramps are scheduled on the audio clock; no timers here.
 */

/** Fade-in at a song's start, and fade-out over its end. */
export const FADE_IN_S = 1.2;
export const FADE_OUT_S = 3;
/** Quick dip before switching songs, so a skip doesn't click. */
const DIP_S = 0.18;

export class Fader {
  readonly input: GainNode;
  readonly output: GainNode;
  private fadingOut = false;

  constructor(private readonly ctx: AudioContext) {
    this.input = ctx.createGain();
    this.output = ctx.createGain();
    this.input.connect(this.output);
  }

  /** Whether the end-of-song fade-out is running. */
  get isFadingOut(): boolean {
    return this.fadingOut;
  }

  private ramp(param: AudioParam, to: number, seconds: number): void {
    const now = this.ctx.currentTime;
    param.cancelScheduledValues(now);
    param.setValueAtTime(param.value, now);
    if (seconds <= 0) param.setValueAtTime(to, now);
    else param.linearRampToValueAtTime(to, now + seconds);
  }

  /** A song starts: from silence to full. */
  fadeIn(): void {
    this.fadingOut = false;
    const now = this.ctx.currentTime;
    this.input.gain.cancelScheduledValues(now);
    this.input.gain.setValueAtTime(0, now);
    this.input.gain.linearRampToValueAtTime(1, now + FADE_IN_S);
  }

  /** The song ends in `seconds`: reach silence right then. */
  fadeOutBy(seconds: number): void {
    this.fadingOut = true;
    this.ramp(this.input.gain, 0, Math.max(0.05, seconds));
  }

  /** Before a skip: a quick dip; resolves once it is silent. */
  dip(): Promise<void> {
    this.ramp(this.input.gain, 0, DIP_S);
    return new Promise((resolve) => window.setTimeout(resolve, DIP_S * 1000));
  }

  /** Full level again (a seek away from the end, or the feature switched off). */
  restore(seconds = 0.15): void {
    this.fadingOut = false;
    this.ramp(this.input.gain, 1, seconds);
  }

  /** The sleep fade: `level` 0–1 reached over `seconds`. */
  setSleepLevel(level: number, seconds: number): void {
    this.ramp(this.output.gain, Math.max(0, Math.min(1, level)), seconds);
  }
}
