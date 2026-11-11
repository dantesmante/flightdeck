// World airports drawn after the VFR sectional chart legend, kept light:
// blue where the airport has a control tower, magenta everywhere else.
// Every airport sits inside its circle:
//   - a filled circle, with its runways as simple white lines;
//   - ticks around the circle: fuel sold there;
//   - a ring with an R inside: a private airport;
//   - an anchor inside: a seaplane base.
// Only a handful of images exist (colour × kind × fuel), drawn once when the
// map loads; the runways are one white bar image, turned and placed per
// runway by the map itself. Nothing is drawn per airport, so zooming in
// costs nothing.

import type maplibregl from "maplibre-gl";

export const TOWER_BLUE = "#4a90e8";
export const NO_TOWER_MAGENTA = "#c4569f";

/** The circle's radius (CSS pixels at icon size 1). */
export const APT_R = 8;
/** How far from the middle the runway layout may reach (CSS pixels). */
const FIT = 5.6;
/** Runway bar lengths drawn (CSS pixels); each runway takes the nearest. */
const BARS = [4, 6, 8, 10, 11.2];

type Run = { h: number; l: number; x: number; y: number };

export function runs(p: string): Run[] {
  if (!p) return [];
  return p
    .split(";")
    .map((r) => r.split(",").map(Number))
    .filter((a) => a.length === 4 && a.every(Number.isFinite))
    .map(([h, l, x, y]) => ({ h, l: Math.max(l, 10), x, y }));
}

/** Whether the runways can be told apart in the circle: no more than two
 * running the same way, and each parallel pair far enough apart. */
function fits(list: Run[]): boolean {
  const ways: number[] = [];
  for (const r of list) {
    const same = list.filter((o) => {
      const d = Math.abs(o.h - r.h) % 180;
      return Math.min(d, 180 - d) <= 10;
    }).length;
    ways.push(same);
  }
  return Math.max(...ways) <= 2;
}

/** The symbol image's name: colour (b/m), kind (d dot, r private, s seaplane), f with fuel. */
export function symbolName(towered: boolean, kind: "d" | "r" | "s", fuel: boolean): string {
  return `apt-${towered ? "b" : "m"}${kind}${fuel ? "f" : ""}`;
}

/** Runways of one airport as map points: one bar each, turned to its
 * heading and set off from the middle the way the runway lies. */
export function runwayFeatures(p: string, at: [number, number]): GeoJSON.Feature[] {
  let list = runs(p);
  if (!list.length) return [];
  // More runways than the little circle can show clearly (O'Hare, say):
  // just the longest, through the middle.
  if (list.length > 4 || !fits(list)) {
    const top = list.reduce((a, b) => (b.l > a.l ? b : a));
    list = [{ ...top, x: 0, y: 0 }];
  }
  // Scale so the whole layout fits inside the circle.
  let ext = 1;
  for (const r of list) {
    const a = (r.h * Math.PI) / 180;
    const dx = (Math.sin(a) * r.l) / 2;
    const dy = (Math.cos(a) * r.l) / 2;
    ext = Math.max(ext, Math.hypot(r.x - dx, r.y - dy), Math.hypot(r.x + dx, r.y + dy));
  }
  const k = FIT / ext;
  // Parallel runways side by side are often too close to tell apart at
  // this size: spread their middles until they are at least 2.6 px apart.
  let spread = 1;
  for (let i = 0; i < list.length; i++)
    for (let j = i + 1; j < list.length; j++) {
      const [a, b] = [list[i], list[j]];
      const dh = Math.abs(((a.h - b.h + 540) % 180) - 0) % 180;
      if (Math.min(dh, 180 - dh) > 10) continue;
      const t = (a.h * Math.PI) / 180;
      const side = Math.abs((b.x - a.x) * Math.cos(t) - (b.y - a.y) * Math.sin(t)) * k;
      if (side > 0.01 && side < 2.6) spread = Math.max(spread, Math.min(4, 2.6 / side));
    }
  return list.map((r) => {
    const len = r.l * k;
    let bar = 0;
    for (let i = 1; i < BARS.length; i++) if (Math.abs(BARS[i] - len) < Math.abs(BARS[bar] - len)) bar = i;
    // The offset is given as if the turned bar pointed up: turn the
    // runway's middle (east x, north y; screen y down) back by its heading.
    const a = (r.h * Math.PI) / 180;
    const sx = r.x * k * spread;
    const sy = -r.y * k * spread;
    const ox = sx * Math.cos(a) + sy * Math.sin(a);
    const oy = -sx * Math.sin(a) + sy * Math.cos(a);
    return {
      type: "Feature",
      properties: { b: `apt-rwy${bar}`, h: r.h, o: [Math.round(ox * 10) / 10, Math.round(oy * 10) / 10] },
      geometry: { type: "Point", coordinates: at },
    };
  });
}

const HALO = "rgba(14,15,18,0.85)";

function symbol(color: string, kind: "d" | "r" | "s", fuel: boolean): ImageData {
  const S = 48; // device pixels (pixel ratio 2): 24 CSS px
  const R = APT_R * 2;
  const c = document.createElement("canvas");
  c.width = c.height = S;
  const g = c.getContext("2d")!;
  g.translate(S / 2, S / 2);
  if (fuel) {
    // Four ticks, as the chart legend draws fuel.
    g.lineCap = "butt";
    for (const [w, style] of [
      [6.5, HALO],
      [3.6, color],
    ] as const) {
      g.lineWidth = w;
      g.strokeStyle = style;
      for (let i = 0; i < 4; i++) {
        const a = (i * Math.PI) / 2;
        g.beginPath();
        g.moveTo(Math.sin(a) * (R - 1), -Math.cos(a) * (R - 1));
        g.lineTo(Math.sin(a) * (R + 6), -Math.cos(a) * (R + 6));
        g.stroke();
      }
    }
  }
  g.beginPath();
  g.arc(0, 0, R, 0, Math.PI * 2);
  if (kind === "r") {
    g.fillStyle = "rgba(14,15,18,0.8)";
    g.fill();
    g.lineWidth = 6;
    g.strokeStyle = HALO;
    g.stroke();
    g.lineWidth = 3.4;
    g.strokeStyle = color;
    g.stroke();
    g.fillStyle = color;
    g.font = "bold 19px 'IBM Plex Sans', system-ui, sans-serif";
    g.textAlign = "center";
    g.textBaseline = "middle";
    g.fillText("R", 0, 1.5);
  } else {
    g.fillStyle = color;
    g.fill();
    g.lineWidth = 2.4;
    g.strokeStyle = HALO;
    g.stroke();
    if (kind === "s") {
      // A small white anchor.
      g.strokeStyle = "#ffffff";
      g.lineWidth = 2.4;
      g.lineCap = "round";
      g.beginPath();
      g.arc(0, -7.5, 2.2, 0, Math.PI * 2);
      g.moveTo(0, -5.3);
      g.lineTo(0, 9);
      g.moveTo(-4.5, -2);
      g.lineTo(4.5, -2);
      g.moveTo(-8, 3);
      g.quadraticCurveTo(-6.5, 9.5, 0, 9.5);
      g.quadraticCurveTo(6.5, 9.5, 8, 3);
      g.stroke();
    }
  }
  return g.getImageData(0, 0, S, S);
}

function bar(len: number): ImageData {
  const w = Math.round(len * 2);
  const c = document.createElement("canvas");
  c.width = 4;
  c.height = Math.max(2, w);
  const g = c.getContext("2d")!;
  g.fillStyle = "#ffffff";
  g.fillRect(0.6, 0, 2.8, c.height);
  return g.getImageData(0, 0, c.width, c.height);
}

/** Puts every airport image on the map once (a dozen small ones). */
export function addAirportImages(map: maplibregl.Map) {
  for (const t of [true, false])
    for (const kind of ["d", "r", "s"] as const)
      for (const f of [false, true]) {
        const id = symbolName(t, kind, f);
        if (!map.hasImage(id)) map.addImage(id, symbol(t ? TOWER_BLUE : NO_TOWER_MAGENTA, kind, f), { pixelRatio: 2 });
      }
  BARS.forEach((l, i) => {
    if (!map.hasImage(`apt-rwy${i}`)) map.addImage(`apt-rwy${i}`, bar(l), { pixelRatio: 2 });
  });
}
