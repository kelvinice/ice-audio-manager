import assert from 'node:assert/strict';
import test from 'node:test';
import type { LyricsConfig, LyricsDocument, LyricsState } from '../ipc/engine.ts';
import { resolveLyricFrame, shouldShowCardLyrics, shouldShowPillLyrics } from './lyrics.ts';

const sampleDoc: LyricsDocument = {
  trackKey: 'Spotify.exe\0Idol\0YOASOBI\0Idol',
  source: 'lrclib',
  syncKind: 'word',
  instrumental: false,
  plainText: null,
  lines: [
    {
      startMs: 5_000,
      endMs: 9_000,
      text: 'Hello world today',
      subtitle: null,
      words: [
        { startMs: 5_000, endMs: 6_200, text: 'Hello' },
        { startMs: 6_200, endMs: 7_500, text: 'world' },
        { startMs: 7_500, endMs: 9_000, text: 'today' },
      ],
    },
    {
      startMs: 9_000,
      endMs: 13_000,
      text: '無敵の笑顔',
      subtitle: 'muteki no egao',
      words: [
        { startMs: 9_000, endMs: 11_000, text: '無敵' },
        { startMs: 11_000, endMs: 13_000, text: 'の笑顔' },
      ],
    },
  ],
};

test('resolveLyricFrame tracks intro, active line/word, neighbours, and next boundary delay', () => {
  const intro = resolveLyricFrame(sampleDoc, 2_000, true);
  assert.equal(intro.activeIndex, -1);
  assert.equal(intro.nextLine?.text, 'Hello world today');
  assert.equal(intro.nextWakeDelayMs, 2_000);

  const midFirst = resolveLyricFrame(sampleDoc, 6_500, true);
  assert.equal(midFirst.activeIndex, 0);
  assert.equal(midFirst.activeWordIndex, 1);
  assert.equal(midFirst.prevLine, null);
  assert.equal(midFirst.nextLine?.text, '無敵の笑顔');
  assert.equal(midFirst.nextWakeDelayMs, 1_015);

  const second = resolveLyricFrame(sampleDoc, 11_200, true);
  assert.equal(second.activeIndex, 1);
  assert.equal(second.activeWordIndex, 1);
  assert.equal(second.prevLine?.text, 'Hello world today');
  assert.equal(second.activeLine?.subtitle, 'muteki no egao');
  assert.equal(second.nextLine, null);
});

test('shouldShowPillLyrics and shouldShowCardLyrics honour config, sync kind, and drawer state', () => {
  const cfg: LyricsConfig = {
    enabled: true,
    onlineLookup: false,
    showOnPill: true,
    showOnCard: true,
    wordHighlight: true,
    romaji: true,
    cacheMaxMb: 50,
  };
  const synced: LyricsState = { status: 'ready', trackKey: sampleDoc.trackKey, document: sampleDoc };
  const plain: LyricsState = {
    status: 'ready',
    trackKey: sampleDoc.trackKey,
    document: { ...sampleDoc, syncKind: 'plain', lines: [], plainText: 'Line one\nLine two' },
  };

  assert.equal(shouldShowPillLyrics(cfg, synced, true), true);
  assert.equal(shouldShowPillLyrics(cfg, plain, true), false, 'plain lyrics never extend compact pill');
  assert.equal(shouldShowCardLyrics(cfg, plain, true, true), true);
  assert.equal(shouldShowCardLyrics(cfg, synced, true, false), false, 'drawer toggle hides card lyrics');
  assert.equal(shouldShowPillLyrics({ ...cfg, enabled: false }, synced, true), false);
});
