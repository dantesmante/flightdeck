// A flight track with altitude and speed, and the altitude colour scale
// shared by the map line, the mini map and the altitude & speed graph.
import type maplibregl from "maplibre-gl";
import type { TrackPoint } from "./api";

/** [lon, lat, altitude ft, ground speed kt, time ms] */
export type Pt = [number, number, number, number, number];

export function fromTrack(track: TrackPoint[]): Pt[] {
  return track.map((p) => [p.lon, p.lat, p.alt_ft, p.gs_kt, p.t_ms]);
}

export const lonLat = (pts: Pt[]): [number, number][] => pts.map((p) => [p[0], p[1]]);

/** Low to high: warm red on the ground, through coral, amber and lime,
 * to a cool green at cruise. */
export const ALT_STOPS: [number, string][] = [
  [0, "#ef4b4b"],
  [0.22, "#f57d3d"],
  [0.45, "#f6b73c"],
  [0.7, "#a9d65b"],
  [1, "#34c98b"],
];

export const SPEED_COLOR = "#ffd447";

/** The altitude range a flight's colours span: the lowest point flown
 * (about field elevation) up to its highest. */
export function altRange(pts: Pt[]): { lo: number; hi: number; top: number } {
  let lo = Infinity;
  let hi = -Infinity;
  for (const p of pts) {
    if (p[2] < lo) lo = p[2];
    if (p[2] > hi) hi = p[2];
  }
  if (!isFinite(lo)) return { lo: 0, hi: 1, top: 0 };
  // A flight that never climbs much still reads as low (red), not green.
  // `top` is the real highest point, for labels.
  return { lo, hi: Math.max(hi, lo + 1500), top: hi };
}

function hex(c: string): [number, number, number] {
  const n = parseInt(c.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** Colour for a fraction 0..1 of the altitude range. */
export function altColor(f: number): string {
  const x = Math.min(1, Math.max(0, f));
  for (let i = 1; i < ALT_STOPS.length; i++) {
    const [b, cb] = ALT_STOPS[i];
    const [a, ca] = ALT_STOPS[i - 1];
    if (x <= b) {
      const t = (x - a) / (b - a || 1);
      const p = hex(ca);
      const q = hex(cb);
      const m = p.map((v, k) => Math.round(v + (q[k] - v) * t));
      return `rgb(${m[0]},${m[1]},${m[2]})`;
    }
  }
  return ALT_STOPS[ALT_STOPS.length - 1][1];
}

/** MapLibre measures line-progress in Web Mercator, so we do too. */
function merc(lon: number, lat: number): [number, number] {
  const l = Math.max(-85, Math.min(85, lat)) * (Math.PI / 180);
  return [lon / 360, Math.log(Math.tan(Math.PI / 4 + l / 2)) / (2 * Math.PI)];
}

/** A line-gradient expression colouring the track by altitude. The source
 * must be added with lineMetrics: true. */
export function altGradient(pts: Pt[], maxStops = 240): maplibregl.ExpressionSpecification {
  const solid = (c: string): maplibregl.ExpressionSpecification => ["interpolate", ["linear"], ["line-progress"], 0, c, 1, c];
  if (pts.length < 2) return solid(ALT_STOPS[0][1]);
  const { lo, hi } = altRange(pts);
  const cum = [0];
  let [px, py] = merc(pts[0][0], pts[0][1]);
  for (let i = 1; i < pts.length; i++) {
    const [x, y] = merc(pts[i][0], pts[i][1]);
    cum.push(cum[i - 1] + Math.hypot(x - px, y - py));
    px = x;
    py = y;
  }
  const total = cum[cum.length - 1];
  if (!(total > 0)) return solid(altColor((pts[0][2] - lo) / (hi - lo)));
  const stops: (number | string)[] = [];
  let last = -1;
  let lastA = -1;
  const step = 1 / maxStops;
  for (let i = 0; i < pts.length; i++) {
    const f = cum[i] / total;
    const a = (pts[i][2] - lo) / (hi - lo);
    // Thin out stops, but keep the ends and every real change of colour.
    const end = i === 0 || i === pts.length - 1;
    if (!end && f - last < step && Math.abs(a - lastA) < 0.05) continue;
    if (f <= last + 1e-9) continue;
    stops.push(f, altColor(a));
    last = f;
    lastA = a;
  }
  if (stops.length < 4) return solid(String(stops[1] ?? ALT_STOPS[0][1]));
  return ["interpolate", ["linear"], ["line-progress"], ...stops] as maplibregl.ExpressionSpecification;
}

/** A tidy altitude label: 35,000 ft becomes FL350 above 18,000 ft. */
export function altLabel(ft: number): string {
  if (ft >= 18000) return `FL${Math.round(ft / 100)}`;
  return `${(Math.round(ft / 100) * 100).toLocaleString("en-US")} ft`;
}
