<script lang="ts">
  import type { LandingReport } from "./api";
  import { dec, gradeTone, int } from "./format";

  let { landing, compact = false }: { landing: LandingReport; compact?: boolean } = $props();

  function gradeOf(vs: number) {
    const v = Math.abs(vs);
    return v <= 60 ? "Butter" : v <= 180 ? "Smooth" : v <= 300 ? "Firm" : v <= 600 ? "Hard" : "Very hard";
  }
  const grade = $derived(gradeOf(landing.vs_fpm));
  const tone = $derived(gradeTone(grade));
  const cross = $derived(
    Math.abs(landing.crosswind_kt) < 1
      ? "none"
      : compact
        ? `${int(Math.abs(landing.crosswind_kt))} kt ${landing.crosswind_kt > 0 ? "R" : "L"}`
        : `${int(Math.abs(landing.crosswind_kt))} kt from the ${landing.crosswind_kt > 0 ? "right" : "left"}`,
  );
</script>

<section class="landing" class:compact aria-label="Landing">
  <div class="rate">
    <span class="num fpm">{int(landing.vs_fpm)}</span>
    <span class="unit">fpm</span>
    <span class="grade {tone}">{grade}</span>
  </div>
  <dl>
    <div><dt>Peak G</dt><dd class="num">{dec(landing.g_peak)}</dd></div>
    <div><dt>Bounces</dt><dd class="num">{landing.bounces}</dd></div>
    <div><dt>{compact ? "X-wind" : "Crosswind"}</dt><dd title={compact ? `Crosswind ${cross}` : undefined}>{cross}</dd></div>
  </dl>
  {#if landing.vs_source === "derived"}
    <p class="note">Rate measured by FlightDeck from vertical speed at touchdown.</p>
  {/if}
</section>

<style>
  .landing {
    border: 1px solid var(--rule);
    border-radius: 6px;
    padding: 12px 14px;
    background: var(--panel-2);
  }
  .rate {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }
  .fpm {
    font-size: 40px;
    font-weight: 600;
    line-height: 1;
  }
  .unit {
    color: var(--ink-faint);
    font-size: 13px;
  }
  .grade {
    margin-left: auto;
    font-weight: 600;
    padding: 2px 9px;
    border-radius: 99px;
    font-size: 13px;
  }
  .good {
    color: var(--good);
    background: rgba(92, 192, 138, 0.14);
  }
  .warn {
    color: var(--warn);
    background: var(--amber-glow);
  }
  .bad {
    color: var(--bad);
    background: rgba(226, 106, 90, 0.14);
  }
  dl {
    display: grid;
    grid-template-columns: auto auto 1fr;
    gap: 14px;
    margin: 10px 0 0;
  }
  dt {
    font-size: 12px;
    color: var(--ink-dim);
  }
  dd {
    margin: 0;
    font-size: 15px;
  }
  /* One row: the rate and its grade, the figures beside them. */
  .landing.compact {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 16px;
    padding: 8px 12px;
  }
  .compact .rate {
    gap: 5px;
  }
  .compact .fpm {
    font-size: 26px;
  }
  .compact .grade {
    margin-left: 4px;
    font-size: 12px;
    padding: 1px 8px;
  }
  .compact dl {
    margin: 0 0 0 auto;
    gap: 12px;
  }
  .compact dd {
    font-size: 13.5px;
  }
  .compact dt {
    font-size: 11px;
  }
  .compact .note {
    flex-basis: 100%;
    margin: 0;
  }
  .note {
    margin: 8px 0 0;
    font-size: 12px;
    color: var(--ink-faint);
  }
</style>
