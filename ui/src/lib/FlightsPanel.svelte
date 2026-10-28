<script lang="ts">
  // Saved flights. Milestone 3 replaces this list with the paper logbook;
  // until then it is a plain, scannable list with a detail view.
  import { api, type FlightDetail, type FlightPlan, type FlightRow, type Totals } from "./api";
  import PlanCard from "./PlanCard.svelte";
  import { day, dec, gradeTone, hhmm, hoursMinutes, int, zulu } from "./format";
  import AltSpeedGraph from "./AltSpeedGraph.svelte";
  import LandingCard from "./LandingCard.svelte";
  import { fromTrack, type Pt } from "./track";
  import { distance } from "./prefs.svelte";

  interface Props {
    refreshKey: number;
    onselect: (track: Pt[] | null, flightId: number | null, plan?: FlightPlan | null) => void;
    /** Open this flight (set when a route is clicked on the map). */
    request?: { id: number; n: number } | null;
    ondelete?: () => void;
  }
  let { refreshKey, onselect, request = null, ondelete }: Props = $props();

  let flights = $state<FlightRow[]>([]);
  let totals = $state<Totals | null>(null);
  let selected = $state<FlightDetail | null>(null);
  let confirmDelete = $state(false);

  async function load() {
    [flights, totals] = await Promise.all([api.flights(), api.totals()]);
  }

  $effect(() => {
    void refreshKey;
    load();
  });

  async function open(id: number) {
    confirmDelete = false;
    exported = null;
    selected = await api.flight(id);
    onselect(selected ? fromTrack(selected.track) : null, selected?.flight.id ?? null, selected?.plan ?? null);
  }

  $effect(() => {
    if (request) open(request.id);
  });

  function close() {
    selected = null;
    onselect(null, null);
  }

  let exported = $state<string | null>(null);
  async function exportOne() {
    if (!selected) return;
    try {
      exported = `Saved to ${await api.exportFlightVolanta(selected.flight.id)}`;
    } catch (e) {
      exported = String(e);
    }
  }

  async function remove() {
    if (!selected) return;
    await api.deleteFlight(selected.flight.id);
    close();
    await load();
    ondelete?.();
  }

  const route = (f: FlightRow) => `${f.dep_icao ?? "????"} to ${f.arr_icao ?? "????"}`;
</script>

{#if selected}
  {@const f = selected.flight}
  <div class="detail">
    <button class="back" onclick={close}>All flights</button>
    <h2 class="num">{f.dep_icao ?? "????"} <span class="to">to</span> {f.arr_icao ?? "????"}</h2>
    <p class="names">{f.dep_name ?? "Unknown departure"}<br />{f.arr_name ?? "Unknown arrival"}</p>
    <p class="meta">
      {[day(f.start_ms), [f.icao_type || f.aircraft_title, f.registration].filter(Boolean).join(" ")].filter(Boolean).join(", ")}
      {#if f.status !== "complete"}<span class="tag">{f.status}</span>{/if}
    </p>
    <dl class="times">
      <div><dt>Block out</dt><dd class="num">{zulu(f.block_out)}</dd></div>
      <div><dt>Takeoff</dt><dd class="num">{zulu(f.takeoff)}</dd></div>
      <div><dt>Landing</dt><dd class="num">{zulu(f.landing)}</dd></div>
      <div><dt>Block in</dt><dd class="num">{zulu(f.block_in)}</dd></div>
      <div><dt>Total</dt><dd class="num">{hhmm(f.total_min)}</dd></div>
      <div><dt>Distance</dt><dd class="num">{distance(f.distance_nm).value} {distance(f.distance_nm).unit}</dd></div>
      <div><dt>Max altitude</dt><dd class="num">{int(f.max_alt_ft)} ft</dd></div>
      <div><dt>Fuel used</dt><dd class="num">{int(f.fuel_used_kg)} kg</dd></div>
    </dl>
    <AltSpeedGraph pts={fromTrack(selected.track)} height={170} />
    {#if selected.plan}<PlanCard plan={selected.plan} />{/if}
    {#each selected.landings.filter((l) => !l.touch_and_go) as l (l.t_ms)}
      <LandingCard landing={l} />
    {/each}
    <div class="export">
      <button class="ghost" onclick={exportOne}>Export as Volanta file</button>
      {#if exported}<p class="saved">{exported}</p>{/if}
    </div>
    <div class="danger-zone">
      {#if confirmDelete}
        <p>Delete this flight and its track? This cannot be undone.</p>
        <button class="danger" onclick={remove}>Delete flight</button>
        <button class="ghost" onclick={() => (confirmDelete = false)}>Keep it</button>
      {:else}
        <button class="ghost" onclick={() => (confirmDelete = true)}>Delete flight</button>
      {/if}
    </div>
  </div>
{:else}
  <div class="list">
    <header>
      <h2>Flights</h2>
      {#if totals && totals.flights > 0}
        <p class="totals">
          {totals.flights} {totals.flights === 1 ? "flight" : "flights"}, {hoursMinutes(totals.minutes)},
          {distance(totals.distance_nm).value} {distance(totals.distance_nm).unit}, {totals.countries} {totals.countries === 1 ? "country" : "countries"}
        </p>
      {/if}
    </header>
    {#if flights.length === 0}
      <p class="empty">
        No flights yet. Fly with MSFS running and each flight appears here when you park, or try a demo flight
        from the map.
      </p>
    {:else}
      <ul>
        {#each flights as f (f.id)}
          <li>
            <button onclick={() => open(f.id)}>
              <span class="r1">
                <span class="num route">{route(f)}</span>
                <span class="num dur">{hhmm(f.total_min)}</span>
              </span>
              <span class="r2">
                <span>{[day(f.start_ms), f.icao_type || f.aircraft_title].filter(Boolean).join(", ")}</span>
                {#if f.landing_rate_fpm != null}
                  <span class="num rate {gradeTone(f.landing_grade)}">{int(f.landing_rate_fpm)} fpm</span>
                {:else if f.status !== "complete"}
                  <span class="tag">{f.status}</span>
                {/if}
              </span>
            </button>
          </li>
        {/each}
      </ul>
      <p class="hint">The paper logbook replaces this list in Milestone 3.</p>
    {/if}
  </div>
{/if}

<style>
  h2 {
    margin: 0;
    font-family: var(--cond);
    font-weight: 600;
    font-size: 28px;
    line-height: 1.1;
  }
  .totals,
  .empty,
  .hint,
  .names,
  .meta {
    color: var(--ink-dim);
    margin: 4px 0 0;
  }
  .hint {
    font-size: 12px;
    color: var(--ink-faint);
    margin-top: 12px;
  }
  .empty {
    margin-top: 16px;
    max-width: 36ch;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 16px 0 0;
    border-top: 1px solid var(--rule);
  }
  li button {
    width: 100%;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 4px;
    border: none;
    border-bottom: 1px solid var(--rule);
    background: transparent;
  }
  li button:hover {
    background: var(--panel-2);
  }
  .r1,
  .r2 {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
  .route {
    font-size: 19px;
    font-weight: 600;
  }
  .dur {
    font-size: 19px;
    color: var(--ink-dim);
  }
  .r2 {
    font-size: 13px;
    color: var(--ink-dim);
  }
  .rate.good {
    color: var(--good);
  }
  .rate.warn {
    color: var(--warn);
  }
  .rate.bad {
    color: var(--bad);
  }
  .tag {
    font-size: 12px;
    color: var(--amber);
    border: 1px solid var(--amber);
    border-radius: 99px;
    padding: 0 7px;
    margin-left: 6px;
  }
  .detail {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .detail h2 {
    font-size: 34px;
  }
  .to {
    font-family: var(--sans);
    font-size: 18px;
    font-weight: 400;
    color: var(--ink-dim);
  }
  .names {
    margin-top: -8px;
  }
  .times {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px 8px;
    margin: 0;
    padding: 12px 0;
    border-top: 1px solid var(--rule);
    border-bottom: 1px solid var(--rule);
  }
  dt {
    font-size: 12px;
    color: var(--ink-dim);
  }
  dd {
    margin: 0;
    font-size: 17px;
  }
  button.back {
    align-self: flex-start;
    border: none;
    background: none;
    color: var(--amber);
    padding: 0;
    font-weight: 600;
  }
  .danger-zone {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .danger-zone p {
    width: 100%;
    margin: 0;
  }
  .danger-zone button {
    border-radius: 6px;
    padding: 7px 12px;
    font-weight: 600;
  }
  .ghost {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
  }
  .danger {
    border: none;
    background: var(--bad);
    color: #fff;
  }
  .export {
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: flex-start;
  }
  .saved {
    margin: 0;
    font-size: 12.5px;
    color: var(--ink-dim);
    word-break: break-all;
  }
</style>
