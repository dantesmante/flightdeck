// The Icing Potential map layer (test version): FlightDeck's own estimate,
// from NOAA's GFS forecast, of where an aircraft would pick up ice at one
// flight level. For flight simulation only. The one-degree scores are drawn
// once into a picture in the map's projection and laid over the map like
// the temperature (a canvas source: nothing fetched, nothing on the GPU but
// one texture).

import type maplibregl from "maplibre-gl";

/** Flight levels offered. */
export const ICING_FLS = [50, 100, 150, 180, 200, 250, 300, 350, 400];

/** Score bands (upper bound inclusive) and colours, as in the spec. */
export const ICING_BANDS: { max: number; color: [number, number, number]; label: string }[] = [
  { max: 5, color: [0, 0, 0], label: "None" },
  { max: 20, color: [190, 225, 255], label: "6–20" },
  { max: 40, color: [70, 140, 255], label: "21–40" },
  { max: 60, color: [255, 225, 70], label: "41–60" },
  { max: 80, color: [255, 150, 45], label: "61–80" },
  { max: 100, color: [240, 45, 45], label: "81–100" },
];

/** Colour stops at band middles, for smooth edges between bands. */
const STOPS: [number, [number, number, number]][] = [
  [13, [190, 225, 255]],
  [30, [70, 140, 255]],
  [50, [255, 225, 70]],
  [70, [255, 150, 45]],
  [90, [240, 45, 45]],
];

/** RGBA for a score; transparent at 5 and below. */
export function icingColor(v: number): [number, number, number, number] {
  if (v <= 5) return [0, 0, 0, 0];
  let c = STOPS[0][1];
  if (v >= STOPS[STOPS.length - 1][0]) c = STOPS[STOPS.length - 1][1];
  else
    for (let i = 1; i < STOPS.length; i++)
      if (v <= STOPS[i][0]) {
        const a = STOPS[i - 1];
        const b = STOPS[i];
        const t = Math.max(0, (v - a[0]) / (b[0] - a[0]));
        c = [a[1][0] + (b[1][0] - a[1][0]) * t, a[1][1] + (b[1][1] - a[1][1]) * t, a[1][2] + (b[1][2] - a[1][2]) * t];
        break;
      }
  // Fades in from 5 to 20, then semi-transparent so the map shows through.
  const alpha = v < 20 ? ((v - 5) / 15) * 150 : 150 + Math.min(1, (v - 20) / 60) * 40;
  return [c[0], c[1], c[2], alpha];
}

export function decodeIcing(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

const MAX_LAT = 85.0511;
const UNKNOWN = 255;

/**
 * The picture: Mercator, longitudes −180..180. Smooth between four known
 * neighbours; next to unknown cells (under the ground, missing data) the
 * nearest cell is used as is, so no colour is made up across a gap.
 */
export function icingImage(v: Uint8Array, nx: number, ny: number, size = 768): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.width = c.height = size;
  const g = c.getContext("2d")!;
  const img = g.createImageData(size, size);
  const at = (row: number, col: number) => v[Math.min(ny - 1, Math.max(0, row)) * nx + (((col % nx) + nx) % nx)];
  for (let y = 0; y < size; y++) {
    const my = Math.PI * (1 - (2 * (y + 0.5)) / size);
    const lat = (Math.atan(Math.sinh(my)) * 180) / Math.PI;
    const fr = 90 - lat;
    const r0 = Math.floor(fr);
    const rt = fr - r0;
    for (let x = 0; x < size; x++) {
      const lon = -180 + ((x + 0.5) / size) * 360;
      const fc = (lon + 360) % 360;
      const c0 = Math.floor(fc);
      const ct = fc - c0;
      const a = at(r0, c0);
      const b = at(r0, c0 + 1);
      const d = at(r0 + 1, c0);
      const e = at(r0 + 1, c0 + 1);
      let s: number;
      if (a !== UNKNOWN && b !== UNKNOWN && d !== UNKNOWN && e !== UNKNOWN) s = (a * (1 - ct) + b * ct) * (1 - rt) + (d * (1 - ct) + e * ct) * rt;
      else {
        s = rt < 0.5 ? (ct < 0.5 ? a : b) : ct < 0.5 ? d : e;
        if (s === UNKNOWN) continue;
      }
      const [r, gg, bb, al] = icingColor(s);
      if (al <= 0) continue;
      const i = (y * size + x) * 4;
      img.data[i] = r;
      img.data[i + 1] = gg;
      img.data[i + 2] = bb;
      img.data[i + 3] = al;
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

export function setIcingCanvas(map: maplibregl.Map, canvas: HTMLCanvasElement) {
  if (map.getLayer("fd-icing")) map.removeLayer("fd-icing");
  if (map.getSource("fd-icing")) map.removeSource("fd-icing");
  map.addSource("fd-icing", { type: "canvas", canvas, coordinates: COORDS, animate: false });
  map.addLayer(
    { id: "fd-icing", type: "raster", source: "fd-icing", paint: { "raster-opacity": 1, "raster-resampling": "linear", "raster-fade-duration": 0 } },
    map.getLayer("routes-line") ? "routes-line" : undefined,
  );
}

export function showIcing(map: maplibregl.Map, on: boolean) {
  if (map.getLayer("fd-icing")) map.setLayoutProperty("fd-icing", "visibility", on ? "visible" : "none");
}

/** Flight level nearest an altitude, from those offered. */
export function flFor(ft: number): number {
  const fl = ft / 100;
  return ICING_FLS.reduce((best, x) => (Math.abs(x - fl) < Math.abs(best - fl) ? x : best), ICING_FLS[0]);
}
