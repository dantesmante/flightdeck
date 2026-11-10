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
const color = ["match", ["get", "c"], ...KINDS.flatMap((k) => [k, ASP_KINDS[k].color]), "#4a8ce8"] as unknown as maplibregl.ExpressionSpecification;
const DASHED = KINDS.filter((k) => ASP_KINDS[k].dash);
const isDashed = ["in", ["get", "c"], ["literal", DASHED]] as unknown as maplibregl.FilterSpecification;
const isSolid = ["!", ["in", ["get", "c"], ["literal", DASHED]]] as unknown as maplibregl.FilterSpecification;

export const SOURCES = ["us-airspace", "world-airspace"] as const;
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
      paint: { "fill-color": color, "fill-opacity": ["match", ["get", "c"], ["P", "R"], 0.07, ["D", "E", "G", "F", "CTR"], 0.025, 0.045] as unknown as maplibregl.ExpressionSpecification },
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
    map.addLayer({
      id: "asp-ribbon-edge",
      type: "line",
      source: "asp-over",
      filter: ["==", ["get", "t"], "ribbon"],
      layout: { visibility: "none", "line-cap": "round", "line-join": "round" },
      paint: { "line-color": color, "line-width": 18 },
    });
    map.addLayer({
      id: "asp-ribbon",
      type: "line",
      source: "asp-over",
      filter: ["==", ["get", "t"], "ribbon"],
      layout: { visibility: "none", "line-cap": "round", "line-join": "round" },
      paint: { "line-color": "rgba(14,15,18,0.95)", "line-width": 15 },
    });
    if (map.getStyle().glyphs)
      map.addLayer({
        id: "asp-label",
        type: "symbol",
        source: "asp-over",
        filter: ["==", ["get", "t"], "label"],
        layout: {
          visibility: "none",
          "symbol-placement": "line-center",
          "text-field": ["get", "label"],
          "text-font": ["Noto Sans Bold"],
          "text-size": 11,
          "text-letter-spacing": 0.03,
          "text-keep-upright": false,
          "text-max-angle": 85,
          "text-allow-overlap": true,
          "text-ignore-placement": true,
          "text-rotation-alignment": "map",
        },
        paint: { "text-color": "#f2f4f7" },
      });
  }
  showAirspace(map, false, three);
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
  for (const id of ["asp-ribbon-edge", "asp-ribbon", "asp-label"]) vis(id, on);
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

const textPx = (s: string) => s.length * 7.9 + 26;

/** A polyline shifted sideways by d pixels (positive = to the right of travel). */
function offset(pts: P[], d: number): P[] {
  return pts.map((p, i) => {
    const a = pts[Math.max(0, i - 1)];
    const b = pts[Math.min(pts.length - 1, i + 1)];
    let nx = -(b[1] - a[1]);
    let ny = b[0] - a[0];
    const l = Math.hypot(nx, ny) || 1;
    nx /= l;
    ny /= l;
    // (nx, ny) is the right of travel on screen (y down).
    return [p[0] + nx * d, p[1] + ny * d];
  });
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

/** Part of a polyline from distance s0 to s1 (pixels along it). */
function slice(pts: P[], cum: number[], s0: number, s1: number): P[] {
  const at = (s: number): P => {
    let i = 1;
    while (i < cum.length - 1 && cum[i] < s) i++;
    const t = (s - cum[i - 1]) / (cum[i] - cum[i - 1] || 1);
    return [pts[i - 1][0] + (pts[i][0] - pts[i - 1][0]) * t, pts[i - 1][1] + (pts[i][1] - pts[i - 1][1]) * t];
  };
  const out: P[] = [at(s0)];
  for (let i = 0; i < pts.length; i++) if (cum[i] > s0 && cum[i] < s1) out.push(pts[i]);
  out.push(at(s1));
  return out;
}

/**
 * Labels in the view: each area's label along its own edge, just inside it,
 * reading left to right. Where the edge curves gently the label follows it;
 * where it is jagged the label sits on a straight line drawn far enough in.
 * Rims (3D): thin bands at the floor and ceiling of each block in view.
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
  const placed: { x: number; y: number; r: number }[] = [];
  const un = (p: P): [number, number] => {
    const l = map.unproject(p);
    return [l.lng, l.lat];
  };
  for (const f of inView.slice(0, 800)) {
    const label = String(f.properties?.label ?? "");
    const need = textPx(label);
    for (const poly of polygons(f)) {
      poly.forEach((ring, ri) => {
        const pts: P[] = ring.map((c) => {
          const q = map.project(c as [number, number]);
          return [q.x, q.y];
        });
        if (pts.length < 3) return;
        // Which side the area is on: clockwise on screen (y down) = right.
        let area = 0;
        for (let i = 0; i < pts.length - 1; i++) area += pts[i][0] * pts[i + 1][1] - pts[i + 1][0] * pts[i][1];
        const insideRight = ri === 0 ? area > 0 : area < 0;
        // Runs where the edge goes one way across the screen, judged on the
        // edge with its small wiggles smoothed out (a jagged coast is one run).
        const keep = simplifyIdx(pts, 10);
        let start = keep[0];
        const runs: [number, number, number][] = [];
        let dir = 0;
        for (let j = 1; j < keep.length; j++) {
          const dx = pts[keep[j]][0] - pts[keep[j - 1]][0];
          const d = dx > 0.5 ? 1 : dx < -0.5 ? -1 : 0;
          if (d === 0) continue;
          if (dir === 0) dir = d;
          else if (d !== dir) {
            runs.push([start, keep[j - 1], dir]);
            start = keep[j - 1];
            dir = d;
          }
        }
        runs.push([start, keep[keep.length - 1], dir || 1]);
        for (const [a, b, d] of runs) {
          let run = pts.slice(a, b + 1);
          if (d < 0) run = run.reverse();
          // After reversing, the inside is on the other side.
          const right = d > 0 ? insideRight : !insideRight;
          const cum = [0];
          for (let i = 1; i < run.length; i++) cum.push(cum[i - 1] + Math.hypot(run[i][0] - run[i - 1][0], run[i][1] - run[i - 1][1]));
          const len = cum[cum.length - 1];
          if (len < need + 24) continue;
          const n = Math.max(1, Math.floor(len / 700));
          for (let k = 0; k < n; k++) {
            const mid = (len * (k + 0.5)) / n;
            let piece = slice(run, cum, mid - need / 2, mid + need / 2);
            const c = piece[Math.floor(piece.length / 2)];
            if (c[0] < 20 || c[0] > W - 20 || c[1] < 20 || c[1] > H - 20) continue;
            // Jagged: many turns both ways. Then a straight line, set in
            // past the edge's bumps.
            let turn = 0;
            let net = 0;
            for (let i = 2; i < piece.length; i++) {
              const a1 = Math.atan2(piece[i - 1][1] - piece[i - 2][1], piece[i - 1][0] - piece[i - 2][0]);
              const a2 = Math.atan2(piece[i][1] - piece[i - 1][1], piece[i][0] - piece[i - 1][0]);
              let t = a2 - a1;
              while (t > Math.PI) t -= 2 * Math.PI;
              while (t < -Math.PI) t += 2 * Math.PI;
              turn += Math.abs(t);
              net += t;
            }
            const sign = right ? 1 : -1;
            let gap = 13;
            // The text's own line is longer: MapLibre fits a label at the
            // map's whole zoom level, where everything is up to half the size.
            const half = Math.min(mid, len - mid, (need * Math.pow(2, z - Math.floor(z))) / 2 + 12);
            let textLine = slice(run, cum, mid - half, mid + half);
            const jagged = turn > Math.abs(net) * 1.5 + 0.35;
            if (jagged) {
              const p0 = piece[0];
              const p1 = piece[piece.length - 1];
              // How far the edge pokes in past the straight line.
              const dx = p1[0] - p0[0];
              const dy = p1[1] - p0[1];
              const l = Math.hypot(dx, dy) || 1;
              let poke = 0;
              for (const q of piece) poke = Math.max(poke, sign * ((q[0] - p0[0]) * -dy + (q[1] - p0[1]) * dx) / l);
              gap = 16 + Math.max(0, poke);
              piece = [p0, p1];
              const ux = dx / l;
              const uy = dy / l;
              const mx = (p0[0] + p1[0]) / 2;
              const my = (p0[1] + p1[1]) / 2;
              const h = (need * Math.pow(2, z - Math.floor(z))) / 2 + 12;
              textLine = [
                [mx - ux * h, my - uy * h],
                [mx + ux * h, my + uy * h],
              ];
            }
            const line = offset(piece, sign * gap);
            const cc = line[Math.floor(line.length / 2)];
            const r = need * 0.55;
            if (placed.some((q) => Math.hypot(q.x - cc[0], q.y - cc[1]) < Math.max(r, q.r))) continue;
            placed.push({ x: cc[0], y: cc[1], r });
            out.push({ type: "Feature", properties: { t: "ribbon", c: f.properties?.c }, geometry: { type: "LineString", coordinates: line.map(un) } });
            out.push({ type: "Feature", properties: { t: "label", c: f.properties?.c, label }, geometry: { type: "LineString", coordinates: offset(textLine, sign * gap).map(un) } });
          }
        }
      });
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
