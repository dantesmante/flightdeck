// Live precipitation radar from RainViewer (free for personal use; global,
// tiles up to zoom 7, last 2 hours in 10-minute frames).
// https://www.rainviewer.com/api.html

export interface RadarFrame {
  /** Unix seconds. */
  time: number;
  /** Tile URL template for MapLibre. */
  tiles: string;
}

const API = "https://api.rainviewer.com/public/weather-maps.json";
/** 256 px tiles, color scheme 2 ("Universal Blue", the free one),
 * smoothed, snow shown in its own colors. */
const TILE = "/256/{z}/{x}/{y}/2/1_1.png";

// --- Colours as pilots know them -------------------------------------------
// RainViewer's free tiles come only in its "Universal Blue" scheme. Each
// tile is recoloured here as it arrives: rain from green (light) through
// yellow and orange to red and magenta (heavy), as on weather radar
// displays; snow stays in its own pale blues. RainViewer's published colour
// table gives the reflectivity (dBZ) of every colour.

/** Universal Blue rain colours by dBZ (RainViewer's colour table), from -10 to 64. */
const RAIN: [number, number, number, number, number][] = [];
{
  const hex = "63615914 66635a19 69665c1e 6c685d24 6f6b5f29 726e612e 75706234 78736439 7c75653e 7f786744 827b6949 857d6a4e 88806c54 8b826d59 8e856f5e 92887164 9e93756e aa9e7978 b6a97e82 c2b4828c cec08796 d2c48ba0 d6c88faa dacc93b4 ded097be 88ddeeff 6cd1ebff 51c5e8ff 36bae5ff 1baee2ff 00a3e0ff 009ad5ff 0091caff 0088bfff 007fb4ff 0077aaff 0070a3ff 00699cff 006295ff 005b8eff 005588ff 005180ff 004e78ff 004a70ff 004768ff ffee00ff ffe000ff ffd200ff ffc500ff ffb700ff ffaa00ff ff9f00ff ff9500ff ff8b00ff ff8100ff ff4400ff f23600ff e62800ff d91b00ff cd0d00ff c10000ff a80000ff 8f0000ff 760000ff 5d0000ff ffaaffff ff9fffff ff95ffff ff8bffff ff81ffff ff77ffff ff6cffff ff62ffff ff58ffff ff4effff".split(" ");
  hex.forEach((h, i) => RAIN.push([i - 10, parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16), parseInt(h.slice(6, 8), 16)]));
}
/** Universal Blue snow colours (pale cyan to blue): left as they are. */
const SNOW: [number, number, number][] = ["bfffff", "a5e5ff", "8bcbff", "7fbfff", "65a5ff", "4f8fff", "3878ff", "2262ff", "0c4bff", "0035ff", "0000ff"].map((h) => [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)]);

/** The new colour for a reflectivity, as radar displays show it. */
function classic(dbz: number): [number, number, number] {
  const stops: [number, [number, number, number]][] = [
    [5, [120, 200, 120]],
    [15, [80, 200, 80]],
    [25, [20, 150, 40]],
    [30, [10, 110, 30]],
    [35, [245, 230, 40]],
    [40, [250, 190, 20]],
    [45, [245, 120, 20]],
    [50, [225, 30, 30]],
    [55, [170, 0, 20]],
    [60, [230, 40, 210]],
    [65, [255, 255, 255]],
  ];
  if (dbz <= stops[0][0]) return stops[0][1];
  for (let i = 1; i < stops.length; i++) {
    const [d1, c1] = stops[i];
    const [d0, c0] = stops[i - 1];
    if (dbz <= d1) {
      const t = (dbz - d0) / (d1 - d0);
      return [0, 1, 2].map((k) => Math.round(c0[k] + (c1[k] - c0[k]) * t)) as [number, number, number];
    }
  }
  return stops[stops.length - 1][1];
}

/** The key's colour bar, light to heavy. */
export const RADAR_BAR = `linear-gradient(90deg, ${[5, 15, 25, 30, 35, 40, 45, 50, 55, 60].map((d, i) => `rgb(${classic(d).join(",")}) ${i * 11}%`).join(", ")})`;

/** A lookup from the tile's colour (5 bits a channel) to the new one; -1 keeps it (snow). */
let LUT: Int32Array | null = null;
function lut(): Int32Array {
  if (LUT) return LUT;
  const t = new Int32Array(32 * 32 * 32);
  for (let r = 0; r < 32; r++)
    for (let g = 0; g < 32; g++)
      for (let b = 0; b < 32; b++) {
        const R = r * 8 + 4;
        const G = g * 8 + 4;
        const B = b * 8 + 4;
        // Nearest rain colour (opaque ones, 15 dBZ up) or snow colour.
        let best = 1e9;
        let dbz = 0;
        for (const [d, cr, cg, cb, a] of RAIN) {
          if (a < 255) continue;
          const e = (R - cr) ** 2 + (G - cg) ** 2 + (B - cb) ** 2;
          if (e < best) [best, dbz] = [e, d];
        }
        let snow = 1e9;
        for (const [cr, cg, cb] of SNOW) snow = Math.min(snow, (R - cr) ** 2 + (G - cg) ** 2 + (B - cb) ** 2);
        if (snow < best * 0.6) t[(r << 10) | (g << 5) | b] = -1;
        else {
          const [nr, ng, nb] = classic(dbz);
          t[(r << 10) | (g << 5) | b] = (nr << 16) | (ng << 8) | nb;
        }
      }
  LUT = t;
  return t;
}

/** One tile, recoloured: light (see-through) drizzle pale green, the rest by its dBZ. */
async function recolour(url: string, signal: AbortSignal): Promise<ArrayBuffer> {
  const res = await fetch(url, { signal });
  if (!res.ok) throw new Error(`radar tile ${res.status}`);
  const bmp = await createImageBitmap(await res.blob());
  const c = new OffscreenCanvas(bmp.width, bmp.height);
  const g = c.getContext("2d", { willReadFrequently: true })!;
  g.drawImage(bmp, 0, 0);
  const img = g.getImageData(0, 0, c.width, c.height);
  const d = img.data;
  const t = lut();
  for (let i = 0; i < d.length; i += 4) {
    const a = d[i + 3];
    if (a === 0) continue;
    if (a < 250) {
      // Drizzle (below 15 dBZ): drawn see-through; a faint green.
      d[i] = 120;
      d[i + 1] = 200;
      d[i + 2] = 120;
      continue;
    }
    const v = t[((d[i] >> 3) << 10) | ((d[i + 1] >> 3) << 5) | (d[i + 2] >> 3)];
    if (v < 0) continue;
    d[i] = (v >> 16) & 255;
    d[i + 1] = (v >> 8) & 255;
    d[i + 2] = v & 255;
  }
  g.putImageData(img, 0, 0);
  return (await c.convertToBlob({ type: "image/png" })).arrayBuffer();
}

let registered = false;
/** Tiles asked for as fdradar://https://... arrive recoloured. */
export function registerRadarColours(add: (name: string, fn: (p: { url: string }, a: AbortController) => Promise<{ data: ArrayBuffer }>) => void) {
  if (registered) return;
  registered = true;
  add("fdradar", async (p, a) => ({ data: await recolour(p.url.replace(/^fdradar:\/\//, ""), a.signal) }));
}

/** The newest radar frame, or null if RainViewer could not be reached. */
export async function latestRadar(): Promise<RadarFrame | null> {
  try {
    const res = await fetch(API, { cache: "no-store" });
    if (!res.ok) return null;
    const j = (await res.json()) as { host: string; radar?: { past?: { time: number; path: string }[] } };
    const past = j.radar?.past ?? [];
    const f = past[past.length - 1];
    if (!f || !j.host) return null;
    return { time: f.time, tiles: `fdradar://${j.host}${f.path}${TILE}` };
  } catch {
    return null;
  }
}
