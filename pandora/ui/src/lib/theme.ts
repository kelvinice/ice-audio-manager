// Dynamic Material You color engine: extracts a vibrant 3-color palette from the active
// album art (via local CORS-safe proxy or data URI) and updates root CSS custom properties.

interface Rgb {
  r: number;
  g: number;
  b: number;
}

const DEFAULT_PALETTE: [Rgb, Rgb, Rgb] = [
  { r: 109, g: 131, b: 242 },
  { r: 56, g: 189, b: 248 },
  { r: 168, g: 85, b: 247 },
];

export function resolveArtworkUrl(url: string | null, proxyPort: number): string | null {
  if (!url) return null;
  if (url.startsWith("data:") || url.startsWith("blob:") || url.startsWith("http://127.0.0.1")) {
    return url;
  }
  if (proxyPort > 0 && url.startsWith("https://")) {
    return `http://127.0.0.1:${proxyPort}/art?url=${encodeURIComponent(url)}`;
  }
  return url;
}

let artworkRevision = 0;

export function applyDefaultMaterialTheme(): void {
  artworkRevision++;
  for (const property of ["--art-primary", "--art-secondary", "--art-tertiary"]) {
    document.documentElement.style.removeProperty(property);
  }
}

export async function applyMaterialThemeFromArt(artUrl: string | null, proxyPort: number): Promise<void> {
  applyDefaultMaterialTheme();
  const revision = artworkRevision;
  const resolved = resolveArtworkUrl(artUrl, proxyPort);
  if (!resolved) return;
  try {
    const colors = await extractPalette(resolved);
    // A later track or disabling artwork accents invalidates this decode.
    if (revision !== artworkRevision || !colors) return;
    const style = document.documentElement.style;
    const properties = ["--art-primary", "--art-secondary", "--art-tertiary"];
    colors.forEach(({ r, g, b }, i) => style.setProperty(properties[i], `rgb(${r}, ${g}, ${b})`));
  } catch {
    // Missing artwork leaves the selected palette in effect.
  }
}

function extractPalette(src: string): Promise<[Rgb, Rgb, Rgb] | null> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => {
      try {
        const size = 28;
        const canvas = document.createElement("canvas");
        canvas.width = size;
        canvas.height = size;
        const ctx = canvas.getContext("2d");
        if (!ctx) {
          reject(new Error("no 2d context"));
          return;
        }
        ctx.drawImage(img, 0, 0, size, size);
        const data = ctx.getImageData(0, 0, size, size).data;
        const buckets = new Map<string, { r: number; g: number; b: number; weight: number }>();

        for (let i = 0; i < data.length; i += 4) {
          const a = data[i + 3];
          if (a < 180) continue;
          const r = data[i];
          const g = data[i + 1];
          const b = data[i + 2];
          const max = Math.max(r, g, b);
          const min = Math.min(r, g, b);
          const sat = max === 0 ? 0 : (max - min) / max;
          const lum = (r + g + b) / 3;
          if (lum < 28 || lum > 238) continue;

          // Quantize to 24-step buckets and weight colorful pixels higher
          const qr = Math.round(r / 24) * 24;
          const qg = Math.round(g / 24) * 24;
          const qb = Math.round(b / 24) * 24;
          const key = `${qr},${qg},${qb}`;
          const weight = 1 + sat * 4.5;
          const cur = buckets.get(key);
          if (cur) {
            cur.r += r * weight;
            cur.g += g * weight;
            cur.b += b * weight;
            cur.weight += weight;
          } else {
            buckets.set(key, { r: r * weight, g: g * weight, b: b * weight, weight });
          }
        }

        const sorted = [...buckets.values()]
          .sort((a, b) => b.weight - a.weight)
          .map((b) => ({
            r: Math.round(b.r / b.weight),
            g: Math.round(b.g / b.weight),
            b: Math.round(b.b / b.weight),
          }));

        if (sorted.length === 0) {
          resolve(null);
          return;
        }
        const chosen: Rgb[] = [sorted[0]];
        for (const cand of sorted.slice(1)) {
          if (chosen.every((c) => colorDist(c, cand) > 75)) {
            chosen.push(cand);
            if (chosen.length === 3) break;
          }
        }
        while (chosen.length < 3) {
          chosen.push(sorted[chosen.length % sorted.length] ?? DEFAULT_PALETTE[chosen.length]);
        }
        resolve([chosen[0], chosen[1], chosen[2]]);
      } catch (err) {
        reject(err);
      }
    };
    img.onerror = () => reject(new Error("image load failed"));
    img.src = src;
  });
}

function colorDist(a: Rgb, b: Rgb): number {
  return Math.hypot(a.r - b.r, a.g - b.g, a.b - b.b);
}
