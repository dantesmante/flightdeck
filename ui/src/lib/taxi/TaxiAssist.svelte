<script lang="ts">
  // Taxi assist: tap the taxiways (and runways to cross or hold short of)
  // in the order ATC gives them, then follow the blue dotted line and the
  // GPS-style steps. The route is worked out only when the clearance
  // changes; while taxiing only the aircraft's place along it is updated.
  import { onDestroy, onMount, untrack } from "svelte";
  import type maplibregl from "maplibre-gl";
  import type { GeoJSONSource } from "maplibre-gl";
  import { buildGraphAsync, connected, distTo, isRunwayName, loadAeroways, toLL, toXY, type LL, type TaxiGraph, type XY } from "./graph";
  import { pointAt, progress, routeFor, type Item, type Step, type TaxiRoute } from "./route";

  let {
    getMap,
    lat,
    lon,
    hdg,
    insetLeft = 0,
    onclose,
  }: {
    /** The map (it exists before this opens and outlives it). */
    getMap: () => maplibregl.Map;
    lat: number | null;
    lon: number | null;
    hdg: number | null;
    insetLeft?: number;
    onclose: () => void;
  } = $props();
  const map = untrack(() => getMap());

  // By value: the parent's aircraft object is rebuilt twice a second.
  const acLat = $derived(lat);
  const acLon = $derived(lon);
  const acHdg = $derived(hdg);

  let phase = $state<"loading" | "ready" | "none">("loading");
  let graph = $state.raw<TaxiGraph | null>(null);
  let fromMock = false;
  let items = $state.raw<Item[]>([]);
  let pendingRwy = $state<string | null>(null);
  let showAll = $state(false);
  let gateOpen = $state(false);
  let gateQuery = $state("");
  let editing = $state(true);
  let cleared = $state.raw<Set<string>>(new Set());
  let route = $state.raw<TaxiRoute | null>(null);
  let along = $state(0);
  let off = $state(0);
  let buildMs = 0;

  const here = (): LL => {
    const la = untrack(() => acLat);
    const lo = untrack(() => acLon);
    if (la != null && lo != null) return [lo, la];
    const c = map.getCenter();
    return [c.lng, c.lat];
  };
  /** Where the clearance starts from: the aircraft when it was last edited. */
  let startXY = $state.raw<XY>([0, 0]);
  let startHdg = 0;

  onMount(async () => {
    const at = here();
    const t0 = performance.now();
    const raw = await loadAeroways(map, at);
    fromMock = !!(globalThis as { __fdTaxiLayout?: unknown }).__fdTaxiLayout && raw.length > 0;
    const g = await buildGraphAsync(raw, at);
    buildMs = performance.now() - t0;
    (globalThis as { __fdTaxiBuildMs?: number }).__fdTaxiBuildMs = buildMs;
    graph = g;
    phase = g.byName.size ? "ready" : "none";
    startXY = toXY(at, at);
    startHdg = untrack(() => acHdg) ?? 0;
    addLayers();
    // Close enough to see the taxiways.
    if (map.getZoom() < 14.5) map.easeTo({ center: at, zoom: 15, duration: 900 });
  });

  // Work the route out again only when the clearance changes.
  $effect(() => {
    const g = graph;
    const its = items;
    if (!g) return;
    untrack(() => {
      const p = here();
      startXY = toXY(g.origin, p);
      startHdg = acHdg ?? 0;
      const t0 = performance.now();
      route = its.length ? routeFor(g, its, startXY, startHdg) : null;
      if (fromMock) (globalThis as Record<string, unknown>).__fdTaxi = { route, origin: g.origin, ms: performance.now() - t0 };
      along = 0;
      off = 0;
      cleared = new Set();
    });
  });

  // Follow the aircraft along it.
  $effect(() => {
    const la = acLat;
    const lo = acLon;
    const r = route;
    const g = graph;
    if (!r || !g || la == null || lo == null || r.pts.length < 2) return;
    untrack(() => {
      const p = progress(r, toXY(g.origin, [lo, la]), along);
      if (Math.abs(p.along - along) > 2) along = p.along;
      off = p.off;
      // A hold line passed: cleared to cross it.
      const passed = r.holds.filter((h) => along > h.at + 25 && !cleared.has(h.runway));
      if (passed.length) cleared = new Set([...cleared, ...passed.map((h) => h.runway)]);
    });
  });

  // Drawing: redone when the route, the clearance to cross, or the place
  // along it (in 3 m steps) changes.
  const alongStep = $derived(Math.floor(along / 3));
  $effect(() => {
    void alongStep;
    const r = route;
    const c = cleared;
    const g = graph;
    if (!g) return;
    untrack(() => draw(g, r, c));
  });

  function holdAhead(r: TaxiRoute, c: Set<string>) {
    return r.holds.find((h) => !c.has(h.runway) && h.at >= along - 25);
  }

  const SRC = "fd-taxi";
  const NET = "fd-taxi-net";
  const LAYERS = ["fd-taxi-net-line", "fd-taxi-after", "fd-taxi-ahead", "fd-taxi-hold", "fd-taxi-warn", "fd-taxi-end"];
  const empty: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
  function addLayers() {
    if (!map.getSource(SRC)) map.addSource(SRC, { type: "geojson", data: empty });
    if (fromMock && !map.getSource(NET)) map.addSource(NET, { type: "geojson", data: netData() });
    const dots = (w: number): maplibregl.ExpressionSpecification => ["interpolate", ["exponential", 1.6], ["zoom"], 13, w * 0.5, 15, w, 17, w * 1.6, 19, w * 2.4];
    const add = (l: maplibregl.LayerSpecification) => {
      if (!map.getLayer(l.id)) map.addLayer(l);
    };
    if (fromMock)
      add({ id: "fd-taxi-net-line", type: "line", source: NET, paint: { "line-color": ["match", ["get", "runway"], 1, "#9aa0a8", "#c9a43a"], "line-width": ["match", ["get", "runway"], 1, 8, 2], "line-opacity": 0.55 } });
    add({ id: "fd-taxi-after", type: "line", source: SRC, filter: ["==", ["get", "k"], "after"], layout: { "line-cap": "round", "line-join": "round" }, paint: { "line-color": "#8d939c", "line-width": dots(6), "line-dasharray": [0, 2] } });
    add({ id: "fd-taxi-ahead", type: "line", source: SRC, filter: ["==", ["get", "k"], "ahead"], layout: { "line-cap": "round", "line-join": "round" }, paint: { "line-color": "#2f8cff", "line-width": dots(7), "line-dasharray": [0, 1.9] } });
    add({ id: "fd-taxi-hold", type: "line", source: SRC, filter: ["==", ["get", "k"], "hold"], layout: { "line-cap": "butt" }, paint: { "line-color": ["case", ["get", "clear"], "#5b6068", "#ff3b30"], "line-width": dots(5) } });
    add({ id: "fd-taxi-warn", type: "circle", source: SRC, filter: ["==", ["get", "k"], "warn"], paint: { "circle-color": "rgba(255,170,0,0.25)", "circle-stroke-color": "#ffaa00", "circle-stroke-width": 2, "circle-radius": dots(9) } });
    add({ id: "fd-taxi-end", type: "circle", source: SRC, filter: ["==", ["get", "k"], "end"], paint: { "circle-color": "#2f8cff", "circle-stroke-color": "#ffffff", "circle-stroke-width": 2.5, "circle-radius": dots(6) } });
  }
  function removeLayers() {
    for (const id of LAYERS) if (map.getLayer(id)) map.removeLayer(id);
    for (const s of [SRC, NET]) if (map.getSource(s)) map.removeSource(s);
  }
  // A new map style drops the layers; put them back.
  const onStyle = () => {
    if (phase !== "loading" && !map.getSource(SRC)) {
      addLayers();
      if (graph) draw(graph, route, cleared);
    }
  };
  map.on("styledata", onStyle);
  onDestroy(() => {
    map.off("styledata", onStyle);
    try {
      removeLayers();
    } catch {
      /* the map is gone */
    }
  });

  function netData(): GeoJSON.FeatureCollection {
    const g = graph;
    if (!g) return empty;
    return {
      type: "FeatureCollection",
      features: g.edges.map((e) => ({ type: "Feature", properties: { runway: e.runway ? 1 : 0 }, geometry: { type: "LineString", coordinates: [toLL(g.origin, g.nodes[e.a]), toLL(g.origin, g.nodes[e.b])] } })),
    };
  }

  function draw(g: TaxiGraph, r: TaxiRoute | null, c: Set<string>) {
    const src = map.getSource(SRC) as GeoJSONSource | undefined;
    if (!src) return;
    if (fromMock) (map.getSource(NET) as GeoJSONSource | undefined)?.setData(netData());
    const fs: GeoJSON.Feature[] = [];
    if (r && r.pts.length > 1) {
      const ll = (p: XY) => toLL(g.origin, p);
      const hold = holdAhead(r, c);
      const stop = hold ? hold.at : r.length;
      const from = Math.min(along, stop);
      const piece = (a: number, b: number): LL[] => {
        const out: LL[] = [ll(pointAt(r.pts, r.along, a).xy)];
        for (let i = 0; i < r.pts.length; i++) if (r.along[i] > a && r.along[i] < b) out.push(ll(r.pts[i]));
        out.push(ll(pointAt(r.pts, r.along, b).xy));
        return out;
      };
      if (stop - from > 1) fs.push({ type: "Feature", properties: { k: "ahead" }, geometry: { type: "LineString", coordinates: piece(from, stop) } });
      if (r.length - stop > 1) fs.push({ type: "Feature", properties: { k: "after" }, geometry: { type: "LineString", coordinates: piece(stop, r.length) } });
      for (const h of r.holds) {
        const nx = -h.dirXY[1];
        const ny = h.dirXY[0];
        const w = 24;
        fs.push({
          type: "Feature",
          properties: { k: "hold", clear: c.has(h.runway) },
          geometry: { type: "LineString", coordinates: [ll([h.xy[0] - nx * w, h.xy[1] - ny * w]), ll([h.xy[0] + nx * w, h.xy[1] + ny * w])] },
        });
      }
      for (const u of r.uncleared) fs.push({ type: "Feature", properties: { k: "warn" }, geometry: { type: "Point", coordinates: ll(u.xy) } });
      fs.push({ type: "Feature", properties: { k: "end" }, geometry: { type: "Point", coordinates: ll(r.pts[r.pts.length - 1]) } });
    }
    src.setData({ type: "FeatureCollection", features: fs });
  }

  // ---- The bubbles ----
  const lastTwy = $derived.by(() => {
    for (let i = items.length - 1; i >= 0; i--) {
      const it = items[i];
      if (it.t === "twy") return it.name;
    }
    return null;
  });
  const done = $derived(items.length > 0 && items[items.length - 1].t === "gate");
  type Bubble = { name: string; runway: boolean };
  const suggestions = $derived.by((): Bubble[] => {
    const g = graph;
    if (!g) return [];
    const names = [...g.byName.keys()];
    if (showAll) return names.sort((a, b) => a.localeCompare(b, undefined, { numeric: true })).map((name) => ({ name, runway: isRunwayName(g, name) }));
    const base = startXY;
    if (!lastTwy) {
      return names
        .map((name) => ({ name, runway: isRunwayName(g, name), d: distTo(g, name, base) }))
        .filter((b) => b.d < (b.runway ? 700 : 1500))
        .sort((a, b) => a.d - b.d)
        .slice(0, 12);
    }
    const end = route?.pts.length ? route.pts[route.pts.length - 1] : base;
    const lastItem = items[items.length - 1];
    return [...connected(g, lastTwy)]
      .filter(([name]) => !(lastItem.t === "rwy" && lastItem.name === name))
      .map(([name, at]) => ({ name, runway: isRunwayName(g, name), d: Math.hypot(at[0] - end[0], at[1] - end[1]) }))
      .sort((a, b) => a.d - b.d)
      .slice(0, 12);
  });
  const gateList = $derived.by(() => {
    const g = graph;
    if (!g || !g.gates.length) return [];
    const q = gateQuery.trim().toUpperCase();
    if (q) return g.gates.filter((x) => x.ref.toUpperCase().startsWith(q)).slice(0, 8);
    if (!lastTwy) return [];
    return g.gates
      .map((x) => ({ x, d: distTo(g, lastTwy, x.xy) }))
      .sort((a, b) => a.d - b.d)
      .slice(0, 5)
      .map((v) => v.x);
  });

  function pick(b: Bubble) {
    showAll = false;
    if (b.runway) {
      pendingRwy = pendingRwy === b.name ? null : b.name;
      return;
    }
    pendingRwy = null;
    items = [...items.filter((i) => i.t !== "gate"), { t: "twy", name: b.name }];
  }
  function pickRwy(mode: "cross" | "hold") {
    if (!pendingRwy) return;
    items = [...items.filter((i) => i.t !== "gate"), { t: "rwy", name: pendingRwy, mode }];
    pendingRwy = null;
  }
  function pickGate(ref: string, xy: XY) {
    items = [...items.filter((i) => i.t !== "gate"), { t: "gate", ref, xy }];
    gateOpen = false;
    gateQuery = "";
  }
  function removeAt(i: number) {
    items = items.filter((_, j) => j !== i);
    pendingRwy = null;
  }
  const label = (it: Item) => (it.t === "twy" ? it.name : it.t === "rwy" ? `${it.mode === "hold" ? "Hold" : "Cross"} ${it.name}` : `Gate ${it.ref}`);

  // ---- GPS-style directions ----
  const ft = (m: number) => {
    const f = m * 3.28084;
    return f < 1000 ? `${Math.max(0, Math.round(f / 50) * 50)} ft` : `${(f / 6076.12).toFixed(1)} nm`;
  };
  const next = $derived.by(() => {
    const r = route;
    if (!r || r.pts.length < 2) return null;
    const list = r.steps.filter((s) => s.kind !== "warn" || s.at > 0);
    const i = list.findIndex((s) => s.at > along - 4 && !(s.kind === "hold" && cleared.has(s.runway ?? "")));
    if (i < 0) return { step: { at: r.length, kind: "end", text: "You're there" } as Step, then: null };
    return { step: list[i], then: list[i + 1] ?? null };
  });
  const holding = $derived(!!next && next.step.kind === "hold" && next.step.at - along < 30);
  const offRoute = $derived(off > 40 && along < (route?.length ?? 0) - 20);
  const arrow = (s: Step) =>
    s.kind === "hold"
      ? "M4 9h16M4 15h16"
      : s.kind === "gate" || s.kind === "end"
        ? "M12 21s-6-6.2-6-11a6 6 0 1112 0c0 4.8-6 11-6 11z"
        : s.kind === "warn"
          ? "M12 3l10 18H2zM12 10v5M12 18v.5"
          : s.dir === "left"
            ? "M15 20v-7a3 3 0 00-3-3H6M9 6l-4 4 4 4"
            : s.dir === "right"
              ? "M9 20v-7a3 3 0 013-3h6M15 6l4 4-4 4"
              : "M12 20V5M7 10l5-5 5 5";
</script>

<div class="taxi" style:--ml="{insetLeft}px" role="region" aria-label="Taxi assist">
  {#if phase === "loading"}
    <div class="row note">Loading the airport's taxiways…</div>
  {:else if phase === "none"}
    <div class="row note">
      No taxiways mapped here (OpenStreetMap).
      <button class="x" onclick={onclose} aria-label="Close taxi assist">✕</button>
    </div>
  {:else if !editing && route && next}
    <div class="banner" class:hold={holding} class:warn={offRoute || next.step.kind === "warn"}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d={arrow(next.step)} /></svg>
      <div class="bt">
        <div class="main">{offRoute ? `Off route · ${Math.round(off)} m` : next.step.text}</div>
        <div class="sub">
          {#if !offRoute && next.step.kind !== "end"}{ft(next.step.at - along)}{/if}
          {#if next.then}<span class="then">then {next.then.text}</span>{/if}
        </div>
      </div>
      {#if holding && next.step.runway}
        <button class="clr" onclick={() => (cleared = new Set([...cleared, next!.step.runway!]))}>Cleared to cross</button>
      {/if}
      <button class="ghost" onclick={() => (editing = true)}>Edit</button>
      <button class="x" onclick={onclose} aria-label="Close taxi assist">✕</button>
    </div>
  {:else}
    <div class="row readback">
      <span class="k">Clearance</span>
      {#if !items.length}
        <span class="hint">Tap the taxiways in the order ATC gives them</span>
      {/if}
      {#each items as it, i (i)}
        <button class="chip {it.t}" class:hold={it.t === "rwy" && it.mode === "hold"} title="Remove" onclick={() => removeAt(i)}>{label(it)}</button>
      {/each}
      <span class="sp"></span>
      {#if items.length}
        <button class="ghost" onclick={() => removeAt(items.length - 1)} title="Take back the last one" aria-label="Undo">⌫</button>
        <button class="ghost" onclick={() => (items = [])}>Clear</button>
      {/if}
      {#if route && route.pts.length > 1 && !route.problem}
        <button class="go" onclick={() => (editing = false)}>Go</button>
      {/if}
      <button class="x" onclick={onclose} aria-label="Close taxi assist">✕</button>
    </div>
    {#if route?.problem}
      <div class="row problem">{route.problem}</div>
    {:else if route?.openEnd && !done}
      <div class="row tip">Add the gate, or the runway to hold short of, to finish the route.</div>
    {/if}
    {#if pendingRwy}
      <div class="row">
        <span class="k">RWY {pendingRwy}</span>
        <button class="b rwy" onclick={() => pickRwy("cross")}>Cross</button>
        <button class="b rwy hs" onclick={() => pickRwy("hold")}>Hold short</button>
        <button class="ghost" onclick={() => (pendingRwy = null)}>Cancel</button>
      </div>
    {:else}
      <div class="row bubbles">
        {#each suggestions as b (b.name)}
          <button class="b" class:rwy={b.runway} onclick={() => pick(b)}>{b.runway ? `RWY ${b.name}` : b.name}</button>
        {/each}
        <button class="b more" aria-pressed={showAll} onclick={() => (showAll = !showAll)}>{showAll ? "Nearby" : "All"}</button>
        {#if graph?.gates.length}
          <button class="b gate" aria-pressed={gateOpen} onclick={() => (gateOpen = !gateOpen)}>Gate…</button>
        {/if}
      </div>
      {#if gateOpen || (lastTwy && gateList.length)}
        <div class="row bubbles">
          {#if gateOpen}
            <!-- svelte-ignore a11y_autofocus -->
            <input class="gq" placeholder="Gate, e.g. M7" bind:value={gateQuery} autofocus onkeydown={(e) => e.key === "Enter" && gateList[0] && pickGate(gateList[0].ref, gateList[0].xy)} />
          {:else}
            <span class="k">Gates</span>
          {/if}
          {#each gateList as x (x.ref + x.xy[0])}
            <button class="b gate" onclick={() => pickGate(x.ref, x.xy)}>{x.ref}</button>
          {/each}
        </div>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .taxi {
    position: absolute;
    z-index: 6;
    top: 62px;
    left: calc(var(--ml) + (100% - var(--ml)) / 2);
    transform: translateX(-50%);
    width: min(760px, calc(100% - var(--ml) - 24px));
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 10px;
    border-radius: 12px;
    background: rgba(24, 26, 29, 0.94);
    border: 1px solid var(--rule, #34373c);
    box-shadow: 0 6px 22px rgba(0, 0, 0, 0.35);
    color: var(--ink, #eceef1);
    font-size: 13px;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .k {
    font-size: 11px;
    color: var(--ink-faint, #7d828a);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-right: 2px;
  }
  .hint,
  .note,
  .tip {
    color: var(--ink-dim, #a8adb4);
  }
  .tip {
    font-size: 12px;
  }
  .problem {
    color: #ffb340;
    font-size: 12px;
  }
  .sp {
    flex: 1;
  }
  button {
    font: inherit;
    cursor: pointer;
  }
  .b,
  .chip {
    border: 0;
    border-radius: 999px;
    padding: 5px 12px;
    font: 700 13px "IBM Plex Mono", ui-monospace, monospace;
    background: #f2c230;
    color: #111;
    min-width: 38px;
  }
  .b.rwy,
  .chip.rwy {
    background: #c8202a;
    color: #fff;
  }
  .b.rwy.hs,
  .chip.rwy.hold {
    background: #fff;
    color: #c8202a;
    box-shadow: inset 0 0 0 2px #c8202a;
  }
  .b.gate,
  .chip.gate {
    background: #2f8cff;
    color: #fff;
  }
  .b.more {
    background: transparent;
    color: var(--ink-dim, #a8adb4);
    box-shadow: inset 0 0 0 1px var(--rule, #34373c);
  }
  .chip {
    background: #111;
    color: #f2c230;
    box-shadow: inset 0 0 0 2px #f2c230;
  }
  .chip:hover {
    text-decoration: line-through;
  }
  .ghost {
    border: 1px solid var(--rule, #34373c);
    background: transparent;
    color: var(--ink-dim, #a8adb4);
    border-radius: 7px;
    padding: 4px 9px;
  }
  .go {
    border: 0;
    border-radius: 7px;
    padding: 5px 14px;
    background: #2f8cff;
    color: #fff;
    font-weight: 700;
  }
  .x {
    border: 0;
    background: transparent;
    color: var(--ink-faint, #7d828a);
    font-size: 15px;
    padding: 2px 4px;
    margin-left: auto;
  }
  .gq {
    width: 130px;
    padding: 4px 8px;
    border-radius: 7px;
    border: 1px solid var(--rule, #34373c);
    background: #111214;
    color: var(--ink, #eceef1);
    font: 600 13px "IBM Plex Mono", ui-monospace, monospace;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .banner svg {
    flex: none;
    width: 40px;
    height: 40px;
    padding: 6px;
    border-radius: 10px;
    background: #2f8cff;
    fill: none;
    stroke: #fff;
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .banner.hold svg {
    background: #ff3b30;
  }
  .banner.warn svg {
    background: #ffaa00;
  }
  .bt {
    flex: 1;
    min-width: 0;
  }
  .main {
    font: 700 20px "Barlow Condensed", "Barlow", sans-serif;
    letter-spacing: 0.01em;
  }
  .banner.hold .main {
    color: #ff6b62;
  }
  .sub {
    color: var(--ink-dim, #a8adb4);
    font-size: 12.5px;
    display: flex;
    gap: 10px;
  }
  .then {
    color: var(--ink-faint, #7d828a);
  }
  .clr {
    border: 0;
    border-radius: 7px;
    padding: 6px 12px;
    background: #ff3b30;
    color: #fff;
    font-weight: 700;
  }
  @media (max-width: 640px) {
    .taxi {
      top: 58px;
      width: calc(100% - 16px);
      left: 50%;
    }
  }
</style>
