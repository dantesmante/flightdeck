<script lang="ts">
  // The loaded flight plan: callsign, airports, the main numbers and the
  // route string, with a way to remove it.
  import type { FlightPlan } from "./api";
  import { briefing, openBriefing } from "./briefingStore.svelte";
  import { hhmm, int } from "./format";
  import { distance } from "./prefs.svelte";

  let { plan, onremove }: { plan: FlightPlan; onremove?: () => void } = $props();
  let showRoute = $state(false);
  const fl = $derived(plan.cruise_ft >= 18000 ? `FL${Math.round(plan.cruise_ft / 100)}` : `${int(plan.cruise_ft)} ft`);
</script>

<div class="plan">
  <div class="top">
    <span class="src">{plan.source === "simbrief" ? "SimBrief plan" : "Flight plan"}</span>
    {#if onremove}
      <button class="x" onclick={onremove} aria-label="Remove flight plan" title="Remove plan">
        <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
      </button>
    {/if}
  </div>
  <div class="head">
    <span class="cs num">{plan.callsign || "No callsign"}</span>
    {#if plan.aircraft_icao}<span class="ac">{plan.aircraft_icao}{plan.registration ? ` · ${plan.registration}` : ""}</span>{/if}
  </div>
  <div class="legs">
    <div>
      <span class="icao num">{plan.origin.icao}</span>
      <span class="nm">{plan.origin.name}{plan.origin.runway ? `, RWY ${plan.origin.runway}` : ""}</span>
    </div>
    <svg class="arrow" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12h15M14 7l5 5-5 5" /></svg>
    <div class="to">
      <span class="icao num">{plan.destination.icao}</span>
      <span class="nm">{plan.destination.name}{plan.destination.runway ? `, RWY ${plan.destination.runway}` : ""}</span>
    </div>
  </div>
  <dl>
    <div><dt>Cruise</dt><dd class="num">{plan.cruise_ft ? fl : "–"}</dd></div>
    <div><dt>Distance</dt><dd class="num">{plan.distance_nm ? `${distance(plan.distance_nm).value} ${distance(plan.distance_nm).unit}` : "–"}</dd></div>
    <div><dt>Time</dt><dd class="num">{plan.ete_min ? hhmm(plan.ete_min) : "–"}</dd></div>
    <div><dt>Block fuel</dt><dd class="num">{plan.block_fuel_kg ? `${int(plan.block_fuel_kg)} kg` : "–"}</dd></div>
  </dl>
  {#if plan.ofp || plan.source === "simbrief"}
    <button class="brief" onclick={() => openBriefing(plan)} disabled={briefing.loading}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 3h9l4 4v14H6zM14 3v5h5M9 12h7M9 16h7" /></svg>
      {briefing.loading ? "Opening briefing…" : "Briefing (OFP)"}
    </button>
    {#if briefing.error}<p class="err">{briefing.error}</p>{/if}
  {/if}
  {#if plan.route}
    <button class="route" class:open={showRoute} onclick={() => (showRoute = !showRoute)} title="Show the whole route">
      {plan.route}{plan.alternate ? `  ALTN ${plan.alternate}` : ""}
    </button>
  {/if}
</div>

<style>
  .plan {
    border: 1px solid var(--rule);
    border-left: 3px solid #a78bfa;
    border-radius: 8px;
    background: var(--panel-2);
    padding: 10px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .src {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #b9a6fb;
  }
  .x {
    border: none;
    background: transparent;
    color: var(--ink-dim);
    padding: 2px;
    display: grid;
  }
  .x svg {
    width: 16px;
    height: 16px;
    stroke: currentColor;
    stroke-width: 2;
  }
  .x:hover {
    color: var(--ink);
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .cs {
    font-size: 24px;
    font-weight: 600;
    line-height: 1;
  }
  .ac {
    color: var(--ink-dim);
    font-size: 13px;
  }
  .legs {
    display: grid;
    grid-template-columns: 1fr 22px 1fr;
    align-items: start;
    gap: 6px;
  }
  .legs > div {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .to {
    text-align: right;
  }
  .icao {
    font-size: 19px;
    font-weight: 600;
  }
  .nm {
    font-size: 12px;
    color: var(--ink-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .arrow {
    width: 22px;
    height: 22px;
    margin-top: 3px;
    fill: none;
    stroke: #b9a6fb;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
  }
  dt {
    font-size: 11px;
    color: var(--ink-dim);
  }
  dd {
    margin: 0;
    font-size: 15px;
  }
  .route {
    text-align: left;
    border: none;
    padding: 6px 8px;
    border-radius: 6px;
    background: var(--panel);
    color: var(--ink-dim);
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 11.5px;
    line-height: 1.5;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .route.open {
    white-space: normal;
    word-break: break-word;
  }
  .brief {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 7px;
    border: 1px solid rgba(167, 139, 250, 0.45);
    background: rgba(167, 139, 250, 0.1);
    color: #d6ccff;
    font-weight: 600;
    font-size: 14px;
  }
  .brief:hover:not(:disabled) {
    background: rgba(167, 139, 250, 0.2);
    color: #fff;
  }
  .brief svg {
    width: 17px;
    height: 17px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linejoin: round;
  }
  .err {
    margin: 0;
    color: var(--bad);
    font-size: 12.5px;
  }
</style>
