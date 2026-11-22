import '../themes/base.css';
import { mount } from 'svelte';
import SettingsApp from './SettingsApp.svelte';

const target = document.getElementById('app');
if (!target) {
  throw new Error('Missing #app element for settings');
}

mount(SettingsApp, { target });
