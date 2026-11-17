<script lang="ts" module>
  /** The data FlightDeck uses, with its source and licence. */
  export const DATA_CREDITS: { what: string; who: string; url: string }[] = [
    { what: "Base map", who: "OpenFreeMap · OpenMapTiles · © OpenStreetMap contributors (ODbL)", url: "https://www.openstreetmap.org/copyright" },
    { what: "Airports and runways", who: "OurAirports (public domain)", url: "https://ourairports.com/data/" },
    { what: "Airspace outside the US, private airports, fuel", who: "openAIP (CC BY-NC 4.0)", url: "https://www.openaip.net" },
    { what: "US special use airspace, airports, fixes, obstacles", who: "FAA Aeronautical Data Delivery Service", url: "https://adds-faa.opendata.arcgis.com" },
    { what: "Weather reports and forecasts", who: "NOAA Aviation Weather Center", url: "https://aviationweather.gov" },
    { what: "Winds, temperatures and icing potential", who: "NOAA GFS (NOAA Open Data Dissemination)", url: "https://registry.opendata.aws/noaa-gfs-bdp-pds/" },
    { what: "Rain radar", who: "RainViewer", url: "https://www.rainviewer.com" },
    { what: "Satellite imagery", who: "Esri, Maxar, Earthstar Geographics, and the GIS User Community", url: "https://www.esri.com" },
    { what: "Elevation and terrain", who: "Terrain Tiles (Mapzen, AWS Open Data; USGS, SRTM, GMTED, ETOPO1)", url: "https://registry.opendata.aws/terrain-tiles/" },
    { what: "Aircraft photos", who: "Planespotters.net (photographers credited on each photo)", url: "https://www.planespotters.net" },
    { what: "Online networks", who: "VATSIM data feed", url: "https://vatsim.dev" },
    { what: "Flight plans", who: "SimBrief by Navigraph", url: "https://www.simbrief.com" },
  ];
</script>

<script lang="ts">
  import type maplibregl from "maplibre-gl";
  import { openExternal as openLink } from "./external";

  let { getMap, open = $bindable(false) }: { getMap: () => maplibregl.Map | undefined; open?: boolean } = $props();

  /** The credits of the sources on the map right now, as plain text. */
  let live = $state<string[]>([]);
  function readLive() {
    const out = new Set<string>();
    const style = getMap()?.getStyle();
    for (const s of Object.values(style?.sources ?? {})) {
      const a = (s as { attribution?: string }).attribution;
      if (!a) continue;
      const text = new DOMParser().parseFromString(a, "text/html").body.textContent?.replace(/\s+/g, " ").trim();
      if (text) out.add(text);
    }
    live = [...out];
  }
  $effect(() => {
    if (open) readLive();
  });
  let card = $state<HTMLElement | null>(null);
  function outside(e: PointerEvent) {
    if (open && card && !card.contains(e.target as Node) && !(e.target as Element)?.closest?.(".fd-credits-btn")) open = false;
  }
</script>

<svelte:window onpointerdown={outside} onkeydown={(e) => e.key === "Escape" && (open = false)} />

{#if open}
  <div class="credits" bind:this={card} role="dialog" aria-label="Credits">
    <header>
      <strong>Credits</strong>
      <button type="button" class="x" aria-label="Close credits" onclick={() => (open = false)}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
      </button>
    </header>
    {#if live.length}
      <section>
        <h4>On the map now</h4>
        <ul class="live">
          {#each live as l (l)}<li>{l}</li>{/each}
        </ul>
      </section>
    {/if}
    <section>
      <h4>Data FlightDeck uses</h4>
      <dl>
        {#each DATA_CREDITS as c (c.what)}
          <dt>{c.what}</dt>
          <dd><a href={c.url} onclick={(e) => (e.preventDefault(), openLink(c.url))}>{c.who}</a></dd>
        {/each}
      </dl>
    </section>
    <p class="foot">FlightDeck is free and open source (MIT). Not for real-world navigation.</p>
  </div>
{/if}

<style>
  .credits {
    position: absolute;
    right: calc(var(--inset-r, 0px) + 52px);
    bottom: calc(var(--inset-b, 0px) + 10px);
    z-index: 6;
    width: min(380px, calc(100% - 70px));
    max-height: calc(100% - 80px);
    overflow-y: auto;
    padding: 12px 14px 10px;
    border-radius: 12px;
    border: 1px solid var(--rule, #34383e);
    background: rgba(28, 30, 33, 0.97);
    color: var(--ink, #eceef1);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.45);
    font-size: 12.5px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }
  header strong {
    font-size: 14px;
  }
  .x {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--ink-dim, #a8adb4);
    cursor: pointer;
  }
  .x:hover {
    background: rgba(255, 255, 255, 0.07);
  }
  .x svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
  h4 {
    margin: 10px 0 4px;
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-faint, #7d828a);
  }
  .live {
    margin: 0;
    padding-left: 16px;
    color: var(--ink-dim, #a8adb4);
  }
  dl {
    margin: 0;
    display: grid;
    gap: 2px 0;
  }
  dt {
    margin-top: 4px;
    color: var(--ink, #eceef1);
    font-weight: 600;
  }
  dd {
    margin: 0;
  }
  a {
    color: var(--ink-dim, #a8adb4);
    text-decoration: none;
  }
  a:hover {
    color: var(--amber, #f0b23a);
    text-decoration: underline;
  }
  .foot {
    margin: 12px 0 0;
    color: var(--ink-faint, #7d828a);
    font-size: 11.5px;
  }
</style>
