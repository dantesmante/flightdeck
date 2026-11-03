// The night-chart look shared by the main map and the logbook's mini map:
// OpenFreeMap vector tiles recolored at load, and a plain grid offline.
import type maplibregl from "maplibre-gl";

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
    const before = ["terrain-light", "terrain-dark", "fd-regions"].find((id) => map.getLayer(id))
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

/** Relief shading of mountains and valleys (Esri World Hillshade): the dark
 * version over the night chart, the light one over satellite pictures. It
 * sits above the land and pictures and below borders, labels and flights. */
export const TERRAIN_ATTRIBUTION = "Hillshade © Esri, USGS, NGA, NASA, CGIAR";
export function showTerrain(map: maplibregl.Map, on: boolean, satellite: boolean) {
  const kinds: [string, string, number][] = [
    ["terrain-dark", "World_Hillshade_Dark", 0.75],
    ["terrain-light", "World_Hillshade", 0.4],
  ];
  for (const [id, service, opacity] of kinds) {
    const want = on && (id === "terrain-light") === satellite;
    if (want && !map.getLayer(id)) {
      if (!map.getSource(id)) {
        map.addSource(id, {
          type: "raster",
          tiles: [`https://server.arcgisonline.com/ArcGIS/rest/services/Elevation/${service}/MapServer/tile/{z}/{y}/{x}`],
          tileSize: 256,
          maxzoom: 16,
          attribution: TERRAIN_ATTRIBUTION,
        });
      }
      const layers = map.getStyle().layers ?? [];
      const before = map.getLayer("fd-regions")
        ? "fd-regions"
        : layers.find((l) => l.id.startsWith("boundary") || l.type === "symbol")?.id;
      map.addLayer({ id, type: "raster", source: id, paint: { "raster-opacity": opacity, "raster-fade-duration": 0 } }, before);
    }
    if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", want ? "visible" : "none");
  }
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
