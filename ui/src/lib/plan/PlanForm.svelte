<script lang="ts">
  // A simple flight plan: callsign, aircraft, origin, destination and
  // alternate, typed in or imported from SimBrief. Used to start a flight,
  // and mid-flight to add a plan.
  import { onDestroy, onMount, untrack } from "svelte";
  import { draft, pick } from "../flyStore.svelte";
  import { api, type FlightPlan, type HangarItem } from "../api";
  import { modelName } from "../hangar/names";
  import { distance, prefs } from "../prefs.svelte";
  import AirportField, { type Spot } from "./AirportField.svelte";

  interface Props {
    /** start: arm a new flight. attach: add a plan to the flight under way. */
    mode: "start" | "attach";
    /** Preselected hangar aircraft ("Fly this aircraft"). */
    aircraft?: HangarItem | null;
    onsettings: () => void;
    ondone: () => void;
  }
  let { mode, aircraft = null, onsettings, ondone }: Props = $props();

  let callsign = $state("");
  let flightNo = $state("");
  let origin = $state<Spot | null>(null);
  let dest = $state<Spot | null>(null);
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

  // Preview the route on the map while a flight is being set up.
  $effect(() => {
    if (mode !== "start") return;
    draft.origin = origin;
    draft.dest = dest;
  });
  // An airport sent from the map's right-click menu.
  $effect(() => {
    const spot = pick.spot;
    const which = pick.which;
    if (mode !== "start" || !spot || !which) return;
    untrack(() => {
      if (which === "origin") {
        origin = spot;
        originAuto = false;
      } else dest = spot;
      pick.spot = null;
      pick.which = null;
    });
  });
  onDestroy(() => {
    if (mode !== "start") return;
    draft.origin = null;
    draft.dest = null;
  });

  const hasId = $derived(!!prefs.s.simbrief_id?.trim());

  onMount(async () => {
    if (aircraft) aircraftId = aircraft.id;
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

  /** Great-circle distance in nautical miles. */
  function gc(a: Spot, b: Spot): number {
    const r = Math.PI / 180;
    const h =
      Math.sin(((b.lat - a.lat) * r) / 2) ** 2 +
      Math.cos(a.lat * r) * Math.cos(b.lat * r) * Math.sin(((b.lon - a.lon) * r) / 2) ** 2;
    return 2 * 3440.065 * Math.asin(Math.sqrt(h));
  }

  const direct = $derived(origin && dest ? gc(origin, dest) : null);
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
      source: keep ? keep.source : "manual",
      id: keep ? keep.id : `manual-${Date.now()}`,
      generated_ms: keep ? keep.generated_ms : Date.now(),
      callsign: callsign.trim().toUpperCase(),
      airline: sameNo && base ? base.airline : "",
      flight_number: sameNo && base ? base.flight_number : flightNo.trim().toUpperCase(),
      origin: { icao: origin.icao, name: origin.name, lat: origin.lat, lon: origin.lon, runway: keep?.origin.runway ?? "" },
      destination: { icao: dest.icao, name: dest.name, lat: dest.lat, lon: dest.lon, runway: keep?.destination.runway ?? "" },
      alternate: altn?.icao ?? "",
      route: keep?.route ?? "",
      cruise_ft: keep?.cruise_ft ?? 0,
      distance_nm: keep?.distance_nm || Math.round(direct ?? 0),
      ete_min: keep?.ete_min ?? 0,
      aircraft_icao: ac?.icao_type || base?.aircraft_icao || "",
      aircraft_name: ac ? modelName(ac) : (base?.aircraft_name ?? ""),
      registration: ac?.registration || base?.registration || "",
      block_fuel_kg: keep?.block_fuel_kg ?? 0,
      sched_out_ms: keep?.sched_out_ms ?? null,
      sched_in_ms: keep?.sched_in_ms ?? null,
      fixes: keep?.fixes ?? [],
      ofp: keep?.ofp ?? null,
    };
  }

  async function go() {
    busy = true;
    error = null;
    try {
      const plan = build();
      if (mode === "start") await api.startFlight(plan, aircraftId > 0 ? aircraftId : null);
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
</script>

<div class="form">
  <div class="ids">
    <label>
      <span>Callsign</span>
      <input class="num" bind:value={callsign} placeholder="DAL864" maxlength="10" autocomplete="off" spellcheck="false" />
    </label>
    <label>
      <span>Flight no.</span>
      <input class="num" bind:value={flightNo} placeholder="DL864" maxlength="10" autocomplete="off" spellcheck="false" />
    </label>
  </div>

  <ol class="legs">
    {#if mode === "start"}
      <li class="ac">
        <span class="mark plane" aria-hidden="true">
          <svg viewBox="0 0 24 24"><path d="M2.5 13.5l7-1.5 5-8h2.5l-2.5 8 5.5-.5 1.8-2.5H23l-1.2 4 1.2 4h-1.2L20 14.5l-5.5-.5 2.5 8h-2.5l-5-8-7-1.5z" /></svg>
        </span>
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
    <li>
      <span class="mark dot" aria-hidden="true"></span>
      <AirportField label="Origin" placeholder="Add origin" value={origin} onchange={(v) => ((origin = v), (originAuto = false))} />
    </li>
    <li>
      <span class="mark ring" aria-hidden="true"></span>
      <AirportField label="Destination" placeholder="Add destination" value={dest} onchange={(v) => (dest = v)} />
    </li>
    <li>
      <span class="mark ring dim" aria-hidden="true"></span>
      <AirportField label="Alternate" placeholder="Add alternate" value={altn} onchange={(v) => (altn = v)} />
    </li>
  </ol>

  {#if mode === "start" && aircraftId > 0}
    <p class="hint">This flight is logged to {chosen?.registration || "this aircraft"}, whatever the sim shows.</p>
  {/if}
  {#if direct != null}
    <p class="hint">{distance(direct).value} {distance(direct).unit} direct{base && sameRoute && base.distance_nm ? `, ${distance(base.distance_nm).value} ${distance(base.distance_nm).unit} planned` : ""}</p>
  {:else if partial}
    <p class="hint">Add both an origin and a destination to use a flight plan.</p>
  {:else if originAuto && origin && !dest}
    <p class="hint">Departing from {origin.icao}, where the aircraft is. Add a destination for a flight plan.</p>
  {/if}

  <div class="box">
    {#if !hasId}
      <button type="button" class="opt" onclick={onsettings}>
        <strong>Import from SimBrief</strong>
        <span>Add your SimBrief Pilot ID in Settings first</span>
      </button>
    {:else}
      {#if looking}
        <div class="opt wait"><strong>Import from SimBrief</strong><span>Looking for your latest flight…</span></div>
      {:else if latest}
        <button type="button" class="opt sb" onclick={importSimbrief} disabled={importing}>
          <strong>{importing ? "Importing…" : base ? "Imported from SimBrief" : "Import from SimBrief"}</strong>
          <span class="big num">{latest.callsign || `${latest.airline}${latest.flight_number}`} ({latest.origin.icao}–{latest.destination.icao})</span>
          <span>{[latest.aircraft_icao, latest.distance_nm ? `${distance(latest.distance_nm).value} ${distance(latest.distance_nm).unit}` : "", ago(latest.generated_ms)].filter(Boolean).join(", ")}</span>
          {#if latest.route}<span class="route num">{latest.route}</span>{/if}
        </button>
      {:else}
        <button type="button" class="opt" onclick={lookSimbrief}>
          <strong>Import from SimBrief</strong>
          <span class="err">{sbError ?? "No flight found"}</span>
          <span>Click to try again</span>
        </button>
      {/if}
    {/if}
  </div>

  {#if error}<p class="err">{error}</p>{/if}

  <button type="button" class="go" onclick={go} disabled={busy || partial || (mode === "attach" && !(origin && dest))}>
    {#if mode === "attach"}
      Add flight plan
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
    gap: 8px;
  }
  .ids label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-faint);
  }
  .ids input {
    font: inherit;
    font-family: var(--cond);
    font-size: 20px;
    font-weight: 600;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--ink);
    background: transparent;
    border: none;
    border-bottom: 1px dashed var(--ink-faint);
    padding: 2px 0;
    outline: none;
    min-width: 0;
    width: 100%;
    box-sizing: border-box;
  }
  .ids input::placeholder {
    color: #4b5057;
  }
  .ids input:focus {
    border-bottom-color: var(--amber);
  }
  .legs {
    list-style: none;
    margin: 4px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .legs li {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
  }
  /* The dashed line joining the airports. */
  .legs li:not(.ac) + li:not(.ac)::before {
    content: "";
    position: absolute;
    left: 10px;
    top: -14px;
    height: 20px;
    border-left: 1px dashed #5a5f66;
  }
  .mark {
    flex: none;
    width: 21px;
    display: grid;
    place-items: center;
  }
  .dot::after,
  .ring::after {
    content: "";
    width: 12px;
    height: 12px;
    border-radius: 50%;
    box-sizing: border-box;
  }
  .dot::after {
    background: var(--ink);
  }
  .ring::after {
    border: 3px solid var(--ink-dim);
  }
  .ring.dim::after {
    border-color: #5a5f66;
  }
  .plane svg {
    width: 21px;
    height: 21px;
    fill: var(--ink);
    /* The icon points left: flip it to fly up and to the right. */
    transform: rotate(-40deg) scaleX(-1);
  }
  select {
    flex: 1;
    min-width: 0;
    appearance: none;
    font: inherit;
    font-size: 15px;
    color: var(--ink-faint);
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
    margin-top: 4px;
    padding: 12px;
    border: 1px dashed #4b5057;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .opt {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 10px 12px;
    border: 1px solid transparent;
    border-radius: 6px;
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
  .opt.sb span.num {
    color: #c4b5fd;
  }
  .opt .big {
    font-size: 18px;
    letter-spacing: 0.02em;
  }
  .opt.sb .route {
    max-width: 100%;
    margin-top: 2px;
    font-size: 12px;
    color: var(--ink-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .opt.wait {
    cursor: default;
  }
  .go {
    padding: 11px 14px;
    border: none;
    border-radius: 8px;
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
