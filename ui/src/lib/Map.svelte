<script lang="ts">
  // Live and historical flight map: MapLibre GL + OpenFreeMap vector tiles
  // (OpenStreetMap data), recoloured at load into a night-chart palette.
  import maplibregl, { type GeoJSONSource, type LngLatBoundsLike } from "maplibre-gl";
  import { onDestroy, onMount, untrack } from "svelte";
  import type { FlightPlan, HangarItem, Route } from "./api";
  import { C as BASE, STYLE, offlineStyle, setBorders, tintMap } from "./mapStyle";
  import MapLayers from "./MapLayers.svelte";
  import { latestRadar } from "./radar";
  import { nightShade } from "./terminator";
  import { day } from "./format";
  import { modelName } from "./hangar/names";
  import { prefs, setPref } from "./prefs.svelte";
  import { altColor, altGradient, altLabel, altRange, lonLat, sideOnBearing, wallFeatures, wallScale, type Pt } from "./track";

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
    /** The flight plan to draw as a dashed route with waypoints. */
    plan?: FlightPlan | null;
    /** Your aircraft where they were last parked (the Parked aircraft layer).
     * Leave out on maps that should not offer the layer. */
    parked?: HangarItem[] | null;
    /** A flight pointed at in the flights list: its route lights up. */
    highlight?: number | null;
    /** "Fly this aircraft" was pressed in a parked aircraft's card. */
    onfly?: (item: HangarItem) => void;
  }
  let { live, aircraft, origin, history, routes = [], selected = null, onroute, plan = null, parked = null, highlight = null, onfly }: Props = $props();

  const PLAN = "#a78bfa";

  /** The planned route: one line from origin through every fix to the
   * destination (long legs follow the great circle), plus points to label. */
  function planFeatures(p: FlightPlan | null): GeoJSON.FeatureCollection {
    if (!p) return { type: "FeatureCollection", features: [] };
    const pts: [number, number][] = [];
    const add = (lon: number, lat: number) => {
      if (!lat && !lon) return;
      const prev = pts[pts.length - 1];
      if (prev && Math.abs(prev[0] - lon) < 1e-6 && Math.abs(prev[1] - lat) < 1e-6) return;
      if (prev && Math.hypot(lon - prev[0], lat - prev[1]) > 4) {
        // Long oceanic legs: follow the great circle, without repeating the start.
        pts.push(...greatCircle(prev, [lon, lat], 24).slice(1));
      } else {
        pts.push([lon, lat]);
      }
    };
    add(p.origin.lon, p.origin.lat);
    for (const f of p.fixes) add(f.lon, f.lat);
    add(p.destination.lon, p.destination.lat);
    const features: GeoJSON.Feature[] = [
      { type: "Feature", properties: { kind: "line" }, geometry: { type: "LineString", coordinates: pts } },
    ];
    for (const f of p.fixes) {
      if (f.kind === "apt" || (!f.lat && !f.lon)) continue;
      features.push({
        type: "Feature",
        properties: { kind: "fix", name: f.ident, proc: f.procedure ? 1 : 0 },
        geometry: { type: "Point", coordinates: [f.lon, f.lat] },
      });
    }
    for (const a of [p.origin, p.destination]) {
      if (!a.lat && !a.lon) continue;
      features.push({ type: "Feature", properties: { kind: "apt", name: a.icao }, geometry: { type: "Point", coordinates: [a.lon, a.lat] } });
    }
    return { type: "FeatureCollection", features };
  }

  const shown = $derived(history ?? live);
  /** 3D view: the map tilts and the track stands up as a wall. */
  const three = $derived(prefs.s.map_3d === "1");
  const stretch = $derived(three && shown.length > 1 ? wallScale(shown) : 1);
  const range = $derived(altRange(shown));
  // The key runs from the lowest to the highest point actually flown, in
  // the same colours the line uses (a low flight never reaches green).
  const keyBar = $derived.by(() => {
    const { lo, hi, top } = range;
    const end = (top - lo) / (hi - lo || 1);
    const stops = Array.from({ length: 9 }, (_, i) => `${altColor((i / 8) * end)} ${(i / 8) * 100}%`);
    return `linear-gradient(90deg, ${stops.join(", ")})`;
  });

  const C = { ...BASE, route: "#c4c8ce", routeHot: "#f0b23a", airport: "#f0b23a" };

  /** Optional layers, saved as a comma list. Past routes and airports are on
   * unless switched off. */
  const DEFAULT_LAYERS = "routes,airports";
  const layers = $derived(new Set((prefs.s.map_layers ?? DEFAULT_LAYERS).split(",").filter(Boolean)));
  function toggleLayer(key: string) {
    const next = new Set(layers);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    setPref("map_layers", [...next].join(","));
  }
  let radarTime = $state<number | null>(null);
  let radarError = $state(false);

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
    // Night side of the Earth, under everything else.
    map.addSource("night", { type: "geojson", data: nightShade() });
    map.addLayer({
      id: "night",
      type: "fill",
      source: "night",
      filter: ["==", ["get", "kind"], "night"],
      layout: { visibility: "none" },
      paint: { "fill-color": "#000000", "fill-opacity": 0.32 },
    });
    map.addLayer({
      id: "night-edge",
      type: "line",
      source: "night",
      filter: ["==", ["get", "kind"], "edge"],
      layout: { visibility: "none" },
      paint: { "line-color": "#f0b23a", "line-opacity": 0.35, "line-width": 1.2, "line-blur": 1 },
    });
    // Every past flight, faint, under the live track.
    map.addSource("routes", { type: "geojson", data: routeFeatures(routes) });
    const hot: maplibregl.ExpressionSpecification = [
      "any",
      ["boolean", ["feature-state", "hover"], false],
      ["boolean", ["feature-state", "sel"], false],
      ["boolean", ["feature-state", "list"], false],
    ];
    map.addLayer({
      id: "routes-line",
      type: "line",
      source: "routes",
      filter: ["==", ["get", "kind"], "route"],
      paint: {
        "line-color": ["case", hot, C.routeHot, C.route] as maplibregl.ExpressionSpecification,
        "line-opacity": ["case", hot, 0.95, 0.5] as maplibregl.ExpressionSpecification,
        "line-width": ["case", hot, 2.6, 1.6] as maplibregl.ExpressionSpecification,
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
        "circle-stroke-opacity": ["case", hot, 0.95, 0.6] as maplibregl.ExpressionSpecification,
        "circle-stroke-width": ["case", hot, 2.4, 1.4] as maplibregl.ExpressionSpecification,
      },
    });
    map.addLayer({
      id: "routes-airports",
      type: "circle",
      source: "routes",
      filter: ["==", ["get", "kind"], "airport"],
      paint: {
        "circle-radius": ["interpolate", ["linear"], ["zoom"], 2, 2.6, 8, 4],
        "circle-color": C.airport,
        "circle-opacity": 0.95,
        "circle-stroke-color": C.land,
        "circle-stroke-width": 1.2,
      },
    });
    // The flight plan, under the flown track.
    map.addSource("plan", { type: "geojson", data: planFeatures(plan) });
    map.addLayer({
      id: "plan-line",
      type: "line",
      source: "plan",
      filter: ["==", ["get", "kind"], "line"],
      paint: { "line-color": PLAN, "line-width": 2, "line-opacity": 0.85, "line-dasharray": [2, 1.6] },
      layout: { "line-cap": "round", "line-join": "round" },
    });
    map.addLayer({
      id: "plan-fixes",
      type: "circle",
      source: "plan",
      filter: ["==", ["get", "kind"], "fix"],
      minzoom: 4,
      paint: {
        "circle-radius": ["interpolate", ["linear"], ["zoom"], 4, 2, 9, 3.5],
        "circle-color": C.land,
        "circle-stroke-color": PLAN,
        "circle-stroke-width": 1.5,
        "circle-opacity": ["case", ["==", ["get", "proc"], 1], 0.6, 1],
        "circle-stroke-opacity": ["case", ["==", ["get", "proc"], 1], 0.6, 1],
      },
    });
    map.addLayer({
      id: "plan-airports",
      type: "circle",
      source: "plan",
      filter: ["==", ["get", "kind"], "apt"],
      paint: { "circle-radius": 5, "circle-color": PLAN, "circle-stroke-color": C.land, "circle-stroke-width": 2 },
    });
    // Labels need the base map's font; the offline grid has none.
    if (map.getStyle().glyphs) {
      map.addLayer({
        id: "plan-labels",
        type: "symbol",
        source: "plan",
        filter: ["any", ["==", ["get", "kind"], "apt"], ["all", ["==", ["get", "kind"], "fix"], [">=", ["zoom"], 5.5]]],
        layout: {
          "text-field": ["get", "name"],
          "text-size": ["case", ["==", ["get", "kind"], "apt"], 13, 11],
          "text-offset": [0, 1.1],
          "text-anchor": "top",
          "text-font": ["Noto Sans Bold"],
          "text-allow-overlap": false,
        },
        paint: { "text-color": "#cfc4ff", "text-halo-color": C.land, "text-halo-width": 1.4 },
      });
    }
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
    // Soft, even light so the wall keeps its colours from every side.
    map.setLight({ anchor: "viewport", color: "#ffffff", intensity: 0.18, position: [1.15, 210, 30] });
    // The 3D wall, from the ground up to the altitude flown. Only shown in 3D.
    // tolerance 0: the wall is made of many small, thin pieces that the
    // usual simplification would otherwise drop when zoomed out.
    map.addSource("wall", { type: "geojson", data: { type: "FeatureCollection", features: [] }, tolerance: 0 });
    map.addLayer({
      id: "track-wall",
      type: "fill-extrusion",
      source: "wall",
      layout: { visibility: three ? "visible" : "none" },
      paint: {
        "fill-extrusion-color": ["get", "c"],
        "fill-extrusion-height": ["get", "h"],
        "fill-extrusion-base": 0,
        "fill-extrusion-opacity": 0.78,
        "fill-extrusion-vertical-gradient": true,
      },
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
    d.innerHTML = `<svg viewBox="-24 -24 48 48" width="38" height="38" aria-hidden="true"><path d="M0 -20 L3.4 -6 L19 1.5 L19 5.5 L3.4 2.6 L2.4 13 L8 17 L8 19.6 L0 17.8 L-8 19.6 L-8 17 L-2.4 13 L-3.4 2.6 L-19 5.5 L-19 1.5 L-3.4 -6 Z" fill="#F0B23A" stroke="#111214" stroke-width="1.5" stroke-linejoin="round"/></svg>`;
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
      maxPitch: 75,
    });
    map.addControl(new maplibregl.NavigationControl({ showCompass: true, visualizePitch: true }), "bottom-right");
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
    setWall(pts);
  }

  function setWall(pts: Pt[]) {
    const src = map?.getSource("wall") as GeoJSONSource | undefined;
    if (!src) return;
    // Read without tracking, so drawing a line never depends on (and never
    // re-runs because of) the 2D/3D switch.
    const on = untrack(() => three);
    src.setData(on ? wallFeatures(pts, wallScale(pts)) : { type: "FeatureCollection", features: [] });
  }

  function setThree(on: boolean) {
    setPref("map_3d", on ? "1" : "0");
  }

  // The flight plan: redraw when it changes, and show the whole route when
  // a plan arrives before any flying.
  let lastPlanId: string | null = null;
  $effect(() => {
    const p = plan;
    if (!loaded || !map) return;
    (map.getSource("plan") as GeoJSONSource | undefined)?.setData(planFeatures(p));
    const id = p ? `${p.id}-${p.origin.icao}-${p.destination.icao}` : null;
    if (p && id !== lastPlanId && !untrack(() => history) && untrack(() => live.length) < 2) {
      const coords: [number, number][] = [[p.origin.lon, p.origin.lat], ...p.fixes.map((f) => [f.lon, f.lat] as [number, number]), [p.destination.lon, p.destination.lat]];
      fit(coords.filter(([x, y]) => x || y), 8);
    }
    lastPlanId = id;
  });

  // Optional layers: past routes, airports, borders, day/night, radar.
  $effect(() => {
    const on = layers;
    if (!loaded || !map || !map.getLayer("routes-line")) return;
    const vis = (id: string, v: boolean) => map!.getLayer(id) && map!.setLayoutProperty(id, "visibility", v ? "visible" : "none");
    for (const id of ["routes-line", "routes-hit", "routes-loops"]) vis(id, on.has("routes"));
    vis("routes-airports", on.has("airports"));
    vis("night", on.has("night"));
    vis("night-edge", on.has("night"));
    setBorders(map, on.has("borders"));
    showRadar(on.has("radar"));
  });

  // The night side moves; redraw it every minute.
  $effect(() => {
    if (!loaded || !layers.has("night")) return;
    const t = setInterval(() => (map?.getSource("night") as GeoJSONSource | undefined)?.setData(nightShade()), 60_000);
    (map?.getSource("night") as GeoJSONSource | undefined)?.setData(nightShade());
    return () => clearInterval(t);
  });

  let radarTimer: ReturnType<typeof setInterval> | undefined;
  async function refreshRadar() {
    const f = await latestRadar();
    if (!map || !untrack(() => layers.has("radar"))) return;
    radarError = !f;
    if (!f) return;
    radarTime = f.time;
    const src = map.getSource("radar") as maplibregl.RasterTileSource | undefined;
    if (src) {
      src.setTiles([f.tiles]);
    } else {
      // Radar sits above the land but below routes and the flight.
      map.addSource("radar", { type: "raster", tiles: [f.tiles], tileSize: 256, maxzoom: 7, attribution: "Radar © RainViewer" });
      map.addLayer(
        { id: "radar", type: "raster", source: "radar", paint: { "raster-opacity": 0.6, "raster-fade-duration": 0 } },
        map.getLayer("routes-line") ? "routes-line" : undefined,
      );
    }
  }
  function showRadar(on: boolean) {
    if (!map) return;
    if (on) {
      if (!radarTimer) {
        refreshRadar();
        radarTimer = setInterval(refreshRadar, 10 * 60_000);
      } else if (!map.getSource("radar")) {
        refreshRadar();
      }
      if (map.getLayer("radar")) map.setLayoutProperty("radar", "visibility", "visible");
    } else {
      clearInterval(radarTimer);
      radarTimer = undefined;
      if (map.getLayer("radar")) map.setLayoutProperty("radar", "visibility", "none");
    }
  }
  onDestroy(() => clearInterval(radarTimer));

  // Switch between flat and 3D: tilt the camera and raise or hide the wall.
  let wasThree: boolean | null = null;
  $effect(() => {
    const on = three;
    if (!loaded || !map || !map.getLayer("track-wall")) return;
    map.setLayoutProperty("track-wall", "visibility", on ? "visible" : "none");
    map.setPaintProperty("track", "line-opacity", on ? 0.55 : 1);
    untrack(() => setWall(history ?? live));
    if (wasThree !== null && wasThree !== on) {
      const brg = untrack(() => sideOnBearing(history ?? live));
      map.easeTo(on ? { pitch: 60, bearing: brg, duration: 900 } : { pitch: 0, bearing: 0, duration: 700 });
    } else if (wasThree === null && on) {
      map.jumpTo({ pitch: 60, bearing: untrack(() => sideOnBearing(history ?? live)) });
    }
    wasThree = on;
  });

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
    // In 3D keep the tilt, with extra room at the top for the wall.
    const tilted = untrack(() => three);
    map.fitBounds(b, {
      padding: tilted ? { top: 160, bottom: 60, left: 80, right: 80 } : 80,
      maxZoom,
      duration: 600,
      pitch: tilted ? 60 : 0,
      bearing: tilted ? sideOnBearing(untrack(() => history ?? live)) : 0,
    });
  }

  // Keep the route network current, and show the whole network once when
  // the map first opens with nothing in flight.
  let fittedRoutes = false;
  $effect(() => {
    const list = routes;
    if (!loaded || !map) return;
    (map.getSource("routes") as GeoJSONSource | undefined)?.setData(routeFeatures(list));
    if (!fittedRoutes && list.length && !aircraft && !history && !untrack(() => plan)) {
      fittedRoutes = true;
      fit(list.flatMap((r) => [r.from, r.to]), 6);
    }
  });

  // A flight pointed at in the list lights up in amber.
  let lastHi: number | null = null;
  $effect(() => {
    const id = highlight;
    if (!loaded || !map || !map.getSource("routes")) return;
    if (lastHi != null) map.setFeatureState({ source: "routes", id: lastHi }, { list: false });
    if (id != null) map.setFeatureState({ source: "routes", id }, { list: true });
    lastHi = id;
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

  // Parked aircraft: one tag per airport listing every aircraft left
  // there, shown only when the layer is switched on.
  let parkedMarkers: maplibregl.Marker[] = [];
  let parkedPopups: maplibregl.Popup[] = [];
  $effect(() => {
    const list = parked;
    const on = layers.has("parked");
    const fly = onfly;
    if (!loaded || !map) return;
    for (const m of parkedMarkers) m.remove();
    for (const p of parkedPopups) p.remove();
    parkedMarkers = [];
    parkedPopups = [];
    if (!list || !on) return;
    const spots: Record<string, HangarItem[]> = {};
    for (const it of list) {
      if (it.last_lat == null || it.last_lon == null) continue;
      const k = `${it.last_lat.toFixed(3)},${it.last_lon.toFixed(3)}`;
      (spots[k] ??= []).push(it);
    }
    for (const group of Object.values(spots)) {
      const first = group[0];
      const el = document.createElement("div");
      el.className = "parked";
      const regs = group.map((g) => esc(g.registration || g.icao_type || "Aircraft"));
      el.innerHTML =
        `<svg viewBox="-24 -24 48 48" width="22" height="22" aria-hidden="true"><path d="M0 -20 L3.4 -6 L19 1.5 L19 5.5 L3.4 2.6 L2.4 13 L8 17 L8 19.6 L0 17.8 L-8 19.6 L-8 17 L-2.4 13 L-3.4 2.6 L-19 5.5 L-19 1.5 L-3.4 -6 Z" fill="#c4c8ce" stroke="#111214" stroke-width="2" stroke-linejoin="round"/></svg>` +
        `<span>${regs.slice(0, 3).join(" · ")}${regs.length > 3 ? ` +${regs.length - 3}` : ""}</span>`;
      // The card: where, then each aircraft with a Fly this aircraft button.
      const box = document.createElement("div");
      const where = first.last_icao ? `${esc(first.last_icao)}${first.last_name ? ` · ${esc(first.last_name)}` : ""}` : "Last known position";
      box.innerHTML = `<div class="pk-where">Parked at ${where}</div>`;
      for (const g of group) {
        const row = document.createElement("div");
        row.className = "pk-row";
        const hrs = (g.minutes / 60).toFixed(1);
        row.innerHTML = `<strong>${esc(g.registration || g.icao_type || "Unregistered")}</strong><span>${esc(modelName(g))}</span><em>${g.flights} ${g.flights === 1 ? "flight" : "flights"} · ${hrs} h · last flown ${esc(day(g.last_flight_ms))}</em>`;
        if (fly) {
          const b = document.createElement("button");
          b.className = "pk-fly";
          b.type = "button";
          b.textContent = "Fly this aircraft";
          b.addEventListener("click", (ev) => {
            ev.stopPropagation();
            popup.remove();
            fly(g);
          });
          row.appendChild(b);
        }
        box.appendChild(row);
      }
      const popup = new maplibregl.Popup({ className: "parked-tip", offset: [0, -12], closeButton: false, maxWidth: "300px" }).setDOMContent(box);
      const at: [number, number] = [first.last_lon!, first.last_lat!];
      // Handle the click here so it never reaches a route line underneath.
      el.addEventListener("click", (ev) => {
        ev.stopPropagation();
        if (popup.isOpen()) popup.remove();
        else popup.setLngLat(at).addTo(map!);
      });
      parkedMarkers.push(new maplibregl.Marker({ element: el, anchor: "left", offset: [-11, 0] }).setLngLat(at).addTo(map));
      parkedPopups.push(popup);
    }
  });
  onDestroy(() => parkedMarkers.forEach((m) => m.remove()));

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
<MapLayers {three} on={layers} onthree={setThree} ontoggle={toggleLayer} {radarTime} {radarError} parkedLayer={parked != null} />
{#if shown.length > 1}
  <div class="alt-key" aria-label="Line colour shows altitude">
    <span class="t">Altitude{#if three && stretch > 1}<em>heights ×{stretch}</em>{/if}</span>
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
    background: #17191b;
  }
  .map :global(.plane) {
    filter: drop-shadow(0 0 6px rgba(240, 178, 58, 0.55));
  }
  .map :global(.parked) {
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.6));
  }
  .map :global(.parked span) {
    padding: 2px 7px;
    border-radius: 4px;
    background: rgba(28, 30, 33, 0.92);
    border: 1px solid var(--rule);
    color: var(--ink);
    font: 600 11.5px "IBM Plex Mono", ui-monospace, monospace;
    white-space: nowrap;
  }
  .map :global(.parked:hover span) {
    border-color: var(--amber);
  }
  .map :global(.parked-tip .maplibregl-popup-content) {
    background: var(--panel);
    color: var(--ink);
    border: 1px solid var(--rule);
    border-radius: 8px;
    padding: 10px 12px;
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.45);
    font-size: 12.5px;
  }
  .map :global(.parked-tip .maplibregl-popup-tip) {
    display: none;
  }
  .map :global(.pk-where) {
    color: var(--ink-dim);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 6px;
  }
  .map :global(.pk-row) {
    display: flex;
    flex-direction: column;
    padding: 4px 0;
  }
  .map :global(.pk-row + .pk-row) {
    border-top: 1px solid var(--rule);
  }
  .map :global(.pk-row strong) {
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 14px;
  }
  .map :global(.pk-fly) {
    align-self: flex-start;
    margin-top: 6px;
    border: none;
    border-radius: 6px;
    padding: 5px 10px;
    background: var(--amber);
    color: #1b1406;
    font: 600 12px var(--cond), sans-serif;
    cursor: pointer;
  }
  .map :global(.pk-fly:hover) {
    filter: brightness(1.08);
  }
  .map :global(.pk-row em) {
    font-style: normal;
    color: var(--ink-dim);
    font-size: 11.5px;
  }
  .map :global(.origin) {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 3px solid var(--amber);
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
    background: rgba(28, 30, 33, 0.85);
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
    background: rgba(28, 30, 33, 0.88);
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
  .alt-key .t em {
    font-style: normal;
    text-transform: none;
    letter-spacing: 0;
    margin-left: 8px;
    color: var(--ink-dim);
    opacity: 0.8;
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
