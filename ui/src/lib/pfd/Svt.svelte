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
  import { PFD_CY, PFD_H, PFD_W } from "./layout";

  let { d }: { d: Pfd | null } = $props();

  let el: HTMLDivElement;
  let map: maplibregl.Map | undefined;
  let ready = $state(false);
  const DEM = "https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png";
  const FT = 0.3048;
  /** The pilot's eye above the ground, metres. */
  const EYE_M = 3.5;
  /** Near and far clipping planes, metres from the eye. */
  const NEAR_M = 1;
  const FAR_M = 40_000;

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
          // No minzoom on these: the map's zoom here is set by how far away
          // its centre point is (10 km when looking level), not by how
          // close the ground is, so it would hide the runway under the nose.
          // Lakes, rivers and the sea, flat dark blue.
          { id: "water", type: "fill", source: "omt", "source-layer": "water", paint: { "fill-color": "#0e2c5e", "fill-antialias": false } },
          // Airports: aprons and runway areas in concrete grey.
          {
            id: "apron",
            type: "fill",
            source: "omt",
            "source-layer": "aeroway",
            filter: ["all", areas, ["match", cls, ["apron", "runway", "taxiway", "helipad"], true, false]] as unknown as maplibregl.FilterSpecification,
            paint: { "fill-color": ["match", cls, "runway", "#55585d", "#6c6f74"] },
          },
          {
            id: "taxiway",
            type: "line",
            source: "omt",
            "source-layer": "aeroway",
            filter: ["all", lines, ["==", cls, "taxiway"]] as unknown as maplibregl.FilterSpecification,
            layout: { "line-join": "round", "line-cap": "round" },
            paint: { "line-color": "#66696e", "line-width": metres(20, lat) },
          },
          {
            id: "runway",
            type: "line",
            source: "omt",
            "source-layer": "aeroway",
            filter: ["all", lines, ["==", cls, "runway"]] as unknown as maplibregl.FilterSpecification,
            layout: { "line-join": "round" },
            paint: { "line-color": "#4d5055", "line-width": metres(45, lat) },
          },
          {
            id: "taxiway-centre",
            type: "line",
            source: "omt",
            "source-layer": "aeroway",
            filter: ["all", lines, ["==", cls, "taxiway"]] as unknown as maplibregl.FilterSpecification,
            paint: { "line-color": "#e8c22a", "line-width": metres(0.8, lat) },
          },
          {
            id: "runway-centre",
            type: "line",
            source: "omt",
            "source-layer": "aeroway",
            filter: ["all", lines, ["==", cls, "runway"]] as unknown as maplibregl.FilterSpecification,
            // Dashes about 30 m long with 20 m gaps, as painted.
            paint: { "line-color": "#f2f2f2", "line-width": metres(1, lat), "line-dasharray": [30, 20] },
          },
        ],
      },
    });
    // Not "load": that waits for the first tiles, which can be slow.
    map.on("style.load", () => {
      // MapLibre hazes the ground into the horizon when the camera is
      // pitched past 60°. Its haze is worked out per corner of each ground
      // tile, and for tiles reaching behind a camera sitting a few metres
      // up, those numbers run away and the tile paints solid white. Leave
      // the ground unhazed (the G1000's synthetic vision has none either).
      const sky = (map!.style as unknown as { sky?: { calculateFogBlendOpacity(p: number): number } }).sky;
      if (sky) sky.calculateFogBlendOpacity = () => 0;
      // MapLibre loads coarser tiles the closer the view gets to level,
      // to save tiles on tilted maps. Looking straight along the ground,
      // that blurred the runway ahead. Pick each tile's detail from its
      // distance to the eye alone: about one tile pixel per screen pixel.
      for (const id of ["dem", "shade", "omt"]) {
        const src = map!.getSource(id) as unknown as { calculateTileZoom?: unknown } | undefined;
        if (src) src.calculateTileZoom = byDistance;
      }
      ready = true;
    });
    if (import.meta.env.DEV) map.on("style.load", () => ((window as unknown as { __svtMap: maplibregl.Map }).__svtMap = map!));
  });
  onDestroy(() => map?.remove());

  /** A tile's zoom from its distance alone (MapLibre's CalculateTileZoomFunction). */
  function byDistance(centerZoom: number, toTile2d: number, toTileZ: number, toCenter3d: number) {
    return Math.min(18, centerZoom + Math.log2(toCenter3d / Math.max(Math.hypot(toTile2d, toTileZ), 1e-9)));
  }

  // The PFD draws pitch at 500/36.87 units per degree, scaled to fit its
  // frame into the window from the top. Match the camera to it: the same
  // degrees per pixel (field of view over the whole window), and the
  // vanishing point moved up from the window's middle to the PFD's
  // attitude centre (padding at the bottom shifts MapLibre's centre).
  let w = $state(0);
  let h = $state(0);
  const view = $derived.by(() => {
    if (!w || !h) return null;
    const scale = Math.min(w / PFD_W, h / PFD_H);
    const f = (250 * scale) / Math.tan(((36.87 / 2) * Math.PI) / 180);
    const cy = PFD_CY * scale;
    return {
      fov: (2 * Math.atan(h / 2 / f) * 180) / Math.PI,
      padding: { top: Math.max(0, 2 * cy - h), bottom: Math.max(0, h - 2 * cy), left: 0, right: 0 },
    };
  });
  $effect(() => {
    if (map && view) map.setVerticalFieldOfView(view.fov);
  });

  // Put the camera where the pilot sits, looking where the aircraft points.
  $effect(() => {
    if (!map || !ready || !d || !view) return;
    // The elevation tiles and the sim's own scenery can differ by tens of
    // feet, which on the ground (or in the flare) can put the camera under
    // the drawn runway. Keep it at least an eye height above the drawn
    // ground; on the ground, always exactly there, so the runway runs ahead.
    const ground = map.queryTerrainElevation([d.lon, d.lat]);
    let alt = d.alt_msl_ft * FT;
    if (ground != null) alt = d.on_ground ? ground + EYE_M : Math.max(alt, ground + EYE_M);
    const pitch = 90 + d.pitch_deg;
    const tr = (map as unknown as { transform: Tr }).transform;
    // MapLibre places its camera by a centre point it looks at. Its own
    // helper puts that point 10 km ahead whenever the view is near level,
    // and the map's zoom follows from it (about 11). Everything that scales
    // with the zoom then suits a view of the ground 10 km away: the near
    // clipping plane sat a couple of hundred metres ahead (the "cliff":
    // the runway under the nose was cut off and tile edges showed as
    // streaks) and lines were drawn with a soft edge tens of metres wide
    // (a smeared runway). Look at a point 60 m ahead instead, so the zoom
    // suits the ground near the aircraft, and set the near and far planes
    // ourselves: 1 m and 40 km (about 22 nm; farther only loads more tiles).
    const D = 60;
    const camM = maplibregl.MercatorCoordinate.fromLngLat([d.lon, d.lat], Math.max(alt, 2));
    const mPerUnit = 1 / camM.meterInMercatorCoordinateUnits();
    const dM = D / mPerUnit;
    const p = (pitch * Math.PI) / 180;
    const b = (d.hdg_true_deg * Math.PI) / 180;
    const centre = new maplibregl.MercatorCoordinate(camM.x + Math.sin(p) * Math.sin(b) * dM, camM.y - Math.sin(p) * Math.cos(b) * dM);
    const zoom = Math.log2(h / 2 / Math.tan((view.fov * Math.PI) / 360) / dM / 512);
    tr.clearNearFarZOverride();
    map.jumpTo({
      center: centre.toLngLat(),
      elevation: Math.max(alt, 2) - D * Math.cos(p),
      zoom,
      pitch,
      bearing: d.hdg_true_deg,
      roll: d.bank_deg,
      padding: view.padding,
    });
    tr.overrideNearFarZ(tr.pixelsPerMeter * NEAR_M, tr.pixelsPerMeter * FAR_M);
    map.triggerRepaint();
  });
  type Tr = { clearNearFarZOverride(): void; overrideNearFarZ(n: number, f: number): void; pixelsPerMeter: number; farZ: number };
</script>

<div class="svt" bind:this={el} bind:clientWidth={w} bind:clientHeight={h}></div>

<style>
  .svt {
    position: absolute;
    inset: 0;
    background: linear-gradient(#0d47e0, #8db4ff 50%, #3d6b2a 50%);
  }
</style>
