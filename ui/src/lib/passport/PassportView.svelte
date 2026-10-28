<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  import { api, type CountryVisit, type PassportData, type Stamp as StampT } from "../api";
  import Book from "../Book.svelte";
  import { hoursMinutes } from "../format";
  import { prefs, setPref } from "../prefs.svelte";
  import Emblem from "./Emblem.svelte";
  import { holoLines, mrz, paperPattern, passportNumber } from "./paper";
  import Stamp from "./Stamp.svelte";
  import WorldMap from "./WorldMap.svelte";

  let { onsettings }: { onsettings: () => void } = $props();

  const W = 360;
  const H = 510;
  const PER_PAGE = 6;
  const INDEX_ROWS = 16;

  type Page =
    | { kind: "map"; half: "left" | "right" }
    | { kind: "inner" }
    | { kind: "data" }
    | { kind: "visa"; stamps: StampT[]; no: number }
    | { kind: "index"; countries: CountryVisit[]; first: boolean }
    | { kind: "blank" };

  let data = $state<PassportData | null>(null);
  type World = { w: number; h: number; countries: { c: string; n: string; d: string; dot?: [number, number] }[] };
  let world = $state<World | null>(null);
  let flags = $state<Record<string, string>>({});
  /** Pointer position over the photo, 0..100, for the hologram's shimmer. */
  let mx = $state(50);
  let my = $state(50);
  function tilt(e: PointerEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    mx = ((e.clientX - r.left) / r.width) * 100;
    my = ((e.clientY - r.top) / r.height) * 100;
  }
  let spread = $state(-1);
  let book = $state<Book | null>(null);

  /** Cover colours, all real passport shades that carry gold foil well. */
  const COVERS = [
    { id: "burgundy", name: "Burgundy", color: "#5a1e2a" },
    { id: "red", name: "Red", color: "#7a1c24" },
    { id: "navy", name: "Navy", color: "#1b2a4e" },
    { id: "blue", name: "Royal blue", color: "#22407a" },
    { id: "teal", name: "Teal", color: "#144548" },
    { id: "green", name: "Green", color: "#1f4030" },
    { id: "brown", name: "Brown", color: "#4a2d1e" },
    { id: "black", name: "Black", color: "#18191d" },
  ];
  const coverId = $derived(prefs.s.passport_color ?? "burgundy");
  const coverColor = $derived((COVERS.find((c) => c.id === coverId) ?? COVERS[0]).color);

  onMount(async () => {
    const [d, w, f] = await Promise.all([api.passport(), import("./world.json"), import("./flags.json")]);
    data = d;
    world = w.default as World;
    flags = f.default as Record<string, string>;
  });

  const visited = $derived(new Set((data?.countries ?? []).map((c) => c.country)));

  const pages = $derived.by<Page[]>(() => {
    if (!data) return [];
    const out: Page[] = [{ kind: "map", half: "left" }, { kind: "map", half: "right" }, { kind: "inner" }, { kind: "data" }];
    const n = Math.max(2, Math.ceil(data.stamps.length / PER_PAGE));
    for (let i = 0; i < n; i++) {
      out.push({ kind: "visa", stamps: data.stamps.slice(i * PER_PAGE, (i + 1) * PER_PAGE), no: i + 1 });
    }
    if (out.length % 2 === 1) out.push({ kind: "visa", stamps: [], no: n + 1 });
    const c = data.countries;
    const indexPages = Math.max(1, Math.ceil(c.length / INDEX_ROWS));
    for (let i = 0; i < indexPages; i++) {
      out.push({ kind: "index", countries: c.slice(i * INDEX_ROWS, (i + 1) * INDEX_ROWS), first: i === 0 });
    }
    if (out.length % 2 === 1) out.push({ kind: "blank" });
    return out;
  });


  const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  const printed = (ms: number | null | undefined) => {
    if (ms == null) return "–";
    const d = new Date(ms);
    return `${String(d.getUTCDate()).padStart(2, "0")} ${months[d.getUTCMonth()].toUpperCase()} ${d.getUTCFullYear()}`;
  };

  /** Seeded spot for each stamp: a 2 x 3 grid, nudged so it looks hand-placed. */
  function slot(s: StampT, i: number) {
    const col = i % 2;
    const row = Math.floor(i / 2);
    const jx = (((s.seed >>> 3) % 29) - 14) * 1.0;
    const jy = (((s.seed >>> 9) % 23) - 11) * 1.0;
    return { x: 24 + col * 160 + 78 + jx, y: 60 + row * 140 + 68 + jy };
  }

  const nameParts = $derived.by(() => {
    const parts = (data?.pilot.name ?? "").trim().split(/\s+/).filter(Boolean);
    return parts.length > 1
      ? { surname: parts[parts.length - 1], given: parts.slice(0, -1).join(" ") }
      : { surname: parts[0] ?? "", given: "" };
  });
  const number = $derived(data ? passportNumber(data.pilot.pilot_id, data.pilot.name) : "");
  const lines = $derived(
    data
      ? mrz({ name: data.pilot.name || "Pilot", nationality: data.pilot.nationality, number, issued: data.first_flight_ms ?? Date.now() })
      : ["", ""],
  );
  const stateCode = $derived(
    data && /^[A-Za-z]{3}$/.test(data.pilot.nationality) ? data.pilot.nationality.toUpperCase() : "UTO",
  );
  const pageLabel = $derived(`Pages ${spread * 2 + 1} and ${spread * 2 + 2} of ${pages.length}`);
</script>

{#snippet coverView()}
  <button class="cover" style:background-color={coverColor} onclick={() => book?.next()} aria-label="Open passport">
    <span class="foil title">Pilot passport</span>
    <span class="foil-emblem"><Emblem size={170} color="#d8b46a" /></span>
    <span class="foil brand">FlightDeck</span>
    <span class="chip" aria-hidden="true"><svg viewBox="0 0 40 26"><rect x="1" y="1" width="38" height="24" rx="3" /><circle cx="20" cy="13" r="6" /><path d="M1 13h13M26 13h13" /></svg></span>
  </button>
{/snippet}

{#snippet paper(content: Snippet, no: number | null = null)}
  <div class="paper" style:background-image={paperPattern()}>
    {@render content()}
    {#if no != null}<span class="folio">{no}</span>{/if}
  </div>
{/snippet}

{#snippet pageView(i: number)}
  {@const p = pages[i]}
  {#if p.kind === "map"}
    {@const mp = p}
    {#snippet mapPage()}
      <div class="endpaper" class:right={mp.half === "right"}>
        <h3>{mp.half === "left" ? "Countries visited" : visited.size ? `${visited.size} ${visited.size === 1 ? "country" : "countries"}` : "No countries yet"}</h3>
        <div class="mapbox">
          {#if world}
            <WorldMap {world} {visited} half={mp.half} width={340} />
          {/if}
        </div>
        {#if mp.half === "left"}
          <p class="legend"><span class="swatch"></span>Countries you have landed in</p>
        {:else if data && data.countries.length}
          <p class="legend first">First country: {data.countries[0].name}</p>
        {/if}
      </div>
    {/snippet}
    {@render paper(mapPage)}
  {:else if p.kind === "inner"}
    {#snippet inner()}
      <div class="inner">
        <div class="watermark"><Emblem size={260} color="#7f9f9b" /></div>
        <p class="lead">This passport records every border its bearer has crossed by air.</p>
        <p>A stamp is issued each time you land in a different country from the one you left.</p>
        <p>The first landing in any country receives a first-entry seal.</p>
        <p class="small">Issued by FlightDeck. Valid for all simulators.</p>
      </div>
    {/snippet}
    {@render paper(inner)}
  {:else if p.kind === "data" && data}
    {#snippet dataPage()}
      <div class="data">
        <div class="head">
          <span class="type">Pilot passport</span>
          <span class="mini"><Emblem size={30} color="#5d4a7a" /></span>
        </div>
        <div class="grid">
          <div
            class="photo"
            onpointermove={tilt}
            style:--mx="{mx}%"
            style:--my="{my}%"
            style:--lines={holoLines()}
            role="presentation"
          >
            {#if data!.pilot.photo_url}
              <img src={data!.pilot.photo_url} alt="Pilot" />
            {:else}
              <button class="photo-empty" onclick={onsettings}>
                <svg viewBox="0 0 100 120" aria-hidden="true"><circle cx="50" cy="42" r="22" /><path d="M10 120 C 12 82, 88 82, 90 120 Z" /></svg>
                <span>Add a photo</span>
              </button>
            {/if}
            <span class="holo" aria-hidden="true"></span>
          </div>
          <dl>
            <div class="row3">
              <div><dt>Type</dt><dd>P</dd></div>
              <div><dt>Code</dt><dd>{stateCode}</dd></div>
              <div><dt>Passport no.</dt><dd>{number}</dd></div>
            </div>
            <div><dt>Surname</dt><dd>{nameParts.surname.toUpperCase() || "—"}</dd></div>
            <div><dt>Given names</dt><dd>{nameParts.given.toUpperCase() || "—"}</dd></div>
            <div><dt>Nationality</dt><dd>{data!.pilot.nationality.toUpperCase() || "—"}</dd></div>
            <div><dt>Date of first flight</dt><dd>{printed(data!.first_flight_ms)}</dd></div>
            <div><dt>Home airport</dt><dd>{data!.pilot.home_icao || "—"}{data!.home_name ? ` ${data!.home_name.toUpperCase()}` : ""}</dd></div>
            <div class="row2">
              <div><dt>Flight time</dt><dd>{hoursMinutes(data!.totals.minutes).toUpperCase()}</dd></div>
              <div><dt>Countries</dt><dd>{data!.countries.length}</dd></div>
            </div>
          </dl>
        </div>
        <div class="sig">
          <dt>Holder's signature</dt>
          <div class="hand">{data!.pilot.name || ""}</div>
        </div>
        {#if !data!.pilot.name}
          <button class="fill-in" onclick={onsettings}>Fill in your details</button>
        {/if}
        <div class="mrz" aria-label="Machine-readable zone">
          <div>{lines[0]}</div>
          <div>{lines[1]}</div>
        </div>
      </div>
    {/snippet}
    {@render paper(dataPage)}
  {:else if p.kind === "visa"}
    {@const vp = p}
    {#snippet visa()}
      <div class="visa">
        <h3>Visas</h3>
        {#if vp.stamps.length === 0 && vp.no === 1}
          <p class="empty">No stamps yet. Fly to another country and your first stamps appear here.</p>
        {/if}
        {#each vp.stamps as s, j (`${s.flight_id}-${s.kind}`)}
          {@const at = slot(s, j)}
          <div class="slot" style:left="{at.x}px" style:top="{at.y}px">
            <Stamp stamp={s} size={s.kind === "first_entry" ? 142 : 132} flag={flags[s.country] ?? null} />
          </div>
        {/each}
      </div>
    {/snippet}
    {@render paper(visa, i + 1)}
  {:else if p.kind === "index"}
    {@const ip = p}
    {#snippet index()}
      <div class="index">
        <h3>{ip.first ? "Countries visited" : "Countries visited, continued"}</h3>
        {#if ip.countries.length === 0}
          <p class="empty">Countries appear here as you fly into them.</p>
        {:else}
          <ol>
            {#each ip.countries as c (c.country)}
              <li>
                <span class="cn">{c.name}</span>
                <span class="dots"></span>
                <span class="since">{printed(c.first_ms)}</span>
              </li>
            {/each}
          </ol>
        {/if}
      </div>
    {/snippet}
    {@render paper(index, i + 1)}
  {:else}
    {#snippet blank()}<div></div>{/snippet}
    {@render paper(blank, i + 1)}
  {/if}
{/snippet}

<section class="passport">
  {#if !data}
    <p class="loading">Opening your passport…</p>
  {:else}
    <div class="toolbar">
      {#if spread < 0}
        <span class="where">
          {data.stamps.length} {data.stamps.length === 1 ? "stamp" : "stamps"}, {data.countries.length}
          {data.countries.length === 1 ? "country" : "countries"}. Click the cover to open.
        </span>
        <div class="colours" role="radiogroup" aria-label="Cover colour">
          <span class="clabel">Cover</span>
          {#each COVERS as c (c.id)}
            <button
              class="colour"
              class:on={c.id === coverId}
              style:--c={c.color}
              role="radio"
              aria-checked={c.id === coverId}
              aria-label={c.name}
              title={c.name}
              onclick={() => setPref("passport_color", c.id)}
            ></button>
          {/each}
        </div>
      {:else}
        <button class="ghost" onclick={() => book?.close()}>Close passport</button>
        <span class="where">{pageLabel}</span>
      {/if}
    </div>
    <div class="stage">
      <Book bind:this={book} bind:spread pageCount={pages.length} pageWidth={W} pageHeight={H} page={pageView} cover={coverView} label="Passport" />
    </div>
  {/if}
</section>

<style>
  .passport {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    background:
      radial-gradient(ellipse at 50% 40%, rgba(240, 178, 58, 0.06), transparent 60%),
      linear-gradient(180deg, #1d1f22, #141517);
  }
  .loading {
    margin: auto;
    color: var(--ink-dim);
  }

  /* Cover ------------------------------------------------------------ */
  .cover {
    position: relative;
    width: 100%;
    height: 100%;
    border: none;
    border-radius: 6px 14px 14px 6px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    padding: 58px 0 52px;
    color: #d8b46a;
    background-image:
      linear-gradient(90deg, rgba(0, 0, 0, 0.35), rgba(0, 0, 0, 0) 7%, rgba(255, 255, 255, 0.05) 9%, rgba(0, 0, 0, 0) 12%),
      radial-gradient(ellipse at 30% 20%, rgba(255, 255, 255, 0.09), transparent 55%),
      url("data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='180'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='3'/%3E%3CfeColorMatrix values='0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.35 0'/%3E%3C/filter%3E%3Crect width='180' height='180' filter='url(%23n)'/%3E%3C/svg%3E");
    box-shadow:
      inset 0 0 0 2px rgba(0, 0, 0, 0.25),
      inset -6px 0 12px rgba(0, 0, 0, 0.35);
    transition: background-color 0.35s, filter 0.2s;
    cursor: pointer;
  }
  .cover:hover {
    filter: brightness(1.12);
  }
  .colours {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-left: auto;
  }
  .clabel {
    color: var(--ink-dim);
    font-size: 13px;
    margin-right: 4px;
  }
  .colour {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    padding: 0;
    border: 2px solid rgba(255, 255, 255, 0.12);
    background: radial-gradient(circle at 35% 30%, rgba(255, 255, 255, 0.22), transparent 60%), var(--c);
    cursor: pointer;
    transition: transform 0.15s, border-color 0.15s, box-shadow 0.15s;
  }
  .colour:hover {
    transform: scale(1.15);
  }
  .colour.on {
    border-color: #d8b46a;
    box-shadow: 0 0 0 3px rgba(216, 180, 106, 0.25);
  }
  .foil {
    font-family: "Source Serif 4", Georgia, serif;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    background: linear-gradient(180deg, #f3dc9e 0%, #c99a48 45%, #f0d48d 60%, #a87a35 100%);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    filter: drop-shadow(0 1px 0 rgba(0, 0, 0, 0.45));
  }
  .title {
    font-size: 25px;
  }
  .brand {
    font-size: 17px;
    letter-spacing: 0.32em;
  }
  .foil-emblem {
    filter: drop-shadow(0 1px 0 rgba(0, 0, 0, 0.5)) drop-shadow(0 -1px 0 rgba(255, 230, 170, 0.15));
  }
  .chip svg {
    width: 34px;
    fill: none;
    stroke: #d8b46a;
    stroke-width: 1.6;
  }

  /* Open book -------------------------------------------------------- */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 46px;
    padding: 14px 20px 0;
  }
  .where {
    color: var(--ink-dim);
    font-size: 13px;
  }
  .ghost {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    border-radius: 6px;
    padding: 6px 12px;
    font-weight: 600;
  }
  .ghost:hover:not(:disabled) {
    color: var(--ink);
  }
  .ghost:disabled {
    opacity: 0.4;
  }
  .stage {
    flex: 1;
    min-height: 0;
  }

  /* Paper ------------------------------------------------------------ */
  .paper {
    position: relative;
    width: 100%;
    height: 100%;
    background-color: #f3eedf;
    color: #3a3446;
    font-family: "Source Serif 4", Georgia, serif;
  }
  .folio {
    position: absolute;
    bottom: 12px;
    left: 0;
    right: 0;
    text-align: center;
    font-size: 11px;
    color: #8a8296;
  }
  .endpaper {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 18px 0 0 20px;
  }
  .endpaper.right {
    padding: 18px 20px 0 0;
  }
  .endpaper h3 {
    padding: 0 20px;
  }
  .mapbox {
    margin-top: 72px;
    display: flex;
  }
  .endpaper:not(.right) .mapbox {
    justify-content: flex-end;
  }
  .legend {
    margin: 34px 24px 0;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-style: italic;
    color: #5f5770;
  }
  .legend.first {
    justify-content: flex-end;
  }
  .swatch {
    width: 22px;
    height: 13px;
    border: 1px solid #2f5a55;
    background: repeating-linear-gradient(45deg, #5f8f88 0 2px, #3f6f69 2px 3px);
  }
  .inner {
    padding: 70px 44px;
    position: relative;
    height: 100%;
  }
  .inner p {
    position: relative;
    font-size: 14px;
    line-height: 1.6;
    max-width: 30ch;
    margin: 0 0 14px;
  }
  .inner .lead {
    font-size: 17px;
    font-style: italic;
  }
  .inner .small {
    font-size: 12px;
    color: #6f6880;
    margin-top: 26px;
  }
  .watermark {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    opacity: 0.18;
  }

  .data {
    position: relative;
    height: 100%;
    padding: 26px 24px 18px;
    display: flex;
    flex-direction: column;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid #b9b0c6;
    padding-bottom: 8px;
  }
  .type {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #5d4a7a;
  }
  .grid {
    display: grid;
    grid-template-columns: 108px 1fr;
    gap: 14px;
    margin-top: 14px;
  }
  .photo {
    width: 108px;
    height: 140px;
    border-radius: 4px;
    overflow: hidden;
    background: #ddd6e6;
  }
  /* Hologram: a translucent rainbow film over the photo that shows when
     the pointer is over it and shimmers as the pointer moves. */
  .photo {
    position: relative;
  }
  .holo {
    position: absolute;
    inset: 0;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.35s ease;
    /* Saturated rainbow that slides with the pointer, seen only through the
       line art mask: the photo between the lines is left untouched. */
    background:
      linear-gradient(
        125deg,
        #ff5c8a,
        #ffb347,
        #fff275,
        #7dffb0,
        #59d8ff,
        #8c7bff,
        #ff6fd8,
        #ff5c8a
      );
    background-size: 300% 300%;
    background-position: var(--mx) var(--my);
    /* Only the lines within 30px of the pointer light up: the line art is
       intersected with a soft-edged 30px circle that follows the pointer. */
    -webkit-mask-image: radial-gradient(circle 30px at var(--mx) var(--my), #000 0, #000 60%, transparent 100%), var(--lines);
    mask-image: radial-gradient(circle 30px at var(--mx) var(--my), #000 0, #000 60%, transparent 100%), var(--lines);
    -webkit-mask-size: 100% 100%, 100% 100%;
    mask-size: 100% 100%, 100% 100%;
    -webkit-mask-repeat: no-repeat;
    mask-repeat: no-repeat;
    -webkit-mask-composite: source-in;
    mask-composite: intersect;
    mix-blend-mode: screen;
  }
  .photo:hover .holo {
    opacity: 0.9;
  }
  .photo img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: grayscale(0.25) contrast(1.05);
  }
  .photo-empty {
    width: 100%;
    height: 100%;
    border: none;
    background: #ddd6e6;
    color: #6f6880;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    gap: 2px;
    padding: 0;
    font-family: var(--sans);
    font-size: 11px;
  }
  .photo-empty svg {
    width: 84px;
    fill: #b9b0c6;
  }
  .photo-empty span {
    padding-bottom: 6px;
  }
  dl {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .row3,
  .row2 {
    display: grid;
    gap: 8px;
  }
  .row3 {
    grid-template-columns: 0.5fr 0.7fr 1.4fr;
  }
  .row2 {
    grid-template-columns: 1fr 1fr;
  }
  dt {
    font-size: 9.5px;
    font-style: italic;
    color: #7a7189;
  }
  dd {
    margin: 0;
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 11.5px;
    color: #232030;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sig {
    margin-top: 14px;
  }
  .hand {
    font-family: "Caveat", cursive;
    font-size: 30px;
    line-height: 1;
    color: #213a7a;
    border-bottom: 1px solid #b9b0c6;
    min-height: 34px;
  }
  .fill-in {
    align-self: flex-start;
    margin-top: 10px;
    border: 1px solid #5d4a7a;
    background: transparent;
    color: #5d4a7a;
    border-radius: 6px;
    padding: 5px 10px;
    font-family: var(--sans);
    font-weight: 600;
  }
  .mrz {
    margin-top: auto;
    padding-top: 10px;
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 10.6px;
    letter-spacing: 0;
    color: #1d1b26;
    white-space: pre;
    overflow: hidden;
  }

  .visa,
  .index {
    position: relative;
    height: 100%;
    padding: 18px 24px;
  }
  h3 {
    margin: 0;
    text-align: center;
    font-weight: 600;
    font-size: 12px;
    letter-spacing: 0.3em;
    text-transform: uppercase;
    color: #8a8296;
  }
  .empty {
    margin: 120px auto 0;
    max-width: 24ch;
    text-align: center;
    font-style: italic;
    color: #7a7189;
  }
  .slot {
    position: absolute;
    transform: translate(-50%, -50%);
  }
  .index ol {
    list-style: none;
    margin: 18px 0 0;
    padding: 0;
  }
  .index li {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 5px 0;
    font-size: 14px;
  }
  .dots {
    flex: 1;
    border-bottom: 1px dotted #b9b0c6;
    transform: translateY(-3px);
  }
  .since {
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 11px;
    color: #6f6880;
  }
</style>
