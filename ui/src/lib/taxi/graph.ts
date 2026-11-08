// The taxi network of an airport, from the base map's OpenStreetMap
// aeroway data (taxiway and runway centrelines with their letters and
// numbers, gate points with their numbers). The map tiles cut every line
// at their edges (with some overlap), so the pieces are joined again here:
// points closer than a metre become one, lines are split wherever a point
// of another line sits on them or two lines cross, and pieces that then
// match are kept once. The result is a graph of named edges to route on.
//
// This is the OpenStreetMap fallback; the sim's own taxi network (MSFS
// facilities, X-Plane apt.dat) can feed the same graph later.

import { VectorTile } from "@mapbox/vector-tile";
import Pbf from "pbf";
import type maplibregl from "maplibre-gl";

export type LL = [number, number];
export type XY = [number, number];

/** A line or point from the aeroway data. */
export interface RawFeature {
  kind: "taxiway" | "runway" | "gate";
  ref: string;
  /** [lon, lat] points: two or more for a line, one for a gate. */
  coords: LL[];
}

export interface Edge {
  a: number;
  b: number;
  len: number;
  /** Taxiway letters ("GG") or runway numbers ("10L/28R"); "" when unnamed. */
  name: string;
  runway: boolean;
}

export interface Gate {
  ref: string;
  xy: XY;
}

export interface TaxiGraph {
  /** Local metres around this point. */
  origin: LL;
  nodes: XY[];
  edges: Edge[];
  /** Edge ids at each node. */
  adj: number[][];
  gates: Gate[];
  /** Taxiway names, and runway names, each with the edges carrying it. */
  byName: Map<string, number[]>;
  /** Runways touching each node (crossings), by node. */
  runwayAt: Map<number, Set<string>>;
}

const KX = 111_320;
const KY = 110_540;

export function toXY(origin: LL, p: LL): XY {
  return [(p[0] - origin[0]) * Math.cos((origin[1] * Math.PI) / 180) * KX, (p[1] - origin[1]) * KY];
}
export function toLL(origin: LL, p: XY): LL {
  return [origin[0] + p[0] / (Math.cos((origin[1] * Math.PI) / 180) * KX), origin[1] + p[1] / KY];
}

const tileCache = new Map<string, Promise<RawFeature[]>>();

function tileUrls(map: maplibregl.Map): { url: string; maxzoom: number } | null {
  const src = map.getSource("openmaptiles") as unknown as { tiles?: string[]; maxzoom?: number } | undefined;
  const url = src?.tiles?.[0];
  return url ? { url, maxzoom: Math.min(14, src?.maxzoom ?? 14) } : null;
}

function cleanRef(v: unknown): string {
  return String(v ?? "")
    .trim()
    .toUpperCase()
    .replace(/\s+/g, "");
}

async function fetchTile(url: string, z: number, x: number, y: number): Promise<RawFeature[]> {
  const res = await fetch(url.replace("{z}", String(z)).replace("{x}", String(x)).replace("{y}", String(y)));
  if (!res.ok) return [];
  const tile = new VectorTile(new Pbf(new Uint8Array(await res.arrayBuffer())));
  const layer = tile.layers["aeroway"];
  const out: RawFeature[] = [];
  if (!layer) return out;
  for (let i = 0; i < layer.length; i++) {
    const f = layer.feature(i);
    const cls = String(f.properties.class ?? "");
    const kind = cls === "taxiway" ? "taxiway" : cls === "runway" ? "runway" : cls === "gate" ? "gate" : null;
    if (!kind) continue;
    const g = f.toGeoJSON(x, y, z).geometry;
    const ref = cleanRef(f.properties.ref);
    if (kind === "gate") {
      if (g.type === "Point" && ref) out.push({ kind, ref, coords: [g.coordinates as LL] });
      continue;
    }
    const lines = g.type === "LineString" ? [g.coordinates as LL[]] : g.type === "MultiLineString" ? (g.coordinates as LL[][]) : [];
    for (const l of lines) if (l.length >= 2) out.push({ kind, ref, coords: l });
  }
  return out;
}

/** The aeroway lines and gates within `radiusM` of a point, from the map tiles. */
export async function loadAeroways(map: maplibregl.Map, at: LL, radiusM = 3500): Promise<RawFeature[]> {
  const mock = (globalThis as { __fdTaxiLayout?: (at: LL) => RawFeature[] | null }).__fdTaxiLayout?.(at);
  if (mock) return mock;
  const src = tileUrls(map);
  if (!src) return [];
  const z = src.maxzoom;
  const n = 2 ** z;
  const dLat = radiusM / KY;
  const dLon = radiusM / (Math.cos((at[1] * Math.PI) / 180) * KX);
  const tx = (lon: number) => Math.floor(((lon + 180) / 360) * n);
  const ty = (lat: number) => {
    const r = (lat * Math.PI) / 180;
    return Math.floor(((1 - Math.log(Math.tan(r) + 1 / Math.cos(r)) / Math.PI) / 2) * n);
  };
  const jobs: Promise<RawFeature[]>[] = [];
  for (let x = tx(at[0] - dLon); x <= tx(at[0] + dLon); x++)
    for (let y = ty(at[1] + dLat); y <= ty(at[1] - dLat); y++) {
      const key = `${src.url}|${z}/${x}/${y}`;
      let p = tileCache.get(key);
      if (!p) {
        p = fetchTile(src.url, z, x, y).catch(() => {
          tileCache.delete(key);
          return [];
        });
        tileCache.set(key, p);
      }
      jobs.push(p);
    }
  return (await Promise.all(jobs)).flat();
}

/** The graph, built in a worker when the browser has them. */
export function buildGraphAsync(raw: RawFeature[], origin: LL): Promise<TaxiGraph> {
  if (typeof Worker === "undefined") return Promise.resolve(buildGraph(raw, origin));
  return new Promise((resolve) => {
    let w: Worker;
    try {
      w = new Worker(new URL("./graph.worker.ts", import.meta.url), { type: "module" });
    } catch {
      resolve(buildGraph(raw, origin));
      return;
    }
    w.onmessage = (e: MessageEvent<TaxiGraph>) => {
      w.terminate();
      resolve(e.data);
    };
    w.onerror = () => {
      w.terminate();
      resolve(buildGraph(raw, origin));
    };
    w.postMessage({ raw, origin });
  });
}

/** Joins the pieces into one graph (see the top of this file). */
export function buildGraph(raw: RawFeature[], origin: LL): TaxiGraph {
  const SNAP = 1.0;
  const nodes: XY[] = [];
  const cells = new Map<number, number[]>();
  const ck = (cx: number, cy: number) => (cx + 1_000_000) * 2_000_000 + (cy + 1_000_000);
  function node(p: XY): number {
    const cx = Math.floor(p[0] / 2);
    const cy = Math.floor(p[1] / 2);
    for (let i = -1; i <= 1; i++)
      for (let j = -1; j <= 1; j++) {
        const c = cells.get(ck(cx + i, cy + j));
        if (c)
          for (const id of c) {
            const q = nodes[id];
            const dx = q[0] - p[0];
            const dy = q[1] - p[1];
            if (dx * dx + dy * dy <= SNAP * SNAP) return id;
          }
      }
    const id = nodes.length;
    nodes.push(p);
    const k = ck(cx, cy);
    const c = cells.get(k);
    if (c) c.push(id);
    else cells.set(k, [id]);
    return id;
  }

  type Seg = { a: number; b: number; name: string; runway: boolean; cuts: number[] };
  const segs: Seg[] = [];
  const gates: Gate[] = [];
  const seenGate = new Set<string>();
  for (const f of raw) {
    if (f.kind === "gate") {
      const xy = toXY(origin, f.coords[0]);
      const k = `${f.ref}@${Math.round(xy[0] / 5)},${Math.round(xy[1] / 5)}`;
      if (!seenGate.has(k)) {
        seenGate.add(k);
        gates.push({ ref: f.ref, xy });
      }
      continue;
    }
    let prev = node(toXY(origin, f.coords[0]));
    for (let i = 1; i < f.coords.length; i++) {
      const cur = node(toXY(origin, f.coords[i]));
      if (cur !== prev) segs.push({ a: prev, b: cur, name: f.ref, runway: f.kind === "runway", cuts: [] });
      prev = cur;
    }
  }

  // Segments in 50 m cells, to find what touches what.
  const G = 50;
  const segCells = new Map<number, number[]>();
  const box = segs.map((s) => {
    const p = nodes[s.a];
    const q = nodes[s.b];
    return [Math.min(p[0], q[0]) - SNAP, Math.min(p[1], q[1]) - SNAP, Math.max(p[0], q[0]) + SNAP, Math.max(p[1], q[1]) + SNAP];
  });
  const cellKey = (x: number, y: number) => (x + 50_000) * 100_000 + (y + 50_000);
  segs.forEach((_, i) => {
    const [x0, y0, x1, y1] = box[i];
    for (let x = Math.floor(x0 / G); x <= Math.floor(x1 / G); x++)
      for (let y = Math.floor(y0 / G); y <= Math.floor(y1 / G); y++) {
        const k = cellKey(x, y);
        const c = segCells.get(k);
        if (c) c.push(i);
        else segCells.set(k, [i]);
      }
  });

  // Points lying on another segment, and crossings.
  const cut = (s: Seg, id: number) => {
    if (id !== s.a && id !== s.b && !s.cuts.includes(id)) s.cuts.push(id);
  };
  for (const [ck0, list] of segCells) {
    for (let i = 0; i < list.length; i++)
      for (let j = i + 1; j < list.length; j++) {
        const si = list[i];
        const sj = list[j];
        const bi = box[si];
        const bj = box[sj];
        if (bi[0] > bj[2] || bj[0] > bi[2] || bi[1] > bj[3] || bj[1] > bi[3]) continue;
        // Each pair once: in the cell holding the corner of their overlap.
        if (cellKey(Math.floor(Math.max(bi[0], bj[0]) / G), Math.floor(Math.max(bi[1], bj[1]) / G)) !== ck0) continue;
        const s = segs[si];
        const t = segs[sj];
        // Ends of one on the other.
        for (const [x, y] of [
          [s, t],
          [t, s],
        ] as const)
          for (const id of [y.a, y.b]) if (onSeg(nodes[id], nodes[x.a], nodes[x.b], SNAP)) cut(x, id);
        const hit = crossing(nodes[s.a], nodes[s.b], nodes[t.a], nodes[t.b]);
        if (hit) {
          const id = node(hit);
          cut(s, id);
          cut(t, id);
        }
      }
  }

  // Split at the cuts; keep each piece once.
  const edges: Edge[] = [];
  const seen = new Set<number>();
  const nameIds = new Map<string, number>();
  const nameId = (nm: string, rw: boolean) => {
    const k = (rw ? "R" : "T") + nm;
    let v = nameIds.get(k);
    if (v == null) nameIds.set(k, (v = nameIds.size));
    return v;
  };
  for (const s of segs) {
    const p = nodes[s.a];
    const q = nodes[s.b];
    const dx = q[0] - p[0];
    const dy = q[1] - p[1];
    const l2 = dx * dx + dy * dy || 1;
    const pts = [s.a, ...s.cuts.sort((m, n) => ((nodes[m][0] - p[0]) * dx + (nodes[m][1] - p[1]) * dy) / l2 - ((nodes[n][0] - p[0]) * dx + (nodes[n][1] - p[1]) * dy) / l2), s.b];
    for (let i = 1; i < pts.length; i++) {
      const a = pts[i - 1];
      const b = pts[i];
      if (a === b) continue;
      const k = (Math.min(a, b) * nodes.length + Math.max(a, b)) * 4096 + nameId(s.name, s.runway);
      if (seen.has(k)) continue;
      seen.add(k);
      edges.push({ a, b, len: Math.hypot(nodes[b][0] - nodes[a][0], nodes[b][1] - nodes[a][1]), name: s.name, runway: s.runway });
    }
  }

  const adj: number[][] = nodes.map(() => []);
  const byName = new Map<string, number[]>();
  const runwayAt = new Map<number, Set<string>>();
  edges.forEach((e, i) => {
    adj[e.a].push(i);
    adj[e.b].push(i);
    if (e.name) {
      const l = byName.get(e.name);
      if (l) l.push(i);
      else byName.set(e.name, [i]);
    }
    if (e.runway && e.name)
      for (const n of [e.a, e.b]) {
        const s = runwayAt.get(n) ?? new Set<string>();
        s.add(e.name);
        runwayAt.set(n, s);
      }
  });
  // A runway crossing only counts where a taxiway meets the runway.
  for (const [n] of runwayAt) if (!adj[n].some((i) => !edges[i].runway)) runwayAt.delete(n);
  return { origin, nodes, edges, adj, gates, byName, runwayAt };
}

function onSeg(p: XY, a: XY, b: XY, tol: number): boolean {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const l2 = dx * dx + dy * dy;
  if (l2 === 0) return false;
  const t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2;
  if (t <= 0 || t >= 1) return false;
  return Math.hypot(a[0] + t * dx - p[0], a[1] + t * dy - p[1]) <= tol;
}

function crossing(a: XY, b: XY, c: XY, d: XY): XY | null {
  const r = [b[0] - a[0], b[1] - a[1]];
  const s = [d[0] - c[0], d[1] - c[1]];
  const den = r[0] * s[1] - r[1] * s[0];
  if (Math.abs(den) < 1e-9) return null;
  const t = ((c[0] - a[0]) * s[1] - (c[1] - a[1]) * s[0]) / den;
  const u = ((c[0] - a[0]) * r[1] - (c[1] - a[1]) * r[0]) / den;
  const e = 1e-6;
  if (t <= e || t >= 1 - e || u <= e || u >= 1 - e) return null;
  return [a[0] + t * r[0], a[1] + t * r[1]];
}

/** Nearest point on an edge to p: the edge, the point and how far along (0–1). */
export function nearestEdge(g: TaxiGraph, p: XY, maxM = 120, filter?: (e: Edge) => boolean): { edge: number; t: number; at: XY; d: number } | null {
  let best: { edge: number; t: number; at: XY; d: number } | null = null;
  g.edges.forEach((e, i) => {
    if (filter && !filter(e)) return;
    const a = g.nodes[e.a];
    const b = g.nodes[e.b];
    const dx = b[0] - a[0];
    const dy = b[1] - a[1];
    const l2 = dx * dx + dy * dy || 1;
    const t = Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2));
    const at: XY = [a[0] + t * dx, a[1] + t * dy];
    const d = Math.hypot(at[0] - p[0], at[1] - p[1]);
    if (d <= maxM && (!best || d < best.d)) best = { edge: i, t, at, d };
  });
  return best;
}

/** Taxiways and runways meeting the named one, each with where they meet. */
export function connected(g: TaxiGraph, name: string): Map<string, XY> {
  const out = new Map<string, XY>();
  for (const ei of g.byName.get(name) ?? [])
    for (const n of [g.edges[ei].a, g.edges[ei].b])
      for (const j of g.adj[n]) {
        const o = g.edges[j].name;
        if (o && o !== name && !out.has(o)) out.set(o, g.nodes[n]);
      }
  return out;
}

/** Distance from p to the nearest edge with this name. */
export function distTo(g: TaxiGraph, name: string, p: XY): number {
  let best = Infinity;
  for (const ei of g.byName.get(name) ?? []) {
    const e = g.edges[ei];
    const a = g.nodes[e.a];
    const b = g.nodes[e.b];
    const dx = b[0] - a[0];
    const dy = b[1] - a[1];
    const l2 = dx * dx + dy * dy || 1;
    const t = Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2));
    best = Math.min(best, Math.hypot(a[0] + t * dx - p[0], a[1] + t * dy - p[1]));
  }
  return best;
}

export function isRunwayName(g: TaxiGraph, name: string): boolean {
  const e = g.byName.get(name)?.[0];
  return e != null && g.edges[e].runway;
}
