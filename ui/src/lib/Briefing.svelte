<script lang="ts">
  // The briefing: SimBrief's OFP as one long document, with the sections
  // pilots look for (times, weights, flight log, winds, ICAO flight plan,
  // weather, NOTAMs) after it. The tabs jump to each one.
  import type { FlightPlan, Ofp } from "./api";

  let { plan, ofp, onclose }: { plan: FlightPlan; ofp: Ofp; onclose: () => void } = $props();

  const SECTIONS = [
    { id: "ofp", label: "OFP" },
    { id: "times", label: "Times" },
    { id: "weights", label: "Weights" },
    { id: "log", label: "Flight log" },
    { id: "winds", label: "Winds" },
    { id: "fpl", label: "ICAO FPL" },
    { id: "weather", label: "Weather" },
    { id: "notams", label: "NOTAMs" },
  ];

  let scroller: HTMLDivElement;
  let active = $state("ofp");

  function go(id: string) {
    const el = scroller?.querySelector<HTMLElement>(`[data-sec="${id}"]`);
    if (el) scroller.scrollTo({ top: el.offsetTop - 8, behavior: "smooth" });
    active = id;
  }

  function spy() {
    let cur = "ofp";
    for (const s of SECTIONS) {
      const el = scroller.querySelector<HTMLElement>(`[data-sec="${s.id}"]`);
      if (el && el.offsetTop - 40 <= scroller.scrollTop) cur = s.id;
    }
    // At the very bottom, the last section is the one being read.
    if (scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 4) cur = SECTIONS[SECTIONS.length - 1].id;
    active = cur;
  }

  // SimBrief's OFP comes as HTML. Keep only plain text formatting: no
  // scripts, images, links or attributes.
  const ALLOWED = new Set(["PRE", "B", "STRONG", "I", "EM", "U", "BR", "DIV", "SPAN", "P", "H1", "H2", "H3", "H4", "TABLE", "THEAD", "TBODY", "TR", "TD", "TH", "FONT"]);
  function clean(html: string): string {
    const doc = new DOMParser().parseFromString(html, "text/html");
    const walk = (node: Element) => {
      for (const child of [...node.children]) {
        if (["SCRIPT", "STYLE", "IFRAME", "OBJECT", "EMBED", "IMG", "SVG", "LINK", "META", "FORM", "INPUT", "BUTTON"].includes(child.tagName)) {
          child.remove();
          continue;
        }
        walk(child);
        for (const a of [...child.attributes]) child.removeAttribute(a.name);
        if (!ALLOWED.has(child.tagName)) child.replaceWith(...child.childNodes);
      }
    };
    walk(doc.body);
    return doc.body.innerHTML;
  }
  const ofpHtml = $derived(ofp.html ? clean(ofp.html) : "");

  const num = (v: unknown) => {
    const n = Number(v);
    return Number.isFinite(n) ? n : null;
  };
  const clock = (v: unknown) => {
    const n = num(v);
    if (!n) return "–";
    const d = new Date(n * 1000);
    return `${String(d.getUTCHours()).padStart(2, "0")}${String(d.getUTCMinutes()).padStart(2, "0")}Z`;
  };
  const dur = (v: unknown) => {
    const n = num(v);
    if (n == null) return "–";
    const m = Math.round(n / 60);
    return `${Math.floor(m / 60)}:${String(m % 60).padStart(2, "0")}`;
  };
  const wt = (v: unknown) => {
    const n = num(v);
    return n == null ? "–" : Math.round(n).toLocaleString("en-US");
  };
  const unit = $derived(ofp.units.startsWith("lb") ? "lb" : "kg");

  const TIMES: [string, string, "clock" | "dur"][] = [
    ["sched_out", "Scheduled out", "clock"],
    ["sched_off", "Scheduled off", "clock"],
    ["sched_on", "Scheduled on", "clock"],
    ["sched_in", "Scheduled in", "clock"],
    ["est_out", "Estimated out", "clock"],
    ["est_off", "Estimated off", "clock"],
    ["est_on", "Estimated on", "clock"],
    ["est_in", "Estimated in", "clock"],
    ["sched_block", "Scheduled block time", "dur"],
    ["est_block", "Estimated block time", "dur"],
    ["est_time_enroute", "Time en route", "dur"],
    ["taxi_out", "Taxi out", "dur"],
    ["taxi_in", "Taxi in", "dur"],
    ["contfuel_time", "Contingency fuel time", "dur"],
    ["reserve_time", "Reserve fuel time", "dur"],
    ["endurance", "Endurance", "dur"],
  ];
  const WEIGHTS: [string, string, string | null][] = [
    ["oew", "Operating empty weight", null],
    ["pax_count", "Passengers", null],
    ["cargo", "Cargo", null],
    ["payload", "Payload", null],
    ["est_zfw", "Zero fuel weight", "max_zfw"],
    ["est_tow", "Takeoff weight", "max_tow"],
    ["est_ldw", "Landing weight", "max_ldw"],
    ["est_ramp", "Ramp weight", null],
  ];
  const FUEL: [string, string][] = [
    ["taxi", "Taxi"],
    ["enroute_burn", "Trip"],
    ["contingency", "Contingency"],
    ["alternate_burn", "Alternate"],
    ["reserve", "Final reserve"],
    ["etops", "ETOPS"],
    ["extra", "Extra"],
    ["min_takeoff", "Minimum takeoff fuel"],
    ["plan_takeoff", "Takeoff fuel"],
    ["plan_ramp", "Block fuel"],
    ["plan_landing", "Landing fuel"],
    ["avg_fuel_flow", "Average fuel flow per hour"],
  ];

  const fl = (ft: number) => (ft >= 18000 ? `FL${String(Math.round(ft / 100)).padStart(3, "0")}` : `${Math.round(ft)}`);
  const wind = (d: number, s: number) => `${String(Math.round(d)).padStart(3, "0")}/${String(Math.round(s)).padStart(2, "0")}`;

  /** Wind levels shown as columns, from the first fix that has them. */
  const levels = $derived((ofp.log.find((r) => r.winds.length)?.winds ?? []).map((w) => w[0]));
  const windRows = $derived(ofp.log.filter((r) => r.winds.length && r.stage !== "CLB" && r.stage !== "DSC").slice(0, 60));

  const notamsByPlace = $derived.by(() => {
    const m = new Map<string, typeof ofp.notams>();
    for (const n of ofp.notams) {
      const k = n.location || "Other";
      if (!m.has(k)) m.set(k, []);
      m.get(k)!.push(n);
    }
    return [...m.entries()];
  });

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={key} />

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="Flight briefing">
    <header>
      <div class="title">
        <span class="kicker">Briefing</span>
        <span class="num">{plan.callsign} · {plan.origin.icao}–{plan.destination.icao}{plan.aircraft_icao ? ` · ${plan.aircraft_icao}` : ""}</span>
      </div>
      <button class="x" aria-label="Close briefing" onclick={onclose}>
        <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
      </button>
    </header>
    <nav aria-label="Briefing sections">
      {#each SECTIONS as s (s.id)}
        <button class:on={active === s.id} onclick={() => go(s.id)}>{s.label}</button>
      {/each}
    </nav>
    <div class="doc" bind:this={scroller} onscroll={spy}>
      <section data-sec="ofp">
        {#if ofpHtml}
          <div class="ofp">{@html ofpHtml}</div>
        {:else}
          <p class="none">SimBrief did not include the OFP text.</p>
        {/if}
      </section>

      <section data-sec="times">
        <h3>Times</h3>
        <table>
          <tbody>
            {#each TIMES.filter(([k]) => ofp.times[k] != null) as [k, label, kind] (k)}
              <tr><th>{label}</th><td>{kind === "clock" ? clock(ofp.times[k]) : dur(ofp.times[k])}</td></tr>
            {/each}
          </tbody>
        </table>
      </section>

      <section data-sec="weights">
        <h3>Weights <small>({unit})</small></h3>
        <table>
          <thead><tr><th></th><td>Planned</td><td>Maximum</td></tr></thead>
          <tbody>
            {#each WEIGHTS.filter(([k]) => ofp.weights[k] != null) as [k, label, max] (k)}
              <tr><th>{label}</th><td>{wt(ofp.weights[k])}</td><td>{max && ofp.weights[max] != null ? wt(ofp.weights[max]) : ""}</td></tr>
            {/each}
          </tbody>
        </table>
        <h4>Fuel <small>({unit})</small></h4>
        <table>
          <tbody>
            {#each FUEL.filter(([k]) => ofp.fuel[k] != null) as [k, label] (k)}
              <tr><th>{label}</th><td>{wt(ofp.fuel[k])}</td></tr>
            {/each}
          </tbody>
        </table>
      </section>

      <section data-sec="log">
        <h3>Flight log</h3>
        <div class="wide">
          <table class="grid">
            <thead>
              <tr><td>Fix</td><td>Via</td><td>Alt</td><td>Trk</td><td>Dist</td><td>Time</td><td>Wind</td><td>OAT</td><td>TAS</td><td>GS</td><td>Fuel</td></tr>
            </thead>
            <tbody>
              {#each ofp.log as r, i (i)}
                <tr class:proc={r.stage !== "CRZ"}>
                  <th>{r.ident}</th><td>{r.airway}</td><td>{fl(r.alt_ft)}</td><td>{String(Math.round(r.track_mag)).padStart(3, "0")}</td>
                  <td>{Math.round(r.dist_leg)}</td><td>{dur(r.time_total)}</td><td>{wind(r.wind_dir, r.wind_kt)}</td>
                  <td>{Math.round(r.oat)}</td><td>{Math.round(r.tas)}</td><td>{Math.round(r.gs)}</td><td>{wt(r.fuel_onboard)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>

      <section data-sec="winds">
        <h3>Winds aloft</h3>
        {#if levels.length && windRows.length}
          <div class="wide">
            <table class="grid">
              <thead><tr><td>Fix</td>{#each levels as l (l)}<td>{fl(l)}</td>{/each}</tr></thead>
              <tbody>
                {#each windRows as r, i (i)}
                  <tr>
                    <th>{r.ident}</th>
                    {#each r.winds as w, j (j)}<td>{wind(w[1], w[2])} <span class="oat">{Math.round(w[3])}</span></td>{/each}
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <p class="none">No winds aloft in this plan.</p>
        {/if}
      </section>

      <section data-sec="fpl">
        <h3>ICAO flight plan</h3>
        {#if ofp.fpl}<pre class="block">{ofp.fpl}</pre>{:else}<p class="none">Not included.</p>{/if}
      </section>

      <section data-sec="weather">
        <h3>Weather</h3>
        {#each ofp.weather as [label, t] (label)}
          <h4>{label}</h4>
          <pre class="block">{t}</pre>
        {:else}
          <p class="none">No weather in this plan.</p>
        {/each}
      </section>

      <section data-sec="notams">
        <h3>NOTAMs</h3>
        {#each notamsByPlace as [place, list] (place)}
          <h4>{place} <small>({list.length})</small></h4>
          {#each list as n, i (n.id || i)}
            <pre class="block notam">{n.id ? `${n.id}  ` : ""}{n.text}</pre>
          {/each}
        {:else}
          <p class="none">No NOTAMs in this plan. Turn NOTAMs on in SimBrief's options to include them.</p>
        {/each}
      </section>
    </div>
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
    width: min(980px, 100%);
    height: 100%;
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    overflow: hidden;
    background: var(--panel);
    border: 1px solid var(--rule);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px 10px;
  }
  .title {
    display: flex;
    flex-direction: column;
  }
  .kicker {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #b9a6fb;
  }
  .title .num {
    font-size: 21px;
    font-weight: 600;
  }
  .x {
    border: none;
    background: transparent;
    color: var(--ink-dim);
    padding: 4px;
    display: grid;
  }
  .x svg {
    width: 20px;
    height: 20px;
    stroke: currentColor;
    stroke-width: 2;
  }
  .x:hover {
    color: var(--ink);
  }
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 0 18px 12px;
    border-bottom: 1px solid var(--rule);
  }
  nav button {
    border: 1px solid transparent;
    background: var(--panel-3);
    color: var(--ink-dim);
    border-radius: 6px;
    padding: 6px 12px;
    font-size: 13px;
    font-weight: 600;
  }
  nav button:hover {
    color: var(--ink);
  }
  nav button.on {
    background: var(--amber);
    color: #1b1406;
  }
  .doc {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 6px 18px 40vh;
    position: relative;
    background: #1a1b1d;
  }
  section {
    padding: 14px 0 6px;
  }
  section + section {
    border-top: 1px dashed #3a3d42;
  }
  h3 {
    margin: 0 0 10px;
    font-family: var(--cond);
    font-size: 18px;
    font-weight: 600;
    color: var(--ink);
  }
  h4 {
    margin: 14px 0 6px;
    font-size: 13px;
    color: var(--ink-dim);
  }
  small {
    font-weight: 400;
    color: var(--ink-faint);
  }
  .ofp,
  .block,
  table {
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 12.5px;
    color: #d9dce0;
  }
  .ofp :global(pre) {
    margin: 0;
    white-space: pre;
    line-height: 1.45;
    font: inherit;
  }
  .ofp {
    overflow-x: auto;
  }
  .ofp :global(b),
  .ofp :global(strong) {
    color: #fff;
  }
  .ofp :global(h1),
  .ofp :global(h2),
  .ofp :global(h3) {
    font-size: 13px;
    margin: 10px 0;
  }
  .block {
    margin: 0 0 8px;
    padding: 8px 10px;
    border-radius: 6px;
    background: var(--panel-2);
    white-space: pre-wrap;
    word-break: break-word;
    line-height: 1.45;
  }
  .notam {
    font-size: 12px;
  }
  table {
    border-collapse: collapse;
  }
  th {
    text-align: left;
    font-weight: 400;
    color: var(--ink-dim);
    padding: 3px 18px 3px 0;
    font-family: var(--sans);
    font-size: 13px;
  }
  td {
    padding: 3px 14px 3px 0;
    text-align: right;
  }
  thead td {
    color: var(--ink-faint);
    font-size: 11px;
  }
  thead td:first-child {
    text-align: left;
  }
  .wide {
    overflow-x: auto;
  }
  .grid th {
    font-family: inherit;
    font-size: 12.5px;
    color: #fff;
  }
  .grid tbody tr:nth-child(odd) {
    background: rgba(255, 255, 255, 0.025);
  }
  .grid tr.proc th,
  .grid tr.proc td {
    color: var(--ink-dim);
  }
  .oat {
    color: var(--ink-faint);
  }
  .none {
    margin: 0;
    color: var(--ink-dim);
    font-style: italic;
  }
</style>
