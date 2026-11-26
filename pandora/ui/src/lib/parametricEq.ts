import type { ParametricBand, ParametricEqProfile } from "./types";

const MAX_BANDS = 24;

export function mapParametricFilterType(ft: string): BiquadFilterType {
  switch (ft.toUpperCase()) {
    case "LSC":
      return "lowshelf";
    case "HSC":
      return "highshelf";
    case "LPQ":
      return "lowpass";
    case "HPQ":
      return "highpass";
    default:
      return "peaking";
  }
}

export function parseParametricEqText(text: string, profileName = "Custom AutoEq"): ParametricEqProfile {
  const lines = text.split(/\r?\n/);
  let preamp = 0;
  const bands: ParametricBand[] = [];

  for (const raw of lines) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;

    if (/^Preamp:/i.test(line)) {
      const m = line.match(/Preamp:\s*([-+]?\d+(?:\.\d+)?)\s*dB/i);
      if (m) {
        const val = Number.parseFloat(m[1]);
        if (Number.isFinite(val)) {
          preamp = Math.max(-30, Math.min(30, val));
        }
      }
      continue;
    }

    if (/^Filter\b/i.test(line)) {
      if (!/\bON\b/i.test(line)) continue;
      const ftMatch = line.match(/\b(PK|LSC|HSC|LPQ|HPQ)\b/i);
      const fcMatch = line.match(/\bFc\s+([-+]?\d+(?:\.\d+)?)\s*Hz\b/i);
      const gainMatch = line.match(/\bGain\s+([-+]?\d+(?:\.\d+)?)\s*dB\b/i);
      const qMatch = line.match(/\bQ\s+([-+]?\d+(?:\.\d+)?)\b/i);

      if (!ftMatch || !fcMatch || !qMatch) continue;
      const filterType = ftMatch[1].toUpperCase();
      const frequency = Number.parseFloat(fcMatch[1]);
      const gain = gainMatch ? Number.parseFloat(gainMatch[1]) : 0;
      const q = Number.parseFloat(qMatch[1]);

      if (
        !Number.isFinite(frequency) ||
        frequency < 10 ||
        frequency > 22000 ||
        !Number.isFinite(gain) ||
        gain < -24 ||
        gain > 24 ||
        !Number.isFinite(q) ||
        q < 0.05 ||
        q > 20
      ) {
        continue;
      }

      bands.push({ filterType, frequency, gain, q });
      if (bands.length >= MAX_BANDS) break;
    }
  }

  if (bands.length === 0) {
    throw new Error("No valid ON filters found in ParametricEQ text");
  }

  return {
    name: profileName.trim() || "AutoEq Profile",
    preamp,
    bands,
  };
}
