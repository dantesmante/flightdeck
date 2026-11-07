<script lang="ts">
  // Synthetic vision behind the PFD on phones and tablets: the terrain ahead
  // as seen from the aircraft, from the same free elevation tiles as the
  // Topographic layer (Terrain Tiles, AWS open data), coloured by height
  // (green low, browns higher, white peaks) with relief shading. The device
  // draws it, so the simulator PC does no extra work.
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

  onMount(() => {
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
        ],
      },
    });
    // Not "load": that waits for the first tiles, which can be slow.
    map.on("style.load", () => (ready = true));
    if (import.meta.env.DEV) (window as unknown as { __svtMap: maplibregl.Map }).__svtMap = map;
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
    const cam = map.calculateCameraOptionsFromCameraLngLatAltRotation(
      [d.lon, d.lat],
      Math.max(d.alt_msl_ft * FT, 10),
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
