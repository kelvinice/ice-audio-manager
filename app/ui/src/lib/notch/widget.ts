// Experimental notch widget (owner, 2026-11-27): the user's own quick buttons, in their order, in place of the fixed
// side-rail (Floating) and toolbar-capsule (Edge Nook) buttons. Rust owns the list (`ice_core::widget`); this file only
// labels the buttons and decides which ones can show right now.
import type { NotchWidget, WidgetButton } from '../ipc/engine';
import type { PandoraAction, PandoraView } from '../ipc/types';

/** Everything the widget's buttons draw and do; built once by the controller (`NotchController.widget`). */
export interface WidgetBinding {
  buttons: WidgetButton[];
  pinned: boolean;
  micMuted: boolean;
  lyricsOn: boolean;
  lyricsAvailable: boolean;
  callModeOn: boolean;
  pandora: PandoraView | undefined;
  /** Preset `off` / reduced motion: no busy pulse. */
  instant?: boolean;
  onTogglePin: () => void;
  onToggleMicMute: () => void;
  onToggleLyrics: () => void;
  onHibernate: () => void;
  onToggleCallMode: () => void;
  onPandora: (action: PandoraAction) => void;
  onOpenSettings: () => void;
}

// Twin: ice_core::widget::MAX_BUTTONS.
export const MAX_WIDGET_BUTTONS = 5;

export type WidgetIcon = 'pin' | 'mic' | 'lyrics' | 'moon' | 'call' | 'recognize' | 'pandora' | 'settings';

export interface WidgetButtonInfo {
  label: string;
  icon: WidgetIcon;
  /** One line for Settings. */
  description: string;
}

export const WIDGET_BUTTONS: Record<WidgetButton, WidgetButtonInfo> = {
  pin: { label: 'Pin', icon: 'pin', description: 'Keep the card open.' },
  mic: { label: 'Microphone', icon: 'mic', description: 'Mute or unmute your microphone.' },
  lyrics: { label: 'Lyrics', icon: 'lyrics', description: 'Show or hide lyrics (only while a song has them).' },
  hibernate: { label: 'Hibernate', icon: 'moon', description: 'A plain notch for meetings and screen sharing.' },
  call_mode: { label: 'Call mode', icon: 'call', description: 'Lower the music while people talk on a call.' },
  pandora_recognize: {
    label: 'Recognise song',
    icon: 'recognize',
    description: 'Pandora listens to what this PC plays and names the song.',
  },
  pandora_open: { label: 'Open Pandora', icon: 'pandora', description: 'Bring up the Pandora player.' },
  settings: { label: 'Settings', icon: 'settings', description: 'Open Ice’s settings.' },
};

/** Every button, in the order Settings offers them. Twin: `WidgetButton::ALL`. */
export const ALL_WIDGET_BUTTONS: WidgetButton[] = [
  'pin',
  'mic',
  'lyrics',
  'hibernate',
  'call_mode',
  'pandora_recognize',
  'pandora_open',
  'settings',
];

/** The buttons to draw: the user's order, minus the lyrics button while the card offers no lyrics. */
export function visibleWidgetButtons(widget: NotchWidget | undefined, lyricsToggle: boolean): WidgetButton[] {
  if (!widget?.enabled) return [];
  return widget.buttons.filter((b) => b !== 'lyrics' || lyricsToggle).slice(0, MAX_WIDGET_BUTTONS);
}

/** Adds `button` at the end, unless it is already there or the widget is full. */
export function addWidgetButton(list: WidgetButton[], button: WidgetButton): WidgetButton[] {
  if (list.includes(button) || list.length >= MAX_WIDGET_BUTTONS) return list;
  return [...list, button];
}

/** Tooltip for a Pandora button: busy, last answer, or what it does. */
export function pandoraTitle(button: 'pandora_recognize' | 'pandora_open', view: PandoraView | undefined): string {
  const action = button === 'pandora_recognize' ? 'recognize' : 'open';
  if (view?.busy === action) return action === 'recognize' ? 'Pandora is listening…' : 'Opening Pandora…';
  const base = WIDGET_BUTTONS[button].label;
  return action === 'recognize' && view?.lastResult ? `${base} · last: ${view.lastResult}` : base;
}
