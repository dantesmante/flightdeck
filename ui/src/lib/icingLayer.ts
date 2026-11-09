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
export const ICING_BANDS: { min: number; label: string; color: string; angle: 45 | -45; offset: number; width: number }[] = [
  { min: 6, label: "6–20", color: "#a9d8ff", angle: 45, offset: 0, width: 1.6 },
  { min: 21, label: "21–40", color: "#6fa6ff", angle: -45, offset: 0, width: 1.6 },
  { min: 41, label: "41–60", color: "#9a7dff", angle: 45, offset: 0.5, width: 1.8 },
  { min: 61, label: "61–80", color: "#8a3cf0", angle: -45, offset: 0.5, width: 2 },
  { min: 81, label: "81–100", color: "#6a12c8", angle: 45, offset: 0.25, width: 2.6 },
];

const TILE = 24; // pattern tile, device pixels (12 CSS px at pixel ratio 2)

/** One band's hatching as a seamless tile. */
function hatch(b: (typeof ICING_BANDS)[number], ctx: CanvasRenderingContext2D) {
  ctx.strokeStyle = b.color;
  ctx.lineWidth = b.width * 2;
  ctx.lineCap = "square";
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

function patternImage(i: number): ImageData {
  const c = document.createElement("canvas");
  c.width = c.height = TILE;
  const g = c.getContext("2d")!;
  hatch(ICING_BANDS[i], g);
  return g.getImageData(0, 0, TILE, TILE);
}

/** A legend swatch: the hatching as it looks in band i (all lower bands' lines too). */
export function icingSwatch(i: number): string {
  if (typeof document === "undefined") return "";
  const c = document.createElement("canvas");
  c.width = c.height = TILE;
  const g = c.getContext("2d")!;
  g.fillStyle = "#1c1f24";
  g.fillRect(0, 0, TILE, TILE);
  for (let k = 0; k <= i; k++) hatch(ICING_BANDS[k], g);
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
  const before = map.getLayer("routes-line") ? "routes-line" : undefined;
  ICING_BANDS.forEach((b, i) => {
    map.addLayer({ id: LAYERS[i], type: "fill", source: "fd-icing", filter: ["==", ["get", "band"], i], paint: { "fill-pattern": `icing-hatch-${i}`, "fill-antialias": false } }, before);
  });
  map.addLayer(
    {
      id: "fd-icing-edge",
      type: "line",
      source: "fd-icing",
      paint: { "line-color": ["match", ["get", "band"], ...ICING_BANDS.flatMap((b, i) => [i, b.color]), "#a9d8ff"] as unknown as maplibregl.ExpressionSpecification, "line-width": 1, "line-opacity": 0.75 },
    },
    before,
  );
}

export function showIcing(map: maplibregl.Map, on: boolean) {
  for (const id of [...LAYERS, "fd-icing-edge"]) if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", on ? "visible" : "none");
}

/** Flight level nearest an altitude, from those offered. */
export function flFor(ft: number): number {
  const fl = ft / 100;
  return ICING_FLS.reduce((best, x) => (Math.abs(x - fl) < Math.abs(best - fl) ? x : best), ICING_FLS[0]);
}
