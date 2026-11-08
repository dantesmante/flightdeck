// The vertical profile of a route: planned altitude along it, the ground
// under it (coarse, from the free Terrain Tiles), FAA obstacles and US
// class B/C/D airspace it passes through, and where the aircraft is along
// it. Everything here is plain arithmetic for the processor; the chart is
// SVG.

export interface ProfileRoute {
  origin: { icao: string; lat: number; lon: number };
  dest: { icao: string; lat: number; lon: number; runway?: string };
  fixes: { ident: string; lat: number; lon: number; alt_ft: number; stage?: string }[];
  /** Planned cruise altitude, feet (0: not known). */
  cruise_ft: number;
}

export interface RoutePoint {
  ident: string;
  lat: number;
  lon: number;
  /** Distance from the origin, nm. */
  d: number;
  /** Planned altitude, ft (NaN where not planned: the airports). */
  alt: number;
}

const NM_M = 1852;
const R_NM = 6371008.8 / NM_M;
const rad = (d: number) => (d * Math.PI) / 180;

/** Great-circle distance, nm. */
export function gcNm(aLat: number, aLon: number, bLat: number, bLon: number): number {
  const dLat = rad(bLat - aLat);
  const dLon = rad(bLon - aLon);
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(rad(aLat)) * Math.cos(rad(bLat)) * Math.sin(dLon / 2) ** 2;
  return 2 * R_NM * Math.asin(Math.min(1, Math.sqrt(h)));
}

/** The route as points with their distance from the origin. */
export function routePoints(r: ProfileRoute): RoutePoint[] {
  const raw = [
    { ident: r.origin.icao, lat: r.origin.lat, lon: r.origin.lon, alt: NaN },
    ...r.fixes
      // SimBrief lists the airports as fixes too; keep them once.
      .filter((f, i, a) => !(i === 0 && f.ident === r.origin.icao) && !(i === a.length - 1 && f.ident === r.dest.icao))
      .map((f) => ({ ident: f.ident, lat: f.lat, lon: f.lon, alt: f.alt_ft > 0 ? f.alt_ft : NaN })),
    { ident: r.dest.icao, lat: r.dest.lat, lon: r.dest.lon, alt: NaN },
  ];
  const out: RoutePoint[] = [];
  let d = 0;
  for (let i = 0; i < raw.length; i++) {
    if (i > 0) d += gcNm(raw[i - 1].lat, raw[i - 1].lon, raw[i].lat, raw[i].lon);
    out.push({ ...raw[i], d });
  }
  return out;
}

/** A point at distance `d` (nm from the origin) along the route. */
export function pointAt(pts: RoutePoint[], d: number): [number, number] {
  for (let i = 1; i < pts.length; i++) {
    if (d <= pts[i].d || i === pts.length - 1) {
      const a = pts[i - 1];
      const b = pts[i];
      const t = b.d > a.d ? Math.min(1, Math.max(0, (d - a.d) / (b.d - a.d))) : 0;
      return [a.lon + (b.lon - a.lon) * t, a.lat + (b.lat - a.lat) * t];
    }
  }
  return [pts[0].lon, pts[0].lat];
}

// --- Planned climb and descent --------------------------------------------------
// For a route without SimBrief's own altitudes (typed in or drawn), worked
// out from the cruise altitude with the usual rules of thumb:
//  - Climb: about 350 ft per nm below 10,000 ft, 250 ft per nm up to
//    FL240 and 180 ft per nm above (a jet climbs fewer feet per mile as it
//    gets higher and faster; a light aircraft rarely goes that high).
//  - Descent: the 3-to-1 rule, a 3° path (318 ft per nm) all the way to
//    the runway, with 5 nm level at 10,000 ft to slow to 250 kt when coming
//    down from above it.
// On a route too short to reach the cruise altitude, the climb and the
// descent simply meet.

/** Feet per nm of climb at an altitude. */
const climbGradient = (alt: number) => (alt < 10000 ? 350 : alt < 24000 ? 250 : 180);
const DESCENT_FT_NM = 318;
const SLOW_DOWN_NM = 5;

/** Altitude reached `d` nm after leaving `from` ft, climbing to `top`. */
export function climbAlt(from: number, top: number, d: number): number {
  let alt = from;
  let left = d;
  while (left > 0 && alt < top) {
    const g = climbGradient(alt);
    // Up to the next band edge (10,000 or 24,000) or the top.
    const edge = Math.min(top, alt < 10000 ? 10000 : alt < 24000 ? 24000 : top);
    const need = (edge - alt) / g;
    if (need >= left) return alt + left * g;
    alt = edge;
    left -= need;
  }
  return Math.min(alt, top);
}

/** Highest the aircraft may be `d` nm before landing at `to` ft, on the
 * way down from `top`. */
export function descentAlt(to: number, top: number, d: number): number {
  const slow = top > 11000 && to < 10000;
  const toTen = (10000 - to) / DESCENT_FT_NM;
  if (!slow || d <= toTen) return Math.min(top, to + d * DESCENT_FT_NM);
  if (d <= toTen + SLOW_DOWN_NM) return 10000;
  return Math.min(top, 10000 + (d - toTen - SLOW_DOWN_NM) * DESCENT_FT_NM);
}

/**
 * Planned altitude along the route as [distance, altitude] corners, from
 * the origin's to the destination's elevation. SimBrief's fixes carry its
 * own climb and descent; a route flown at one altitude throughout gets the
 * climb and descent above.
 */
export function plannedPath(pts: RoutePoint[], cruiseFt: number, origElev: number, destElev: number): [number, number][] {
  const total = pts[pts.length - 1].d;
  const alts = pts.slice(1, -1).map((p) => p.alt).filter((a) => !Number.isNaN(a));
  const cruise = cruiseFt > 0 ? cruiseFt : alts.length ? Math.max(...alts) : 0;
  if (cruise <= 0 || total <= 0) return [];
  const shaped = alts.length > 1 && new Set(alts.map((a) => Math.round(a / 100))).size > 1;
  if (shaped) {
    const out: [number, number][] = [[0, origElev]];
    for (const p of pts.slice(1, -1)) if (!Number.isNaN(p.alt)) out.push([p.d, p.alt]);
    out.push([total, destElev]);
    return out;
  }
  // Sampled finely enough to show every bend (a few hundred points).
  const n = 300;
  const out: [number, number][] = [];
  for (let i = 0; i <= n; i++) {
    const d = (total * i) / n;
    out.push([d, Math.min(cruise, climbAlt(origElev, cruise, d), descentAlt(destElev, cruise, total - d))]);
  }
  return out;
}

/** Planned altitude at distance d. */
export function altAt(path: [number, number][], d: number): number {
  if (!path.length) return NaN;
  if (d <= path[0][0]) return path[0][1];
  for (let i = 1; i < path.length; i++) {
    if (d <= path[i][0]) {
      const [x0, y0] = path[i - 1];
      const [x1, y1] = path[i];
      return x1 > x0 ? y0 + ((y1 - y0) * (d - x0)) / (x1 - x0) : y1;
    }
  }
  return path[path.length - 1][1];
}

/**
 * Where a position lies along the route: the distance from the origin of
 * the nearest point on it, and how far off the route it is (nm). Flat
 * geometry per leg, which is plenty for placing a symbol.
 */
export function project(pts: RoutePoint[], lat: number, lon: number): { d: number; off: number } {
  let best = { d: 0, off: Infinity };
  for (let i = 1; i < pts.length; i++) {
    const a = pts[i - 1];
    const b = pts[i];
    const k = Math.cos(rad((a.lat + b.lat) / 2)) * 60;
    const bx = (b.lon - a.lon) * k;
    const by = (b.lat - a.lat) * 60;
    const px = (lon - a.lon) * k;
    const py = (lat - a.lat) * 60;
    const len2 = bx * bx + by * by;
    const t = len2 > 0 ? Math.max(0, Math.min(1, (px * bx + py * by) / len2)) : 0;
    const off = Math.hypot(px - bx * t, py - by * t);
    if (off < best.off) best = { d: a.d + (b.d - a.d) * t, off };
  }
  return best;
}

// --- Ground -------------------------------------------------------------------

const DEM = "https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png";
const tiles = new Map<string, Promise<Uint8ClampedArray | null>>();

function tile(z: number, x: number, y: number): Promise<Uint8ClampedArray | null> {
  const key = `${z}/${x}/${y}`;
  let p = tiles.get(key);
  if (!p) {
    p = fetch(DEM.replace("{z}", String(z)).replace("{x}", String(x)).replace("{y}", String(y)))
      .then((r) => (r.ok ? r.blob() : Promise.reject(new Error(String(r.status)))))
      .then((b) => createImageBitmap(b))
      .then((img) => {
        const c = document.createElement("canvas");
        c.width = img.width;
        c.height = img.height;
        const g = c.getContext("2d", { willReadFrequently: true })!;
        g.drawImage(img, 0, 0);
        return g.getImageData(0, 0, img.width, img.height).data;
      })
      .catch(() => null);
    tiles.set(key, p);
    // Keep the cache small: a long route needs a few dozen tiles at most.
    if (tiles.size > 300) tiles.delete(tiles.keys().next().value!);
  }
  return p;
}

/** Ground elevation (ft, sea level for water) at each [lon, lat], from
 * tiles at zoom z; null where a tile can't be had. */
export async function groundFt(points: [number, number][], z: number): Promise<(number | null)[]> {
  const n = 2 ** z;
  return Promise.all(
    points.map(async ([lon, lat]) => {
      const fx = ((lon + 180) / 360) * n;
      const s = Math.sin(rad(Math.max(-85, Math.min(85, lat))));
      const fy = (0.5 - Math.log((1 + s) / (1 - s)) / (4 * Math.PI)) * n;
      const tx = Math.floor(fx);
      const ty = Math.floor(fy);
      const data = await tile(z, ((tx % n) + n) % n, Math.max(0, Math.min(n - 1, ty)));
      if (!data) return null;
      const px = Math.min(255, Math.floor((fx - tx) * 256));
      const py = Math.min(255, Math.floor((fy - ty) * 256));
      const o = (py * 256 + px) * 4;
      const m = data[o] * 256 + data[o + 1] + data[o + 2] / 256 - 32768;
      return Math.max(0, m) * 3.28084;
    }),
  );
}

/** A tile zoom coarse enough for the route's length: a few dozen tiles. */
export function groundZoom(totalNm: number): number {
  return totalNm <= 150 ? 10 : totalNm <= 600 ? 9 : totalNm <= 2000 ? 8 : 7;
}

// --- Obstacles ------------------------------------------------------------------

export interface ProfileObstacle {
  d: number;
  /** Top, ft above sea level, and height above the ground. */
  amsl: number;
  agl: number;
}

/** FAA obstacles within `widthNm` either side of the route. */
export function obstaclesAlong(pts: RoutePoint[], fc: GeoJSON.FeatureCollection, widthNm = 2): ProfileObstacle[] {
  const pad = widthNm / 60 + 0.01;
  const lats = pts.map((p) => p.lat);
  const lons = pts.map((p) => p.lon);
  const box = [Math.min(...lons) - pad * 2, Math.min(...lats) - pad, Math.max(...lons) + pad * 2, Math.max(...lats) + pad];
  const out: ProfileObstacle[] = [];
  for (const f of fc.features) {
    if (f.geometry.type !== "Point") continue;
    const [lon, lat] = f.geometry.coordinates;
    if (lon < box[0] || lon > box[2] || lat < box[1] || lat > box[3]) continue;
    const at = project(pts, lat, lon);
    if (at.off > widthNm) continue;
    const p = f.properties as { amsl?: number; agl?: number };
    if (p.amsl == null) continue;
    out.push({ d: at.d, amsl: p.amsl, agl: p.agl ?? 0 });
  }
  return out.sort((a, b) => a.d - b.d);
}

// --- Airspace -------------------------------------------------------------------

export interface ProfileAirspace {
  cls: "B" | "C" | "D";
  name: string;
  from: number;
  to: number;
  /** Floor and ceiling, ft above sea level (floor 0: the surface). */
  floor: number;
  ceiling: number;
}

/** Floor and ceiling from the chart label FlightDeck stores: "100/30",
 * "40/SFC", "[25]" (class D: surface to 2,500). */
export function parseLabel(cls: string, label: string): { floor: number; ceiling: number } | null {
  const hundreds = (s: string) => (s.trim().toUpperCase().startsWith("SFC") ? 0 : parseFloat(s) * 100);
  if (cls === "D") {
    const m = label.match(/\[(\d+)/);
    return m ? { floor: 0, ceiling: Number(m[1]) * 100 } : null;
  }
  const [hi, lo] = label.split("/");
  if (!hi) return null;
  const ceiling = hundreds(hi);
  const floor = lo ? hundreds(lo) : 0;
  return Number.isFinite(ceiling) && Number.isFinite(floor) ? { floor, ceiling } : null;
}

function inRing(ring: number[][], x: number, y: number): boolean {
  let inside = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const [xi, yi] = ring[i];
    const [xj, yj] = ring[j];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}
function inPolygon(rings: number[][][], x: number, y: number): boolean {
  if (!inRing(rings[0], x, y)) return false;
  for (let i = 1; i < rings.length; i++) if (inRing(rings[i], x, y)) return false;
  return true;
}

/** Where the route passes through each class B, C or D piece: sampled
 * every `step` nm along it. */
export function airspaceAlong(pts: RoutePoint[], fc: GeoJSON.FeatureCollection, step: number): ProfileAirspace[] {
  const total = pts[pts.length - 1].d;
  const samples: [number, number, number][] = [];
  for (let d = 0; d <= total + 1e-9; d += step) {
    const [lon, lat] = pointAt(pts, d);
    samples.push([d, lon, lat]);
  }
  const lons = samples.map((s) => s[1]);
  const lats = samples.map((s) => s[2]);
  const rb = [Math.min(...lons), Math.min(...lats), Math.max(...lons), Math.max(...lats)];
  const out: ProfileAirspace[] = [];
  for (const f of fc.features) {
    const g = f.geometry;
    if (g.type !== "Polygon" && g.type !== "MultiPolygon") continue;
    const p = f.properties as { c?: string; name?: string; label?: string };
    const cls = p.c;
    if (cls !== "B" && cls !== "C" && cls !== "D") continue;
    const polys = (g.type === "Polygon" ? [g.coordinates] : g.coordinates) as number[][][][];
    // Quick reject by bounding box.
    let bx0 = 180, by0 = 90, bx1 = -180, by1 = -90;
    for (const poly of polys) for (const [x, y] of poly[0]) {
      if (x < bx0) bx0 = x;
      if (x > bx1) bx1 = x;
      if (y < by0) by0 = y;
      if (y > by1) by1 = y;
    }
    if (bx1 < rb[0] || bx0 > rb[2] || by1 < rb[1] || by0 > rb[3]) continue;
    const alt = parseLabel(cls, p.label ?? "");
    if (!alt) continue;
    let start: number | null = null;
    for (let i = 0; i <= samples.length; i++) {
      const s = samples[i];
      const inside = !!s && s[1] >= bx0 && s[1] <= bx1 && s[2] >= by0 && s[2] <= by1 && polys.some((poly) => inPolygon(poly, s[1], s[2]));
      if (inside && start == null) start = s[0];
      if (!inside && start != null) {
        out.push({ cls, name: p.name ?? "", from: Math.max(0, start - step / 2), to: Math.min(total, samples[i - 1][0] + step / 2), ...alt });
        start = null;
      }
    }
  }
  // Big boxes first, so smaller ones draw on top of them.
  return out.sort((a, b) => b.ceiling - b.floor - (a.ceiling - a.floor));
}

// --- Airspace as one shape ---------------------------------------------------------

export interface Rect {
  x0: number;
  x1: number;
  /** Top and bottom on screen (y0 < y1). */
  y0: number;
  y1: number;
}

/**
 * Overlapping and touching boxes (class B shelves, say) as one shape: an SVG
 * fill path covering their union and an outline path around it only, with
 * no lines between the boxes. Also where each separate stretch of it starts,
 * for a label.
 */
export function unionShape(rects: Rect[]): { fill: string; edge: string; labels: { x: number; y: number }[] } {
  if (!rects.length) return { fill: "", edge: "", labels: [] };
  const uniq = (v: number[]) => [...new Set(v.map((n) => Math.round(n * 10) / 10))].sort((a, b) => a - b);
  const xs = uniq(rects.flatMap((r) => [r.x0, r.x1]));
  const ys = uniq(rects.flatMap((r) => [r.y0, r.y1]));
  const nx = xs.length - 1;
  const ny = ys.length - 1;
  const on: boolean[][] = [];
  for (let i = 0; i < nx; i++) {
    on.push([]);
    const cx = (xs[i] + xs[i + 1]) / 2;
    for (let j = 0; j < ny; j++) {
      const cy = (ys[j] + ys[j + 1]) / 2;
      on[i].push(rects.some((r) => cx > r.x0 && cx < r.x1 && cy > r.y0 && cy < r.y1));
    }
  }
  const at = (i: number, j: number) => i >= 0 && i < nx && j >= 0 && j < ny && on[i][j];
  // Fill: each column's covered runs, a hair wider so neighbours meet
  // without a seam (one path, so the overlap isn't darker).
  let fill = "";
  for (let i = 0; i < nx; i++) {
    for (let j = 0; j < ny; j++) {
      if (!on[i][j]) continue;
      let k = j;
      while (k + 1 < ny && on[i][k + 1]) k++;
      fill += `M${xs[i] - 0.3} ${ys[j]}H${xs[i + 1] + 0.3}V${ys[k + 1]}H${xs[i] - 0.3}Z`;
      j = k;
    }
  }
  // Outline: cell sides with the inside on one side only, joined into
  // straight runs.
  let edge = "";
  for (let i = 0; i <= nx; i++) {
    for (let j = 0; j < ny; j++) {
      if (at(i - 1, j) === at(i, j)) continue;
      let k = j;
      while (k + 1 < ny && at(i - 1, k + 1) !== at(i, k + 1)) k++;
      edge += `M${xs[i]} ${ys[j]}V${ys[k + 1]}`;
      j = k;
    }
  }
  for (let j = 0; j <= ny; j++) {
    for (let i = 0; i < nx; i++) {
      if (at(i, j - 1) === at(i, j)) continue;
      let k = i;
      while (k + 1 < nx && at(k + 1, j - 1) !== at(k + 1, j)) k++;
      edge += `M${xs[i]} ${ys[j]}H${xs[k + 1]}`;
      i = k;
    }
  }
  // A label at the top left of each separate stretch along the route.
  const labels: { x: number; y: number }[] = [];
  let inRun = false;
  for (let i = 0; i < nx; i++) {
    const top = on[i].indexOf(true);
    if (top >= 0 && !inRun) labels.push({ x: xs[i], y: ys[top] });
    inRun = top >= 0;
  }
  return { fill, edge, labels };
}

/** Initial great-circle bearing from a to b, degrees true. */
export function bearingDeg(aLat: number, aLon: number, bLat: number, bLon: number): number {
  const p1 = rad(aLat);
  const p2 = rad(bLat);
  const dl = rad(bLon - aLon);
  const y = Math.sin(dl) * Math.cos(p2);
  const x = Math.cos(p1) * Math.sin(p2) - Math.sin(p1) * Math.cos(p2) * Math.cos(dl);
  return ((Math.atan2(y, x) * 180) / Math.PI + 360) % 360;
}

/** The point `nm` from (lat, lon) on a true bearing: [lon, lat]. */
export function destinationNm(lat: number, lon: number, brgDeg: number, nm: number): [number, number] {
  const d = nm / R_NM;
  const b = rad(brgDeg);
  const p1 = rad(lat);
  const l1 = rad(lon);
  const p2 = Math.asin(Math.sin(p1) * Math.cos(d) + Math.cos(p1) * Math.sin(d) * Math.cos(b));
  const l2 = l1 + Math.atan2(Math.sin(b) * Math.sin(d) * Math.cos(p1), Math.cos(d) - Math.sin(p1) * Math.sin(p2));
  return [((((l2 * 180) / Math.PI) + 540) % 360) - 180, (p2 * 180) / Math.PI];
}

/** Smallest difference between two headings, degrees (0–180). */
export function headingGap(a: number, b: number): number {
  const d = Math.abs((((a - b) % 360) + 540) % 360 - 180);
  return d;
}

/** Feet per nm of a 3° glidepath. */
export const GLIDE_FT_NM = 6076.12 * Math.tan((3 * Math.PI) / 180);
/** Threshold crossing height, ft. */
export const TCH_FT = 50;
