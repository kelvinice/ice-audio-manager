import assert from 'node:assert/strict';
import test from 'node:test';
import {
  ALL_WIDGET_BUTTONS,
  MAX_WIDGET_BUTTONS,
  WIDGET_BUTTONS,
  addWidgetButton,
  pandoraTitle,
  railFitsServices,
  visibleWidgetButtons,
} from './widget.ts';

test('the widget draws nothing while it is off', () => {
  assert.deepEqual(visibleWidgetButtons({ enabled: false, buttons: ['pin', 'mic'] }, true), []);
  assert.deepEqual(visibleWidgetButtons(undefined, true), []);
});

test('the widget keeps the user order and drops lyrics when the card has none', () => {
  const widget = { enabled: true, buttons: ['lyrics', 'pandora_recognize', 'pin'] } as const;
  assert.deepEqual(visibleWidgetButtons({ ...widget, buttons: [...widget.buttons] }, true), [
    'lyrics',
    'pandora_recognize',
    'pin',
  ]);
  assert.deepEqual(visibleWidgetButtons({ ...widget, buttons: [...widget.buttons] }, false), ['pandora_recognize', 'pin']);
});

test('adding a button never repeats it or passes the limit (twin of ice_core::widget::validate)', () => {
  assert.deepEqual(addWidgetButton(['pin'], 'pin'), ['pin']);
  const full = ALL_WIDGET_BUTTONS.slice(0, MAX_WIDGET_BUTTONS);
  assert.equal(addWidgetButton(full, 'settings'), full);
  assert.deepEqual(addWidgetButton(['pin'], 'call_mode'), ['pin', 'call_mode']);
});

test('music services join the rail only while five buttons are not exceeded', () => {
  assert.equal(railFitsServices(3, 2), true);
  assert.equal(railFitsServices(4, 2), false);
});

test('every button has a label, and Pandora state shows in the tooltip', () => {
  for (const b of ALL_WIDGET_BUTTONS) assert.ok(WIDGET_BUTTONS[b].label.length > 0);
  assert.equal(pandoraTitle('pandora_recognize', { busy: 'recognize', lastResult: null }), 'Pandora is listening…');
  assert.match(pandoraTitle('pandora_recognize', { busy: null, lastResult: 'Song — Artist' }), /Song — Artist/);
  assert.equal(pandoraTitle('pandora_open', { busy: null, lastResult: 'Song — Artist' }), 'Open Pandora');
});
