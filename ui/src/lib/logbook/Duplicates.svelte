<script lang="ts">
  // Find duplicate flights: the same day, aircraft, departure and arrival.
  // In each set the most complete record is kept and the others are ticked
  // for deletion; nothing is deleted until the pilot confirms the list.
  import { onMount } from "svelte";
  import { api, type DuplicateFlight } from "../api";
  import { day, hhmm, int, zulu } from "../format";

  let { onclose, ondone }: { onclose: () => void; ondone: () => void } = $props();

  let groups = $state<DuplicateFlight[][] | null>(null);
  let picked = $state<Set<number>>(new Set());
  let confirming = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let deleted = $state<number | null>(null);

  onMount(async () => {
    try {
      groups = (await api.findDuplicates()) ?? [];
      // Keep the first (most complete) of each set; tick the rest.
      picked = new Set(groups.flatMap((g) => g.slice(1).map((f) => f.id)));
    } catch (e) {
      error = String(e);
      groups = [];
    }
  });

  function toggle(id: number) {
    const n = new Set(picked);
    if (n.has(id)) n.delete(id);
    else n.add(id);
    picked = n;
  }

  const chosen = $derived((groups ?? []).flat().filter((f) => picked.has(f.id)));
  /** A set where every flight is ticked would lose the flight entirely. */
  const wipes = $derived((groups ?? []).filter((g) => g.every((f) => picked.has(f.id))).length);

  const SOURCE: Record<string, string> = { "": "Recorded", manual: "Entered by hand", volanta: "From Volanta", elevatex: "From Elevatex" };

  async function remove() {
    busy = true;
    error = null;
    try {
      deleted = await api.deleteFlights([...picked]);
      ondone();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
      confirming = false;
    }
  }

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") (confirming ? (confirming = false) : onclose());
  }
</script>

<svelte:window onkeydown={key} />

{#snippet line(f: DuplicateFlight, check: boolean)}
  <span class="route num">{f.dep_icao ?? "????"} → {f.arr_icao ?? "????"}</span>
  <span class="ac">{[f.registration, f.icao_type || f.aircraft_title].filter(Boolean).join(" · ")}</span>
  <span class="when num">{day(f.start_ms)} {zulu(f.block_out ?? f.takeoff ?? f.start_ms)}</span>
  <span class="dur num">{hhmm(f.total_min)}</span>
  <span class="src">
    {SOURCE[f.source] ?? f.source}{f.has_track ? ", with track" : ", no track"}{f.landing_rate_fpm != null ? `, ${int(f.landing_rate_fpm)} fpm` : ""}
  </span>
  {#if check}<span class="tag" class:del={picked.has(f.id)}>{picked.has(f.id) ? "Delete" : "Keep"}</span>{/if}
{/snippet}

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="Duplicate flights">
    <header>
      <div>
        <h3>Duplicate flights</h3>
        <p>Flights on the same day, in the same aircraft, between the same airports. The most complete one in each set is kept.</p>
      </div>
      <button class="x" aria-label="Close" onclick={onclose}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg></button>
    </header>

    {#if deleted != null}
      <div class="done">
        <p class="ok">Deleted {deleted} {deleted === 1 ? "flight" : "flights"}.</p>
        <button class="primary" onclick={onclose}>Done</button>
      </div>
    {:else if groups == null}
      <p class="empty">Looking for duplicates…</p>
    {:else if groups.length === 0}
      <div class="done">
        <p class="empty">No duplicates found. Every flight in your logbook is unique.</p>
        <button class="primary" onclick={onclose}>Done</button>
      </div>
    {:else if confirming}
      <div class="confirm">
        <p>
          <strong>Delete these {chosen.length} {chosen.length === 1 ? "flight" : "flights"}?</strong>
          Their logbook lines, tracks and passport stamps go too. This cannot be undone.
        </p>
        <ul class="list">
          {#each chosen as f (f.id)}<li class="row">{@render line(f, false)}</li>{/each}
        </ul>
        {#if wipes}
          <p class="warn">
            {wipes === 1 ? "One set has" : `${wipes} sets have`} every copy ticked: that flight will be gone from your logbook entirely.
          </p>
        {/if}
        <footer>
          {#if error}<span class="err">{error}</span>{/if}
          <span class="sp"></span>
          <button class="ghost" onclick={() => (confirming = false)} disabled={busy}>Back</button>
          <button class="danger-btn" onclick={remove} disabled={busy}>{busy ? "Deleting…" : `Delete ${chosen.length}`}</button>
        </footer>
      </div>
    {:else}
      <p class="count">{groups.length} {groups.length === 1 ? "set" : "sets"} found. Untick a flight to keep it.</p>
      <div class="groups">
        {#each groups as g, gi (gi)}
          <ul class="list group">
            {#each g as f (f.id)}
              <li>
                <label class="row" class:del={picked.has(f.id)}>
                  <input type="checkbox" checked={picked.has(f.id)} onchange={() => toggle(f.id)} />
                  {@render line(f, true)}
                </label>
              </li>
            {/each}
          </ul>
        {/each}
      </div>
      <footer>
        {#if error}<span class="err">{error}</span>{/if}
        <span class="note">{chosen.length} {chosen.length === 1 ? "flight" : "flights"} selected</span>
        <span class="sp"></span>
        <button class="ghost" onclick={onclose}>Cancel</button>
        <button class="primary" onclick={() => (confirming = true)} disabled={chosen.length === 0}>Review and delete…</button>
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
    width: min(820px, 100%);
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
  header p,
  .count,
  .empty,
  .note {
    margin: 4px 0 0;
    color: var(--ink-dim);
    font-size: 13px;
  }
  .count,
  .empty {
    padding: 0 18px 10px;
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
  .groups {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 14px 6px;
    border-top: 1px solid var(--rule);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .group {
    margin: 10px 0;
    border: 1px solid var(--rule);
    border-radius: 8px;
    overflow: hidden;
  }
  .group li + li {
    border-top: 1px solid var(--rule);
  }
  .row {
    display: grid;
    grid-template-columns: 22px 116px 1fr 120px 42px 58px;
    grid-template-areas:
      "box route ac when dur tag"
      ". src src src src src";
    align-items: center;
    column-gap: 10px;
    row-gap: 2px;
    padding: 8px 10px;
    font-size: 14px;
    cursor: pointer;
  }
  .confirm .row {
    grid-template-columns: 116px 1fr 120px 42px;
    grid-template-areas:
      "route ac when dur"
      "src src src src";
    cursor: default;
    border-bottom: 1px solid var(--rule);
  }
  .row input {
    grid-area: box;
    width: 16px;
    height: 16px;
    accent-color: var(--bad);
  }
  .row.del {
    background: rgba(226, 106, 90, 0.07);
  }
  .route {
    grid-area: route;
    font-weight: 600;
  }
  .ac {
    grid-area: ac;
    color: var(--ink-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .when {
    grid-area: when;
    color: var(--ink-dim);
    text-align: right;
  }
  .dur {
    grid-area: dur;
    text-align: right;
  }
  .src {
    grid-area: src;
    font-size: 12px;
    color: var(--ink-dim);
  }
  .tag {
    grid-area: tag;
    justify-self: end;
    font-size: 11px;
    font-weight: 700;
    padding: 1px 7px;
    border-radius: 999px;
    color: var(--good);
    border: 1px solid currentColor;
  }
  .tag.del {
    color: var(--bad);
  }
  .confirm {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 0 18px;
  }
  .confirm > p {
    margin: 0 0 10px;
    font-size: 14px;
  }
  .confirm .list {
    overflow-y: auto;
    border-top: 1px solid var(--rule);
    max-height: 50vh;
  }
  .warn {
    margin: 10px 0 0;
    font-size: 13px;
    color: var(--amber);
  }
  footer {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 18px;
  }
  .confirm footer {
    padding: 12px 0;
  }
  .sp {
    flex: 1;
  }
  .note {
    margin: 0;
  }
  .err {
    color: var(--bad);
    font-size: 13px;
  }
  .done {
    padding: 0 18px 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-items: flex-start;
  }
  .done .empty {
    padding: 0;
  }
  .ok {
    margin: 0;
    color: var(--good);
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
  .danger-btn {
    border: none;
    border-radius: 6px;
    padding: 8px 14px;
    background: var(--bad);
    color: #fff;
    font-weight: 600;
  }
  .danger-btn:disabled {
    opacity: 0.5;
  }
</style>
