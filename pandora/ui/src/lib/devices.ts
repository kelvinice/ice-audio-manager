import type { DeviceSummary, SpatialMode } from "./types";

/** Windows EndpointFormFactor: 3 headphones, 5 headset; everything else reads as a speaker. */
export function isHeadphones(device: DeviceSummary): boolean {
  return device.formFactor === 3 || device.formFactor === 5;
}

/** What kind of output a device is, in words. */
export function deviceKind(device: DeviceSummary): string {
  if (device.formFactor === 5) return "Headset";
  if (device.formFactor === 3) return "Headphones";
  if (device.isVirtual) return "Virtual device";
  return "Speakers";
}

/** Spatial sound formats as Windows names them. */
export const SPATIAL_LABELS: Record<SpatialMode, string> = {
  keep: "Unchanged",
  off: "Off",
  windowsSonic: "Windows Sonic",
  dolbyAtmosHeadphones: "Dolby Atmos for Headphones",
  dolbyAtmosHomeTheater: "Dolby Atmos for Home Theater",
  dolbyAtmosSpeakers: "Dolby Atmos for Speakers",
  dtsHeadphoneX: "DTS Headphone:X",
  dtsXUltra: "DTS:X Ultra",
  dtsXHomeTheater: "DTS:X for Home Theater",
};

/** The formats offered in the spatial sound picker. */
export const SPATIAL_CHOICES: SpatialMode[] = ["off", "windowsSonic", "dolbyAtmosHeadphones", "dtsHeadphoneX"];
