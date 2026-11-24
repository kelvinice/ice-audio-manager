import assert from 'node:assert/strict';
import test from 'node:test';
import type { LyricsConfig, LyricsDocument, LyricsState } from '../ipc/engine.ts';
import {
  hasSubtitleLines,
  ladderY,
  lineSubtitle,
  lyricRows,
  resolveLyricFrame,
  romajiLitCount,
  shouldShowCardLyrics,
  shouldShowPillLyrics,
} from './lyrics.ts';

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
      romaji: null,
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
      subtitle: null,
      romaji: 'muteki no egao',
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
  // The pill's first row previews line 1 during the intro, so its second row shows line 2.
  assert.equal(intro.followingLine?.text, '無敵の笑顔');
  assert.equal(intro.nextWakeDelayMs, 2_000);

  const midFirst = resolveLyricFrame(sampleDoc, 6_500, true);
  assert.equal(midFirst.activeIndex, 0);
  assert.equal(midFirst.activeWordIndex, 1);
  assert.equal(midFirst.prevLine, null);
  assert.equal(midFirst.nextLine?.text, '無敵の笑顔');
  assert.equal(midFirst.followingLine, null, 'nothing after the last line');
  assert.equal(midFirst.nextWakeDelayMs, 1_015);

  const second = resolveLyricFrame(sampleDoc, 11_200, true);
  assert.equal(second.activeIndex, 1);
  assert.equal(second.activeWordIndex, 1);
  assert.equal(second.prevLine?.text, 'Hello world today');
  assert.equal(second.activeLine?.romaji, 'muteki no egao');
  assert.equal(second.beforePrevLine, null);
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

test('lineSubtitle: the source\'s paired line always shows; generated Romaji only while the setting is on', () => {
  const paired = { ...sampleDoc.lines[0], subtitle: 'Invincible smile', romaji: null };
  const generated = sampleDoc.lines[1];
  assert.deepEqual(lineSubtitle(paired, false), { text: 'Invincible smile', generatedRomaji: false });
  assert.deepEqual(lineSubtitle(generated, true), { text: 'muteki no egao', generatedRomaji: true });
  assert.equal(lineSubtitle(generated, false), null);
  assert.equal(hasSubtitleLines(sampleDoc, false), false);
  assert.equal(hasSubtitleLines(sampleDoc, true), true);
});

test('romajiLitCount follows the sung share of the line', () => {
  assert.equal(romajiLitCount(3, -1, 2), 0, 'nothing sung yet');
  assert.equal(romajiLitCount(3, 0, 2), 2, 'half the words → about half the Romaji');
  assert.equal(romajiLitCount(3, 1, 2), 3);
  assert.equal(romajiLitCount(3, -1, 0), 3, 'no word timings: all lit');
});

test('lyricRows keeps a line on the same key as it moves up, so the change slides instead of swapping', () => {
  const first = lyricRows(resolveLyricFrame(sampleDoc, 6_500, true), 'pill');
  const second = lyricRows(resolveLyricFrame(sampleDoc, 11_200, true), 'pill');
  const line2 = String(sampleDoc.lines[1].startMs);
  assert.equal(first.find((r) => r.key === line2)?.slot, 1, 'line 2 waits in the next row');
  assert.equal(second.find((r) => r.key === line2)?.slot, 0, 'then moves into the main row');
  assert.equal(second.find((r) => r.key === String(sampleDoc.lines[0].startMs))?.slot, -1, 'line 1 slides out above');

  const intro = lyricRows(resolveLyricFrame(sampleDoc, 2_000, true), 'pill');
  assert.deepEqual(intro.map((r) => [r.slot, r.role]), [[0, 'upcoming'], [1, 'next']]);

  const card = lyricRows(resolveLyricFrame(sampleDoc, 11_200, true), 'card');
  assert.deepEqual(card.map((r) => r.slot), [-1, 0], 'previous above the current line; nothing after the last');
});

test('ladderY stacks slots top to bottom and reserves the subtitle row', () => {
  for (const mode of ['pill', 'card'] as const) {
    for (const hasSub of [false, true]) {
      const ys = [-2, -1, 0, 1].map((s) => ladderY(mode, s, hasSub));
      assert.ok(ys.every((y, i) => i === 0 || y > ys[i - 1]), `${mode} ${hasSub}: ${ys}`);
    }
  }
  assert.equal(ladderY('pill', 0, false), 0);
  assert.equal(ladderY('pill', 1, true) - ladderY('pill', 1, false), 14, 'room for the subtitle under the main row');
  assert.ok(ladderY('card', 1, true) + 15 <= 68, 'the card fits its drawer with a subtitle');
});
