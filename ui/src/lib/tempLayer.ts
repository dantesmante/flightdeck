// The Temperature map layer: air temperature 2 m above the ground from
// NOAA's GFS forecast (public domain), the same file the wind layer reads.
// The one-degree grid is drawn once into a small picture in the map's own
// (Mercator) projection and laid over the map as one image: the graphics
// card only stretches a single texture.

import type maplibregl from "maplibre-gl";

/** Colour stops: °C → RGB. Purple cold, blue near freezing, green mild, red hot. */
export const TEMP_STOPS: [number, [number, number, number]][] = [
  [-40, [120, 60, 170]],
  [-25, [70, 80, 200]],
  [-10, [60, 140, 230]],
  [0, [110, 200, 240]],
  [10, [90, 200, 120]],
  [20, [235, 220, 80]],
  [30, [240, 140, 50]],
  [40, [210, 40, 40]],
];

export function tempColor(c: number): [number, number, number] {
  const s = TEMP_STOPS;
  if (c <= s[0][0]) return s[0][1];
  for (let i = 1; i < s.length; i++) {
    if (c <= s[i][0]) {
      const t = (c - s[i - 1][0]) / (s[i][0] - s[i - 1][0]);
      const a = s[i - 1][1];
      const b = s[i][1];
      return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
    }
  }
  return s[s.length - 1][1];
}

/** Tenths of °C, 16-bit little-endian, base64 → °C (NaN where missing). */
export function decodeTemps(b64: string): Float32Array {
  const bin = atob(b64);
  const out = new Float32Array(bin.length / 2);
  for (let i = 0; i < out.length; i++) {
    let v = bin.charCodeAt(2 * i) | (bin.charCodeAt(2 * i + 1) << 8);
    if (v & 0x8000) v -= 0x10000;
    out[i] = v / 10;
  }
  return out;
}

/** Temperature (°C) at a point, bilinear between grid points; NaN if missing. */
export function tempAt(t: Float32Array, nx: number, ny: number, lon: number, lat: number): number {
  const fr = Math.min(ny - 1, Math.max(0, 90 - lat));
  const fc = (((lon % 360) + 360) % 360);
  const r0 = Math.min(ny - 2, Math.floor(fr));
  const c0 = Math.floor(fc) % nx;
  const c1 = (c0 + 1) % nx;
  const rt = fr - r0;
  const ct = fc - Math.floor(fc);
  const v = (r: number, c: number) => t[r * nx + c];
  return (v(r0, c0) * (1 - ct) + v(r0, c1) * ct) * (1 - rt) + (v(r0 + 1, c0) * (1 - ct) + v(r0 + 1, c1) * ct) * rt;
}

/** The key's colour bar, −40 to +40 °C. */
export const TEMP_BAR = `linear-gradient(90deg, ${TEMP_STOPS.map(([c, [r, g, b]]) => `rgb(${r},${g},${b}) ${((c + 40) / 80) * 100}%`).join(", ")})`;

const MAX_LAT = 85.0511;

/** The picture: `size` pixels square, Mercator, longitudes −180..180. */
export function tempImage(t: Float32Array, nx: number, ny: number, size = 512): HTMLCanvasElement {
  return gridImage(t, nx, ny, (v) => [...tempColor(v), 255], size);
}

/**
 * Any one-degree grid as a picture (Mercator, −180..180), each value
 * coloured by `color` (RGBA, or null for nothing).
 */
export function gridImage(
  t: Float32Array,
  nx: number,
  ny: number,
  color: (v: number) => readonly [number, number, number, number] | null,
  size = 512,
): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.width = c.height = size;
  const g = c.getContext("2d")!;
  const img = g.createImageData(size, size);
  const at = (row: number, col: number) => t[Math.min(ny - 1, Math.max(0, row)) * nx + (((col % nx) + nx) % nx)];
  for (let y = 0; y < size; y++) {
    const my = Math.PI * (1 - (2 * (y + 0.5)) / size);
    const lat = (Math.atan(Math.sinh(my)) * 180) / Math.PI;
    const fr = 90 - lat; // grid rows from 90N, one per degree
    const r0 = Math.floor(fr);
    const rt = fr - r0;
    for (let x = 0; x < size; x++) {
      const lon = -180 + ((x + 0.5) / size) * 360;
      const fc = (lon + 360) % 360; // grid columns from 0E
      const c0 = Math.floor(fc);
      const ct = fc - c0;
      const v =
        (at(r0, c0) * (1 - ct) + at(r0, c0 + 1) * ct) * (1 - rt) + (at(r0 + 1, c0) * (1 - ct) + at(r0 + 1, c0 + 1) * ct) * rt;
      const i = (y * size + x) * 4;
      if (!Number.isFinite(v)) continue;
      const col = color(v);
      if (!col) continue;
      img.data[i] = col[0];
      img.data[i + 1] = col[1];
      img.data[i + 2] = col[2];
      img.data[i + 3] = col[3];
    }
  }
  g.putImageData(img, 0, 0);
  return c;
}

const COORDS: [[number, number], [number, number], [number, number], [number, number]] = [
  [-180, MAX_LAT],
  [180, MAX_LAT],
  [180, -MAX_LAT],
  [-180, -MAX_LAT],
];

/**
 * Puts the picture on the map as a canvas source: nothing to download, so
 * the app's content rules (which block a data: URL for map images) do not
 * get in the way. A new picture replaces the source.
 */
export function setTempCanvas(map: maplibregl.Map, canvas: HTMLCanvasElement) {
  setGridCanvas(map, "fd-temp", canvas, 0.5);
}

export function showTemp(map: maplibregl.Map, on: boolean) {
  showGrid(map, "fd-temp", on);
}

/** A grid picture as its own map layer (`id`), under the routes. */
export function setGridCanvas(map: maplibregl.Map, id: string, canvas: HTMLCanvasElement, opacity: number) {
  if (map.getLayer(id)) map.removeLayer(id);
  if (map.getSource(id)) map.removeSource(id);
  map.addSource(id, { type: "canvas", canvas, coordinates: COORDS, animate: false });
  map.addLayer(
    { id, type: "raster", source: id, paint: { "raster-opacity": opacity, "raster-resampling": "linear", "raster-fade-duration": 0 } },
    map.getLayer("routes-line") ? "routes-line" : undefined,
  );
}

export function showGrid(map: maplibregl.Map, id: string, on: boolean) {
  if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", on ? "visible" : "none");
}

// --- Cloud cover and visibility (same forecast file, same kind of picture) ---

/** Cloud cover (%): white, more solid as it thickens; nothing under 10 %. */
export function cloudColor(pct: number): readonly [number, number, number, number] | null {
  if (pct < 10) return null;
  const a = Math.min(1, (pct - 10) / 90);
  return [236, 240, 245, Math.round(30 + a * 170)];
}
export const CLOUD_BAR = "linear-gradient(90deg, rgba(236,240,245,0.05), rgba(236,240,245,0.78))";

/** Visibility, statute miles: flight-category colours; clear (5 SM and more) shows nothing. */
export const VIS_BANDS = [
  { max: 1, label: "< 1", color: "#cf5bd6", rgba: [207, 91, 214, 150] },
  { max: 3, label: "1–3", color: "#e2453c", rgba: [226, 69, 60, 120] },
  { max: 5, label: "3–5", color: "#4f8fe8", rgba: [79, 143, 232, 95] },
] as const;
export const SM = 1609.344;
/** Grid value (hundreds of metres after decoding) → statute miles. */
export const visMiles = (v: number) => (v * 100) / SM;
export function visColor(v: number): readonly [number, number, number, number] | null {
  const mi = visMiles(v);
  for (const b of VIS_BANDS) if (mi < b.max) return b.rgba;
  return null;
}
