<script lang="ts" module>
  import type { Net, NetAirport, NetArea, NetController, NetPilot } from "./api";

  /** Something clicked on the map in an online network's layers. */
  export type NetPick =
    | { kind: "area"; net: Net; area: NetArea }
    | { kind: "airport"; net: Net; airport: NetAirport }
    | { kind: "pilot"; net: Net; pilot: NetPilot };

  export const NET_NAME: Record<Net, string> = { vatsim: "VATSIM", ivao: "IVAO" };
  export const NET_COLOR: Record<Net, string> = { vatsim: "#5cc08a", ivao: "#5aa0f0" };
  /** Position letters and colors, as on the map's airport badges. */
  export const POSITION: Record<string, { letter: string; color: string; name: string }> = {
    DEL: { letter: "D", color: "#4f8fe8", name: "Delivery" },
    GND: { letter: "G", color: "#2fae73", name: "Ground" },
    RMP: { letter: "R", color: "#2fae73", name: "Ramp" },
    TWR: { letter: "T", color: "#d6453d", name: "Tower" },
    APP: { letter: "A", color: "#c0a14a", name: "Approach" },
    DEP: { letter: "P", color: "#c0a14a", name: "Departure" },
    ATIS: { letter: "I", color: "#8c94a0", name: "ATIS" },
    CTR: { letter: "C", color: "#9aa3ad", name: "Enroute" },
    FSS: { letter: "F", color: "#9aa3ad", name: "Flight service" },
  };
</script>

<script lang="ts">
  // The card for something on an online network: a centre's area, an
  // airport's controllers, or a pilot.
  import { api } from "./api";

  interface Props {
    pick: NetPick;
    onclose: () => void;
    /** Open the airport's own card (runways, weather). */
    onairport?: (icao: string) => void;
  }
  let { pick, onclose, onairport }: Props = $props();

  let detail = $state<NetPilot | null>(null);
  $effect(() => {
    const p = pick;
    detail = null;
    if (p.kind === "pilot") api.onlinePilot(p.net, p.pilot.callsign).then((d) => (detail = d ?? null));
  });

  function online(ms: number): string {
    if (!ms) return "–";
    const min = Math.max(0, Math.round((Date.now() - ms) / 60_000));
    return `${String(Math.floor(min / 60)).padStart(2, "0")}:${String(min % 60).padStart(2, "0")}`;
  }
  const who = (c: { name: string | null; id: string }) => c.name || `ID ${c.id}`;
  const order = ["DEL", "GND", "RMP", "TWR", "DEP", "APP", "ATIS"];
  const sorted = (list: NetController[]) => [...list].sort((a, b) => order.indexOf(a.kind) - order.indexOf(b.kind));
  const flight = $derived(pick.kind === "pilot" ? (detail ?? pick.pilot) : null);
</script>

<aside class="nc" aria-label="Online network">
  <button class="x" aria-label="Close" onclick={onclose}>
    <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
  </button>
  <span class="net" style:--c={NET_COLOR[pick.net]}>{NET_NAME[pick.net]}</span>

  {#if pick.kind === "area"}
    {@const a = pick.area}
    <header>
      <h3 class="mono">{a.controllers.map((c) => c.callsign).join(" · ")}</h3>
      <p class="sub">{a.name}{a.oceanic ? " (oceanic)" : ""}</p>
    </header>
    {#each a.controllers as c (c.callsign)}
      <section>
        {#if a.controllers.length > 1}<h4 class="mono">{c.callsign}</h4>{/if}
        <p class="sub">{who(c)}{c.rating ? ` · ${c.rating}` : ""}</p>
        <div class="facts">
          <div><span class="lbl">Facility</span><b>{POSITION[c.kind]?.name ?? c.kind}</b></div>
          <div><span class="lbl">Frequency</span><b class="mono">{c.frequency}</b></div>
          <div><span class="lbl">Time online</span><b class="mono">{online(c.logon_ms)}</b></div>
        </div>
        {#if c.text.length}<p class="text">{c.text.join("\n")}</p>{/if}
      </section>
    {/each}
  {:else if pick.kind === "airport"}
    {@const ap = pick.airport}
    <header>
      <h3 class="mono">{ap.icao}</h3>
      <p class="sub">{ap.name}</p>
    </header>
    <section>
      <ul class="positions">
        {#each sorted(ap.controllers) as c (c.callsign)}
          <li>
            <span class="badge" style:--c={POSITION[c.kind]?.color ?? "#8c94a0"}>{POSITION[c.kind]?.letter ?? "?"}</span>
            <div class="who">
              <b class="mono">{c.callsign}</b>
              <span>{POSITION[c.kind]?.name ?? c.kind}{c.atis_code ? ` ${c.atis_code}` : ""} · {who(c)}</span>
            </div>
            <div class="freq">
              <b class="mono">{c.frequency}</b>
              <span class="mono">{online(c.logon_ms)}</span>
            </div>
            {#if c.text.length}
              <details class="atis">
                <summary>{c.kind === "ATIS" ? "ATIS text" : "Controller info"}</summary>
                <p class="text">{c.text.join("\n")}</p>
              </details>
            {/if}
          </li>
        {/each}
      </ul>
    </section>
    {#if onairport}
      <button class="more" onclick={() => onairport?.(ap.icao)}>Airport info, runways and weather</button>
    {/if}
  {:else if flight}
    {@const p = flight}
    <header>
      <h3 class="mono">{p.callsign}</h3>
      <p class="sub">{who(p)}</p>
    </header>
    <div class="route">
      <span class="mono big">{p.dep || "----"}</span>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12h15M14 7l5 5-5 5" /></svg>
      <span class="mono big">{p.arr || "----"}</span>
      <span class="ac mono">{p.aircraft || "–"}</span>
    </div>
    <div class="facts four">
      <div><span class="lbl">Altitude</span><b class="mono">{p.alt_ft.toLocaleString("en-US")} ft</b></div>
      <div><span class="lbl">Speed</span><b class="mono">{p.gs_kt} kt</b></div>
      <div><span class="lbl">Heading</span><b class="mono">{String(p.hdg).padStart(3, "0")}°</b></div>
      <div><span class="lbl">Online</span><b class="mono">{online(p.logon_ms)}</b></div>
    </div>
    {#if p.flight_rules || p.cruise}
      <p class="sub small">{p.flight_rules === "V" ? "VFR" : p.flight_rules === "I" ? "IFR" : p.flight_rules || ""}{p.cruise ? ` · cruise ${p.cruise}` : ""}</p>
    {/if}
    {#if detail?.route}<p class="text">{detail.route}</p>{/if}
  {/if}
  <p class="credit">Live data from {NET_NAME[pick.net]}. Areas from the VAT-Spy Data Project (CC BY-SA 4.0).</p>
</aside>

<style>
  .nc {
    position: absolute;
    top: 16px;
    left: 16px;
    width: 340px;
    max-width: calc(100% - 32px);
    max-height: calc(100% - 32px);
    overflow-y: auto;
    scrollbar-width: thin;
    z-index: 4;
    padding: 12px 14px 10px;
    background: rgba(28, 30, 33, 0.96);
    border: 1px solid var(--rule);
    border-radius: 10px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
    color: var(--ink);
  }
  .x {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 30px;
    height: 30px;
    padding: 6px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    cursor: pointer;
  }
  .x svg {
    width: 18px;
    height: 18px;
    stroke: var(--ink-dim);
    stroke-width: 2;
    fill: none;
  }
  .net {
    display: inline-block;
    padding: 1px 8px;
    border-radius: 10px;
    border: 1px solid var(--c);
    color: var(--c);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
  }
  header {
    margin-top: 6px;
    padding-right: 24px;
  }
  h3 {
    margin: 0;
    font-size: 20px;
    color: var(--amber);
    word-break: break-word;
  }
  h4 {
    margin: 0 0 2px;
    font-size: 14px;
  }
  .mono {
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-weight: 600;
  }
  .sub {
    margin: 2px 0 0;
    color: var(--ink-dim);
    font-size: 13px;
  }
  .sub.small {
    margin-top: 8px;
  }
  section {
    margin-top: 10px;
    padding-top: 9px;
    border-top: 1px solid var(--rule);
  }
  .facts {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
    margin-top: 8px;
  }
  .facts.four {
    grid-template-columns: repeat(4, 1fr);
  }
  .lbl {
    display: block;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-faint);
  }
  .facts b {
    font-size: 13.5px;
  }
  .text {
    margin: 8px 0 0;
    padding: 6px 8px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.25);
    font: 11.5px/1.4 "IBM Plex Mono", ui-monospace, monospace;
    color: var(--ink-dim);
    white-space: pre-wrap;
    word-break: break-word;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .positions li {
    display: grid;
    grid-template-columns: 24px 1fr auto;
    align-items: center;
    gap: 2px 10px;
    padding: 5px 0;
  }
  .badge {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 5px;
    background: var(--c);
    color: #fff;
    font: 700 12px "IBM Plex Mono", ui-monospace, monospace;
  }
  .who,
  .freq {
    display: grid;
    min-width: 0;
  }
  .who span,
  .freq span {
    color: var(--ink-dim);
    font-size: 12px;
    font-weight: 400;
  }
  .who span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .freq {
    text-align: right;
  }
  .atis {
    grid-column: 2 / -1;
  }
  .atis summary {
    cursor: pointer;
    color: var(--ink-dim);
    font-size: 12px;
  }
  .route {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
  }
  .route .big {
    font-size: 18px;
  }
  .route svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: var(--ink-dim);
    stroke-width: 2;
  }
  .ac {
    margin-left: auto;
    padding: 1px 8px;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.06);
    font-size: 13px;
  }
  .more {
    margin-top: 10px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--amber);
    font-size: 13px;
    cursor: pointer;
  }
  .credit {
    margin: 10px 0 0;
    color: var(--ink-faint);
    font-size: 11px;
  }
</style>
