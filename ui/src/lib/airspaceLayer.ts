// The Airspace layer: US Class B, C and D and special use airspace (FAA),
// and the rest of the world from openAIP. Two sources with the same layers:
// "us-airspace" (read once a cycle) and "world-airspace" (the countries in
// view). Lines as charts draw them, labels along the edge in a small box
// ("G: GND-2000AGL"), and in 3D each area stands up from its floor to its
// ceiling as a see-through block.

import type maplibregl from "maplibre-gl";

/** Kinds and how they are drawn: colour, and solid or dashed edge. */
export const ASP_KINDS: Record<string, { color: string; dash?: boolean; name: string }> = {
  A: { color: "#9fb0c4", name: "Class A" },
  B: { color: "#4a8ce8", name: "Class B" },
  C: { color: "#c25bb0", name: "Class C" },
  D: { color: "#4a8ce8", dash: true, name: "Class D" },
  E: { color: "#c25bb0", dash: true, name: "Class E" },
  F: { color: "#7fae6a", dash: true, name: "Class F" },
  G: { color: "#7fae6a", dash: true, name: "Class G" },
  CTR: { color: "#4a8ce8", dash: true, name: "Control zone" },
  P: { color: "#ff4f6d", name: "Prohibited" },
  R: { color: "#ff4f6d", name: "Restricted" },
  DNG: { color: "#ff8a3d", name: "Danger" },
  W: { color: "#ff4f6d", dash: true, name: "Warning" },
  AL: { color: "#f2b13d", dash: true, name: "Alert" },
  MOA: { color: "#b06ad9", dash: true, name: "MOA" },
  NSA: { color: "#ff4f6d", dash: true, name: "National security" },
  TRA: { color: "#ff8a3d", dash: true, name: "Temporary reserved" },
  TSA: { color: "#ff8a3d", dash: true, name: "Temporary segregated" },
  MIL: { color: "#ff8a3d", dash: true, name: "Military" },
};

const KINDS = Object.keys(ASP_KINDS);
const color = ["match", ["get", "c"], ...KINDS.flatMap((k) => [k, ASP_KINDS[k].color]), "#4a8ce8"] as unknown as maplibregl.ExpressionSpecification;
const DASHED = KINDS.filter((k) => ASP_KINDS[k].dash);
const isDashed = ["in", ["get", "c"], ["literal", DASHED]] as unknown as maplibregl.FilterSpecification;
const isSolid = ["!", ["in", ["get", "c"], ["literal", DASHED]]] as unknown as maplibregl.FilterSpecification;

export const SOURCES = ["us-airspace", "world-airspace"] as const;
const ids = (src: string) => ({ fill: `${src}-fill`, line: `${src}-line`, dash: `${src}-dash`, label: `${src}-label`, block: `${src}-3d` });

/** A small label box, stretched around the text: dark, with a coloured edge. */
function box(c: string): { img: ImageData; opts: { pixelRatio: number; stretchX: [number, number][]; stretchY: [number, number][]; content: [number, number, number, number] } } {
  const W = 28;
  const H = 24;
  const cv = document.createElement("canvas");
  cv.width = W;
  cv.height = H;
  const g = cv.getContext("2d")!;
  const r = 5;
  g.beginPath();
  g.moveTo(r + 1, 1);
  g.arcTo(W - 1, 1, W - 1, H - 1, r);
  g.arcTo(W - 1, H - 1, 1, H - 1, r);
  g.arcTo(1, H - 1, 1, 1, r);
  g.arcTo(1, 1, W - 1, 1, r);
  g.closePath();
  g.fillStyle = "rgba(14,15,18,0.94)";
  g.fill();
  g.lineWidth = 2.4;
  g.strokeStyle = c;
  g.stroke();
  return {
    img: g.getImageData(0, 0, W, H),
    opts: { pixelRatio: 2, stretchX: [[8, 20]], stretchY: [[8, 16]], content: [6, 5, 22, 19] },
  };
}

export function addAirspaceLayers(map: maplibregl.Map, three: boolean) {
  for (const k of KINDS) {
    const id = `asp-box-${k}`;
    if (!map.hasImage(id)) {
      const b = box(ASP_KINDS[k].color);
      map.addImage(id, b.img, b.opts);
    }
  }
  const empty: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
  for (const src of SOURCES) {
    if (map.getSource(src)) continue;
    map.addSource(src, { type: "geojson", data: empty, tolerance: 0.5 });
    const L = ids(src);
    map.addLayer({
      id: L.fill,
      type: "fill",
      source: src,
      minzoom: 5,
      layout: { visibility: "none" },
      paint: { "fill-color": color, "fill-opacity": ["match", ["get", "c"], ["P", "R"], 0.07, ["D", "E", "G", "F", "CTR"], 0.025, 0.045] as unknown as maplibregl.ExpressionSpecification },
    });
    map.addLayer({
      id: L.block,
      type: "fill-extrusion",
      source: src,
      minzoom: 5,
      layout: { visibility: "none" },
      paint: {
        "fill-extrusion-color": color,
        "fill-extrusion-base": ["*", ["min", ["get", "lo"], 45000], 0.3048] as unknown as maplibregl.ExpressionSpecification,
        "fill-extrusion-height": ["*", ["min", ["get", "hi"], 45000], 0.3048] as unknown as maplibregl.ExpressionSpecification,
        "fill-extrusion-opacity": 0.16,
      },
    });
    map.addLayer({
      id: L.line,
      type: "line",
      source: src,
      minzoom: 5,
      filter: isSolid,
      layout: { visibility: "none", "line-join": "round" },
      paint: { "line-color": color, "line-width": ["interpolate", ["linear"], ["zoom"], 5, 1, 10, ["match", ["get", "c"], ["B", "P", "R"], 2.4, 1.8]] as unknown as maplibregl.ExpressionSpecification, "line-opacity": 0.9 },
    });
    map.addLayer({
      id: L.dash,
      type: "line",
      source: src,
      minzoom: 6,
      filter: isDashed,
      layout: { visibility: "none" },
      paint: { "line-color": color, "line-width": 1.5, "line-dasharray": [3, 2], "line-opacity": 0.9 },
    });
    if (map.getStyle().glyphs)
      map.addLayer({
        id: L.label,
        type: "symbol",
        source: src,
        minzoom: 8,
        layout: {
          visibility: "none",
          "symbol-placement": "line",
          "symbol-spacing": 360,
          "text-field": ["get", "label"],
          "text-font": ["Noto Sans Bold"],
          "text-size": 11,
          "text-letter-spacing": 0.03,
          "text-keep-upright": true,
          "text-max-angle": 30,
          "text-rotation-alignment": "map",
          "icon-image": ["concat", "asp-box-", ["get", "c"]],
          "icon-text-fit": "both",
          "icon-text-fit-padding": [2, 5, 2, 5],
          "icon-rotation-alignment": "map",
        },
        paint: { "text-color": "#f2f4f7" },
      });
  }
  showAirspace(map, false, three);
}

/** Lines and labels when on; in 3D the blocks too, instead of the flat tint. */
export function showAirspace(map: maplibregl.Map, on: boolean, three: boolean) {
  for (const src of SOURCES) {
    const L = ids(src);
    const vis = (id: string, v: boolean) => map.getLayer(id) && map.setLayoutProperty(id, "visibility", v ? "visible" : "none");
    vis(L.fill, on && !three);
    vis(L.block, on && three);
    vis(L.line, on);
    vis(L.dash, on);
    vis(L.label, on);
  }
}

/** The 3D blocks at the same height scale as the flight's wall. */
export function scaleAirspace(map: maplibregl.Map, k: number) {
  for (const src of SOURCES) {
    const id = ids(src).block;
    if (!map.getLayer(id)) continue;
    map.setPaintProperty(id, "fill-extrusion-base", ["*", ["min", ["get", "lo"], 45000], 0.3048 * k]);
    map.setPaintProperty(id, "fill-extrusion-height", ["*", ["min", ["get", "hi"], 45000], 0.3048 * k]);
  }
}
