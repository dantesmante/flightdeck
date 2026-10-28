<script lang="ts">
  // One aircraft: its picture and registration, statistics, and every
  // flight flown in it, listed and drawn on a map.
  import { onMount } from "svelte";
  import { api, type FlightRow, type HangarItem, type Route } from "../api";
  import { day, gradeTone, hhmm, hoursMinutes, int } from "../format";
  import Map from "../Map.svelte";
  import { distance } from "../prefs.svelte";
  import { fromTrack, type Pt } from "../track";
  import Blueprint from "./Blueprint.svelte";

  interface Props {
    item: HangarItem;
    photo: string | null;
    displayName: string;
    onback: () => void;
    onupload: (e: Event) => void;
    onflight?: (id: number) => void;
  }
  let { item, photo, displayName, onback, onupload, onflight }: Props = $props();

  let tab = $state<"stats" | "flights">("flights");
  let flights = $state<FlightRow[]>([]);
  let routes = $state<Route[]>([]);
  let selected = $state<number | null>(null);
  let history = $state<Pt[] | null>(null);
  let query = $state("");
  let newestFirst = $state(true);

  onMount(async () => {
    flights = (await api.aircraftFlights(item.id)) ?? [];
    const ids = new Set(flights.map((f) => f.id));
    routes = ((await api.routes()) ?? []).filter((r) => ids.has(r.flight_id));
  });

  const shown = $derived.by(() => {
    const q = query.trim().toUpperCase();
    const list = q
      ? flights.filter((f) =>
          [f.dep_icao, f.arr_icao, f.callsign, f.dep_name, f.arr_name, day(f.start_ms)].some((s) => (s ?? "").toUpperCase().includes(q)),
        )
      : flights;
    return newestFirst ? list : [...list].reverse();
  });

  async function pick(f: FlightRow) {
    if (selected === f.id) {
      selected = null;
      history = null;
      return;
    }
    selected = f.id;
    const d = await api.flight(f.id);
    if (selected === f.id) history = d ? fromTrack(d.track) : null;
  }

  const months = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
  const stamp = (ms: number) => {
    const d = new Date(ms);
    return `${d.getUTCDate()} ${months[d.getUTCMonth()]} ${d.getUTCFullYear()} · ${String(d.getUTCHours()).padStart(2, "0")}:${String(d.getUTCMinutes()).padStart(2, "0")}Z`;
  };
  const grade = (v: number | null) => {
    if (v == null) return null;
    const a = Math.abs(v);
    return a <= 60 ? "Butter" : a <= 180 ? "Smooth" : a <= 300 ? "Firm" : a <= 600 ? "Hard" : "Very hard";
  };
  const dist = $derived(distance(item.distance_nm));
</script>

<div class="detail">
  <aside class="side">
    <div class="hero" class:photo={!!photo}>
      {#if photo}
        <img src={photo} alt={item.registration || displayName} />
      {:else}
        <div class="bp"><Blueprint icao_type={item.icao_type} engine_type={item.engine_type} engine_count={item.engine_count} title={item.title} /></div>
      {/if}
      <div class="shade"></div>
      <button class="back" onclick={onback}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
        Hangar
      </button>
      <label class="change">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 18h10a4 4 0 0 0 .6-7.96A6 6 0 0 0 6.1 9.1 4.5 4.5 0 0 0 7 18zM12 16v-6M9.5 12.5 12 10l2.5 2.5" /></svg>
        {photo ? "Change image" : "Upload image"}
        <input type="file" accept="image/*" onchange={onupload} />
      </label>
      <div class="title">
        <h2 class="num">{item.registration || item.icao_type || "Unregistered"}</h2>
        <p>{displayName}{item.icao_type ? ` · ${item.icao_type}` : ""}</p>
      </div>
    </div>

    <div class="tabs" role="tablist">
      <button role="tab" aria-selected={tab === "stats"} class:on={tab === "stats"} onclick={() => (tab = "stats")}>Statistics</button>
      <button role="tab" aria-selected={tab === "flights"} class:on={tab === "flights"} onclick={() => (tab = "flights")}>Flights ({flights.length})</button>
    </div>

    {#if tab === "stats"}
      <dl class="stats">
        <div><dt>Flights</dt><dd class="num">{item.flights}</dd></div>
        <div><dt>Hours</dt><dd class="num">{hoursMinutes(item.minutes)}</dd></div>
        <div><dt>Distance</dt><dd class="num">{dist.value} <small>{dist.unit}</small></dd></div>
        <div><dt>Landings</dt><dd class="num">{item.landings}</dd></div>
        <div><dt>Average landing</dt><dd class="num {gradeTone(grade(item.avg_landing_fpm))}">{item.avg_landing_fpm != null ? `${int(item.avg_landing_fpm)} fpm` : "–"}</dd></div>
        <div><dt>Best landing</dt><dd class="num {gradeTone(grade(item.best_landing_fpm))}">{item.best_landing_fpm != null ? `${int(item.best_landing_fpm)} fpm` : "–"}</dd></div>
        <div><dt>First flown</dt><dd class="num">{day(item.first_flight_ms)}</dd></div>
        <div><dt>Last flown</dt><dd class="num">{day(item.last_flight_ms)}</dd></div>
        <div class="wide">
          <dt>Current location</dt>
          <dd><span class="num">{item.last_icao ?? "Unknown"}</span> {item.last_name ?? ""}{item.last_country_name ? `, ${item.last_country_name}` : ""}</dd>
        </div>
        <div class="wide"><dt>Aircraft</dt><dd>{item.title || "–"}</dd></div>
      </dl>
    {:else}
      <div class="tools">
        <label class="search">
          <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="6.5" /><path d="M16 16l4.5 4.5" /></svg>
          <input bind:value={query} placeholder="Search flights…" aria-label="Search flights" />
        </label>
        <button class="sort" onclick={() => (newestFirst = !newestFirst)} title={newestFirst ? "Newest first" : "Oldest first"} aria-label="Change order">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 4v16M4.5 7.5 8 4l3.5 3.5M16 20V4M12.5 16.5 16 20l3.5-3.5" /></svg>
        </button>
      </div>
      <ul class="list">
        {#each shown as f (f.id)}
          <li>
            <button class:on={selected === f.id} onclick={() => pick(f)}>
              <span class="r1">
                <strong class="num">{f.callsign || `${f.dep_icao ?? "????"} → ${f.arr_icao ?? "????"}`}</strong>
                <span class="when">{stamp(f.start_ms)}</span>
              </span>
              <span class="r2">
                <span class="num">{f.dep_icao ?? "????"} → {f.arr_icao ?? "????"} · {hhmm(f.total_min)}</span>
                {#if f.landing_rate_fpm != null}<span class="num {gradeTone(f.landing_grade)}">{int(f.landing_rate_fpm)} fpm</span>{/if}
              </span>
            </button>
            {#if selected === f.id && onflight}
              <button class="more" onclick={() => onflight(f.id)}>Full flight info →</button>
            {/if}
          </li>
        {:else}
          <li class="none">{flights.length ? "No flights match." : "Loading flights…"}</li>
        {/each}
      </ul>
    {/if}
  </aside>
  <div class="mapwrap">
    <Map live={[]} aircraft={null} origin={null} {history} {routes} {selected} onroute={(id) => { const f = flights.find((x) => x.id === id); if (f) { tab = "flights"; pick(f); } }} />
  </div>
</div>

<style>
  .detail {
    display: grid;
    grid-template-columns: 400px 1fr;
    height: 100%;
    min-height: 0;
  }
  .side {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--panel);
    border-right: 1px solid var(--rule);
  }
  .mapwrap {
    position: relative;
    min-width: 0;
  }
  .hero {
    position: relative;
    height: 230px;
    flex: none;
    overflow: hidden;
    background: linear-gradient(160deg, #1b58b8, #1747a0 60%, #143e8e);
  }
  .hero img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .bp {
    position: absolute;
    inset: 6px 100px 62px;
  }
  .shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(180deg, rgba(0, 0, 0, 0.35), transparent 30%, transparent 55%, rgba(0, 0, 0, 0.65));
    pointer-events: none;
  }
  .back,
  .change {
    position: absolute;
    top: 10px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border-radius: 6px;
    border: none;
    background: rgba(0, 0, 0, 0.35);
    color: #fff;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .back {
    left: 10px;
  }
  .change {
    right: 10px;
    opacity: 0.85;
  }
  .change:hover,
  .back:hover {
    background: rgba(0, 0, 0, 0.55);
    opacity: 1;
  }
  .back svg,
  .change svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .change input {
    display: none;
  }
  .title {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 10px;
    text-align: center;
    color: #fff;
  }
  .title h2 {
    margin: 0;
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 30px;
    font-weight: 600;
    text-shadow: 0 2px 8px rgba(0, 0, 0, 0.5);
  }
  .title p {
    margin: 0;
    font-size: 14px;
    opacity: 0.9;
  }
  .tabs {
    display: flex;
    gap: 4px;
    margin: 14px 16px 10px;
    padding: 4px;
    border-radius: 8px;
    background: var(--panel-2);
  }
  .tabs button {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--ink-dim);
    padding: 7px;
    border-radius: 6px;
    font-weight: 600;
    font-size: 13px;
  }
  .tabs button.on {
    background: var(--amber);
    color: #1b1406;
  }
  .stats {
    margin: 0;
    padding: 4px 16px 20px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px 16px;
  }
  .stats .wide {
    grid-column: 1 / -1;
  }
  dt {
    font-size: 12px;
    color: var(--ink-dim);
  }
  dd {
    margin: 2px 0 0;
    font-size: 17px;
  }
  .tools {
    display: flex;
    gap: 8px;
    margin: 0 16px 10px;
  }
  .search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    border-radius: 8px;
    background: var(--panel-2);
    border: 1px solid var(--rule);
  }
  .search svg,
  .sort svg {
    width: 17px;
    height: 17px;
    fill: none;
    stroke: var(--ink-dim);
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
    flex: none;
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    color: var(--ink);
    font: inherit;
    padding: 9px 0;
    outline: none;
  }
  .sort {
    width: 42px;
    border-radius: 8px;
    border: 1px solid var(--rule);
    background: var(--panel-2);
    display: grid;
    place-items: center;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .list li > button:first-child {
    width: 100%;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 14px;
    border-radius: 8px;
    border: 1px solid var(--rule);
    background: var(--panel-2);
    color: var(--ink);
  }
  .list li > button:first-child:hover {
    border-color: #555a61;
  }
  .list li > button.on {
    border-color: var(--amber);
    background: #2a2a25;
  }
  .r1,
  .r2 {
    display: flex;
    justify-content: space-between;
    gap: 10px;
  }
  .r1 strong {
    font-size: 18px;
  }
  .when {
    font-size: 12px;
    color: var(--ink-dim);
    white-space: nowrap;
  }
  .r2 {
    font-size: 13px;
    color: var(--ink-dim);
  }
  .more {
    margin-top: 4px;
    border: none;
    background: transparent;
    color: var(--amber);
    font-weight: 600;
    font-size: 13px;
    padding: 2px 4px;
  }
  .none {
    color: var(--ink-dim);
    padding: 8px 2px;
  }
</style>
