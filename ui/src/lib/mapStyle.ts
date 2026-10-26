// The night-chart look shared by the main map and the logbook's mini map:
// OpenFreeMap vector tiles recoloured at load, and a plain grid offline.
import type maplibregl from "maplibre-gl";

export const STYLE = "https://tiles.openfreemap.org/styles/positron";

export const C = {
  land: "#132029",
  water: "#0d1820",
  green: "#16252f",
  built: "#1a2a36",
  road: "#22384a",
  roadMajor: "#2b4558",
  aeroway: "#4b6276",
  border: "#3e5d73",
  text: "#8fa3b3",
  halo: "#0d1820",
  track: "#e2468f",
};

function setPaint(map: maplibregl.Map, id: string, prop: string, value: unknown) {
    try {
      map.setPaintProperty(id, prop, value);
    } catch {
      /* property not valid for this layer */
    }
  }

/** Recolour every layer of the base style by what it draws. */
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
