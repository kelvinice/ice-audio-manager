// Dev-only stand-in for the shell, so `npm run dev` can show Settings in a browser for visual QA (code-practices §4:
// light/dark, 100 %/150 %). Never bundled: main.ts imports it only when `import.meta.env.DEV` and Tauri is absent.
// The real rules live in `ice_core::config::Config::apply_setting`; this only mirrors enough to see the UI react.
import type { Command, DeviceSummary, EngineSnapshot, SavedDevice } from '../lib/ipc/engine.ts';
import type { NotchSnapshot } from '../lib/ipc/types.ts';
import type { Backend } from '../lib/settings/client.svelte.ts';

const device = (id: string, name: string, flow: 'render' | 'capture', extra: Partial<DeviceSummary> = {}): DeviceSummary => ({
  id,
  name,
  flow,
  state: 'active',
  isDefault: false,
  isDefaultCommunications: false,
  formFactor: flow === 'render' ? 1 : 4,
  isVirtual: false,
  spatial: null,
  ...extra,
});

const saved = (d: DeviceSummary): SavedDevice => ({ name: d.name, flow: d.flow, savedId: d.id, deviceId: d.id });

const arctis = device('dev-arctis', 'Headphones (Arctis Nova 7)', 'render', { formFactor: 3, isDefault: true, spatial: 'dolbyAtmosHeadphones' });
const speakers = device('dev-realtek', 'Speakers (Realtek(R) Audio)', 'render');
const lg = device('dev-lg', 'LG FULL HD (NVIDIA High Definition Audio)', 'render', { formFactor: 9 });
const steam = device('dev-steam', 'Speakers (Steam Streaming Speakers)', 'render', { isVirtual: true });
const arctisMic = device('dev-arctis-mic', 'Microphone (Arctis Nova 7)', 'capture', { isDefault: true, isDefaultCommunications: true });
const ugreen = device('dev-ugreen', 'Microphone (UGREEN CM564)', 'capture');

function initialSnapshot(): EngineSnapshot {
  return {
    revision: 1,
    playback: [arctis, speakers, lg, steam],
    recording: [arctisMic, ugreen],
    disconnected: [],
    volume: { percent: 42, muted: false },
    micMuted: false,
    micVolume: 80,
    notices: [],
    media: null,
    settings: {
      appearance: { theme: 'ice', preset: 'smooth', idleStyle: 'mini_pill', nowPlaying: 'brief', movingBackdrop: true },
      performanceProfile: 'instant',
      pinnedMonitor: 'DISPLAY-PATH-1',
      defaultRoles: 'default',
      togglePairs: [{ id: 'main', hotkey: 'Ctrl+Alt+F12', primary: saved(speakers), secondary: saved(arctis), roles: null }],
      bindings: [{ id: 'binding-1', hotkey: 'Ctrl+Alt+F11', device: saved(lg), roles: null }],
      favouritesPlayback: [saved(arctis), saved(speakers)],
      favouritesRecording: [],
      devices: [
        { device: saved(arctis), spatial: 'dolbyAtmosHeadphones', alias: 'Arctis', hidden: false },
        {
          device: { name: 'Digital Output (Realtek(R) Audio)', flow: 'render', savedId: 'dev-spdif', deviceId: 'dev-spdif' },
          spatial: 'keep',
          alias: null,
          hidden: true,
        },
      ],
      knobGestures: true,
      showDisconnected: false,
      idleLingerSecs: 15,
      musicServices: { spotify: true, youtubeMusic: true },
      showNotchHotkey: null,
    },
  };
}

const notch: NotchSnapshot = {
  revision: 1,
  eventSeq: 0,
  activity: 'idle',
  volumePercent: 42,
  muted: false,
  micMuted: false,
  activeDevice: arctis.name,
  targetDevice: speakers.name,
  spatialBadge: 'Atmos',
  preset: 'smooth',
  systemReduceMotion: false,
  theme: 'ice',
  idleStyle: 'mini_pill',
  expanded: false,
  suspended: false,
  hiddenForBench: false,
  activeMonitor: 0,
  monitors: [
    { index: 0, id: '\\\\.\\DISPLAY1', displayPath: 'DISPLAY-PATH-1', name: 'LG ULTRAGEAR', isPrimary: true, left: 0, top: 0, width: 2560, height: 1440, scaleFactor: 1.5, refreshHz: 144 },
    { index: 1, id: '\\\\.\\DISPLAY2', displayPath: 'DISPLAY-PATH-2', name: 'ASUS VP249', isPrimary: false, left: 2560, top: 220, width: 1920, height: 1080, scaleFactor: 1, refreshHz: 75 },
  ],
};

function applyCommand(s: EngineSnapshot, c: Command): void {
  const st = s.settings;
  const all = [...s.playback, ...s.recording, ...st.devices.map((p) => ({ ...speakers, id: p.device.savedId, name: p.device.name }))];
  const ref = (id: string): SavedDevice => saved(all.find((d) => d.id === id) ?? speakers);
  switch (c.type) {
    case 'setTheme': st.appearance.theme = c.theme; break;
    case 'setPreset': st.appearance.preset = c.preset; break;
    case 'setIdleStyle': st.appearance.idleStyle = c.style; break;
    case 'setNowPlayingPill': st.appearance.nowPlaying = c.mode; break;
    case 'setMovingBackdrop': st.appearance.movingBackdrop = c.enabled; break;
    case 'setPerformanceProfile': st.performanceProfile = c.profile; break;
    case 'setPinnedMonitor': st.pinnedMonitor = c.displayPath; break;
    case 'setKnobGestures': st.knobGestures = c.enabled; break;
    case 'setShowDisconnected': st.showDisconnected = c.enabled; break;
    case 'setIdleLinger': st.idleLingerSecs = c.secs; break;
    case 'setMusicService': st.musicServices = { ...st.musicServices, [c.service === 'spotify' ? 'spotify' : 'youtubeMusic']: c.enabled }; break;
    case 'setShowNotchHotkey': st.showNotchHotkey = c.hotkey; break;
    case 'setDefaultRoles': st.defaultRoles = c.roles; break;
    case 'setFavourites': st.favouritesPlayback = c.deviceIds.map(ref); break;
    case 'setDevicePrefs': {
      let p = st.devices.find((d) => d.device.deviceId === c.deviceId);
      if (!p) st.devices.push((p = { device: ref(c.deviceId), spatial: 'keep', alias: null, hidden: false }));
      if (c.spatial !== null) p.spatial = c.spatial;
      if (c.alias !== null) p.alias = c.alias.trim() || null;
      if (c.hidden !== null) p.hidden = c.hidden;
      break;
    }
    case 'setTogglePair': {
      const pair = { id: c.pairId, hotkey: c.hotkey, primary: ref(c.primaryId), secondary: ref(c.secondaryId), roles: c.roles };
      st.togglePairs = [...st.togglePairs.filter((p) => p.id !== c.pairId), pair];
      break;
    }
    case 'removeTogglePair': st.togglePairs = st.togglePairs.filter((p) => p.id !== c.pairId); break;
    case 'setBinding': {
      const binding = { id: c.bindingId, hotkey: c.hotkey, device: ref(c.deviceId), roles: c.roles };
      st.bindings = [...st.bindings.filter((b) => b.id !== c.bindingId), binding];
      break;
    }
    case 'removeBinding': st.bindings = st.bindings.filter((b) => b.id !== c.bindingId); break;
    default: break;
  }
}

export function createDevBackend(): Backend {
  const snapshot = initialSnapshot();
  const listeners = new Map<string, Set<(payload: unknown) => void>>();
  let launch: boolean | null = true;
  const emit = (event: string, payload: unknown) => listeners.get(event)?.forEach((h) => h(payload));
  return {
    async invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
      switch (command) {
        case 'get_engine_snapshot': return structuredClone(snapshot) as T;
        case 'get_notch_snapshot': return structuredClone(notch) as T;
        case 'get_launch_at_sign_in': return launch as T;
        case 'plugin:app|version': return '0.0.1 (preview)' as T;
        case 'set_launch_at_sign_in': launch = Boolean(args.enabled); return launch as T;
        case 'apply_setting': {
          const c = args.command as Command;
          if (c.type === 'setBinding' && c.hotkey === 'Ctrl+Alt+F12') {
            snapshot.revision += 1;
            emit('engine://delta/v1', { revision: snapshot.revision, event: { type: 'notice', notice: { type: 'settingRejected', error: 'Ctrl+Alt+F12 is already used by toggle pair "main"' } } });
            return undefined as T;
          }
          applyCommand(snapshot, c);
          snapshot.revision += 1;
          emit('engine://delta/v1', { revision: snapshot.revision, event: { type: 'settingsChanged', settings: structuredClone(snapshot.settings) } });
          return undefined as T;
        }
        default: return undefined as T;
      }
    },
    async listen<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
      const set = listeners.get(event) ?? new Set();
      listeners.set(event, set);
      const h = handler as (payload: unknown) => void;
      set.add(h);
      return () => set.delete(h);
    },
  };
}
