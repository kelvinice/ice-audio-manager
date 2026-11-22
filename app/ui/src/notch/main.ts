import '../themes/base.css';
import { mount } from 'svelte';
import NotchApp from './NotchApp.svelte';

const target = document.getElementById('app');
if (!target) {
  throw new Error('Missing #app element for notch');
}

mount(NotchApp, { target });
