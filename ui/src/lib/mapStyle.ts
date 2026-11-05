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

/**
 * Where each runway's two numbers go: its runway lines in view (cut up by
 * the map tiles) are joined back by name and place, the two ends found,
 * and each end given the number of the runway that starts there (the one
 * whose heading points down the runway from it: 10 at the west end of
 * 10/28).
 */
export function runwayEnds(map: maplibregl.Map): GeoJSON.FeatureCollection {
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
  const features: GeoJSON.Feature[] = [];
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
    const gap = Math.max(60, far * 0.035);
    features.push(
      { type: "Feature", properties: { ref: atA }, geometry: { type: "Point", coordinates: beyond(b, a, gap) } },
      { type: "Feature", properties: { ref: atB }, geometry: { type: "Point", coordinates: beyond(a, b, gap) } },
    );
  }
  return { type: "FeatureCollection", features };
}
