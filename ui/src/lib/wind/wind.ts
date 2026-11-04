// Wind on the map: the forecast grid, the colour scale and the levels.

import type { WindGrid } from "../api";

/** Wind speed colours (knots), shared by the particles, arrows and key. */
export const WIND_STOPS: [number, string][] = [
  [0, "#6f93c8"],
  [10, "#4fb0c6"],
  [20, "#49c27a"],
  [35, "#c9d94a"],
  [50, "#f0b23a"],
  [70, "#ec6a3a"],
  [100, "#e2453c"],
  [140, "#d84fc4"],
];

/** The colour bucket (index into WIND_STOPS) of a speed. */
export function windBucket(kt: number): number {
  let i = 0;
  while (i < WIND_STOPS.length - 1 && kt >= WIND_STOPS[i + 1][0]) i++;
  return i;
}

/** The levels offered: the backend's name, a pilot's name and a typical altitude. */
export const WIND_LEVELS: { id: string; label: string; ft: number }[] = [
  { id: "sfc", label: "Surface", ft: 0 },
  { id: "850", label: "FL050", ft: 4800 },
  { id: "700", label: "FL100", ft: 9900 },
  { id: "500", label: "FL180", ft: 18300 },
  { id: "400", label: "FL240", ft: 23600 },
  { id: "300", label: "FL300", ft: 30000 },
  { id: "250", label: "FL340", ft: 34000 },
  { id: "200", label: "FL390", ft: 38600 },
];

/** The level nearest an altitude in feet. */
export function levelFor(ft: number): string {
  let best = WIND_LEVELS[0];
  for (const l of WIND_LEVELS) if (Math.abs(l.ft - ft) < Math.abs(best.ft - ft)) best = l;
  return best.id;
}

/** A decoded grid: components in knots, 360 x 181 from 90N and 0E. */
export interface Field {
  u: Int16Array;
  v: Int16Array;
  run_ms: number;
  valid_ms: number;
  level: string;
}

function int16(b64: string): Int16Array {
  const s = atob(b64);
  const bytes = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) bytes[i] = s.charCodeAt(i);
  // Little-endian, as every machine running FlightDeck is.
  return new Int16Array(bytes.buffer, 0, bytes.length >> 1);
}

export function decodeGrid(g: WindGrid): Field | null {
  if (g.error || !g.u || g.nx !== 360 || g.ny !== 181) return null;
  return { u: int16(g.u), v: int16(g.v), run_ms: g.run_ms, valid_ms: g.valid_ms, level: g.level };
}

/** Wind at a place, bilinear between the grid points: [east, north] knots. */
export function sample(f: Field, lon: number, lat: number): [number, number] {
  const x = (((lon % 360) + 360) % 360);
  const y = Math.min(180, Math.max(0, 90 - lat));
  const i0 = Math.floor(x);
  const j0 = Math.min(179, Math.floor(y));
  const i1 = (i0 + 1) % 360;
  const j1 = j0 + 1;
  const fx = x - i0;
  const fy = y - j0;
  const at = (a: Int16Array, i: number, j: number) => a[j * 360 + i];
  const lerp = (a: Int16Array) =>
    (at(a, i0, j0) * (1 - fx) + at(a, i1, j0) * fx) * (1 - fy) + (at(a, i0, j1) * (1 - fx) + at(a, i1, j1) * fx) * fy;
  return [lerp(f.u), lerp(f.v)];
}

/** Direction the wind blows from (degrees true) and speed (knots). */
export function dirSpeed([u, v]: [number, number]): [number, number] {
  const kt = Math.hypot(u, v);
  const dir = (Math.atan2(-u, -v) * 180) / Math.PI;
  return [Math.round((dir + 360) % 360) || 360, kt];
}
