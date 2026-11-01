<script lang="ts">
  // Every flight with what it scored, newest first. A rated flight opens to
  // show how its score was made up.
  import type { FlightScore, ScorePart } from "../api";
  import { day, hoursMinutes, int } from "../format";

  let { flights, onflight }: { flights: FlightScore[]; onflight: (id: number) => void } = $props();

  let open = $state<number | null>(null);
  let shown = $state(12);
  const tone = (v: number | null | undefined) => (v == null ? "dim" : v >= 85 ? "good" : v >= 65 ? "warn" : "bad");
  const pct = (p: ScorePart | null) => (p ? `${Math.round(p.weight * 100)}%` : "");
  const delta = (f: FlightScore) => (f.rating_after ?? 0) - (f.rating_before ?? 0);

  function side(ft: number) {
    const v = Math.abs(ft);
    if (v < 1.5) return "on the centerline";
    return `${int(v)} ft ${ft > 0 ? "right" : "left"}`;
  }
</script>

{#snippet bar(label: string, detail: string, part: ScorePart | null, note = "")}
  <div class="part" class:off={!part}>
    <span class="plabel">{label}</span>
    <span class="detail">{detail}</span>
    <span class="meter" aria-hidden="true"><i class={tone(part?.score)} style="width:{part ? part.score : 0}%"></i></span>
    <span class="num pscore">{part ? Math.round(part.score) : "–"}</span>
    <span class="weight">{part ? pct(part) : note}</span>
  </div>
{/snippet}

{#if !flights.length}
  <p class="empty">Your rated flights will show here. Fly with FlightDeck recording, from gate to gate, and every landing and flight gets a score.</p>
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
              <span><em>Safety</em><b class="num {tone(f.safety?.score)}">{Math.round(f.safety!.score)}</b></span>
            </span>
            <span class="num delta" class:up={delta(f) >= 0} class:down={delta(f) < 0}>
              {delta(f) >= 0 ? "▲" : "▼"}
              {Math.abs(delta(f)).toFixed(1)}
            </span>
          {:else}
            <span class="why">{f.unrated}</span>
          {/if}
          <span class="num xp">{f.xp ? `+${int(f.xp)} XP` : "0 XP"}</span>
        </button>
        {#if open === f.flight_id}
          <div class="more">
            {#if rated && f.landing && f.safety}
              {@const l = f.landing}
              {@const s = f.safety}
              <section>
                <h4>Landing quality <b class="num {tone(l.score)}">{Math.round(l.score)}</b></h4>
                {@render bar("Touchdown rate", `${int(l.vs_fpm)} fpm`, l.rate)}
                {@render bar("Touchdown G", `${l.g_peak.toFixed(2)} G`, l.g)}
                {@render bar(
                  "Centerline",
                  l.runway ? `${side(l.runway.offset_ft)}, ${l.runway.airport} ${l.runway.runway}` : "No runway data here",
                  l.centerline,
                  "not scored",
                )}
                {@render bar("Bounces", l.bounces ? String(l.bounces) : "None", l.bounce)}
                <p class="notes">
                  {#if l.crosswind_kt >= 1}{int(l.crosswind_kt)} kt crosswind{l.centerline ? `, centerline +${int(Math.min(25, l.crosswind_kt))}%` : ""}.{/if}
                  {#if l.discarded}{l.discarded === 1 ? "An earlier landing (go-around or touch-and-go) was" : `${l.discarded} earlier landings were`} not counted.{/if}
                </p>
              </section>
              <section>
                <h4>Flight safety <b class="num {tone(s.score)}">{Math.round(s.score)}</b></h4>
                {@render bar("G-force", `${s.g_min.toFixed(1)} to ${s.g_max.toFixed(1)} G`, s.g)}
                {@render bar("Bank angle", s.bank ? `up to ${int(s.bank_max)}°` : "aerobatic aircraft", s.bank, "not scored")}
                {@render bar(
                  "Pitch",
                  s.pitch ? `${int(s.pitch_max)}° up, ${int(Math.abs(s.pitch_min))}° down` : "aerobatic aircraft",
                  s.pitch,
                  "not scored",
                )}
              </section>
              <footer>
                <span>
                  <span>Rating <b class="num">{f.rating_before!.toFixed(1)}</b> → <b class="num">{f.rating_after!.toFixed(1)}</b></span>
                  <small>Expected {f.expected}, scored {Math.round(f.performance!)}, factor {f.k}</small>
                </span>
                <span>
                  <span><b class="num">+{int(f.xp)} XP</b></span>
                  <small>{int(f.xp_base)} for the flight + {f.xp_bonus} for {hoursMinutes(f.airborne_min)} in the air</small>
                </span>
                <button class="link" onclick={() => onflight(f.flight_id)}>Open flight</button>
              </footer>
            {:else}
              <footer>
                <span>
                  Not rated: {f.unrated?.toLowerCase()}.
                  <small>
                    {f.xp
                      ? `${int(f.xp_base)} XP for completing it + ${f.xp_bonus} for ${hoursMinutes(f.airborne_min)} in the air.`
                      : "Flights that end before landing earn no XP."}
                  </small>
                </span>
                <button class="link" onclick={() => onflight(f.flight_id)}>Open flight</button>
              </footer>
            {/if}
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
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px 28px;
    padding: 4px 16px 14px 74px;
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
    align-items: center;
    gap: 8px;
    font-size: 13px;
    padding: 3px 0;
  }
  .part.off {
    opacity: 0.5;
  }
  .plabel {
    color: var(--ink);
  }
  .detail {
    color: var(--ink-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meter {
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
    text-align: right;
    font-size: 14px;
  }
  .weight {
    color: var(--ink-faint);
    font-size: 11px;
    text-align: right;
  }
  .notes {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--ink-dim);
    min-height: 0;
  }
  footer {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 28px;
    border-top: 1px solid var(--rule);
    padding-top: 10px;
    margin-top: 6px;
    font-size: 13px;
  }
  footer > span {
    display: grid;
  }
  footer small {
    color: var(--ink-dim);
    font-size: 12px;
  }
  footer .link {
    margin-left: auto;
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
