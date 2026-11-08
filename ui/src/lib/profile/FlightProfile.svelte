<script lang="ts">
  // The flight profile under the map: the planned climb, cruise and
  // descent along the route, the ground beneath it (coarse: a guide, not a
  // survey), FAA obstacles and US class B/C/D airspace along the way, and
  // the aircraft, side on, flying left to right. Plain SVG drawn to the
  // panel's size; the data is worked out once per route.
  import { api } from "../api";
  import {
    airspaceAlong,
    altAt,
    groundFt,
    groundZoom,
    obstaclesAlong,
    plannedPath,
    pointAt,
    project,
    routePoints,
    type ProfileAirspace,
    type ProfileObstacle,
    type ProfileRoute,
  } from "./build";

  let {
    route,
    aircraft,
    onclose,
  }: {
    route: ProfileRoute | null;
    /** The aircraft while the sim is connected; `tracking` while a flight is recorded. */
    aircraft: { lat: number; lon: number; alt: number; tracking: boolean } | null;
    onclose: () => void;
  } = $props();

  // --- Data ------------------------------------------------------------------
  const pts = $derived(route ? routePoints(route) : []);
  const total = $derived(pts.length ? pts[pts.length - 1].d : 0);
  /** The route's identity: work out the ground and the rest again only when it changes. */
  const key = $derived(pts.map((p) => `${p.ident}${p.lat.toFixed(4)}${p.lon.toFixed(4)}${Math.round(p.alt || 0)}`).join("|"));

  let ground = $state<{ d: number; ft: number | null }[]>([]);
  let groundBusy = $state(false);
  $effect(() => {
    void key;
    const p = pts;
    const t = total;
    ground = [];
    if (p.length < 2 || t <= 0) return;
    let gone = false;
    const step = Math.max(0.25, t / 400);
    const ds: number[] = [];
    for (let d = 0; d < t; d += step) ds.push(d);
    ds.push(t);
    groundBusy = true;
    groundFt(
      ds.map((d) => pointAt(p, d)),
      groundZoom(t),
    ).then((ft) => {
      if (gone) return;
      ground = ds.map((d, i) => ({ d, ft: ft[i] }));
      groundBusy = false;
    });
    return () => {
      gone = true;
    };
  });

  // Obstacles and airspace: the same FAA files the map layers use, loaded
  // once (they are cached on the PC).
  let obstFc: GeoJSON.FeatureCollection | null = null;
  let aspFc: GeoJSON.FeatureCollection | null = null;
  let obstacles = $state<ProfileObstacle[]>([]);
  let airspace = $state<ProfileAirspace[]>([]);
  let faaNote = $state("");
  $effect(() => {
    void key;
    const p = pts;
    obstacles = [];
    airspace = [];
    if (p.length < 2) return;
    let gone = false;
    (async () => {
      try {
        obstFc ??= JSON.parse(await api.usObstacles()) as GeoJSON.FeatureCollection;
        if (!gone) obstacles = obstaclesAlong(p, obstFc);
      } catch {
        if (!gone) faaNote = "FAA obstacles not reachable";
      }
      try {
        aspFc ??= JSON.parse(await api.usAirspace()) as GeoJSON.FeatureCollection;
        if (!gone) airspace = airspaceAlong(p, aspFc, Math.max(0.2, total / 600));
      } catch {
        if (!gone) faaNote = faaNote ? "FAA data not reachable" : "FAA airspace not reachable";
      }
    })();
    return () => {
      gone = true;
    };
  });

  const elevAt = (d: number): number | null => {
    if (!ground.length) return null;
    let best = ground[0];
    for (const g of ground) if (Math.abs(g.d - d) < Math.abs(best.d - d)) best = g;
    return best.ft;
  };
  const origElev = $derived(ground[0]?.ft ?? 0);
  const destElev = $derived(ground[ground.length - 1]?.ft ?? 0);
  const path = $derived(pts.length >= 2 ? plannedPath(pts, route?.cruise_ft ?? 0, origElev, destElev) : []);
  const cruise = $derived(path.length ? Math.max(...path.map((p) => p[1])) : 0);

  /** The aircraft along the route, if it is anywhere near it. */
  const ac = $derived.by(() => {
    if (!aircraft || pts.length < 2) return null;
    const at = project(pts, aircraft.lat, aircraft.lon);
    return { ...at, alt: aircraft.alt, tracking: aircraft.tracking, near: at.off <= 25 };
  });

  // Highest ground or obstacle top (ahead of the aircraft while it flies the
  // route), and the cruise altitude's margin over it.
  const highest = $derived.by(() => {
    const from = ac?.near && ac.tracking ? ac.d : 0;
    let h = -Infinity;
    for (const g of ground) if (g.d >= from && g.ft != null && g.ft > h) h = g.ft;
    if (show.obst) for (const o of obstacles) if (o.d >= from && o.amsl > h) h = o.amsl;
    return Number.isFinite(h) ? h : null;
  });
  const ahead = $derived(!!(ac?.near && ac.tracking));

  // Layers of the chart, switched in its header.
  let show = $state({ terrain: true, obst: true, asp: true });

  // --- Chart -------------------------------------------------------------------
  let w = $state(0);
  let h = $state(0);
  const M = { l: 58, r: 18, t: 30, b: 42 };
  const cw = $derived(Math.max(10, w - M.l - M.r));
  const ch = $derived(Math.max(10, h - M.t - M.b));
  const yMax = $derived.by(() => {
    let top = Math.max(cruise, highest ?? 0, ac?.near ? ac.alt : 0, 1000);
    for (const a of airspace) if (show.asp) top = Math.max(top, Math.min(a.ceiling, cruise * 1.4 || a.ceiling));
    const step = niceStep(top * 1.15, 5);
    return Math.ceil((top * 1.12) / step) * step;
  });
  const x = (d: number) => M.l + (total > 0 ? (d / total) * cw : 0);
  const y = (ft: number) => M.t + (1 - Math.max(0, ft) / yMax) * ch;

  function niceStep(span: number, n: number): number {
    const raw = span / n;
    for (const s of [1, 2, 5, 10, 20, 25, 50, 100, 200, 250, 500, 1000, 2000, 2500, 5000, 10000, 20000, 25000, 50000]) if (s >= raw) return s;
    return 100000;
  }
  const yTicks = $derived.by(() => {
    const step = niceStep(yMax, Math.max(2, Math.floor(ch / 42)));
    const out: number[] = [];
    for (let v = 0; v <= yMax + 1; v += step) out.push(v);
    return out;
  });
  const xTicks = $derived.by(() => {
    if (total <= 0) return [];
    const step = niceStep(total, Math.max(2, Math.floor(cw / 70)));
    const out: number[] = [];
    for (let v = 0; v <= total + 1e-6; v += step) out.push(v);
    return out;
  });
  const fl = (ft: number) => `FL${String(Math.round(ft / 100)).padStart(3, "0")}`;
  const fmt = (n: number) => Math.round(n).toLocaleString("en-US");
  const nm = (n: number) => (n < 10 && n % 1 ? n.toFixed(1) : String(Math.round(n)));

  const groundPath = $derived.by(() => {
    const g = ground.filter((p) => p.ft != null) as { d: number; ft: number }[];
    if (g.length < 2) return "";
    let s = `M ${x(g[0].d)} ${y(0)}`;
    for (const p of g) s += ` L ${x(p.d)} ${y(p.ft)}`;
    return s + ` L ${x(g[g.length - 1].d)} ${y(0)} Z`;
  });
  const planLine = $derived(path.map(([d, a]) => `${x(d)},${y(a)}`).join(" "));

  /** Fix names along the top, leaving out those that would overlap. */
  const fixLabels = $derived.by(() => {
    const out: { d: number; ident: string }[] = [];
    // Clear of the airport names at both ends.
    let last = M.l + 4;
    for (const p of pts.slice(1, -1)) {
      const px = x(p.d);
      if (px - last < 44 || px > M.l + cw - 44) continue;
      out.push({ d: p.d, ident: p.ident });
      last = px;
    }
    return out;
  });

  function obstColor(o: ProfileObstacle): string {
    const plan = altAt(path, o.d);
    if (Number.isNaN(plan)) return "#ffffff";
    return o.amsl >= plan - 100 ? "#ff3b30" : o.amsl >= plan - 1000 ? "#ecc428" : "#ffffff";
  }

  // Hover: distance, plan and ground under the pointer.
  let hoverD = $state<number | null>(null);
  function move(e: PointerEvent) {
    const r = (e.currentTarget as SVGElement).getBoundingClientRect();
    const px = e.clientX - r.left;
    hoverD = px < M.l || px > M.l + cw || total <= 0 ? null : ((px - M.l) / cw) * total;
  }
</script>

<section class="profile" aria-label="Flight profile">
  <header>
    <h3>Flight profile</h3>
    {#if route}
      <span class="route">{route.origin.icao} → {route.dest.icao} · {nm(total)} nm</span>
      <span class="read">
        <span class="k">{ahead ? "Highest ahead" : "Highest"}</span>
        <b>{highest != null ? `${fmt(highest)} ft` : groundBusy ? "…" : "–"}</b>
      </span>
      {#if cruise > 0}
        <span class="read" title="Cruise altitude above the highest ground or obstacle">
          <span class="k">Clearance</span>
          <b class:warn={highest != null && cruise - highest < 1000}>{highest != null ? `${fmt(cruise - highest)} ft` : "–"}</b>
        </span>
      {/if}
      {#if ac && !ac.near}<span class="note">Aircraft {Math.round(ac.off)} nm off the route</span>{/if}
      {#if faaNote}<span class="note">{faaNote}</span>{/if}
    {/if}
    <span class="grow"></span>
    {#if route}
      <div class="chips" role="group" aria-label="Show in the profile">
        <button aria-pressed={show.terrain} class:on={show.terrain} onclick={() => (show.terrain = !show.terrain)}>Terrain</button>
        <button aria-pressed={show.obst} class:on={show.obst} onclick={() => (show.obst = !show.obst)} title="FAA obstacles within 2 nm of the route (US only)">Obstacles</button>
        <button aria-pressed={show.asp} class:on={show.asp} onclick={() => (show.asp = !show.asp)} title="Class B, C and D airspace (US only)">Airspace</button>
      </div>
    {/if}
    <button class="x" aria-label="Close the flight profile" onclick={onclose}>
      <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
    </button>
  </header>

  <div class="chart" bind:clientWidth={w} bind:clientHeight={h}>
    {#if !route}
      <p class="empty">
        No route yet. Load a SimBrief plan, or set an origin and destination (or draw a route) in Start a flight, and its profile shows here.
      </p>
    {:else if w > 0 && h > 0}
      <svg width={w} height={h} role="img" aria-label="Vertical profile of the route" onpointermove={move} onpointerleave={() => (hoverD = null)}>
        <defs>
          <linearGradient id="fp-sky" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stop-color="#16244a" />
            <stop offset="1" stop-color="#22396a" />
          </linearGradient>
        </defs>
        <rect x={M.l} y={M.t} width={cw} height={ch} fill="url(#fp-sky)" />

        <!-- Altitude grid. -->
        {#each yTicks as v (v)}
          <line x1={M.l} x2={M.l + cw} y1={y(v)} y2={y(v)} class="grid" />
          <text x={M.l - 8} y={y(v) + 4} text-anchor="end" class="axis">{fl(v)}</text>
        {/each}

        <!-- Airspace. -->
        {#if show.asp}
          {#each airspace as a, i (i)}
            {@const top = Math.min(a.ceiling, yMax)}
            <rect x={x(a.from)} y={y(top)} width={Math.max(1, x(a.to) - x(a.from))} height={Math.max(1, y(a.floor) - y(top))} class="asp {a.cls}" />
            {#if x(a.to) - x(a.from) > 26}
              <text x={x(a.from) + 5} y={y(top) + 13} class="asp-l {a.cls}">{a.cls}</text>
            {/if}
          {/each}
        {/if}

        <!-- Ground. -->
        {#if show.terrain && groundPath}
          <path d={groundPath} class="ground" />
        {/if}

        <!-- Obstacles. -->
        {#if show.obst}
          {#each obstacles as o, i (i)}
            {@const g = elevAt(o.d)}
            <!-- Standing on the drawn ground, which is coarser than the FAA's survey. -->
            {@const base = g ?? Math.max(0, o.amsl - o.agl)}
            <line x1={x(o.d)} x2={x(o.d)} y1={y(base)} y2={y(Math.max(o.amsl, base + o.agl * 0.5))} stroke={obstColor(o)} class="obst" />
          {/each}
        {/if}

        <!-- Planned path. -->
        {#if planLine}
          <polyline points={planLine} class="plan" />
        {/if}

        <!-- Fixes along the top. -->
        {#each fixLabels as f (f.d)}
          <line x1={x(f.d)} x2={x(f.d)} y1={M.t} y2={M.t + 5} class="tick" />
          <text x={x(f.d)} y={M.t - 6} text-anchor="middle" class="fix">{f.ident}</text>
        {/each}
        <text x={M.l} y={M.t - 8} class="apt">{route.origin.icao}</text>
        <text x={M.l + cw} y={M.t - 8} text-anchor="end" class="apt">{route.dest.icao}</text>

        <!-- Distance: flown from the origin, and to go. -->
        {#each xTicks as v (v)}
          <line x1={x(v)} x2={x(v)} y1={M.t + ch} y2={M.t + ch + 5} class="tick" />
          {@const anchor = v === 0 ? "start" : x(total) - x(v) < 14 ? "end" : "middle"}
          <text x={x(v)} y={M.t + ch + 17} text-anchor={anchor} class="axis">{nm(v)}</text>
          <text x={x(v)} y={M.t + ch + 33} text-anchor={anchor} class="axis dim">{nm(Math.max(0, total - v))}</text>
        {/each}
        <text x={M.l - 8} y={M.t + ch + 17} text-anchor="end" class="axis dim">NM</text>
        <text x={M.l - 8} y={M.t + ch + 33} text-anchor="end" class="axis dim">To go</text>

        <!-- Pointer readout. -->
        {#if hoverD != null}
          {@const plan = altAt(path, hoverD)}
          {@const g = elevAt(hoverD)}
          {@const hx = x(hoverD)}
          {@const left = hx > M.l + cw - 170}
          <line x1={hx} x2={hx} y1={M.t} y2={M.t + ch} class="hover" />
          <g transform="translate({left ? hx - 162 : hx + 8} {M.t + 8})">
            <rect width="154" height={Number.isNaN(plan) ? 36 : 52} rx="5" class="tip" />
            <text x="9" y="16" class="tip-t">{nm(hoverD)} nm · {nm(Math.max(0, total - hoverD))} to go</text>
            {#if !Number.isNaN(plan)}<text x="9" y="31" class="tip-t">Plan {fmt(plan)} ft</text>{/if}
            <text x="9" y={Number.isNaN(plan) ? 30 : 46} class="tip-t">Ground {g != null ? `${fmt(g)} ft` : "–"}</text>
          </g>
        {/if}

        <!-- The aircraft, side on, flying to the right. -->
        {#if ac?.near}
          {@const ax = x(Math.min(total, Math.max(0, ac.d)))}
          {@const ay = y(ac.alt)}
          <line x1={ax} x2={ax} y1={ay} y2={M.t + ch} class="ac-guide" />
          <g transform="translate({ax} {ay})" class="ac" class:idle={!ac.tracking}>
            <path d="M -15 -1.5 L -12 -9 L -9.5 -9 L -6.5 -2.5 L 9 -2.5 C 13 -2.5 15.5 -1.2 15.5 0.3 C 15.5 1.8 13 2.6 9 2.6 L -12.5 2.6 C -14.5 2.6 -15.5 1 -15 -1.5 Z M -3.5 2.2 L 2.5 2.2 L -1.5 8 L -4.5 8 Z M -14.5 -0.5 L -9 -0.5 L -12.5 3.8 L -15 3.8 Z" />
          </g>
        {/if}
      </svg>
    {/if}
  </div>
</section>

<style>
  .profile {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 50%;
    display: flex;
    flex-direction: column;
    background: var(--panel, #1c1e21);
    border-top: 1px solid var(--rule, #34373c);
    z-index: 4;
  }
  header {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 16px;
    padding: 8px 12px 6px 16px;
    min-height: 40px;
  }
  h3 {
    margin: 0;
    font: 600 14px "Oswald", "Inter", system-ui, sans-serif;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink, #eceef1);
  }
  .route {
    font: 600 12.5px "IBM Plex Mono", ui-monospace, monospace;
    color: var(--ink-dim, #a8adb4);
  }
  .read {
    display: inline-flex;
    flex-direction: column;
    line-height: 1.15;
  }
  .read .k {
    font-size: 10.5px;
    color: var(--ink-faint, #7d828a);
  }
  .read b {
    font: 600 14px "IBM Plex Mono", ui-monospace, monospace;
    color: #5aa9ff;
  }
  .read b.warn {
    color: #ff6b5e;
  }
  .note {
    font-size: 12px;
    color: var(--amber, #f0b23a);
  }
  .grow {
    flex: 1;
  }
  .chips {
    display: flex;
    gap: 4px;
  }
  .chips button {
    padding: 3px 9px;
    border: 1px solid var(--rule, #34373c);
    border-radius: 999px;
    background: transparent;
    color: var(--ink-dim, #a8adb4);
    font: 600 11.5px "Inter", system-ui, sans-serif;
    cursor: pointer;
  }
  .chips button.on {
    border-color: #4f8dff;
    background: rgba(79, 141, 255, 0.18);
    color: var(--ink, #eceef1);
  }
  .x {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--ink-dim, #a8adb4);
    cursor: pointer;
  }
  .x:hover {
    background: rgba(255, 255, 255, 0.07);
    color: var(--ink, #eceef1);
  }
  .x svg {
    width: 16px;
    height: 16px;
    stroke: currentColor;
    stroke-width: 2;
    fill: none;
  }
  @media (max-width: 640px) {
    header {
      position: relative;
      padding-right: 44px;
    }
    .x {
      position: absolute;
      top: 6px;
      right: 8px;
    }
  }
  .chart {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .chart svg {
    position: absolute;
    inset: 0;
    display: block;
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    touch-action: none;
  }
  .empty {
    margin: 0;
    padding: 24px;
    max-width: 520px;
    color: var(--ink-dim, #a8adb4);
    font-size: 13.5px;
    line-height: 1.5;
  }
  .grid {
    stroke: rgba(255, 255, 255, 0.09);
  }
  .axis {
    fill: #c9cdd3;
    font-size: 11px;
  }
  .axis.dim {
    fill: #80868e;
  }
  .tick {
    stroke: rgba(255, 255, 255, 0.45);
  }
  .fix {
    fill: #c9cdd3;
    font-size: 10px;
  }
  .apt {
    fill: #fff;
    font-size: 12px;
    font-weight: 700;
  }
  .ground {
    fill: #6b6d33;
    stroke: #9a9b52;
    stroke-width: 1.2;
  }
  .obst {
    stroke-width: 1.6;
    opacity: 0.9;
  }
  .plan {
    fill: none;
    stroke: #ff3dd1;
    stroke-width: 3;
    stroke-linejoin: round;
  }
  .asp {
    stroke-width: 1.4;
  }
  .asp.B {
    fill: rgba(79, 141, 255, 0.16);
    stroke: #4f8dff;
  }
  .asp.C {
    fill: rgba(176, 72, 214, 0.26);
    stroke: #c060e0;
  }
  .asp.D {
    fill: rgba(79, 141, 255, 0.08);
    stroke: #4f8dff;
    stroke-dasharray: 5 3;
  }
  .asp-l {
    font-size: 11px;
    font-weight: 700;
  }
  .asp-l.B,
  .asp-l.D {
    fill: #8db8ff;
  }
  .asp-l.C {
    fill: #dc9cf2;
  }
  .hover {
    stroke: rgba(255, 255, 255, 0.5);
    stroke-dasharray: 3 3;
  }
  .tip {
    fill: rgba(17, 18, 20, 0.92);
    stroke: rgba(255, 255, 255, 0.18);
  }
  .tip-t {
    fill: #eceef1;
    font-size: 11px;
  }
  .ac-guide {
    stroke: rgba(240, 178, 58, 0.55);
    stroke-dasharray: 4 3;
  }
  .ac path {
    fill: #f0b23a;
    stroke: #111214;
    stroke-width: 1.2;
    stroke-linejoin: round;
    filter: drop-shadow(0 0 4px rgba(240, 178, 58, 0.5));
  }
  .ac.idle path {
    fill: #f4f5f7;
    stroke: #3b3e44;
    filter: none;
  }
</style>
