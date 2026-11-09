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
      const [r, gg, b] = tempColor(v);
      img.data[i] = r;
      img.data[i + 1] = gg;
      img.data[i + 2] = b;
      img.data[i + 3] = 255;
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
  if (map.getLayer("fd-temp")) map.removeLayer("fd-temp");
  if (map.getSource("fd-temp")) map.removeSource("fd-temp");
  map.addSource("fd-temp", { type: "canvas", canvas, coordinates: COORDS, animate: false });
  map.addLayer(
    { id: "fd-temp", type: "raster", source: "fd-temp", paint: { "raster-opacity": 0.5, "raster-resampling": "linear", "raster-fade-duration": 0 } },
    map.getLayer("routes-line") ? "routes-line" : undefined,
  );
}

export function showTemp(map: maplibregl.Map, on: boolean) {
  if (map.getLayer("fd-temp")) map.setLayoutProperty("fd-temp", "visibility", on ? "visible" : "none");
}
