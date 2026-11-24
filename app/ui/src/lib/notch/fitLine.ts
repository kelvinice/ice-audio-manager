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
  return { scale, x: shift > 0 ? -shift : 0, fadeLeft: shift > 0.5, fadeRight: shift < overflow - 0.5 };
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

interface CachedMeasures {
  key: string;
  enabled: boolean;
  avail: number;
  need: number;
  wordEnds: number[];
}

export function fitLayoutsEqual(a: FitLayout | null, b: FitLayout): boolean {
  return (
    a !== null &&
    Math.abs(a.scale - b.scale) < 1e-4 &&
    Math.abs(a.x - b.x) < 0.05 &&
    a.fadeLeft === b.fadeLeft &&
    a.fadeRight === b.fadeRight
  );
}

/**
 * Svelte action on `.rung-text`; its first child `.fit` holds the line. Measures word positions once per line/font
 * settle so word-highlight updates avoid synchronous layout reflows, and skips DOM writes when layout is unchanged.
 */
export function fitLine(node: HTMLElement, initial: FitParams) {
  let params = initial;
  let cache: CachedMeasures | null = null;
  let lastLayout: FitLayout | null = null;
  let destroyed = false;

  const measure = () => {
    const inner = node.firstElementChild as HTMLElement | null;
    if (!inner || !params.enabled) {
      cache = null;
      return;
    }
    const words = inner.querySelectorAll<HTMLElement>('.word');
    const wordEnds = new Array<number>(words.length);
    for (let i = 0; i < words.length; i += 1) {
      const w = words[i];
      wordEnds[i] = w.offsetLeft + w.offsetWidth;
    }
    cache = {
      key: params.key,
      enabled: params.enabled,
      avail: node.clientWidth,
      need: inner.offsetWidth,
      wordEnds,
    };
  };

  const apply = (remeasure: boolean) => {
    if (destroyed) return;
    const inner = node.firstElementChild as HTMLElement | null;
    if (!inner) return;
    if (!params.enabled) {
      cache = null;
      if (lastLayout !== null) {
        lastLayout = null;
        inner.style.transform = '';
        node.classList.remove('fade-l', 'fade-r', 'fitting');
      }
      return;
    }
    if (remeasure || !cache || cache.key !== params.key || cache.enabled !== params.enabled) {
      measure();
    }
    if (!cache) return;
    const activeEnd =
      params.word >= 0 && params.word < cache.wordEnds.length
        ? cache.wordEnds[params.word]
        : cache.wordEnds.length > 0
          ? 0
          : null;
    const layout = fitLineLayout(cache.avail, cache.need, activeEnd, params.center);
    if (fitLayoutsEqual(lastLayout, layout)) return;
    lastLayout = layout;
    const fitting = layout.scale < 1 || layout.x !== 0;
    inner.style.transform = fitting ? `translateX(${layout.x.toFixed(1)}px) scale(${layout.scale.toFixed(3)})` : '';
    node.classList.toggle('fitting', fitting);
    node.classList.toggle('fade-l', layout.fadeLeft);
    node.classList.toggle('fade-r', layout.fadeRight);
  };

  apply(true);
  void document.fonts?.ready.then(() => {
    if (!destroyed) apply(true);
  });
  return {
    update(next: FitParams) {
      const remeasure = next.key !== params.key || next.enabled !== params.enabled || next.center !== params.center;
      params = next;
      apply(remeasure);
    },
    destroy() {
      destroyed = true;
    },
  };
}
