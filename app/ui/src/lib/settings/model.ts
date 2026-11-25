// Pure logic for the Settings window (Phase 4, F19–F23). No Tauri, no DOM: unit-tested in model.test.ts.
import type {
  DevicePrefsView,
  DeviceSummary,
  EngineSnapshot,
  Envelope,
  Flow,
  SavedDevice,
  SettingsView,
} from '../ipc/engine.ts';

export type EnvelopeOutcome = { snapshot: EngineSnapshot; resnapshot: boolean };

/**
 * Applies one engine delta (D-03): older revisions are ignored, a gap asks for a fresh snapshot, and only the fields
 * Settings shows are copied. Events that only describe an activity (switch, plug) just advance the revision.
 */
export function applyEnvelope(snapshot: EngineSnapshot, envelope: Envelope): EnvelopeOutcome {
  if (envelope.revision <= snapshot.revision) return { snapshot, resnapshot: false };
  if (envelope.revision > snapshot.revision + 1) return { snapshot, resnapshot: true };
  const next: EngineSnapshot = { ...snapshot, revision: envelope.revision };
  const event = envelope.event;
  switch (event.type) {
    case 'devicesChanged':
      next.playback = event.playback;
      next.recording = event.recording;
      next.disconnected = event.disconnected;
      break;
    case 'volumeChanged':
      next.volume = event.volume;
      break;
    case 'micMuteChanged':
      next.micMuted = event.muted;
      break;
    case 'micVolumeChanged':
      next.micVolume = event.percent;
      break;
    case 'mediaChanged':
      next.media = event.media;
      break;
    case 'lyricsChanged':
      next.lyrics = event.lyrics;
      break;
    case 'noticesChanged':
      next.notices = event.notices;
      break;
    case 'settingsChanged':
      next.settings = event.settings;
      break;
    default:
      break;
  }
  return { snapshot: next, resnapshot: false };
}

function sameId(a: string | null | undefined, b: string | null | undefined): boolean {
  return Boolean(a && b && a.toLowerCase() === b.toLowerCase());
}

export function prefsFor(settings: SettingsView | null, deviceId: string): DevicePrefsView | null {
  return settings?.devices.find((p) => sameId(p.device.deviceId, deviceId)) ?? null;
}

/** The user's alias, else Windows' name. */
export function deviceLabel(device: Pick<DeviceSummary, 'id' | 'name'>, settings: SettingsView | null): string {
  return prefsFor(settings, device.id)?.alias || device.name;
}

/** A saved device (pair, shortcut, favourite) by its alias, else the name it was saved under. Works while unplugged. */
export function savedLabel(saved: SavedDevice, settings: SettingsView | null): string {
  return settings?.devices.find((p) => sameId(p.device.savedId, saved.savedId))?.alias || saved.name;
}

/** Hidden devices are left out of the snapshot's device lists; Settings lists them from the saved preferences. */
export function hiddenDevices(settings: SettingsView | null, flow: Flow): DevicePrefsView[] {
  return settings?.devices.filter((p) => p.hidden && p.device.flow === flow) ?? [];
}

/** `items` with the entry at `index` moved by `delta`, or `null` if it can't move. */
export function moveItem<T>(items: readonly T[], index: number, delta: -1 | 1): T[] | null {
  const to = index + delta;
  if (index < 0 || index >= items.length || to < 0 || to >= items.length) return null;
  const next = [...items];
  const [item] = next.splice(index, 1);
  next.splice(to, 0, item as T);
  return next;
}

/** A new id like `pair-2` that isn't in `taken` (config ids only need to be unique and readable). */
export function nextId(prefix: string, taken: readonly string[]): string {
  for (let n = taken.length + 1; ; n += 1) {
    const id = `${prefix}-${n}`;
    if (!taken.includes(id)) return id;
  }
}

// ---- Hotkey recorder ----

export type KeyInput = Pick<KeyboardEvent, 'code' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>;

/** What the recorder shows while keys are held, and the finished chord once a non-modifier key is pressed. */
export type RecordedKeys =
  | { kind: 'partial'; parts: string[] }
  | { kind: 'chord'; chord: string }
  | { kind: 'invalid'; parts: string[]; reason: string };

const MODIFIER_CODES = new Set([
  'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight', 'MetaLeft', 'MetaRight',
]);

/** Browser `KeyboardEvent.code` → the key names `ice_core::hotkey::Chord` parses. */
const NAMED_CODES: Record<string, string> = {
  Space: 'Space', PageUp: 'PageUp', PageDown: 'PageDown', End: 'End', Home: 'Home',
  ArrowLeft: 'Left', ArrowUp: 'Up', ArrowRight: 'Right', ArrowDown: 'Down',
  Insert: 'Insert', Delete: 'Delete', Pause: 'Pause', ScrollLock: 'ScrollLock',
  AudioVolumeMute: 'VolumeMute', AudioVolumeDown: 'VolumeDown', AudioVolumeUp: 'VolumeUp',
  MediaTrackNext: 'MediaNext', MediaTrackPrevious: 'MediaPrev', MediaStop: 'MediaStop', MediaPlayPause: 'MediaPlayPause',
};

function keyName(code: string): string | null {
  if (code in NAMED_CODES) return NAMED_CODES[code] ?? null;
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) return letter[1] ?? null;
  const digit = /^(?:Digit|Numpad)([0-9])$/.exec(code);
  if (digit) return digit[1] ?? null;
  const fn = /^F([1-9]|1[0-9]|2[0-4])$/.exec(code);
  if (fn) return code;
  return null;
}

/** The modifiers still held, in chord order. */
export function modifierParts(e: Omit<KeyInput, 'code'>): string[] {
  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  if (e.metaKey) parts.push('Win');
  return parts;
}

/**
 * Turns a keydown into a chord string in the engine's format (`Ctrl+Alt+F12`). The engine stays the authority (it
 * rejects bad chords with `SettingRejected`); this only stops shortcuts that would swallow ordinary typing.
 */
export function recordKeys(e: KeyInput): RecordedKeys {
  const mods = modifierParts(e);
  if (MODIFIER_CODES.has(e.code)) return { kind: 'partial', parts: mods };
  const key = keyName(e.code);
  if (!key) return { kind: 'invalid', parts: mods, reason: 'That key can’t be used in a shortcut.' };
  const typingKey = key.length === 1 || key === 'Space' || ['Left', 'Up', 'Right', 'Down'].includes(key);
  const onlyShift = mods.length === 1 && mods[0] === 'Shift';
  if (typingKey && (mods.length === 0 || onlyShift)) {
    return { kind: 'invalid', parts: [...mods, key], reason: 'Add Ctrl, Alt or Win so it doesn’t get in the way of typing.' };
  }
  return { kind: 'chord', chord: [...mods, key].join('+') };
}

/** `"Ctrl+Alt+F12"` → `["Ctrl", "Alt", "F12"]` for key caps. */
export function chordParts(chord: string | null | undefined): string[] {
  return chord ? chord.split('+').map((p) => p.trim()).filter(Boolean) : [];
}

/** Choices for a playback device's lyrics delay (Settings → Devices). `'auto'` = the engine decides (Bluetooth 200 ms). */
export const LYRICS_DELAY_STEPS_MS = [-200, -100, 0, 100, 150, 200, 250, 300, 400, 500, 750, 1000] as const;

/**
 * The lyrics-delay select: "Auto (N ms)" while nothing is saved (N = the device's current automatic value), then the
 * steps. A value saved by hand that isn't a step stays visible, like the spatial select.
 */
export function lyricsDelayChoice(
  savedMs: number | null | undefined,
  effectiveMs: number,
): { value: string; options: { value: string; label: string }[] } {
  const ms = (n: number) => (n > 0 ? `+${n} ms` : `${n} ms`);
  const auto = savedMs === null || savedMs === undefined;
  const steps: number[] = [...LYRICS_DELAY_STEPS_MS];
  if (!auto && !steps.includes(savedMs)) steps.push(savedMs);
  steps.sort((a, b) => a - b);
  return {
    value: auto ? 'auto' : String(savedMs),
    options: [
      { value: 'auto', label: auto ? `Auto (${ms(effectiveMs)})` : 'Auto' },
      ...steps.map((n) => ({ value: String(n), label: n === 0 ? 'None (0 ms)' : ms(n) })),
    ],
  };
}

/** The `SetLyricsDelay` value for a select choice: `null` = automatic. */
export function lyricsDelayFromChoice(choice: string): number | null {
  if (choice === 'auto') return null;
  const n = Number(choice);
  return Number.isFinite(n) ? Math.round(n) : null;
}
