<script lang="ts">
  // A small, still map of one flight: the flown track (or a straight line
  // between the airports when no track was saved) with rings at each end.
  import maplibregl from "maplibre-gl";
  import { onDestroy, onMount } from "svelte";
  import { C, STYLE, offlineStyle, tintMap } from "./mapStyle";
  import { altGradient, lonLat, type Pt } from "./track";

  interface Props {
    track: Pt[];
    height?: number;
    /** Allow zooming and panning, with zoom buttons. */
    interactive?: boolean;
  }
  let { track, height = 220, interactive = false }: Props = $props();

  let el: HTMLDivElement;
  let map: maplibregl.Map | undefined;
  let ready = false;

  function draw() {
    if (!map || !ready) return;
    const pts = track;
    const coords = lonLat(pts);
    const g = altGradient(pts);
    const line: GeoJSON.Feature = { type: "Feature", properties: {}, geometry: { type: "LineString", coordinates: coords } };
    const ends: GeoJSON.FeatureCollection = {
      type: "FeatureCollection",
      features: [coords[0], coords[coords.length - 1]]
        .filter(Boolean)
        .map((c) => ({ type: "Feature", properties: {}, geometry: { type: "Point", coordinates: c } })),
    };
    if (map.getSource("t")) {
      (map.getSource("t") as maplibregl.GeoJSONSource).setData(line);
      (map.getSource("e") as maplibregl.GeoJSONSource).setData(ends);
      map.setPaintProperty("glow", "line-gradient", g);
      map.setPaintProperty("line", "line-gradient", g);
    } else {
      map.addSource("t", { type: "geojson", data: line, lineMetrics: true });
      map.addSource("e", { type: "geojson", data: ends });
      map.addLayer({ id: "glow", type: "line", source: "t", paint: { "line-gradient": g, "line-width": 8, "line-opacity": 0.25, "line-blur": 3 } });
      map.addLayer({ id: "line", type: "line", source: "t", paint: { "line-gradient": g, "line-width": 2.8 }, layout: { "line-cap": "round", "line-join": "round" } });
      map.addLayer({
        id: "ends",
        type: "circle",
        source: "e",
        paint: { "circle-radius": 5, "circle-color": C.land, "circle-stroke-color": "#f0b23a", "circle-stroke-width": 2.5 },
      });
    }
    if (coords.length >= 2) {
      const b = new maplibregl.LngLatBounds(coords[0], coords[0]);
      for (const p of coords) b.extend(p);
      map.fitBounds(b, { padding: 28, maxZoom: 11.5, duration: 0 });
    } else if (coords.length === 1) {
      map.jumpTo({ center: coords[0], zoom: 10 });
    }
  }

  onMount(() => {
    map = new maplibregl.Map({
      container: el,
      style: STYLE,
      interactive,
      dragRotate: false,
      pitchWithRotate: false,
      touchPitch: false,
      attributionControl: { compact: true },
      fadeDuration: 0,
      center: track[0] ? [track[0][0], track[0][1]] : [0, 20],
      zoom: 3,
    });
    if (interactive) {
      map.addControl(new maplibregl.NavigationControl({ showCompass: false }), "bottom-right");
      map.touchZoomRotate.disableRotation();
    }
    let fellBack = false;
    const fallback = () => {
      if (fellBack || ready || !map) return;
      fellBack = true;
      map.setStyle(offlineStyle());
    };
    const timer = setTimeout(fallback, 6000);
    map.on("error", () => !ready && fallback());
    map.on("style.load", () => {
      clearTimeout(timer);
      if (!fellBack) tintMap(map!);
      ready = true;
      draw();
    });
  });

  $effect(() => {
    void track;
    draw();
  });

  onDestroy(() => map?.remove());
</script>

<div class="wrap">
  <div class="mini" class:interactive style:height="{height}px" bind:this={el} role="img" aria-label="Map of the flight's route"></div>
  {#if interactive}
    <button class="reset" onclick={() => draw()} title="Show the whole flight">Whole flight</button>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
  }
  .reset {
    position: absolute;
    top: 8px;
    right: 8px;
    padding: 4px 9px;
    border-radius: 6px;
    border: 1px solid var(--rule);
    background: rgba(28, 30, 33, 0.88);
    color: var(--ink-dim);
    font-size: 12px;
    font-weight: 600;
  }
  .reset:hover {
    color: var(--ink);
  }
  .mini.interactive {
    cursor: grab;
  }
  .mini :global(.maplibregl-ctrl-group) {
    background: var(--panel-2);
    border: 1px solid var(--rule);
    box-shadow: none;
  }
  .mini :global(.maplibregl-ctrl-group button + button) {
    border-top-color: var(--rule);
  }
  .mini :global(.maplibregl-ctrl-icon) {
    filter: invert(0.85);
  }
  .mini {
    width: 100%;
    border-radius: 8px;
    overflow: hidden;
    background: #17191b;
    border: 1px solid var(--rule);
  }
  .mini :global(.maplibregl-ctrl-attrib) {
    background: rgba(28, 30, 33, 0.85);
    color: var(--ink-dim);
    font-size: 10px;
  }
  .mini :global(.maplibregl-ctrl-attrib a) {
    color: var(--ink-dim);
  }
</style>
