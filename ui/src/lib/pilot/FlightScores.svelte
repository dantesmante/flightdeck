<script lang="ts">
  // Every flight with what it scored, newest first. A rated flight opens to
  // show how its score was made up.
  import type { FlightScore } from "../api";
  import { day, int } from "../format";
  import ScoreBreakdown from "./ScoreBreakdown.svelte";

  let { flights, onflight }: { flights: FlightScore[]; onflight: (id: number) => void } = $props();

  let open = $state<number | null>(null);
  let shown = $state(12);
  const tone = (v: number | null | undefined) => (v == null ? "dim" : v >= 85 ? "good" : v >= 65 ? "warn" : "bad");
  const delta = (f: FlightScore) => Math.round(f.rating_after ?? 0) - Math.round(f.rating_before ?? 0);

</script>

{#if !flights.length}
  <p class="empty">Your flights and their scores will show here. Fly with FlightDeck recording, or import flights from Volanta or Elevatex in Settings.</p>
{:else}
  <ol class="list">
    {#each flights.slice(0, shown) as f (f.flight_id)}
      {@const rated = f.performance != null}
      <li class:rated class:open={open === f.flight_id}>
        <button class="row" onclick={() => (open = open === f.flight_id ? null : f.flight_id)} aria-expanded={open === f.flight_id}>
          <span class="badge num {tone(f.performance)}" class:none={!rated}>{rated ? Math.round(f.performance!) : "–"}</span>
          <span class="what">
            <strong class="num">{f.dep_icao || "····"} <span class="arrow">→</span> {f.arr_icao || "····"}</strong>
            <small>{day(f.start_ms)}{f.icao_type ? ` · ${f.icao_type}` : ""}</small>
          </span>
          {#if rated}
            <span class="split">
              <span><em>Landing</em><b class="num {tone(f.landing?.score)}">{Math.round(f.landing!.score)}</b></span>
              <span><em>Stability</em><b class="num {tone(f.safety?.score)}">{Math.round(f.safety!.score)}</b></span>
            </span>
            <span class="num delta" class:up={delta(f) >= 0} class:down={delta(f) < 0}>
              {delta(f) >= 0 ? "▲" : "▼"}
              {Math.abs(delta(f))}
            </span>
          {:else}
            <span class="why">{f.unrated}</span>
          {/if}
          <span class="num xp">{f.xp ? `+${int(f.xp)} XP` : "0 XP"}</span>
        </button>
        {#if open === f.flight_id}
          <div class="more">
            <ScoreBreakdown score={f} />
            <button class="link" onclick={() => onflight(f.flight_id)}>Open flight</button>
          </div>
        {/if}
      </li>
    {/each}
  </ol>
  {#if flights.length > shown}
    <button class="more-btn" onclick={() => (shown += 20)}>Show more ({flights.length - shown} left)</button>
  {/if}
{/if}

<style>
  .empty {
    color: var(--ink-dim);
    margin: 0;
    padding: 18px;
    border: 1px dashed var(--rule);
    border-radius: 10px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  li {
    border: 1px solid var(--rule);
    border-radius: 10px;
    background: var(--panel-2);
    overflow: hidden;
  }
  li:not(.rated) {
    background: transparent;
  }
  li.open {
    border-color: #4a4f57;
  }
  .row {
    width: 100%;
    display: grid;
    grid-template-columns: 46px minmax(140px, 1fr) auto 78px 88px;
    align-items: center;
    gap: 14px;
    padding: 10px 14px;
    border: none;
    background: transparent;
    text-align: left;
  }
  .row:hover {
    background: rgba(255, 255, 255, 0.025);
  }
  .badge {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    font-size: 19px;
    font-weight: 600;
    border: 2px solid currentColor;
  }
  .badge.none {
    color: var(--ink-faint);
    border-style: dashed;
  }
  .what {
    display: grid;
    min-width: 0;
  }
  .what strong {
    font-size: 17px;
    letter-spacing: 0.02em;
  }
  .arrow {
    color: var(--ink-faint);
  }
  .what small {
    color: var(--ink-dim);
    font-size: 12px;
  }
  .split {
    display: flex;
    gap: 16px;
  }
  .split span {
    display: grid;
    justify-items: end;
  }
  .split em {
    font-style: normal;
    font-size: 11px;
    color: var(--ink-faint);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .split b {
    font-size: 17px;
  }
  .why {
    color: var(--ink-faint);
    font-size: 13px;
    grid-column: span 2;
    text-align: right;
  }
  .delta {
    font-size: 15px;
    text-align: right;
  }
  .up {
    color: var(--good);
  }
  .down {
    color: var(--bad);
  }
  .xp {
    text-align: right;
    color: var(--amber);
    font-size: 15px;
  }
  li:not(.rated) .xp {
    color: var(--ink-dim);
  }
  .good {
    color: var(--good);
  }
  .warn {
    color: var(--amber);
  }
  .bad {
    color: var(--bad);
  }
  .dim {
    color: var(--ink-faint);
  }
  .more {
    padding: 4px 16px 14px 74px;
  }
  .more > .link {
    display: block;
    margin: 10px 0 0 auto;
  }
  .link {
    border: 1px solid var(--rule);
    background: var(--panel-3);
    border-radius: 6px;
    padding: 6px 12px;
    color: var(--ink);
    font-size: 13px;
  }
  .link:hover {
    border-color: var(--amber);
    color: var(--amber);
  }
  .more-btn {
    margin-top: 10px;
    width: 100%;
    padding: 9px;
    border: 1px dashed var(--rule);
    border-radius: 8px;
    background: transparent;
    color: var(--ink-dim);
  }
  .more-btn:hover {
    color: var(--ink);
  }
</style>
