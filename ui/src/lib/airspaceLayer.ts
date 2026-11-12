// The Airspace layer: US Class B, C and D and special use airspace (FAA),
// and the rest of the world from openAIP. Two sources with the same layers:
// "us-airspace" (read once a cycle) and "world-airspace" (the countries in
// view). Lines as charts draw them, labels along the edge in a small box
// ("G: GND-2000AGL"), and in 3D each area stands up from its floor to its
// ceiling as a see-through block.

import type maplibregl from "maplibre-gl";

/** Kinds and how they are drawn: colour, and solid or dashed edge. */
export const ASP_KINDS: Record<string, { color: string; dash?: boolean; name: string }> = {
  A: { color: "#9fb0c4", name: "Class A" },
  B: { color: "#4a8ce8", name: "Class B" },
  C: { color: "#c25bb0", name: "Class C" },
  D: { color: "#4a8ce8", dash: true, name: "Class D" },
  E: { color: "#c25bb0", dash: true, name: "Class E" },
  F: { color: "#7fae6a", dash: true, name: "Class F" },
  G: { color: "#7fae6a", dash: true, name: "Class G" },
  CTR: { color: "#4a8ce8", dash: true, name: "Control zone" },
  P: { color: "#ff4f6d", name: "Prohibited" },
  R: { color: "#ff4f6d", name: "Restricted" },
  DNG: { color: "#ff8a3d", name: "Danger" },
  W: { color: "#ff4f6d", dash: true, name: "Warning" },
  AL: { color: "#f2b13d", dash: true, name: "Alert" },
  MOA: { color: "#b06ad9", dash: true, name: "MOA" },
  NSA: { color: "#ff4f6d", dash: true, name: "National security" },
  TRA: { color: "#ff8a3d", dash: true, name: "Temporary reserved" },
  TSA: { color: "#ff8a3d", dash: true, name: "Temporary segregated" },
  MIL: { color: "#ff8a3d", dash: true, name: "Military" },
};

const KINDS = Object.keys(ASP_KINDS);
/** An area's colour by its kind, as a map expression. */
export const ASP_COLOR: unknown = ["match", ["get", "c"], ...Object.keys(ASP_KINDS).flatMap((k) => [k, ASP_KINDS[k].color]), "#4a8ce8"];
const color = ["match", ["get", "c"], ...KINDS.flatMap((k) => [k, ASP_KINDS[k].color]), "#4a8ce8"] as unknown as maplibregl.ExpressionSpecification;
const DASHED = KINDS.filter((k) => ASP_KINDS[k].dash);
const isDashed = ["in", ["get", "c"], ["literal", DASHED]] as unknown as maplibregl.FilterSpecification;
const isSolid = ["!", ["in", ["get", "c"], ["literal", DASHED]]] as unknown as maplibregl.FilterSpecification;

export const SOURCES = ["us-airspace", "world-airspace"] as const;
const baseFill: unknown = ["match", ["get", "c"], ["P", "R"], 0.08, ["D", "E", "G", "F", "CTR"], 0.035, 0.055];
const ids = (src: string) => ({ fill: `${src}-fill`, line: `${src}-line`, dash: `${src}-dash`, block: `${src}-3d` });

export function addAirspaceLayers(map: maplibregl.Map, three: boolean) {
  const empty: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
  for (const src of SOURCES) {
    if (map.getSource(src)) continue;
    map.addSource(src, { type: "geojson", data: empty, tolerance: 0.5 });
    const L = ids(src);
    map.addLayer({
      id: L.fill,
      type: "fill",
      source: src,
      minzoom: 5,
      layout: { visibility: "none" },
      paint: { "fill-color": color, "fill-opacity": baseFill as maplibregl.ExpressionSpecification },
    });
    map.addLayer({
      id: L.block,
      type: "fill-extrusion",
      source: src,
      minzoom: 5,
      layout: { visibility: "none" },
      paint: {
        "fill-extrusion-color": color,
        "fill-extrusion-base": ["*", ["min", ["get", "lo"], 45000], 0.3048] as unknown as maplibregl.ExpressionSpecification,
        "fill-extrusion-height": ["*", ["min", ["get", "hi"], 45000], 0.3048] as unknown as maplibregl.ExpressionSpecification,
        "fill-extrusion-opacity": 0.16,
      },
    });
    map.addLayer({
      id: L.line,
      type: "line",
      source: src,
      minzoom: 5,
      filter: isSolid,
      layout: { visibility: "none", "line-join": "round" },
      paint: { "line-color": color, "line-width": ["interpolate", ["linear"], ["zoom"], 5, 1, 10, ["match", ["get", "c"], ["B", "P", "R"], 2.4, 1.8]] as unknown as maplibregl.ExpressionSpecification, "line-opacity": 0.9 },
    });
    map.addLayer({
      id: L.dash,
      type: "line",
      source: src,
      minzoom: 6,
      filter: isDashed,
      layout: { visibility: "none" },
      paint: { "line-color": color, "line-width": 1.5, "line-dasharray": [3, 2], "line-opacity": 0.9 },
    });
  }
  // Overlays made for the view: labels inside each area's own edge, and in
  // 3D a thin light rim at each block's floor and ceiling.
  if (!map.getSource("asp-over")) {
    // A wide buffer: a label line crossing a tile edge is whole in both tiles.
    map.addSource("asp-over", { type: "geojson", data: empty, buffer: 512, tolerance: 0.2 });
    const rim = (id: string, base: unknown, top: unknown) =>
      map.addLayer({
        id,
        type: "fill-extrusion",
        source: "asp-over",
        filter: ["==", ["get", "t"], "rim"],
        layout: { visibility: "none" },
        paint: { "fill-extrusion-color": rimColor, "fill-extrusion-base": base as maplibregl.ExpressionSpecification, "fill-extrusion-height": top as maplibregl.ExpressionSpecification, "fill-extrusion-opacity": 0.75 },
      });
    rim("asp-rim-top", ["get", "hb"], ["get", "ht"]);
    rim("asp-rim-low", ["get", "lb"], ["get", "lt"]);
    // Altitude boxes as VFR charts print them: ceiling over floor, a line
    // between, in a box of the area's colour.
    for (const k of KINDS) {
      const id = `asp-alt-${k}`;
      if (!map.hasImage(id)) {
        const b = altBox(ASP_KINDS[k].color);
        map.addImage(id, b.img, b.opts);
      }
    }
    if (map.getStyle().glyphs)
      map.addLayer({
        id: "asp-label",
        type: "symbol",
        source: "asp-over",
        filter: ["==", ["get", "t"], "label"],
        layout: {
          visibility: "none",
          "text-field": ["get", "label"],
          "text-font": ["Noto Sans Bold"],
          "text-size": 11,
          "text-line-height": 1.35,
          "text-letter-spacing": 0.02,
          "text-padding": 4,
          "icon-image": ["concat", "asp-alt-", ["get", "c"]],
          "icon-text-fit": "both",
          "icon-text-fit-padding": [3, 6, 3, 6],
        },
        paint: { "text-color": "#ffffff" },
      });
  }
  showAirspace(map, false, three);
}

/** A box in the area's colour with a thin line across its middle. */
function altBox(c: string) {
  const W = 24;
  const H = 40;
  const cv = document.createElement("canvas");
  cv.width = W;
  cv.height = H;
  const g = cv.getContext("2d")!;
  const r = 5;
  g.beginPath();
  g.moveTo(r, 0);
  g.arcTo(W, 0, W, H, r);
  g.arcTo(W, H, 0, H, r);
  g.arcTo(0, H, 0, 0, r);
  g.arcTo(0, 0, W, 0, r);
  g.closePath();
  g.fillStyle = c;
  g.globalAlpha = 0.92;
  g.fill();
  g.globalAlpha = 1;
  g.fillStyle = "rgba(255,255,255,0.8)";
  g.fillRect(5, H / 2 - 1, W - 10, 2);
  return {
    img: g.getImageData(0, 0, W, H),
    opts: { pixelRatio: 2, stretchX: [[8, 16]] as [number, number][], stretchY: [[6, 17], [23, 34]] as [number, number][], content: [6, 5, 18, 35] as [number, number, number, number] },
  };
}

/** "10000MSL" → "100", "GND" → "SFC", "2000AGL" → "20 AGL", "FL180" stays. */
export function chartAlt(t: string): string {
  if (!t) return "";
  if (t === "GND") return "SFC";
  if (t === "UNL" || t.startsWith("FL")) return t;
  const m = /^(\d+)(MSL|AGL)$/.exec(t);
  if (!m) return t;
  const h = Math.round(Number(m[1]) / 100);
  return m[2] === "AGL" ? `${h} AGL` : String(h);
}

/** Fill only the areas the aircraft is in by height (2D); all when there is none. */
export function fillForAltitude(map: maplibregl.Map, alt: number | null) {
  for (const src of SOURCES) {
    const id = ids(src).fill;
    if (!map.getLayer(id)) continue;
    map.setPaintProperty(id, "fill-opacity", (alt == null ? baseFill : ["case", ["all", [">=", alt, ["get", "lo"]], ["<=", alt, ["get", "hi"]]], baseFill, 0]) as maplibregl.ExpressionSpecification);
  }
}

const rimColor = ["match", ["get", "c"], ...KINDS.flatMap((k) => [k, lighten(ASP_KINDS[k].color)]), "#cfe0ff"] as unknown as maplibregl.ExpressionSpecification;
function lighten(hex: string): string {
  const n = parseInt(hex.slice(1), 16);
  const m = (v: number) => Math.round(v + (255 - v) * 0.55);
  return `rgb(${m(n >> 16)},${m((n >> 8) & 255)},${m(n & 255)})`;
}

/** Lines and labels when on; in 3D the blocks too, instead of the flat tint. */
export function showAirspace(map: maplibregl.Map, on: boolean, three: boolean) {
  for (const src of SOURCES) {
    const L = ids(src);
    const vis = (id: string, v: boolean) => map.getLayer(id) && map.setLayoutProperty(id, "visibility", v ? "visible" : "none");
    vis(L.fill, on && !three);
    vis(L.block, on && three);
    vis(L.line, on);
    vis(L.dash, on);
  }
  const vis = (id: string, v: boolean) => map.getLayer(id) && map.setLayoutProperty(id, "visibility", v ? "visible" : "none");
  vis("asp-label", on);
  for (const id of ["asp-rim-top", "asp-rim-low"]) vis(id, on && three);
}

let vScale = 1;
/** The 3D blocks at the same height scale as the flight's wall. */
export function scaleAirspace(map: maplibregl.Map, k: number) {
  vScale = k;
  for (const src of SOURCES) {
    const id = ids(src).block;
    if (!map.getLayer(id)) continue;
    map.setPaintProperty(id, "fill-extrusion-base", ["*", ["min", ["get", "lo"], 45000], 0.3048 * k]);
    map.setPaintProperty(id, "fill-extrusion-height", ["*", ["min", ["get", "hi"], 45000], 0.3048 * k]);
  }
}

// --- Labels and rims for the view -------------------------------------------

type P = [number, number];
const bboxes = new WeakMap<GeoJSON.Feature, [number, number, number, number]>();
function bbox(f: GeoJSON.Feature): [number, number, number, number] {
  let b = bboxes.get(f);
  if (b) return b;
  b = [180, 90, -180, -90];
  const walk = (c: unknown): void => {
    if (Array.isArray(c) && typeof c[0] === "number") {
      const [x, y] = c as number[];
      b![0] = Math.min(b![0], x);
      b![1] = Math.min(b![1], y);
      b![2] = Math.max(b![2], x);
      b![3] = Math.max(b![3], y);
    } else if (Array.isArray(c)) c.forEach(walk);
  };
  walk((f.geometry as GeoJSON.Polygon).coordinates);
  bboxes.set(f, b);
  return b;
}

function polygons(f: GeoJSON.Feature): number[][][][] {
  const g = f.geometry;
  if (g.type === "Polygon") return [g.coordinates];
  if (g.type === "MultiPolygon") return g.coordinates;
  return [];
}


/**
 * The point inside a polygon (screen rings, holes after the first) furthest
 * from its edges, kept within the view; null if no spot is at least `need`
 * pixels from every edge.
 */
function inside(rings: P[][], W: number, H: number, need: number): P | null {
  const outer = rings[0];
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const [x, y] of outer) {
    x0 = Math.min(x0, x);
    y0 = Math.min(y0, y);
    x1 = Math.max(x1, x);
    y1 = Math.max(y1, y);
  }
  x0 = Math.max(x0, 30);
  y0 = Math.max(y0, 70);
  x1 = Math.min(x1, W - 30);
  y1 = Math.min(y1, H - 30);
  if (x1 - x0 < need * 2 || y1 - y0 < need * 1.2) return null;
  const isIn = (x: number, y: number) => {
    let c = false;
    for (const r of rings)
      for (let i = 0, j = r.length - 1; i < r.length; j = i++) {
        const [xi, yi] = r[i];
        const [xj, yj] = r[j];
        if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) c = !c;
      }
    return c;
  };
  const dist = (x: number, y: number) => {
    let d = Infinity;
    for (const r of rings)
      for (let i = 1; i < r.length; i++) {
        const [ax, ay] = r[i - 1];
        const dx = r[i][0] - ax;
        const dy = r[i][1] - ay;
        const l2 = dx * dx + dy * dy || 1;
        const t = Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / l2));
        d = Math.min(d, Math.hypot(x - ax - t * dx, y - ay - t * dy));
      }
    return d;
  };
  let best: P | null = null;
  let bd = need;
  const N = 9;
  for (let i = 0; i <= N; i++)
    for (let j = 0; j <= N; j++) {
      const x = x0 + ((x1 - x0) * (i + 0.5)) / (N + 1);
      const y = y0 + ((y1 - y0) * (j + 0.5)) / (N + 1);
      if (!isIn(x, y)) continue;
      const d = dist(x, y);
      if (d > bd) {
        bd = d;
        best = [x, y];
      }
    }
  return best;
}

/** Douglas-Peucker on screen points: the indices kept (tolerance in pixels). */
function simplifyIdx(pts: P[], tol: number): number[] {
  const keep = new Uint8Array(pts.length);
  keep[0] = keep[pts.length - 1] = 1;
  const stack: [number, number][] = [[0, pts.length - 1]];
  while (stack.length) {
    const [a, b] = stack.pop()!;
    const [ax, ay] = pts[a];
    const dx = pts[b][0] - ax;
    const dy = pts[b][1] - ay;
    const l = Math.hypot(dx, dy);
    let far = -1;
    let fd = tol;
    for (let i = a + 1; i < b; i++) {
      const d = l ? Math.abs((pts[i][0] - ax) * dy - (pts[i][1] - ay) * dx) / l : Math.hypot(pts[i][0] - ax, pts[i][1] - ay);
      if (d > fd) {
        fd = d;
        far = i;
      }
    }
    if (far > 0) {
      keep[far] = 1;
      stack.push([a, far], [far, b]);
    }
  }
  const out: number[] = [];
  keep.forEach((k, i) => k && out.push(i));
  return out;
}

/**
 * Made for the view: an altitude box inside each area (ceiling over floor,
 * as VFR charts print them), and in 3D thin rims at each block's floor and
 * ceiling.
 */
export function airspaceOverlay(map: maplibregl.Map, feats: GeoJSON.Feature[], three: boolean): GeoJSON.FeatureCollection {
  const out: GeoJSON.Feature[] = [];
  const z = map.getZoom();
  if (z < 7.5 || !feats.length) return { type: "FeatureCollection", features: out };
  const v = map.getBounds();
  const W = map.getCanvas().clientWidth;
  const H = map.getCanvas().clientHeight;
  const inView = feats.filter((f) => {
    const b = bbox(f);
    return b[2] >= v.getWest() && b[0] <= v.getEast() && b[3] >= v.getSouth() && b[1] <= v.getNorth();
  });
  const un = (p: P): [number, number] => {
    const l = map.unproject(p);
    return [l.lng, l.lat];
  };
  // One altitude box per area, at the point inside it furthest from its
  // edges (within the view), so it is clear which area it belongs to.
  for (const f of inView.slice(0, 800)) {
    const pr = f.properties ?? {};
    const hiT = chartAlt(String(pr.hi_t ?? ""));
    const loT = chartAlt(String(pr.lo_t ?? ""));
    if (!hiT && !loT) continue;
    const label = `${hiT || "?"}\n${loT || "?"}`;
    const boxW = Math.max(hiT.length, loT.length) * 7.5 + 16;
    for (const poly of polygons(f)) {
      const rings: P[][] = poly.map((ring) => {
        const pts: P[] = ring.map((c) => {
          const q = map.project(c as [number, number]);
          return [q.x, q.y];
        });
        return simplifyIdx(pts, 2).map((i) => pts[i]);
      });
      const at = inside(rings, W, H, Math.max(22, boxW / 2 + 4));
      if (at) out.push({ type: "Feature", properties: { t: "label", c: pr.c, label }, geometry: { type: "Point", coordinates: un(at) } });
    }
  }
  if (three) {
    // Rims: a thin band all round the edge at the floor and the ceiling.
    const k = vScale;
    for (const f of inView.slice(0, 400)) {
      const p = f.properties ?? {};
      const lo = Math.min(Number(p.lo) || 0, 45000) * 0.3048 * k;
      const hi = Math.min(Number(p.hi) || 0, 45000) * 0.3048 * k;
      const band = Math.max(25, (hi - lo) * 0.012);
      const quads: number[][][][] = [];
      for (const poly of polygons(f))
        for (const ring of poly) {
          const pts: P[] = ring.map((c) => {
            const q = map.project(c as [number, number]);
            return [q.x, q.y];
          });
          for (let i = 1; i < pts.length; i++) {
            const a = pts[i - 1];
            const b = pts[i];
            const dx = b[0] - a[0];
            const dy = b[1] - a[1];
            const l = Math.hypot(dx, dy);
            if (l < 0.5) continue;
            const nx = (-dy / l) * 0.8;
            const ny = (dx / l) * 0.8;
            quads.push([[un([a[0] + nx, a[1] + ny]), un([b[0] + nx, b[1] + ny]), un([b[0] - nx, b[1] - ny]), un([a[0] - nx, a[1] - ny]), un([a[0] + nx, a[1] + ny])]]);
          }
        }
      if (quads.length)
        out.push({
          type: "Feature",
          properties: { t: "rim", c: p.c, lb: lo, lt: lo + band, hb: Math.max(lo, hi - band), ht: hi },
          geometry: { type: "MultiPolygon", coordinates: quads },
        });
    }
  }
  return { type: "FeatureCollection", features: out };
}

/** The airspace at a point on the map (the areas drawn there), for its card. */
export function airspaceAt(map: maplibregl.Map, p: { x: number; y: number }): maplibregl.MapGeoJSONFeature[] {
  const layers = SOURCES.flatMap((s) => [ids(s).fill, ids(s).block]).filter((id) => map.getLayer(id) && map.getLayoutProperty(id, "visibility") !== "none");
  if (!layers.length) return [];
  const seen = new Set<string>();
  return map.queryRenderedFeatures([p.x, p.y], { layers }).filter((f) => {
    const k = `${f.properties.c}|${f.properties.label}`;
    if (seen.has(k)) return false;
    seen.add(k);
    return true;
  });
}

/** The little card for the airspace clicked (several when they overlap). */
export function airspaceCard(list: maplibregl.MapGeoJSONFeature[], onclose: () => void): HTMLElement {
  const box = document.createElement("div");
  box.className = "nav-card asp-card";
  const head = document.createElement("div");
  head.className = "nc-head";
  const title = document.createElement("strong");
  title.textContent = list.length > 1 ? `${list.length} airspaces here` : "Airspace";
  const x = document.createElement("button");
  x.type = "button";
  x.className = "nc-x";
  x.setAttribute("aria-label", "Close");
  x.textContent = "×";
  x.onclick = onclose;
  head.append(title, x);
  box.append(head);
  const sorted = [...list].sort((a, b) => (Number(a.properties.lo) || 0) - (Number(b.properties.lo) || 0)).slice(0, 6);
  for (const f of sorted) {
    const p = f.properties as Record<string, string>;
    const kind = ASP_KINDS[p.c] ?? { color: "#4a8ce8", name: p.c };
    const item = document.createElement("div");
    item.className = "asp-item";
    item.style.setProperty("--c", kind.color);
    const name = document.createElement("div");
    name.className = "asp-name";
    name.textContent = p.name || kind.name;
    const sub = document.createElement("div");
    sub.className = "asp-sub";
    const lim = p.lo_t || p.hi_t ? `${p.lo_t || "?"} to ${p.hi_t || "?"}` : String(p.label).split(": ")[1] ?? "";
    sub.textContent = `${kind.name}${lim ? ` · ${lim}` : ""}`;
    item.append(name, sub);
    box.append(item);
  }
  return box;
}
