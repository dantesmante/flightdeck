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

/** Track length in km, along the ground. */
function lengthKm(pts: Pt[]): number {
  let km = 0;
  for (let i = 1; i < pts.length; i++) {
    const [lo1, la1] = pts[i - 1];
    const [lo2, la2] = pts[i];
    const x = (lo2 - lo1) * Math.cos(((la1 + la2) / 2) * (Math.PI / 180));
    km += Math.hypot(x, la2 - la1) * 111.32;
  }
  return km;
}

/** How much the 3D wall's height is stretched so it reads on screen: real
 * height for short hops, up to 8x for long flights where 35,000 ft would
 * otherwise be a sliver next to 1,000 km of track. */
export function wallScale(pts: Pt[]): number {
  const { lo, top } = altRange(pts);
  const tallKm = Math.max(0.05, ((top - lo) * 0.3048) / 1000);
  return Math.round(Math.min(8, Math.max(1, (0.045 * lengthKm(pts)) / tallKm)) * 2) / 2;
}

/** The 3D "curtain": one thin strip per track segment, raised from the
 * ground to the altitude flown there (above the lowest point of the flight),
 * coloured like the line. For a fill-extrusion layer reading the `h` (metres)
 * and `c` (colour) properties. */
export function wallFeatures(pts: Pt[], scale: number, maxPieces = 1800): GeoJSON.FeatureCollection {
  const features: GeoJSON.Feature[] = [];
  if (pts.length < 2) return { type: "FeatureCollection", features };
  const { lo, hi } = altRange(pts);
  // Ground length of each segment (in degrees, latitude-corrected), so long
  // straight legs can be cut into short pieces with a smooth height.
  const seg: number[] = [];
  let total = 0;
  for (let i = 1; i < pts.length; i++) {
    const k = Math.cos(((pts[i - 1][1] + pts[i][1]) / 2) * (Math.PI / 180));
    const d = Math.hypot((pts[i][0] - pts[i - 1][0]) * k, pts[i][1] - pts[i - 1][1]);
    seg.push(d);
    total += d;
  }
  if (!(total > 0)) return { type: "FeatureCollection", features };
  const piece = total / maxPieces;
  // Half-thickness of the wall: thin, but never zero.
  const half = 0.00006;
  const push = (ax: number, ay: number, bx: number, by: number, alt: number) => {
    const k = Math.cos(((ay + by) / 2) * (Math.PI / 180)) || 1e-6;
    const dx = (bx - ax) * k;
    const dy = by - ay;
    const len = Math.hypot(dx, dy);
    if (len < 1e-9) return;
    const ox = ((-dy / len) * half) / k;
    const oy = (dx / len) * half;
    features.push({
      type: "Feature",
      properties: { h: Math.max(2, (alt - lo) * 0.3048 * scale), c: altColor((alt - lo) / (hi - lo)) },
      geometry: {
        type: "Polygon",
        coordinates: [[
          [ax + ox, ay + oy],
          [bx + ox, by + oy],
          [bx - ox, by - oy],
          [ax - ox, ay - oy],
          [ax + ox, ay + oy],
        ]],
      },
    });
  };
  // Short segments are merged until they make up one piece; long ones are
  // split. Each piece takes the altitude at its middle.
  let start = 0;
  let acc = 0;
  for (let i = 1; i < pts.length; i++) {
    const a = pts[i - 1];
    const b = pts[i];
    const d = seg[i - 1];
    if (d > piece * 1.5) {
      // Close any pending run first.
      if (i - 1 > start) push(pts[start][0], pts[start][1], a[0], a[1], (pts[start][2] + a[2]) / 2);
      const n = Math.min(400, Math.ceil(d / piece));
      for (let j = 0; j < n; j++) {
        const t0 = j / n;
        const t1 = (j + 1) / n;
        const tm = (t0 + t1) / 2;
        push(
          a[0] + (b[0] - a[0]) * t0, a[1] + (b[1] - a[1]) * t0,
          a[0] + (b[0] - a[0]) * t1, a[1] + (b[1] - a[1]) * t1,
          a[2] + (b[2] - a[2]) * tm,
        );
      }
      start = i;
      acc = 0;
      continue;
    }
    acc += d;
    if (acc >= piece || i === pts.length - 1) {
      // Altitude: average of the run's points, so short bumps still show.
      let sum = 0;
      for (let j = start; j <= i; j++) sum += pts[j][2];
      const alt = sum / (i - start + 1);
      // Draw the run as its own small segments so curves stay curved.
      for (let j = start + 1; j <= i; j++) push(pts[j - 1][0], pts[j - 1][1], pts[j][0], pts[j][1], alt);
      start = i;
      acc = 0;
    }
  }
  return { type: "FeatureCollection", features };
}

/** A camera bearing that looks across the flight rather than along it, so
 * the wall's face is in view. Loops (same start and end) get a fixed angle. */
export function sideOnBearing(pts: Pt[]): number {
  if (pts.length < 2) return -20;
  const a = pts[0];
  const b = pts[pts.length - 1];
  const k = Math.cos(((a[1] + b[1]) / 2) * (Math.PI / 180));
  const dx = (b[0] - a[0]) * k;
  const dy = b[1] - a[1];
  if (Math.hypot(dx, dy) < 0.05) return -20;
  const course = (Math.atan2(dx, dy) * 180) / Math.PI;
  // Course minus 70 degrees: the route runs across the screen at a slant.
  let brg = course - 70;
  while (brg > 180) brg -= 360;
  while (brg < -180) brg += 360;
  // Prefer looking north-ish so the map is not upside down.
  if (Math.abs(brg) > 90) brg = brg > 0 ? brg - 180 : brg + 180;
  return brg;
}
