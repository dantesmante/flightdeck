<script lang="ts">
  // Synthetic vision behind the PFD on phones and tablets: the terrain ahead
  // as seen from the aircraft, from the same free elevation tiles as the
  // Topographic layer (Terrain Tiles, AWS open data), coloured by height
  // (green low, browns higher, white peaks) with relief shading; water in
  // dark blue and airports in grey concrete with runway and taxiway
  // centrelines, draped from the OpenFreeMap tiles (OpenStreetMap). The
  // device draws it, so the simulator PC does no extra work.
  import maplibregl from "maplibre-gl";
  import "maplibre-gl/dist/maplibre-gl.css";
  import { onDestroy, onMount } from "svelte";
  import type { Pfd } from "../api";

  let { d }: { d: Pfd | null } = $props();

  let el: HTMLDivElement;
  let map: maplibregl.Map | undefined;
  let ready = $state(false);
  const DEM = "https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png";
  const FT = 0.3048;
  /** The pilot's eye above the ground, metres. */
  const EYE_M = 3.5;

  /** A width in metres on the ground, as line widths at each zoom level. */
  function metres(m: number, lat: number): maplibregl.ExpressionSpecification {
    const mpp = (z: number) => (40075016.686 * Math.cos((lat * Math.PI) / 180)) / (512 * 2 ** z);
    return ["interpolate", ["exponential", 2], ["zoom"], 8, m / mpp(8), 20, m / mpp(20)] as unknown as maplibregl.ExpressionSpecification;
  }

  onMount(() => {
    const lat = d?.lat ?? 40;
    const cls = ["get", "class"] as unknown as maplibregl.ExpressionSpecification;
    const lines = ["match", ["geometry-type"], ["LineString", "MultiLineString"], true, false] as unknown as maplibregl.ExpressionSpecification;
    const areas = ["match", ["geometry-type"], ["Polygon", "MultiPolygon"], true, false] as unknown as maplibregl.ExpressionSpecification;
    map = new maplibregl.Map({
      container: el,
      interactive: false,
      attributionControl: false,
      maxPitch: 180,
      // The camera stays where the aircraft is. Left on, the map keeps its
      // view point on the ground and drags the camera with it whenever the
      // nose comes up past the horizon (its own docs: needed above 90°).
      centerClampedToGround: false,
      center: [d?.lon ?? 0, d?.lat ?? 0],
      zoom: 11,
      style: {
        version: 8,
        sources: {
          dem: { type: "raster-dem", tiles: [DEM], encoding: "terrarium", tileSize: 256, maxzoom: 12 },
          shade: { type: "raster-dem", tiles: [DEM], encoding: "terrarium", tileSize: 256, maxzoom: 12 },
          omt: { type: "vector", url: "https://tiles.openfreemap.org/planet" },
        },
        sky: {
          "sky-color": "#0d47e0",
          "horizon-color": "#8db4ff",
          "fog-color": "#b8cdf0",
          "sky-horizon-blend": 0.6,
          "horizon-fog-blend": 0.4,
          "fog-ground-blend": 0.75,
          "atmosphere-blend": 0,
        },
        terrain: { source: "dem", exaggeration: 1 },
        layers: [
          {
            id: "relief",
            type: "color-relief",
            source: "shade",
            paint: {
              "color-relief-color": [
                "interpolate",
                ["linear"],
                ["elevation"],
                -10, "#2d5f8a",
                0, "#3d6b2a",
                600, "#5d8a34",
                1200, "#8a8a3a",
                1800, "#8a6a3a",
                2600, "#7a5a40",
                3400, "#d8d8d8",
              ],
            },
          },
          {
            id: "hills",
            type: "hillshade",
            source: "shade",
            paint: { "hillshade-shadow-color": "#1a2a10", "hillshade-highlight-color": "#e8f0d0", "hillshade-exaggeration": 0.5 },
          },
          // Lakes, rivers and the sea, flat dark blue.
          { id: "water", type: "fill", source: "omt", "source-layer": "water", paint: { "fill-color": "#0e2c5e", "fill-antialias": false } },
          // Airports: aprons and runway areas in concrete grey.
          {
            id: "apron",
            type: "fill",
            source: "omt",
            "source-layer": "aeroway",
            minzoom: 10,
            filter: ["all", areas, ["match", cls, ["apron", "runway", "taxiway", "helipad"], true, false]] as unknown as maplibregl.FilterSpecification,
            paint: { "fill-color": ["match", cls, "runway", "#55585d", "#6c6f74"] },
          },
          {
            id: "taxiway",
            type: "line",
            source: "omt",
            "source-layer": "aeroway",
            minzoom: 11,
            filter: ["all", lines, ["==", cls, "taxiway"]] as unknown as maplibregl.FilterSpecification,
            layout: { "line-join": "round", "line-cap": "round" },
            paint: { "line-color": "#66696e", "line-width": metres(20, lat) },
          },
          {
            id: "runway",
            type: "line",
            source: "omt",
            "source-layer": "aeroway",
            minzoom: 9,
            filter: ["all", lines, ["==", cls, "runway"]] as unknown as maplibregl.FilterSpecification,
            layout: { "line-join": "round" },
            paint: { "line-color": "#4d5055", "line-width": metres(45, lat) },
          },
          {
            id: "taxiway-centre",
            type: "line",
            source: "omt",
            "source-layer": "aeroway",
            minzoom: 13,
            filter: ["all", lines, ["==", cls, "taxiway"]] as unknown as maplibregl.FilterSpecification,
            paint: { "line-color": "#e8c22a", "line-width": metres(0.8, lat) },
          },
          {
            id: "runway-centre",
            type: "line",
            source: "omt",
            "source-layer": "aeroway",
            minzoom: 12,
            filter: ["all", lines, ["==", cls, "runway"]] as unknown as maplibregl.FilterSpecification,
            // Dashes about 30 m long with 20 m gaps, as painted.
            paint: { "line-color": "#f2f2f2", "line-width": metres(1, lat), "line-dasharray": [30, 20] },
          },
        ],
      },
    });
    // Not "load": that waits for the first tiles, which can be slow.
    map.on("style.load", () => (ready = true));
    if (import.meta.env.DEV) map.on("style.load", () => ((window as unknown as { __svtMap: maplibregl.Map }).__svtMap = map!));
  });
  onDestroy(() => map?.remove());

  // The PFD draws pitch at 500/36.87 units per degree, scaled to fit its
  // 800 × 500 frame into the window. A window of another shape (split
  // screen, a phone) shows more above and below: widen the camera's view to
  // match, so the terrain's horizon stays on the PFD's.
  let w = $state(0);
  let h = $state(0);
  $effect(() => {
    if (!map || !w || !h) return;
    const scale = Math.min(w / 800, h / 500);
    const f = (250 * scale) / Math.tan(((36.87 / 2) * Math.PI) / 180);
    map.setVerticalFieldOfView((2 * Math.atan(h / 2 / f) * 180) / Math.PI);
  });

  // Put the camera where the pilot sits, looking where the aircraft points.
  $effect(() => {
    if (!map || !ready || !d) return;
    // The elevation tiles and the sim's own scenery can differ by tens of
    // feet, which on the ground (or in the flare) can put the camera under
    // the drawn runway. Keep it at least an eye height above the drawn
    // ground; on the ground, always exactly there, so the runway runs ahead.
    const ground = map.queryTerrainElevation([d.lon, d.lat]);
    let alt = d.alt_msl_ft * FT;
    if (ground != null) alt = d.on_ground ? ground + EYE_M : Math.max(alt, ground + EYE_M);
    const cam = map.calculateCameraOptionsFromCameraLngLatAltRotation(
      [d.lon, d.lat],
      Math.max(alt, 2),
      d.hdg_true_deg,
      90 + d.pitch_deg,
      d.bank_deg,
    );
    map.jumpTo(cam);
  });
</script>

<div class="svt" bind:this={el} bind:clientWidth={w} bind:clientHeight={h}></div>

<style>
  .svt {
    position: absolute;
    inset: 0;
    background: linear-gradient(#0d47e0, #8db4ff 50%, #3d6b2a 50%);
  }
</style>
