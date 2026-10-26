<script lang="ts">
  // A small, still map of one flight: the flown track (or a straight line
  // between the airports when no track was saved) with rings at each end.
  import maplibregl from "maplibre-gl";
  import { onDestroy, onMount } from "svelte";
  import { C, STYLE, offlineStyle, tintMap } from "./mapStyle";

  interface Props {
    track: [number, number][];
    height?: number;
  }
  let { track, height = 220 }: Props = $props();

  let el: HTMLDivElement;
  let map: maplibregl.Map | undefined;
  let ready = false;

  function draw() {
    if (!map || !ready) return;
    const line: GeoJSON.Feature = { type: "Feature", properties: {}, geometry: { type: "LineString", coordinates: track } };
    const ends: GeoJSON.FeatureCollection = {
      type: "FeatureCollection",
      features: [track[0], track[track.length - 1]]
        .filter(Boolean)
        .map((c) => ({ type: "Feature", properties: {}, geometry: { type: "Point", coordinates: c } })),
    };
    if (map.getSource("t")) {
      (map.getSource("t") as maplibregl.GeoJSONSource).setData(line);
      (map.getSource("e") as maplibregl.GeoJSONSource).setData(ends);
    } else {
      map.addSource("t", { type: "geojson", data: line });
      map.addSource("e", { type: "geojson", data: ends });
      map.addLayer({ id: "glow", type: "line", source: "t", paint: { "line-color": C.track, "line-width": 8, "line-opacity": 0.2, "line-blur": 3 } });
      map.addLayer({ id: "line", type: "line", source: "t", paint: { "line-color": C.track, "line-width": 2.5 }, layout: { "line-cap": "round", "line-join": "round" } });
      map.addLayer({
        id: "ends",
        type: "circle",
        source: "e",
        paint: { "circle-radius": 5, "circle-color": C.land, "circle-stroke-color": C.track, "circle-stroke-width": 2.5 },
      });
    }
    if (track.length >= 2) {
      const b = new maplibregl.LngLatBounds(track[0], track[0]);
      for (const p of track) b.extend(p);
      map.fitBounds(b, { padding: 28, maxZoom: 11.5, duration: 0 });
    } else if (track.length === 1) {
      map.jumpTo({ center: track[0], zoom: 10 });
    }
  }

  onMount(() => {
    map = new maplibregl.Map({
      container: el,
      style: STYLE,
      interactive: false,
      attributionControl: { compact: true },
      fadeDuration: 0,
      center: track[0] ?? [0, 20],
      zoom: 3,
    });
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

<div class="mini" style:height="{height}px" bind:this={el} role="img" aria-label="Map of the flight's route"></div>

<style>
  .mini {
    width: 100%;
    border-radius: 8px;
    overflow: hidden;
    background: #132029;
    border: 1px solid var(--rule);
  }
  .mini :global(.maplibregl-ctrl-attrib) {
    background: rgba(23, 36, 48, 0.85);
    color: var(--ink-dim);
    font-size: 10px;
  }
  .mini :global(.maplibregl-ctrl-attrib a) {
    color: var(--ink-dim);
  }
</style>
