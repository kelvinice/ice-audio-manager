import { loadSession, saveSession } from "./session";
import { writable, get } from "svelte/store";
import {
  fetchLyrics,
  recordHistoryTrack,
  saveEqState,
  ytmRelated,
} from "./ipc";
import { MagicShuffle } from "./magicShuffle";
import { mapParametricFilterType } from "./parametricEq";
import { applyDefaultMaterialTheme, applyMaterialThemeFromArt, resolveArtworkUrl, setArtworkAccents } from "./theme";
import {
  DEFAULT_PANDORA_SETTINGS,
  type EqState,
  type PandoraLyrics,
  type PandoraSettings,
  type Track,
} from "./types";

export const EQ_FREQUENCIES = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];

export const EQ_PRESETS: Record<string, number[]> = {
  Flat: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  "Bass Boost": [6, 5, 4, 2, 0, 0, 0, 1, 2, 2],
  Vocal: [-2, -1, 0, 2, 4, 5, 4, 2, 0, -1],
  Electronic: [5, 4, 2, 0, -2, 0, 2, 4, 5, 5],
  Acoustic: [2, 2, 1, 0, 1, 2, 3, 3, 2, 1],
  "Late Night": [-4, -2, 0, 2, 2, 2, 1, 0, -2, -3],
  "V-Shape": [5, 4, 2, -2, -4, -3, 1, 3, 5, 6],
};

export type ShuffleMode = "off" | "on" | "magic";

/** Shuffle button labels: the current mode and what a click does. */
export const SHUFFLE_TITLES: Record<ShuffleMode, string> = {
  off: "Shuffle: off (click for shuffle)",
  on: "Shuffle: on (click for Magic shuffle)",
  magic: "Magic shuffle: recommended songs play in between (click to turn off)",
};

export interface PlayerState {
  queue: Track[];
  currentIndex: number;
  currentTrack: Track | null;
  playing: boolean;
  loading: boolean;
  currentTimeMs: number;
  durationMs: number;
  volume: number;
  muted: boolean;
  /** `magic` shuffles and plays a recommended song every few songs (`lib/magicShuffle.ts`). */
  shuffle: ShuffleMode;
  /** Songs Magic shuffle put into the queue (marked in the queue panel). */
  magicIds: string[];
  repeat: "off" | "all" | "one";
  eq: EqState;
  lyrics: PandoraLyrics | null;
  sleepRemainingSec: number | null;
  sleepEndOfTrack: boolean;
  error: string | null;
}

const initialState: PlayerState = {
  queue: [],
  currentIndex: -1,
  currentTrack: null,
  playing: false,
  loading: false,
  currentTimeMs: 0,
  durationMs: 0,
  volume: 0.85,
  muted: false,
  shuffle: "off",
  magicIds: [],
  repeat: "off",
  eq: {
    enabled: true,
    preset: "Flat",
    bands: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    normalize: false,
    skipSilence: false,
    playbackRate: 1,
    parametric: null,
  },
  lyrics: null,
  sleepRemainingSec: null,
  sleepEndOfTrack: false,
  error: null,
};

export const playerStore = writable<PlayerState>(initialState);

/** How often the listening session is saved while playing (also on pause and when the window closes). */
const SESSION_SAVE_MS = 5_000;

class PandoraAudioEngine {
  private audio: HTMLAudioElement | null = null;
  /** The restored song and where to pick it up; applied by the next `playTrack` of that song. */
  private resume: { trackId: string; ms: number } | null = null;
  private lastSessionSaveMs = 0;
  private ctx: AudioContext | null = null;
  private sourceNode: MediaElementAudioSourceNode | null = null;
  private filters: BiquadFilterNode[] = [];
  private preampNode: GainNode | null = null;
  private parametricNodes: BiquadFilterNode[] = [];
  private compressor: DynamicsCompressorNode | null = null;
  private analyser: AnalyserNode | null = null;
  private gainNode: GainNode | null = null;
  private freqBuf: Float32Array<ArrayBuffer> | null = null;
  private timeBuf: Uint8Array<ArrayBuffer> | null = null;
  private proxyPort = 0;
  private silenceSinceMs: number | null = null;
  private lastSilenceJumpAt = 0;
  private sleepTimerId: number | null = null;
  private onHistoryRecorded: (() => void) | null = null;
  private settings: PandoraSettings = { ...DEFAULT_PANDORA_SETTINGS };
  /** The list being played (its songs without Magic shuffle's additions), to notice when a new list starts. */
  private sourceKey = "";
  /** Songs already played in this shuffle pass, so shuffle plays every song once before repeating. */
  private shufflePlayed = new Set<string>();
  private magic = new MagicShuffle();

  init(
    proxyPort: number,
    initialEq: EqState,
    initialSettings: PandoraSettings,
    onHistory: () => void,
  ) {
    this.proxyPort = proxyPort;
    this.onHistoryRecorded = onHistory;
    this.settings = { ...DEFAULT_PANDORA_SETTINGS, ...initialSettings };
    const initialVol = Math.max(0, Math.min(1, this.settings.defaultVolume ?? 0.85));
    playerStore.update((s) => ({ ...s, eq: initialEq, volume: initialVol }));
    if (this.gainNode) {
      this.gainNode.gain.value = get(playerStore).muted ? 0 : initialVol;
    }
    if (this.audio) return;

    const el = new Audio();
    el.crossOrigin = "anonymous";
    el.preload = "auto";
    el.volume = 1;

    el.addEventListener("timeupdate", () => {
      const s = get(playerStore);
      const curMs = Math.floor(el.currentTime * 1000);
      const durMs = Number.isFinite(el.duration) && el.duration > 0 ? Math.floor(el.duration * 1000) : s.durationMs;
      playerStore.update((st) => ({ ...st, currentTimeMs: curMs, durationMs: durMs }));
      this.syncMediaSessionPosition(curMs, durMs);
      this.checkSkipSilence();
      if (performance.now() - this.lastSessionSaveMs > SESSION_SAVE_MS) this.persistSession();
    });

    el.addEventListener("loadedmetadata", () => {
      if (Number.isFinite(el.duration) && el.duration > 0) {
        playerStore.update((st) => ({ ...st, durationMs: Math.floor(el.duration * 1000), loading: false }));
      }
    });

    el.addEventListener("canplay", () => {
      playerStore.update((st) => ({ ...st, loading: false }));
    });

    el.addEventListener("play", () => {
      playerStore.update((st) => ({ ...st, playing: true, error: null }));
      if (this.settings.mediaSessionEnabled && "mediaSession" in navigator) {
        navigator.mediaSession.playbackState = "playing";
      }
    });

    el.addEventListener("pause", () => {
      playerStore.update((st) => ({ ...st, playing: false }));
      this.persistSession();
      if (this.settings.mediaSessionEnabled && "mediaSession" in navigator) {
        navigator.mediaSession.playbackState = "paused";
      }
    });

    el.addEventListener("ended", () => {
      void this.handleEnded();
    });

    el.addEventListener("error", () => {
      playerStore.update((st) => ({
        ...st,
        playing: false,
        loading: false,
        error: "Audio stream could not be decoded or reached. Try caching or switching track.",
      }));
    });

    this.audio = el;
    this.bindMediaSessionHandlers();
    window.addEventListener("pagehide", () => this.persistSession());
  }

  /**
   * At launch: shows the last session's song paused at its saved spot, with its queue, lyrics and colours, so the
   * player never opens empty. Nothing streams until Play. Skipped if something is already loaded.
   */
  restoreSession(lastPlayed: Track | null) {
    if (get(playerStore).currentTrack || this.audio?.src) return;
    const session = loadSession(lastPlayed);
    if (!session) return;
    const track = session.queue[session.index];
    this.resume = { trackId: track.id, ms: session.positionMs };
    playerStore.update((s) => ({
      ...s,
      queue: session.queue,
      currentIndex: session.index,
      currentTrack: track,
      currentTimeMs: session.positionMs,
      durationMs: track.durationMs ?? 0,
      playing: false,
      loading: false,
      error: null,
      lyrics: { trackKey: track.id, status: "loading", source: null, lines: [], plainText: null, error: null },
    }));
    void applyMaterialThemeFromArt(track.artworkUrl, this.proxyPort);
    void this.loadLyricsForTrack(track);
  }

  private persistSession() {
    this.lastSessionSaveMs = performance.now();
    const st = get(playerStore);
    if (st.currentTrack) saveSession(st.queue, st.currentIndex, st.currentTimeMs);
  }

  private ensureWebAudioGraph() {
    if (!this.audio || this.ctx) return;
    const ctx = new AudioContext();
    const source = ctx.createMediaElementSource(this.audio);

    const filters = EQ_FREQUENCIES.map((freq, idx) => {
      const f = ctx.createBiquadFilter();
      f.type = idx === 0 ? "lowshelf" : idx === EQ_FREQUENCIES.length - 1 ? "highshelf" : "peaking";
      f.frequency.value = freq;
      f.Q.value = 1.25;
      f.gain.value = 0;
      return f;
    });

    const compressor = ctx.createDynamicsCompressor();
    compressor.threshold.value = -24;
    compressor.knee.value = 24;
    compressor.ratio.value = 1; // bypassed unless normalize=true
    compressor.attack.value = 0.004;
    compressor.release.value = 0.25;

    const analyser = ctx.createAnalyser();
    // 4096 points gives ~12 Hz bins, so the narrow bass bars read different bins (getSpectrumDb interpolates the
    // rest); less frame smoothing than before keeps the longer window just as punchy.
    analyser.fftSize = 4096;
    analyser.smoothingTimeConstant = 0.6;

    const gain = ctx.createGain();
    const st = get(playerStore);
    gain.gain.value = st.muted ? 0 : st.volume;

    const preampNode = ctx.createGain();
    preampNode.gain.value = 1;

    let node: AudioNode = source;
    for (const f of filters) {
      node.connect(f);
      node = f;
    }
    node.connect(preampNode);
    preampNode.connect(compressor);
    compressor.connect(analyser);
    analyser.connect(gain);
    gain.connect(ctx.destination);

    this.ctx = ctx;
    this.sourceNode = source;
    this.filters = filters;
    this.preampNode = preampNode;
    this.compressor = compressor;
    this.analyser = analyser;
    this.gainNode = gain;
    this.freqBuf = new Float32Array(new ArrayBuffer(analyser.frequencyBinCount * 4));
    this.timeBuf = new Uint8Array(new ArrayBuffer(analyser.fftSize));
    this.applyEqToGraph(st.eq);
  }

  applySettings(next: PandoraSettings) {
    const prev = this.settings;
    this.settings = { ...DEFAULT_PANDORA_SETTINGS, ...next };
    const track = get(playerStore).currentTrack;
    setArtworkAccents(this.settings.artworkAccents);

    if (prev.mediaSessionEnabled !== this.settings.mediaSessionEnabled && "mediaSession" in navigator) {
      if (!this.settings.mediaSessionEnabled) {
        navigator.mediaSession.metadata = null;
        navigator.mediaSession.playbackState = "none";
      } else if (track) {
        this.updateMediaSessionMetadata(track);
        navigator.mediaSession.playbackState = get(playerStore).playing ? "playing" : "paused";
      }
    }
  }

  private applyEqToGraph(eq: EqState) {
    if (this.audio) {
      this.audio.playbackRate = eq.playbackRate;
    }
    this.filters.forEach((f, idx) => {
      f.gain.value = eq.enabled ? (eq.bands[idx] ?? 0) : 0;
    });
    if (this.ctx && this.preampNode && this.compressor) {
      this.preampNode.disconnect();
      for (const pn of this.parametricNodes) {
        pn.disconnect();
      }
      this.parametricNodes = [];

      const profile = eq.enabled ? eq.parametric : null;
      if (profile && profile.bands.length > 0) {
        this.preampNode.gain.value = Math.pow(10, (profile.preamp || 0) / 20);
        let tail: AudioNode = this.preampNode;
        for (const band of profile.bands) {
          const bq = this.ctx.createBiquadFilter();
          bq.type = mapParametricFilterType(band.filterType);
          bq.frequency.value = band.frequency;
          bq.gain.value = band.gain;
          bq.Q.value = band.q;
          tail.connect(bq);
          tail = bq;
          this.parametricNodes.push(bq);
        }
        tail.connect(this.compressor);
      } else {
        this.preampNode.gain.value = 1;
        this.preampNode.connect(this.compressor);
      }
    }
    if (this.compressor) {
      if (eq.normalize) {
        this.compressor.threshold.value = -20;
        this.compressor.ratio.value = 5;
      } else {
        this.compressor.threshold.value = 0;
        this.compressor.ratio.value = 1;
      }
    }
  }

  async playTrack(track: Track, newQueue?: Track[]) {
    if (!this.audio) return;
    this.ensureWebAudioGraph();
    if (this.ctx && this.ctx.state === "suspended") {
      await this.ctx.resume().catch(() => {});
    }

    const st = get(playerStore);
    let queue = newQueue ? [...newQueue] : [...st.queue];
    let idx = queue.findIndex((t) => t.id === track.id);
    if (idx === -1) {
      queue = [track, ...queue];
      idx = 0;
    }
    this.noteSource(queue);
    this.shufflePlayed.add(track.id);

    const src = this.resolvePlaybackUrl(track);
    if (!src) {
      playerStore.update((s) => ({ ...s, error: "Track has no playable source" }));
      return;
    }

    this.silenceSinceMs = null;
    const resumeMs = this.resume?.trackId === track.id ? this.resume.ms : 0;
    this.resume = null;
    playerStore.update((s) => ({
      ...s,
      queue,
      currentIndex: idx,
      currentTrack: track,
      loading: true,
      currentTimeMs: resumeMs,
      durationMs: track.durationMs ?? 0,
      error: null,
      lyrics: {
        trackKey: track.id,
        status: "loading",
        source: null,
        lines: [],
        plainText: null,
        error: null,
      },
    }));

    this.audio.src = src;
    // Before metadata loads this becomes the start position, so a restored song continues where it stopped.
    if (resumeMs > 0) this.audio.currentTime = resumeMs / 1000;
    this.persistSession();
    this.audio.playbackRate = get(playerStore).eq.playbackRate;
    void applyMaterialThemeFromArt(track.artworkUrl, this.proxyPort);
    if (this.settings.mediaSessionEnabled) {
      this.updateMediaSessionMetadata(track);
    }

    try {
      await this.audio.play();
    } catch (e) {
      playerStore.update((s) => ({
        ...s,
        playing: false,
        loading: false,
        error: e instanceof Error ? e.message : String(e),
      }));
    }

    // Background enrichment: history, lyrics, and Vivi-style Up Next radio queue
    void recordHistoryTrack(track).then(() => this.onHistoryRecorded?.()).catch(() => {});
    void this.loadLyricsForTrack(track);

    if (this.settings.autoQueueRelated && track.source === "ytmusic" && queue.length - idx <= 2) {
      void ytmRelated(track.id)
        .then((related) => {
          if (related.length === 0) return;
          playerStore.update((s) => {
            const existing = new Set(s.queue.map((q) => q.id));
            const additions = related.filter((r) => !existing.has(r.id));
            return { ...s, queue: [...s.queue, ...additions] };
          });
        })
        .catch(() => {});
    }
  }

  private resolvePlaybackUrl(track: Track): string | null {
    if (track.source === "demo" && track.demoDataUrl) {
      return track.demoDataUrl;
    }
    if (track.source === "local" && track.localPath && this.proxyPort > 0) {
      return `http://127.0.0.1:${this.proxyPort}/stream/local?path=${encodeURIComponent(track.localPath)}`;
    }
    if (track.source === "ytmusic" && this.proxyPort > 0) {
      return `http://127.0.0.1:${this.proxyPort}/stream/yt/${encodeURIComponent(track.id)}`;
    }
    return null;
  }

  private async loadLyricsForTrack(track: Track) {
    try {
      const res = await fetchLyrics(
        track.title,
        track.artist,
        track.album,
        track.durationMs,
        track.demoLrc ?? null,
      );
      if (get(playerStore).currentTrack?.id === track.id) {
        playerStore.update((s) => ({ ...s, lyrics: res }));
      }
    } catch {
      if (get(playerStore).currentTrack?.id === track.id) {
        playerStore.update((s) => ({
          ...s,
          lyrics: {
            trackKey: track.id,
            status: "not_found",
            source: null,
            lines: [],
            plainText: null,
            error: null,
          },
        }));
      }
    }
  }

  async togglePlayPause() {
    if (!this.audio) return;
    this.ensureWebAudioGraph();
    if (this.ctx && this.ctx.state === "suspended") {
      await this.ctx.resume().catch(() => {});
    }
    if (this.audio.paused) {
      if (!this.audio.src) {
        const st = get(playerStore);
        if (st.currentTrack) await this.playTrack(st.currentTrack);
        return;
      }
      await this.audio.play().catch(() => {});
    } else {
      this.audio.pause();
    }
  }

  seekMs(ms: number) {
    if (!this.audio) return;
    // A restored song that has not started yet: move where Play will pick it up.
    if (!this.audio.src && this.resume) this.resume.ms = Math.max(0, ms);
    this.audio.currentTime = Math.max(0, ms / 1000);
    playerStore.update((s) => ({ ...s, currentTimeMs: Math.max(0, ms) }));
  }

  /** A new list (not just this list with Magic shuffle's additions) restarts the shuffle pass and Magic shuffle. */
  private noteSource(queue: Track[]) {
    const base = queue.filter((t) => !this.magic.added.has(t.id));
    const key = base
      .map((t) => t.id)
      .sort()
      .join("|");
    if (key === this.sourceKey) return;
    this.sourceKey = key;
    this.shufflePlayed.clear();
    this.magic.reset(base);
    playerStore.update((s) => ({ ...s, magicIds: [] }));
    if (get(playerStore).shuffle === "magic") this.magic.prime();
  }

  /** A random song of the list not yet played in this pass; null when the pass is over and repeat is off. */
  private pickShuffled(st: PlayerState): number | null {
    const candidates = (pool: Set<string>) =>
      st.queue
        .map((t, i) => ({ t, i }))
        .filter(({ t, i }) => i !== st.currentIndex && !pool.has(t.id) && !this.magic.added.has(t.id))
        .map(({ i }) => i);
    let open = candidates(this.shufflePlayed);
    if (open.length === 0) {
      if (st.repeat !== "all") return null;
      this.shufflePlayed.clear();
      open = candidates(this.shufflePlayed);
      if (open.length === 0) return null;
    }
    return open[Math.floor(Math.random() * open.length)];
  }

  async next() {
    const st = get(playerStore);
    if (st.queue.length === 0) return;
    let nextIdx = st.currentIndex + 1;
    if (st.shuffle !== "off" && st.queue.length > 1) {
      const rec = st.shuffle === "magic" ? this.magic.takeTurn() : null;
      if (rec) {
        // Right after the current song, so the queue panel shows it as next.
        const current = st.queue[st.currentIndex];
        const queue = st.queue.filter((t) => t.id !== rec.id);
        queue.splice(current ? queue.indexOf(current) + 1 : 0, 0, rec);
        playerStore.update((s) => ({ ...s, magicIds: [...this.magic.added] }));
        await this.playTrack(rec, queue);
        return;
      }
      const pick = this.pickShuffled(st);
      if (pick === null) return;
      nextIdx = pick;
    } else if (nextIdx >= st.queue.length) {
      if (st.repeat === "all") {
        nextIdx = 0;
      } else {
        return;
      }
    }
    await this.playTrack(st.queue[nextIdx], st.queue);
  }

  async previous() {
    const st = get(playerStore);
    if (this.audio && this.audio.currentTime > 3) {
      this.seekMs(0);
      return;
    }
    if (st.queue.length === 0) return;
    const prevIdx = st.currentIndex > 0 ? st.currentIndex - 1 : st.queue.length - 1;
    await this.playTrack(st.queue[prevIdx], st.queue);
  }

  private async handleEnded() {
    const st = get(playerStore);
    if (st.sleepEndOfTrack) {
      playerStore.update((s) => ({ ...s, sleepEndOfTrack: false, playing: false }));
      if (this.audio) {
        this.audio.pause();
      }
      return;
    }
    if (st.repeat === "one" && this.audio) {
      this.seekMs(0);
      await this.audio.play().catch(() => {});
      return;
    }
    await this.next();
  }

  enqueue(track: Track) {
    playerStore.update((s) => {
      if (s.queue.some((q) => q.id === track.id)) return s;
      return { ...s, queue: [...s.queue, track] };
    });
  }

  removeFromQueue(idx: number) {
    playerStore.update((s) => {
      if (idx < 0 || idx >= s.queue.length || idx === s.currentIndex) return s;
      const queue = s.queue.filter((_, i) => i !== idx);
      const currentIndex = idx < s.currentIndex ? s.currentIndex - 1 : s.currentIndex;
      return { ...s, queue, currentIndex };
    });
  }

  setVolume(vol: number) {
    const clamped = Math.max(0, Math.min(1, vol));
    playerStore.update((s) => ({ ...s, volume: clamped, muted: clamped === 0 ? s.muted : false }));
    if (this.gainNode) {
      this.gainNode.gain.value = clamped;
    }
  }

  toggleMute() {
    playerStore.update((s) => {
      const muted = !s.muted;
      if (this.gainNode) {
        this.gainNode.gain.value = muted ? 0 : s.volume;
      }
      return { ...s, muted };
    });
  }

  /** Off → Shuffle → Magic shuffle → off. */
  cycleShuffle() {
    playerStore.update((s) => {
      const shuffle: ShuffleMode = s.shuffle === "off" ? "on" : s.shuffle === "on" ? "magic" : "off";
      return { ...s, shuffle };
    });
    if (get(playerStore).shuffle === "magic") this.magic.prime();
  }

  cycleRepeat() {
    playerStore.update((s) => ({
      ...s,
      repeat: s.repeat === "off" ? "all" : s.repeat === "all" ? "one" : "off",
    }));
  }

  updateEq(nextEq: EqState) {
    playerStore.update((s) => ({ ...s, eq: nextEq }));
    this.applyEqToGraph(nextEq);
    void saveEqState(nextEq).catch(() => {});
  }

  setSleepTimer(minutes: number | null) {
    if (this.sleepTimerId !== null) {
      window.clearInterval(this.sleepTimerId);
      this.sleepTimerId = null;
    }
    if (!minutes || minutes <= 0) {
      playerStore.update((s) => ({ ...s, sleepRemainingSec: null, sleepEndOfTrack: false }));
      return;
    }
    let remaining = Math.round(minutes * 60);
    playerStore.update((s) => ({ ...s, sleepRemainingSec: remaining, sleepEndOfTrack: false }));
    this.sleepTimerId = window.setInterval(() => {
      remaining -= 1;
      if (remaining <= 0) {
        if (this.sleepTimerId !== null) window.clearInterval(this.sleepTimerId);
        this.sleepTimerId = null;
        if (this.audio && !this.audio.paused) {
          this.audio.pause();
        }
        playerStore.update((s) => ({ ...s, sleepRemainingSec: null }));
      } else {
        playerStore.update((s) => ({ ...s, sleepRemainingSec: remaining }));
      }
    }, 1000);
  }

  setSleepEndOfTrack(enabled: boolean) {
    if (this.sleepTimerId !== null) {
      window.clearInterval(this.sleepTimerId);
      this.sleepTimerId = null;
    }
    playerStore.update((s) => ({
      ...s,
      sleepRemainingSec: null,
      sleepEndOfTrack: enabled,
    }));
  }

  /**
   * Fills `out` with one dB level for each of `count` log-spaced bands from ~55 Hz to ~15 kHz, so each bar covers an
   * equal musical range: the loudest bin when a band spans two or more bins, otherwise the level interpolated at the
   * band's centre. (Low bands are narrower than a bin; reading whole bins made neighbouring bass bars identical.)
   * Shaping into bar heights is `lib/spectrum.ts`. Returns false (nothing written) while paused.
   */
  getSpectrumDb(count: number, out: Float32Array): boolean {
    if (!this.analyser || !this.freqBuf || !get(playerStore).playing) return false;
    this.analyser.getFloatFrequencyData(this.freqBuf);
    const buf = this.freqBuf;
    const bins = buf.length;
    const binHz = this.analyser.context.sampleRate / 2 / bins;
    const lo = 55;
    const hi = Math.min(15_000, (bins - 1) * binHz);
    for (let i = 0; i < count; i++) {
      const b0 = (lo * (hi / lo) ** (i / count)) / binHz;
      const b1 = (lo * (hi / lo) ** ((i + 1) / count)) / binHz;
      if (b1 - b0 < 2) {
        const at = Math.min(bins - 1.001, Math.sqrt(b0 * b1));
        const j = Math.floor(at);
        out[i] = buf[j] + (buf[j + 1] - buf[j]) * (at - j);
        continue;
      }
      let peak = -Infinity;
      for (let j = Math.ceil(b0); j <= Math.min(bins - 1, Math.floor(b1)); j++) peak = Math.max(peak, buf[j]);
      out[i] = peak;
    }
    return true;
  }

  private checkSkipSilence() {
    const st = get(playerStore);
    if (!st.eq.skipSilence || !st.playing || !this.analyser || !this.timeBuf || !this.audio) {
      this.silenceSinceMs = null;
      return;
    }
    this.analyser.getByteTimeDomainData(this.timeBuf);
    let sumSq = 0;
    for (let i = 0; i < this.timeBuf.length; i++) {
      const v = (this.timeBuf[i] - 128) / 128;
      sumSq += v * v;
    }
    const rms = Math.sqrt(sumSq / this.timeBuf.length);
    const now = performance.now();
    if (rms < 0.008) {
      if (this.silenceSinceMs === null) {
        this.silenceSinceMs = now;
      } else if (now - this.silenceSinceMs > 450 && now - this.lastSilenceJumpAt > 350) {
        if (this.audio.duration && this.audio.currentTime + 0.35 < this.audio.duration - 1) {
          this.audio.currentTime += 0.28;
          this.lastSilenceJumpAt = now;
        }
      }
    } else {
      this.silenceSinceMs = null;
    }
  }

  private updateMediaSessionMetadata(track: Track) {
    if (!this.settings.mediaSessionEnabled || !("mediaSession" in navigator)) return;
    const art = resolveArtworkUrl(track.artworkUrl, this.proxyPort);
    navigator.mediaSession.metadata = new MediaMetadata({
      title: track.title,
      artist: track.artist,
      album: track.album || "Pandora",
      artwork: art ? [{ src: art, sizes: "512x512" }] : [],
    });
  }

  private syncMediaSessionPosition(posMs: number, durMs: number) {
    if (!this.settings.mediaSessionEnabled || !("mediaSession" in navigator) || durMs <= 0) return;
    try {
      navigator.mediaSession.setPositionState({
        duration: durMs / 1000,
        position: Math.min(posMs / 1000, durMs / 1000),
        playbackRate: get(playerStore).eq.playbackRate,
      });
    } catch {
      // Ignore transient position mismatches
    }
  }

  private bindMediaSessionHandlers() {
    if (!("mediaSession" in navigator)) return;
    const ms = navigator.mediaSession;
    ms.setActionHandler("play", () => void this.togglePlayPause());
    ms.setActionHandler("pause", () => void this.togglePlayPause());
    ms.setActionHandler("nexttrack", () => void this.next());
    ms.setActionHandler("previoustrack", () => void this.previous());
    ms.setActionHandler("seekto", (d) => {
      if (typeof d.seekTime === "number") this.seekMs(d.seekTime * 1000);
    });
  }
}

export const audioEngine = new PandoraAudioEngine();

export function resetTheme() {
  applyDefaultMaterialTheme();
}
