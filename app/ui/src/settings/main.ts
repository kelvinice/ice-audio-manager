import '../themes/base.css';
import '../themes/settings.css';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { mount } from 'svelte';
import { type Backend, SettingsClient } from '../lib/settings/client.svelte';
import SettingsApp from './SettingsApp.svelte';

const tauri: Backend = {
  invoke: (command, args) => invoke(command, args),
  listen: (event, handler) => listen(event, (e) => handler(e.payload as never)),
};

async function backend(): Promise<Backend> {
  // Dead code in production builds (`DEV` is false), so the preview backend is never bundled.
  if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
    document.documentElement.dataset.backdrop = 'solid';
    return (await import('./devBackend')).createDevBackend();
  }
  return tauri;
}

const target = document.getElementById('app');
if (!target) {
  throw new Error('Missing #app element for settings');
}

mount(SettingsApp, { target, props: { client: new SettingsClient(await backend()) } });
