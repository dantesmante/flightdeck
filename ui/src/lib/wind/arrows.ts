// Airport wind arrows: small pictures drawn once and handed to the map,
// one per length (light, moderate, strong) and colour. Each points up (to
// the north) and is turned on the map to point where the wind blows.

import type { Map as MlMap } from "maplibre-gl";
import { WIND_STOPS } from "./wind";

/** Arrow length (logical px) for under 10, 10 to 20, and 20+ knots. */
const LENGTHS = [34, 50, 68];
const WIDTH = 22;
const RATIO = 2;

function arrow(len: number, color: string): ImageData {
  const w = WIDTH * RATIO;
  const h = len * RATIO;
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  const g = c.getContext("2d")!;
  g.scale(RATIO, RATIO);
  const mid = WIDTH / 2;
  const head = 11;
  const draw = (stroke: string, width: number) => {
    g.strokeStyle = stroke;
    g.fillStyle = stroke;
    g.lineWidth = width;
    g.lineCap = "round";
    g.lineJoin = "round";
    g.beginPath();
    g.moveTo(mid, len - 3);
    g.lineTo(mid, head);
    g.stroke();
    g.beginPath();
    g.moveTo(mid, 1.5);
    g.lineTo(mid + 6.5, head + 2);
    g.lineTo(mid - 6.5, head + 2);
    g.closePath();
    g.fill();
    g.stroke();
    // Tail: a small bar across the upwind end.
    g.beginPath();
    g.moveTo(mid - 4, len - 3);
    g.lineTo(mid + 4, len - 3);
    g.stroke();
  };
  draw("rgba(12,13,15,0.85)", 5.5);
  draw(color, 2.6);
  return g.getImageData(0, 0, w, h);
}

function ring(color: string): ImageData {
  const s = 18 * RATIO;
  const c = document.createElement("canvas");
  c.width = s;
  c.height = s;
  const g = c.getContext("2d")!;
  g.scale(RATIO, RATIO);
  g.lineWidth = 4.5;
  g.strokeStyle = "rgba(12,13,15,0.85)";
  g.beginPath();
  g.arc(9, 9, 6, 0, Math.PI * 2);
  g.stroke();
  g.lineWidth = 2.2;
  g.strokeStyle = color;
  g.stroke();
  return g.getImageData(0, 0, s, s);
}

/** Adds the arrow pictures to the map (again after a style change). */
export function addWindIcons(map: MlMap) {
  WIND_STOPS.forEach(([, color], b) => {
    LENGTHS.forEach((len, l) => {
      const id = `wind-${l}-${b}`;
      if (!map.hasImage(id)) map.addImage(id, arrow(len, color), { pixelRatio: RATIO });
    });
    if (!map.hasImage(`wind-ring-${b}`)) map.addImage(`wind-ring-${b}`, ring(color), { pixelRatio: RATIO });
  });
}

/**
 * The map expression for an arrow's picture from its properties: kt
 * (speed), dir (from, -1 variable). Calm and variable winds get a ring.
 */
export function windIconExpr(): unknown {
  const bucket: unknown[] = ["step", ["get", "kt"], 0];
  WIND_STOPS.slice(1).forEach(([kt], i) => bucket.push(kt, i + 1));
  const len = ["step", ["get", "kt"], 0, 10, 1, 20, 2];
  return [
    "case",
    ["any", ["<", ["get", "dir"], 0], ["<", ["get", "kt"], 1]],
    ["concat", "wind-ring-", ["to-string", bucket]],
    ["concat", "wind-", ["to-string", len], "-", ["to-string", bucket]],
  ];
}

/** Short wind text: "170° 12G20", "VRB 3", "Calm". */
export function windText(dir: number | null, kt: number, gust: number): string {
  if (kt < 1) return "Calm";
  const d = dir == null || dir < 0 ? "VRB" : `${String(Math.round(dir) % 360 || 360).padStart(3, "0")}°`;
  return `${d} ${Math.round(kt)}${gust > 0 ? `G${Math.round(gust)}` : ""} kt`;
}
