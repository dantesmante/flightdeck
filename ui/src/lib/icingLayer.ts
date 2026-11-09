// The Icing Potential map layer (test version): FlightDeck's own estimate,
// from NOAA's GFS forecast, of where an aircraft would pick up ice at one
// flight level. For flight simulation only. Drawn as hatching (not a
// coloured wash, so it does not look like temperature or radar): the
// one-degree scores are outlined into shapes once per level and forecast,
// and each band is filled with its own lines.

import type maplibregl from "maplibre-gl";
import type { GeoJSONSource } from "maplibre-gl";
import { contours } from "d3-contour";

/** Flight levels offered. */
export const ICING_FLS = [50, 100, 150, 180, 200, 250, 300, 350, 400];

/**
 * Bands (lowest score in each) and their hatching, light blue to deep
 * purple. The areas nest (each band covers the ones above it), so the
 * hatching builds up: one set of lines where icing is light, a dense
 * cross-hatch where it is high.
 */
export const ICING_BANDS: { min: number; label: string; color: string; rgb: string; angle: 45 | -45; offset: number }[] = [
  { min: 6, label: "6–20", color: "#7cc0ff", rgb: "124,192,255", angle: 45, offset: 0 },
  { min: 21, label: "21–40", color: "#5a8cff", rgb: "90,140,255", angle: -45, offset: 0 },
  { min: 41, label: "41–60", color: "#8a6cff", rgb: "138,108,255", angle: 45, offset: 0.5 },
  { min: 61, label: "61–80", color: "#9a4dff", rgb: "154,77,255", angle: -45, offset: 0.5 },
  { min: 81, label: "81–100", color: "#7a1fe0", rgb: "122,31,224", angle: 45, offset: 0.25 },
];

/**
 * Pattern tile in device pixels (6 CSS px at pixel ratio 2): a see-through
 * tint of the band's colour carries the reading, with faint lines one
 * device pixel wide as texture. The bands nest, so the colour deepens
 * towards the middle of an icing area while the map still shows through.
 */
const TILE = 12;

function hatch(b: (typeof ICING_BANDS)[number], ctx: CanvasRenderingContext2D) {
  ctx.fillStyle = `rgba(${b.rgb},0.15)`;
  ctx.fillRect(0, 0, TILE, TILE);
  ctx.strokeStyle = `rgba(${b.rgb},0.3)`;
  ctx.lineWidth = 1;
  const o = b.offset * TILE;
  for (const k of [-1, 0, 1]) {
    ctx.beginPath();
    if (b.angle === 45) {
      ctx.moveTo(k * TILE + o, TILE);
      ctx.lineTo(k * TILE + o + TILE, 0);
    } else {
      ctx.moveTo(k * TILE + o, 0);
      ctx.lineTo(k * TILE + o + TILE, TILE);
    }
    ctx.stroke();
  }
}

function tile(i: number): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.width = c.height = TILE;
  hatch(ICING_BANDS[i], c.getContext("2d")!);
  return c;
}

function patternImage(i: number): ImageData {
  return tile(i).getContext("2d")!.getImageData(0, 0, TILE, TILE);
}

/** A legend swatch: band i as it looks on the map (the lower bands under it). */
export function icingSwatch(i: number): string {
  if (typeof document === "undefined") return "";
  const S = 36;
  const c = document.createElement("canvas");
  c.width = c.height = S;
  const g = c.getContext("2d")!;
  g.fillStyle = "#1c1f24";
  g.fillRect(0, 0, S, S);
  for (let k = 0; k <= i; k++) {
    g.fillStyle = g.createPattern(tile(k), "repeat")!;
    g.fillRect(0, 0, S, S);
  }
  return c.toDataURL();
}

export function decodeIcing(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

const UNKNOWN = 255;
const UP = 3; // grid refinement before outlining, for smooth edges

/**
 * Outlines of the bands as map shapes. The one-degree scores are refined
 * (bilinear, three times finer) and outlined at each band's lowest score.
 * Where any neighbour is unknown the refined value is 0: no shape is ever
 * drawn into a gap in the data.
 */
export function icingShapes(v: Uint8Array, nx: number, ny: number): GeoJSON.FeatureCollection {
  // Columns from 180°W, plus the first again at the end (no seam at the date line).
  const at = (r: number, c: number) => v[r * nx + ((c + nx / 2) % nx)];
  const W = nx * UP + 1;
  const H = (ny - 1) * UP + 1;
  const g = new Float64Array(W * H);
  for (let y = 0; y < H; y++) {
    const fr = y / UP;
    const r0 = Math.min(ny - 2, Math.floor(fr));
    const rt = fr - r0;
    for (let x = 0; x < W; x++) {
      const fc = x / UP;
      const c0 = Math.min(nx - 1, Math.floor(fc));
      const ct = fc - c0;
      const a = at(r0, c0);
      const b = at(r0, c0 + 1);
      const d = at(r0 + 1, c0);
      const e = at(r0 + 1, c0 + 1);
      g[y * W + x] = a === UNKNOWN || b === UNKNOWN || d === UNKNOWN || e === UNKNOWN ? 0 : (a * (1 - ct) + b * ct) * (1 - rt) + (d * (1 - ct) + e * ct) * rt;
    }
  }
  const multi = contours()
    .size([W, H])
    .thresholds(ICING_BANDS.map((b) => b.min))(Array.from(g));
  const lonlat = ([x, y]: number[]): [number, number] => [-180 + (x - 0.5) / UP, Math.max(-85, Math.min(85, 90 - (y - 0.5) / UP))];
  return {
    type: "FeatureCollection",
    features: multi
      .filter((m) => m.coordinates.length)
      .map((m) => ({
        type: "Feature",
        properties: { band: ICING_BANDS.findIndex((b) => b.min === m.value) },
        geometry: { type: "MultiPolygon", coordinates: m.coordinates.map((poly) => poly.map((ring) => ring.map(lonlat))) },
      })),
  };
}

const LAYERS = ICING_BANDS.map((_, i) => `fd-icing-${i}`);

export function setIcingShapes(map: maplibregl.Map, fc: GeoJSON.FeatureCollection) {
  const src = map.getSource("fd-icing") as GeoJSONSource | undefined;
  if (src) {
    src.setData(fc);
    return;
  }
  ICING_BANDS.forEach((_, i) => {
    if (!map.hasImage(`icing-hatch-${i}`)) map.addImage(`icing-hatch-${i}`, patternImage(i), { pixelRatio: 2 });
  });
  map.addSource("fd-icing", { type: "geojson", data: fc, tolerance: 0.6 });
  // Under the SIGMET and AIRMET outlines when those are already there.
  const before = map.getLayer("fd-icing-adv-fill") ? "fd-icing-adv-fill" : map.getLayer("routes-line") ? "routes-line" : undefined;
  ICING_BANDS.forEach((b, i) => {
    map.addLayer({ id: LAYERS[i], type: "fill", source: "fd-icing", filter: ["==", ["get", "band"], i], paint: { "fill-pattern": `icing-hatch-${i}`, "fill-antialias": false } }, before);
  });
  map.addLayer(
    {
      id: "fd-icing-edge",
      type: "line",
      source: "fd-icing",
      paint: { "line-color": ["match", ["get", "band"], ...ICING_BANDS.flatMap((b, i) => [i, b.color]), "#a9d8ff"] as unknown as maplibregl.ExpressionSpecification, "line-width": 0.7, "line-opacity": 0.35 },
    },
    before,
  );
}

// --- Icing SIGMETs and AIRMETs ----------------------------------------------
// Official advisories from the Aviation Weather Center (the same feed as the
// SIGMETs and AIRMETs layer), drawn over FlightDeck's estimate: icing
// SIGMETs are for severe icing, icing AIRMETs (G-AIRMET) for moderate.

export const ICING_ADV = {
  SEV: { color: "#ff4f9a", label: "Severe icing (SIGMET)" },
  MOD: { color: "#e8f4ff", label: "Moderate icing (AIRMET)" },
} as const;

const ADV_LAYERS = ["fd-icing-adv-fill", "fd-icing-adv-line", "fd-icing-adv-label"];

/** Severity of an icing advisory: SIGMETs severe, AIRMETs moderate, unless it says otherwise. */
export function icingSeverity(kind: string, qualifier: string): "SEV" | "MOD" {
  const q = qualifier.toUpperCase();
  if (q.includes("SEV")) return "SEV";
  if (q.includes("MOD")) return "MOD";
  return kind === "AIRMET" ? "MOD" : "SEV";
}

/** Feet from an advisory level: "12000", "FL120", "120" (hundreds), "SFC"; NaN if unknown ("FZL"). */
function levelFt(v: string): number {
  const t = v.trim().toUpperCase();
  if (!t) return NaN;
  if (t === "SFC" || t === "SURFACE") return 0;
  const n = Number(t.replace(/^FL/, ""));
  if (!Number.isFinite(n)) return NaN;
  return t.startsWith("FL") || n < 1000 ? n * 100 : n;
}

/** Whether an advisory covers a flight level (unknown levels count as yes). */
export function coversFl(base: string, top: string, fl: number): boolean {
  const ft = fl * 100;
  const b = levelFt(base);
  const t = levelFt(top);
  if (Number.isFinite(b) && ft < b - 1) return false;
  if (Number.isFinite(t) && t > 0 && ft > t + 1) return false;
  return true;
}

export function setIcingAdvisories(map: maplibregl.Map, fc: GeoJSON.FeatureCollection) {
  const src = map.getSource("fd-icing-adv") as GeoJSONSource | undefined;
  if (src) {
    src.setData(fc);
    return;
  }
  map.addSource("fd-icing-adv", { type: "geojson", data: fc });
  const before = map.getLayer("routes-line") ? "routes-line" : undefined;
  const color = ["match", ["get", "sev"], "SEV", ICING_ADV.SEV.color, ICING_ADV.MOD.color] as unknown as maplibregl.ExpressionSpecification;
  const op = (on: number, off: number) => ["case", ["get", "at"], on, off] as unknown as maplibregl.ExpressionSpecification;
  // A barely visible fill, so a click anywhere inside opens the card.
  map.addLayer({ id: ADV_LAYERS[0], type: "fill", source: "fd-icing-adv", paint: { "fill-color": color, "fill-opacity": op(0.06, 0.02) } }, before);
  map.addLayer(
    {
      id: ADV_LAYERS[1],
      type: "line",
      source: "fd-icing-adv",
      paint: { "line-color": color, "line-width": ["match", ["get", "sev"], "SEV", 2, 1.5] as unknown as maplibregl.ExpressionSpecification, "line-opacity": op(0.95, 0.35), "line-dasharray": [4, 2] },
    },
    before,
  );
  map.addLayer(
    {
      id: ADV_LAYERS[2],
      type: "symbol",
      source: "fd-icing-adv",
      layout: { "text-field": ["match", ["get", "sev"], "SEV", "SEV ICE", "MOD ICE"] as unknown as maplibregl.ExpressionSpecification, "text-font": ["Noto Sans Bold"], "text-size": 11, "text-optional": true },
      paint: { "text-color": color, "text-opacity": op(1, 0.45), "text-halo-color": "rgba(15,17,20,0.9)", "text-halo-width": 1.4 },
    },
    before,
  );
}

export function showIcing(map: maplibregl.Map, on: boolean) {
  for (const id of [...LAYERS, "fd-icing-edge", ...ADV_LAYERS]) if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", on ? "visible" : "none");
}

/** Flight level nearest an altitude, from those offered. */
export function flFor(ft: number): number {
  const fl = ft / 100;
  return ICING_FLS.reduce((best, x) => (Math.abs(x - fl) < Math.abs(best - fl) ? x : best), ICING_FLS[0]);
}
