<script lang="ts">
  // Export: pick which flights to save as Volanta-format files.
  import { onMount } from "svelte";
  import { api, type FlightRow } from "./api";
  import { day, hhmm } from "./format";

  let { onclose }: { onclose: () => void } = $props();

  let flights = $state<FlightRow[]>([]);
  let loaded = $state(false);
  let picked = $state<Set<number>>(new Set());
  let query = $state("");
  let busy = $state(false);
  let done = $state<string | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    flights = (await api.flights(100000)) ?? [];
    picked = new Set(flights.map((f) => f.id));
    loaded = true;
  });

  const shown = $derived.by(() => {
    const q = query.trim().toUpperCase();
    if (!q) return flights;
    return flights.filter((f) =>
      [f.dep_icao, f.arr_icao, f.callsign, f.icao_type, f.registration, f.aircraft_title, day(f.start_ms)].some((s) =>
        (s ?? "").toUpperCase().includes(q),
      ),
    );
  });
  const allShownPicked = $derived(shown.length > 0 && shown.every((f) => picked.has(f.id)));

  function toggle(id: number) {
    const n = new Set(picked);
    if (n.has(id)) n.delete(id);
    else n.add(id);
    picked = n;
  }
  function setShown(on: boolean) {
    const n = new Set(picked);
    for (const f of shown) {
      if (on) n.add(f.id);
      else n.delete(f.id);
    }
    picked = n;
  }

  async function run() {
    busy = true;
    error = null;
    try {
      const r = await api.exportAllVolanta([...picked]);
      done = `Saved ${r.count} ${r.count === 1 ? "flight" : "flights"} to ${r.folder}`;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={key} />

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="Export flights">
    <header>
      <div>
        <h3>Export flights</h3>
        <p>Choose the flights to save as Volanta-format files (.json), one per flight, in Downloads\FlightDeck export.</p>
      </div>
      <button class="x" aria-label="Close" onclick={onclose}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg></button>
    </header>
    {#if done}
      <div class="done">
        <p class="ok">{done}</p>
        <button class="primary" onclick={onclose}>Done</button>
      </div>
    {:else}
      <div class="tools">
        <input bind:value={query} placeholder="Search by airport, aircraft, callsign or date…" aria-label="Search flights" />
        <button class="ghost" onclick={() => setShown(!allShownPicked)}>{allShownPicked ? "Select none" : "Select all"}</button>
      </div>
      <ul class="list">
        {#each shown as f (f.id)}
          <li>
            <label class:on={picked.has(f.id)}>
              <input type="checkbox" checked={picked.has(f.id)} onchange={() => toggle(f.id)} />
              <span class="route num">{f.dep_icao ?? "????"} → {f.arr_icao ?? "????"}</span>
              <span class="ac">{[f.callsign, f.icao_type || f.aircraft_title, f.registration].filter(Boolean).join(" · ")}</span>
              <span class="when num">{day(f.start_ms)}</span>
              <span class="dur num">{hhmm(f.total_min)}</span>
            </label>
          </li>
        {:else}
          <li class="none">{loaded ? "No flights match." : "Loading flights…"}</li>
        {/each}
      </ul>
      <footer>
        <span>{picked.size} of {flights.length} selected</span>
        {#if error}<span class="err">{error}</span>{/if}
        <button class="ghost" onclick={onclose}>Cancel</button>
        <button class="primary" onclick={run} disabled={busy || picked.size === 0}>
          {busy ? "Exporting…" : `Export ${picked.size} ${picked.size === 1 ? "flight" : "flights"}`}
        </button>
      </footer>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: rgba(0, 0, 0, 0.55);
    display: grid;
    place-items: center;
    padding: 24px;
  }
  .sheet {
    width: min(760px, 100%);
    max-height: 100%;
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    background: var(--panel);
    border: 1px solid var(--rule);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 18px 10px;
  }
  h3 {
    margin: 0;
    font-family: var(--cond);
    font-size: 22px;
  }
  header p {
    margin: 4px 0 0;
    color: var(--ink-dim);
    font-size: 13px;
  }
  .x {
    border: none;
    background: transparent;
    color: var(--ink-dim);
    align-self: flex-start;
    padding: 2px;
    display: grid;
  }
  .x svg {
    width: 20px;
    height: 20px;
    stroke: currentColor;
    stroke-width: 2;
  }
  .tools {
    display: flex;
    gap: 8px;
    padding: 0 18px 10px;
  }
  .tools input {
    flex: 1;
    font: inherit;
    color: var(--ink);
    background: var(--panel-2);
    border: 1px solid var(--rule);
    border-radius: 6px;
    padding: 7px 10px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0 10px;
    overflow-y: auto;
    flex: 1;
    min-height: 120px;
    border-top: 1px solid var(--rule);
    border-bottom: 1px solid var(--rule);
  }
  label {
    display: grid;
    grid-template-columns: 22px 120px 1fr 96px 46px;
    align-items: center;
    gap: 10px;
    padding: 8px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
  }
  label:hover {
    background: var(--panel-2);
  }
  label:not(.on) {
    color: var(--ink-dim);
  }
  input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: var(--amber);
  }
  .route {
    font-weight: 600;
  }
  .ac {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ink-dim);
  }
  .when,
  .dur {
    text-align: right;
    color: var(--ink-dim);
  }
  .none {
    padding: 16px 8px;
    color: var(--ink-dim);
  }
  footer {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 18px;
  }
  footer span:first-child {
    margin-right: auto;
    color: var(--ink-dim);
    font-size: 13px;
  }
  .err {
    color: var(--bad);
    font-size: 13px;
  }
  .done {
    padding: 8px 18px 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
  }
  .ok {
    margin: 0;
    color: var(--good);
    word-break: break-all;
  }
  .primary {
    border: none;
    border-radius: 6px;
    padding: 8px 14px;
    background: var(--amber);
    color: #1b1406;
    font-weight: 600;
  }
  .primary:disabled {
    opacity: 0.5;
  }
  .ghost {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    border-radius: 6px;
    padding: 7px 12px;
    font-weight: 600;
  }
  .ghost:hover {
    color: var(--ink);
  }
</style>
