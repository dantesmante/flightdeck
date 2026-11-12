<script lang="ts">
  // The flight profile under the map: the planned climb, cruise and
  // descent along the route, the ground beneath it (coarse: a guide, not a
  // survey), FAA obstacles and every airspace the route passes through
  // (FAA in the US, openAIP elsewhere, whether or not the map shows it), and
  // the aircraft, side on, flying left to right. Plain SVG drawn to the
  // panel's size; the data is worked out once per route.
  //
  // It zooms and scrolls sideways only, like a sound editor: the wheel
  // zooms where the pointer is, dragging (or a sideways swipe) scrolls.
  //
  // Glideslope: the final approach on its own, the aircraft on the left, the
  // runway threshold on the right and a 3° path drawn back from it. It opens
  // by itself once the aircraft is within 10 nm of the destination in the
  // air, or with its button.
  import { onDestroy, untrack } from "svelte";
  import { api, type RunwayEnd, type RunwayEnds } from "../api";
  import RunwayPicker from "../plan/RunwayPicker.svelte";
  import { areasNear, crossingsAlong } from "../routeAirspace.svelte";
  import { ASP_KINDS } from "../airspaceLayer";
  import {
    altAt,
    bearingDeg,
    destinationNm,
    gcNm,
    GLIDE_FT_NM,
    groundFt,
    groundZoom,
    headingGap,
    obstaclesAlong,
    plannedPath,
    pointAt,
    project,
    routePoints,
    TCH_FT,
    unionShape,
    type ProfileAirspace,
    type ProfileObstacle,
    type ProfileRoute,
    type RoutePoint,
  } from "./build";

  let {
    route,
    aircraft,
    onclose,
    onrunway,
  }: {
    route: ProfileRoute | null;
    /** The aircraft while the sim is connected; `tracking` while a flight is recorded. */
    aircraft: { lat: number; lon: number; alt: number; onGround: boolean; tracking: boolean } | null;
    onclose: () => void;
    /** The pilot picked the runway to land on. */
    onrunway: (rwy: string) => void;
  } = $props();

  // --- The route -----------------------------------------------------------
  /** The route's identity. The route object itself is rebuilt on every
   * status update (twice a second) even when nothing in it changed, so
   * everything below is worked out from this string, and again only when
   * it changes. */
  const key = $derived(
    route
      ? [route.origin, ...route.fixes, route.dest]
          .map((p) => `${"icao" in p ? p.icao : p.ident}${p.lat.toFixed(4)}${p.lon.toFixed(4)}${Math.round(("alt_ft" in p ? p.alt_ft : 0) || 0)}`)
          .join("|")
      : "",
  );
  const pts = $derived.by(() => {
    void key;
    return untrack(() => (route ? routePoints(route) : []));
  });
  const total = $derived(pts.length ? pts[pts.length - 1].d : 0);
  const cruiseFt = $derived(route?.cruise_ft ?? 0);

  type Ground = { d: number; ft: number | null }[];
  /** Ground under a line of points, sampled `n` times at tile zoom z. */
  function sampleGround(p: RoutePoint[], n: number, z: number, set: (g: Ground) => void): () => void {
    const t = p[p.length - 1].d;
    let gone = false;
    const step = Math.max(t / n, 0.05);
    const ds: number[] = [];
    for (let d = 0; d < t; d += step) ds.push(d);
    ds.push(t);
    groundFt(ds.map((d) => pointAt(p, d)), z).then((ft) => {
      if (!gone) set(ds.map((d, i) => ({ d, ft: ft[i] })));
    });
    return () => (gone = true);
  }

  // $state.raw throughout: these are big, replaced whole, never edited in
  // place, and a deep $state proxy made every read of them slow.
  let ground = $state.raw<Ground>([]);
  let groundBusy = $state(false);
  // The route object is rebuilt on every status update (twice a second)
  // even when the route is the same: work from `key` alone, or all of this
  // ran again each time.
  $effect(() => {
    void key;
    const [p, t] = untrack(() => [pts, total] as const);
    ground = [];
    if (p.length < 2 || t <= 0) return;
    groundBusy = true;
    return sampleGround(p, 400, groundZoom(t), (g) => ((ground = g), (groundBusy = false)));
  });

  // Obstacles and airspace: the same FAA files the map layers use, loaded
  // once (they are cached on the PC).
  /** The FAA obstacle file, parsed once (100,000-odd points): plain, not reactive. */
  let obstFc: GeoJSON.FeatureCollection | null = null;
  let obstReady = $state(false);
  let obstacles = $state.raw<ProfileObstacle[]>([]);
  let airspace = $state.raw<ProfileAirspace[]>([]);
  let faaNote = $state("");
  $effect(() => {
    void key;
    const p = untrack(() => pts);
    const t = untrack(() => total);
    obstacles = [];
    airspace = [];
    if (p.length < 2) return;
    let gone = false;
    (async () => {
      try {
        obstFc ??= JSON.parse(await api.usObstacles()) as GeoJSON.FeatureCollection;
        obstReady = true;
        if (!gone) obstacles = obstaclesAlong(p, obstFc!);
      } catch {
        if (!gone) faaNote = "FAA obstacles not reachable";
      }
      try {
        // Every area along the route, whatever the map shows.
        const r = untrack(() => route);
        const areas = await areasNear(p);
        if (!gone && r)
          airspace = crossingsAlong(r, p, areas, Math.max(0.2, t / 600))
            .map((c) => ({ cls: c.c, name: c.name, from: c.from, to: c.to, floor: c.lo, ceiling: c.hi }))
            // Big boxes first, so smaller ones draw on top of them.
            .sort((a, b) => b.ceiling - b.floor - (a.ceiling - a.floor));
      } catch {
        if (!gone) faaNote = faaNote ? "Obstacles and airspace not reachable" : "Airspace not reachable";
      }
    })();
    return () => {
      gone = true;
    };
  });

  /** Ground at d: the samples are evenly spaced (bar the last), so index straight in. */
  const elevIn = (g: Ground, d: number): number | null => {
    if (!g.length) return null;
    if (g.length === 1) return g[0].ft;
    const step = g[1].d - g[0].d;
    const i = Math.max(0, Math.min(g.length - 1, Math.round((d - g[0].d) / step)));
    return g[i].ft;
  };
  const origElev = $derived(ground[0]?.ft ?? 0);
  const destElev = $derived(ground[ground.length - 1]?.ft ?? 0);
  const path = $derived(pts.length >= 2 ? plannedPath(pts, cruiseFt, origElev, destElev) : []);
  const cruise = $derived(path.length ? Math.max(...path.map((p) => p[1])) : 0);

  /** The aircraft along the route, if it is anywhere near it. */
  const ac = $derived.by(() => {
    if (!aircraft || pts.length < 2) return null;
    const at = project(pts, aircraft.lat, aircraft.lon);
    return { ...at, alt: aircraft.alt, tracking: aircraft.tracking, near: at.off <= 25 };
  });

  // Layers of the chart, switched in its header.
  let show = $state({ terrain: true, obst: true, asp: true });

  // Highest ground or obstacle top (ahead of the aircraft while it flies the
  // route), and the cruise altitude's margin over it.
  const ahead = $derived(!!(ac?.near && ac.tracking));
  const aheadFrom = $derived(ahead ? Math.floor(ac!.d) : 0);
  const highest = $derived.by(() => {
    const from = aheadFrom;
    let h = -Infinity;
    for (const g of ground) if (g.d >= from && g.ft != null && g.ft > h) h = g.ft;
    if (show.obst) for (const o of obstacles) if (o.d >= from && o.amsl > h) h = o.amsl;
    return Number.isFinite(h) ? h : null;
  });

  // --- Glideslope -------------------------------------------------------------
  let mode = $state<"route" | "glide">("route");
  const FINAL_NM = 10;
  let ends = $state.raw<RunwayEnds | null>(null);
  const destIcao = $derived(route?.dest.icao ?? "");
  $effect(() => {
    // Only when the destination changes (not on every status update).
    const icao = destIcao;
    ends = null;
    if (!icao) return;
    let gone = false;
    api.runwayEnds(icao).then(
      (r) => !gone && (ends = r),
      () => {},
    );
    return () => (gone = true);
  });
  const toDest = $derived(aircraft && route ? gcNm(aircraft.lat, aircraft.lon, route.dest.lat, route.dest.lon) : null);
  // Opens by itself once per approach: within 10 nm in the air; ready again
  // after going back beyond 15 nm.
  let autoDone = false;
  $effect(() => {
    if (toDest == null || !aircraft) return;
    if (toDest > 15) autoDone = false;
    else if (!autoDone && toDest <= FINAL_NM && !aircraft.onGround && aircraft.tracking) {
      autoDone = true;
      mode = "glide";
    }
  });

  /** The runway being flown to: the one the pilot picked (in Start a
   * flight or in this header). With none picked when the glideslope opens,
   * the one the aircraft is lined up with is picked once (and saved as the
   * choice, so it never flips between parallels by itself). */
  const destRwy = $derived(route?.dest.runway ?? "");
  const rwy = $derived.by<RunwayEnd | null>(() => {
    if (!route) return null;
    const list = ends?.ends ?? [];
    const picked = destRwy ? list.find((e) => e.ident === destRwy) : undefined;
    if (picked) return picked;
    if (!list.length) {
      // No surveyed runways: the airport itself, approached from where the
      // aircraft is (or along the route's last leg).
      const from = untrack(() => aircraft) ?? pts[pts.length - 2] ?? null;
      const hdg = from ? bearingDeg(from.lat, from.lon, route.dest.lat, route.dest.lon) : 0;
      return { ident: route.dest.icao, lat: route.dest.lat, lon: route.dest.lon, heading_deg: hdg };
    }
    return null;
  });
  /** The runway end the aircraft is best lined up with, right now. */
  function linedUp(): RunwayEnd | null {
    const list = ends?.ends ?? [];
    const ref = aircraft ?? pts[Math.max(0, pts.length - 2)];
    if (!list.length || !ref) return list[0] ?? null;
    const score = (e: RunwayEnd) => {
      const brg = bearingDeg(e.lat, e.lon, ref.lat, ref.lon);
      const angle = headingGap(brg, (e.heading_deg + 180) % 360);
      const dist = gcNm(e.lat, e.lon, ref.lat, ref.lon);
      return angle + Math.abs(Math.sin((angle * Math.PI) / 180) * dist) * 20;
    };
    let best = list[0];
    for (const e of list) if (score(e) < score(best)) best = e;
    return best;
  }
  $effect(() => {
    if (mode !== "glide" || destRwy || !ends?.ends.length) return;
    const pick = untrack(linedUp);
    if (pick) onrunway(pick.ident);
  });
  const thrElev = $derived(ends?.elevation_ft ?? destElev);
  /** The approach shown: R nm of extended centreline out to the threshold.
   * Wider when the aircraft is farther out; kept steady otherwise. */
  let R = $state(10);
  const acToThr = $derived(aircraft && rwy ? gcNm(aircraft.lat, aircraft.lon, rwy.lat, rwy.lon) : null);
  let glideOpen = false;
  $effect(() => {
    if (mode !== "glide") {
      glideOpen = false;
      return;
    }
    const d = acToThr;
    // A little past the aircraft, so it starts near the left edge.
    const want = d == null ? 10 : Math.max(3, Math.min(30, Math.ceil(d * 1.12 + 0.3)));
    if (!glideOpen || d == null || d > R * 0.95 || want < R * 0.5) R = want;
    glideOpen = true;
  });
  // The approach line, rebuilt only when the runway or the window changes:
  // the runway is worked out again on every update, so compare by value.
  const gKey = $derived(rwy ? `${rwy.ident}|${rwy.lat}|${rwy.lon}|${rwy.heading_deg}|${R}` : "");
  const gPts = $derived.by<RoutePoint[]>(() => {
    if (!gKey) return [];
    const [ident, la, lo, hdg, r] = gKey.split("|");
    const [lat0, lon0, h0, R0] = [Number(la), Number(lo), Number(hdg), Number(r)];
    const [lon, lat] = destinationNm(lat0, lon0, (h0 + 180) % 360, R0);
    return [
      { ident: "", lat, lon, d: 0, alt: NaN },
      { ident, lat: lat0, lon: lon0, d: R0, alt: NaN },
    ];
  });
  let gGround = $state.raw<Ground>([]);
  $effect(() => {
    void gKey;
    const p = gPts;
    gGround = [];
    if (mode !== "glide" || p.length < 2) return;
    return sampleGround(p, 200, R <= 12 ? 12 : 11, (g) => (gGround = g));
  });
  // Obstacles near the approach: scanned once per approach line, not per update.
  const gObst = $derived(mode === "glide" && obstReady && obstFc && gPts.length ? obstaclesAlong(gPts, obstFc, 1) : []);
  /** The 3° path: threshold crossing height, rising 318 ft per nm. */
  const glideAt = (d: number) => thrElev + TCH_FT + (R - d) * GLIDE_FT_NM;
  const gAc = $derived.by(() => {
    if (!aircraft || gPts.length < 2 || acToThr == null) return null;
    const at = project(gPts, aircraft.lat, aircraft.lon);
    return { d: R - acToThr, off: at.off, alt: aircraft.alt, tracking: aircraft.tracking, near: at.off <= 3 && acToThr <= R * 1.05 };
  });
  const deviation = $derived(gAc?.near ? gAc.alt - glideAt(gAc.d) : null);

  // --- What's on the chart, for either view -----------------------------------
  const glide = $derived(mode === "glide" && gPts.length > 1);
  const span = $derived(glide ? R : total);
  const G = $derived(glide ? gGround : ground);
  const O = $derived(glide ? gObst : obstacles);
  const plan = $derived<[number, number][]>(glide ? [[0, glideAt(0)], [R, thrElev + TCH_FT]] : path);
  const A = $derived(glide ? gAc : ac);
  const elevAt = (d: number) => elevIn(G, d);

  // --- Sideways zoom and scroll ----------------------------------------------
  let view = $state({ a: 0, b: 1 });
  let viewFor = "";
  $effect(() => {
    // A new route or view: show all of it.
    const k = `${glide ? "g" + gKey : "r" + key}`;
    if (k !== viewFor) {
      viewFor = k;
      view = { a: 0, b: span || 1 };
    } else if (view.b > span || view.a < 0) view = { a: 0, b: span || 1 };
  });
  const zoomed = $derived(span > 0 && view.b - view.a < span * 0.999);
  const minSpan = $derived(Math.min(span, glide ? 0.5 : 2));
  function setView(a: number, b: number) {
    let w = Math.max(minSpan, Math.min(span, b - a));
    a = Math.max(0, Math.min(span - w, a));
    if (!Number.isFinite(a) || w <= 0) return;
    view = { a, b: a + w };
  }
  function zoomAt(d: number, factor: number) {
    const w = (view.b - view.a) * factor;
    const t = (d - view.a) / (view.b - view.a);
    setView(d - t * w, d - t * w + w);
  }
  const zoomBy = (factor: number) => zoomAt((view.a + view.b) / 2, factor);
  const fit = () => setView(0, span);

  // --- Chart -------------------------------------------------------------------
  let w = $state(0);
  let h = $state(0);
  const M = $derived({ l: 58, r: 18, t: 30, b: zoomed ? 52 : 42 });
  const cw = $derived(Math.max(10, w - M.l - M.r));
  const ch = $derived(Math.max(10, h - M.t - M.b));
  const yMin = $derived.by(() => {
    if (!glide) return 0;
    let lo = thrElev;
    for (const g of G) if (g.ft != null && g.ft < lo) lo = g.ft;
    return Math.max(0, Math.floor((lo - 150) / 100) * 100);
  });
  const acTop = $derived(A?.near ? Math.ceil(A.alt / 500) * 500 : 0);
  const yMax = $derived.by(() => {
    let top: number;
    if (glide) {
      top = Math.max(glideAt(view.a), acTop, thrElev + 500);
      for (const o of O) if (o.d >= view.a) top = Math.max(top, o.amsl);
    } else {
      top = Math.max(cruise, highest ?? 0, acTop, 1000);
      for (const a of airspace) if (show.asp) top = Math.max(top, Math.min(a.ceiling, cruise * 1.4 || a.ceiling));
    }
    const step = niceStep((top - yMin) * 1.15, 5);
    return yMin + Math.ceil(((top - yMin) * 1.12) / step) * step;
  });
  const x = (d: number) => M.l + ((d - view.a) / (view.b - view.a)) * cw;
  const y = (ft: number) => M.t + (1 - (Math.max(yMin, ft) - yMin) / (yMax - yMin)) * ch;
  const dAt = (px: number) => view.a + ((px - M.l) / cw) * (view.b - view.a);

  function niceStep(range: number, n: number): number {
    const raw = range / n;
    for (const s of [0.1, 0.2, 0.25, 0.5, 1, 2, 5, 10, 20, 25, 50, 100, 200, 250, 500, 1000, 2000, 2500, 5000, 10000, 20000, 25000, 50000]) if (s >= raw) return s;
    return 100000;
  }
  const yTicks = $derived.by(() => {
    const step = niceStep(yMax - yMin, Math.max(2, Math.floor(ch / 42)));
    const out: number[] = [];
    for (let v = Math.ceil(yMin / step) * step; v <= yMax + 1; v += step) out.push(v);
    return out;
  });
  const xTicks = $derived.by(() => {
    if (span <= 0) return [];
    const step = niceStep(view.b - view.a, Math.max(2, Math.floor(cw / 70)));
    const out: number[] = [];
    for (let v = Math.ceil((view.a - 1e-9) / step) * step; v <= view.b + 1e-6; v += step) out.push(Math.round(v * 1000) / 1000);
    return out;
  });
  const fl = (ft: number) => (glide ? fmt(ft) : `FL${String(Math.round(ft / 100)).padStart(3, "0")}`);
  const fmt = (n: number) => Math.round(n).toLocaleString("en-US");
  const nm = (n: number) => (n < 10 && Math.abs(n % 1) > 0.01 ? n.toFixed(1) : String(Math.round(n)));

  const groundPath = $derived.by(() => {
    const g = G.filter((p) => p.ft != null && p.d >= view.a - span / 200 && p.d <= view.b + span / 200) as { d: number; ft: number }[];
    if (g.length < 2) return "";
    let s = `M ${x(g[0].d)} ${y(yMin)}`;
    for (const p of g) s += ` L ${x(p.d)} ${y(p.ft)}`;
    return s + ` L ${x(g[g.length - 1].d)} ${y(yMin)} Z`;
  });
  const planLine = $derived(plan.map(([d, a]) => `${x(d)},${y(a)}`).join(" "));

  /** Airspace by class, each as one merged shape. */
  const aspShapes = $derived.by(() => {
    if (glide || !show.asp) return [];
    const kinds = [...new Set(airspace.map((a) => a.cls))];
    return kinds.map((cls) => ({
      cls,
      color: ASP_KINDS[cls]?.color ?? "#4a8ce8",
      dash: !!ASP_KINDS[cls]?.dash,
      ...unionShape(
        airspace
          .filter((a) => a.cls === cls && a.to > view.a && a.from < view.b)
          .map((a) => ({ x0: x(a.from), x1: x(a.to), y0: y(Math.min(a.ceiling, yMax)), y1: y(a.floor) })),
      ),
    }));
  });

  /** Fix names along the top, leaving out those that would overlap. */
  const fixLabels = $derived.by(() => {
    if (glide) return [];
    const out: { d: number; ident: string }[] = [];
    let last = M.l + (view.a === 0 ? 40 : 0);
    const end = M.l + cw - (view.b >= span - 1e-6 ? 44 : 0);
    for (const p of pts.slice(1, -1)) {
      const px = x(p.d);
      if (px < M.l || px - last < 44 || px > end) continue;
      out.push({ d: p.d, ident: p.ident });
      last = px;
    }
    return out;
  });

  /** Obstacles in view, the tallest per pixel column, standing on the drawn
   * ground (coarser than the FAA's survey). */
  const obstLines = $derived.by(() => {
    if (!show.obst) return [];
    const cols = new Map<number, ProfileObstacle>();
    for (const o of O) {
      if (o.d < view.a || o.d > view.b) continue;
      const c = Math.round(x(o.d));
      const was = cols.get(c);
      if (!was || o.amsl > was.amsl) cols.set(c, o);
    }
    return [...cols].map(([c, o]) => {
      const base = elevAt(o.d) ?? Math.max(0, o.amsl - o.agl);
      return { x: c, y1: y(base), y2: y(Math.max(o.amsl, base + o.agl * 0.5)), c: obstColor(o) };
    });
  });

  function obstColor(o: ProfileObstacle): string {
    const p = altAt(plan, o.d);
    if (Number.isNaN(p)) return "#ffffff";
    return o.amsl >= p - 100 ? "#ff3b30" : o.amsl >= p - 1000 ? "#ecc428" : "#ffffff";
  }

  // --- Pointer: hover readout, drag to scroll, wheel and pinch to zoom -------
  let svgEl = $state<SVGSVGElement | null>(null);
  let hoverD = $state<number | null>(null);
  const touches = new Map<number, number>();
  let drag: { x: number; a: number; b: number } | null = null;
  let pinch: { dist: number; mid: number; a: number; b: number } | null = null;
  const localX = (e: { clientX: number }) => e.clientX - (svgEl?.getBoundingClientRect().left ?? 0);
  function down(e: PointerEvent) {
    touches.set(e.pointerId, localX(e));
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
    if (touches.size === 1) drag = { x: localX(e), a: view.a, b: view.b };
    else if (touches.size === 2) {
      const [p, q] = [...touches.values()];
      pinch = { dist: Math.abs(p - q) || 1, mid: dAt((p + q) / 2), a: view.a, b: view.b };
      drag = null;
    }
  }
  function move(e: PointerEvent) {
    const px = localX(e);
    if (touches.has(e.pointerId)) touches.set(e.pointerId, px);
    if (pinch && touches.size >= 2) {
      const [p, q] = [...touches.values()];
      const f = pinch.dist / (Math.abs(p - q) || 1);
      const wNew = (pinch.b - pinch.a) * f;
      const t = ((p + q) / 2 - M.l) / cw;
      setView(pinch.mid - t * wNew, pinch.mid - t * wNew + wNew);
      hoverD = null;
      return;
    }
    if (drag) {
      const dd = ((px - drag.x) / cw) * (drag.b - drag.a);
      if (Math.abs(px - drag.x) > 2) setView(drag.a - dd, drag.b - dd);
    }
    hoverD = e.pointerType === "touch" && drag ? null : px < M.l || px > M.l + cw || span <= 0 ? null : dAt(px);
  }
  function up(e: PointerEvent) {
    touches.delete(e.pointerId);
    if (touches.size < 2) pinch = null;
    if (touches.size === 0) drag = null;
  }
  // The wheel needs a listener that may cancel the page's own scrolling.
  $effect(() => {
    const el = svgEl;
    if (!el) return;
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      const scrollX = e.shiftKey ? e.deltaY : e.deltaX;
      if (Math.abs(scrollX) > Math.abs(e.deltaY) || e.shiftKey) {
        const dd = (scrollX / cw) * (view.b - view.a);
        setView(view.a + dd, view.b + dd);
      } else zoomAt(dAt(localX(e)), Math.exp(e.deltaY * (e.deltaMode === 1 ? 0.05 : 0.0015)));
    };
    el.addEventListener("wheel", wheel, { passive: false });
    return () => el.removeEventListener("wheel", wheel);
  });
  // The scroll bar under the chart, while zoomed in.
  let barDrag: { x: number; a: number } | null = null;
  function barDown(e: PointerEvent) {
    e.stopPropagation();
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
    barDrag = { x: e.clientX, a: view.a };
  }
  function barMove(e: PointerEvent) {
    if (!barDrag) return;
    e.stopPropagation();
    const dd = ((e.clientX - barDrag.x) / cw) * span;
    setView(barDrag.a + dd, barDrag.a + dd + (view.b - view.a));
  }
  const barUp = () => (barDrag = null);
  onDestroy(() => touches.clear());
</script>

<section class="profile" aria-label="Flight profile">
  <header>
    <h3>{glide ? "Glideslope" : "Flight profile"}</h3>
    {#if route}
      {#if glide}
        <span class="route">{route.dest.icao} · 3°</span>
        <RunwayPicker compact icao={destIcao} value={destRwy} onchange={(r) => r && onrunway(r)} />
        <span class="read">
          <span class="k">To threshold</span>
          <b>{acToThr != null ? `${nm(acToThr)} nm` : "–"}</b>
        </span>
        <span class="read" title="Height above (+) or below (−) the 3° glideslope">
          <span class="k">Glideslope</span>
          <b class:warn={deviation != null && Math.abs(deviation) > 300}>
            {deviation == null ? "–" : Math.abs(deviation) < 50 ? "On path" : `${deviation > 0 ? "+" : "−"}${fmt(Math.abs(deviation))} ft`}
          </b>
        </span>
        {#if gAc && !gAc.near && acToThr != null && acToThr <= R * 1.05}<span class="note">{nm(gAc.off)} nm off the centreline</span>{/if}
      {:else}
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
      {/if}
      {#if faaNote}<span class="note">{faaNote}</span>{/if}
    {/if}
    <span class="grow"></span>
    {#if route}
      <div class="chips" role="group" aria-label="Zoom">
        <button class="sq" aria-label="Zoom out" onclick={() => zoomBy(1.6)} disabled={!zoomed}>−</button>
        <button class="sq" aria-label="Zoom in" onclick={() => zoomBy(1 / 1.6)} disabled={view.b - view.a <= minSpan + 1e-6}>+</button>
        <button onclick={fit} disabled={!zoomed}>Fit</button>
      </div>
      <div class="chips" role="group" aria-label="Show in the profile">
        <button aria-pressed={show.terrain} class:on={show.terrain} onclick={() => (show.terrain = !show.terrain)}>Terrain</button>
        <button aria-pressed={show.obst} class:on={show.obst} onclick={() => (show.obst = !show.obst)} title="FAA obstacles near the route (US only)">Obstacles</button>
        {#if !glide}
          <button aria-pressed={show.asp} class:on={show.asp} onclick={() => (show.asp = !show.asp)} title="Airspace along the route, whether or not the map shows it">Airspace</button>
        {/if}
      </div>
      <button class="glide-btn" class:on={glide} aria-pressed={glide} onclick={() => (mode = glide ? "route" : "glide")} title="The final approach with a 3° glideslope">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 6l18 12" stroke-dasharray="3 2" /><path d="M15 19h7" /></svg>
        Glideslope
      </button>
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
      <svg
        bind:this={svgEl}
        width={w}
        height={h}
        role="img"
        aria-label={glide ? "Final approach with a 3° glideslope" : "Vertical profile of the route"}
        class:grab={zoomed}
        onpointerdown={down}
        onpointermove={move}
        onpointerup={up}
        onpointercancel={up}
        onpointerleave={() => (hoverD = null)}
      >
        <defs>
          <linearGradient id="fp-sky" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stop-color="#16244a" />
            <stop offset="1" stop-color="#22396a" />
          </linearGradient>
          <clipPath id="fp-plot"><rect x={M.l} y={M.t - 14} width={cw} height={ch + 14} /></clipPath>
        </defs>
        <rect x={M.l} y={M.t} width={cw} height={ch} fill="url(#fp-sky)" />

        <!-- Altitude grid. -->
        {#each yTicks as v (v)}
          <line x1={M.l} x2={M.l + cw} y1={y(v)} y2={y(v)} class="grid" />
          <text x={M.l - 8} y={y(v) + 4} text-anchor="end" class="axis">{fl(v)}</text>
        {/each}

        <g clip-path="url(#fp-plot)">
          <!-- Airspace: one shape per class. -->
          {#each aspShapes as s (s.cls)}
            {#if s.fill}
              <path d={s.fill} class="asp-fill" style:fill={s.color} />
              <path d={s.edge} class="asp-edge" class:dash={s.dash} style:stroke={s.color} />
              {#each s.labels as l, i (i)}
                <text x={l.x + 5} y={l.y + 13} class="asp-l" style:fill={s.color}>{s.cls}</text>
              {/each}
            {/if}
          {/each}

          <!-- Ground. -->
          {#if show.terrain && groundPath}
            <path d={groundPath} class="ground" />
          {/if}

          <!-- Obstacles. -->
          {#if show.obst}
            {#each obstLines as l, i (i)}
              <line x1={l.x} x2={l.x} y1={l.y1} y2={l.y2} stroke={l.c} class="obst" />
            {/each}
          {/if}

          <!-- Planned path, or the 3° glideslope (dashed) back from the threshold. -->
          {#if planLine}
            <polyline points={planLine} class="plan" class:gs={glide} />
          {/if}
          {#if glide}
            <!-- The runway, at the right edge. -->
            <line x1={x(R) - 3} x2={x(R) + 30} y1={y(thrElev)} y2={y(thrElev)} class="rwy" />
          {/if}

          <!-- Fixes along the top. -->
          {#each fixLabels as f (f.d)}
            <line x1={x(f.d)} x2={x(f.d)} y1={M.t} y2={M.t + 5} class="tick" />
            <text x={x(f.d)} y={M.t - 6} text-anchor="middle" class="fix">{f.ident}</text>
          {/each}
        </g>
        {#if glide}
          {#if view.b >= span - 1e-6}<text x={M.l + cw} y={M.t - 8} text-anchor="end" class="apt">{rwy && rwy.ident !== route.dest.icao ? `RWY ${rwy.ident}` : route.dest.icao}</text>{/if}
        {:else}
          {#if view.a <= 1e-6}<text x={M.l} y={M.t - 8} class="apt">{route.origin.icao}</text>{/if}
          {#if view.b >= span - 1e-6}<text x={M.l + cw} y={M.t - 8} text-anchor="end" class="apt">{route.dest.icao}</text>{/if}
        {/if}

        <!-- Distance: flown from the origin and to go (glideslope: to the threshold). -->
        {#each xTicks as v (v)}
          {@const anchor = x(v) - M.l < 14 ? "start" : M.l + cw - x(v) < 14 ? "end" : "middle"}
          <line x1={x(v)} x2={x(v)} y1={M.t + ch} y2={M.t + ch + 5} class="tick" />
          {#if glide}
            <text x={x(v)} y={M.t + ch + 17} text-anchor={anchor} class="axis">{nm(Math.max(0, span - v))}</text>
          {:else}
            <text x={x(v)} y={M.t + ch + 17} text-anchor={anchor} class="axis">{nm(v)}</text>
            <text x={x(v)} y={M.t + ch + 33} text-anchor={anchor} class="axis dim">{nm(Math.max(0, span - v))}</text>
          {/if}
        {/each}
        {#if glide}
          <text x={M.l - 8} y={M.t + ch + 17} text-anchor="end" class="axis dim">To go</text>
        {:else}
          <text x={M.l - 8} y={M.t + ch + 17} text-anchor="end" class="axis dim">NM</text>
          <text x={M.l - 8} y={M.t + ch + 33} text-anchor="end" class="axis dim">To go</text>
        {/if}

        <!-- Scroll bar while zoomed in. -->
        {#if zoomed}
          <rect x={M.l} y={h - 10} width={cw} height="6" rx="3" class="bar" />
          <rect
            x={M.l + (view.a / span) * cw}
            y={h - 11}
            width={Math.max(14, ((view.b - view.a) / span) * cw)}
            height="8"
            rx="4"
            class="thumb"
            role="scrollbar"
            aria-controls="fp-plot"
            aria-valuenow={Math.round(view.a)}
            tabindex="-1"
            onpointerdown={barDown}
            onpointermove={barMove}
            onpointerup={barUp}
            onpointercancel={barUp}
          />
        {/if}

        <!-- Pointer readout. -->
        {#if hoverD != null}
          {@const p = altAt(plan, hoverD)}
          {@const g = elevAt(hoverD)}
          {@const hx = x(hoverD)}
          {@const left = hx > M.l + cw - 170}
          <line x1={hx} x2={hx} y1={M.t} y2={M.t + ch} class="hover" />
          <g transform="translate({left ? hx - 162 : hx + 8} {M.t + 8})">
            <rect width="154" height={Number.isNaN(p) ? 36 : 52} rx="5" class="tip" />
            <text x="9" y="16" class="tip-t">{glide ? `${nm(span - hoverD)} nm to threshold` : `${nm(hoverD)} nm · ${nm(Math.max(0, span - hoverD))} to go`}</text>
            {#if !Number.isNaN(p)}<text x="9" y="31" class="tip-t">{glide ? "Glideslope" : "Plan"} {fmt(p)} ft</text>{/if}
            <text x="9" y={Number.isNaN(p) ? 30 : 46} class="tip-t">Ground {g != null ? `${fmt(g)} ft` : "–"}</text>
          </g>
        {/if}

        <!-- The aircraft, side on, flying to the right. -->
        {#if A?.near && A.d >= view.a - 0.01 && A.d <= view.b + 0.01}
          {@const ax = x(Math.min(span, Math.max(0, A.d)))}
          {@const ay = y(A.alt)}
          <line x1={ax} x2={ax} y1={ay} y2={M.t + ch} class="ac-guide" />
          <g transform="translate({ax} {ay})" class="ac" class:idle={!A.tracking}>
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
  .chips button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .chips button.sq {
    width: 26px;
    padding: 3px 0;
    font-size: 14px;
    line-height: 1;
  }
  .glide-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 11px;
    border: 1px solid var(--rule, #34373c);
    border-radius: 999px;
    background: transparent;
    color: var(--ink, #eceef1);
    font: 600 12px "Inter", system-ui, sans-serif;
    cursor: pointer;
  }
  .glide-btn svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }
  .glide-btn.on {
    border-color: var(--amber, #f0b23a);
    background: rgba(240, 178, 58, 0.16);
    color: var(--amber, #f0b23a);
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
  /* The 3° glideslope, projected back from the runway. */
  .plan.gs {
    stroke: #f2f4f7;
    stroke-width: 2;
    stroke-dasharray: 9 6;
  }
  .rwy {
    stroke: #e8eaed;
    stroke-width: 5;
  }
  .bar {
    fill: rgba(255, 255, 255, 0.08);
  }
  .thumb {
    fill: rgba(255, 255, 255, 0.38);
    cursor: grab;
  }
  svg.grab {
    cursor: grab;
  }
  svg.grab:active {
    cursor: grabbing;
  }
  /* Airspace: one fill and one outline per kind, in the map's colours. */
  .asp-fill {
    fill-opacity: 0.14;
  }
  .asp-edge {
    fill: none;
    stroke-width: 1.5;
    stroke-linecap: square;
  }
  .asp-edge.dash {
    stroke-dasharray: 5 3;
  }
  .asp-l {
    font-size: 11px;
    font-weight: 700;
    filter: brightness(1.35);
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
