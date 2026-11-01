<script lang="ts">
  // How one flight's score was made up: each part of landing quality and
  // flight safety, what was worked out rather than measured, and what it
  // did to the rating and XP. Narrow containers stack it.
  import type { FlightScore, ScorePart } from "../api";
  import { hoursMinutes, int } from "../format";

  let { score: f }: { score: FlightScore } = $props();

  const tone = (v: number | null | undefined) => (v == null ? "dim" : v >= 85 ? "good" : v >= 65 ? "warn" : "bad");
  const pct = (p: ScorePart | null) => (p ? `${Math.round(p.weight * 100)}%` : "");
</script>

{#snippet bar(label: string, detail: string, part: ScorePart | null, note = "not recorded")}
  <div class="part" class:off={!part}>
    <span class="plabel">{label}</span>
    <span class="detail">{part ? detail : `${detail === "–" ? "" : `${detail}, `}${note}`}</span>
    <span class="meter" aria-hidden="true"><i class={tone(part?.score)} style="width:{part ? part.score : 0}%"></i></span>
    <span class="num pscore">{part ? Math.round(part.score) : "–"}</span>
    <span class="weight">{part ? pct(part) : ""}</span>
  </div>
{/snippet}

<div class="breakdown">
  {#if f.landing && f.safety}
    {@const l = f.landing}
    {@const s = f.safety}
    <div class="cols">
      <section>
        <h4>Landing quality <b class="num {tone(l.score)}">{Math.round(l.score)}</b></h4>
        {@render bar("Touchdown rate", `${int(l.vs_fpm)} fpm${l.source === "track" ? " (est.)" : ""}`, l.rate)}
        {@render bar("Touchdown G", l.g ? `${l.g_peak.toFixed(2)} G` : "–", l.g)}
        {@render bar("Bounces", l.bounce ? (l.bounces ? String(l.bounces) : "None") : "–", l.bounce)}
        {#if l.discarded}
          <p class="notes">
            {#if l.discarded}{l.discarded === 1 ? "An earlier landing (go-around or touch-and-go) was" : `${l.discarded} earlier landings were`} not counted.{/if}
          </p>
        {/if}
      </section>
      <section>
        <h4>Flight safety <b class="num {tone(s.score)}">{Math.round(s.score)}</b></h4>
        {@render bar("G-force", s.g ? `${s.g_min.toFixed(1)} to ${s.g_max.toFixed(1)} G` : "–", s.g)}
        {@render bar("Bank angle", s.bank ? `up to ${int(s.bank_max)}°` : "aerobatic aircraft", s.bank, "not scored")}
        {@render bar(
          "Pitch",
          s.pitch ? `${int(s.pitch_max)}° up, ${int(Math.abs(s.pitch_min))}° down` : "aerobatic aircraft",
          s.pitch,
          "not scored",
        )}
      </section>
    </div>
    {#if f.estimated.length}
      <div class="estimated">
        <strong>Partly worked out{f.confidence < 1 ? `, counts ${Math.round(f.confidence * 100)}% toward your rating` : ""}</strong>
        <ul>
          {#each f.estimated as e (e)}<li>{e}.</li>{/each}
        </ul>
      </div>
    {/if}
    <div class="sums">
      <span>
        <span>Rating <b class="num">{f.rating_before!.toFixed(1)}</b> → <b class="num">{f.rating_after!.toFixed(1)}</b></span>
        <small>Expected {f.expected}, scored {Math.round(f.performance!)}, factor {f.k}</small>
      </span>
      <span>
        <span><b class="num">+{int(f.xp)} XP</b></span>
        <small>{int(f.xp_base)} for the flight + {f.xp_bonus} for {hoursMinutes(f.airborne_min)} in the air</small>
      </span>
    </div>
  {:else}
    <div class="sums">
      <span>
        <span>Not rated: {f.unrated?.toLowerCase()}.</span>
        <small>
          {f.xp
            ? `${int(f.xp_base)} XP for completing it + ${f.xp_bonus} for ${hoursMinutes(f.airborne_min)} in the air.`
            : "Flights that end before landing earn no XP."}
        </small>
      </span>
    </div>
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
    grid-template-columns: 104px 1fr 72px 28px 36px;
    grid-template-areas: "label detail meter score weight";
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
  .weight {
    grid-area: weight;
    color: var(--ink-faint);
    font-size: 11px;
    text-align: right;
    white-space: nowrap;
  }
  .notes {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--ink-dim);
  }
  .estimated {
    margin-top: 10px;
    padding: 8px 12px;
    border-radius: 8px;
    background: rgba(240, 178, 58, 0.07);
    border: 1px dashed rgba(240, 178, 58, 0.35);
    font-size: 12px;
    color: var(--ink-dim);
  }
  .estimated strong {
    color: var(--amber);
    font-weight: 600;
  }
  .estimated ul {
    margin: 4px 0 0;
    padding-left: 16px;
  }
  .sums {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 28px;
    border-top: 1px solid var(--rule);
    padding-top: 10px;
    margin-top: 10px;
    font-size: 13px;
  }
  .sums > span {
    display: grid;
  }
  .sums small {
    color: var(--ink-dim);
    font-size: 12px;
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
      grid-template-columns: 1fr 56px 24px 30px;
      grid-template-areas:
        "label meter score weight"
        "detail detail detail detail";
      row-gap: 0;
      padding: 4px 0;
    }
    .detail {
      font-size: 12px;
    }
  }
</style>
