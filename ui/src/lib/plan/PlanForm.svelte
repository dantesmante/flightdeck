<script lang="ts">
  // A simple flight plan: callsign, aircraft, origin, destination and
  // alternate, typed in or imported from SimBrief. Used to start a flight,
  import RunwayPicker from "./RunwayPicker.svelte";
  // and mid-flight to add a plan.
  import { onDestroy, onMount, untrack } from "svelte";
  import { draft, drawing, pick, planEdit } from "../flyStore.svelte";
  import { routeAsp } from "../routeAirspace.svelte";
  import { ASP_KINDS } from "../airspaceLayer";
  import { parsePln } from "./pln";
  import { estimate, fuelText, perfFor, type AircraftHint } from "./perf";
  import { decodeGrid, levelFor, sample, type Field } from "../wind/wind";
  import type { AircraftInfo, Telemetry } from "../api";
  import { briefing, openBriefing } from "../briefingStore.svelte";
  import { api, inTauri, type FlightPlan, type HangarItem, type PlanFix, type PlnRoute } from "../api";
  import { modelName } from "../hangar/names";
  import { distance, prefs } from "../prefs.svelte";
  import AirportField, { type Spot } from "./AirportField.svelte";

  interface Props {
    /** start: arm a new flight. attach: add a plan to the flight under way.
     * edit: change the plan in use, at any stage of the flight. */
    mode: "start" | "attach" | "edit";
    /** The plan being edited (edit). */
    initial?: FlightPlan | null;
    /** Preselected hangar aircraft ("Fly this aircraft"). */
    aircraft?: HangarItem | null;
    onsettings: () => void;
    ondone: () => void;
  }
  let { mode, aircraft = null, initial = null, onsettings, ondone }: Props = $props();

  let callsign = $state("");
  let flightNo = $state("");
  let origin = $state<Spot | null>(null);
  let dest = $state<Spot | null>(null);
  /** The runway to land on at the destination (SimBrief's, or picked). */
  let landRwy = $state("");
  // A new destination: SimBrief's runway when it is SimBrief's route, else none yet.
  let rwyFor = "";
  $effect(() => {
    const d = dest?.icao ?? "";
    if (d === rwyFor) return;
    rwyFor = d;
    landRwy = untrack(() => (base && d === base.destination.icao ? base.destination.runway : ""));
  });
  let altn = $state<Spot | null>(null);
  let fleet = $state<HangarItem[]>([]);
  let aircraftId = $state(-1);
  /** The SimBrief plan the form was filled from, if any. */
  let base = $state<FlightPlan | null>(null);
  let importing = $state(false);
  /** The latest SimBrief plan, shown before it is imported. */
  let latest = $state<FlightPlan | null>(null);
  let looking = $state(false);
  let sbError = $state<string | null>(null);
  /** The origin was filled in from where the aircraft is, not typed. */
  let originAuto = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);

  // A route drawn on the map: the points between origin and destination.
  let drawn = $state<PlanFix[] | null>(null);
  let drawnFor = { o: "", d: "" };
  let rules = $state<"VFR" | "IFR">("VFR");
  /** Cruise altitude typed in (ft), for a route without SimBrief's own:
   * 5,000 ft unless changed. */
  let cruiseFt = $state<number | null>(5000);
  let saved = $state<string | null>(null);
  let saving = $state(false);
  // Another origin or destination than the route was drawn for: it no
  // longer fits.
  $effect(() => {
    if (drawn && (origin?.icao !== drawnFor.o || dest?.icao !== drawnFor.d)) untrack(() => (drawn = null));
  });

  // Preview the route on the map while a flight is being set up.
  $effect(() => {
    if (mode !== "start") return;
    draft.origin = origin;
    draft.dest = dest;
    // The route itself: drawn, or SimBrief's while its airports are kept.
    draft.fixes = routeFixes;
    // The flight profile's cruise: SimBrief's, or the one typed in.
    draft.cruise_ft = cruiseNow;
    draft.runway = landRwy;
  });
  // The route reshaped on the map (a point dragged, put in or taken out):
  // it becomes this form's own route, whichever way it was entered.
  $effect(() => {
    const fx = planEdit.fixes;
    if (mode === "attach" || !fx) return;
    untrack(() => {
      planEdit.fixes = null;
      if (!origin || !dest) return;
      drawn = renumber(fx);
      drawnFor = { o: origin.icao, d: dest.icao };
      saved = null;
    });
  });
  /** User waypoints numbered in order: WP1, WP2, ... */
  function renumber(fx: PlanFix[]): PlanFix[] {
    let n = 0;
    return fx.map((f) => (f.kind === "ltlg" && (!f.ident || /^WP\d+$/.test(f.ident)) ? { ...f, ident: `WP${++n}` } : f));
  }
  // An airport sent from the map's right-click menu.
  $effect(() => {
    const spot = pick.spot;
    const which = pick.which;
    if (mode !== "start" || !spot || !which) return;
    untrack(() => {
      pick.spot = null;
      pick.which = null;
      // While drawing: a new start, or the end of the route.
      if (drawing.stage === "route") {
        if (which === "origin") {
          drawing.origin = spot;
          return;
        }
        const last = drawing.pts[drawing.pts.length - 1];
        if (!last || last.icao !== spot.icao) drawing.pts = [...drawing.pts, { icao: spot.icao, name: spot.name, lat: spot.lat, lon: spot.lon }];
        finishDrawing();
        return;
      }
      if (which === "origin") {
        origin = spot;
        originAuto = false;
      } else dest = spot;
      // Origin from the map with nowhere to go yet: straight on to drawing
      // the route, so (right-)clicks on the map add its waypoints.
      if (which === "origin" && !dest && drawing.stage === "off") startDrawing();
    });
  });
  onDestroy(() => {
    if (mode !== "start") return;
    draft.origin = null;
    draft.dest = null;
    draft.fixes = [];
    draft.cruise_ft = 0;
    draft.runway = "";
    drawing.stage = "off";
  });

  // --- Draw route ------------------------------------------------------------
  /** Draw a route, or (`edit`) carry on editing the one drawn. With the
   * origin known, straight to adding points. */
  function startDrawing(edit = false) {
    drawing.origin = origin;
    drawing.pts =
      edit && drawn && dest
        ? [
            ...drawn.map((f) => ({ icao: f.kind === "apt" ? f.ident : "", name: "", lat: f.lat, lon: f.lon, alt: f.stage === "USR" ? f.alt_ft : null })),
            { icao: dest.icao, name: dest.name, lat: dest.lat, lon: dest.lon },
          ]
        : [];
    drawing.note = "";
    drawing.stage = origin ? "route" : "origin";
    altEdit = false;
    saved = null;
  }
  /** The altitudes list (Altitudes button) while drawing. */
  let altEdit = $state(false);
  /** The cruise altitude in use: SimBrief's for its own route, else the one typed. */
  const cruiseNow = $derived(!!base && base.source === "simbrief" && origin?.icao === base.origin.icao && dest?.icao === base.destination.icao ? base.cruise_ft : (cruiseFt ?? 0));
  /** The route's points with their altitudes: their own (set by the pilot,
   * or SimBrief's climb and descent), else the cruise. */
  const drawnFixes = $derived(drawn ? drawn.map((f) => ({ ...f, alt_ft: f.stage === "CRZ" ? cruiseNow : f.alt_ft })) : null);
  /** The route between origin and destination: this form's own, or SimBrief's while its airports are kept. */
  const routeFixes = $derived(drawnFixes ?? (!!base && origin?.icao === base.origin.icao && dest?.icao === base.destination.icao ? base.fixes : []));
  function cancelDrawing() {
    drawing.stage = "off";
    drawing.pts = [];
  }
  const lastIsAirport = $derived(drawing.pts.length > 0 && !!drawing.pts[drawing.pts.length - 1].icao);
  const drawnNm = $derived.by(() => {
    if (!drawing.origin) return 0;
    let t = 0;
    let prev: { lat: number; lon: number } = drawing.origin;
    for (const p of drawing.pts) {
      t += gc(prev as Spot, p as Spot);
      prev = p;
    }
    return t;
  });
  function finishDrawing() {
    if (!drawing.origin || !lastIsAirport) return;
    const end = drawing.pts[drawing.pts.length - 1];
    let n = 0;
    const fixes: PlanFix[] = drawing.pts.slice(0, -1).map((p) => ({
      ident: p.icao || `WP${++n}`,
      kind: p.icao ? "apt" : "ltlg",
      lat: p.lat,
      lon: p.lon,
      alt_ft: p.alt ?? cruiseFt ?? 0,
      // "USR": an altitude set for this point; else it follows the cruise.
      stage: p.alt ? "USR" : "CRZ",
      procedure: false,
    }));
    origin = { ...drawing.origin };
    originAuto = false;
    dest = { icao: end.icao, name: end.name, lat: end.lat, lon: end.lon };
    drawnFor = { o: origin.icao, d: dest.icao };
    drawn = fixes;
    base = null;
    drawing.stage = "off";
    drawing.pts = [];
  }
  /** The drawn route as the sim's flight plan. */
  function plnRoute(): PlnRoute | null {
    if (!origin || !dest || !drawn) return null;
    const pt = (s: Spot) => ({ ident: s.icao, name: s.name, lat: s.lat, lon: s.lon, airport: true });
    return {
      rules,
      // The sim wants one: a usual light-aircraft altitude if none was typed.
      cruise_ft: cruiseFt ?? 5500,
      origin: pt(origin),
      destination: pt(dest),
      waypoints: drawn.map((f) => ({ ident: f.ident, name: "", lat: f.lat, lon: f.lon, airport: f.kind === "apt", alt_ft: f.stage === "USR" ? f.alt_ft : null })),
    };
  }
  async function savePln() {
    const r = plnRoute();
    if (!r) return;
    saving = true;
    error = null;
    try {
      if (inTauri) {
        const path = await api.savePln(r);
        if (path) saved = path;
      } else {
        // On a phone or tablet: the browser downloads it.
        const [name, text] = await api.plnText(r);
        const url = URL.createObjectURL(new Blob([text], { type: "application/xml" }));
        const a = document.createElement("a");
        a.href = url;
        a.download = name;
        a.click();
        setTimeout(() => URL.revokeObjectURL(url), 5000);
        saved = name;
      }
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  const hasId = $derived(!!prefs.s.simbrief_id?.trim());

  onMount(async () => {
    if (aircraft) aircraftId = aircraft.id;
    if (mode === "edit" && initial) {
      await fillFrom(initial);
      lookSimbrief();
    }
    if (mode === "start") {
      lookSimbrief();
      const h = (await api.hangar().catch(() => [])) ?? [];
      fleet = h.sort((a, b) => Number(b.favorite) - Number(a.favorite) || b.last_flight_ms - a.last_flight_ms);
      await fillOrigin();
    }
  });

  const chosen = $derived(fleet.find((a) => a.id === aircraftId) ?? (aircraft?.id === aircraftId ? aircraft : null));

  /**
   * The origin is where the aircraft is: where the sim has it sitting on the
   * ground if a sim is connected, otherwise where the chosen hangar aircraft
   * was parked after its last flight.
   */
  async function fillOrigin() {
    if (origin && !originAuto) return;
    const st = await api.status().catch(() => null);
    const t = st?.link === "connected" && !st.live ? st.telemetry : null;
    let spot: Spot | null = null;
    if (t && t.on_ground && (Math.abs(t.lat) > 0.01 || Math.abs(t.lon) > 0.01)) {
      const hit = await api.airportAt(t.lat, t.lon).catch(() => null);
      if (hit) spot = { icao: hit.icao, name: hit.name, lat: hit.lat, lon: hit.lon };
    }
    const ac = chosen;
    if (!spot && ac?.last_icao && ac.last_lat != null && ac.last_lon != null) {
      spot = { icao: ac.last_icao, name: ac.last_name ?? "", lat: ac.last_lat, lon: ac.last_lon };
    }
    if (spot && (!origin || originAuto)) {
      origin = spot;
      originAuto = true;
    }
  }

  // Choosing another aircraft moves an origin that came from the last one.
  let lastAircraft = -2;
  $effect(() => {
    const id = aircraftId;
    if (lastAircraft !== -2 && id !== lastAircraft && (originAuto || !origin) && mode === "start") fillOrigin();
    lastAircraft = id;
  });

  /** Look up the latest SimBrief plan to show it before it is imported. */
  async function lookSimbrief() {
    if (!hasId) return;
    looking = true;
    sbError = null;
    try {
      latest = await api.simbriefLatest();
    } catch (e) {
      sbError = String(e);
    } finally {
      looking = false;
    }
  }

  async function importSimbrief() {
    importing = true;
    error = null;
    try {
      const p = latest ?? (await api.simbriefLatest());
      latest = p;
      base = p;
      drawn = null;
      originAuto = false;
      callsign = p.callsign;
      flightNo = `${p.airline}${p.flight_number}`;
      origin = { icao: p.origin.icao, name: p.origin.name, lat: p.origin.lat, lon: p.origin.lon };
      dest = { icao: p.destination.icao, name: p.destination.name, lat: p.destination.lat, lon: p.destination.lon };
      altn = null;
      if (p.alternate) {
        const hit = ((await api.searchAirports(p.alternate).catch(() => [])) ?? []).find((h) => h.icao === p.alternate);
        altn = hit ? { icao: hit.icao, name: hit.name, lat: hit.lat, lon: hit.lon } : { icao: p.alternate, name: "", lat: 0, lon: 0 };
      }
    } catch (e) {
      error = String(e);
    } finally {
      importing = false;
    }
  }

  /** Editing: the form filled from the plan in use. */
  async function fillFrom(p: FlightPlan) {
    base = p;
    callsign = p.callsign;
    flightNo = `${p.airline}${p.flight_number}`;
    originAuto = false;
    origin = { icao: p.origin.icao, name: p.origin.name, lat: p.origin.lat, lon: p.origin.lon };
    dest = { icao: p.destination.icao, name: p.destination.name, lat: p.destination.lat, lon: p.destination.lon };
    cruiseFt = p.cruise_ft > 0 ? p.cruise_ft : null;
    // A route drawn in FlightDeck stays editable as one.
    if (p.source === "manual" && p.fixes.length) {
      drawn = p.fixes.map((f) => ({ ...f }));
      drawnFor = { o: p.origin.icao, d: p.destination.icao };
    }
    if (p.alternate) {
      const hit = ((await api.searchAirports(p.alternate).catch(() => [])) ?? []).find((h) => h.icao === p.alternate);
      altn = hit ? { icao: hit.icao, name: hit.name, lat: hit.lat, lon: hit.lon } : { icao: p.alternate, name: "", lat: 0, lon: 0 };
    }
  }

  /** Great-circle distance in nautical miles. */
  function gc(a: Spot, b: Spot): number {
    const r = Math.PI / 180;
    const h =
      Math.sin(((b.lat - a.lat) * r) / 2) ** 2 +
      Math.cos(a.lat * r) * Math.cos(b.lat * r) * Math.sin(((b.lon - a.lon) * r) / 2) ** 2;
    return 2 * 3440.065 * Math.asin(Math.sqrt(h));
  }

  const direct = $derived(origin && dest ? gc(origin, dest) : null);
  /** Along the drawn route. */
  const routeNm = $derived.by(() => {
    if (!origin || !dest || !drawn) return null;
    let t = 0;
    let prev: Spot = origin;
    for (const f of drawn) {
      const p = { icao: f.ident, name: "", lat: f.lat, lon: f.lon };
      t += gc(prev, p);
      prev = p;
    }
    return t + gc(prev, dest);
  });
  /** Origin and destination still as SimBrief planned them. */
  const sameRoute = $derived(!!base && origin?.icao === base.origin.icao && dest?.icao === base.destination.icao);
  // Half a plan cannot be flown, except an origin filled in from where the
  // aircraft is: that is just the departure, with no plan yet.
  const partial = $derived(!!origin !== !!dest && !(originAuto && !dest));

  function build(): FlightPlan | null {
    if (!origin || !dest) return null;
    const keep = sameRoute && base ? base : null;
    const ac = chosen;
    const sameNo = !!base && flightNo === `${base.airline}${base.flight_number}`;
    return {
      // Editing: anything the form does not show stays as it was.
      ...(mode === "edit" && initial && sameRoute ? initial : {}),
      source: keep ? keep.source : "manual",
      id: keep ? keep.id : `manual-${Date.now()}`,
      generated_ms: keep ? keep.generated_ms : Date.now(),
      callsign: callsign.trim().toUpperCase(),
      airline: sameNo && base ? base.airline : "",
      flight_number: sameNo && base ? base.flight_number : flightNo.trim().toUpperCase(),
      origin: { icao: origin.icao, name: origin.name, lat: origin.lat, lon: origin.lon, runway: keep?.origin.runway ?? "" },
      destination: { icao: dest.icao, name: dest.name, lat: dest.lat, lon: dest.lon, runway: landRwy },
      alternate: altn?.icao ?? "",
      route: drawn ? [origin.icao, ...drawn.map((f) => f.ident), dest.icao].join(" ") : (keep?.route ?? ""),
      cruise_ft: mode === "edit" && cruiseFt != null ? cruiseFt : keep ? keep.cruise_ft : (cruiseFt ?? 0),
      distance_nm: drawn ? Math.round(routeNm ?? 0) : keep?.distance_nm || Math.round(direct ?? 0),
      ete_min: keep?.ete_min || est?.ete || 0,
      aircraft_icao: ac?.icao_type || base?.aircraft_icao || "",
      aircraft_name: ac ? modelName(ac) : (base?.aircraft_name ?? ""),
      registration: ac?.registration || base?.registration || "",
      block_fuel_kg: keep?.block_fuel_kg || (est ? Math.round(est.block * 0.45359237) : 0),
      sched_out_ms: keep?.sched_out_ms ?? null,
      sched_in_ms: keep?.sched_in_ms ?? null,
      fixes: drawnFixes ?? keep?.fixes ?? [],
      ofp: keep?.ofp ?? null,
    };
  }

  async function go() {
    busy = true;
    error = null;
    try {
      const plan = build();
      if (mode === "start") await api.startFlight(plan, aircraftId > 0 ? aircraftId : null);
      else if (mode === "edit" && !plan) return;
      else await api.setPlan(plan);
      ondone();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  const ago = (ms: number) => {
    const m = Math.round((Date.now() - ms) / 60000);
    if (m < 2) return "dispatched just now";
    if (m < 90) return `dispatched ${m} min ago`;
    const h = Math.round(m / 60);
    return h < 36 ? `dispatched ${h} h ago` : `dispatched ${Math.round(h / 24)} days ago`;
  };

  // --- Import a .PLN -----------------------------------------------------------
  let plnInput = $state<HTMLInputElement>();
  let plnBusy = $state(false);
  let plnName = $state<string | null>(null);
  async function importPln(file: File | undefined) {
    if (!file) return;
    plnBusy = true;
    error = null;
    try {
      const p = parsePln(await file.text());
      const named = async (a: { icao: string; lat: number; lon: number }): Promise<Spot> => {
        const h = ((await api.searchAirports(a.icao).catch(() => [])) ?? []).find((x) => x.icao === a.icao);
        // The file's own position for the airport, its name from FlightDeck's list.
        return { icao: a.icao, name: h?.name ?? "", lat: a.lat || h?.lat || 0, lon: a.lon || h?.lon || 0 };
      };
      const [o, d] = await Promise.all([named(p.dep), named(p.dest)]);
      base = null;
      originAuto = false;
      origin = o;
      dest = d;
      if (p.cruiseFt > 0) cruiseFt = p.cruiseFt;
      rules = p.rules;
      drawn = renumber(p.fixes);
      drawnFor = { o: o.icao, d: d.icao };
      saved = null;
      plnName = file.name;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      plnBusy = false;
      if (plnInput) plnInput.value = "";
    }
  }

  // --- Airspace the route enters ---------------------------------------------------
  /** Prohibited, restricted and danger areas entered at the planned altitude. */
  const conflicts = $derived(origin && dest && drawing.stage === "off" ? routeAsp.list.filter((c) => c.conflict) : []);
  const ftText = (ft: number) => (ft >= 18000 ? `FL${String(Math.round(ft / 100)).padStart(3, "0")}` : `${Math.round(ft).toLocaleString("en-US")} ft`);

  // --- Time and fuel ---------------------------------------------------------------
  /** The sim's aircraft and fuel on board, kept fresh while the form is open. */
  let simAc = $state.raw<{ aircraft: AircraftInfo | null; telemetry: Telemetry | null } | null>(null);
  onMount(() => {
    const look = () =>
      api
        .status()
        .then((s) => (simAc = s?.link === "connected" ? { aircraft: s.aircraft, telemetry: s.telemetry } : null))
        .catch(() => {});
    look();
    const t = setInterval(look, 5000);
    return () => clearInterval(t);
  });
  /** What performance to plan with: the sim's aircraft, or the hangar one chosen. */
  const acHint = $derived.by<AircraftHint | null>(() => {
    const ac = chosen;
    if (aircraftId > 0 && ac) return { icao: ac.icao_type, title: ac.title, engineType: ac.engine_type, engineCount: ac.engine_count };
    const a = simAc?.aircraft;
    if (a) return { icao: a.icao_type, title: a.title, engineType: a.engine_type, engineCount: a.engine_count, design: simAc?.telemetry?.design ?? null };
    // No sim: the plan's own aircraft type, if SimBrief gave one.
    if (base?.aircraft_icao) return { icao: base.aircraft_icao, title: base.aircraft_name, engineType: "", engineCount: 0 };
    return null;
  });
  const acPerf = $derived(acHint ? perfFor(acHint) : null);
  /** Winds aloft at the cruise level (GFS), loaded once per level. */
  const winds = new Map<string, Field | null>();
  let windField = $state.raw<Field | null>(null);
  $effect(() => {
    const ft = cruiseNow;
    if (!origin || !dest || ft <= 0) return;
    const lvl = levelFor(ft);
    if (winds.has(lvl)) {
      windField = winds.get(lvl) ?? null;
      return;
    }
    winds.set(lvl, null);
    api
      .windGrid(lvl)
      .then((g) => {
        const f = decodeGrid(g);
        winds.set(lvl, f);
        if (levelFor(untrack(() => cruiseNow)) === lvl) windField = f;
      })
      .catch(() => {});
  });
  const est = $derived.by(() => {
    const perf = acPerf;
    if (!perf || !origin || !dest || drawing.stage !== "off") return null;
    const pts = [origin, ...routeFixes.filter((f) => f.lat || f.lon), dest];
    const f = windField;
    const flying = sameRoute && base?.source === "simbrief" ? "IFR" : rules;
    return estimate(perf, pts, cruiseNow > 0 ? cruiseNow : perf.cruiseFt, {
      rules: flying,
      alternateNm: altn && (altn.lat || altn.lon) ? gc(dest, altn) : 0,
      wind: f ? (lat, lon) => sample(f, lon, lat) : null,
    });
  });
  const hm = (m: number) => (m >= 60 ? `${Math.floor(m / 60)} h ${String(Math.round(m % 60)).padStart(2, "0")} min` : `${Math.round(m)} min`);
  /** Fuel on board now (sim aircraft only), lb. */
  const onBoardLb = $derived(aircraftId <= 0 && simAc?.telemetry && simAc.telemetry.fuel_kg > 0 ? simAc.telemetry.fuel_kg / 0.45359237 : null);
</script>

{#snippet perf()}
  <div class="perf" aria-hidden="true"><span></span></div>
{/snippet}

{#snippet glyph(rot: number)}
  <svg viewBox="-24 -24 48 48" aria-hidden="true">
    <g transform="rotate({rot}) scale(0.78)">
      <path d="M0 -20 L3.4 -6 L19 1.5 L19 5.5 L3.4 2.6 L2.4 13 L8 17 L8 19.6 L0 17.8 L-8 19.6 L-8 17 L-2.4 13 L-3.4 2.6 L-19 5.5 L-19 1.5 L-3.4 -6 Z" />
    </g>
  </svg>
{/snippet}

<div class="form" class:start={mode === "start"}>
  <div class="ids">
    <label>
      <span>Callsign</span>
      <input class="num" bind:value={callsign} placeholder="AMERICAN" maxlength="10" autocomplete="off" spellcheck="false" />
    </label>
    <label>
      <span>Flight no.</span>
      <input class="num" bind:value={flightNo} placeholder="AAL1920" maxlength="10" autocomplete="off" spellcheck="false" />
    </label>
  </div>

  {#if mode === "start"}{@render perf()}{/if}

  {#if drawing.stage !== "off" && mode !== "attach"}
    <section class="draw" aria-label="Draw a route">
      <h3>Draw a route</h3>
      {#if drawing.stage === "origin"}
        <p class="step">Choose where you depart from: type it, or click an airport on the map.</p>
        <ol class="legs">
          <li>
            <span class="mark icon" aria-hidden="true">{@render glyph(45)}</span>
            <AirportField
              label="Origin"
              placeholder="Add origin"
              value={drawing.origin}
              onchange={(v) => {
                drawing.origin = v;
                if (v) drawing.stage = "route";
              }}
            />
          </li>
        </ol>
        {#if drawing.note}<p class="note">{drawing.note}</p>{/if}
        <div class="row">
          <button type="button" class="ghost" onclick={cancelDrawing}>Cancel</button>
        </div>
      {:else}
        <p class="step">
          Click or right-click the map to add waypoints from <b>{drawing.origin?.icao}</b>. Clicking an airport passes over it; right-click your
          destination airport and choose Set as destination to finish. Drag a point to move it, click it to take it out, or drag the line to put one
          in.
        </p>
        <div class="chain num">
          <span class="apt">{drawing.origin?.icao}</span>
          {#each drawing.pts as p, i (i)}
            <span class="sep">›</span><span class:apt={!!p.icao}>{p.icao || `WP${drawing.pts.slice(0, i + 1).filter((x) => !x.icao).length}`}</span>
          {/each}
        </div>
        <p class="total"><b class="num">{Math.round(drawnNm)}</b> nm · {drawing.pts.length} point{drawing.pts.length === 1 ? "" : "s"}</p>
        {#if altEdit}
          <!-- Altitudes by point; empty follows the cruise altitude. -->
          <ol class="alts" aria-label="Waypoint altitudes">
            {#each drawing.pts as p, i (i)}
              {#if !(i === drawing.pts.length - 1 && p.icao && lastIsAirport)}
                <li>
                  <span class="num" class:apt={!!p.icao}>{p.icao || `WP${drawing.pts.slice(0, i + 1).filter((x) => !x.icao).length}`}</span>
                  <input
                    class="num"
                    type="number"
                    min="0"
                    max="60000"
                    step="100"
                    placeholder={cruiseFt ? `${cruiseFt}` : "Cruise"}
                    value={p.alt ?? ""}
                    oninput={(e) => {
                      const v = (e.currentTarget as HTMLInputElement).valueAsNumber;
                      drawing.pts[i].alt = Number.isFinite(v) && v > 0 ? v : null;
                    }}
                    aria-label="Altitude at {p.icao || 'waypoint ' + (i + 1)}, feet"
                  />
                  <span class="u">ft</span>
                </li>
              {/if}
            {/each}
          </ol>
        {/if}
        {#if drawing.pts.length && !lastIsAirport}<p class="note">Finish on an airport: click your destination.</p>{/if}
        <div class="row">
          <button type="button" class="ghost" onclick={cancelDrawing}>Cancel</button>
          <button type="button" class="ghost" disabled={!drawing.pts.length} onclick={() => (drawing.pts = drawing.pts.slice(0, -1))}>Undo</button>
          <button type="button" class="ghost" class:on={altEdit} aria-pressed={altEdit} disabled={!drawing.pts.length} onclick={() => (altEdit = !altEdit)}>Altitudes</button>
          <button type="button" class="next" disabled={!lastIsAirport} onclick={finishDrawing}>Finish</button>
        </div>
      {/if}
    </section>
  {:else}
    <ol class="legs">
      {#if mode === "start"}
        <li class="ac">
          <span class="mark plane" aria-hidden="true">{@render glyph(45)}</span>
          <select bind:value={aircraftId} aria-label="Aircraft" class:set={aircraftId > 0}>
            <option value={-1}>Aircraft in the sim</option>
            {#if aircraft && !fleet.some((a) => a.id === aircraft.id)}
              <option value={aircraft.id}>{aircraft.registration || aircraft.icao_type} · {modelName(aircraft)}</option>
            {/if}
            {#each fleet as a (a.id)}
              <option value={a.id}>{a.favorite ? "★ " : ""}{a.registration || a.icao_type || "Unregistered"} · {modelName(a)}</option>
            {/each}
          </select>
        </li>
      {/if}
      <li class="pt solid">
        <span class="mark icon" aria-hidden="true">{@render glyph(45)}</span>
        <AirportField label="Origin" placeholder="Origin" value={origin} onchange={(v) => ((origin = v), (originAuto = false))} />
      </li>
      <li class="pt">
        <span class="mark icon" aria-hidden="true">{@render glyph(135)}</span>
        <AirportField label="Destination" placeholder="Destination" value={dest} onchange={(v) => (dest = v)} />
      </li>
      {#if dest}
        <!-- The runway to land on: the profile's glideslope uses it (changeable in flight too). -->
        <li class="pt rwy-row">
          <span class="mark icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M9 3h6l2 18H7z" /><path d="M12 6v2M12 11v2M12 16v2" /></svg>
          </span>
          <div class="rwy-pick">
            <span class="rwy-k">Landing runway</span>
            <RunwayPicker icao={dest.icao} value={landRwy} onchange={(r) => (landRwy = r)} />
          </div>
        </li>
      {/if}
      <li class="pt dotted">
        <span class="mark icon alt" aria-hidden="true">{@render glyph(90)}</span>
        <AirportField label="Alternate" placeholder="Alternate" value={altn} onchange={(v) => (altn = v)} />
      </li>
      {#if dest && !(sameRoute && base?.source === "simbrief")}
        <!-- Cruise altitude: the flight profile climbs to it and descends from it. -->
        <li class="pt cruise-row">
          <span class="mark icon" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path d="M4 18h3l4-8h9v2h-8l-4 8H4z" /></svg>
          </span>
          <label class="cruise">
            <input type="number" min="500" max="60000" step="500" placeholder="Cruise altitude" bind:value={cruiseFt} class="num" aria-label="Cruise altitude in feet" />
            ft
          </label>
        </li>
      {/if}
    </ol>

    {#if mode === "start" && aircraftId > 0}
      <p class="hint">This flight is logged to {chosen?.registration || "this aircraft"}, whatever the sim shows.</p>
    {/if}
    {#if (drawn && routeNm != null) || direct != null}
      <div class="dist-row">
        {#if drawn && routeNm != null}
          <p class="hint">{distance(routeNm).value} {distance(routeNm).unit} along the route ({drawn.length} point{drawn.length === 1 ? "" : "s"})</p>
        {:else if direct != null}
          <p class="hint">{distance(direct).value} {distance(direct).unit} direct{base && sameRoute && base.distance_nm ? `, ${distance(base.distance_nm).value} ${distance(base.distance_nm).unit} planned` : ""}</p>
        {/if}
        {#if conflicts.length}
          <!-- The route enters a prohibited, restricted or danger area: hover for which, click to see it on the map. -->
          <span class="conflict">
            <button type="button" class="warn" aria-label="Airspace conflict: show on the map" onclick={() => routeAsp.focus++}>!</button>
            <span class="warn-card" role="tooltip">
              <strong>Airspace conflict</strong>
              {#each conflicts as c (c.id)}
                <span class="wc-row">
                  <i style:background={ASP_KINDS[c.c]?.color ?? "#ff4f6d"}></i>
                  <span>
                    <b>{ASP_KINDS[c.c]?.name ?? c.c}</b>{c.name ? ` · ${c.name}` : ""}<br />
                    <em>{c.loText || ftText(c.lo)} to {c.hiText || ftText(c.hi)} · route at {ftText(c.alt)}, {Math.round(c.from)}–{Math.round(c.to)} nm from {origin?.icao}</em>
                  </span>
                </span>
              {/each}
              <span class="wc-foot">Click to show on the map.</span>
            </span>
          </span>
        {/if}
      </div>
    {:else if partial}
      <p class="hint">Add both an origin and a destination to use a flight plan.</p>
    {:else if originAuto && origin && !dest}
      <p class="hint">Departing from {origin.icao}, where the aircraft is. Add a destination for a flight plan.</p>
    {/if}

    {#if est && acPerf && mode !== "attach"}
      <!-- Time and fuel: typical acPerformance of the aircraft type, the route, the cruise and forecast winds. -->
      <details class="est">
        <summary>
          <span class="est-k">ETE</span><b class="num">{hm(est.ete)}</b>
          <span class="est-k">Fuel</span><b class="num">{fuelText(acPerf, est.block).main}</b>
          <span class="est-s">with reserves</span>
          {#if est.block > est.usable}<span class="est-bad" title="More than the tanks hold">Over capacity</span>{/if}
        </summary>
        <div class="est-body">
          <p class="est-src">{acPerf.name}{acPerf.source === "design" ? " (from the sim's figures)" : ""} · {est.cruiseFt < cruiseNow - 50 ? `${ftText(est.cruiseFt)} on this short route` : ftText(est.cruiseFt)}{est.headwind != null ? ` · ${Math.abs(Math.round(est.headwind))} kt ${est.headwind >= 0 ? "headwind" : "tailwind"} (GFS)` : ""}</p>
          <dl>
            <dt>Climb · cruise · descent</dt><dd class="num">{hm(est.climbMin)} · {hm(est.cruiseMin)} · {hm(est.descentMin)}</dd>
            <dt>Taxi</dt><dd class="num">{fuelText(acPerf, est.taxi).main}</dd>
            <dt>Trip</dt><dd class="num">{fuelText(acPerf, est.trip).main}</dd>
            {#if est.contingency > 0}<dt>Contingency 5%</dt><dd class="num">{fuelText(acPerf, est.contingency).main}</dd>{/if}
            {#if est.alternate > 0}<dt>Alternate {altn?.icao}</dt><dd class="num">{fuelText(acPerf, est.alternate).main}</dd>{/if}
            <dt>Reserve {est.reserveMin} min</dt><dd class="num">{fuelText(acPerf, est.reserve).main}</dd>
            <dt class="tot">Block fuel</dt><dd class="num tot">{fuelText(acPerf, est.block).main} <span>{fuelText(acPerf, est.block).sub}</span></dd>
            <dt>Usable in the tanks</dt><dd class="num">{fuelText(acPerf, est.usable).main}</dd>
            {#if onBoardLb != null}
              <dt>On board now</dt><dd class="num" class:short={onBoardLb < est.block}>{fuelText(acPerf, onBoardLb).main}{onBoardLb < est.block ? ` · ${fuelText(acPerf, est.block - onBoardLb).main} short` : ""}</dd>
            {/if}
          </dl>
          <p class="est-note">An estimate from typical published figures for the type, for planning in the sim only.</p>
        </div>
      </details>
    {/if}

    {#if drawn && mode !== "attach"}
      <section class="drawn" aria-label="Your route">
        <div class="drawn-top">
          <strong>{plnName ? "Imported route" : "Your route"}</strong>
          <span class="seg" role="radiogroup" aria-label="Flight rules">
            <button type="button" role="radio" aria-checked={rules === "VFR"} class:on={rules === "VFR"} onclick={() => (rules = "VFR")}>VFR</button>
            <button type="button" role="radio" aria-checked={rules === "IFR"} class:on={rules === "IFR"} onclick={() => (rules = "IFR")}>IFR</button>
          </span>
        </div>
        <div class="row acts">
          <button type="button" class="pln" disabled={saving} onclick={savePln} title="Save as a .PLN file for MSFS">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v11M7 10l5 5 5-5M5 19h14" /></svg>
            {saving ? "Saving…" : "Save .PLN"}
          </button>
          <button type="button" class="ghost" onclick={() => startDrawing()}>Redraw</button>
          <button type="button" class="ghost" onclick={() => ((drawn = null), (plnName = null))}>Remove</button>
        </div>
        {#if saved}<p class="hint">Saved {saved}. In MSFS 2020 or 2024, load it from the world map's flight plan menu.</p>{/if}
        <p class="hint">On the map: drag a point to move it, drag the line to add one, right-click a point to take it out.</p>
      </section>
    {/if}

    <input bind:this={plnInput} type="file" accept=".pln,.PLN,application/xml,text/xml" hidden onchange={(e) => importPln((e.currentTarget as HTMLInputElement).files?.[0])} />
    <div class="box sb-pln">
      {#if !hasId}
        <button type="button" class="opt" onclick={onsettings}>
          <strong>Import from SimBrief</strong>
          <span>Add your SimBrief Pilot ID in Settings first</span>
        </button>
      {:else if looking}
        <div class="opt wait"><strong>Import from SimBrief</strong><span>Looking for your latest flight…</span></div>
      {:else if latest}
        <!-- One compact row: the flight, then Import (or Imported) and the OFP. -->
        <div class="sbrow">
          <button type="button" class="opt sb" onclick={importSimbrief} disabled={importing} title={latest.route || undefined}>
            <span class="sb-k">SimBrief</span>
            <span class="sb-main num">{latest.callsign || `${latest.airline}${latest.flight_number}`} · {latest.origin.icao}–{latest.destination.icao}</span>
            <span class="sb-sub">{[latest.aircraft_icao, latest.distance_nm ? `${distance(latest.distance_nm).value} ${distance(latest.distance_nm).unit}` : "", ago(latest.generated_ms)].filter(Boolean).join(" · ")}</span>
            <span class="sb-act" class:done={!!base && base.id === latest.id && !importing}>{importing ? "Importing…" : base && base.id === latest.id ? "✓ Imported" : "Import"}</span>
          </button>
          {#if base && base.source === "simbrief"}
            <button type="button" class="ofp" onclick={() => base && openBriefing(base)} disabled={briefing.loading} title="View the OFP">
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 3h7l4 4v14H7zM14 3v4h4M10 11h5M10 14h5M10 17h3" /></svg>
              {briefing.loading ? "…" : "OFP"}
            </button>
          {/if}
        </div>
      {:else}
        <button type="button" class="opt" onclick={lookSimbrief}>
          <strong>Import from SimBrief</strong>
          <span>{sbError ?? "No flight found"}</span>
          <span class="again">Click to try again</span>
        </button>
      {/if}
      {#if mode !== "attach"}
        <button type="button" class="opt pln-in" disabled={plnBusy} onclick={() => plnInput?.click()} title="Import an MSFS flight plan (.PLN)">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 3h7l4 4v14H7zM14 3v4h4" /><path d="M12.5 10v7M9.5 14l3 3 3-3" /></svg>
          <strong>{plnBusy ? "Reading…" : ".PLN"}</strong>
          <span>Import</span>
        </button>
      {/if}
    </div>

    {#if mode !== "attach"}
      <div class="two">
        <button type="button" class="opt small" disabled title="Enter route manually: coming in a future version" aria-label="Enter route manually (coming soon)">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h16M4 12h10M4 17h7" /></svg>
          Enter route manually
          <em>Soon</em>
        </button>
        <button type="button" class="opt small" onclick={() => startDrawing()}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="6" cy="6" r="2" /><path d="M7.5 7.5c2 2 1 4 3 6s4 1 6 3" stroke-dasharray="2 2.4" /><path d="M16 16.5l3 3M19 16.5l-3 3" /></svg>
          Draw route
        </button>
      </div>
    {/if}
  {/if}

  {#if error}<p class="err">{error}</p>{/if}

  {#if mode === "start"}{@render perf()}{/if}

  <button type="button" class="go" onclick={go} disabled={busy || partial || drawing.stage !== "off" || (mode !== "start" && !(origin && dest))}>
    {#if mode === "attach"}
      Add flight plan
    {:else if mode === "edit"}
      Save flight plan
    {:else if origin && dest}
      Start flight
    {:else}
      Start without a flight plan
    {/if}
  </button>
  {#if mode === "start"}
    <p class="hint center">Tracking starts as soon as you are in the cockpit.</p>
  {/if}
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .ids {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  .ids label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-faint);
  }
  .ids input {
    font: inherit;
    font-family: var(--cond);
    font-size: 19px;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--ink);
    background: transparent;
    border: none;
    border-bottom: 1px solid transparent;
    padding: 1px 0;
    outline: none;
    min-width: 0;
    width: 100%;
    box-sizing: border-box;
  }
  .ids input::placeholder {
    color: #464b52;
  }
  .ids input:hover {
    border-bottom-color: #3a3e44;
  }
  .ids input:focus {
    border-bottom-color: var(--amber);
  }

  /* A boarding pass's tear line: dashes across, a notch at each edge. */
  .perf {
    position: relative;
    height: 14px;
    margin: 2px -18px;
  }
  .perf span {
    position: absolute;
    left: 14px;
    right: 14px;
    top: 50%;
    border-top: 1.5px dashed #464b52;
  }
  .perf::before,
  .perf::after {
    content: "";
    position: absolute;
    top: 0;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #0f1012;
    border: 1px solid var(--rule);
    box-sizing: border-box;
  }
  .perf::before {
    left: -8px;
  }
  .perf::after {
    right: -8px;
  }

  .legs {
    list-style: none;
    margin: 2px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .legs li {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 40px;
  }
  /* The line from origin to destination (solid), on to the alternate (dotted). */
  .legs li.pt.solid::after,
  .legs li.pt:not(.solid):not(.dotted)::after {
    content: "";
    position: absolute;
    left: 11px;
    top: calc(50% + 12px);
    height: calc(100% - 24px);
    border-left: 2px solid #8c9199;
  }
  .legs li.pt:not(.solid):not(.dotted)::after {
    border-left: 2px dotted #6b7078;
  }
  .mark {
    flex: none;
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
  }
  .mark.icon {
    border-radius: 50%;
    background: #d9dce0;
  }
  .mark.icon svg {
    width: 18px;
    height: 18px;
    fill: #1b1c1e;
  }
  .mark.icon.alt {
    background: transparent;
    border: 1.6px dashed #8c9199;
    box-sizing: border-box;
  }
  .mark.icon.alt svg {
    fill: #8c9199;
  }
  .plane svg {
    width: 24px;
    height: 24px;
    fill: var(--ink);
  }
  select {
    flex: 1;
    min-width: 0;
    appearance: none;
    font: inherit;
    font-size: 15px;
    color: var(--ink-dim);
    background: transparent;
    border: none;
    padding: 6px 0;
    cursor: pointer;
    text-overflow: ellipsis;
  }
  select.set {
    color: var(--ink);
    font-weight: 600;
  }
  select:hover {
    color: var(--ink);
  }
  select option {
    background: var(--panel-3);
    color: var(--ink);
    font-weight: 400;
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--ink-dim);
  }
  .hint.center {
    text-align: center;
    margin-top: -4px;
  }
  .box {
    margin-top: 2px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .opt {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 11px 12px;
    border: 1px solid transparent;
    border-radius: 9px;
    background: var(--panel-3);
    color: var(--ink);
    font-size: 13px;
    text-align: center;
    transition: border-color 0.15s, background 0.15s;
  }
  .opt:hover:not(:disabled) {
    border-color: #a78bfa;
    background: #33373c;
  }
  .opt strong {
    font-size: 14.5px;
  }
  .opt span {
    color: var(--ink-dim);
  }
  .opt .again {
    margin-top: 3px;
    color: var(--ink-faint);
  }
  .sbrow {
    display: flex;
    gap: 6px;
    align-items: stretch;
  }
  /* SimBrief's flight in one compact block: label and action on the top
     line, the flight, then its details. */
  .opt.sb {
    flex: 1;
    min-width: 0;
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-areas: "k act" "main main" "sub sub";
    align-items: center;
    gap: 1px 8px;
    padding: 8px 11px;
    text-align: left;
  }
  .sb-k {
    grid-area: k;
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #a78bfa !important;
  }
  .sb-main {
    grid-area: main;
    font-size: 15px;
    color: #c4b5fd !important;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sb-sub {
    grid-area: sub;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sb-act {
    grid-area: act;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink) !important;
  }
  .sb-act.done {
    color: #7ddc9a !important;
  }
  .sbrow .ofp {
    flex-direction: column;
    gap: 3px;
    padding: 6px 10px;
    font-size: 12px;
  }

  .ofp {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 7px 10px;
    border-radius: 8px;
    border: 1px solid #6d5bd0;
    background: transparent;
    color: #c4b5fd;
    font-size: 13px;
    font-weight: 600;
  }
  .ofp:hover:not(:disabled) {
    background: rgba(167, 139, 250, 0.12);
  }
  .ofp svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .opt.wait {
    cursor: default;
  }
  /* SimBrief, with the .PLN import to its right. */
  .box.sb-pln {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 74px;
    align-items: stretch;
  }
  .opt.pln-in {
    justify-content: center;
    gap: 1px;
    padding: 6px 4px;
  }
  .opt.pln-in svg {
    width: 20px;
    height: 20px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
    stroke-linecap: round;
    stroke-linejoin: round;
    color: var(--ink-dim);
  }
  .opt.pln-in strong {
    font-size: 12.5px;
  }
  .opt.pln-in span {
    font-size: 11px;
  }
  /* Distance, with the airspace warning at its right. */
  .dist-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .dist-row .hint {
    flex: 1;
  }
  .conflict {
    position: relative;
    flex: none;
  }
  .warn {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 0;
    padding: 0;
    background: #e5484d;
    color: #fff;
    font: 800 14px/22px var(--cond, sans-serif);
    cursor: pointer;
    box-shadow: 0 0 0 3px rgba(229, 72, 77, 0.25);
  }
  .warn:hover,
  .warn:focus-visible {
    box-shadow: 0 0 0 4px rgba(229, 72, 77, 0.4);
  }
  .warn-card {
    position: absolute;
    right: -6px;
    top: calc(100% + 8px);
    width: 290px;
    z-index: 20;
    display: none;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    background: rgba(28, 30, 33, 0.98);
    border: 1px solid rgba(229, 72, 77, 0.55);
    border-radius: 10px;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.5);
    font-size: 12px;
    color: var(--ink);
  }
  .conflict:hover .warn-card,
  .conflict:focus-within .warn-card {
    display: flex;
  }
  .warn-card strong {
    color: #ff8a8e;
    font-size: 12.5px;
  }
  .wc-row {
    display: flex;
    gap: 8px;
    line-height: 1.35;
  }
  .wc-row i {
    flex: none;
    width: 8px;
    height: 8px;
    margin-top: 4px;
    border-radius: 2px;
  }
  .wc-row em {
    font-style: normal;
    color: var(--ink-dim);
  }
  .wc-foot {
    color: var(--ink-faint);
    font-size: 11px;
  }
  /* Time and fuel. */
  .est {
    border: 1px solid var(--rule);
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.02);
  }
  .est summary {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 8px 10px;
    cursor: pointer;
    list-style: none;
    font-size: 12.5px;
  }
  .est summary::-webkit-details-marker {
    display: none;
  }
  .est-k {
    color: var(--ink-faint);
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .est summary b {
    color: var(--ink);
    margin-right: 6px;
  }
  .est-s {
    color: var(--ink-faint);
    font-size: 11px;
  }
  .est-bad {
    margin-left: auto;
    color: #ff8a8e;
    font-size: 11px;
    font-weight: 600;
  }
  .est-body {
    padding: 0 10px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .est-src,
  .est-note {
    margin: 0;
    font-size: 11.5px;
    color: var(--ink-dim);
  }
  .est-note {
    color: var(--ink-faint);
    font-size: 10.5px;
  }
  .est dl {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 3px 10px;
    margin: 0;
    font-size: 12px;
  }
  .est dt {
    color: var(--ink-dim);
  }
  .est dd {
    margin: 0;
    text-align: right;
    color: var(--ink);
  }
  .est .tot {
    font-weight: 700;
    color: var(--ink);
    border-top: 1px solid var(--rule);
    padding-top: 3px;
  }
  .est dd.tot span {
    color: var(--ink-faint);
    font-weight: 500;
  }
  .est dd.short {
    color: #ff8a8e;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .opt.small {
    position: relative;
    flex-direction: row;
    justify-content: center;
    gap: 7px;
    padding: 10px 4px;
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
  }
  .opt.small svg {
    width: 17px;
    height: 17px;
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .opt.small:disabled {
    color: var(--ink-faint);
    cursor: default;
  }
  .opt.small em {
    position: absolute;
    top: -7px;
    right: 6px;
    font-style: normal;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    padding: 1px 5px;
    border-radius: 4px;
    background: #3a3e44;
    color: var(--ink-dim);
    border: 1px solid #23262a;
  }

  /* Drawing a route, and the route drawn. */
  .draw,
  .drawn {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border-radius: 10px;
    background: var(--panel-3);
  }
  .draw h3 {
    margin: 0;
    font-family: var(--cond);
    font-size: 18px;
    font-weight: 600;
  }
  .step {
    margin: 0;
    font-size: 13px;
    color: var(--ink-dim);
  }
  .step b {
    color: var(--amber);
  }
  .note {
    margin: 0;
    font-size: 12.5px;
    color: var(--amber);
  }
  .alts {
    list-style: none;
    margin: 0 0 8px;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 4px 10px;
    max-height: 168px;
    overflow-y: auto;
  }
  .alts li {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .alts span.num {
    width: 46px;
    font-size: 12.5px;
    color: var(--ink-dim);
  }
  .alts span.apt {
    color: var(--amber);
  }
  .alts input {
    width: 72px;
    padding: 3px 6px;
    border: 1px solid var(--rule);
    border-radius: 5px;
    background: transparent;
    color: var(--ink);
    font-size: 13px;
  }
  .alts .u {
    font-size: 11.5px;
    color: var(--ink-faint);
  }
  .draw .row button.ghost.on {
    border-color: #5aa9ff;
    color: #a9d1ff;
  }
  .chain {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 5px;
    font-size: 14px;
    color: var(--ink-dim);
  }
  .chain .apt {
    color: var(--ink);
    font-weight: 600;
  }
  .chain .sep {
    color: var(--ink-faint);
  }
  .total {
    margin: 0;
    font-size: 13px;
    color: var(--ink-dim);
  }
  .total b {
    font-size: 17px;
    color: var(--amber);
  }
  .row {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
    flex-wrap: wrap;
  }
  .ghost,
  .next,
  .pln {
    padding: 7px 12px;
    border-radius: 7px;
    border: 1px solid #464b52;
    background: transparent;
    color: var(--ink-dim);
    font-size: 13px;
  }
  .ghost:hover:not(:disabled) {
    color: var(--ink);
    border-color: var(--ink-faint);
  }
  .next,
  .pln {
    border-color: var(--amber);
    background: var(--amber);
    color: #1b1406;
    font-weight: 600;
  }
  .pln {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-right: auto;
  }
  /* Save, Edit, Redraw and Remove on one line. */
  .drawn .acts {
    display: flex;
    flex-wrap: nowrap;
    gap: 5px;
  }
  .drawn .acts button {
    padding: 6px 9px;
    font-size: 12.5px;
    white-space: nowrap;
    flex: none;
  }
  .drawn .acts .pln {
    padding: 6px 10px;
  }
  .pln svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .ghost:disabled,
  .next:disabled {
    opacity: 0.45;
  }
  .drawn-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .drawn-top strong {
    font-size: 14px;
  }
  .seg {
    display: inline-flex;
    padding: 2px;
    border-radius: 7px;
    background: #23262a;
  }
  .seg button {
    padding: 3px 10px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--ink-dim);
    font-size: 12px;
    font-weight: 600;
  }
  .seg button.on {
    background: var(--amber);
    color: #1b1406;
  }
  .cruise {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--ink-dim);
  }
  .rwy-pick {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    flex: 1;
  }
  .rwy-k {
    font-size: 12px;
    color: var(--ink-faint, #7d828a);
  }
  .cruise-row .cruise {
    flex: 1;
    min-width: 0;
  }
  .cruise input {
    width: 150px;
    padding: 5px 8px;
    border-radius: 6px;
    border: 1px solid #464b52;
    background: #23262a;
    color: var(--ink);
    font-size: 14px;
  }
  .go {
    padding: 13px 14px;
    border: none;
    border-radius: 10px;
    background: var(--amber);
    color: #1b1406;
    font-family: var(--cond);
    font-size: 18px;
    font-weight: 600;
  }
  .go:hover:not(:disabled) {
    filter: brightness(1.06);
  }
  .go:disabled {
    opacity: 0.5;
  }
  .err {
    margin: 0;
    color: var(--bad);
    font-size: 13px;
  }
</style>
