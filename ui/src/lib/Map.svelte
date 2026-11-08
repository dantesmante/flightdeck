<script lang="ts">
  // Live and historical flight map: MapLibre GL + OpenFreeMap vector tiles
  // (OpenStreetMap data), recolored at load into a night-chart palette.
  import maplibregl, { type GeoJSONSource, type LngLatBoundsLike } from "maplibre-gl";
  import { onDestroy, onMount, untrack } from "svelte";
  import { api, type FlightPlan, type HangarItem, type MapAirport, type Route } from "./api";
  import AirportCard from "./AirportCard.svelte";
  import NetworkCard, { NET_COLOR, POSITION, type NetPick } from "./NetworkCard.svelte";
  import WxCard, { CATEGORY, WX_GROUP } from "./WxCard.svelte";
  import type { WxAdvisoryProps } from "./api";
  import type { Net, NetworkData } from "./api";
  import { showRoads, setRoadsOpacity, showTaws, showObstacles, showRings, autoRingStep, C as BASE, STYLE, addAirportLayout, addRegionLines, approachLines, offlineStyle, runwayEnds, setBorders, showAirportLayout, showSatellite, showTerrain, tintMap } from "./mapStyle";
  import MapLayers from "./MapLayers.svelte";
  import { shots as shotStore, type PlacedShot } from "./shots.svelte";
  import { WindParticles } from "./wind/particles";
  import { addWindIcons, windIconExpr, windText } from "./wind/arrows";
  import { WIND_LEVELS, WIND_STOPS, decodeGrid, dirSpeed, levelFor, sample, type Field } from "./wind/wind";
  import { latestRadar } from "./radar";
  import { nightShade } from "./terminator";
  import { day } from "./format";
  import { modelName } from "./hangar/names";
  import { prefs, setPref } from "./prefs.svelte";
  import { drawing, pick } from "./flyStore.svelte";
  import { pfdState } from "./pfd/store.svelte";
  import { profileState } from "./profile/store.svelte";
  import { altColor, altGradient, altLabel, altRange, lonLat, sideOnBearing, wallFeatures, wallScale, type Pt } from "./track";

  interface Props {
    /** Live track with altitude. */
    live: Pt[];
    /** Current aircraft position and heading while the sim is connected;
     * `tracking` while a flight is being recorded. */
    aircraft: { lon: number; lat: number; hdg: number; tracking?: boolean } | null;
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
    /** A route being set up and not flown yet: drawn like a plan when there is none. */
    draft?: { origin: { icao: string; name: string; lat: number; lon: number }; dest: { icao: string; name: string; lat: number; lon: number }; fixes?: FlightPlan["fixes"] } | null;
    /** Room taken by cards over the right and bottom of the map, in pixels:
     * the map controls move clear of it and routes fit beside it. */
    insetRight?: number;
    insetBottom?: number;
    /** Room taken by a card over the left of the map, in pixels. */
    insetLeft?: number;
    /** Screenshots to mark on the map where they were taken. */
    shots?: PlacedShot[];
    /** Layers this map shows from the start whatever the saved choice
     * (they can still be switched off here, without changing it). */
    initialOn?: string[];
    /** Right-click on an airport can send it to the flight planner (the
     * Map tab, where Start a flight is). */
    planner?: boolean;
    /** The Photos tab: the map shows nothing but the screenshot pins. */
    bare?: boolean;
    /** A screenshot pin to bring out (its thumbnail is under the pointer). */
    shotAccent?: number | null;
    /** The pointer is on a screenshot pin (or left it: null). */
    onshothover?: (id: number | null) => void;
    /** The aircraft's altitude (ft above sea level) while a sim is connected,
     * for the terrain and obstacle colours. */
    acAltFt?: number | null;
    /** The aircraft on the ground, and where it is [lon, lat] (terrain colours). */
    acOnGround?: boolean;
    acPos?: [number, number] | null;
  }
  let {
    live,
    aircraft,
    origin,
    history,
    routes = [],
    selected = null,
    onroute,
    plan = null,
    parked = null,
    highlight = null,
    onfly,
    draft = null,
    insetRight = 0,
    insetBottom = 0,
    insetLeft = 0,
    initialOn = [],
    shots: shotList = [],
    planner = false,
    bare = false,
    shotAccent = null,
    onshothover,
    acAltFt = null,
    acOnGround = false,
    acPos = null,
  }: Props = $props();

  /** The plan drawn: the real one, or the route being set up. */
  const shownPlan = $derived<FlightPlan | null>(
    plan ??
      (draft
        ? ({
            id: "draft",
            origin: { ...draft.origin, runway: "" },
            destination: { ...draft.dest, runway: "" },
            fixes: draft.fixes ?? [],
          } as unknown as FlightPlan)
        : null),
  );

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
  const three = $derived(!bare && prefs.s.map_3d === "1");
  const stretch = $derived(three && shown.length > 1 ? wallScale(shown) : 1);
  const range = $derived(altRange(shown));
  // The key runs from the lowest to the highest point actually flown, in
  // the same colors the line uses (a low flight never reaches green).
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
  /** Layers on for this map only (initialOn), until switched off here. */
  let localOn = $state(new Set(untrack(() => initialOn)));
  // A new set (as when the app moves to the Flights page) starts over.
  let lastInitial = untrack(() => initialOn.join(","));
  $effect(() => {
    const k = initialOn.join(",");
    if (k !== lastInitial) {
      lastInitial = k;
      localOn = new Set(initialOn);
    }
  });
  /** How the map itself looks (kept in the Photos tab, where nothing else shows). */
  const LOOKS = ["night", "satellite", "terrain", "borders", "roads"];
  const layers = $derived.by(() => {
    const all = new Set([...(prefs.s.map_layers ?? DEFAULT_LAYERS).split(",").filter(Boolean), ...localOn]);
    return bare ? new Set([...LOOKS.filter((k) => all.has(k)), "aeroway-off"]) : all;
  });
  function toggleLayer(key: string) {
    if (localOn.has(key)) {
      const next = new Set(localOn);
      next.delete(key);
      localOn = next;
      const saved = new Set((prefs.s.map_layers ?? DEFAULT_LAYERS).split(",").filter(Boolean));
      if (saved.has(key)) {
        saved.delete(key);
        setPref("map_layers", [...saved].join(","));
      }
      return;
    }
    const next = new Set(layers);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    setPref("map_layers", [...next].join(","));
  }
  /** The airport whose card is open. */
  let openAirport = $state<string | null>(null);
  /** A SIGMET or AIRMET whose card is open. */
  let wxPick = $state<WxAdvisoryProps | null>(null);
  // How see-through the roads are (Layers → Map).
  const roadsOpacity = $derived(Math.min(1, Math.max(0.15, Number(prefs.s.map_roads_opacity ?? "1") || 1)));
  $effect(() => {
    const o = roadsOpacity;
    if (loaded && map) setRoadsOpacity(map, o);
  });

  // Terrain and obstacles coloured against the aircraft's altitude, redrawn
  // when it has changed by 50 ft or more.
  let altStep = $state<number | null>(null);
  $effect(() => {
    const a = acAltFt;
    untrack(() => {
      if (a == null) altStep = null;
      else if (altStep == null || Math.abs(a - altStep) >= 50) altStep = Math.round(a);
    });
  });
  // Where the aircraft is, in steps of about 5 nm (the terrain area follows it).
  const posStep = $derived<[number, number] | null>(acPos ? [Math.round(acPos[0] * 12) / 12, Math.round(acPos[1] * 12) / 12] : null);
  $effect(() => {
    const on = layers.has("taws");
    const alt = altStep;
    const ground = acOnGround;
    const pos = posStep;
    if (!loaded || !map) return;
    showTaws(map, on && !offline, alt, ground, pos);
  });
  // Range rings around the aircraft (Layers → Map): 2 or 3 rings, at set
  // distances or spaced to suit the zoom ("auto").
  const ringCount = $derived(prefs.s.map_rings_count === "3" ? 3 : 2);
  const ringSet = $derived(["5", "10", "15"].includes(prefs.s.map_rings_nm ?? "") ? prefs.s.map_rings_nm! : "auto");
  const ringStep = $derived.by(() => {
    if (ringSet !== "auto") return Number(ringSet);
    const el = map?.getContainer();
    return autoRingStep(zoomNow, acPos?.[1] ?? 0, el ? Math.min(el.clientWidth, el.clientHeight) : 800, ringCount);
  });
  $effect(() => {
    const on = layers.has("rings") && !bare;
    const pos = acPos;
    const nm = Array.from({ length: ringCount }, (_, i) => (i + 1) * ringStep);
    if (!loaded || !map) return;
    showRings(map, on, pos, nm);
  });
  let obstData: GeoJSON.FeatureCollection | null = null;
  let obstState = $state<"idle" | "loading" | "ok" | "error">("idle");
  $effect(() => {
    const on = layers.has("obstacles");
    const alt = altStep;
    if (!loaded || !map) return;
    showObstacles(map, on, alt, acOnGround);
    if (!on || obstData || untrack(() => obstState) === "loading") return;
    untrack(() => (obstState = "loading"));
    api.usObstacles()
      .then((text) => {
        obstData = JSON.parse(text) as GeoJSON.FeatureCollection;
        obstState = "ok";
        (map?.getSource("fd-obst") as GeoJSONSource | undefined)?.setData(obstData);
      })
      .catch(() => (obstState = "error"));
  });
  // Style reloads (satellite) start the obstacles empty again.
  $effect(() => {
    void loaded;
    if (loaded && obstData) (map?.getSource("fd-obst") as GeoJSONSource | undefined)?.setData(obstData);
  });

  // Leaving the Map tab (or opening Photos): tools and cards put away.
  $effect(() => {
    if (planner && !bare) return;
    untrack(() => {
      if (measuring) setMeasuring(false);
      mPts = [];
      ctx = null;
      if (bare) {
        openAirport = null;
        wxPick = null;
        netPick = null;
      }
    });
  });

  // US airspace: read once when the layer is first switched on (FlightDeck
  // keeps a copy for the FAA's 28-day cycle).
  let aspData: GeoJSON.FeatureCollection | null = null;
  let aspState = $state<"idle" | "loading" | "ok" | "error">("idle");
  $effect(() => {
    const on = layers.has("airspace");
    if (!loaded || !map || !map.getSource("us-airspace")) return;
    for (const id of ["asp-fill", "asp-line", "asp-dash", "asp-label"]) if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", on ? "visible" : "none");
    if (!on) return;
    if (aspData) {
      (map.getSource("us-airspace") as GeoJSONSource).setData(aspData);
      return;
    }
    if (aspState === "loading") return;
    untrack(() => (aspState = "loading"));
    api.usAirspace()
      .then((text) => {
        aspData = JSON.parse(text) as GeoJSON.FeatureCollection;
        aspState = "ok";
        (map?.getSource("us-airspace") as GeoJSONSource | undefined)?.setData(aspData);
      })
      .catch(() => (aspState = "error"));
  });

  // Weather from the Aviation Weather Center: only while its layers are on,
  // every 10 minutes (FlightDeck's own cache has the same age).
  $effect(() => {
    const cat = layers.has("wx-cat");
    const adv = layers.has("wx-adv");
    if (!loaded || !map || !map.getSource("wx-cat")) return;
    const set = (id: string, fc: GeoJSON.FeatureCollection) => (map?.getSource(id) as GeoJSONSource | undefined)?.setData(fc);
    const none: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
    if (!cat) set("wx-cat", none);
    if (!adv) set("wx-adv", none);
    if (!cat && !adv) return;
    let stop = false;
    const load = () => {
      if (document.hidden) return;
      if (cat)
        api.wxCategories().then((d) => {
          if (stop || !d) return;
          set("wx-cat", {
            type: "FeatureCollection",
            features: d.stations.map(([id, lon, lat, c]) => ({ type: "Feature", properties: { id, c }, geometry: { type: "Point", coordinates: [lon, lat] } })),
          });
        });
      if (adv)
        api.wxAdvisories().then((d) => {
          if (!stop && d) set("wx-adv", { type: "FeatureCollection", features: d.features });
        });
    };
    load();
    const t = setInterval(load, 10 * 60_000);
    return () => {
      stop = true;
      clearInterval(t);
    };
  });

  /** The airport whose approach lines are drawn ("" none). */
  let approachShown = "";

  // --- Screenshots: a small camera where each was taken ----------------------
  let shotMarkers: maplibregl.Marker[] = [];
  let shotEls = new globalThis.Map<number, HTMLElement>();
  /** The Photos tab fits its pins once each time it opens. */
  let bareFitted = false;
  $effect(() => {
    if (!bare) bareFitted = false;
  });
  $effect(() => {
    const list = shotList.filter((s) => s.lon != null && s.lat != null);
    if (!loaded || !map) return;
    for (const m of shotMarkers) m.remove();
    shotEls = new globalThis.Map();
    if (bare && list.length && !untrack(() => bareFitted)) {
      bareFitted = true;
      const b = new maplibregl.LngLatBounds();
      for (const s of list) b.extend([s.lon!, s.lat!]);
      map.fitBounds(b, { padding: { top: 80, bottom: 60, left: 60 + insetLeft, right: 60 + insetRight }, maxZoom: 9, duration: 0 });
    }
    shotMarkers = list.map((s) => {
      const el = document.createElement("button");
      el.className = "shot-pin";
      el.title = "Screenshot";
      el.setAttribute("aria-label", "Screenshot taken here");
      el.innerHTML =
        '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 8h3l2-2.5h6L17 8h3v11H4z"/><circle cx="12" cy="13" r="3.4"/></svg>';
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        shotStore.viewer = { list: shotList, index: shotList.indexOf(s) };
      });
      shotEls.set(s.id, el);
      el.addEventListener("mouseenter", () => {
        el.classList.add("hot");
        onshothover?.(s.id);
      });
      el.addEventListener("mouseleave", () => {
        el.classList.toggle("hot", shotAccent === s.id && !onshothover);
        onshothover?.(null);
      });
      return new maplibregl.Marker({ element: el, anchor: "bottom" }).setLngLat([s.lon!, s.lat!]).addTo(map!);
    });
    return () => {
      for (const m of shotMarkers) m.remove();
      shotMarkers = [];
    };
  });

  // The pin of the thumbnail under the pointer stands out, and on top.
  $effect(() => {
    const id = shotAccent;
    void shotList;
    for (const [sid, el] of shotEls) {
      el.classList.toggle("hot", sid === id);
      el.style.zIndex = sid === id ? "5" : "";
    }
  });

  // --- Wind -----------------------------------------------------------------

  /** Wind level chosen: "auto" follows the aircraft, else a level id. */
  const windPref = $derived(prefs.s.map_wind_level ?? "auto");
  const flyingAlt = $derived(live.length ? live[live.length - 1][2] : null);
  const windLevel = $derived(windPref === "auto" ? levelFor(flyingAlt ?? 0) : windPref);
  let windField = $state<Field | null>(null);
  let windError = $state(false);
  let particles: WindParticles | null = null;

  // Forecast wind at the level shown: only while the layer is on, again
  // every 30 minutes (the forecast moves on every 3 hours).
  $effect(() => {
    const on = layers.has("wind");
    const level = windLevel;
    if (!loaded || !map) return;
    if (!on) {
      particles?.stop();
      windField = null;
      return;
    }
    particles ??= new WindParticles(map);
    particles.start();
    let stop = false;
    const load = () => {
      if (document.hidden) return;
      api.windGrid(level).then(
        (g) => {
          if (stop) return;
          const f = g ? decodeGrid(g) : null;
          windError = !f;
          if (f || !windField) {
            windField = f;
            particles?.setField(f);
          }
        },
        () => {
          if (!stop) windError = true;
        },
      );
    };
    load();
    const t = setInterval(load, 30 * 60_000);
    return () => {
      stop = true;
      clearInterval(t);
    };
  });
  onDestroy(() => particles?.destroy());

  /** Wind under the mouse, for the wind key. */
  let windAt = $state<string | null>(null);
  function windUnder(e: maplibregl.MapMouseEvent) {
    const f = windField;
    if (!f) {
      windAt = null;
      return;
    }
    const [dir, kt] = dirSpeed(sample(f, e.lngLat.lng, e.lngLat.lat));
    windAt = kt < 1 ? "Calm" : `${String(dir).padStart(3, "0")}° ${Math.round(kt)} kt`;
  }
  const windKey = $derived(
    `linear-gradient(90deg, ${WIND_STOPS.map(([kt, c]) => `${c} ${Math.round((kt / 140) * 100)}%`).join(", ")})`,
  );
  const zulu = (ms: number) => `${String(new Date(ms).getUTCHours()).padStart(2, "0")}Z`;

  /** An airport zoomed to from its card: its wind is drawn big. */
  let windFocus = $state<{ ident: string; lon: number; lat: number } | null>(null);
  let focusWx = $state<{ station: string; dir: number | null; kt: number; gust: number; nm: number } | null>(null);
  $effect(() => {
    const f = windFocus;
    focusWx = null;
    if (!f) return;
    api.airportWeather(f.ident).then((w) => {
      if (windFocus !== f || !w || w.wind_kt == null) return;
      focusWx = { station: w.station, dir: w.wind_dir, kt: w.wind_kt, gust: w.gust_kt ?? 0, nm: w.distance_nm };
    });
  });


  // --- Flight categories for the airport tip -------------------------------
  // Known ahead (every 10 minutes, from FlightDeck's own METAR cache), so the
  // tip shows VFR or IFR at once; an airport not in it shows nothing.
  let cats = new globalThis.Map<string, number>();
  function loadCats() {
    if (document.hidden) return;
    api.wxCategories().then((d) => {
      if (d?.stations?.length) cats = new globalThis.Map(d.stations.map(([id, , , c]) => [id, c]));
    }).catch(() => {});
  }

  // --- Right-click on an airport ------------------------------------------
  let ctx = $state<{ x: number; y: number; ident: string; name: string; at: [number, number] } | null>(null);
  async function sendToPlanner(which: "origin" | "dest") {
    if (!ctx || pick.locked) return;
    const c = ctx;
    ctx = null;
    // The full name and the airport's own position, as the planner's search gives them.
    const hit = ((await api.searchAirports(c.ident).catch(() => [])) ?? []).find((h) => h.icao === c.ident);
    pick.which = which;
    pick.spot = hit ? { icao: hit.icao, name: hit.name, lat: hit.lat, lon: hit.lon } : { icao: c.ident, name: c.name, lat: c.at[1], lon: c.at[0] };
  }
  /** The whole airport in view: a box around it as long as its longest
   * runway, clear of the cards over the map. */
  async function fitAirport(ident: string, at: [number, number]) {
    ctx = null;
    if (!map) return;
    const card = await api.airportCard(ident).catch(() => null);
    const longest = Math.max(0, ...(card?.runways ?? []).map((r) => r.length_ft)) * 0.3048;
    const r = Math.max(longest * 0.75, 700);
    const lat = card?.lat ?? at[1];
    const lon = card?.lon ?? at[0];
    const dLat = r / 111_320;
    const dLon = r / (111_320 * Math.max(0.2, Math.cos((lat * Math.PI) / 180)));
    const right = (openAirport || netPick || wxPick ? 372 : 0) + 40;
    map.fitBounds(
      [
        [lon - dLon, lat - dLat],
        [lon + dLon, lat + dLat],
      ],
      { padding: { top: 70, bottom: 40 + insetBottom, left: 40 + insetLeft, right: Math.max(right, 40 + insetRight) }, maxZoom: 16.5, duration: 1200 },
    );
  }

  // --- The airport clicked: a pulsing ring around it ----------------------
  let ringAt = $state<[number, number] | null>(null);
  let ringMarker: maplibregl.Marker | undefined;
  /** Zoomed in this far, the airport fills the view and needs no ring. */
  const RING_HIDE = 12.5;
  let zoomNow = $state(2);
  /** The airport the ring was placed for. */
  let ringFor: string | null = null;
  $effect(() => {
    const at = openAirport ? ringAt : null;
    if (!map || !loaded) return;
    if (!at) {
      ringMarker?.remove();
      ringMarker = undefined;
      return;
    }
    if (!ringMarker) {
      const d = document.createElement("div");
      // The map sets the marker's own opacity, so the glow and its hiding
      // live on an element inside it.
      d.className = "ap-ring";
      d.innerHTML = "<span></span>";
      ringMarker = new maplibregl.Marker({ element: d });
    }
    ringMarker.setLngLat(at).addTo(map);
  });
  $effect(() => {
    const hide = zoomNow >= RING_HIDE;
    void ringAt;
    void openAirport;
    ringMarker?.getElement().firstElementChild?.classList.toggle("hide", hide);
  });
  // A card opened from elsewhere (an online controller's airport): find
  // where the airport is.
  $effect(() => {
    const id = openAirport;
    if (!id) return;
    untrack(() => {
      if (ringFor === id) return;
      ringAt = null;
      api.searchAirports(id).then((hits) => {
        const h = hits?.find((x) => x.icao === id);
        if (h && openAirport === id && ringFor !== id) {
          ringFor = id;
          ringAt = [h.lon, h.lat];
        }
      }).catch(() => {});
    });
  });

  // --- Measuring ----------------------------------------------------------
  let measuring = $state(false);
  let mPts = $state<[number, number][]>([]);
  /** Draw route (from Start a flight) uses the measuring line: the origin
   * and the points clicked so far. */
  const drawOn = $derived(planner && (drawing.stage === "origin" || drawing.stage === "route"));
  const drawRoute = $derived(planner && drawing.stage === "route");
  /** Points clicked on the map are being traced (measuring or drawing). */
  const tracing = $derived(measuring || drawRoute);
  const linePts = $derived<[number, number][]>(
    drawOn ? (drawing.origin ? [[drawing.origin.lon, drawing.origin.lat], ...drawing.pts.map((p) => [p.lon, p.lat] as [number, number])] : []) : mPts,
  );
  function undoPoint() {
    if (drawOn) drawing.pts = drawing.pts.slice(0, -1);
    else mPts = mPts.slice(0, -1);
  }
  // Drawing: a crosshair while points are being added.
  $effect(() => {
    const on = drawRoute;
    if (!map || measuring) return;
    map.getCanvas().style.cursor = on ? "crosshair" : "";
    if (on) map.doubleClickZoom.disable();
    else map.doubleClickZoom.enable();
    if (on) {
      ctx = null;
      hoverTip = null;
    } else untrack(() => (mHover = null));
  });
  /** A click while drawing: the origin airport first, then the route. */
  async function drawClick(pt: maplibregl.Point, at: [number, number]) {
    const hit = airportHit(pt);
    const named = async (ident: string, where: [number, number]) => {
      const h = ((await api.searchAirports(ident).catch(() => [])) ?? []).find((x) => x.icao === ident);
      return h ? { icao: h.icao, name: h.name, lat: h.lat, lon: h.lon } : { icao: ident, name: "", lat: where[1], lon: where[0] };
    };
    if (drawing.stage === "origin") {
      if (!hit) {
        drawing.note = "Click an airport to start from.";
        return;
      }
      drawing.note = "";
      drawing.origin = await named(hit.ident, hit.at);
      return;
    }
    if (hit) {
      drawing.pts = [...drawing.pts, await named(hit.ident, hit.at)];
    } else {
      drawing.pts = [...drawing.pts, { icao: "", name: "", lat: at[1], lon: at[0] }];
    }
    drawing.note = "";
  }
  let mHover = $state<[number, number] | null>(null);
  let legMarkers: maplibregl.Marker[] = [];
  const R_NM = 3440.065;
  function gcNm(a: [number, number], b: [number, number]) {
    const rad = Math.PI / 180;
    const dLat = (b[1] - a[1]) * rad;
    const dLon = (b[0] - a[0]) * rad;
    const h = Math.sin(dLat / 2) ** 2 + Math.cos(a[1] * rad) * Math.cos(b[1] * rad) * Math.sin(dLon / 2) ** 2;
    return 2 * R_NM * Math.asin(Math.min(1, Math.sqrt(h)));
  }
  /** Initial true course from a to b, in degrees. */
  function course(a: [number, number], b: [number, number]) {
    const rad = Math.PI / 180;
    const y = Math.sin((b[0] - a[0]) * rad) * Math.cos(b[1] * rad);
    const x = Math.cos(a[1] * rad) * Math.sin(b[1] * rad) - Math.sin(a[1] * rad) * Math.cos(b[1] * rad) * Math.cos((b[0] - a[0]) * rad);
    return ((Math.atan2(y, x) / rad + 360) % 360);
  }
  const legs = $derived(linePts.slice(1).map((b, i) => ({ nm: gcNm(linePts[i], b), crs: course(linePts[i], b) })));
  const mTotal = $derived(legs.reduce((t, l) => t + l.nm, 0));
  const mNext = $derived.by(() => {
    const pts = linePts;
    if (!tracing || !mHover || !pts.length) return null;
    const nm = gcNm(pts[pts.length - 1], mHover);
    return nm < 0.05 ? null : { nm, crs: course(pts[pts.length - 1], mHover) };
  });
  const fmt = (nm: number) => (nm < 10 ? nm.toFixed(1) : Math.round(nm).toLocaleString("en-US"));
  const crs3 = (c: number) => String(Math.round(c) % 360 || 360).padStart(3, "0");
  function measureData(): GeoJSON.FeatureCollection {
    const features: GeoJSON.Feature[] = [];
    const mPts = linePts;
    if (mPts.length > 1) {
      const line: [number, number][] = [mPts[0]];
      for (let i = 1; i < mPts.length; i++) line.push(...greatCircle(line[line.length - 1], mPts[i], 32).slice(1));
      features.push({ type: "Feature", properties: { kind: "line" }, geometry: { type: "LineString", coordinates: line } });
    }
    if (tracing && mHover && mPts.length && mNext) {
      features.push({ type: "Feature", properties: { kind: "next" }, geometry: { type: "LineString", coordinates: greatCircle(mPts[mPts.length - 1], mHover, 24) } });
    }
    mPts.forEach((p, i) => features.push({ type: "Feature", properties: { kind: "pt", first: i === 0 }, geometry: { type: "Point", coordinates: p } }));
    return { type: "FeatureCollection", features };
  }
  function addMeasureLayers() {
    if (!map || map.getSource("fd-measure")) return;
    map.addSource("fd-measure", { type: "geojson", data: measureData() });
    map.addLayer({ id: "fd-measure-casing", type: "line", source: "fd-measure", filter: ["==", ["get", "kind"], "line"], layout: { "line-cap": "round", "line-join": "round" }, paint: { "line-color": "#111214", "line-width": 5, "line-opacity": 0.7 } });
    map.addLayer({ id: "fd-measure-line", type: "line", source: "fd-measure", filter: ["==", ["get", "kind"], "line"], layout: { "line-cap": "round", "line-join": "round" }, paint: { "line-color": "#f0b23a", "line-width": 2.5 } });
    map.addLayer({ id: "fd-measure-next", type: "line", source: "fd-measure", filter: ["==", ["get", "kind"], "next"], paint: { "line-color": "#f0b23a", "line-width": 2, "line-dasharray": [2, 2], "line-opacity": 0.8 } });
    map.addLayer({
      id: "fd-measure-pts",
      type: "circle",
      source: "fd-measure",
      filter: ["==", ["get", "kind"], "pt"],
      paint: { "circle-radius": 5, "circle-color": ["case", ["get", "first"], "#f0b23a", "#111214"], "circle-stroke-color": "#f0b23a", "circle-stroke-width": 2 },
    });
  }
  // Redraw the line, and a label on each leg.
  $effect(() => {
    const data = measureData();
    if (!map || !loaded) return;
    (map.getSource("fd-measure") as GeoJSONSource | undefined)?.setData(data);
  });
  $effect(() => {
    const ls = legs;
    const pts = linePts;
    if (!map || !loaded) return;
    for (const m of legMarkers) m.remove();
    legMarkers = ls.flatMap((l, i) => {
      if (l.nm < 0.05) return [];
      const mid = greatCircle(pts[i], pts[i + 1], 2)[1];
      const d = document.createElement("div");
      d.className = "leg-label";
      d.textContent = `${fmt(l.nm)} nm · ${crs3(l.crs)}°`;
      return [new maplibregl.Marker({ element: d }).setLngLat(mid).addTo(map!)];
    });
  });
  function setMeasuring(on: boolean) {
    measuring = on;
    mHover = null;
    if (on) {
      ctx = null;
      hoverTip = null;
    }
    if (!map) return;
    map.getCanvas().style.cursor = on ? "crosshair" : "";
    if (on) map.doubleClickZoom.disable();
    else map.doubleClickZoom.enable();
  }
  function clearMeasure() {
    mPts = [];
    mHover = null;
  }
  function measureKey(e: KeyboardEvent) {
    if (ctx && e.key === "Escape") ctx = null;
    if (!tracing) return;
    const t = e.target as HTMLElement | null;
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
    if (e.key === "Escape" && measuring) setMeasuring(false);
    else if (e.key === "Backspace" || (e.key === "z" && (e.ctrlKey || e.metaKey))) {
      e.preventDefault();
      undoPoint();
    }
  }

  /** "Zoom to" in an airport's card. */
  function zoomToAirport(a: { ident: string; lon: number; lat: number }) {
    if (!map) return;
    windFocus = { ...a };
    const covered = Math.min(372, el.clientWidth / 2);
    map.flyTo({ center: [a.lon, a.lat], zoom: 13, offset: [-covered / 2, 0], duration: 1400 });
  }

  // Airport wind arrows: METAR winds while the layer is on, every 10 minutes.
  $effect(() => {
    const on = layers.has("wx-wind");
    if (!loaded || !map || !map.getSource("wx-wind")) return;
    map.setLayoutProperty("wx-wind-arrows", "visibility", on ? "visible" : "none");
    map.setLayoutProperty("wx-wind-focus", "visibility", on ? "visible" : "none");
    if (!on) return;
    let stop = false;
    const load = () => {
      if (document.hidden) return;
      api.wxCategories().then((d) => {
        if (stop || !d) return;
        (map?.getSource("wx-wind") as GeoJSONSource | undefined)?.setData({
          type: "FeatureCollection",
          features: (d.winds ?? []).map(([id, lon, lat, dir, kt, gust]) => ({
            type: "Feature",
            properties: { id, dir, kt, gust, rot: dir < 0 ? 0 : (dir + 180) % 360, text: windText(dir, kt, gust) },
            geometry: { type: "Point", coordinates: [lon, lat] },
          })),
        });
      });
    };
    load();
    const t = setInterval(load, 10 * 60_000);
    return () => {
      stop = true;
      clearInterval(t);
    };
  });

  // The zoomed-to airport's own arrow, and its station left out of the rest.
  $effect(() => {
    const f = windFocus;
    const w = focusWx;
    if (!loaded || !map || !map.getSource("wx-wind-focus")) return;
    const features: GeoJSON.Feature[] =
      f && w
        ? [
            {
              type: "Feature",
              properties: {
                dir: w.dir ?? -1,
                kt: w.kt,
                rot: w.dir == null ? 0 : (w.dir + 180) % 360,
                text: `${f.ident}  ${windText(w.dir, w.kt, w.gust)}${w.nm > 0 ? `\n(${w.station}, ${w.nm} nm away)` : ""}`,
              },
              geometry: { type: "Point", coordinates: [f.lon, f.lat] },
            },
          ]
        : [];
    (map.getSource("wx-wind-focus") as GeoJSONSource).setData({ type: "FeatureCollection", features });
    map.setFilter("wx-wind-arrows", f && w ? ["!=", ["get", "id"], w.station] : null);
  });

  /** What is open from an online network's layers. */
  let netPick = $state<NetPick | null>(null);

  // Online networks: download only while one of their layers is on, every
  // 30 seconds (the networks refresh every 15), and not while hidden.
  let netData = $state<Partial<Record<Net, NetworkData>>>({});
  const netOn = (net: Net) => layers.has(`${net}-atc`) || layers.has(`${net}-pilots`);
  $effect(() => {
    const nets = (["vatsim", "ivao"] as Net[]).filter(netOn);
    if (!nets.length) return;
    let stop = false;
    const load = () => {
      if (document.hidden) return;
      for (const n of nets) {
        api.onlineNetwork(n).then((d) => {
          if (!stop && d) netData = { ...netData, [n]: d };
        });
      }
    };
    load();
    const t = setInterval(load, 30_000);
    return () => {
      stop = true;
      clearInterval(t);
    };
  });
  $effect(() => {
    const data = netData;
    const on = layers;
    if (!loaded || !map || !map.getSource("net-areas")) return;
    const areas: GeoJSON.Feature[] = [];
    const labels: GeoJSON.Feature[] = [];
    const airports: GeoJSON.Feature[] = [];
    const pilots: GeoJSON.Feature[] = [];
    for (const net of ["vatsim", "ivao"] as Net[]) {
      const d = data[net];
      if (!d) continue;
      if (on.has(`${net}-atc`)) {
        d.areas.forEach((a, i) => {
          const props = { net, i, kind: "area" };
          areas.push({ type: "Feature", properties: props, geometry: { type: "MultiPolygon", coordinates: a.polys } });
          labels.push({
            type: "Feature",
            properties: { ...props, text: a.controllers.map((c) => c.callsign).join("\n") },
            geometry: { type: "Point", coordinates: a.label },
          });
        });
        d.airports.forEach((ap, i) => {
          const kinds = new Set(ap.controllers.map((c) => c.kind));
          const letters = ["DEL", "GND", "TWR", "ATIS"].filter((k) => kinds.has(k as never)).map((k) => POSITION[k].letter);
          airports.push({
            type: "Feature",
            properties: { net, i, kind: "airport", app: kinds.has("APP") || kinds.has("DEP"), badge: `${ap.icao}${letters.length ? ` ${letters.join(" ")}` : ""}` },
            geometry: { type: "Point", coordinates: [ap.lon, ap.lat] },
          });
        });
      }
      if (on.has(`${net}-pilots`)) {
        d.pilots.forEach((p, i) => {
          pilots.push({ type: "Feature", properties: { net, i, kind: "pilot", hdg: p.hdg, callsign: p.callsign }, geometry: { type: "Point", coordinates: [p.lon, p.lat] } });
        });
      }
    }
    const set = (id: string, features: GeoJSON.Feature[]) =>
      (map!.getSource(id) as GeoJSONSource | undefined)?.setData({ type: "FeatureCollection", features });
    set("net-areas", areas);
    set("net-labels", labels);
    set("net-airports", airports);
    set("net-pilots", pilots);
  });
  // A picked pilot: a line from their departure airport to where they are
  // now, kept up to date as positions come in.
  let pickedDep = $state<{ net: Net; callsign: string; at: [number, number] | null } | null>(null);
  $effect(() => {
    const p = netPick;
    pickedDep = null;
    if (p?.kind !== "pilot") return;
    const { net } = p;
    const callsign = p.pilot.callsign;
    api.onlinePilot(net, callsign).then((d) => {
      if (netPick?.kind === "pilot" && netPick.pilot.callsign === callsign) pickedDep = { net, callsign, at: d?.dep_pos ?? null };
    });
  });
  $effect(() => {
    const dep = pickedDep;
    const data = netData;
    if (!loaded || !map) return;
    const src = map.getSource("net-track") as GeoJSONSource | undefined;
    if (!src) return;
    const now = dep?.at ? data[dep.net]?.pilots.find((x) => x.callsign === dep.callsign) : undefined;
    src.setData(
      dep?.at && now
        ? {
            type: "FeatureCollection",
            features: [{ type: "Feature", properties: { net: dep.net }, geometry: { type: "LineString", coordinates: greatCircle(dep.at, [now.lon, now.lat]) } }],
          }
        : { type: "FeatureCollection", features: [] },
    );
  });
  /** The online-network thing at a point on the map, if any. */
  const NET_LAYERS = ["net-pilots", "net-airport-dot", "net-airport-labels", "net-area-labels", "net-app-ring", "net-area-fill"];
  function netAt(pt: maplibregl.PointLike): NetPick | null {
    if (!map) return null;
    const p = pt as maplibregl.Point;
    const box: [maplibregl.PointLike, maplibregl.PointLike] = [[p.x - 4, p.y - 4], [p.x + 4, p.y + 4]];
    // Pilots and airports first, then the area under them.
    for (const layer of NET_LAYERS) {
      if (!map.getLayer(layer)) continue;
      // An area opens from its label; the whole area only when there are no labels.
      if (layer === "net-area-fill" && map.getLayer("net-area-labels")) continue;
      const f = map.queryRenderedFeatures(layer === "net-area-fill" ? [p.x, p.y] as maplibregl.PointLike : box, { layers: [layer] })[0];
      if (!f) continue;
      const { net, i, kind } = f.properties as { net: Net; i: number; kind: string };
      const d = netData[net];
      if (!d) continue;
      if (kind === "pilot" && d.pilots[i]) return { kind: "pilot", net, pilot: d.pilots[i] };
      if (kind === "airport" && d.airports[i]) return { kind: "airport", net, airport: d.airports[i] };
      if (kind === "area" && d.areas[i]) return { kind: "area", net, area: d.areas[i] };
    }
    return null;
  }
  /** Dots and their codes: either opens the airport. */
  const AIRPORT_LAYERS = ["wa-major", "wa-regional", "wa-small", "wa-labels-0", "wa-labels-1", "wa-labels-2", "routes-airports", "wx-cat"];
  /** The airport (code) at a point on the map, if any. */
  function airportAt(pt: maplibregl.PointLike): string | null {
    return airportHit(pt)?.ident ?? null;
  }
  /** The airport at a point on the map: its code and where it is. */
  function airportHit(pt: maplibregl.PointLike): { ident: string; at: [number, number] } | null {
    if (!map) return null;
    const layers = AIRPORT_LAYERS.filter((l) => map!.getLayer(l) && map!.getLayoutProperty(l, "visibility") !== "none");
    if (!layers.length) return null;
    const p = pt as maplibregl.Point;
    const box: [maplibregl.PointLike, maplibregl.PointLike] = [[p.x - 5, p.y - 5], [p.x + 5, p.y + 5]];
    for (const f of map.queryRenderedFeatures(box, { layers })) {
      const props = f.properties as { id?: string; ident?: string };
      const ident = props.id ?? props.ident;
      if (ident && f.geometry.type === "Point") return { ident, at: f.geometry.coordinates as [number, number] };
    }
    return null;
  }
  /** World airports detail: 1 major, 2 + regional, 3 + small airports and strips. */
  const airportDetail = $derived(Math.min(3, Math.max(1, Number(prefs.s.map_airport_detail ?? "1") || 1)));
  const worldCache = new globalThis.Map<number, GeoJSON.FeatureCollection>();
  async function worldAirports(detail: number): Promise<GeoJSON.FeatureCollection> {
    const hit = worldCache.get(detail);
    if (hit) return hit;
    const list: MapAirport[] = (await api.mapAirports(detail)) ?? [];
    const fc: GeoJSON.FeatureCollection = {
      type: "FeatureCollection",
      features: list.map(([id, name, r, lon, lat]) => ({ type: "Feature", properties: { id, name, r }, geometry: { type: "Point", coordinates: [lon, lat] } })),
    };
    worldCache.set(detail, fc);
    return fc;
  }
  $effect(() => {
    const on = layers.has("world");
    const detail = airportDetail;
    if (!loaded || !map || !map.getSource("world-airports")) return;
    for (const id of ["wa-major", "wa-regional", "wa-small", "wa-labels-0", "wa-labels-1", "wa-labels-2"]) {
      if (map.getLayer(id)) map.setLayoutProperty(id, "visibility", on ? "visible" : "none");
    }
    if (!on) return;
    worldAirports(detail).then((fc) => {
      if (detail === untrack(() => airportDetail)) (map?.getSource("world-airports") as GeoJSONSource | undefined)?.setData(fc);
    });
  });
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
    const airports = new globalThis.Map<string, { at: [number, number]; ident: string }>();
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
      airports.set(`${r.from[0].toFixed(3)},${r.from[1].toFixed(3)}`, { at: r.from, ident: r.dep_icao ?? "" });
      airports.set(`${r.to[0].toFixed(3)},${r.to[1].toFixed(3)}`, { at: r.to, ident: r.arr_icao ?? "" });
    }
    let k = 0;
    for (const { at, ident } of airports.values()) {
      features.push({ type: "Feature", id: -(++k), properties: { kind: "airport", ident }, geometry: { type: "Point", coordinates: at } });
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
    if (!map) return;
    tintMap(map);
    addRegionLines(map);
  }

  function addOverlays() {
    if (!map) return;
    addAirportLayout(map);
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
    // SIGMETs and AIRMETs, under airports and routes.
    const wxEmpty: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
    map.addSource("wx-adv", { type: "geojson", data: wxEmpty });
    map.addSource("wx-cat", { type: "geojson", data: wxEmpty });
    const wxColor: maplibregl.ExpressionSpecification = [
      "match",
      ["get", "group"],
      ...Object.entries(WX_GROUP).flatMap(([k, v]) => [k, v.color]),
      WX_GROUP.OTHER.color,
    ] as unknown as maplibregl.ExpressionSpecification;
    map.addLayer({ id: "wx-adv-fill", type: "fill", source: "wx-adv", paint: { "fill-color": wxColor, "fill-opacity": 0.1 } });
    map.addLayer({
      id: "wx-adv-line",
      type: "line",
      source: "wx-adv",
      filter: ["!=", ["get", "kind"], "AIRMET"],
      paint: { "line-color": wxColor, "line-width": 1.6, "line-opacity": 0.9 },
    });
    // AIRMETs (less severe) dashed.
    map.addLayer({
      id: "wx-adv-dash",
      type: "line",
      source: "wx-adv",
      filter: ["==", ["get", "kind"], "AIRMET"],
      paint: { "line-color": wxColor, "line-width": 1.3, "line-opacity": 0.85, "line-dasharray": [3, 2] },
    });
    // US airspace (Class B, C and D), as charts draw it: B solid blue, C
    // magenta, D dashed blue, with ceiling/floor along the edges.
    map.addSource("us-airspace", { type: "geojson", data: wxEmpty });
    const ASP: maplibregl.ExpressionSpecification = ["match", ["get", "c"], "C", "#c25bb0", "#4a8ce8"] as unknown as maplibregl.ExpressionSpecification;
    map.addLayer({ id: "asp-fill", type: "fill", source: "us-airspace", minzoom: 5, layout: { visibility: "none" }, paint: { "fill-color": ASP, "fill-opacity": ["match", ["get", "c"], "D", 0.03, 0.05] } });
    map.addLayer({
      id: "asp-line",
      type: "line",
      source: "us-airspace",
      minzoom: 5,
      filter: ["!=", ["get", "c"], "D"],
      layout: { visibility: "none", "line-join": "round" },
      paint: { "line-color": ASP, "line-width": ["interpolate", ["linear"], ["zoom"], 5, 1, 10, ["match", ["get", "c"], "B", 2.4, 1.8]], "line-opacity": 0.9 },
    });
    map.addLayer({
      id: "asp-dash",
      type: "line",
      source: "us-airspace",
      minzoom: 7,
      filter: ["==", ["get", "c"], "D"],
      layout: { visibility: "none" },
      paint: { "line-color": "#4a8ce8", "line-width": 1.4, "line-dasharray": [3, 2], "line-opacity": 0.9 },
    });
    if (map.getStyle().glyphs)
      map.addLayer({
        id: "asp-label",
        type: "symbol",
        source: "us-airspace",
        minzoom: 8,
        layout: {
          visibility: "none",
          "symbol-placement": "line",
          "symbol-spacing": 320,
          "text-field": ["get", "label"],
          "text-font": ["Noto Sans Bold"],
          "text-size": 11,
          "text-keep-upright": true,
        },
        paint: { "text-color": ASP, "text-halo-color": "#111214", "text-halo-width": 1.6 },
      });
    // The world's airports (the Airports layer, by detail level), under
    // your routes. Smaller airports appear as you zoom in.
    map.addSource("world-airports", { type: "geojson", data: { type: "FeatureCollection", features: [] } });
    const WORLD: [string, number, string, number[]][] = [
      // [layer, rank, color, [radius at zoom 3, at zoom 10]]
      ["wa-small", 2, "#a98be0", [1.6, 3.2]],
      ["wa-regional", 1, "#5cc08a", [2, 4]],
      ["wa-major", 0, "#4f9cf0", [3.1, 6]],
    ];
    const MINZOOM = [0, 4.5, 7];
    for (const [id, rank, color, [r0, r1]] of WORLD) {
      map.addLayer({
        id,
        type: "circle",
        source: "world-airports",
        minzoom: MINZOOM[rank],
        filter: ["==", ["get", "r"], rank],
        layout: { visibility: "none" },
        paint: {
          "circle-radius": ["interpolate", ["linear"], ["zoom"], 3, r0, 10, r1],
          "circle-color": color,
          "circle-stroke-color": "#111214",
          "circle-stroke-width": 1,
        },
      });
    }
    // Codes beside the dots, once zoomed in (needs the online style's fonts).
    if (map.getStyle().glyphs) {
      for (const [rank, minzoom] of [[0, 5], [1, 7], [2, 9]]) {
        map.addLayer({
          id: `wa-labels-${rank}`,
          type: "symbol",
          source: "world-airports",
          minzoom,
          filter: ["==", ["get", "r"], rank],
          layout: {
            visibility: "none",
            "text-field": ["get", "id"],
            "text-font": ["Noto Sans Regular"],
            "text-size": 11,
            "text-offset": [0.8, 0],
            "text-anchor": "left",
            "text-optional": true,
          },
          paint: { "text-color": ["#8fc0f7", "#8fd6ad", "#c6b2ec"][rank], "text-halo-color": "#111214", "text-halo-width": 1.2 },
        });
      }
    }
    // Online networks' controlled areas and approach rings, under your routes.
    const empty: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
    const netColor: maplibregl.ExpressionSpecification = ["match", ["get", "net"], "ivao", NET_COLOR.ivao, NET_COLOR.vatsim];
    map.addSource("net-areas", { type: "geojson", data: empty });
    map.addSource("net-airports", { type: "geojson", data: empty });
    map.addSource("net-pilots", { type: "geojson", data: empty });
    map.addSource("net-labels", { type: "geojson", data: empty });
    map.addSource("net-track", { type: "geojson", data: empty });
    map.addLayer({ id: "net-area-fill", type: "fill", source: "net-areas", paint: { "fill-color": netColor, "fill-opacity": 0.07 } });
    map.addLayer({
      id: "net-area-line",
      type: "line",
      source: "net-areas",
      paint: { "line-color": netColor, "line-opacity": 0.75, "line-width": 1.2 },
    });
    // Approach: a ring of about 30 nm around the airport, whatever the zoom.
    map.addLayer({
      id: "net-app-ring",
      type: "circle",
      source: "net-airports",
      filter: ["==", ["get", "app"], true],
      paint: {
        "circle-radius": ["interpolate", ["exponential", 2], ["zoom"], 0, 0.71, 22, 0.71 * 2 ** 22],
        "circle-color": netColor,
        "circle-opacity": 0.06,
        "circle-stroke-color": netColor,
        "circle-stroke-opacity": 0.7,
        "circle-stroke-width": 1.2,
        "circle-pitch-alignment": "map",
      },
    });
    // Flight category: a coloured ring around each reporting airport.
    map.addLayer({
      id: "wx-cat",
      type: "circle",
      source: "wx-cat",
      paint: {
        "circle-radius": ["interpolate", ["linear"], ["zoom"], 2, 3, 6, 5, 10, 7],
        "circle-color": ["match", ["get", "c"], 0, CATEGORY[0].color, 1, CATEGORY[1].color, 2, CATEGORY[2].color, CATEGORY[3].color],
        "circle-opacity": 0.35,
        "circle-stroke-color": ["match", ["get", "c"], 0, CATEGORY[0].color, 1, CATEGORY[1].color, 2, CATEGORY[2].color, CATEGORY[3].color],
        "circle-stroke-width": ["interpolate", ["linear"], ["zoom"], 2, 1, 8, 2],
      },
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
    map.addSource("plan", { type: "geojson", data: planFeatures(untrack(() => shownPlan)) });
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
    // Soft, even light so the wall keeps its colors from every side.
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
    // Online networks: staffed airports, pilots and labels, above everything.
    if (!map.hasImage("net-plane")) map.addImage("net-plane", planeIcon(), { sdf: true });
    // The picked pilot's way so far: a great circle from where they departed.
    map.addLayer({
      id: "net-track",
      type: "line",
      source: "net-track",
      layout: { "line-cap": "round" },
      paint: { "line-color": netColor, "line-width": 2, "line-opacity": 0.85 },
    });
    map.addLayer({
      id: "net-airport-dot",
      type: "circle",
      source: "net-airports",
      paint: { "circle-radius": 4, "circle-color": netColor, "circle-stroke-color": "#111214", "circle-stroke-width": 1.2 },
    });
    map.addLayer({
      id: "net-pilots",
      type: "symbol",
      source: "net-pilots",
      layout: {
        "icon-image": "net-plane",
        "icon-size": ["interpolate", ["linear"], ["zoom"], 2, 0.42, 6, 0.6, 10, 0.8],
        "icon-rotate": ["get", "hdg"],
        "icon-rotation-alignment": "map",
        "icon-allow-overlap": true,
        "icon-ignore-placement": true,
      },
      paint: { "icon-color": netColor, "icon-halo-color": "#111214", "icon-halo-width": 1 },
    });
    if (map.getStyle().glyphs) {
      const font = ["Noto Sans Regular"];
      map.addLayer({
        id: "net-pilot-labels",
        type: "symbol",
        source: "net-pilots",
        minzoom: 7,
        layout: { "text-field": ["get", "callsign"], "text-font": font, "text-size": 10, "text-offset": [0, 1.3], "text-optional": true },
        paint: { "text-color": "#cfd5dc", "text-halo-color": "#111214", "text-halo-width": 1.2 },
      });
      map.addLayer({
        id: "net-area-labels",
        type: "symbol",
        source: "net-labels",
        layout: { "text-field": ["get", "text"], "text-font": font, "text-size": 11.5, "text-allow-overlap": false },
        paint: { "text-color": "#eef1f4", "text-halo-color": "#111214", "text-halo-width": 1.5 },
      });
      map.addLayer({
        id: "net-airport-labels",
        type: "symbol",
        source: "net-airports",
        minzoom: 4,
        layout: {
          "text-field": ["get", "badge"],
          "text-font": font,
          "text-size": 11,
          "text-anchor": "top",
          "text-offset": [0, 0.6],
          "text-allow-overlap": false,
        },
        paint: { "text-color": "#eef1f4", "text-halo-color": "#111214", "text-halo-width": 1.5 },
      });
    }
    addWindArrows();
  }

  /** Airport wind arrows: every METAR station's wind once zoomed in, and a
   * bigger one for an airport you zoomed to from its card. */
  function addWindArrows() {
    if (!map) return;
    addWindIcons(map);
    const empty: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
    map.addSource("wx-wind", { type: "geojson", data: empty });
    map.addSource("wx-wind-focus", { type: "geojson", data: empty });
    const glyphs = !!map.getStyle().glyphs;
    const font = ["Noto Sans Regular"];
    map.addLayer({
      id: "wx-wind-arrows",
      type: "symbol",
      source: "wx-wind",
      minzoom: 6,
      layout: {
        visibility: "none",
        "icon-image": windIconExpr() as maplibregl.ExpressionSpecification,
        "icon-size": ["interpolate", ["linear"], ["zoom"], 6, 0.55, 9, 0.8, 12, 1.1],
        "icon-rotate": ["get", "rot"],
        "icon-rotation-alignment": "map",
        "icon-allow-overlap": true,
        "icon-ignore-placement": true,
        ...(glyphs
          ? {
              "text-field": ["step", ["zoom"], "", 8, ["get", "text"]],
              "text-font": font,
              "text-size": 11,
              "text-offset": [0, 1.9],
              "text-optional": true,
            }
          : {}),
      } as maplibregl.SymbolLayerSpecification["layout"],
      paint: glyphs ? { "text-color": "#eef1f4", "text-halo-color": "#111214", "text-halo-width": 1.4 } : {},
    });
    map.addLayer({
      id: "wx-wind-focus",
      type: "symbol",
      source: "wx-wind-focus",
      layout: {
        visibility: "none",
        "icon-image": windIconExpr() as maplibregl.ExpressionSpecification,
        "icon-size": ["interpolate", ["linear"], ["zoom"], 6, 1, 10, 1.8, 13, 2.6],
        "icon-rotate": ["get", "rot"],
        "icon-rotation-alignment": "map",
        "icon-allow-overlap": true,
        "icon-ignore-placement": true,
        ...(glyphs
          ? {
              "text-field": ["get", "text"],
              "text-font": font,
              "text-size": 14,
              "text-anchor": "top",
              "text-offset": [0, 5.2],
              "text-allow-overlap": true,
            }
          : {}),
      } as maplibregl.SymbolLayerSpecification["layout"],
      paint: glyphs ? { "text-color": "#ffffff", "text-halo-color": "#111214", "text-halo-width": 2 } : {},
    });
  }

  /** A plane silhouette, drawn once as a tintable icon. */
  function planeIcon(): ImageData {
    const c = document.createElement("canvas");
    c.width = c.height = 32;
    const g = c.getContext("2d")!;
    g.translate(16, 16);
    g.scale(0.72, 0.72);
    g.fillStyle = "#fff";
    g.fill(new Path2D("M0 -20 L3.4 -6 L19 1.5 L19 5.5 L3.4 2.6 L2.4 13 L8 17 L8 19.6 L0 17.8 L-8 19.6 L-8 17 L-2.4 13 L-3.4 2.6 L-19 5.5 L-19 1.5 L-3.4 -6 Z"));
    return g.getImageData(0, 0, 32, 32);
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
    // For automated checks of the interface in development builds.
    if (import.meta.env.DEV) (window as unknown as { __fdMap?: maplibregl.Map }).__fdMap = map;
    map.addControl(new maplibregl.NavigationControl({ showCompass: true, visualizePitch: true }), "bottom-right");
    map.on("style.load", () => {
      tint();
      addOverlays();
      addMeasureLayers();
      loaded = true;
      if (!cats.size) loadCats();
      // The credits start folded to their (i) button: open, they run across the map.
      el.querySelector(".maplibregl-ctrl-attrib")?.classList.remove("maplibregl-compact-show");
    });
    map.on("error", (e) => {
      if (!loaded && String(e.error?.message ?? "").match(/fetch|network|load|style/i)) useOfflineStyle();
    });
    // Give the online style a few seconds before falling back.
    const fallback = setTimeout(useOfflineStyle, 8000);
    map.once("style.load", () => clearTimeout(fallback));
    map.on("dragstart", () => (follow = false));

    // Hover and click on past routes. The tip is our own element, always
    // in the page and only moved and shown: a map popup is created afresh
    // each time and could flash in the window's corner before it was placed.
    let hovered: number | null = null;
    const clearHover = () => {
      if (hovered != null) map?.setFeatureState({ source: "routes", id: hovered }, { hover: false });
      hovered = null;
      hoverTip = null;
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
        // An airport under the pointer has its own tip.
        if (tracing || airportAt(e.point)) return;
        hoverTip = { x: e.point.x, y: e.point.y, label: p.label, sub: p.sub, hint: "Click for flight details" };
      });
      map.on("mouseleave", layer, clearHover);
      map.on("click", layer, (e) => {
        const f = e.features?.[0];
        clearHover();
        // An airport or online pilot under the pointer wins over a route.
        if (tracing || drawOn || airportAt(e.point)) return;
        const n = netAt(e.point);
        if (n && n.kind !== "area") return;
        if (f?.id != null) onroute?.(Number(f.id));
      });
    }
    // Airports: a tip on hover, the airport card on click.
    for (const layer of AIRPORT_LAYERS) {
      map.on("mousemove", layer, (e) => {
        const f = e.features?.[0];
        const p = f?.properties as { id?: string; name?: string; ident?: string } | undefined;
        const ident = p?.id ?? p?.ident;
        if (!ident || !map || tracing) return;
        map.getCanvas().style.cursor = "pointer";
        hoverTip = { x: e.point.x, y: e.point.y, label: ident, sub: p?.name ?? "", hint: "Click for airport info", cat: cats.get(ident) ?? null, more: "Right-click for flight options" };
      });
      map.on("mouseleave", layer, () => {
        clearHover();
        if (tracing && map) map.getCanvas().style.cursor = "crosshair";
      });
    }
    // Right-click on an airport: send it to the planner, or fit it in view.
    map.on("contextmenu", (e) => {
      if (tracing) {
        // While measuring or drawing, a right-click takes back the last point.
        undoPoint();
        return;
      }
      const hit = airportHit(e.point);
      if (!hit) {
        ctx = null;
        return;
      }
      clearHover();
      const f = map!.queryRenderedFeatures([[e.point.x - 5, e.point.y - 5], [e.point.x + 5, e.point.y + 5]], {
        layers: AIRPORT_LAYERS.filter((l) => map!.getLayer(l)),
      });
      const name = f.map((x) => (x.properties as { name?: string }).name).find((n) => !!n) ?? "";
      ctx = { x: e.point.x, y: e.point.y, ident: hit.ident, name, at: hit.at };
    });
    map.on("mousemove", (e) => {
      if (tracing) mHover = [e.lngLat.lng, e.lngLat.lat];
    });
    map.on("mouseout", () => (mHover = null));
    map.on("zoom", () => (zoomNow = map!.getZoom()));
    map.on("click", (e) => {
      ctx = null;
      if (drawOn) {
        drawClick(e.point, [e.lngLat.lng, e.lngLat.lat]);
        return;
      }
      if (measuring) {
        mPts = [...mPts, [e.lngLat.lng, e.lngLat.lat]];
        return;
      }
      const hit = airportHit(e.point);
      if (!hit) {
        // Online network: a pilot, a staffed airport or a controlled area.
        const pick = netAt(e.point);
        if (pick) {
          clearHover();
          openAirport = null;
          wxPick = null;
          netPick = pick;
          return;
        }
        // A SIGMET or AIRMET under the click.
        if (map?.getLayer("wx-adv-fill")) {
          const f = map.queryRenderedFeatures(e.point, { layers: ["wx-adv-fill"] })[0];
          if (f) {
            openAirport = null;
            netPick = null;
            wxPick = f.properties as unknown as WxAdvisoryProps;
          }
        }
        return;
      }
      if (hit) {
        clearHover();
        netPick = null;
        wxPick = null;
        openAirport = hit.ident;
        ringFor = hit.ident;
        ringAt = hit.at;
        // Bring the airport to the middle of the part of the map the card
        // leaves free, a little closer in.
        // The card is 340 px wide, 16 px in from the left.
        const covered = Math.min(372, el.clientWidth / 2);
        map!.easeTo({
          center: hit.at,
          zoom: Math.min(Math.max(map!.getZoom() + 1.5, 6), 12),
          offset: [-covered / 2, 0],
          duration: 700,
        });
      }
    });
    for (const layer of ["net-pilots", "net-airport-dot", "net-airport-labels", "net-area-labels"]) {
      map.on("mouseenter", layer, () => (map!.getCanvas().style.cursor = "pointer"));
      map.on("mouseleave", layer, () => (map!.getCanvas().style.cursor = ""));
    }
    map.on("movestart", () => {
      hoverTip = null;
      ctx = null;
    });
    map.on("zoomend", () => {
      if (windFocus && map!.getZoom() < 8) windFocus = null;
    });
    map.on("mousemove", windUnder);
    // Runway numbers at the runway ends, for the runways in view.
    map.on("idle", () => {
      if (!map) return;
      if (map.getZoom() >= 12 && !layers.has("aeroway-off")) {
        (map.getSource("fd-runway-ends") as GeoJSONSource | undefined)?.setData(runwayEnds(map));
      }
      // Approach lines into the planned destination's runways, once its
      // runways are in the map tiles (zoom 10 and in).
      const dest = shownPlan?.destination;
      const src = map.getSource("fd-approach") as GeoJSONSource | undefined;
      if (!dest || !(dest.lat || dest.lon)) {
        if (approachShown) {
          src?.setData({ type: "FeatureCollection", features: [] });
          approachShown = "";
        }
      } else if (approachShown && approachShown !== dest.icao && map.getZoom() < 10) {
        // Diverted elsewhere while zoomed out: the old lines go.
        src?.setData({ type: "FeatureCollection", features: [] });
        approachShown = "";
      } else if (map.getZoom() >= 10) {
        const lines = approachLines(map, dest);
        if (lines.features.length || approachShown !== dest.icao) {
          src?.setData(lines);
          approachShown = lines.features.length ? dest.icao : "";
        }
      }
    });
    map.on("mouseout", () => (windAt = null));
    // Stop drawing while the window is hidden.
    const vis = () => (document.hidden ? map?.stop() : map?.triggerRepaint());
    document.addEventListener("visibilitychange", vis);
    return () => document.removeEventListener("visibilitychange", vis);
  });

  const catTimer = setInterval(loadCats, 10 * 60_000);
  onDestroy(() => {
    clearInterval(catTimer);
    for (const m of legMarkers) m.remove();
    ringMarker?.remove();
    map?.remove();
  });

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
    const p = shownPlan;
    if (!loaded || !map) return;
    (map.getSource("plan") as GeoJSONSource | undefined)?.setData(planFeatures(p));
    const id = p ? `${p.id}-${p.origin.icao}-${p.destination.icao}` : null;
    // A route being set up always shows; a real plan only before any flying.
    if (p && id !== lastPlanId && planInView(p)) fitPlan(p);
    lastPlanId = id;
  });
  function planInView(p: FlightPlan): boolean {
    return p.id === "draft" || (!untrack(() => history) && untrack(() => live.length) < 2);
  }
  function fitPlan(p: FlightPlan) {
    const coords: [number, number][] = [[p.origin.lon, p.origin.lat], ...p.fixes.map((f) => [f.lon, f.lat] as [number, number]), [p.destination.lon, p.destination.lat]];
    fit(coords.filter(([x, y]) => x || y), 8);
  }
  // A card opening over the map: move the planned route out from under it.
  let lastInset = 0;
  $effect(() => {
    const r = insetRight;
    if (!loaded || !map) return;
    const p = untrack(() => shownPlan);
    if (r > lastInset && p && planInView(p)) fitPlan(p);
    lastInset = r;
  });

  // Optional layers: past routes, airports, borders, satellite, day/night, radar.
  $effect(() => {
    const on = layers;
    if (!loaded || !map || !map.getLayer("routes-line")) return;
    const vis = (id: string, v: boolean) => map!.getLayer(id) && map!.setLayoutProperty(id, "visibility", v ? "visible" : "none");
    for (const id of ["routes-line", "routes-hit", "routes-loops"]) vis(id, on.has("routes"));
    vis("routes-airports", on.has("airports"));
    vis("night", on.has("night"));
    vis("night-edge", on.has("night"));
    showSatellite(map, on.has("satellite") && !offline);
    showTerrain(map, on.has("terrain") && !offline, on.has("satellite"));
    setBorders(map, on.has("borders"), on.has("satellite") && !offline);
    showRoads(map, on.has("roads"));
    setRoadsOpacity(map, roadsOpacity);
    // On unless switched off ("aeroway-off"), so it shows for everyone.
    showAirportLayout(map, !on.has("aeroway-off"));
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
      padding: (() => {
        const r = untrack(() => insetRight);
        const bt = untrack(() => insetBottom);
        const l = untrack(() => insetLeft);
        return tilted ? { top: 160, bottom: 60 + bt, left: 80 + l, right: 80 + r } : { top: 80, bottom: 80 + bt, left: 80 + l, right: 80 + r };
      })(),
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
    // A flight opened to its own track: its straight line is not drawn too.
    // (A flight entered by hand has no track and keeps its line.)
    const open = history && history.length > 1 ? selected : null;
    if (!loaded || !map) return;
    (map.getSource("routes") as GeoJSONSource | undefined)?.setData(
      routeFeatures(open == null ? list : list.filter((r) => r.flight_id !== open)),
    );
    if (!fittedRoutes && list.length && !aircraft?.tracking && !history && !untrack(() => plan)) {
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
    const tracking = !!aircraft.tracking;
    if (!marker) {
      marker = new maplibregl.Marker({ element: planeEl(), rotationAlignment: "map" }).setLngLat(at).addTo(map);
      // Only shown, not tracked: don't pull the map away from what you're
      // looking at, except on the Map tab when it opens.
      follow = tracking;
      if (tracking || untrack(() => planner)) map.jumpTo({ center: at, zoom: Math.max(map.getZoom(), 8) });
    }
    // A flight starts: follow it, as before.
    if (tracking && !wasTracking) follow = true;
    wasTracking = tracking;
    marker.getElement().classList.toggle("idle", !tracking);
    marker.setLngLat(at).setRotation(aircraft.hdg);
    if (follow) map.easeTo({ center: at, duration: 450, easing: (t) => t });
  });
  let wasTracking = false;

  // Parked aircraft: one tag per airport listing every aircraft left
  // there, shown only when the layer is switched on.
  let parkedMarkers: maplibregl.Marker[] = [];
  let parkedPopups: maplibregl.Popup[] = [];
  let parkedKey: string | null = null;
  /** The route under the pointer, for the hover tip. */
  let hoverTip = $state<{ x: number; y: number; label: string; sub: string; hint: string; cat?: number | null; more?: string } | null>(null);
  let mapW = $state(0);
  $effect(() => {
    const list = parked;
    const on = layers.has("parked");
    const fly = onfly;
    if (!loaded || !map) return;
    // The list arrives anew with every status update: rebuild the markers
    // (and close an open card) only when something shown has changed.
    const key = on && list
      ? JSON.stringify([!!fly, list.map((g) => [g.id, g.registration, g.title, g.model, g.last_lat, g.last_lon, g.flights, g.minutes, g.last_icao])])
      : "";
    if (key === parkedKey) return;
    parkedKey = key;
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

<div class="map" bind:this={el} bind:clientWidth={mapW} style:--inset-r="{insetRight}px" style:--inset-b="{insetBottom}px"></div>
<div
  class="hover-tip"
  class:show={!!hoverTip}
  aria-hidden="true"
  style:left="{Math.min(Math.max(hoverTip?.x ?? 0, 110), Math.max(110, mapW - 110))}px"
  style:top="{hoverTip?.y ?? 0}px"
  class:below={(hoverTip?.y ?? 100) < 90}
>
  <div class="tip-head">
    <strong>{hoverTip?.label ?? ""}</strong>
    {#if hoverTip?.cat != null && CATEGORY[hoverTip.cat]}
      <b class="cat" style:--c={CATEGORY[hoverTip.cat].color}>{CATEGORY[hoverTip.cat].name}</b>
    {/if}
  </div>
  <span>{hoverTip?.sub ?? ""}</span>
  <em>{hoverTip?.hint ?? ""}</em>
  {#if hoverTip?.more}<small>{hoverTip.more}</small>{/if}
</div>
<svelte:window onkeydown={measureKey} />
{#if ctx}
  <div
    class="ctx"
    role="menu"
    aria-label="Flight options for {ctx.ident}"
    style:left="{Math.min(ctx.x, mapW - 236)}px"
    style:top="{ctx.y}px"
  >
    <header><strong>{ctx.ident}</strong>{#if ctx.name}<span>{ctx.name}</span>{/if}</header>
    {#if planner}
      <button role="menuitem" disabled={pick.locked} onclick={() => sendToPlanner("origin")}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="4" /><path d="M12 2v4M12 18v4M2 12h4M18 12h4" /></svg>Set as origin
      </button>
      <button role="menuitem" disabled={pick.locked} onclick={() => sendToPlanner("dest")}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 21V4h11l-2 4 2 4H5" /></svg>Set as destination
      </button>
      {#if pick.locked}<p>A flight is under way: the plan is set.</p>{/if}
    {/if}
    <button role="menuitem" onclick={() => ctx && fitAirport(ctx.ident, ctx.at)}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" /><circle cx="12" cy="12" r="2.5" /></svg>Zoom to fit
    </button>
  </div>
{/if}
{#if !drawOn && (measuring || mPts.length > 1)}
  <div class="measure" style:--ml="{insetLeft}px" style:--mr="{openAirport || netPick || wxPick ? Math.max(372, insetRight) : insetRight}px" style:bottom="{16 + insetBottom}px">
    <div class="m-total">
      <span class="num big">{fmt(mTotal + (mNext?.nm ?? 0))}<small>nm</small></span>
      <span class="num alt">{fmt((mTotal + (mNext?.nm ?? 0)) * 1.150779)} mi · {fmt((mTotal + (mNext?.nm ?? 0)) * 1.852)} km</span>
    </div>
    <div class="m-sub">
      {#if !mPts.length}
        Click the map to start measuring.
      {:else if mNext}
        {legs.length ? `${legs.length} leg${legs.length > 1 ? "s" : ""} + ` : ""}next {fmt(mNext.nm)} nm, course {crs3(mNext.crs)}°
      {:else if measuring}
        {legs.length} leg{legs.length === 1 ? "" : "s"}. Click to add a point; right-click takes one back.
      {:else}
        {legs.length} leg{legs.length === 1 ? "" : "s"}, great-circle distance
      {/if}
    </div>
    <div class="m-act">
      {#if mPts.length}<button onclick={() => (mPts = mPts.slice(0, -1))} title="Take back the last point (Backspace)">Undo</button>{/if}
      {#if mPts.length}<button onclick={clearMeasure}>Clear</button>{/if}
      {#if measuring}
        <button class="done" onclick={() => setMeasuring(false)}>Done</button>
      {:else}
        <button onclick={() => setMeasuring(true)}>Add points</button>
        <button class="done" onclick={clearMeasure}>Close</button>
      {/if}
    </div>
  </div>
{/if}
{#if aircraft && !follow && !history}
  <button class="follow" style:left="{16 + insetLeft}px" onclick={() => (follow = true)}>Follow aircraft</button>
{/if}
{#if !bare}
<MapLayers
  {three}
  on={layers}
  onthree={setThree}
  ontoggle={toggleLayer}
  {radarTime}
  {radarError}
  parkedLayer={parked != null}
  {airportDetail}
  ondetail={(d) => setPref("map_airport_detail", String(d))}
  windLevel={windPref}
  onwindlevel={(l) => setPref("map_wind_level", l)}
  cardOpen={!!(openAirport || (netPick && !openAirport) || (wxPick && !openAirport && !netPick))}
  {measuring}
  onmeasure={() => setMeasuring(!measuring)}
  airspaceState={aspState}
  obstaclesState={obstState}
  hasAircraft={acAltFt != null}
  {roadsOpacity}
  onroadsopacity={(o) => setPref("map_roads_opacity", o.toFixed(2))}
  {ringCount}
  {ringSet}
  onrings={(count, set) => {
    setPref("map_rings_count", String(count));
    setPref("map_rings_nm", set);
  }}
  profileOpen={profileState.open}
  onprofile={planner ? () => (profileState.open = !profileState.open) : undefined}
  pfdOpen={pfdState.open}
  onpfd={planner ? () => (pfdState.open = !pfdState.open) : undefined}
/>
{/if}
{#if wxPick && !openAirport && !netPick}
  <WxCard item={wxPick} onclose={() => (wxPick = null)} />
{/if}
{#if netPick && !openAirport}
  <NetworkCard
    pick={netPick}
    onclose={() => (netPick = null)}
    onairport={(icao) => {
      netPick = null;
      openAirport = icao;
    }}
  />
{/if}
{#if openAirport}
  <AirportCard
    ident={openAirport}
    onclose={() => (openAirport = null)}
    onzoom={zoomToAirport}
    windOn={layers.has("wx-wind")}
    onwind={() => toggleLayer("wx-wind")}
    onflight={onroute
      ? (id) => {
          openAirport = null;
          onroute?.(id);
        }
      : undefined}
  />
{/if}
<div class="keys">
{#if layers.has("wind")}
  <div class="wind-key" aria-label="Wind speed colours">
    <span class="t">
      Wind
      <select aria-label="Wind level" value={windPref} onchange={(e) => setPref("map_wind_level", (e.currentTarget as HTMLSelectElement).value)}>
        <option value="auto">Auto ({WIND_LEVELS.find((l) => l.id === windLevel)?.label})</option>
        {#each WIND_LEVELS as l (l.id)}<option value={l.id}>{l.label}</option>{/each}
      </select>
    </span>
    <span class="bar" style:background={windKey}></span>
    <span class="ends"><span>0</span><span>70</span><span>140+ kt</span></span>
    <span class="sub">
      {#if windError && !windField}Forecast not reachable{:else if !windField}Loading…{:else if windAt}Here: {windAt}{:else}GFS {zulu(windField.run_ms)} run, for {zulu(windField.valid_ms)}{/if}
    </span>
  </div>
{/if}
{#if shown.length > 1}
  <div class="alt-key" aria-label="Line color shows altitude">
    <span class="t">Altitude{#if three && stretch > 1}<em>heights ×{stretch}</em>{/if}</span>
    <span class="bar" style:background={keyBar}></span>
    <span class="ends"><span>{altLabel(range.lo)}</span><span>{altLabel(range.top)}</span></span>
  </div>
{/if}
</div>
{#if offline}
  <p class="offline">Map tiles could not load, so the map shows a plain grid. Tracking keeps working.</p>
{/if}

<style>
  .map {
    position: absolute;
    inset: 0;
    background: #17191b;
  }
  .map :global(.maplibregl-ctrl-bottom-right) {
    right: var(--inset-r, 0px);
    bottom: var(--inset-b, 0px);
    transition: right 0.25s ease, bottom 0.25s ease;
  }
  .map :global(.plane) {
    filter: drop-shadow(0 0 6px rgba(240, 178, 58, 0.55));
  }
  /* Shown but not tracked: white with a dark grey outline. */
  .map :global(.plane.idle) {
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.55));
  }
  .map :global(.plane.idle path) {
    fill: #f4f5f7;
    stroke: #3b3e44;
    stroke-width: 2;
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
  .hover-tip {
    position: absolute;
    z-index: 3;
    transform: translate(-50%, calc(-100% - 12px));
    pointer-events: none;
    visibility: hidden;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 7px 10px;
    border-radius: 6px;
    background: var(--panel-2);
    color: var(--ink);
    border: 1px solid var(--rule);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.4);
    font-family: var(--sans);
    font-size: 13px;
    white-space: nowrap;
  }
  .hover-tip.below {
    transform: translate(-50%, 16px);
  }
  .hover-tip.show {
    visibility: visible;
  }
  .hover-tip strong {
    font-family: var(--cond);
    font-size: 17px;
  }
  .hover-tip span {
    color: var(--ink-dim);
  }
  .hover-tip em {
    font-style: normal;
    font-size: 11px;
    color: var(--amber);
  }
  .tip-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .hover-tip .cat {
    padding: 1px 6px;
    border-radius: 4px;
    font-family: var(--cond);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 16%, transparent);
    border: 1px solid color-mix(in srgb, var(--c) 55%, transparent);
  }
  .hover-tip small {
    font-size: 10.5px;
    color: var(--ink-faint);
  }
  @media (hover: none) {
    .hover-tip small {
      display: none;
    }
  }

  /* Right-click menu on an airport. */
  .ctx {
    position: absolute;
    z-index: 7;
    width: 220px;
    margin: 6px 0 0 6px;
    padding: 6px;
    border-radius: 10px;
    background: rgba(28, 30, 33, 0.97);
    border: 1px solid var(--rule);
    box-shadow: 0 14px 34px rgba(0, 0, 0, 0.5);
  }
  .ctx header {
    display: grid;
    padding: 4px 8px 6px;
    border-bottom: 1px solid var(--rule);
    margin-bottom: 4px;
  }
  .ctx header strong {
    font-family: var(--cond);
    font-size: 17px;
  }
  .ctx header span {
    font-size: 12px;
    color: var(--ink-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ctx button {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 8px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--ink);
    font-size: 14px;
    text-align: left;
  }
  .ctx button:hover:not(:disabled) {
    background: rgba(240, 178, 58, 0.14);
    color: var(--amber);
  }
  .ctx button:disabled {
    color: var(--ink-faint);
  }
  .ctx svg {
    width: 17px;
    height: 17px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .ctx p {
    margin: 2px 8px 6px;
    font-size: 11.5px;
    color: var(--ink-faint);
  }

  /* The airport clicked: an amber ring that glows softly in and out. Only
     its opacity changes, which every browser draws cheaply. */
  .map :global(.ap-ring) {
    width: 44px;
    height: 44px;
    pointer-events: none;
  }
  .map :global(.ap-ring span) {
    position: absolute;
    inset: 0;
    border: 2px solid #f0b23a;
    border-radius: 50%;
    box-shadow:
      0 0 10px 2px rgba(240, 178, 58, 0.45),
      inset 0 0 8px rgba(240, 178, 58, 0.35);
    animation: ap-glow 3.2s ease-in-out infinite;
  }
  .map :global(.ap-ring span.hide) {
    display: none;
  }
  @keyframes ap-glow {
    0%,
    100% {
      opacity: 0.25;
    }
    50% {
      opacity: 0.95;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .map :global(.ap-ring span) {
      animation: none;
      opacity: 0.8;
    }
  }

  /* Measuring: a label on each leg and the total along the bottom. */
  .map :global(.leg-label) {
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(17, 18, 20, 0.88);
    border: 1px solid rgba(240, 178, 58, 0.6);
    color: #f6d58f;
    font-family: var(--cond);
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    pointer-events: none;
  }
  .measure {
    position: absolute;
    z-index: 5;
    left: calc(var(--ml) + (100% - var(--ml) - var(--mr)) / 2);
    transform: translateX(-50%);
    display: grid;
    grid-template-columns: auto auto;
    align-items: center;
    gap: 2px 18px;
    width: max-content;
    max-width: calc(100% - var(--ml) - var(--mr) - 24px);
    padding: 10px 14px;
    border-radius: 12px;
    background: rgba(28, 30, 33, 0.94);
    border: 1px solid var(--rule);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.45);
  }
  .m-total {
    display: grid;
  }
  .m-total .big {
    font-family: var(--cond);
    font-size: 26px;
    font-weight: 600;
    color: var(--amber);
    line-height: 1.1;
  }
  .m-total .big small {
    margin-left: 4px;
    font-size: 14px;
    color: var(--ink-dim);
  }
  .m-total .alt {
    font-size: 12px;
    color: var(--ink-dim);
  }
  .m-sub {
    grid-column: 1 / -1;
    grid-row: 2;
    font-size: 12px;
    color: var(--ink-faint);
  }
  .m-act {
    display: flex;
    gap: 6px;
    justify-self: end;
  }
  .m-act button {
    padding: 6px 11px;
    border-radius: 7px;
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    font-size: 13px;
  }
  .m-act button:hover {
    color: var(--ink);
    border-color: var(--ink-faint);
  }
  .m-act .done {
    background: var(--amber);
    border-color: var(--amber);
    color: #1b1406;
    font-weight: 600;
  }
  @media (max-width: 640px) {
    .measure {
      left: 10px;
      right: 10px;
      transform: none;
      width: auto;
      max-width: none;
    }
  }
  .keys {
    position: absolute;
    right: 56px;
    bottom: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }
  .wind-key {
    width: 190px;
    padding: 8px 10px 7px;
    border-radius: 8px;
    background: rgba(28, 30, 33, 0.9);
    border: 1px solid var(--rule);
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .wind-key .t {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-family: var(--cond);
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-dim);
  }
  .wind-key select {
    pointer-events: auto;
    padding: 1px 4px;
    border: 1px solid var(--rule);
    border-radius: 5px;
    background: var(--panel-2);
    color: var(--ink);
    font: 600 11.5px var(--cond);
    letter-spacing: 0.02em;
  }
  .wind-key .bar {
    height: 6px;
    border-radius: 3px;
  }
  .wind-key .ends {
    display: flex;
    justify-content: space-between;
    font-family: "IBM Plex Mono", monospace;
    font-size: 10.5px;
    color: var(--ink);
  }
  .wind-key .sub {
    font-size: 11px;
    color: var(--ink-dim);
    font-family: "IBM Plex Mono", monospace;
  }
  .alt-key {
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
  /* Top left, clear of the map's menus at the top right on any screen. */
  .follow {
    position: absolute;
    z-index: 4;
    top: 16px;
    left: 16px;
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
  /* Phones: pinch to zoom instead of the zoom buttons; keys sit above the
     Start a flight button. */
  @media (max-width: 640px) {
    .map :global(.maplibregl-ctrl-group) {
      display: none;
    }
    .keys {
      right: 10px;
      bottom: 10px;
    }
    .keys > * {
      width: 170px;
    }
  }
  .map :global(.shot-pin) {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 2px solid #1b1c1e;
    border-radius: 50% 50% 50% 4px;
    transform: rotate(-45deg);
    background: var(--amber);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.5);
    cursor: pointer;
  }
  .map :global(.shot-pin.hot) {
    background: #fff;
    border-color: var(--amber);
    box-shadow:
      0 0 0 4px rgba(240, 178, 58, 0.45),
      0 4px 12px rgba(0, 0, 0, 0.6);
  }
  .map :global(.shot-pin svg) {
    width: 14px;
    height: 14px;
    transform: rotate(45deg);
    fill: none;
    stroke: #1b1406;
    stroke-width: 2;
    stroke-linejoin: round;
  }
</style>
