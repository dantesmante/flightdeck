// The Nav aids map layer: VORs, NDBs and waypoints from the sim's own
// navigation database (FlightDeck asks MSFS for them while the layer is on
// and keeps a copy). Only what is in view is fetched, after the map stops
// moving; waypoints from zoom 8, radio aids from zoom 5.

import type maplibregl from "maplibre-gl";
import type { GeoJSONSource } from "maplibre-gl";
import type { NavAid } from "./api";

export const NAV_IDS = ["fd-nav-wpt", "fd-nav-radio", "fd-nav-wpt-label", "fd-nav-radio-label"];
export const NAV_MIN_ZOOM = 5;
export const WPT_MIN_ZOOM = 8;

const VOR = "#5aa9ff";
const NDB = "#c98bff";
const WPT = "#c9ced6";

/** Small symbols drawn once on a canvas: no image files. */
function icon(kind: "vor" | "vordme" | "dme" | "ndb" | "wpt"): ImageData {
  const s = 32;
  const c = document.createElement("canvas");
  c.width = c.height = s;
  const g = c.getContext("2d")!;
  g.lineWidth = 2.4;
  g.lineJoin = "round";
  const hex = (r: number) => {
    g.beginPath();
    for (let i = 0; i < 6; i++) {
      const a = (Math.PI / 3) * i;
      g.lineTo(16 + r * Math.cos(a), 16 + r * Math.sin(a));
    }
    g.closePath();
  };
  if (kind === "vor" || kind === "vordme" || kind === "dme") {
    g.strokeStyle = VOR;
    g.fillStyle = "rgba(20,24,30,0.85)";
    if (kind !== "vor") {
      g.fillRect(4, 6, 24, 20);
      g.strokeRect(4, 6, 24, 20);
    }
    if (kind !== "dme") {
      hex(kind === "vor" ? 12 : 9);
      g.fill();
      g.stroke();
    }
    g.fillStyle = VOR;
    g.beginPath();
    g.arc(16, 16, 2.2, 0, Math.PI * 2);
    g.fill();
  } else if (kind === "ndb") {
    g.fillStyle = NDB;
    for (const [r, n] of [
      [12, 14],
      [7.5, 9],
    ] as const)
      for (let i = 0; i < n; i++) {
        const a = ((Math.PI * 2) / n) * i;
        g.beginPath();
        g.arc(16 + r * Math.cos(a), 16 + r * Math.sin(a), 1.4, 0, Math.PI * 2);
        g.fill();
      }
    g.beginPath();
    g.arc(16, 16, 2.6, 0, Math.PI * 2);
    g.fill();
  } else {
    g.strokeStyle = WPT;
    g.fillStyle = "rgba(20,24,30,0.7)";
    g.beginPath();
    g.moveTo(16, 6);
    g.lineTo(26, 24);
    g.lineTo(6, 24);
    g.closePath();
    g.fill();
    g.stroke();
  }
  return g.getImageData(0, 0, s, s);
}

export function addNavLayers(map: maplibregl.Map) {
  if (map.getSource("fd-nav")) return;
  for (const k of ["vor", "vordme", "dme", "ndb", "wpt"] as const) if (!map.hasImage(`nav-${k}`)) map.addImage(`nav-${k}`, icon(k), { pixelRatio: 2 });
  map.addSource("fd-nav", { type: "geojson", data: { type: "FeatureCollection", features: [] } });
  const before = map.getLayer("routes-line") ? "routes-line" : undefined;
  const font = ["Noto Sans Bold"];
  map.addLayer(
    {
      id: "fd-nav-wpt",
      type: "symbol",
      source: "fd-nav",
      minzoom: WPT_MIN_ZOOM,
      filter: ["==", ["get", "k"], "wpt"],
      layout: { "icon-image": "nav-wpt", "icon-size": ["interpolate", ["linear"], ["zoom"], 8, 0.7, 12, 1], "icon-allow-overlap": true },
    },
    before,
  );
  map.addLayer(
    {
      id: "fd-nav-wpt-label",
      type: "symbol",
      source: "fd-nav",
      minzoom: 9,
      filter: ["==", ["get", "k"], "wpt"],
      layout: { "text-field": ["get", "id"], "text-font": ["Noto Sans Regular"], "text-size": 10.5, "text-offset": [0, 0.95], "text-anchor": "top", "text-optional": true },
      paint: { "text-color": WPT, "text-halo-color": "rgba(15,17,20,0.9)", "text-halo-width": 1.4 },
    },
    before,
  );
  map.addLayer(
    {
      id: "fd-nav-radio",
      type: "symbol",
      source: "fd-nav",
      minzoom: NAV_MIN_ZOOM,
      filter: ["!=", ["get", "k"], "wpt"],
      layout: {
        "icon-image": ["concat", "nav-", ["get", "k"]],
        "icon-size": ["interpolate", ["linear"], ["zoom"], 5, 0.85, 9, 1.25, 12, 1.4],
        "icon-allow-overlap": true,
      },
    },
    before,
  );
  map.addLayer(
    {
      id: "fd-nav-radio-label",
      type: "symbol",
      source: "fd-nav",
      minzoom: 6.5,
      filter: ["!=", ["get", "k"], "wpt"],
      layout: {
        "text-field": ["format", ["get", "id"], { "font-scale": 1 }, "\n", {}, ["get", "f"], { "font-scale": 0.85 }],
        "text-font": font,
        "text-size": 11.5,
        "text-offset": [0, 1.35],
        "text-anchor": "top",
        "text-optional": true,
      },
      paint: { "text-color": ["match", ["get", "k"], "ndb", NDB, VOR], "text-halo-color": "rgba(15,17,20,0.92)", "text-halo-width": 1.5 },
    },
    before,
  );
}

export function showNavLayers(map: maplibregl.Map, on: boolean) {
  if (on) addNavLayers(map);
  for (const id of NAV_IDS) if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", on ? "visible" : "none");
}

/** "116.80" for a VOR, "350" (kHz) for an NDB. */
export function navFreq(a: NavAid): string {
  const f = a.freq_hz;
  if (!f) return "";
  if (a.kind === "vor") return (f >= 1e6 ? f / 1e6 : f / 1e3 >= 100 ? f / 1e3 : f / 100).toFixed(2);
  const khz = f >= 1e5 ? f / 1e3 : f;
  return Number.isInteger(khz) ? String(khz) : khz.toFixed(1);
}

export function setNavData(map: maplibregl.Map, items: NavAid[]) {
  const src = map.getSource("fd-nav") as GeoJSONSource | undefined;
  if (!src) return;
  src.setData({
    type: "FeatureCollection",
    features: items.map((a) => ({
      type: "Feature",
      properties: {
        k: a.kind === "waypoint" ? "wpt" : a.kind === "ndb" ? "ndb" : a.dme_only ? "dme" : a.dme ? "vordme" : "vor",
        id: a.ident,
        f: navFreq(a),
        r: a.region,
      },
      geometry: { type: "Point", coordinates: [a.lon, a.lat] },
    })),
  });
}

const KIND_NAME: Record<string, string> = { vor: "VOR", vordme: "VOR-DME", dme: "DME", ndb: "NDB", wpt: "Waypoint / intersection" };

/** "N39.1238° W121.7695°" */
function coord(lat: number, lon: number): string {
  return `${lat >= 0 ? "N" : "S"}${Math.abs(lat).toFixed(4)}° ${lon >= 0 ? "E" : "W"}${Math.abs(lon).toFixed(4)}°`;
}

/** The nav aid under a point on the map (a few pixels of slack), if any. */
export function navAt(map: maplibregl.Map, p: { x: number; y: number }): maplibregl.MapGeoJSONFeature | undefined {
  const layers = NAV_IDS.filter((id) => map.getLayer(id) && map.getLayoutProperty(id, "visibility") !== "none");
  if (!layers.length) return undefined;
  const r = 7;
  const near = map.queryRenderedFeatures(
    [
      [p.x - r, p.y - r],
      [p.x + r, p.y + r],
    ],
    { layers },
  );
  // Radio aids before waypoints, then the closest.
  const at = (f: maplibregl.MapGeoJSONFeature) => {
    const c = map.project((f.geometry as GeoJSON.Point).coordinates as [number, number]);
    return (f.properties.k === "wpt" ? 1000 : 0) + Math.hypot(c.x - p.x, c.y - p.y);
  };
  return near.sort((a, b) => at(a) - at(b))[0];
}

/** The little card for a clicked nav aid. */
export function navCard(f: maplibregl.MapGeoJSONFeature, onclose: () => void): HTMLElement {
  const p = f.properties as { k: string; id: string; f?: string; r?: string };
  const [lon, lat] = (f.geometry as GeoJSON.Point).coordinates;
  const box = document.createElement("div");
  box.className = "nav-card";
  const head = document.createElement("div");
  head.className = "nc-head";
  const title = document.createElement("strong");
  title.textContent = p.id;
  const x = document.createElement("button");
  x.type = "button";
  x.className = "nc-x";
  x.setAttribute("aria-label", "Close");
  x.textContent = "×";
  x.onclick = onclose;
  head.append(title, x);
  const rows: [string, string][] = [["Type", KIND_NAME[p.k] ?? p.k]];
  if (p.f) rows.push(["Frequency", p.k === "ndb" ? `${p.f} kHz` : `${p.f} MHz`]);
  if (p.r) rows.push(["Region", p.r]);
  rows.push(["Coordinates", coord(lat, lon)]);
  const dl = document.createElement("dl");
  for (const [k, v] of rows) {
    const dt = document.createElement("dt");
    dt.textContent = k;
    const dd = document.createElement("dd");
    dd.textContent = v;
    dl.append(dt, dd);
  }
  const src = document.createElement("p");
  src.className = "nc-src";
  src.textContent = "From MSFS's navigation data";
  box.append(head, dl, src);
  return box;
}
