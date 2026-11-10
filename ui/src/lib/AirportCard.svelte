<script lang="ts">
  // An airport opened on the map: what and where it is, the latest weather
  // (METAR and TAF from aviationweather.gov, fetched when the card opens),
  // runways, frequencies and your own flights there.
  import { api, type AirportCardData, type AirportWeather } from "./api";
  import { day } from "./format";

  interface Props {
    ident: string;
    onclose: () => void;
    /** Open one of your flights. */
    onflight?: (id: number) => void;
    /** "Zoom to": bring the map in close on the airport. */
    onzoom?: (a: { ident: string; lon: number; lat: number }) => void;
    /** Airport wind arrows on the map, and switching them. */
    windOn?: boolean;
    onwind?: () => void;
  }
  let { ident, onclose, onflight, onzoom, windOn = false, onwind }: Props = $props();

  let card = $state<AirportCardData | null | undefined>(undefined);
  let wx = $state<AirportWeather | null | undefined>(undefined);
  let wxError = $state("");
  let allFreqs = $state(false);

  $effect(() => {
    const id = ident;
    card = undefined;
    wx = undefined;
    wxError = "";
    allFreqs = false;
    api.airportCard(id).then((c) => {
      if (id === ident) card = c ?? null;
    });
    api.airportWeather(id).then(
      (w) => {
        if (id === ident) wx = w ?? null;
      },
      (e) => {
        if (id !== ident) return;
        wx = null;
        wxError = String(e?.message ?? e ?? "Could not load the weather.");
      },
    );
  });

  const ft = (v: number) => v.toLocaleString("en-US");
  const pad3 = (d: number) => String(Math.round(d) % 360 || 360).padStart(3, "0");

  function wind(w: AirportWeather): string {
    if (w.wind_kt == null) return "–";
    if (w.wind_kt === 0) return "Calm";
    const dir = w.wind_dir == null ? "VRB" : `${pad3(w.wind_dir)}°`;
    return `${dir} ${Math.round(w.wind_kt)}${w.gust_kt ? `G${Math.round(w.gust_kt)}` : ""} kt`;
  }
  function vis(v: number | null): string {
    if (v == null) return "–";
    return v >= 10 ? "10+ SM" : `${Number(v.toFixed(2))} SM`;
  }
  function age(ms: number | null): string {
    if (!ms) return "";
    const min = Math.max(0, Math.round((Date.now() - ms) / 60_000));
    return min < 60 ? `${min} min ago` : `${Math.floor(min / 60)} h ${min % 60} min ago`;
  }
  const inHg = (hpa: number) => (hpa * 0.02953).toFixed(2);
  const freqs = $derived(card ? (allFreqs ? card.frequencies : card.frequencies.slice(0, 6)) : []);
</script>

<aside class="ap" aria-label="Airport">
  <button class="x" aria-label="Close" onclick={onclose}>
    <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
  </button>
  {#if card === undefined}
    <p class="dim">Loading {ident}…</p>
  {:else if card === null}
    <p class="dim">No details for {ident}.</p>
  {:else}
    <header>
      <div class="codes">
        <span class="ident">{card.ident}</span>
        {#if card.iata}<span class="iata">{card.iata}</span>{/if}
        <span class="kind">{card.kind}</span>
      </div>
      <h3>{card.name}</h3>
      <p class="where">{[card.city, card.region_name, card.country_name].filter(Boolean).join(", ")}</p>
    </header>

    {#if onzoom || onwind}
      <div class="tools">
        {#if onzoom}
          {@const c = card}
          <button class="zoom" onclick={() => onzoom?.({ ident: c.ident, lon: c.lon, lat: c.lat })}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6" /><path d="M15 15l5 5M10.5 8v5M8 10.5h5" /></svg>
            Zoom to
          </button>
        {/if}
        {#if onwind}
          <button class="wind" class:on={windOn} role="switch" aria-checked={windOn} onclick={onwind} title="Wind arrows at airports on the map">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 20V5M7 9.5L12 4l5 5.5M9 20h6" /></svg>
            Wind arrow {windOn ? "on" : "off"}
          </button>
        {/if}
      </div>
    {/if}

    <div class="facts">
      <div><span class="lbl">Elevation</span><b>{ft(card.elevation_ft)} ft</b></div>
      <div><span class="lbl">Runways</span><b>{card.runways.length || "–"}</b></div>
      <div>
        <span class="lbl">Longest</span>
        <b>{card.runways[0]?.length_ft ? `${ft(card.runways[0].length_ft)} ft` : "–"}</b>
      </div>
    </div>

    <section>
      <h4>Weather</h4>
      {#if wx === undefined}
        <p class="dim">Getting the latest METAR…</p>
      {:else if wx === null}
        <p class="dim">{wxError || "No weather report within 30 nm."}</p>
      {:else}
        <div class="wx-head">
          {#if wx.category}<span class="cat {wx.category.toLowerCase()}">{wx.category}</span>{/if}
          <span class="dim">
            {#if wx.distance_nm > 0}From {wx.station}, {wx.distance_nm} nm away · {/if}{age(wx.observed_ms)}
          </span>
        </div>
        <div class="wx-grid">
          <div><span class="lbl">Wind</span><b>{wind(wx)}</b></div>
          <div><span class="lbl">Visibility</span><b>{vis(wx.visibility_sm)}</b></div>
          <div><span class="lbl">Ceiling</span><b>{wx.ceiling_ft != null ? `${ft(wx.ceiling_ft)} ft` : "None"}</b></div>
          <div>
            <span class="lbl">Temp / dew</span>
            <b>{wx.temp_c != null ? Math.round(wx.temp_c) : "–"}° / {wx.dewpoint_c != null ? Math.round(wx.dewpoint_c) : "–"}° C</b>
          </div>
          <div>
            <span class="lbl">Altimeter</span>
            <b>{wx.altimeter_hpa != null ? `${inHg(wx.altimeter_hpa)} inHg · ${Math.round(wx.altimeter_hpa)} hPa` : "–"}</b>
          </div>
        </div>
        <p class="raw">{wx.metar}</p>
        {#if wx.taf}
          <details>
            <summary>Forecast (TAF)</summary>
            <p class="raw">{wx.taf}</p>
          </details>
        {/if}
      {/if}
    </section>

    {#if card.runways.length}
      <section>
        <h4>Runways</h4>
        <ul class="rwys">
          {#each card.runways as r (r.name)}
            <li>
              <b class="mono">{r.name}</b>
              <span>{r.length_ft ? `${ft(r.length_ft)} × ${r.width_ft || "?"} ft` : "Size not known"}</span>
              <em>{[r.surface, r.lighted ? "Lit" : ""].filter(Boolean).join(" · ")}</em>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if card.frequencies.length}
      <section>
        <details class="fold">
        <summary><h4>Frequencies <span class="count">{card.frequencies.length}</span></h4></summary>
        <ul class="freqs">
          {#each freqs as f, i (i)}
            <li><span class="fk">{f.kind}</span><span class="fd">{f.description}</span><b class="mono">{f.mhz.toFixed(3)}</b></li>
          {/each}
        </ul>
        {#if card.frequencies.length > 6}
          <button class="more" onclick={() => (allFreqs = !allFreqs)}>
            {allFreqs ? "Fewer" : `All ${card.frequencies.length} frequencies`}
          </button>
        {/if}
        </details>
      </section>
    {/if}

    <section>
      <details class="fold">
      <summary><h4>Your flights here <span class="count">{card.departures + card.arrivals || ""}</span></h4></summary>
      {#if card.departures + card.arrivals === 0}
        <p class="dim">You have not flown here yet.</p>
      {:else}
        <div class="facts">
          <div><span class="lbl">Departures</span><b>{card.departures}</b></div>
          <div><span class="lbl">Arrivals</span><b>{card.arrivals}</b></div>
          <div><span class="lbl">Softest landing</span><b>{card.best_landing_fpm != null ? `${Math.round(card.best_landing_fpm)} fpm` : "–"}</b></div>
        </div>
        <ul class="visits">
          {#each card.recent as v (v.id)}
            <li>
              <button onclick={() => onflight?.(v.id)} disabled={!onflight}>
                <b class="mono">{v.dep_icao ?? "?"} → {v.arr_icao ?? "?"}</b>
                <span>{day(v.start_ms)}{v.icao_type ? ` · ${v.icao_type}` : ""}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      </details>
    </section>
    <p class="credit">Airport data from OurAirports. Weather from aviationweather.gov.</p>
  {/if}
</aside>

<style>
  .ap {
    position: absolute;
    top: 74px;
    /* Left of the layers panel when that is open (it keeps the top right). */
    right: var(--card-r, 16px);
    width: 340px;
    max-width: calc(100% - 32px);
    max-height: calc(100% - 90px);
    overflow-y: auto;
    z-index: 4;
    padding: 12px 14px 10px;
    scrollbar-width: thin;
    background: rgba(28, 30, 33, 0.96);
    border: 1px solid var(--rule);
    border-radius: 10px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(6px);
    color: var(--ink);
  }
  .x {
    position: absolute;
    top: 10px;
    right: 10px;
    width: 30px;
    height: 30px;
    padding: 6px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    cursor: pointer;
  }
  .x:hover {
    background: rgba(255, 255, 255, 0.06);
  }
  .x svg {
    width: 18px;
    height: 18px;
    stroke: var(--ink-dim);
    stroke-width: 2;
    fill: none;
  }
  header {
    padding-right: 28px;
  }
  .codes {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .ident {
    font: 600 21px "IBM Plex Mono", ui-monospace, monospace;
    color: var(--amber);
  }
  .iata {
    font: 500 14px "IBM Plex Mono", ui-monospace, monospace;
    color: var(--ink-dim);
  }
  .kind {
    margin-left: auto;
    padding: 2px 8px;
    border: 1px solid var(--rule);
    border-radius: 10px;
    font-size: 12px;
    color: var(--ink-dim);
  }
  h3 {
    margin: 2px 0 0;
    font-family: var(--cond);
    font-size: 18px;
    font-weight: 600;
    line-height: 1.15;
  }
  .where {
    margin: 2px 0 0;
    color: var(--ink-dim);
    font-size: 13px;
  }
  .tools {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  .tools button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .tools button:hover {
    color: var(--ink);
    border-color: #5a5f66;
  }
  .tools .zoom {
    border-color: var(--amber);
    color: var(--amber);
  }
  .tools .wind.on {
    color: #49c27a;
    border-color: #49c27a;
  }
  .tools svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .facts {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
    margin-top: 8px;
  }
  .lbl {
    display: block;
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-faint);
  }
  .facts b,
  .wx-grid b {
    font-weight: 600;
    font-size: 14px;
  }
  section {
    margin-top: 10px;
    padding-top: 9px;
    border-top: 1px solid var(--rule);
  }
  h4 {
    margin: 0 0 6px;
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-dim);
  }
  .fold summary {
    list-style: none;
    cursor: pointer;
    position: relative;
  }
  .fold summary::-webkit-details-marker {
    display: none;
  }
  .fold summary h4 {
    margin: 0;
  }
  .fold summary::after {
    content: "";
    position: absolute;
    right: 4px;
    top: 3px;
    width: 7px;
    height: 7px;
    border-right: 1.6px solid var(--ink-dim);
    border-bottom: 1.6px solid var(--ink-dim);
    transform: rotate(45deg);
    transition: transform 0.15s;
  }
  .fold[open] summary::after {
    transform: rotate(-135deg);
    top: 7px;
  }
  .fold[open] summary {
    margin-bottom: 8px;
  }
  .count {
    margin-left: 4px;
    color: var(--ink-faint);
    letter-spacing: 0;
  }
  .dim {
    color: var(--ink-dim);
    font-size: 13px;
    margin: 0;
  }
  .wx-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }
  .cat {
    padding: 2px 8px;
    border-radius: 4px;
    font: 700 12px "IBM Plex Mono", ui-monospace, monospace;
    color: #111214;
  }
  .cat.vfr {
    background: #5cc08a;
  }
  .cat.mvfr {
    background: #5aa0f0;
  }
  .cat.ifr {
    background: #e26a5a;
  }
  .cat.lifr {
    background: #d36ad6;
  }
  .wx-grid {
    display: grid;
    grid-template-columns: 1.25fr 0.9fr 1fr;
    gap: 6px 10px;
  }
  .wx-grid div:last-child {
    grid-column: span 2;
  }
  .raw {
    margin: 8px 0 0;
    padding: 6px 8px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.25);
    font: 11.5px/1.4 "IBM Plex Mono", ui-monospace, monospace;
    color: var(--ink-dim);
    white-space: pre-wrap;
    word-break: break-word;
  }
  details summary {
    margin-top: 8px;
    cursor: pointer;
    color: var(--ink-dim);
    font-size: 13px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .rwys li {
    display: grid;
    grid-template-columns: 70px auto 1fr;
    align-items: baseline;
    gap: 0 8px;
    padding: 2px 0;
    font-size: 13px;
  }
  .rwys em {
    text-align: right;
    font-style: normal;
    color: var(--ink-faint);
    font-size: 12px;
  }
  .mono {
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-weight: 600;
  }
  .freqs li {
    display: grid;
    grid-template-columns: 52px 1fr auto;
    gap: 8px;
    padding: 3px 0;
    font-size: 13px;
  }
  .fk {
    color: var(--ink-faint);
    font-size: 12px;
  }
  .fd {
    color: var(--ink-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .more {
    margin-top: 6px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--amber);
    font-size: 13px;
    cursor: pointer;
  }
  .visits {
    margin-top: 10px;
  }
  .visits button {
    display: flex;
    justify-content: space-between;
    width: 100%;
    padding: 6px 8px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--ink);
    font-size: 13px;
    cursor: pointer;
    text-align: left;
  }
  .visits button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.05);
  }
  .visits span {
    color: var(--ink-dim);
  }
  .credit {
    margin: 10px 0 0;
    color: var(--ink-faint);
    font-size: 11px;
  }
  /* Phones: across the screen, under the map's menus. */
  @media (max-width: 640px) {
    .ap {
      top: 58px;
      left: 10px;
      right: 10px;
      width: auto;
      max-width: none;
      max-height: calc(100% - 120px);
    }
  }
</style>
