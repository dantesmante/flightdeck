// App-wide preferences, loaded once and kept in sync with the database.

import { api, type Settings } from "./api";

export type DistanceUnit = "nm" | "mi" | "km";

const defaults: Settings = {
  distance_unit: "nm",
  logbook_layout: "faa",
  logbook_hand: "hand",
  regional_stamps: "true",
};

export const prefs = $state<{ s: Settings; loaded: boolean }>({ s: { ...defaults }, loaded: false });

export async function loadPrefs() {
  const s = await api.settings();
  prefs.s = { ...defaults, ...(s ?? {}) };
  prefs.loaded = true;
}

export async function setPref(key: string, value: string) {
  prefs.s = { ...prefs.s, [key]: value };
  await api.setSetting(key, value);
}

const FACTOR: Record<DistanceUnit, number> = { nm: 1, mi: 1.150779, km: 1.852 };

/** A distance stored in nautical miles, in the pilot's unit. */
export function distance(nm: number | null | undefined): { value: string; unit: string } {
  const unit = (prefs.s.distance_unit as DistanceUnit) ?? "nm";
  if (nm == null) return { value: "–", unit };
  const v = nm * (FACTOR[unit] ?? 1);
  return { value: new Intl.NumberFormat("en-US", { maximumFractionDigits: v < 100 ? 1 : 0 }).format(v), unit };
}
