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
  border: "#34373b",
  borderStrong: "#7a8089",
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

/** Country and state borders: faint by default, or picked out clearly. */
const borderWidths = new WeakMap<maplibregl.Map, Map<string, unknown>>();
export function setBorders(map: maplibregl.Map, strong: boolean) {
  let widths = borderWidths.get(map);
  if (!widths) borderWidths.set(map, (widths = new Map()));
  for (const layer of map.getStyle().layers ?? []) {
    if (layer.type !== "line") continue;
    const src = "source-layer" in layer ? String(layer["source-layer"] ?? "") : "";
    const key = `${layer.id} ${src}`.toLowerCase();
    if (!key.match(/boundary|admin/)) continue;
    setPaint(map, layer.id, "line-color", strong ? C.borderStrong : C.border);
    setPaint(map, layer.id, "line-opacity", strong ? 0.9 : 1);
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
