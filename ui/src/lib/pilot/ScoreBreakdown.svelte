<script lang="ts">
  // How one flight's score was made up: each part of landing quality and
  // flight stability with what was measured. How the parts are weighed is
  // in the scoring guide.
  // Narrow containers stack it.
  import type { FlightScore, ScorePart } from "../api";
  import { int } from "../format";

  let { score: f }: { score: FlightScore } = $props();

  const tone = (v: number | null | undefined) => (v == null ? "dim" : v >= 85 ? "good" : v >= 65 ? "warn" : "bad");
</script>

{#snippet bar(label: string, detail: string, part: ScorePart | null, note = "not recorded")}
  <div class="part" class:off={!part}>
    <span class="plabel">{label}</span>
    <span class="detail">{part ? detail : `${detail === "" ? "" : `${detail}, `}${note}`}</span>
    <span class="meter" aria-hidden="true"><i class={tone(part?.score)} style="width:{part ? part.score : 0}%"></i></span>
    <span class="num pscore">{part ? Math.round(part.score) : "–"}</span>
  </div>
{/snippet}

<div class="breakdown">
  {#if f.landing && f.safety}
    {@const l = f.landing}
    {@const s = f.safety}
    <div class="cols">
      <section>
        <h4>Landing quality <b class="num {tone(l.score)}">{Math.round(l.score)}</b></h4>
        {@render bar("Touchdown rate", `${int(l.vs_fpm)} fpm${l.source === "track" ? " (from positions)" : ""}`, l.rate)}
        {@render bar("Touchdown G", l.g ? `${l.g_peak.toFixed(1)} G` : "", l.g)}
        {@render bar("Bounces", l.bounce ? (l.bounces ? String(l.bounces) : "None") : "", l.bounce)}
        <div class="part" class:off={!l.crosswind_bonus}>
          <span class="plabel">Crosswind bonus</span>
          <span class="detail">{l.crosswind_kt >= 0.5 ? `${int(l.crosswind_kt)} kt across` : "No crosswind"}</span>
          <span class="meter" aria-hidden="true"><i class="good" style="width:{(l.crosswind_bonus / 0.25) * 100}%"></i></span>
          <span class="num pscore">+{Math.round(l.crosswind_bonus * 100)}%</span>
        </div>
        {#if l.discarded}
          <p class="notes">
            {l.discarded === 1 ? "An earlier landing (go-around or touch-and-go) was" : `${l.discarded} earlier landings were`} not counted.
          </p>
        {/if}
      </section>
      <section>
        <h4>Flight stability <b class="num {tone(s.score)}">{Math.round(s.score)}</b></h4>
        {@render bar("G-force", s.g ? `${s.g_min.toFixed(1)} to ${s.g_max.toFixed(1)} G` : "", s.g)}
        {@render bar("Bank angle", s.bank ? `up to ${int(s.bank_max)}°` : "aerobatic aircraft", s.bank, "not scored")}
        {@render bar("Pitch", s.pitch ? `${int(s.pitch_max)}° up, ${int(Math.abs(s.pitch_min))}° down` : "aerobatic aircraft", s.pitch, "not scored")}
        {@render bar(
          "Smoothness",
          s.smoothness ? `rolls up to ${int(s.roll_rate_max)}°/s` : s.aerobatic ? "aerobatic aircraft" : "",
          s.smoothness,
          s.aerobatic ? "not scored" : "not recorded",
        )}
        {@render bar(
          "Stable approach",
          s.approach ? `sink up to ${int(s.approach_sink_max)} fpm, bank ${int(s.approach_bank_max)}°` : "",
          s.approach,
        )}
        {@render bar("Speed limit", s.speed ? `${int(s.speed_max_low)} kt top below 10,000 ft` : "", s.speed)}
      </section>
    </div>
  {:else}
    <p class="notes">Not rated: {f.unrated?.toLowerCase()}.</p>
  {/if}
</div>

<style>
  .breakdown {
    container-type: inline-size;
  }
  .cols {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px 28px;
  }
  h4 {
    margin: 6px 0 8px;
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-dim);
    display: flex;
    justify-content: space-between;
  }
  h4 b {
    font-size: 15px;
  }
  .part {
    display: grid;
    grid-template-columns: 112px 1fr 72px 36px;
    grid-template-areas: "label detail meter score";
    align-items: center;
    gap: 8px;
    font-size: 13px;
    padding: 3px 0;
  }
  .part.off {
    opacity: 0.5;
  }
  .plabel {
    grid-area: label;
    color: var(--ink);
  }
  .detail {
    grid-area: detail;
    color: var(--ink-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meter {
    grid-area: meter;
    height: 5px;
    border-radius: 3px;
    background: var(--panel-3);
    overflow: hidden;
  }
  .meter i {
    display: block;
    height: 100%;
    border-radius: 3px;
    background: currentColor;
  }
  .pscore {
    grid-area: score;
    text-align: right;
    font-size: 14px;
  }
  .notes {
    margin: 6px 0 0;
    font-size: 12px;
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
  @container (max-width: 640px) {
    .cols {
      grid-template-columns: 1fr;
    }
    .part {
      grid-template-columns: 1fr 64px 40px;
      grid-template-areas:
        "label meter score"
        "detail detail detail";
      row-gap: 0;
      padding: 4px 0;
    }
    .detail {
      font-size: 12px;
    }
  }
</style>
