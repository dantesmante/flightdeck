// Screenshots: where to load each picture from, where it was taken, and
// which ones are open in the viewer.
import type { Screenshot } from "./api";
import { inTauri } from "./api";
import { inRemote, remoteToken } from "./remote.svelte";
import type { Pt } from "./track";

/** The address of a screenshot's picture: read from where it is on the PC. */
export function shotUrl(id: number): string {
  if (inRemote) return `/api/shot/${id}?token=${encodeURIComponent(remoteToken() ?? "")}`;
  if (inTauri) return /Windows/i.test(navigator.userAgent) ? `http://fdshot.localhost/${id}` : `fdshot://localhost/${id}`;
  // The browser preview has no PC to read from: a stand-in picture.
  return mockPicture(id);
}

function mockPicture(id: number): string {
  const hue = (id * 67) % 360;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="900"><defs><linearGradient id="s" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="hsl(${hue},55%,62%)"/><stop offset="0.62" stop-color="hsl(${(hue + 30) % 360},45%,82%)"/><stop offset="0.62" stop-color="hsl(${(hue + 120) % 360},25%,35%)"/><stop offset="1" stop-color="hsl(${(hue + 120) % 360},30%,22%)"/></linearGradient></defs><rect width="1600" height="900" fill="url(#s)"/><text x="60" y="840" font-family="sans-serif" font-size="48" fill="white" opacity="0.8">Screenshot ${id}</text></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

/** Where on a track a moment was: position and altitude, between points. */
export function placeOn(track: Pt[], t: number): { lon: number; lat: number; alt: number } | null {
  if (!track.length) return null;
  if (t <= track[0][4]) return { lon: track[0][0], lat: track[0][1], alt: track[0][2] };
  const last = track[track.length - 1];
  if (t >= last[4]) return { lon: last[0], lat: last[1], alt: last[2] };
  let lo = 0;
  let hi = track.length - 1;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (track[mid][4] <= t) lo = mid;
    else hi = mid;
  }
  const a = track[lo];
  const b = track[hi];
  const f = b[4] > a[4] ? (t - a[4]) / (b[4] - a[4]) : 0;
  return { lon: a[0] + (b[0] - a[0]) * f, lat: a[1] + (b[1] - a[1]) * f, alt: a[2] + (b[2] - a[2]) * f };
}

export interface PlacedShot extends Screenshot {
  lon: number | null;
  lat: number | null;
  alt: number | null;
}

/** Screenshots shown now: the open flight's, the flight under way's, and
 * the picture open in the viewer. */
export const shots = $state<{
  flight: Screenshot[];
  live: Screenshot[];
  viewer: { list: PlacedShot[]; index: number } | null;
}>({ flight: [], live: [], viewer: null });

export function place(list: Screenshot[], track: Pt[]): PlacedShot[] {
  return list.map((s) => {
    const p = placeOn(track, s.taken_ms);
    return { ...s, lon: p?.lon ?? null, lat: p?.lat ?? null, alt: p?.alt ?? null };
  });
}
