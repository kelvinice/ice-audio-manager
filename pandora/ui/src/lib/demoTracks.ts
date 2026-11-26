import type { Track } from "./types";

function makeSvgCover(c1: string, c2: string, c3: string, label: string): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
    <defs>
      <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="${c1}"/>
        <stop offset="55%" stop-color="${c2}"/>
        <stop offset="100%" stop-color="${c3}"/>
      </linearGradient>
    </defs>
    <rect width="512" height="512" rx="48" fill="url(#g)"/>
    <circle cx="256" cy="240" r="136" fill="rgba(255,255,255,0.10)" stroke="rgba(255,255,255,0.28)" stroke-width="4"/>
    <circle cx="256" cy="240" r="44" fill="rgba(10,12,22,0.55)"/>
    <text x="256" y="440" text-anchor="middle" fill="rgba(255,255,255,0.9)" font-family="Segoe UI, sans-serif" font-size="30" font-weight="700" letter-spacing="4">${label}</text>
  </svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

// Synthesizes a clean stereo WAV ambient chord progression in memory so prototype playback,
// EQ, normalization, visualizer, SMTC, and synced lyrics work 100% offline.
function synthWavDataUrl(freqs: number[], durationSec: number): string {
  const sampleRate = 22050;
  const numSamples = Math.floor(sampleRate * durationSec);
  const dataSize = numSamples * 2;
  const buf = new ArrayBuffer(44 + dataSize);
  const view = new DataView(buf);
  const writeStr = (off: number, s: string) => {
    for (let i = 0; i < s.length; i++) view.setUint8(off + i, s.charCodeAt(i));
  };
  writeStr(0, "RIFF");
  view.setUint32(4, 36 + dataSize, true);
  writeStr(8, "WAVE");
  writeStr(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * 2, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  writeStr(36, "data");
  view.setUint32(40, dataSize, true);

  for (let i = 0; i < numSamples; i++) {
    const t = i / sampleRate;
    const chordIdx = Math.floor(t / 4) % freqs.length;
    const base = freqs[chordIdx];
    const pulse = 0.65 + 0.35 * Math.sin(2 * Math.PI * 1.8 * t);
    const sig =
      0.42 * Math.sin(2 * Math.PI * base * t) +
      0.26 * Math.sin(2 * Math.PI * (base * 1.25) * t) +
      0.18 * Math.sin(2 * Math.PI * (base * 1.5) * t) +
      0.12 * Math.sin(2 * Math.PI * (base * 0.5) * t);
    const env = Math.min(1, t * 3) * Math.min(1, (durationSec - t) * 2);
    const sample = Math.max(-1, Math.min(1, sig * pulse * env * 0.6));
    view.setInt16(44 + i * 2, Math.round(sample * 32767), true);
  }

  const bytes = new Uint8Array(buf);
  let bin = "";
  const chunk = 8192;
  for (let i = 0; i < bytes.length; i += chunk) {
    bin += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return `data:audio/wav;base64,${btoa(bin)}`;
}

let cachedDemo: Track[] | null = null;

export function getDemoTracks(): Track[] {
  if (cachedDemo) return cachedDemo;
  cachedDemo = [
    {
      id: "demo-midnight-glass",
      source: "demo",
      title: "Midnight Glass",
      artist: "Pandora Studio",
      album: "Prototype Sessions",
      durationMs: 24000,
      artworkUrl: makeSvgCover("#1e1b4b", "#4f46e5", "#06b6d4", "MIDNIGHT"),
      localPath: null,
      cached: true,
      liked: false,
      demoDataUrl: synthWavDataUrl([220, 261.63, 196, 174.61], 24),
      demoLrc: `[00:00.50] Neon reflections drift across the glass
[00:04.20] Every frequency tuned to the night
[00:08.40] Switching speakers without missing a beat
[00:12.60] Ice Audio keeping every room in sync
[00:17.20] Hold the silence, let the sub bass breathe
[00:20.80] Pandora wakes the desktop air`,
    },
    {
      id: "demo-neon-rain",
      source: "demo",
      title: "Neon Rain (Tokyo)",
      artist: "Kuro & Synth",
      album: "Shibuya Afterglow",
      durationMs: 24000,
      artworkUrl: makeSvgCover("#4c0519", "#db2777", "#7c3aed", "NEON RAIN"),
      localPath: null,
      cached: true,
      liked: false,
      demoDataUrl: synthWavDataUrl([246.94, 293.66, 220, 196], 24),
      demoLrc: `[00:00.40] よるのまちに あめがふる
[00:04.50] ひかりのなみ ゆれるおと
[00:08.80] Static fades into midnight blue
[00:13.20] Every bar dances with the room
[00:17.50] もういちど きみをきかせて`,
    },
    {
      id: "demo-amber-horizon",
      source: "demo",
      title: "Amber Horizon",
      artist: "Solaris Drift",
      album: "Warm Signal",
      durationMs: 24000,
      artworkUrl: makeSvgCover("#451a03", "#ea580c", "#e11d48", "AMBER"),
      localPath: null,
      cached: true,
      liked: false,
      demoDataUrl: synthWavDataUrl([196, 246.94, 293.66, 220], 24),
      demoLrc: `[00:00.60] Warm analog waves across the desk
[00:05.00] Ten bands sculpted to your headphones
[00:10.00] Material colors bloom from the cover art
[00:15.50] Offline or streaming, the music stays yours`,
    },
  ];
  return cachedDemo;
}
