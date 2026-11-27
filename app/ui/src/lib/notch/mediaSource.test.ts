import assert from 'node:assert/strict';
import test from 'node:test';
import { mediaSource } from './mediaSource.ts';

test('known players get a friendly name and icon', () => {
  assert.deepEqual(mediaSource('Spotify.exe'), { label: 'Spotify', icon: 'spotify' });
  assert.deepEqual(mediaSource('pandora.exe'), { label: 'Pandora', icon: 'pandora' });
  assert.equal(mediaSource('music.youtube.com-Default').label, 'YouTube Music');
  assert.equal(mediaSource('YouTube Music').icon, 'youtube-music');
  assert.equal(mediaSource('msedge.exe').label, 'Edge');
  assert.equal(mediaSource('Chrome').label, 'Chrome');
  assert.equal(mediaSource('Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic').label, 'Media Player');
});

test('unknown players fall back to their exe name, then to "Media"', () => {
  assert.equal(mediaSource('C:\\Program Files\\foobar2000\\foobar2000.exe').label, 'foobar2000');
  assert.equal(mediaSource('').label, 'Media');
  assert.equal(mediaSource(null).label, 'Media');
  assert.equal(mediaSource('x'.repeat(40)).label, 'Media', 'very long ids are not shown');
});
