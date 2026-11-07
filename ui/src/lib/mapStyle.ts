// The night-chart look shared by the main map and the logbook's mini map:
// OpenFreeMap vector tiles recolored at load, and a plain grid offline.
import maplibre from "maplibre-gl";
import type maplibregl from "maplibre-gl";
import mlcontour from "maplibre-contour";

export const STYLE = "https://tiles.openfreemap.org/styles/positron";

export const C = {
  // A quiet, low-contrast gray chart, so flights stand out on top of it.
  land: "#1b1c1e",
  water: "#141516",
  green: "#1c1e1e",
  built: "#1f2022",
  road: "#242628",
  roadMajor: "#2b2d30",
  aeroway: "#3d4045",
  border: "#3e4146",
  borderStrong: "#7a8089",
  region: "#34373b",
  regionStrong: "#626870",
  text: "#6c7076",
  halo: "#141516",
  track: "#e2468f",
};

function setPaint(map: maplibregl.Map, id: string, prop: string, value: unknown) {
    try {
      map.setPaintProperty(id, prop, value);
    } catch {
      /* property not valid for this layer */
    }
  }

/** Recolor every layer of the base style by what it draws. */
export function tintMap(map: maplibregl.Map) {
    for (const layer of map.getStyle().layers ?? []) {
      const id = layer.id.toLowerCase();
      const src = "source-layer" in layer ? String(layer["source-layer"] ?? "") : "";
      const key = `${id} ${src}`;
      if (layer.type === "background") setPaint(map, layer.id, "background-color", C.land);
      else if (layer.type === "fill") {
        const color = key.includes("water")
          ? C.water
          : key.includes("aeroway")
            ? C.roadMajor
            : key.match(/park|wood|grass|landcover|forest/)
              ? C.green
              : key.match(/building|landuse|residential/)
                ? C.built
                : C.land;
        setPaint(map, layer.id, "fill-color", color);
        setPaint(map, layer.id, "fill-outline-color", color);
      } else if (layer.type === "line") {
        const color = key.includes("water")
          ? C.water
          : key.includes("aeroway")
            ? C.aeroway
            : key.match(/boundary|admin/)
              ? C.border
              : key.match(/motorway|trunk|primary|major/)
                ? C.roadMajor
                : C.road;
        setPaint(map, layer.id, "line-color", color);
      } else if (layer.type === "symbol") {
        if (key.match(/poi|shield|oneway/)) map.setLayoutProperty(layer.id, "visibility", "none");
        setPaint(map, layer.id, "text-color", C.text);
        setPaint(map, layer.id, "text-halo-color", C.halo);
      }
    }
  }


/** A 10 degree lat/lon grid, so a map still orients you with no tiles. */
export function graticule(): GeoJSON.FeatureCollection {
  const lines: GeoJSON.Feature[] = [];
  for (let lon = -180; lon <= 180; lon += 10) {
    lines.push({ type: "Feature", properties: {}, geometry: { type: "LineString", coordinates: [[lon, -85], [lon, 0], [lon, 85]] } });
  }
  for (let lat = -80; lat <= 80; lat += 10) {
    const coords: [number, number][] = [];
    for (let lon = -180; lon <= 180; lon += 5) coords.push([lon, lat]);
    lines.push({ type: "Feature", properties: {}, geometry: { type: "LineString", coordinates: coords } });
  }
  return { type: "FeatureCollection", features: lines };
}

export function offlineStyle(): maplibregl.StyleSpecification {
  return {
    version: 8,
    sources: { grid: { type: "geojson", data: graticule() } },
    layers: [
      { id: "bg", type: "background", paint: { "background-color": C.land } },
      { id: "grid", type: "line", source: "grid", paint: { "line-color": C.road, "line-width": 1 } },
    ],
  };
}

/** States, provinces and other regions (the base style only draws them
 * from zoom 8): dashed lines from the zoom where the tiles carry them. */
export function addRegionLines(map: maplibregl.Map) {
  if (map.getLayer("fd-regions") || !map.getSource("openmaptiles")) return;
  const before = (map.getStyle().layers ?? []).find((l) => l.id.startsWith("boundary"))?.id;
  map.addLayer(
    {
      id: "fd-regions",
      type: "line",
      source: "openmaptiles",
      "source-layer": "boundary",
      minzoom: 3,
      filter: ["all", [">=", ["get", "admin_level"], 3], ["<=", ["get", "admin_level"], 4], ["!=", ["get", "maritime"], 1]],
      layout: { "line-join": "round", "line-cap": "round" },
      paint: {
        "line-color": C.region,
        "line-width": ["interpolate", ["linear"], ["zoom"], 3, 0.6, 6, 1, 10, 1.6],
        "line-dasharray": [3, 2],
      },
    },
    before,
  );
}

/** Satellite pictures (Esri World Imagery) under the borders, labels and
 * your flights; the chart's own land, water and roads are covered. */
export const SATELLITE_ATTRIBUTION = "Imagery © Esri, Maxar, Earthstar Geographics, and the GIS User Community";
export function showSatellite(map: maplibregl.Map, on: boolean) {
  if (on && !map.getLayer("satellite")) {
    if (!map.getSource("satellite")) {
      map.addSource("satellite", {
        type: "raster",
        tiles: ["https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}"],
        tileSize: 256,
        maxzoom: 19,
        attribution: SATELLITE_ATTRIBUTION,
      });
    }
    const layers = map.getStyle().layers ?? [];
    // Under the terrain shading, if that is already on the map.
    const before = ["terrain-hillshade", "fd-regions"].find((id) => map.getLayer(id))
      ?? layers.find((l) => l.id.startsWith("boundary") || l.type === "symbol")?.id;
    map.addLayer({ id: "satellite", type: "raster", source: "satellite", paint: { "raster-fade-duration": 0 } }, before);
  }
  if (map.getLayer("satellite")) map.setLayoutProperty("satellite", "visibility", on ? "visible" : "none");
  // Labels stay readable on the pictures.
  for (const layer of map.getStyle().layers ?? []) {
    if (layer.type !== "symbol" || layer.id.startsWith("fd-") || "source" in layer === false) continue;
    if ((layer as { source?: string }).source !== "openmaptiles") continue;
    setPaint(map, layer.id, "text-color", on ? "#eceef1" : C.text);
    setPaint(map, layer.id, "text-halo-color", on ? "#0b0c0d" : C.halo);
  }
}

/** Elevation for the Topographic layer: Terrain Tiles, open elevation data
 * on Amazon's open-data program (no key). Shared by the relief shading and
 * the contour lines, which maplibre-contour works out in a background
 * worker, only for what is on screen. */
const DEM_URL = "https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png";
export const TERRAIN_ATTRIBUTION = "Elevation: Terrain Tiles (Mapzen, AWS Open Data; USGS, SRTM, GMTED, ETOPO1)";
let dem: InstanceType<typeof mlcontour.DemSource> | null = null;
function demSource(): InstanceType<typeof mlcontour.DemSource> {
  if (!dem) {
    dem = new mlcontour.DemSource({ url: DEM_URL, encoding: "terrarium", maxzoom: 12, worker: true, cacheSize: 120 });
    dem.setupMaplibre(maplibre);
  }
  return dem;
}

/** Relief shading and contour lines (feet), above the land and satellite
 * pictures and below borders, labels and flights. */
export function showTerrain(map: maplibregl.Map, on: boolean, satellite: boolean) {
  const ids = ["terrain-hillshade", "terrain-contours", "terrain-contour-labels"];
  if (on && !map.getLayer("terrain-hillshade")) {
    const src = demSource();
    if (!map.getSource("fd-dem")) {
      map.addSource("fd-dem", {
        type: "raster-dem",
        encoding: "terrarium",
        tiles: [src.sharedDemProtocolUrl],
        tileSize: 256,
        maxzoom: 12,
        attribution: TERRAIN_ATTRIBUTION,
      });
    }
    if (!map.getSource("fd-contours")) {
      map.addSource("fd-contours", {
        type: "vector",
        tiles: [
          src.contourProtocolUrl({
            multiplier: 3.28084,
            thresholds: { 9: [500, 2500], 10: [500, 2500], 11: [200, 1000], 12: [100, 500], 13: [100, 500], 14: [50, 250] },
            contourLayer: "contours",
            elevationKey: "ele",
            levelKey: "level",
          }),
        ],
        maxzoom: 14,
      });
    }
    const layers = map.getStyle().layers ?? [];
    const before = map.getLayer("fd-regions")
      ? "fd-regions"
      : layers.find((l) => l.id.startsWith("boundary") || l.type === "symbol")?.id;
    map.addLayer({ id: "terrain-hillshade", type: "hillshade", source: "fd-dem", paint: {} }, before);
    map.addLayer(
      {
        id: "terrain-contours",
        type: "line",
        source: "fd-contours",
        "source-layer": "contours",
        minzoom: 9,
        paint: { "line-width": ["match", ["get", "level"], 1, 1, 0.5] },
      },
      before,
    );
    if (map.getStyle().glyphs) {
      map.addLayer(
        {
          id: "terrain-contour-labels",
          type: "symbol",
          source: "fd-contours",
          "source-layer": "contours",
          minzoom: 11,
          filter: [">", ["get", "level"], 0],
          layout: {
            "symbol-placement": "line",
            "text-size": 10,
            "text-field": ["concat", ["number-format", ["get", "ele"], {}], " ft"],
            "text-font": ["Noto Sans Regular"],
          },
          paint: { "text-halo-width": 1 },
        },
        before,
      );
    }
  }
  for (const id of ids) if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", on ? "visible" : "none");
  if (!on || !map.getLayer("terrain-hillshade")) return;
  // Darker, quieter shading on the night chart; lighter over the pictures.
  setPaint(map, "terrain-hillshade", "hillshade-exaggeration", satellite ? 0.3 : 0.45);
  setPaint(map, "terrain-hillshade", "hillshade-shadow-color", satellite ? "rgba(0, 0, 0, 0.45)" : "#000000");
  setPaint(map, "terrain-hillshade", "hillshade-highlight-color", satellite ? "rgba(255, 255, 255, 0.25)" : "rgba(210, 214, 220, 0.18)");
  setPaint(map, "terrain-hillshade", "hillshade-accent-color", satellite ? "rgba(0, 0, 0, 0.2)" : "#000000");
  setPaint(map, "terrain-contours", "line-color", satellite ? "rgba(255, 245, 225, 0.45)" : "rgba(196, 184, 150, 0.32)");
  setPaint(map, "terrain-contour-labels", "text-color", satellite ? "#fff5e1" : "#b8ad8d");
  setPaint(map, "terrain-contour-labels", "text-halo-color", satellite ? "rgba(0, 0, 0, 0.7)" : "#141516");
}

/** Country borders and region lines: faint by default, or picked out
 * clearly; lighter over satellite pictures. */
const borderWidths = new WeakMap<maplibregl.Map, Map<string, unknown>>();
export function setBorders(map: maplibregl.Map, strong: boolean, satellite = false) {
  let widths = borderWidths.get(map);
  if (!widths) borderWidths.set(map, (widths = new Map()));
  for (const layer of map.getStyle().layers ?? []) {
    if (layer.type !== "line") continue;
    const src = "source-layer" in layer ? String(layer["source-layer"] ?? "") : "";
    const key = `${layer.id} ${src}`.toLowerCase();
    const region = layer.id === "fd-regions";
    if (!region && !key.match(/boundary|admin/)) continue;
    const color = satellite
      ? region
        ? "#d9dce1"
        : "#f2f3f5"
      : region
        ? strong
          ? C.regionStrong
          : C.region
        : strong
          ? C.borderStrong
          : C.border;
    setPaint(map, layer.id, "line-color", color);
    setPaint(map, layer.id, "line-opacity", satellite ? (strong ? 0.85 : 0.55) : strong ? 0.9 : 1);
    if (region) continue;
    // Remember the style's own width so turning it off restores it.
    if (!widths.has(layer.id)) widths.set(layer.id, map.getPaintProperty(layer.id, "line-width"));
    setPaint(
      map,
      layer.id,
      "line-width",
      strong ? ["interpolate", ["linear"], ["zoom"], 2, 1, 6, 1.6, 10, 2.2] : widths.get(layer.id),
    );
  }
}

/** Base map layers the airport layout replaces while it is on. */
const BASE_AEROWAY = ["aeroway-taxiway", "aeroway-runway-casing", "aeroway-area", "aeroway-runway"];

/**
 * Airport layout from the base map's OpenStreetMap aeroway data: aprons,
 * runways with their numbers, taxiways as yellow centrelines with their
 * letters, and gate numbers, once zoomed in on an airport. It costs no
 * extra download: it is already in the map tiles.
 */
export function addAirportLayout(map: maplibregl.Map) {
  if (!map.getSource("openmaptiles") || map.getLayer("fd-apron")) return;
  const layers = map.getStyle().layers ?? [];
  const before = layers.find((l) => l.type === "symbol")?.id;
  const glyphs = !!map.getStyle().glyphs;
  const cls = (...c: string[]) => ["match", ["get", "class"], c, true, false] as maplibregl.ExpressionSpecification;
  const poly = ["match", ["geometry-type"], ["Polygon", "MultiPolygon"], true, false] as maplibregl.ExpressionSpecification;
  const line = ["match", ["geometry-type"], ["LineString", "MultiLineString"], true, false] as maplibregl.ExpressionSpecification;
  const src = { source: "openmaptiles", "source-layer": "aeroway" } as const;
  const add = (l: maplibregl.LayerSpecification, at = before) => map.addLayer({ ...l, layout: { ...(l.layout ?? {}), visibility: "none" } } as maplibregl.LayerSpecification, at);
  add({ id: "fd-apron", type: "fill", ...src, minzoom: 12, filter: ["all", poly, cls("apron")], paint: { "fill-color": "#2a2c30", "fill-opacity": 0.9 } });
  add({ id: "fd-taxiway-area", type: "fill", ...src, minzoom: 12, filter: ["all", poly, cls("taxiway")], paint: { "fill-color": "#34373b" } });
  add({ id: "fd-runway-area", type: "fill", ...src, minzoom: 11, filter: ["all", poly, cls("runway")], paint: { "fill-color": "#55595f" } });
  add({
    id: "fd-taxiway",
    type: "line",
    ...src,
    minzoom: 12,
    filter: ["all", line, cls("taxiway")],
    layout: { "line-cap": "round", "line-join": "round" },
    paint: {
      "line-color": "#d4b24a",
      "line-opacity": ["interpolate", ["linear"], ["zoom"], 12, 0.5, 14, 0.9],
      "line-width": ["interpolate", ["exponential", 1.6], ["zoom"], 12, 0.7, 15, 2, 18, 6],
    },
  });
  add({
    id: "fd-runway",
    type: "line",
    ...src,
    minzoom: 10,
    filter: ["all", line, cls("runway")],
    layout: { "line-cap": "butt", "line-join": "round" },
    paint: { "line-color": "#5d6168", "line-width": ["interpolate", ["exponential", 1.5], ["zoom"], 10, 2, 13, 9, 16, 46, 18, 150] },
  });
  add({
    id: "fd-runway-centre",
    type: "line",
    ...src,
    minzoom: 13,
    filter: ["all", line, cls("runway")],
    paint: { "line-color": "#e8eaed", "line-width": ["interpolate", ["linear"], ["zoom"], 13, 0.6, 16, 1.4, 18, 3], "line-dasharray": [6, 5] },
  });
  if (!glyphs) return;
  const font = ["Noto Sans Bold"];
  add({
    id: "fd-taxiway-label",
    type: "symbol",
    ...src,
    minzoom: 14,
    filter: ["all", line, cls("taxiway"), ["has", "ref"]],
    layout: {
      "symbol-placement": "line",
      "symbol-spacing": 220,
      "text-field": ["get", "ref"],
      "text-font": font,
      "text-size": ["interpolate", ["linear"], ["zoom"], 14, 10, 17, 14],
      "text-rotation-alignment": "viewport",
      "text-keep-upright": true,
    },
    paint: { "text-color": "#1b1406", "text-halo-color": "#e2c15a", "text-halo-width": 3 },
  });
  add({
    id: "fd-runway-label",
    type: "symbol",
    ...src,
    minzoom: 11.5,
    maxzoom: 12.5,
    filter: ["all", line, cls("runway"), ["has", "ref"]],
    layout: {
      "symbol-placement": "line-center",
      "text-field": ["get", "ref"],
      "text-font": font,
      "text-size": ["interpolate", ["linear"], ["zoom"], 12, 11, 16, 16],
      "text-rotation-alignment": "viewport",
      "text-allow-overlap": false,
    },
    paint: { "text-color": "#ffffff", "text-halo-color": "#16171a", "text-halo-width": 2.4 },
  });
  // Approach lines at the planned destination (see approachLines): white,
  // dashed, always shown with a plan (not part of the switchable layout).
  if (!map.getSource("fd-approach")) map.addSource("fd-approach", { type: "geojson", data: { type: "FeatureCollection", features: [] } });
  map.addLayer(
    {
      id: "fd-approach",
      type: "line",
      source: "fd-approach",
      minzoom: 9,
      filter: ["==", ["get", "kind"], "line"],
      paint: {
        "line-color": "#ffffff",
        "line-opacity": 0.75,
        "line-width": ["interpolate", ["linear"], ["zoom"], 9, 1, 14, 1.8],
        "line-dasharray": [4, 3],
      },
    },
    before,
  );
  map.addLayer(
    {
      id: "fd-approach-label",
      type: "symbol",
      source: "fd-approach",
      minzoom: 9,
      filter: ["==", ["get", "kind"], "label"],
      layout: { "text-field": ["get", "ref"], "text-font": font, "text-size": 11.5, "text-allow-overlap": false },
      paint: { "text-color": "#ffffff", "text-halo-color": "#0e0f11", "text-halo-width": 2 },
    } as maplibregl.LayerSpecification,
    before,
  );
  // Runway numbers at each end, from the runways in view (see runwayEnds).
  if (!map.getSource("fd-runway-ends")) map.addSource("fd-runway-ends", { type: "geojson", data: { type: "FeatureCollection", features: [] } });
  add(
    {
      id: "fd-runway-end",
      type: "symbol",
      source: "fd-runway-ends",
      minzoom: 12.5,
      layout: {
        "text-field": ["get", "ref"],
        "text-font": font,
        "text-size": ["interpolate", ["linear"], ["zoom"], 12.5, 12, 16, 18],
        "text-allow-overlap": true,
        "text-ignore-placement": true,
      },
      paint: { "text-color": "#ffffff", "text-halo-color": "#0e0f11", "text-halo-width": 2.6 },
    } as maplibregl.LayerSpecification,
  );
  add({
    id: "fd-gate-label",
    type: "symbol",
    ...src,
    minzoom: 16,
    filter: ["all", cls("gate"), ["has", "ref"]],
    layout: { "text-field": ["get", "ref"], "text-font": ["Noto Sans Regular"], "text-size": 11 },
    paint: { "text-color": "#c9ccd1", "text-halo-color": "#16171a", "text-halo-width": 1.6 },
  });
}

/** Show or hide the airport layout (the base map's plain aeroways otherwise). */
export function showAirportLayout(map: maplibregl.Map, on: boolean) {
  for (const id of ["fd-apron", "fd-taxiway-area", "fd-runway-area", "fd-taxiway", "fd-runway", "fd-runway-centre", "fd-taxiway-label", "fd-runway-label", "fd-runway-end", "fd-gate-label"]) {
    if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", on ? "visible" : "none");
  }
  for (const id of BASE_AEROWAY) if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", on ? "none" : "visible");
}

type LL = [number, number];
const R = Math.PI / 180;
function bearing(a: LL, b: LL): number {
  const y = Math.sin((b[0] - a[0]) * R) * Math.cos(b[1] * R);
  const x = Math.cos(a[1] * R) * Math.sin(b[1] * R) - Math.sin(a[1] * R) * Math.cos(b[1] * R) * Math.cos((b[0] - a[0]) * R);
  return (Math.atan2(y, x) / R + 360) % 360;
}
function metres(a: LL, b: LL): number {
  const dx = (b[0] - a[0]) * Math.cos(((a[1] + b[1]) / 2) * R) * 111_320;
  const dy = (b[1] - a[1]) * 110_540;
  return Math.hypot(dx, dy);
}
/** A point beyond `b`, past the end of the line a→b by `m` metres. */
function beyond(a: LL, b: LL, m: number): LL {
  const d = metres(a, b) || 1;
  return [b[0] + ((b[0] - a[0]) * m) / d, b[1] + ((b[1] - a[1]) * m) / d];
}

/** A runway in view: its two ends, each with the number of the runway
 * that starts there. */
export interface RunwaySeen {
  a: LL;
  b: LL;
  /** The number at end a (landing from a towards b), and at end b. */
  atA: string;
  atB: string;
  length_m: number;
}

/**
 * The runways in view, from the base map's OpenStreetMap runway lines
 * (cut up by the map tiles): joined back by name and place, the two ends
 * found, and each end given the number of the runway that starts there
 * (the one whose heading points down the runway from it: 10 at the west
 * end of 10/28).
 */
export function runwaysInView(map: maplibregl.Map): RunwaySeen[] {
  const parts = map.querySourceFeatures("openmaptiles", {
    sourceLayer: "aeroway",
    filter: ["all", ["==", ["get", "class"], "runway"], ["has", "ref"]],
  });
  // Runways by name, then by place (many airports have an 18/36).
  const groups: { ref: string; mid: LL; ends: LL[] }[] = [];
  for (const f of parts) {
    const g = f.geometry;
    const lines: LL[][] = g.type === "LineString" ? [g.coordinates as LL[]] : g.type === "MultiLineString" ? (g.coordinates as LL[][]) : [];
    const ref = String(f.properties?.ref ?? "").trim();
    for (const l of lines) {
      if (l.length < 2 || !ref) continue;
      const a = l[0];
      const b = l[l.length - 1];
      const mid: LL = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
      const near = groups.find((x) => x.ref === ref && metres(x.mid, mid) < 6000);
      if (near) near.ends.push(a, b);
      else groups.push({ ref, mid, ends: [a, b] });
    }
  }
  const out: RunwaySeen[] = [];
  for (const g of groups) {
    // The two ends: the farthest-apart pair of the pieces' ends.
    let best: [LL, LL] | null = null;
    let far = 0;
    for (let i = 0; i < g.ends.length; i++)
      for (let j = i + 1; j < g.ends.length; j++) {
        const d = metres(g.ends[i], g.ends[j]);
        if (d > far) {
          far = d;
          best = [g.ends[i], g.ends[j]];
        }
      }
    if (!best || far < 150) continue;
    const names = g.ref.split("/").map((s) => s.trim()).filter(Boolean);
    if (names.length !== 2) continue;
    const [a, b] = best;
    const heading = (n: string) => (parseInt(n, 10) * 10) % 360;
    const off = (x: number, y: number) => Math.min(Math.abs(x - y), 360 - Math.abs(x - y));
    const ab = bearing(a, b);
    // Which name belongs at end a: the one pointing from a towards b.
    const atA = off(heading(names[0]), ab) <= off(heading(names[1]), ab) ? names[0] : names[1];
    const atB = atA === names[0] ? names[1] : names[0];
    out.push({ a, b, atA, atB, length_m: far });
  }
  return out;
}

/** Where each runway's two numbers go: just beyond each end. */
export function runwayEnds(map: maplibregl.Map): GeoJSON.FeatureCollection {
  const features: GeoJSON.Feature[] = [];
  for (const r of runwaysInView(map)) {
    const gap = Math.max(60, r.length_m * 0.035);
    features.push(
      { type: "Feature", properties: { ref: r.atA }, geometry: { type: "Point", coordinates: beyond(r.b, r.a, gap) } },
      { type: "Feature", properties: { ref: r.atB }, geometry: { type: "Point", coordinates: beyond(r.a, r.b, gap) } },
    );
  }
  return { type: "FeatureCollection", features };
}

/** How far out the approach lines reach from each threshold. */
const APPROACH_M = 10 * 1852;

/**
 * Approach lines at an airport: from each runway end, the runway's line
 * carried straight out for 10 nm the way an aircraft comes in to land on
 * it, labelled with that runway's number at the far end. Only runways
 * within 5 km of the airport.
 */
export function approachLines(map: maplibregl.Map, at: { lat: number; lon: number }): GeoJSON.FeatureCollection {
  const here: LL = [at.lon, at.lat];
  const features: GeoJSON.Feature[] = [];
  for (const r of runwaysInView(map)) {
    const mid: LL = [(r.a[0] + r.b[0]) / 2, (r.a[1] + r.b[1]) / 2];
    if (metres(mid, here) > 5000) continue;
    // Landing on the runway that starts at a, aircraft come from beyond a.
    for (const [from, to, name] of [
      [r.a, r.b, r.atA],
      [r.b, r.a, r.atB],
    ] as [LL, LL, string][]) {
      const far = beyond(to, from, APPROACH_M);
      features.push(
        { type: "Feature", properties: { kind: "line", ref: name }, geometry: { type: "LineString", coordinates: [from, far] } },
        { type: "Feature", properties: { kind: "label", ref: `RWY ${name}` }, geometry: { type: "Point", coordinates: far } },
      );
    }
  }
  return { type: "FeatureCollection", features };
}

// --- Roads ------------------------------------------------------------------
// From the same OpenFreeMap tiles as the base map (OpenStreetMap data,
// OpenMapTiles schema): roads coloured by importance, their names along
// them, and route shields (Interstate, US and state highways, E-roads,
// motorways) drawn here, so nothing else is downloaded.

/** Road colours, most important first. */
const ROAD = {
  motorway: "#f08c3c",
  trunk: "#f2a64b",
  primary: "#e8c55c",
  secondary: "#d6d9de",
  tertiary: "#a4abb3",
  minor: "#7d848c",
};
const ROAD_IDS = ["fd-road-casing", "fd-road", "fd-road-name", "fd-road-shield"];

type ShieldKind = "i" | "us" | "st" | "e" | "m" | "g";
/** The shield's ink and the colour of its number. */
const SHIELD_TEXT: Record<ShieldKind, string> = { i: "#ffffff", us: "#111111", st: "#111111", e: "#ffffff", m: "#ffffff", g: "#f1f3f5" };

/** A route shield, drawn at twice the size for sharp screens. */
function shield(kind: ShieldKind, w: number): ImageData {
  const s = 2;
  const W = w * s;
  const H = 26 * s;
  const c = document.createElement("canvas");
  c.width = W;
  c.height = H;
  const g = c.getContext("2d")!;
  g.lineJoin = "round";
  const round = (x: number, y: number, ww: number, hh: number, r: number) => {
    g.beginPath();
    g.roundRect(x, y, ww, hh, r);
  };
  if (kind === "i" || kind === "us") {
    // A shield: flat-ish top, sides curving to a point at the bottom.
    const path = () => {
      g.beginPath();
      g.moveTo(3 * s, 3 * s);
      g.quadraticCurveTo(W / 2, 0.5 * s, W - 3 * s, 3 * s);
      g.lineTo(W - 2 * s, H * 0.48);
      g.quadraticCurveTo(W - 3 * s, H * 0.86, W / 2, H - 1.5 * s);
      g.quadraticCurveTo(3 * s, H * 0.86, 2 * s, H * 0.48);
      g.closePath();
    };
    path();
    g.fillStyle = kind === "i" ? "#1d4fa0" : "#ffffff";
    g.fill();
    if (kind === "i") {
      // The red band across the top.
      g.save();
      path();
      g.clip();
      g.fillStyle = "#c8102e";
      g.fillRect(0, 0, W, H * 0.3);
      g.restore();
    }
    path();
    g.lineWidth = 1.6 * s;
    g.strokeStyle = kind === "i" ? "#ffffff" : "#111111";
    g.stroke();
  } else if (kind === "st") {
    g.beginPath();
    g.ellipse(W / 2, H / 2, W / 2 - 1.5 * s, H / 2 - 2 * s, 0, 0, Math.PI * 2);
    g.fillStyle = "#ffffff";
    g.fill();
    g.lineWidth = 1.6 * s;
    g.strokeStyle = "#111111";
    g.stroke();
  } else {
    round(1.5 * s, 4 * s, W - 3 * s, H - 8 * s, 3.5 * s);
    g.fillStyle = kind === "e" ? "#1b8a3a" : kind === "m" ? "#1d4fa0" : "#3a3f46";
    g.fill();
    g.lineWidth = 1.4 * s;
    g.strokeStyle = kind === "g" ? "#8a9098" : "#ffffff";
    g.stroke();
  }
  return g.getImageData(0, 0, W, H);
}

/** Roads, under the place names and over the land (and satellite). Hidden
 * until switched on with showRoads. */
export function addRoads(map: maplibregl.Map) {
  if (map.getLayer("fd-road") || !map.getSource("openmaptiles")) return;
  for (const k of Object.keys(SHIELD_TEXT) as ShieldKind[]) {
    for (const [n, w] of [[2, 28], [3, 34], [4, 42]] as const) {
      const id = `fd-sh-${k}-${n}`;
      if (!map.hasImage(id)) map.addImage(id, shield(k, k === "st" && n > 2 ? w + 4 : w), { pixelRatio: 2 });
    }
  }
  const layers = map.getStyle().layers ?? [];
  // Under the first base-map label, above everything drawn on the ground.
  const before = layers.find((l) => l.type === "symbol" && !l.id.startsWith("fd-"))?.id;
  const cls = ["get", "class"] as unknown as maplibregl.ExpressionSpecification;
  const color = ["match", cls, "motorway", ROAD.motorway, "trunk", ROAD.trunk, "primary", ROAD.primary, "secondary", ROAD.secondary, "tertiary", ROAD.tertiary, ROAD.minor] as unknown as maplibregl.ExpressionSpecification;
  const width = (k: number) =>
    [
      "interpolate", ["exponential", 1.5], ["zoom"],
      5, ["match", cls, "motorway", 1.1 * k, "trunk", 0.9 * k, "primary", 0.6 * k, 0.3 * k],
      10, ["match", cls, "motorway", 2.6 * k, "trunk", 2.2 * k, "primary", 1.8 * k, "secondary", 1.3 * k, "tertiary", 1 * k, 0.6 * k],
      14, ["match", cls, "motorway", 6 * k, "trunk", 5 * k, "primary", 4.5 * k, "secondary", 3.6 * k, "tertiary", 3 * k, 1.8 * k],
      18, ["match", cls, "motorway", 18 * k, "trunk", 16 * k, "primary", 14 * k, "secondary", 12 * k, "tertiary", 10 * k, 7 * k],
    ] as unknown as maplibregl.ExpressionSpecification;
  // Smaller roads appear as you zoom in.
  const filter = [
    "all",
    ["match", ["geometry-type"], ["LineString", "MultiLineString"], true, false],
    [
      "any",
      ["match", cls, ["motorway", "trunk"], true, false],
      ["all", ["==", cls, "primary"], [">=", ["zoom"], 7]],
      ["all", ["==", cls, "secondary"], [">=", ["zoom"], 9]],
      ["all", ["==", cls, "tertiary"], [">=", ["zoom"], 10]],
      ["all", ["==", cls, "minor"], [">=", ["zoom"], 12.5]],
      ["all", ["==", cls, "service"], [">=", ["zoom"], 14]],
    ],
  ] as unknown as maplibregl.FilterSpecification;
  map.addLayer(
    {
      id: "fd-road-casing",
      type: "line",
      source: "openmaptiles",
      "source-layer": "transportation",
      minzoom: 5,
      filter,
      layout: { visibility: "none", "line-join": "round", "line-cap": "round" },
      paint: { "line-color": "#15171a", "line-opacity": 0.75, "line-width": width(1.6) },
    },
    before,
  );
  map.addLayer(
    {
      id: "fd-road",
      type: "line",
      source: "openmaptiles",
      "source-layer": "transportation",
      minzoom: 5,
      filter,
      layout: { visibility: "none", "line-join": "round", "line-cap": "round" },
      paint: { "line-color": color, "line-width": width(1) },
    },
    before,
  );
  map.addLayer(
    {
      id: "fd-road-name",
      type: "symbol",
      source: "openmaptiles",
      "source-layer": "transportation_name",
      minzoom: 10,
      filter: [
        "all",
        ["match", ["geometry-type"], ["LineString", "MultiLineString"], true, false],
        ["match", cls, ["trunk", "primary", "secondary", "tertiary", "minor"], true, false],
        ["any", ["!=", cls, "minor"], [">=", ["zoom"], 13.5]],
      ] as unknown as maplibregl.FilterSpecification,
      layout: {
        visibility: "none",
        "symbol-placement": "line",
        "symbol-spacing": 320,
        "text-field": ["coalesce", ["get", "name_en"], ["get", "name"]],
        "text-font": ["Noto Sans Regular"],
        "text-size": ["interpolate", ["linear"], ["zoom"], 10, 10.5, 15, 13],
        "text-max-angle": 30,
      },
      paint: { "text-color": "#e6e8eb", "text-halo-color": "#15171a", "text-halo-width": 1.4 },
    },
    before,
  );
  const kind = [
    "match", ["get", "network"],
    "us-interstate", "i",
    "us-highway", "us",
    "us-state", "st",
    "e-road", "e",
    ["gb-motorway", "ie-motorway", "ca-transcanada"], "m",
    "g",
  ] as unknown as maplibregl.ExpressionSpecification;
  map.addLayer(
    {
      id: "fd-road-shield",
      type: "symbol",
      source: "openmaptiles",
      "source-layer": "transportation_name",
      minzoom: 6,
      filter: [
        "all",
        ["has", "ref"],
        ["<=", ["coalesce", ["get", "ref_length"], 9], 6],
        ["match", ["geometry-type"], ["LineString", "MultiLineString"], true, false],
        ["any", ["match", cls, ["motorway", "trunk"], true, false], [">=", ["zoom"], 8]],
      ] as unknown as maplibregl.FilterSpecification,
      layout: {
        visibility: "none",
        "symbol-placement": "line",
        "symbol-spacing": ["interpolate", ["linear"], ["zoom"], 6, 260, 12, 420],
        "icon-image": ["concat", "fd-sh-", kind, "-", ["to-string", ["min", 4, ["max", 2, ["coalesce", ["get", "ref_length"], 2]]]]],
        "icon-rotation-alignment": "viewport",
        "text-rotation-alignment": "viewport",
        "text-field": ["to-string", ["get", "ref"]],
        "text-font": ["Noto Sans Bold"],
        "text-size": 10.5,
        "text-offset": ["match", kind, ["i", "us"], ["literal", [0, 0.15]], ["literal", [0, 0]]],
      } as maplibregl.SymbolLayerSpecification["layout"],
      paint: {
        "text-color": ["match", kind, "i", SHIELD_TEXT.i, "us", SHIELD_TEXT.us, "st", SHIELD_TEXT.st, "e", SHIELD_TEXT.e, "m", SHIELD_TEXT.m, SHIELD_TEXT.g],
      },
    },
    before,
  );
}

/** Roads on or off. While on, the base map's own faint road names step aside. */
export function showRoads(map: maplibregl.Map, on: boolean) {
  addRoads(map);
  if (on) {
    // Back over anything added since (satellite pictures), under the labels.
    const before = (map.getStyle().layers ?? []).find((l) => l.type === "symbol" && !l.id.startsWith("fd-"))?.id;
    for (const id of ROAD_IDS) if (map.getLayer(id)) map.moveLayer(id, before);
  }
  for (const id of ROAD_IDS) if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", on ? "visible" : "none");
  for (const l of map.getStyle().layers ?? []) {
    if (l.type === "symbol" && !l.id.startsWith("fd-") && /highway-name|road_shield|highway-shield/.test(l.id)) {
      if (!/shield/.test(l.id)) map.setLayoutProperty(l.id, "visibility", on ? "none" : "visible");
    }
  }
}

/** How see-through the roads are, 0.15 to 1. */
export function setRoadsOpacity(map: maplibregl.Map, o: number) {
  if (!map.getLayer("fd-road")) return;
  map.setPaintProperty("fd-road", "line-opacity", o);
  map.setPaintProperty("fd-road-casing", "line-opacity", 0.75 * o);
  map.setPaintProperty("fd-road-name", "text-opacity", Math.min(1, o + 0.2));
  map.setPaintProperty("fd-road-shield", "icon-opacity", Math.min(1, o + 0.2));
  map.setPaintProperty("fd-road-shield", "text-opacity", Math.min(1, o + 0.2));
}

// --- Terrain awareness ---------------------------------------------------------
// Terrain coloured by its height against the aircraft's, by the G1000's
// TAWS rules:
//   in the air: red at or above the aircraft or within 100 ft below it,
//   yellow 100 to 1,000 ft below, green 1,000 to 2,000 ft below, black at
//   least 2,000 ft below;
//   on the ground: red only where terrain rises 400 ft or more above the
//   aircraft, black everywhere else.
// Only about 100 statute miles around the aircraft is drawn (and only from
// zoom 6 in), so just the elevation tiles there are fetched.

const TAWS_IDS = ["fd-taws"];
/** Half the drawn square, in degrees of latitude: 100 statute miles. */
const TAWS_RADIUS_DEG = 100 / 69.05;
/** The area is moved once the aircraft is this far (degrees) from its centre. */
const TAWS_MOVE_DEG = 25 / 69.05;
const tawsAt = new WeakMap<maplibregl.Map, [number, number]>();

/** The colours for an aircraft at `altFt` (feet above sea level). The map
 * draws colour ramps only from "interpolate", so each hard edge is two
 * stops a centimetre apart. */
function tawsRamp(altFt: number, onGround: boolean): maplibregl.ExpressionSpecification {
  const m = (ft: number) => ft * 0.3048;
  const black = "rgba(0, 0, 0, 0.82)";
  const green = "rgba(46, 160, 67, 0.88)";
  const yellow = "rgba(236, 196, 40, 0.92)";
  const red = "rgba(214, 38, 34, 0.94)";
  const bands: [number, string][] = onGround
    ? [[m(altFt + 400), red]]
    : [
        [m(altFt - 2000), green],
        [m(altFt - 1000), yellow],
        [m(altFt - 100), red],
      ];
  const out: unknown[] = ["interpolate", ["linear"], ["elevation"], Math.min(-500, bands[0][0] - 10), black];
  let prev = black;
  for (const [at, color] of bands) {
    out.push(at - 0.01, prev, at, color);
    prev = color;
  }
  out.push(Math.max(9000, bands[bands.length - 1][0] + 10), prev);
  return out as maplibregl.ExpressionSpecification;
}

/** Terrain awareness on or off, for an aircraft at altFt (null: no flight;
 * nothing is drawn) at `pos` [lon, lat]. Cheap to call again as the
 * altitude changes; the area follows the aircraft. */
export function showTaws(map: maplibregl.Map, on: boolean, altFt: number | null, onGround = false, pos: [number, number] | null = null) {
  const show = on && altFt != null && pos != null;
  const at = tawsAt.get(map);
  // Far enough from the drawn area's centre: draw it around the aircraft again.
  if (show && at && (Math.abs(pos![1] - at[1]) > TAWS_MOVE_DEG || Math.abs(pos![0] - at[0]) * Math.cos((pos![1] * Math.PI) / 180) > TAWS_MOVE_DEG)) {
    if (map.getLayer("fd-taws")) map.removeLayer("fd-taws");
    if (map.getSource("fd-taws-dem")) map.removeSource("fd-taws-dem");
    tawsAt.delete(map);
  }
  if (show && !map.getLayer("fd-taws")) {
    const [lon, lat] = pos!;
    const dLon = TAWS_RADIUS_DEG / Math.max(0.2, Math.cos((lat * Math.PI) / 180));
    map.addSource("fd-taws-dem", {
      type: "raster-dem",
      encoding: "terrarium",
      tiles: [demSource().sharedDemProtocolUrl],
      tileSize: 256,
      maxzoom: 11,
      bounds: [lon - dLon, Math.max(-85, lat - TAWS_RADIUS_DEG), lon + dLon, Math.min(85, lat + TAWS_RADIUS_DEG)],
      attribution: TERRAIN_ATTRIBUTION,
    });
    const before = map.getLayer("fd-road-casing") ? "fd-road-casing" : (map.getStyle().layers ?? []).find((l) => l.type === "symbol" && !l.id.startsWith("fd-"))?.id;
    map.addLayer(
      { id: "fd-taws", type: "color-relief", source: "fd-taws-dem", minzoom: 6, paint: { "color-relief-color": tawsRamp(altFt!, onGround), "color-relief-opacity": 1 } },
      before,
    );
    tawsAt.set(map, [lon, lat]);
  }
  for (const id of TAWS_IDS) if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", show ? "visible" : "none");
  if (show && map.getLayer("fd-taws")) map.setPaintProperty("fd-taws", "color-relief-color", tawsRamp(altFt!, onGround));
}

// --- Obstacles -------------------------------------------------------------------
// Towers, antennas, wind turbines, stacks (FAA Digital Obstacle File, US):
// a chart-style symbol, red when its top is within 100 ft of the
// aircraft's altitude or above it, yellow within 1,000 ft below, white
// otherwise; the top's height (and height above ground) as you zoom in.

function obstacleIcon(lit: boolean): ImageData {
  const s = 2;
  const W = 22 * s;
  const H = 26 * s;
  const c = document.createElement("canvas");
  c.width = W;
  c.height = H;
  const g = c.getContext("2d")!;
  g.strokeStyle = "#ffffff";
  g.fillStyle = "#ffffff";
  g.lineWidth = 2.6 * s;
  g.lineCap = "round";
  g.lineJoin = "round";
  // The chart symbol: an inverted V with a dot at its foot.
  g.beginPath();
  g.moveTo(5 * s, 24 * s);
  g.lineTo(11 * s, 7 * s);
  g.lineTo(17 * s, 24 * s);
  g.stroke();
  g.beginPath();
  g.arc(11 * s, 22 * s, 1.8 * s, 0, Math.PI * 2);
  g.fill();
  if (lit) {
    // Light rays from the top.
    g.lineWidth = 1.8 * s;
    for (const [dx, dy] of [[-6, -2], [6, -2], [-4, -5.5], [4, -5.5]]) {
      g.beginPath();
      g.moveTo(11 * s + dx * 0.45 * s, 6 * s + dy * 0.45 * s);
      g.lineTo(11 * s + dx * s, 6 * s + dy * s);
      g.stroke();
    }
  }
  return g.getImageData(0, 0, W, H);
}

const OBST_IDS = ["fd-obst", "fd-obst-label"];

/** Obstacles by the same rules as terrain, from the height of their tops. */
function obstColor(altFt: number | null, onGround = false): maplibregl.ExpressionSpecification {
  if (altFt == null) return "#ffffff" as unknown as maplibregl.ExpressionSpecification;
  if (onGround) return ["step", ["get", "amsl"], "#ffffff", altFt + 400, "#ff3b30"] as unknown as maplibregl.ExpressionSpecification;
  return ["step", ["get", "amsl"], "#ffffff", altFt - 1000, "#ecc428", altFt - 100, "#ff3b30"] as unknown as maplibregl.ExpressionSpecification;
}

export function addObstacles(map: maplibregl.Map) {
  if (map.getSource("fd-obst")) return;
  for (const lit of [false, true]) {
    const id = lit ? "fd-obst-lit" : "fd-obst-icon";
    if (!map.hasImage(id)) map.addImage(id, obstacleIcon(lit), { pixelRatio: 2, sdf: true });
  }
  map.addSource("fd-obst", { type: "geojson", data: { type: "FeatureCollection", features: [] }, maxzoom: 12, attribution: "Obstacles FAA" });
  const before = (map.getStyle().layers ?? []).find((l) => l.id.startsWith("routes") || l.id === "plan-line")?.id;
  map.addLayer(
    {
      id: "fd-obst",
      type: "symbol",
      source: "fd-obst",
      minzoom: 8,
      // Taller ones first, so they win where symbols would collide.
      layout: {
        visibility: "none",
        "symbol-sort-key": ["-", 0, ["get", "agl"]],
        "icon-image": ["case", ["get", "lit"], "fd-obst-lit", "fd-obst-icon"],
        "icon-size": ["interpolate", ["linear"], ["get", "agl"], 200, 0.62, 1000, 0.95, 2000, 1.15],
        "icon-anchor": "bottom",
        "icon-allow-overlap": ["step", ["zoom"], false, 11, true] as unknown as boolean,
      },
      paint: { "icon-color": "#ffffff", "icon-halo-color": "rgba(0,0,0,0.85)", "icon-halo-width": 1.6 },
    },
    before,
  );
  map.addLayer(
    {
      id: "fd-obst-label",
      type: "symbol",
      source: "fd-obst",
      minzoom: 10.5,
      layout: {
        visibility: "none",
        "symbol-sort-key": ["-", 0, ["get", "agl"]],
        "text-field": ["format", ["to-string", ["get", "amsl"]], { "font-scale": 1 }, "\n(", {}, ["to-string", ["get", "agl"]], {}, ")", {}],
        "text-font": ["Noto Sans Bold"],
        "text-size": 10.5,
        "text-anchor": "top",
        "text-offset": [0, 0.3],
        "text-line-height": 1.05,
      },
      paint: { "text-color": "#ffffff", "text-halo-color": "#111214", "text-halo-width": 1.4 },
    },
    before,
  );
}

/** Obstacles on or off, coloured against the aircraft's altitude (null: no flight). */
export function showObstacles(map: maplibregl.Map, on: boolean, altFt: number | null, onGround = false) {
  addObstacles(map);
  for (const id of OBST_IDS) if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", on ? "visible" : "none");
  if (on) {
    map.setPaintProperty("fd-obst", "icon-color", obstColor(altFt, onGround));
    map.setPaintProperty("fd-obst-label", "text-color", obstColor(altFt, onGround));
  }
}
