import type { DeviceSummary, SavedDevice, SettingsView } from '../ipc/engine.ts';
import { shortDeviceName } from './scheduler.ts';

export const MAX_VISIBLE_DEVICE_CHIPS = 4;

export interface NotchDeviceChip {
  id: string;
  savedId: string;
  label: string;
  fullName: string;
  formFactor: number | null;
  isCurrent: boolean;
  disconnected: boolean;
}

/** The notch has no device picker: favourites and their order are set in Settings → Devices (owner, 2026-11-24). */
export interface NotchDeviceRowModel {
  chips: NotchDeviceChip[];
}

function sameId(a: string | null | undefined, b: string | null | undefined): boolean {
  return Boolean(a && b && a.toLowerCase() === b.toLowerCase());
}

export function isDeviceHidden(
  settings: SettingsView | null,
  deviceId: string | null | undefined,
  savedId?: string | null,
): boolean {
  if (!settings) return false;
  return settings.devices.some(
    (pref) =>
      pref.hidden &&
      (sameId(pref.device.deviceId, deviceId) ||
        sameId(pref.device.savedId, deviceId) ||
        sameId(pref.device.savedId, savedId)),
  );
}

function hasCustomAlias(settings: SettingsView | null, liveId: string): boolean {
  if (!settings) return false;
  return settings.devices.some(
    (pref) =>
      Boolean(pref.alias && pref.alias.trim().length > 0) &&
      (sameId(pref.device.deviceId, liveId) || sameId(pref.device.savedId, liveId)),
  );
}

function formatConnectedLabel(device: DeviceSummary, settings: SettingsView | null): string {
  // snapshot.playback entries already carry the user's alias in `device.name`.
  if (hasCustomAlias(settings, device.id)) {
    return device.name.trim() || shortDeviceName(device.name);
  }
  return shortDeviceName(device.name);
}

function connectedChip(
  device: DeviceSummary,
  settings: SettingsView | null,
  savedId = device.id,
): NotchDeviceChip {
  return {
    id: device.id,
    savedId,
    label: formatConnectedLabel(device, settings),
    fullName: device.name,
    formFactor: device.formFactor,
    isCurrent: device.isDefault,
    disconnected: false,
  };
}

function disconnectedChip(saved: SavedDevice, disconnected: DeviceSummary[]): NotchDeviceChip {
  const summary = disconnected.find((d) => sameId(d.id, saved.savedId));
  return {
    id: saved.savedId,
    savedId: saved.savedId,
    label: shortDeviceName(saved.name),
    fullName: saved.name,
    formFactor: summary?.formFactor ?? null,
    isCurrent: false,
    disconnected: true,
  };
}

function pushSavedEntry(
  chips: NotchDeviceChip[],
  saved: SavedDevice,
  playback: DeviceSummary[],
  settings: SettingsView | null,
  disconnected: DeviceSummary[],
): void {
  if (isDeviceHidden(settings, saved.deviceId ?? saved.savedId, saved.savedId)) return;
  if (chips.some((c) => sameId(c.id, saved.deviceId) || sameId(c.savedId, saved.savedId))) return;

  if (saved.deviceId !== null) {
    const live = playback.find((d) => sameId(d.id, saved.deviceId));
    if (live) {
      chips.push(connectedChip(live, settings, saved.savedId));
      return;
    }
  }
  if (settings?.showDisconnected) {
    chips.push(disconnectedChip(saved, disconnected));
  }
}

export function buildNotchDeviceRow(
  playback: DeviceSummary[],
  settings: SettingsView | null,
  disconnected: DeviceSummary[] = [],
  maxChips = MAX_VISIBLE_DEVICE_CHIPS,
): NotchDeviceRowModel {
  const limit = Math.max(1, maxChips);
  const currentDefault = playback.find((d) => d.isDefault && !isDeviceHidden(settings, d.id)) ?? null;
  const favourites = settings?.favouritesPlayback ?? [];

  if (favourites.length > 0) {
    const chips: NotchDeviceChip[] = [];
    for (const fav of favourites) {
      pushSavedEntry(chips, fav, playback, settings, disconnected);
    }
    if (currentDefault && !chips.some((c) => sameId(c.id, currentDefault.id))) {
      chips.unshift(connectedChip(currentDefault, settings));
    }
    return { chips: chips.slice(0, limit) };
  }

  const chips: NotchDeviceChip[] = [];
  if (currentDefault) {
    chips.push(connectedChip(currentDefault, settings));
  }
  const pair = settings?.togglePairs[0];
  if (pair) {
    const partner = currentDefault && sameId(pair.primary.deviceId, currentDefault.id)
      ? pair.secondary
      : currentDefault && sameId(pair.secondary.deviceId, currentDefault.id)
        ? pair.primary
        : pair.primary.deviceId !== null
          ? pair.primary
          : pair.secondary;
    pushSavedEntry(chips, partner, playback, settings, disconnected);
  }
  return { chips: chips.slice(0, limit) };
}

export function isDeviceFavourite(
  favourites: SavedDevice[] | undefined,
  deviceId: string,
): boolean {
  if (!favourites) return false;
  return favourites.some((f) => sameId(f.deviceId, deviceId) || sameId(f.savedId, deviceId));
}

export function toggleFavouriteDeviceIds(
  favourites: SavedDevice[] | undefined,
  targetDeviceId: string,
): string[] {
  const current = favourites ?? [];
  const included = isDeviceFavourite(current, targetDeviceId);
  if (included) {
    return current
      .filter((f) => !sameId(f.deviceId, targetDeviceId) && !sameId(f.savedId, targetDeviceId))
      .map((f) => f.savedId);
  }
  return [...current.map((f) => f.savedId), targetDeviceId];
}
