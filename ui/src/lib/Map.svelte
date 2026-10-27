<script lang="ts">
  // Live and historical flight map: MapLibre GL + OpenFreeMap vector tiles
  // (OpenStreetMap data), recoloured at load into a night-chart palette.
  import maplibregl, { type GeoJSONSource, type LngLatBoundsLike } from "maplibre-gl";
  import { onDestroy, onMount } from "svelte";
  import type { Route } from "./api";
  import { C as BASE, STYLE, offlineStyle, tintMap } from "./mapStyle";
  import { altColor, altGradient, altLabel, altRange, lonLat, type Pt } from "./track";

  interface Props {
    /** Live track with altitude. */
    live: Pt[];
    /** Current aircraft position and heading, if flying. */
    aircraft: { lon: number; lat: number; hdg: number } | null;
    /** Departure point of the live flight. */
    origin: [number, number] | null;
    /** A saved flight's track to show instead of the live one. */
    history: Pt[] | null;
    /** Every flight flown, drawn faintly between its airports. */
    routes?: Route[];
    /** The flight whose route should stand out. */
    selected?: number | null;
    /** A route line was clicked. */
    onroute?: (flightId: number) => void;
  }
  let { live, aircraft, origin, history, routes = [], selected = null, onroute }: Props = $props();

  const shown = $derived(history ?? live);
  const range = $derived(altRange(shown));
  // The key runs from the lowest to the highest point actually flown, in
  // the same colours the line uses (a low flight never reaches green).
  const keyBar = $derived.by(() => {
    const { lo, hi, top } = range;
    const end = (top - lo) / (hi - lo || 1);
    const stops = Array.from({ length: 9 }, (_, i) => `${altColor((i / 8) * end)} ${(i / 8) * 100}%`);
    return `linear-gradient(90deg, ${stops.join(", ")})`;
  });

  const C = { ...BASE, route: "#a9bfd1", routeHot: "#f0b23a" };

  const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  const esc = (s: string) => s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);

  /** Points along the great circle from a to b, longitudes unwrapped so the
   * line never jumps across the antimeridian. */
  function greatCircle(a: [number, number], b: [number, number], n = 64): [number, number][] {
    const rad = Math.PI / 180;
    const v = ([lon, lat]: [number, number]) => [
      Math.cos(lat * rad) * Math.cos(lon * rad),
      Math.cos(lat * rad) * Math.sin(lon * rad),
      Math.sin(lat * rad),
    ];
    const p = v(a);
    const q = v(b);
    const dot = Math.min(1, Math.max(-1, p[0] * q[0] + p[1] * q[1] + p[2] * q[2]));
    const w = Math.acos(dot);
    if (w < 1e-6) return [a, b];
    const out: [number, number][] = [];
    let prev = a[0];
    for (let i = 0; i <= n; i++) {
      const f = i / n;
      const s1 = Math.sin((1 - f) * w) / Math.sin(w);
      const s2 = Math.sin(f * w) / Math.sin(w);
      const x = s1 * p[0] + s2 * q[0];
      const y = s1 * p[1] + s2 * q[1];
      const z = s1 * p[2] + s2 * q[2];
      let lon = Math.atan2(y, x) / rad;
      const lat = Math.atan2(z, Math.hypot(x, y)) / rad;
      while (lon - prev > 180) lon -= 360;
      while (lon - prev < -180) lon += 360;
      prev = lon;
      out.push([lon, lat]);
    }
    return out;
  }

  function routeFeatures(list: Route[]): GeoJSON.FeatureCollection {
    const features: GeoJSON.Feature[] = [];
    const airports = new globalThis.Map<string, [number, number]>();
    for (const r of list) {
      const label = `${r.dep_icao ?? "?"} to ${r.arr_icao ?? "?"}`;
      const d = new Date(r.date_ms);
      const props = {
        id: r.flight_id,
        label,
        sub: `${d.getUTCDate()} ${months[d.getUTCMonth()]} ${d.getUTCFullYear()}${r.aircraft_type ? `, ${r.aircraft_type}` : ""}`,
      };
      const same = Math.abs(r.from[0] - r.to[0]) < 0.02 && Math.abs(r.from[1] - r.to[1]) < 0.02;
      if (same) {
        // Took off and landed at the same place: a ring at the airport.
        features.push({ type: "Feature", id: r.flight_id, properties: { ...props, kind: "loop" }, geometry: { type: "Point", coordinates: r.from } });
      } else {
        features.push({
          type: "Feature",
          id: r.flight_id,
          properties: { ...props, kind: "route" },
          geometry: { type: "LineString", coordinates: greatCircle(r.from, r.to) },
        });
      }
      airports.set(`${r.from[0].toFixed(3)},${r.from[1].toFixed(3)}`, r.from);
      airports.set(`${r.to[0].toFixed(3)},${r.to[1].toFixed(3)}`, r.to);
    }
    let k = 0;
    for (const at of airports.values()) {
      features.push({ type: "Feature", id: -(++k), properties: { kind: "airport" }, geometry: { type: "Point", coordinates: at } });
    }
    return { type: "FeatureCollection", features };
  }

  let el: HTMLDivElement;
  let map: maplibregl.Map | undefined;
  let marker: maplibregl.Marker | undefined;
  let originMarker: maplibregl.Marker | undefined;
  let endMarker: maplibregl.Marker | undefined;
  let loaded = $state(false);
  let follow = $state(true);
  let offline = $state(false);

  function tint() {
    if (map) tintMap(map);
  }

  function addOverlays() {
    if (!map) return;
    // Every past flight, faint, under the live track.
    map.addSource("routes", { type: "geojson", data: routeFeatures(routes) });
    const hot: maplibregl.ExpressionSpecification = [
      "any",
      ["boolean", ["feature-state", "hover"], false],
      ["boolean", ["feature-state", "sel"], false],
    ];
    map.addLayer({
      id: "routes-line",
      type: "line",
      source: "routes",
      filter: ["==", ["get", "kind"], "route"],
      paint: {
        "line-color": ["case", hot, C.routeHot, C.route] as maplibregl.ExpressionSpecification,
        "line-opacity": ["case", hot, 0.95, 0.32] as maplibregl.ExpressionSpecification,
        "line-width": ["case", hot, 2.6, 1.4] as maplibregl.ExpressionSpecification,
      },
      layout: { "line-cap": "round" },
    });
    map.addLayer({
      id: "routes-hit",
      type: "line",
      source: "routes",
      filter: ["==", ["get", "kind"], "route"],
      paint: { "line-color": "#000", "line-opacity": 0, "line-width": 14 },
    });
    map.addLayer({
      id: "routes-loops",
      type: "circle",
      source: "routes",
      filter: ["==", ["get", "kind"], "loop"],
      paint: {
        "circle-radius": 8,
        "circle-color": "rgba(0,0,0,0)",
        "circle-stroke-color": ["case", hot, C.routeHot, C.route] as maplibregl.ExpressionSpecification,
        "circle-stroke-opacity": ["case", hot, 0.95, 0.45] as maplibregl.ExpressionSpecification,
        "circle-stroke-width": ["case", hot, 2.4, 1.4] as maplibregl.ExpressionSpecification,
      },
    });
    map.addLayer({
      id: "routes-airports",
      type: "circle",
      source: "routes",
      filter: ["==", ["get", "kind"], "airport"],
      paint: { "circle-radius": 2.6, "circle-color": "#c9d6e0", "circle-opacity": 0.65 },
    });
    map.addSource("track", {
      type: "geojson",
      data: { type: "Feature", properties: {}, geometry: { type: "LineString", coordinates: [] } },
      lineMetrics: true,
    });
    map.addLayer({
      id: "track-glow",
      type: "line",
      source: "track",
      paint: { "line-gradient": altGradient([]), "line-width": 10, "line-opacity": 0.22, "line-blur": 5 },
      layout: { "line-cap": "round", "line-join": "round" },
    });
    map.addLayer({
      id: "track",
      type: "line",
      source: "track",
      paint: { "line-gradient": altGradient([]), "line-width": 3 },
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

  /** Offline fallback: a plain chart, so the flight is still drawn. */
  function useOfflineStyle() {
    if (!map || loaded || offline) return;
    offline = true;
    map.setStyle(offlineStyle());
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

    // Hover and click on past routes.
    const tip = new maplibregl.Popup({ closeButton: false, closeOnClick: false, className: "route-tip", offset: 10 });
    let hovered: number | null = null;
    const clearHover = () => {
      if (hovered != null) map?.setFeatureState({ source: "routes", id: hovered }, { hover: false });
      hovered = null;
      tip.remove();
      map!.getCanvas().style.cursor = "";
    };
    for (const layer of ["routes-hit", "routes-loops"]) {
      map.on("mousemove", layer, (e) => {
        const f = e.features?.[0];
        if (!f || f.id == null || !map) return;
        const id = Number(f.id);
        if (hovered !== id) {
          clearHover();
          hovered = id;
          map.setFeatureState({ source: "routes", id }, { hover: true });
        }
        map.getCanvas().style.cursor = "pointer";
        const p = f.properties as { label: string; sub: string };
        tip.setLngLat(e.lngLat).setHTML(`<strong>${esc(p.label)}</strong><span>${esc(p.sub)}</span><em>Click for flight details</em>`).addTo(map);
      });
      map.on("mouseleave", layer, clearHover);
      map.on("click", layer, (e) => {
        const f = e.features?.[0];
        clearHover();
        if (f?.id != null) onroute?.(Number(f.id));
      });
    }
    map.on("movestart", () => tip.remove());
    // Stop drawing while the window is hidden.
    const vis = () => (document.hidden ? map?.stop() : map?.triggerRepaint());
    document.addEventListener("visibilitychange", vis);
    return () => document.removeEventListener("visibilitychange", vis);
  });

  onDestroy(() => map?.remove());

  function setLine(pts: Pt[]) {
    const src = map?.getSource("track") as GeoJSONSource | undefined;
    if (!src || !map) return;
    src.setData({ type: "Feature", properties: {}, geometry: { type: "LineString", coordinates: lonLat(pts) } });
    const g = altGradient(pts);
    map.setPaintProperty("track", "line-gradient", g);
    map.setPaintProperty("track-glow", "line-gradient", g);
  }

  function fit(coords: [number, number][], maxZoom = 11) {
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
    map.fitBounds(b, { padding: 80, maxZoom, duration: 600 });
  }

  // Keep the route network current, and show the whole network once when
  // the map first opens with nothing in flight.
  let fittedRoutes = false;
  $effect(() => {
    const list = routes;
    if (!loaded || !map) return;
    (map.getSource("routes") as GeoJSONSource | undefined)?.setData(routeFeatures(list));
    if (!fittedRoutes && list.length && !aircraft && !history) {
      fittedRoutes = true;
      fit(list.flatMap((r) => [r.from, r.to]), 6);
    }
  });

  // The selected flight's route stands out.
  let lastSel: number | null = null;
  $effect(() => {
    const sel = selected;
    if (!loaded || !map || !map.getSource("routes")) return;
    if (lastSel != null) map.setFeatureState({ source: "routes", id: lastSel }, { sel: false });
    if (sel != null) map.setFeatureState({ source: "routes", id: sel }, { sel: true });
    lastSel = sel;
  });

  // Which line to draw: a selected saved flight, else the live track.
  $effect(() => {
    if (!loaded) return;
    if (history) {
      setLine(history);
      fit(lonLat(history));
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
    const first = history?.[0];
    const lastPt = history && history.length > 1 ? history[history.length - 1] : null;
    const start: [number, number] | null = history ? (first ? [first[0], first[1]] : null) : origin;
    const end: [number, number] | null = lastPt ? [lastPt[0], lastPt[1]] : null;
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
{#if shown.length > 1}
  <div class="alt-key" aria-label="Line colour shows altitude">
    <span class="t">Altitude</span>
    <span class="bar" style:background={keyBar}></span>
    <span class="ends"><span>{altLabel(range.lo)}</span><span>{altLabel(range.top)}</span></span>
  </div>
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
    border: 3px solid #e8eef2;
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
  .map :global(.route-tip .maplibregl-popup-content) {
    background: var(--panel-2);
    color: var(--ink);
    border: 1px solid var(--rule);
    border-radius: 6px;
    padding: 7px 10px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    gap: 1px;
    font-family: var(--sans);
    font-size: 13px;
  }
  .map :global(.route-tip strong) {
    font-family: var(--cond);
    font-size: 17px;
  }
  .map :global(.route-tip span) {
    color: var(--ink-dim);
  }
  .map :global(.route-tip em) {
    font-style: normal;
    font-size: 11px;
    color: var(--amber);
  }
  .map :global(.route-tip .maplibregl-popup-tip) {
    display: none;
  }
  .alt-key {
    position: absolute;
    left: 12px;
    bottom: 62px;
    width: 190px;
    padding: 8px 10px 7px;
    border-radius: 8px;
    background: rgba(23, 36, 48, 0.88);
    border: 1px solid var(--rule);
    display: flex;
    flex-direction: column;
    gap: 5px;
    pointer-events: none;
  }
  .alt-key .t {
    font-family: var(--cond);
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-dim);
  }
  .alt-key .bar {
    height: 6px;
    border-radius: 3px;
  }
  .alt-key .ends {
    display: flex;
    justify-content: space-between;
    font-family: var(--mono, "IBM Plex Mono", monospace);
    font-size: 11px;
    color: var(--ink);
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
