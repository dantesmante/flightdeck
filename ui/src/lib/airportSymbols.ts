// World airports drawn after the VFR sectional chart legend: blue where the
// airport has a control tower, magenta everywhere else. Zoomed out, plain
// dots; zoomed in, each airport's chart symbol:
//   - a ring: runways other than hard-surfaced;
//   - a filled circle with its runways in white: hard runways 1,500 to
//     8,069 ft;
//   - the runway layout itself: longer, or several hard runways;
//   - a double ring: military, other than hard-surfaced;
//   - an anchor: seaplane base.
// Each symbol is drawn once, the first time the map needs it (the runway
// layout is the airport's own, pointed the way its runways really go).

export const TOWER_BLUE = "#4a90e8";
export const NO_TOWER_MAGENTA = "#c4569f";

/** The airport's details the symbol needs (from the map's airport list). */
export type SymbolInfo = { s: number; t: boolean; p: string };

type Run = { h: number; l: number; x: number; y: number };

function runs(p: string): Run[] {
  if (!p) return [];
  return p
    .split(";")
    .map((r) => r.split(",").map(Number))
    .filter((a) => a.length === 4 && a.every(Number.isFinite))
    .map(([h, l, x, y]) => ({ h, l: Math.max(l, 10), x, y }));
}

/** Runway end points, scaled so the whole layout fits a radius `fit` (pixels), north up. */
function layout(list: Run[], fit: number): [number, number, number, number][] {
  const segs = list.map((r) => {
    const a = (r.h * Math.PI) / 180;
    const dx = (Math.sin(a) * r.l) / 2;
    const dy = (Math.cos(a) * r.l) / 2;
    // Screen y grows downwards: north is up.
    return [r.x - dx, -(r.y - dy), r.x + dx, -(r.y + dy)] as [number, number, number, number];
  });
  let ext = 1;
  for (const [x1, y1, x2, y2] of segs) ext = Math.max(ext, Math.hypot(x1, y1), Math.hypot(x2, y2));
  const k = fit / ext;
  return segs.map(([x1, y1, x2, y2]) => [x1 * k, y1 * k, x2 * k, y2 * k]);
}

/** The symbol as an image (device pixels; add with pixelRatio 2). */
export function airportSymbol(info: SymbolInfo): ImageData {
  const color = info.t ? TOWER_BLUE : NO_TOWER_MAGENTA;
  const list = runs(info.p);
  const S = info.s === 2 ? 68 : info.s === 4 ? 40 : 40;
  const c = document.createElement("canvas");
  c.width = c.height = S;
  const g = c.getContext("2d")!;
  g.translate(S / 2, S / 2);
  g.lineCap = "round";
  g.lineJoin = "round";
  // A dark edge keeps the symbol readable on any map.
  const halo = "rgba(14,15,18,0.85)";
  const ring = (r: number, w: number) => {
    g.beginPath();
    g.arc(0, 0, r, 0, Math.PI * 2);
    g.strokeStyle = halo;
    g.lineWidth = w + 3;
    g.stroke();
    g.strokeStyle = color;
    g.lineWidth = w;
    g.stroke();
  };
  if (info.s === 1 && list.length) {
    g.beginPath();
    g.arc(0, 0, 16, 0, Math.PI * 2);
    g.fillStyle = color;
    g.fill();
    g.strokeStyle = halo;
    g.lineWidth = 2.5;
    g.stroke();
    g.strokeStyle = "#ffffff";
    g.lineWidth = 3.4;
    for (const [x1, y1, x2, y2] of layout(list, 10.5)) {
      g.beginPath();
      g.moveTo(x1, y1);
      g.lineTo(x2, y2);
      g.stroke();
    }
  } else if (info.s === 2 && list.length) {
    const segs = layout(list, 28);
    const pass = (style: string, w: number) => {
      g.strokeStyle = style;
      g.lineWidth = w;
      g.lineCap = "butt";
      for (const [x1, y1, x2, y2] of segs) {
        g.beginPath();
        g.moveTo(x1, y1);
        g.lineTo(x2, y2);
        g.stroke();
      }
    };
    pass(halo, 13);
    pass(color, 9.5);
    pass("#ffffff", 2.6);
  } else if (info.s === 3) {
    ring(12, 3.6);
    ring(6, 3.6);
  } else if (info.s === 4) {
    // An anchor.
    g.strokeStyle = halo;
    g.lineWidth = 7;
    const anchor = () => {
      g.beginPath();
      g.arc(0, -11, 3.2, 0, Math.PI * 2);
      g.moveTo(0, -7.8);
      g.lineTo(0, 12);
      g.moveTo(-6.5, -3);
      g.lineTo(6.5, -3);
      g.moveTo(-11, 4);
      g.quadraticCurveTo(-9, 12.5, 0, 12.5);
      g.quadraticCurveTo(9, 12.5, 11, 4);
      g.stroke();
    };
    anchor();
    g.strokeStyle = color;
    g.lineWidth = 3.4;
    anchor();
  } else {
    // Other than hard-surfaced (or runways not known): a ring.
    g.beginPath();
    g.arc(0, 0, 10, 0, Math.PI * 2);
    g.fillStyle = "rgba(14,15,18,0.55)";
    g.fill();
    ring(10, 3.8);
  }
  return g.getImageData(0, 0, S, S);
}
