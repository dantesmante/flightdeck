<script lang="ts">
  // Live and historical flight map: MapLibre GL + OpenFreeMap vector tiles
  // (OpenStreetMap data), recoloured at load into a night-chart palette.
  import maplibregl, { type GeoJSONSource, type LngLatBoundsLike } from "maplibre-gl";
  import { onDestroy, onMount } from "svelte";

  interface Props {
    /** Live track, [lon, lat] pairs. */
    live: [number, number][];
    /** Current aircraft position and heading, if flying. */
    aircraft: { lon: number; lat: number; hdg: number } | null;
    /** Departure point of the live flight. */
    origin: [number, number] | null;
    /** A saved flight's track to show instead of the live one. */
    history: [number, number][] | null;
  }
  let { live, aircraft, origin, history }: Props = $props();

  const STYLE = "https://tiles.openfreemap.org/styles/positron";
  const C = {
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

  let el: HTMLDivElement;
  let map: maplibregl.Map | undefined;
  let marker: maplibregl.Marker | undefined;
  let originMarker: maplibregl.Marker | undefined;
  let endMarker: maplibregl.Marker | undefined;
  let loaded = $state(false);
  let follow = $state(true);
  let offline = $state(false);

  function setPaint(id: string, prop: string, value: unknown) {
    try {
      map!.setPaintProperty(id, prop, value);
    } catch {
      /* property not valid for this layer */
    }
  }

  /** Recolour every layer of the base style by what it draws. */
  function tint() {
    if (!map) return;
    for (const layer of map.getStyle().layers ?? []) {
      const id = layer.id.toLowerCase();
      const src = "source-layer" in layer ? String(layer["source-layer"] ?? "") : "";
      const key = `${id} ${src}`;
      if (layer.type === "background") setPaint(layer.id, "background-color", C.land);
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
        setPaint(layer.id, "fill-color", color);
        setPaint(layer.id, "fill-outline-color", color);
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
        setPaint(layer.id, "line-color", color);
      } else if (layer.type === "symbol") {
        if (key.match(/poi|shield|oneway/)) map.setLayoutProperty(layer.id, "visibility", "none");
        setPaint(layer.id, "text-color", C.text);
        setPaint(layer.id, "text-halo-color", C.halo);
      }
    }
  }

  function addOverlays() {
    if (!map) return;
    map.addSource("track", {
      type: "geojson",
      data: { type: "Feature", properties: {}, geometry: { type: "LineString", coordinates: [] } },
    });
    map.addLayer({
      id: "track-glow",
      type: "line",
      source: "track",
      paint: { "line-color": C.track, "line-width": 9, "line-opacity": 0.18, "line-blur": 4 },
      layout: { "line-cap": "round", "line-join": "round" },
    });
    map.addLayer({
      id: "track",
      type: "line",
      source: "track",
      paint: { "line-color": C.track, "line-width": 2.5 },
      layout: { "line-cap": "round", "line-join": "round" },
    });
  }

  function planeEl() {
    const d = document.createElement("div");
    d.className = "plane";
    d.innerHTML = `<svg viewBox="-24 -24 48 48" width="38" height="38" aria-hidden="true"><path d="M0 -20 L3.4 -6 L19 1.5 L19 5.5 L3.4 2.6 L2.4 13 L8 17 L8 19.6 L0 17.8 L-8 19.6 L-8 17 L-2.4 13 L-3.4 2.6 L-19 5.5 L-19 1.5 L-3.4 -6 Z" fill="#F0B23A" stroke="#0d1820" stroke-width="1.5" stroke-linejoin="round"/></svg>`;
    return d;
  }

  function originEl() {
    const d = document.createElement("div");
    d.className = "origin";
    return d;
  }

  /** A 10° lat/lon grid, so the map still orients you with no tiles. */
  function graticule(): GeoJSON.FeatureCollection {
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

  /** Offline fallback: a plain chart, so the flight is still drawn. */
  function useOfflineStyle() {
    if (!map || loaded || offline) return;
    offline = true;
    map.setStyle({
      version: 8,
      sources: { grid: { type: "geojson", data: graticule() } },
      layers: [
        { id: "bg", type: "background", paint: { "background-color": C.land } },
        { id: "grid", type: "line", source: "grid", paint: { "line-color": C.road, "line-width": 1 } },
      ],
    });
  }

  onMount(() => {
    map = new maplibregl.Map({
      container: el,
      style: STYLE,
      center: [-30, 40],
      zoom: 2.2,
      attributionControl: { compact: true },
      fadeDuration: 0,
    });
    map.addControl(new maplibregl.NavigationControl({ showCompass: true }), "bottom-right");
    map.on("style.load", () => {
      tint();
      addOverlays();
      loaded = true;
    });
    map.on("error", (e) => {
      if (!loaded && String(e.error?.message ?? "").match(/fetch|network|load|style/i)) useOfflineStyle();
    });
    // Give the online style a few seconds before falling back.
    const fallback = setTimeout(useOfflineStyle, 8000);
    map.once("style.load", () => clearTimeout(fallback));
    map.on("dragstart", () => (follow = false));
    // Stop drawing while the window is hidden.
    const vis = () => (document.hidden ? map?.stop() : map?.triggerRepaint());
    document.addEventListener("visibilitychange", vis);
    return () => document.removeEventListener("visibilitychange", vis);
  });

  onDestroy(() => map?.remove());

  function setLine(coords: [number, number][]) {
    const src = map?.getSource("track") as GeoJSONSource | undefined;
    src?.setData({ type: "Feature", properties: {}, geometry: { type: "LineString", coordinates: coords } });
  }

  function fit(coords: [number, number][]) {
    if (!map || coords.length < 2) return;
    let [x0, y0, x1, y1] = [180, 90, -180, -90];
    for (const [x, y] of coords) {
      x0 = Math.min(x0, x);
      y0 = Math.min(y0, y);
      x1 = Math.max(x1, x);
      y1 = Math.max(y1, y);
    }
    const b: LngLatBoundsLike = [
      [x0, y0],
      [x1, y1],
    ];
    map.fitBounds(b, { padding: 80, maxZoom: 11, duration: 600 });
  }

  // Which line to draw: a selected saved flight, else the live track.
  $effect(() => {
    if (!loaded) return;
    if (history) {
      setLine(history);
      fit(history);
    } else {
      setLine(live);
    }
  });

  // Aircraft symbol, rotated to heading; the map follows it unless dragged.
  $effect(() => {
    if (!loaded || !map) return;
    if (!aircraft || history) {
      marker?.remove();
      marker = undefined;
      return;
    }
    const at: [number, number] = [aircraft.lon, aircraft.lat];
    if (!marker) {
      marker = new maplibregl.Marker({ element: planeEl(), rotationAlignment: "map" }).setLngLat(at).addTo(map);
      map.jumpTo({ center: at, zoom: Math.max(map.getZoom(), 8) });
    }
    marker.setLngLat(at).setRotation(aircraft.hdg);
    if (follow) map.easeTo({ center: at, duration: 450, easing: (t) => t });
  });

  // Ring at the departure point; on a saved flight, rings at both ends.
  $effect(() => {
    if (!loaded || !map) return;
    const start = history ? (history[0] ?? null) : origin;
    const end = history && history.length > 1 ? history[history.length - 1] : null;
    if (start) {
      originMarker ??= new maplibregl.Marker({ element: originEl() }).setLngLat(start).addTo(map);
      originMarker.setLngLat(start);
    } else {
      originMarker?.remove();
      originMarker = undefined;
    }
    if (end) {
      endMarker ??= new maplibregl.Marker({ element: originEl() }).setLngLat(end).addTo(map);
      endMarker.setLngLat(end);
    } else {
      endMarker?.remove();
      endMarker = undefined;
    }
  });
</script>

<div class="map" bind:this={el}></div>
{#if aircraft && !follow && !history}
  <button class="follow" onclick={() => (follow = true)}>Follow aircraft</button>
{/if}
{#if offline}
  <p class="offline">Map tiles could not load, so the map shows a plain grid. Tracking keeps working.</p>
{/if}

<style>
  .map {
    position: absolute;
    inset: 0;
    background: #132029;
  }
  .map :global(.plane) {
    filter: drop-shadow(0 0 6px rgba(240, 178, 58, 0.55));
  }
  .map :global(.origin) {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 3px solid var(--magenta);
    background: var(--panel);
  }
  .map :global(.maplibregl-ctrl-group) {
    background: var(--panel-2);
    border: 1px solid var(--rule);
    box-shadow: none;
  }
  .map :global(.maplibregl-ctrl-group button + button) {
    border-top-color: var(--rule);
  }
  .map :global(.maplibregl-ctrl-icon) {
    filter: invert(0.85);
  }
  .map :global(.maplibregl-ctrl-attrib) {
    background: rgba(23, 36, 48, 0.85);
    color: var(--ink-dim);
  }
  .map :global(.maplibregl-ctrl-attrib a) {
    color: var(--ink-dim);
  }
  .follow {
    position: absolute;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    padding: 7px 14px;
    border-radius: 6px;
    border: 1px solid var(--amber);
    background: var(--panel);
    color: var(--amber);
    font-weight: 600;
  }
  .offline {
    position: absolute;
    top: 16px;
    left: 16px;
    margin: 0;
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--panel-2);
    border: 1px solid var(--rule);
    color: var(--ink-dim);
    font-size: 13px;
  }
</style>
