// Airspace along a route, whatever map layers are on: the US class
// airspace and special-use areas (FAA) and the rest of the world
// (openAIP), loaded as needed, and every area the route passes through
// with where (nm from the origin) and at what planned altitude. Used by
// the flight profile, the planner's conflict warning and the map's glow
// on crossed areas.

import { api } from "./api";
import { altAt, plannedPath, pointAt, routePoints, type ProfileRoute, type RoutePoint } from "./profile/build";

export interface Crossing {
  /** Unique per area. */
  id: string;
  /** Kind: "B", "C", "D", "E", "CTR", "P", "R", "DNG", "MOA", ... */
  c: string;
  name: string;
  /** Floor and ceiling, ft, and as charted ("GND", "FL180"). */
  lo: number;
  hi: number;
  loText: string;
  hiText: string;
  /** Along the route, nm from the origin. */
  from: number;
  to: number;
  /** The planned altitude is inside the area somewhere along the crossing. */
  inside: boolean;
  /** A special-use area (prohibited, restricted, danger, MOA, warning,
   * military...) the route passes through, at whatever altitude. */
  conflict: boolean;
  /** Where the planned altitude is: inside the area's limits somewhere
   * along the crossing, or wholly below or above them. */
  vert: "in" | "below" | "above";
  /** Planned altitude where the route is first inside it vertically (or at its middle). */
  alt: number;
  feature: GeoJSON.Feature;
}

/** The areas a pilot must know a route crosses: prohibited, restricted,
 * danger, MOAs, warning, national security, temporary reserved or
 * segregated and military areas. */
export const CONFLICT_KINDS = new Set(["P", "R", "DNG", "MOA", "W", "NSA", "TRA", "TSA", "MIL"]);
/** Left out: class G (everything else) and airways. */
const SKIP = new Set(["G"]);

let usFc: GeoJSON.Feature[] | null = null;
let usBusy: Promise<GeoJSON.Feature[]> | null = null;

/** The FAA's areas (class airspace and special use), loaded once. */
function usAreas(): Promise<GeoJSON.Feature[]> {
  if (usFc) return Promise.resolve(usFc);
  usBusy ??= (async () => {
    const out: GeoJSON.Feature[] = [];
    for (const get of [api.usAirspace, api.suaAirspace]) {
      try {
        const fc = JSON.parse(await get()) as GeoJSON.FeatureCollection;
        out.push(...fc.features);
      } catch {
        /* not reachable: the other one may be */
      }
    }
    usFc = out;
    return out;
  })();
  return usBusy;
}

/** openAIP's areas in a box (a few countries read per call, so ask again while more come). */
async function worldAreas(w: number, s: number, e: number, n: number): Promise<GeoJSON.Feature[]> {
  let got: GeoJSON.Feature[] = [];
  for (let i = 0; i < 8; i++) {
    const r = await api.worldAirspace(w, s, e, n).catch(() => null);
    if (!r) break;
    got = (JSON.parse(r.geojson) as GeoJSON.FeatureCollection).features;
    if (!r.more) break;
  }
  return got;
}

/** Every area near the route: the FAA's and openAIP's, airways left out. */
export async function areasNear(pts: RoutePoint[]): Promise<GeoJSON.Feature[]> {
  if (pts.length < 2) return [];
  const step = Math.max(0.5, pts[pts.length - 1].d / 400);
  let [w, s, e, n] = [180, 90, -180, -90];
  for (let d = 0; d <= pts[pts.length - 1].d; d += step) {
    const [x, y] = pointAt(pts, d);
    w = Math.min(w, x);
    e = Math.max(e, x);
    s = Math.min(s, y);
    n = Math.max(n, y);
  }
  const [us, world] = await Promise.all([usAreas(), worldAreas(w - 0.3, s - 0.3, e + 0.3, n + 0.3)]);
  return [...us, ...world].filter((f) => !(f.properties as { aw?: boolean }).aw);
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

/** The planned altitude along the route ([nm, ft] corners), as the profile draws it. */
export function plannedAlt(route: ProfileRoute, pts: RoutePoint[]): [number, number][] {
  return plannedPath(pts, route.cruise_ft, 0, 0);
}

/** Where the route passes through each area, sampled about every `step` nm. */
export function crossingsAlong(route: ProfileRoute, pts: RoutePoint[], areas: GeoJSON.Feature[], step?: number): Crossing[] {
  if (pts.length < 2) return [];
  const total = pts[pts.length - 1].d;
  const st = step ?? Math.max(0.2, total / 600);
  const path = plannedAlt(route, pts);
  const samples: [number, number, number, number][] = [];
  for (let d = 0; d <= total + 1e-9; d += st) {
    const [lon, lat] = pointAt(pts, d);
    samples.push([d, lon, lat, path.length ? altAt(path, d) : route.cruise_ft]);
  }
  const lons = samples.map((x) => x[1]);
  const lats = samples.map((x) => x[2]);
  const rb = [Math.min(...lons), Math.min(...lats), Math.max(...lons), Math.max(...lats)];
  const out: Crossing[] = [];
  const seen = new Set<string>();
  for (const f of areas) {
    const g = f.geometry;
    if (!g || (g.type !== "Polygon" && g.type !== "MultiPolygon")) continue;
    const p = f.properties as { c?: string; name?: string; lo?: number; hi?: number; lo_t?: string; hi_t?: string };
    const c = p.c ?? "";
    if (!c || SKIP.has(c)) continue;
    const polys = (g.type === "Polygon" ? [g.coordinates] : g.coordinates) as number[][][][];
    let bx0 = 180, by0 = 90, bx1 = -180, by1 = -90;
    for (const poly of polys)
      for (const [x, y] of poly[0]) {
        if (x < bx0) bx0 = x;
        if (x > bx1) bx1 = x;
        if (y < by0) by0 = y;
        if (y > by1) by1 = y;
      }
    if (bx1 < rb[0] || bx0 > rb[2] || by1 < rb[1] || by0 > rb[3]) continue;
    const lo = Number(p.lo ?? 0);
    const hi = Number(p.hi ?? lo + 1000);
    const id = `${c}|${p.name ?? ""}|${lo}|${hi}|${bx0.toFixed(3)},${by0.toFixed(3)}`;
    if (seen.has(id)) continue;
    let start: number | null = null;
    let inside = false;
    let below = false;
    let alt = NaN;
    let midAlt = NaN;
    for (let i = 0; i <= samples.length; i++) {
      const sm = samples[i];
      const isIn = !!sm && sm[1] >= bx0 && sm[1] <= bx1 && sm[2] >= by0 && sm[2] <= by1 && polys.some((poly) => inPolygon(poly, sm[1], sm[2]));
      if (isIn) {
        if (start == null) start = sm[0];
        if (Number.isNaN(midAlt)) midAlt = sm[3];
        if (sm[3] < lo) below = true;
        if (sm[3] >= lo && sm[3] <= hi && !inside) {
          inside = true;
          alt = sm[3];
        }
      }
      if (!isIn && start != null) {
        seen.add(id);
        const from = Math.max(0, start - st / 2);
        const to = Math.min(total, samples[i - 1][0] + st / 2);
        const prev = out.find((o) => o.id === id);
        if (prev) {
          // The same area entered again: one entry from first in to last out.
          prev.to = to;
          prev.inside ||= inside;
          prev.vert = prev.inside ? "in" : prev.vert;
        } else
          out.push({
            id,
            c,
            name: p.name ?? "",
            lo,
            hi,
            loText: p.lo_t ?? "",
            hiText: p.hi_t ?? "",
            from,
            to,
            inside,
            conflict: CONFLICT_KINDS.has(c),
            vert: inside ? "in" : below ? "below" : "above",
            alt: Number.isNaN(alt) ? midAlt : alt,
            feature: f,
          });
        start = null;
      }
    }
  }
  return out.sort((a, b) => a.from - b.from);
}

/** The crossings of the route being planned (or flown), for the planner's
 * warning and the map's glow; `focus`: conflicts the pilot asked to see. */
export const routeAsp = $state<{ key: string; list: Crossing[]; pts: RoutePoint[]; focus: number; busy: boolean }>({ key: "", list: [], pts: [], focus: 0, busy: false });

/** Works out the crossings for a route (again only when it changes). */
export async function updateRouteAirspace(route: ProfileRoute | null) {
  const key = route
    ? [route.origin, ...route.fixes, route.dest].map((p) => `${p.lat.toFixed(4)},${p.lon.toFixed(4)}`).join("|") + `@${route.cruise_ft}`
    : "";
  if (key === routeAsp.key) return;
  routeAsp.key = key;
  if (!route) {
    routeAsp.list = [];
    routeAsp.pts = [];
    return;
  }
  routeAsp.busy = true;
  const pts = routePoints(route);
  const areas = await areasNear(pts);
  if (routeAsp.key !== key) return;
  routeAsp.pts = pts;
  routeAsp.list = crossingsAlong(route, pts, areas);
  routeAsp.busy = false;
}
