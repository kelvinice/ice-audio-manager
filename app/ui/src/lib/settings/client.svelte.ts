// Live engine state for the Settings window. Rust stays the single source of truth (code-practices §3): every
// change goes out as a settings `Command`, and the page re-renders from the `SettingsChanged` that comes back.
import type { Command, EngineSnapshot, Envelope, LyricsStorageStats, Notice } from '../ipc/engine.ts';
import type { NotchDelta, NotchSnapshot } from '../ipc/types.ts';
import { applyEnvelope } from './model.ts';

/** The two Tauri calls the window needs, injectable so the dev preview can run in a browser. */
export interface Backend {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(event: string, handler: (payload: T) => void): Promise<() => void>;
}

export type Banner = { tone: 'error' | 'warning'; title: string; text: string };

/** How long the header's "Saved" confirmation stays. */
const SAVED_MS = 1800;
/** A command that changes nothing gets no reply from the engine; after this long, silence counts as done. */
const UNCHANGED_MS = 1500;

export class SettingsClient {
  engine = $state<EngineSnapshot | null>(null);
  notch = $state<NotchSnapshot | null>(null);
  launchAtSignIn = $state<boolean | null>(null);
  lyricsStorage = $state<LyricsStorageStats | null>(null);
  version = $state<string | null>(null);
  banner = $state<Banner | null>(null);
  saved = $state(false);

  private pending: Envelope[] = [];
  /** Resolves the newest `apply` once the engine answers. */
  private settle: ((saved: boolean) => void) | null = null;
  private savedTimer: ReturnType<typeof setTimeout> | null = null;
  private resnapshotting = false;

  constructor(private readonly backend: Backend) {}

  /** Listeners first, then the snapshots (D-03), so nothing between the two is lost. Returns the cleanup. */
  async start(): Promise<() => void> {
    const offs = await Promise.all([
      this.backend.listen<Envelope>('engine://delta/v1', (e) => this.onEnvelope(e)),
      this.backend.listen<EngineSnapshot>('engine://snapshot/v1', (s) => this.onSnapshot(s)),
      this.backend.listen<NotchDelta>('notch://delta/v1', (d) => this.onNotch(d.snapshot)),
    ]);
    const [engine, notch, launch] = await Promise.all([
      this.backend.invoke<EngineSnapshot>('get_engine_snapshot'),
      this.backend.invoke<NotchSnapshot>('get_notch_snapshot'),
      this.backend.invoke<boolean | null>('get_launch_at_sign_in'),
    ]);
    this.onSnapshot(engine);
    this.onNotch(notch);
    this.launchAtSignIn = launch;
    this.version = await this.backend.invoke<string>('plugin:app|version').catch(() => null);
    return () => {
      for (const off of offs) off();
      if (this.savedTimer) clearTimeout(this.savedTimer);
    };
  }

  /** Sends a settings command. Resolves `true` once saved, `false` if it was refused (the banner says why). */
  apply(command: Command): Promise<boolean> {
    this.banner = null;
    this.settle?.(true);
    return new Promise((resolve) => {
      const finish = (saved: boolean) => {
        clearTimeout(timer);
        if (this.settle === finish) this.settle = null;
        resolve(saved);
      };
      const timer = setTimeout(() => finish(true), UNCHANGED_MS);
      this.settle = finish;
      this.backend.invoke('apply_setting', { command }).catch((error: unknown) => {
        this.banner = { tone: 'error', title: 'Couldn’t save that change', text: String(error) };
        finish(false);
      });
    });
  }

  async setLaunchAtSignIn(enabled: boolean): Promise<void> {
    try {
      this.launchAtSignIn = await this.backend.invoke<boolean | null>('set_launch_at_sign_in', { enabled });
      this.flashSaved();
    } catch (error) {
      this.banner = { tone: 'error', title: 'Couldn’t change launch at sign-in', text: String(error) };
    }
  }

  async refreshLyricsStorage(): Promise<void> {
    try {
      this.lyricsStorage = await this.backend.invoke<LyricsStorageStats>('get_lyrics_storage_stats');
    } catch (error) {
      this.banner = { tone: 'warning', title: 'Couldn’t read lyrics storage', text: String(error) };
    }
  }

  async clearLyricsCache(): Promise<void> {
    try {
      this.lyricsStorage = await this.backend.invoke<LyricsStorageStats>('clear_lyrics_cache');
      this.flashSaved();
    } catch (error) {
      this.banner = { tone: 'error', title: 'Couldn’t clear lyrics cache', text: String(error) };
    }
  }

  async openLyricsFolder(): Promise<void> {
    try {
      await this.backend.invoke('open_lyrics_folder');
    } catch (error) {
      this.banner = { tone: 'error', title: 'Couldn’t open lyrics folder', text: String(error) };
    }
  }

  /** First paint, for the shell's Settings timing (`report_settings_ready`). */
  reportReady(): void {
    void this.backend.invoke('report_settings_ready').catch(() => {});
  }

  dismissBanner(): void {
    this.banner = null;
  }

  private onSnapshot(snapshot: EngineSnapshot): void {
    if (this.engine && snapshot.revision < this.engine.revision) return;
    this.engine = snapshot;
    const buffered = this.pending;
    this.pending = [];
    for (const envelope of buffered) this.onEnvelope(envelope);
  }

  private onEnvelope(envelope: Envelope): void {
    if (!this.engine) {
      this.pending.push(envelope);
      return;
    }
    const outcome = applyEnvelope(this.engine, envelope);
    if (outcome.resnapshot) {
      void this.resnapshot();
      return;
    }
    this.engine = outcome.snapshot;
    const event = envelope.event;
    if (event.type === 'settingsChanged' && this.settle) {
      this.settle(true);
      this.flashSaved();
    } else if (event.type === 'notice') {
      this.onNotice(event.notice);
    }
  }

  private onNotice(notice: Notice): void {
    if (notice.type === 'settingRejected') {
      this.settle?.(false);
      this.banner = { tone: 'error', title: 'That change wasn’t saved', text: sentence(notice.error) };
    } else if (notice.type === 'hotkeyConflict') {
      this.banner = {
        tone: 'warning',
        title: `${notice.keys} is already taken`,
        text: 'Another app registered this shortcut first. Pick a different one.',
      };
    }
  }

  private onNotch(snapshot: NotchSnapshot): void {
    if (this.notch && snapshot.revision < this.notch.revision) return;
    this.notch = snapshot;
  }

  private async resnapshot(): Promise<void> {
    if (this.resnapshotting) return;
    this.resnapshotting = true;
    try {
      this.engine = await this.backend.invoke<EngineSnapshot>('get_engine_snapshot');
    } finally {
      this.resnapshotting = false;
    }
  }

  private flashSaved(): void {
    this.saved = true;
    if (this.savedTimer) clearTimeout(this.savedTimer);
    this.savedTimer = setTimeout(() => (this.saved = false), SAVED_MS);
  }
}

/** Engine errors are short lower-case phrases ("device x is not connected"); show them as a sentence. */
function sentence(text: string): string {
  const trimmed = text.trim();
  if (!trimmed) return trimmed;
  const capital = trimmed.charAt(0).toUpperCase() + trimmed.slice(1);
  return /[.!?]$/.test(capital) ? capital : `${capital}.`;
}
