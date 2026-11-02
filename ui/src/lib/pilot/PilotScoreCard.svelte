<script lang="ts">
  // One flight's pilot score, for the side panel: the score, landing and
  // stability, what it did to the rating, XP towards the next rank, and how
  // it was made up.
  import type { FlightScore, Standing } from "../api";
  import { hoursMinutes, int } from "../format";
  import ScoreBreakdown from "./ScoreBreakdown.svelte";
  import ScoreRing from "./ScoreRing.svelte";

  let { score, standing = null, open = false }: { score: FlightScore; standing?: Standing | null; open?: boolean } = $props();

  let showing = $state(false);
  $effect(() => {
    showing = open;
  });
  const tone = (v: number | null | undefined) => (v == null ? "dim" : v >= 85 ? "good" : v >= 65 ? "warn" : "bad");
  const rated = $derived(score.performance != null);
  const delta = $derived(Math.round(score.rating_after ?? 0) - Math.round(score.rating_before ?? 0));
  const progress = $derived.by(() => {
    if (!standing) return 0;
    if (!standing.next_rank) return 1;
    return Math.max(0, Math.min(1, (standing.xp - standing.rank.min_xp) / (standing.next_rank.min_xp - standing.rank.min_xp)));
  });
  const verdict = $derived(
    score.performance == null
      ? "Not rated"
      : score.performance >= 92
        ? "Outstanding"
        : score.performance >= 85
          ? "Excellent"
          : score.performance >= 75
            ? "Good"
            : score.performance >= 65
              ? "Fair"
              : "Needs work",
  );
</script>

<section class="card" aria-label="Pilot score">
  <header>
    <h3>Pilot score</h3>
  </header>
  <div class="top">
    <ScoreRing value={score.performance} label={verdict} size={92} />
    <div class="facts">
      {#if rated}
        <div class="pair">
          <span><em>Landing</em><b class="num {tone(score.landing?.score)}">{Math.round(score.landing!.score)}</b></span>
          <span><em>Stability</em><b class="num {tone(score.safety?.score)}">{Math.round(score.safety!.score)}</b></span>
        </div>
        <p class="rating">
          Rating <b class="num">{int(score.rating_after)}</b>
          <span class="num" class:up={delta >= 0} class:down={delta < 0}>{delta >= 0 ? "▲" : "▼"} {Math.abs(delta)}</span>
        </p>
      {:else}
        <p class="why">{score.unrated}</p>
      {/if}
    </div>
  </div>
  <div class="xp">
    <div class="xpline">
      <b class="num">+{int(score.xp)} XP</b>
      {#if standing}
        <span>{standing.rank.title}{standing.next_rank ? `, ${int(standing.next_rank.min_xp - standing.xp)} to ${standing.next_rank.title}` : ""}</span>
      {/if}
    </div>
    {#if standing}
      <div class="bar" aria-hidden="true"><i style="width:{progress * 100}%"></i></div>
    {/if}
  </div>
  {#if rated}
    <button class="toggle" onclick={() => (showing = !showing)} aria-expanded={showing}>
      {showing ? "Hide details" : "How it was scored"}
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d={showing ? "M6 15l6-6 6 6" : "M6 9l6 6 6-6"} /></svg>
    </button>
    {#if showing}
      <div class="details"><ScoreBreakdown {score} /></div>
    {/if}
  {:else}
    <p class="note">
      {score.xp
        ? `${int(score.xp_base)} XP for completing it + ${score.xp_bonus} for ${hoursMinutes(score.airborne_min)} in the air.`
        : "Flights that end before landing earn no XP."}
    </p>
  {/if}
</section>

<style>
  .card {
    margin: 14px 0;
    padding: 14px 16px 10px;
    border-radius: 12px;
    background: linear-gradient(160deg, rgba(240, 178, 58, 0.08), rgba(240, 178, 58, 0) 55%), var(--panel-2);
    border: 1px solid var(--rule);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }
  h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--ink-dim);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 18px;
  }
  .facts {
    flex: 1;
    min-width: 0;
  }
  .pair {
    display: flex;
    gap: 22px;
  }
  .pair span {
    display: grid;
  }
  .pair em {
    font-style: normal;
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-faint);
  }
  .pair b {
    font-size: 26px;
    line-height: 1.1;
  }
  .rating {
    margin: 8px 0 0;
    font-size: 13px;
    color: var(--ink-dim);
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  .rating b {
    color: var(--ink);
    font-size: 16px;
  }
  .why {
    margin: 0;
    color: var(--ink-dim);
    font-size: 13px;
  }
  .up {
    color: var(--good);
  }
  .down {
    color: var(--bad);
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
  .xp {
    margin-top: 12px;
  }
  .xpline {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-size: 12px;
    color: var(--ink-dim);
  }
  .xpline b {
    color: var(--amber);
    font-size: 15px;
  }
  .bar {
    margin-top: 5px;
    height: 5px;
    border-radius: 3px;
    background: var(--panel-3);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: linear-gradient(90deg, #b8862b, var(--amber));
  }
  .toggle {
    margin-top: 10px;
    width: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 6px;
    padding: 6px;
    border: none;
    border-top: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    font-size: 13px;
  }
  .toggle:hover {
    color: var(--amber);
  }
  .toggle svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }
  .details {
    padding: 4px 0 6px;
  }
  .note {
    margin: 8px 0 4px;
    font-size: 12px;
    color: var(--ink-faint);
  }
</style>
