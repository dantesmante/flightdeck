<script lang="ts">
  // One aircraft: its picture and registration, statistics, and every
  // flight flown in it, listed and drawn on a map.
  import { onMount } from "svelte";
  import { api, type FlightRow, type HangarItem, type Route } from "../api";
  import { day, gradeTone, hhmm, hoursMinutes, int } from "../format";
  import Map from "../Map.svelte";
  import { distance } from "../prefs.svelte";
  import { fromTrack, type Pt } from "../track";
  import { openExternal } from "../external";
  import AircraftPicture from "./AircraftPicture.svelte";
  import { airlineLook } from "./airlines";
  import { spotters } from "./spotters.svelte";

  interface Props {
    item: HangarItem;
    photo: string | null;
    displayName: string;
    onback: () => void;
    /** Edit the registration, model and airline. */
    onedit?: () => void;
    onupload: (e: Event) => void;
    onflight?: (id: number) => void;
  }
  let { item, photo, displayName, onback, onedit, onupload, onflight }: Props = $props();

  let tab = $state<"stats" | "flights">("flights");
  let flights = $state<FlightRow[]>([]);
  let routes = $state<Route[]>([]);
  let selected = $state<number | null>(null);
  let history = $state<Pt[] | null>(null);
  let query = $state("");
  let newestFirst = $state(true);
  let cardW = $state(380);

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
  const look = $derived(airlineLook(item.airline));
  const spot = $derived(!photo && item.registration ? spotters[item.registration.trim().toUpperCase()] : null);
</script>

<div class="detail">
  <Map
    live={[]}
    aircraft={null}
    origin={null}
    {history}
    {routes}
    {selected}
    initialOn={["routes", "airports"]}
    insetLeft={cardW + 16}
    onroute={(id) => {
      const f = flights.find((x) => x.id === id);
      if (f) {
        tab = "flights";
        pick(f);
      }
    }}
  />

  <!-- The aircraft, as its hangar card, floating over the map with its
       statistics and flights. -->
  <article
    class="card"
    class:airline={!!look}
    style:--a={look?.colors[0]}
    style:--b={look?.colors[1]}
    style:--c={look?.colors[2] ?? look?.colors[1]}
    bind:clientWidth={cardW}
    aria-label="Aircraft {item.registration || displayName}"
  >
    <div class="banner">
      <button class="back" onclick={onback} title="Back to Hangar" aria-label="Back to Hangar">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
      </button>
      <h2>{displayName}{item.icao_type ? ` · ${item.icao_type}` : ""}</h2>
      {#if look}
        <span class="al">{look.name}</span>
        <svg class="swoosh" viewBox="0 0 30 40" preserveAspectRatio="none" aria-hidden="true">
          <path d="M30 4 C 28 18, 25 30, 20 40 L 30 40 Z" fill={look.colors[1]} />
          <path d="M21 0 L30 0 L30 4 C 28 18, 25 30, 20 40 L0 40 C 10 33, 18 20, 21 0 Z" fill={look.colors[2] ?? look.colors[1]} />
        </svg>
      {/if}
    </div>
    <div class="picture">
      <AircraftPicture {item} own={photo} variant="hero" credit={false} />
      <span class="reg num">{item.registration || item.icao_type || "No reg."}</span>
      <div class="pic-tools">
        {#if onedit}
          <button class="chip" onclick={onedit} title="Edit registration, model and airline">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16v4zM13.5 6.5l4 4" /></svg>
            Edit
          </button>
        {/if}
        <label class="chip">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 18h10a4 4 0 0 0 .6-7.96A6 6 0 0 0 6.1 9.1 4.5 4.5 0 0 0 7 18zM12 16v-6M9.5 12.5 12 10l2.5 2.5" /></svg>
          {photo ? "Change image" : "Upload image"}
          <input type="file" accept="image/*" onchange={onupload} />
        </label>
      </div>
    </div>
    {#if spot}
      <a class="credit" href={spot.link} onclick={(e) => { e.preventDefault(); openExternal(spot.link); }}>Photo © {spot.photographer || "Photographer"} · Planespotters.net</a>
    {/if}
    <div class="where">
      <span class="lbl">Current aircraft location</span>
      <span class="icao num">{item.last_icao ?? "—"}</span>
      <span class="apt">{item.last_name ?? (item.flights ? "Last position unknown" : "Not flown yet")}{item.last_country_name ? `, ${item.last_country_name}` : ""}</span>
    </div>
    <dl class="foot">
      <div><dt>Flights</dt><dd class="num">{item.flights}</dd></div>
      <div><dt>Hours</dt><dd class="num">{hoursMinutes(item.minutes)}</dd></div>
      <div><dt>Avg. landing</dt><dd class="num {gradeTone(grade(item.avg_landing_fpm))}">{item.avg_landing_fpm != null ? `${int(item.avg_landing_fpm)} fpm` : "–"}</dd></div>
    </dl>

    <div class="tabs" role="tablist">
      <button role="tab" aria-selected={tab === "stats"} class:on={tab === "stats"} onclick={() => (tab = "stats")}>Statistics</button>
      <button role="tab" aria-selected={tab === "flights"} class:on={tab === "flights"} onclick={() => (tab = "flights")}>Flights ({flights.length})</button>
    </div>

    {#if tab === "stats"}
      <dl class="stats">
        <div><dt>Distance</dt><dd class="num">{dist.value} <small>{dist.unit}</small></dd></div>
        <div><dt>Landings</dt><dd class="num">{item.landings}</dd></div>
        <div><dt>Best landing</dt><dd class="num {gradeTone(grade(item.best_landing_fpm))}">{item.best_landing_fpm != null ? `${int(item.best_landing_fpm)} fpm` : "–"}</dd></div>
        <div><dt>First flown</dt><dd class="num">{day(item.first_flight_ms)}</dd></div>
        <div><dt>Last flown</dt><dd class="num">{day(item.last_flight_ms)}</dd></div>
        <div class="wide"><dt>Aircraft in the sim</dt><dd>{item.title || "–"}</dd></div>
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
          <li class="none">{flights.length ? "No flights match." : item.flights ? "Loading flights…" : "No flights yet."}</li>
        {/each}
      </ul>
    {/if}
  </article>
</div>

<style>
  .detail {
    position: relative;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }
  .card {
    position: absolute;
    z-index: 3;
    top: 16px;
    left: 16px;
    max-height: calc(100% - 32px);
    width: 380px;
    max-width: calc(100% - 32px);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    border-radius: 12px;
    background: rgba(38, 40, 44, 0.97);
    border: 1px solid rgba(255, 255, 255, 0.07);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
    color: var(--ink);
  }
  /* The banner, as on the hangar card: the model and the airline's colours. */
  .banner {
    position: relative;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 44px;
    padding: 0 12px 0 6px;
    background: #2b2e32;
  }
  .card.airline .banner {
    background: var(--a);
    color: #fff;
    padding-right: 36px;
  }
  .banner h2 {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .card.airline .banner h2 {
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.35);
  }
  .back {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.18);
    color: inherit;
    cursor: pointer;
  }
  .back:hover {
    background: rgba(0, 0, 0, 0.35);
  }
  .back svg,
  .chip svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .al {
    flex: none;
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
    font-weight: 700;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.45);
  }
  .swoosh {
    position: absolute;
    top: 0;
    right: 0;
    width: 30px;
    height: 100%;
  }
  .picture {
    position: relative;
    flex: none;
    aspect-ratio: 2 / 1;
    overflow: hidden;
    background: #222428;
  }
  .reg {
    position: absolute;
    top: 8px;
    right: 8px;
    padding: 3px 12px;
    border-radius: 999px;
    background: #4a4d52;
    color: #fff;
    font-size: 16px;
    font-weight: 700;
    letter-spacing: 0.02em;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
  }
  .card.airline .reg {
    background: var(--a);
  }
  .pic-tools {
    position: absolute;
    left: 8px;
    bottom: 8px;
    display: flex;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 9px;
    border-radius: 6px;
    border: none;
    background: rgba(0, 0, 0, 0.45);
    color: #fff;
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .chip:hover {
    background: rgba(0, 0, 0, 0.65);
  }
  .chip input {
    display: none;
  }
  .credit {
    display: block;
    padding: 4px 12px 0;
    font-size: 11px;
    color: var(--ink-dim);
    text-decoration: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .credit:hover {
    color: var(--ink);
    text-decoration: underline;
  }
  .where {
    display: flex;
    flex-direction: column;
    padding: 9px 12px 8px;
    border-bottom: 1px solid var(--rule);
  }
  .lbl,
  dt {
    font-size: 10px;
    white-space: nowrap;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--ink-dim);
  }
  .icao {
    font-size: 19px;
    font-weight: 600;
    line-height: 1.2;
  }
  .apt {
    font-size: 12px;
    color: var(--ink-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .foot {
    margin: 0;
    display: grid;
    grid-template-columns: 0.8fr 0.8fr 1.3fr;
    padding: 8px 4px 9px;
    text-align: center;
    border-bottom: 1px solid var(--rule);
  }
  .foot dd {
    margin: 2px 0 0;
    font-size: 15px;
    font-weight: 600;
  }
  .tabs {
    flex: none;
    display: flex;
    gap: 4px;
    margin: 12px 12px 10px;
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
    padding: 4px 14px 18px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px 16px;
  }
  .stats .wide {
    grid-column: 1 / -1;
  }
  .stats dd {
    margin: 2px 0 0;
    font-size: 17px;
  }
  .tools {
    display: flex;
    gap: 8px;
    margin: 0 12px 10px;
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
    padding: 8px 0;
    outline: none;
  }
  .sort {
    width: 40px;
    border-radius: 8px;
    border: 1px solid var(--rule);
    background: var(--panel-2);
    display: grid;
    place-items: center;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .list li > button:first-child {
    width: 100%;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 10px 12px;
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
    font-size: 16px;
  }
  .when {
    font-size: 11.5px;
    color: var(--ink-dim);
    white-space: nowrap;
  }
  .r2 {
    font-size: 12.5px;
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
