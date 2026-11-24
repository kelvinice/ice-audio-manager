import type { NowPlaying, NowPlayingPill } from '../ipc/engine.ts';
import { isMediaActive, type ActivityView } from './scheduler.ts';

/**
 * What the notch shows for the `appearance.nowPlaying` setting:
 * - `while_playing` (default): the idle notch becomes the now-playing pill whenever something plays;
 * - `brief` and `off`: the scheduler's activity as is (`off` never creates a track pop-up).
 *
 * Derived from state, so it needs no timer and survives a WebView reload. It reuses the last pop-up's `trackSeq`,
 * so the brief track pop-up flows into it without replaying the entrance animation. Pausing returns to idle.
 */
export function resolveDisplayedView(
  activity: ActivityView,
  media: NowPlaying | null,
  mode: NowPlayingPill,
  lastTrackSeq: number,
): ActivityView {
  if (activity.kind !== 'idle' || mode !== 'while_playing') return activity;
  if (!isMediaActive(media) || media.status !== 'playing') return activity;
  return {
    kind: 'track',
    media,
    previousTitle: null,
    previousArtist: null,
    previousArt: null,
    trackSeq: lastTrackSeq,
    expiresAtMs: Number.POSITIVE_INFINITY,
  };
}
