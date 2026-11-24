/**
 * Fitting the main lyric line into its row (2026-11-24 fix: long lines lost their last characters).
 *
 * The karaoke line is made of `inline-block` word spans, and `text-overflow: ellipsis` doesn't apply to those, so a
 * line wider than the pill was cut mid-character; the active word's lift (scale 1.07) also poked past the edge.
 * Now the line shrinks to fit (down to `FIT_MIN_SCALE`), and if it's still too long it slides so the word being
 * sung stays in view, with soft fades on the cut edges.
 */

/** Smallest shrink before sliding instead (keeps the text readable at the pill's 12 px). */
export const FIT_MIN_SCALE = 0.82;
/** Room kept free at the edge for the active word's lift and glow. */
export const FIT_EDGE_PX = 4;
/** Where the active word's end is kept while sliding, as a share of the visible width. */
const FOLLOW_AT = 0.7;

export interface FitLayout {
  scale: number;
  /** Horizontal shift in px (≤ 0 while sliding; > 0 only to centre a shrunk line). */
  x: number;
  fadeLeft: boolean;
  fadeRight: boolean;
}

/**
 * Layout for a line `need` px wide (unscaled) in `avail` px. `activeEnd` is the right edge of the word being sung
 * (unscaled, from the line's start), or null without word timings. `center` centres a shrunk line (the card).
 */
export function fitLineLayout(avail: number, need: number, activeEnd: number | null, center = false): FitLayout {
  const room = Math.max(0, avail - FIT_EDGE_PX);
  if (need <= room || room <= 0) return { scale: 1, x: 0, fadeLeft: false, fadeRight: false };
  const scale = Math.max(FIT_MIN_SCALE, room / need);
  const width = need * scale;
  if (width <= room) {
    return { scale, x: center ? (room - width) / 2 : 0, fadeLeft: false, fadeRight: false };
  }
  const overflow = width - room;
  const target = activeEnd === null ? 0 : activeEnd * scale - room * FOLLOW_AT;
  const shift = Math.min(overflow, Math.max(0, target));
  return { scale, x: -shift, fadeLeft: shift > 0.5, fadeRight: shift < overflow - 0.5 };
}

export interface FitParams {
  /** Only the main row is fitted; other rows keep their ellipsis. */
  enabled: boolean;
  /** Active word index (-1 = none yet); re-fits as the line is sung. */
  word: number;
  /** Line key, so a new line re-measures. */
  key: string;
  center: boolean;
}

/**
 * Svelte action on `.rung-text`; its first child `.fit` holds the line. Sets the fit transform and the
 * `fade-l`/`fade-r` classes. Runs on line or word changes only (no loop), plus once when web fonts settle.
 */
export function fitLine(node: HTMLElement, initial: FitParams) {
  let params = initial;
  const apply = () => {
    const inner = node.firstElementChild as HTMLElement | null;
    if (!inner) return;
    if (!params.enabled) {
      inner.style.transform = '';
      node.classList.remove('fade-l', 'fade-r', 'fitting');
      return;
    }
    const words = inner.querySelectorAll<HTMLElement>('.word');
    const active = params.word >= 0 ? (words[params.word] ?? null) : null;
    const activeEnd = active ? active.offsetLeft + active.offsetWidth : words.length > 0 ? 0 : null;
    const layout = fitLineLayout(node.clientWidth, inner.offsetWidth, activeEnd, params.center);
    const fitting = layout.scale < 1 || layout.x !== 0;
    inner.style.transform = fitting ? `translateX(${layout.x.toFixed(1)}px) scale(${layout.scale.toFixed(3)})` : '';
    node.classList.toggle('fitting', fitting);
    node.classList.toggle('fade-l', layout.fadeLeft);
    node.classList.toggle('fade-r', layout.fadeRight);
  };
  apply();
  void document.fonts?.ready.then(apply);
  return {
    update(next: FitParams) {
      params = next;
      apply();
    },
  };
}
