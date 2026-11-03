<script lang="ts">
  // The scoring guide: how a flight's score, the pilot rating and XP are
  // worked out, written for pilots. Keep it in step with crates/rating
  // (scales.rs) whenever the scoring changes, and bump EDITION.
  import { onMount } from "svelte";
  import { RANKS } from "../api";
  import { int } from "../format";

  let { onclose }: { onclose: () => void } = $props();

  const EDITION = "0.1.37";

  let page = $state<HTMLElement | null>(null);
  onMount(() => {
    page?.focus();
    const key = (e: KeyboardEvent) => e.key === "Escape" && onclose();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  });

  const insignia = (k: (typeof RANKS)[number]) => {
    const i = k.insignia;
    const parts = [];
    parts.push(i.stripes ? `${i.stripes} ${i.thin ? "thin " : ""}stripe${i.stripes > 1 ? "s" : ""}` : "plain board");
    if (i.wings) parts.push(`${i.wings} wings`);
    if (i.oak_leaf) parts.push("gold oak leaf");
    return parts.join(", ");
  };
</script>

<div class="scrim" role="presentation" onclick={onclose}></div>
<div class="sheet" role="dialog" aria-modal="true" aria-label="Scoring guide" tabindex="-1" bind:this={page}>
  <button class="close" onclick={onclose} aria-label="Close the scoring guide">
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
  </button>
  <article>
    <header class="masthead">
      <p class="kicker">FlightDeck · Pilot guide</p>
      <h1>How your flying is scored</h1>
      <p class="lead">
        Every flight you complete earns a score out of 100. Scores build your <b>Pilot Rating</b>, a measure of
        skill that can go up or down, and your <b>XP</b>, which only grows and decides your rank.
      </p>
      <p class="edition">Edition {EDITION}</p>
    </header>

    <section>
      <h2>The flight score</h2>
      <p>
        Half of the score is <b>Landing quality</b> and half is <b>Flight stability</b>. All scores are whole numbers. A
        part that is not quite perfect scores at most 99, so even a small slip shows.
      </p>
      <div class="formula">Flight score = ½ Landing quality + ½ Flight stability</div>
      <p>
        Only what was actually recorded is judged. If a part has no data (an older flight, a file from another tracker,
        or a sim that does not report it), it is left out and the other parts share its weight. Nothing is guessed.
      </p>
    </section>

    <section>
      <h2>Landing quality</h2>
      <p>
        The landing that counts is the last one. A go-around or touch-and-go a minute or more before it is not counted;
        touching down again within a minute counts as a bounce.
      </p>
      <table>
        <thead><tr><th>Part</th><th>Weight</th><th>What scores 100</th></tr></thead>
        <tbody>
          <tr><td>Touchdown rate</td><td>50%</td><td>60 fpm or softer</td></tr>
          <tr><td>Touchdown G</td><td>25%</td><td>1.2 G or less</td></tr>
          <tr><td>Bounces</td><td>25%</td><td>None</td></tr>
        </tbody>
      </table>
      <h3>Touchdown rate</h3>
      <table class="scale">
        <thead><tr><th>Rate</th><td>60</td><td>180</td><td>300</td><td>450</td><td>600</td><td>800</td><td>1,000 fpm</td></tr></thead>
        <tbody><tr><th>Score</th><td>100</td><td>90</td><td>70</td><td>45</td><td>20</td><td>5</td><td>0</td></tr></tbody>
      </table>
      <h3>Touchdown G and bounces</h3>
      <table class="scale">
        <thead><tr><th>Peak G</th><td>1.2</td><td>1.4</td><td>1.7</td><td>2.0</td><td>2.5</td></tr></thead>
        <tbody><tr><th>Score</th><td>100</td><td>85</td><td>55</td><td>25</td><td>0</td></tr></tbody>
      </table>
      <table class="scale">
        <thead><tr><th>Bounces</th><td>0</td><td>1</td><td>2</td><td>3 or more</td></tr></thead>
        <tbody><tr><th>Score</th><td>100</td><td>60</td><td>25</td><td>0</td></tr></tbody>
      </table>
      <p class="note">Scores between the listed points are in proportion.</p>
      <h3>Crosswind bonus</h3>
      <p>
        Landing in a crosswind is harder, so each of the three parts earns <b>1% more per knot</b> of crosswind at
        touchdown, up to 25 knots (+25%). No part can go above 100.
      </p>
    </section>

    <section>
      <h2>Flight stability</h2>
      <p>How steadily and carefully the aircraft was flown, from wheels up to touchdown.</p>
      <table>
        <thead><tr><th>Part</th><th>Weight</th><th>Limit</th></tr></thead>
        <tbody>
          <tr><td>G-force</td><td>20%</td><td>0.5 to 1.6 G</td></tr>
          <tr><td>Bank angle</td><td>15%</td><td>30°</td></tr>
          <tr><td>Pitch</td><td>15%</td><td>20° nose up, 10° nose down</td></tr>
          <tr><td>Smoothness</td><td>20%</td><td>Roll 8°/s, pitch 3°/s, G change 0.5 G/s</td></tr>
          <tr><td>Stable approach</td><td>20%</td><td>See below</td></tr>
          <tr><td>Speed limit</td><td>10%</td><td>250 kt below 10,000 ft</td></tr>
        </tbody>
      </table>
      <h3>How limits cost points</h3>
      <p>
        Inside a limit nothing is lost. Past it, <b>every second costs points</b>, and the cost grows with the square of
        how far past you are: twice as far costs four times as much. A brief slip costs little; holding an extreme
        attitude costs everything.
      </p>
      <p>Bank and pitch are measured in 5° steps. In a one-hour flight:</p>
      <ul class="examples">
        <li>10 seconds at 35–40° of bank: about 5 points of the bank score</li>
        <li>A minute at 35–40° of bank: about 28 points</li>
        <li>A minute at 40–45° of bank: about 77 points</li>
        <li>A minute at 15–20° nose down: about 28 points of the pitch score</li>
      </ul>
      <p>
        G-force is averaged over about a second, so a single bump of turbulence barely counts, while a sustained pull
        or push does.
      </p>
      <h3>Smoothness</h3>
      <p>
        Measured once a second: how quickly the bank changes (roll rate), how quickly the nose moves up or down (pitch
        rate) and how quickly G-force changes. Gentle, deliberate control inputs stay inside the limits; snatching at
        the controls does not. Each second past a limit costs points, growing with the square of the excess.
      </p>
      <ul class="examples">
        <li>5 seconds rolling at 12°/s: about 8 points of the smoothness score</li>
        <li>20 seconds rolling at 14°/s: about 68 points</li>
      </ul>
      <h3>Stable approach</h3>
      <p>
        The approach is worked out backwards from the landing: it is the <b>last descent through 1,000 ft above the
        ground before touchdown</b> (at most the final three minutes). This works for any aircraft and any kind of
        arrival, instrument approach or visual circuit, with no flight plan needed. A go-around that climbs back above
        1,000 ft starts it again.
      </p>
      <ul class="examples">
        <li><b>Sink rate:</b> no faster than 1,000 fpm from 1,000 ft down. 10 seconds at 1,300 fpm costs about 18 points.</li>
        <li><b>Bank:</b> no more than 15° from 500 ft down. 10 seconds at 20° costs about 13 points.</li>
        <li>
          <b>Speed:</b> from 500 ft down to the flare (50 ft), judged only while you are descending, so a stretch flown
          level (a low circuit, say) does not count. Your speed should stay within 10 kt of what it was over the
          surrounding 20 seconds: a surge or a sag costs points, but slowing down steadily toward the runway does not.
          10 seconds 15 kt off costs about 25 points. Slowing by more than 25 kt in total over the final costs 2 points
          for each knot beyond, and speeding up by more than 10 kt the same. Speed can cost at most half of the
          approach score.
        </li>
      </ul>
      <h3>Speed limit</h3>
      <p>
        As in real airspace: no more than 250 knots indicated below 10,000 ft. Each second over costs points, growing
        with the square of the excess. 10 seconds at 260 kt costs about 13 points of the speed score.
      </p>
      <h3>Aerobatic, racing and fighter aircraft</h3>
      <p>
        These are flown hard on purpose, so bank, pitch and smoothness are not judged, and G-force only counts beyond
        about +6 and −3 G. The approach and the speed limit still apply.
      </p>
    </section>

    <section>
      <h2>Pilot Rating</h2>
      <p>
        Everyone starts at <b>1,000</b>. After each rated flight your rating moves by how much better or worse you flew
        than your rating expects:
      </p>
      <div class="formula">New rating = rating + factor × (flight score − expected score) ÷ 100</div>
      <div class="two">
        <table>
          <thead><tr><th>Your rating</th><th>Expected score</th></tr></thead>
          <tbody>
            <tr><td>Below 500</td><td>50</td></tr>
            <tr><td>500 – 749</td><td>60</td></tr>
            <tr><td>750 – 999</td><td>70</td></tr>
            <tr><td>1,000 – 1,249</td><td>75</td></tr>
            <tr><td>1,250 – 1,499</td><td>80</td></tr>
            <tr><td>1,500 – 1,749</td><td>85</td></tr>
            <tr><td>1,750 – 1,999</td><td>90</td></tr>
            <tr><td>2,000 and up</td><td>95</td></tr>
          </tbody>
        </table>
        <table>
          <thead><tr><th>Rated flights so far</th><th>Factor</th></tr></thead>
          <tbody>
            <tr><td>First 5</td><td>48</td></tr>
            <tr><td>6 – 20</td><td>40</td></tr>
            <tr><td>21 – 50</td><td>32</td></tr>
            <tr><td>51 and on</td><td>24</td></tr>
          </tbody>
        </table>
      </div>
      <p>
        So a higher rating has to be earned by flying better: at 1,500 you need more than 85 to keep climbing. A new
        pilot's rating settles quickly; an experienced pilot's moves more slowly.
      </p>
      <p class="example-box">
        Rating 1,000 (expects 75), 30 rated flights (factor 32), flight score 88: the rating rises by
        32 × (88 − 75) ÷ 100 = <b>4</b>, to 1,004. A score of 60 would lower it by 5.
      </p>
    </section>

    <section>
      <h2>XP and ranks</h2>
      <p>
        Every flight that ends with a landing earns XP: its <b>score</b> (a score of 83 earns 83 XP), or <b>50</b> if
        it could not be rated, plus a bonus for time in the air. Time counts only from <b>wheels up to touchdown</b>,
        so time on the ground never earns XP. Flights that end before landing, and demo flights, earn none. XP never
        goes down, and the flight length bonus never changes your rating.
      </p>
      <table class="scale">
        <thead><tr><th>In the air</th><td>under 15 min</td><td>15 min</td><td>30 min</td><td>1 h</td><td>2 h</td><td>4 h</td><td>6 h</td><td>8 h+</td></tr></thead>
        <tbody><tr><th>Bonus XP</th><td>0</td><td>2</td><td>5</td><td>8</td><td>12</td><td>15</td><td>18</td><td>20</td></tr></tbody>
      </table>
      <table>
        <thead><tr><th></th><th>Rank</th><th>XP</th><th>Shoulder board</th></tr></thead>
        <tbody>
          {#each RANKS as k (k.level)}
            <tr><td class="num">{k.level}</td><td>{k.title}</td><td class="num">{k.min_xp ? int(k.min_xp) : "Start"}</td><td>{insignia(k)}</td></tr>
          {/each}
        </tbody>
      </table>
    </section>

    <section>
      <h2>Imported and older flights</h2>
      <p>
        Flights imported from Volanta or Elevatex, and flights FlightDeck recorded before a part was measured, are
        scored on what they have, and count in full like any other flight.
      </p>
      <ul class="examples">
        <li>Bank and pitch come from the attitude the other tracker logged, or from the track (bank from the rate of turn, pitch from the climb and descent).</li>
        <li>The approach comes from the positions near the arrival, using the airport's elevation when no height above the ground was logged.</li>
        <li>Touchdown G, bounces, G-force in flight, smoothness and the speed limit are only judged where they were recorded.</li>
        <li>A flight without a landing report takes its touchdown rate from the positions just before touchdown.</li>
      </ul>
      <p>
        To be rated a flight needs a touchdown rate and at least two minutes of positions in the air, mostly no more
        than 30 seconds apart. Otherwise it shows <i>Not enough data</i> and still earns 50 XP plus its time bonus.
      </p>
    </section>

    <footer>
      Scoring changes as FlightDeck improves; this guide is updated with it, and every flight is scored again with the
      current rules.
    </footer>
  </article>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: rgba(8, 9, 11, 0.72);
    backdrop-filter: blur(3px);
  }
  .sheet {
    position: fixed;
    z-index: 61;
    inset: 28px 0;
    margin: 0 auto;
    width: min(820px, calc(100vw - 32px));
    overflow-y: auto;
    border-radius: 10px;
    background: #f6f1e4;
    color: #26241f;
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.6);
    outline: none;
  }
  .close {
    position: sticky;
    top: 14px;
    float: right;
    margin: 14px 14px 0 0;
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid #d8cfb8;
    background: #fffaf0;
    color: #4a463c;
    z-index: 1;
  }
  .close:hover {
    color: #000;
    border-color: #b08a3a;
  }
  .close svg {
    width: 16px;
    height: 16px;
    stroke: currentColor;
    stroke-width: 2;
  }
  article {
    padding: 44px 64px 52px;
    font-size: 15.5px;
    line-height: 1.6;
  }
  .masthead {
    border-bottom: 3px double #b8ab88;
    padding-bottom: 22px;
    margin-bottom: 8px;
  }
  .kicker {
    margin: 0;
    font-size: 12px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: #9a6d1c;
    font-weight: 600;
  }
  h1 {
    margin: 6px 0 12px;
    font-family: var(--cond);
    font-size: 44px;
    line-height: 1.05;
    font-weight: 700;
    color: #1d1b17;
  }
  .lead {
    margin: 0;
    font-size: 18px;
    color: #3a372f;
  }
  .edition {
    margin: 12px 0 0;
    font-size: 12px;
    color: #8b8471;
  }
  section {
    padding: 22px 0 6px;
    border-bottom: 1px solid #ded5bf;
  }
  h2 {
    margin: 0 0 8px;
    font-family: var(--cond);
    font-size: 28px;
    font-weight: 700;
    color: #1d1b17;
  }
  h3 {
    margin: 20px 0 6px;
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.02em;
    color: #6f5216;
  }
  p {
    margin: 0 0 12px;
  }
  b {
    font-weight: 600;
    color: #1d1b17;
  }
  .formula {
    margin: 14px 0 16px;
    padding: 12px 16px;
    border-left: 3px solid #c39231;
    background: #efe6cf;
    font-family: var(--cond);
    font-size: 18px;
    letter-spacing: 0.01em;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin: 10px 0 14px;
    font-size: 14.5px;
  }
  th,
  td {
    text-align: left;
    padding: 6px 10px;
    border-bottom: 1px solid #e0d7c1;
  }
  thead th {
    font-size: 12px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #7b725d;
    border-bottom: 1.5px solid #bfb393;
  }
  .scale th {
    width: 110px;
    font-size: 12px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #7b725d;
  }
  .scale td {
    text-align: center;
    font-family: var(--cond);
    font-size: 16px;
  }
  .scale thead td {
    color: #4a463c;
    font-size: 14px;
  }
  .num {
    font-family: var(--cond);
  }
  .note {
    font-size: 13px;
    color: #7b725d;
  }
  .examples {
    margin: 4px 0 14px;
    padding-left: 20px;
  }
  .examples li {
    margin: 4px 0;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 22px;
  }
  .example-box {
    padding: 12px 16px;
    border-radius: 6px;
    background: #fffaf0;
    border: 1px solid #e0d7c1;
    font-size: 14.5px;
  }
  footer {
    padding-top: 20px;
    font-size: 13px;
    color: #7b725d;
    font-style: italic;
  }
  @media (max-width: 700px) {
    article {
      padding: 32px 22px 40px;
    }
    .two {
      grid-template-columns: 1fr;
    }
    h1 {
      font-size: 34px;
    }
  }
</style>
