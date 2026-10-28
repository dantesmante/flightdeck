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

/** The newest radar frame, or null if RainViewer could not be reached. */
export async function latestRadar(): Promise<RadarFrame | null> {
  try {
    const res = await fetch(API, { cache: "no-store" });
    if (!res.ok) return null;
    const j = (await res.json()) as { host: string; radar?: { past?: { time: number; path: string }[] } };
    const past = j.radar?.past ?? [];
    const f = past[past.length - 1];
    if (!f || !j.host) return null;
    return { time: f.time, tiles: `${j.host}${f.path}${TILE}` };
  } catch {
    return null;
  }
}
