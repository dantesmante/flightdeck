<script lang="ts">
  import { onMount } from "svelte";
  import { api, type LogRow } from "../api";
  import Book from "../Book.svelte";
  import { prefs, setPref } from "../prefs.svelte";
  import { groups, LAYOUTS, printValue, type Col, type Layout } from "./columns";
  import EntryCard from "./EntryCard.svelte";

  let { onflight }: { onflight?: (flightId: number) => void } = $props();

  const W = 560;
  const H = 470;
  const ROWS = 12;

  let rows = $state<LogRow[]>([]);
  let loaded = $state(false);
  let spread = $state(0);
  let book = $state<Book | null>(null);
  let hover = $state<number | null>(null);
  let opened = $state<LogRow | null>(null);

  const layout = $derived((prefs.s.logbook_layout as Layout) ?? "faa");
  const hand = $derived(prefs.s.logbook_hand !== "type");
  const cols = $derived(LAYOUTS[layout]);
  const left = $derived(cols.filter((c) => c.side === "L"));
  const right = $derived(cols.filter((c) => c.side === "R"));
  const spreads = $derived(Math.max(1, Math.ceil(rows.length / ROWS)));

  async function load(toEnd = false) {
    rows = (await api.logbook()) ?? [];
    loaded = true;
    if (toEnd) spread = Math.max(0, Math.ceil(rows.length / ROWS) - 1);
  }

  onMount(() => load(true));

  const template = (cs: Col[]) => cs.map((c) => (c.flex ? "minmax(0, 1fr)" : `${c.width}px`)).join(" ");

  function pageRows(s: number) {
    const slice = rows.slice(s * ROWS, (s + 1) * ROWS);
    return [...slice, ...Array(ROWS - slice.length).fill(null)] as (LogRow | null)[];
  }

  function sum(col: Col, list: LogRow[]) {
    return list.reduce((a, r) => a + (Number(col.value(r)) || 0), 0);
  }

  function footer(col: Col, s: number) {
    const page = rows.slice(s * ROWS, (s + 1) * ROWS);
    const before = rows.slice(0, s * ROWS);
    return {
      page: printValue(col, sum(col, page), layout),
      forward: printValue(col, sum(col, before), layout),
      todate: printValue(col, sum(col, before) + sum(col, page), layout),
    };
  }

  const year = (s: number) => {
    const r = rows[s * ROWS];
    return r ? new Date(r.date_ms).getUTCFullYear() : new Date().getUTCFullYear();
  };
</script>


{#snippet header(cs: Col[])}
  <div class="hdr" style:grid-template-columns={template(cs)}>
    {#each groups(cs) as g, gi (gi)}
      {#if g.label}
        <div
          class="grp"
          style:grid-row="1"
          style:grid-column="{cs.findIndex((c) => c.group === g.label) + 1} / span {g.span}"
        >
          {g.label}
        </div>
      {/if}
    {/each}
    {#each cs as c, ci (c.key)}
      <div class="lbl" class:tall={!c.group} style:grid-row={c.group ? "2" : "1 / span 2"} style:grid-column="{ci + 1}">
        {c.label}
      </div>
    {/each}
  </div>
{/snippet}

{#snippet pageView(i: number)}
  {@const s = Math.floor(i / 2)}
  {@const side = i % 2 === 0 ? left : right}
  {@const list = pageRows(s)}
  {@const firstNum = side.findIndex((c) => c.kind !== "text")}
  {@const labelSpan = firstNum < 0 ? side.length : firstNum}
  <div class="page" class:lp={i % 2 === 0} class:hand>
    <div class="top">
      {#if i % 2 === 0}
        <span>Pilot logbook</span><span class="yr">{year(s)}</span>
      {:else}
        <span></span><span>{layout === "faa" ? "Times in hours and tenths" : "Times in hours and minutes"}</span>
      {/if}
    </div>
    {@render header(side)}
    <div class="body">
      {#each list as r, ri (ri)}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div
          class="row"
          class:hover={r && hover === r.flight_id}
          class:filled={!!r}
          style:grid-template-columns={template(side)}
          role={r ? "button" : undefined}
          tabindex={r ? 0 : undefined}
          aria-label={r ? `Open entry ${r.from_icao} to ${r.to_icao}` : undefined}
          onmouseenter={() => r && (hover = r.flight_id)}
          onmouseleave={() => (hover = null)}
          onclick={() => r && (opened = r)}
          onkeydown={(e) => r && (e.key === "Enter" || e.key === " ") && (e.preventDefault(), (opened = r))}
        >
          {#each side as c (c.key)}
            <div class="cell" class:num={c.kind !== "text"} class:flex={c.flex} title={r && c.flex ? String(c.value(r)) : undefined}>
              {r ? printValue(c, c.value(r), layout) : ""}
            </div>
          {/each}
        </div>
      {/each}
    </div>
    <div class="foot">
      {#each [["page", "Totals this page"], ["forward", "Amount forwarded"], ["todate", "Totals to date"]] as [k, label] (k)}
        <div class="row total" style:grid-template-columns={template(side)}>
          {#if labelSpan > 0}
            <div class="cell flabel" style:grid-column="1 / span {labelSpan}">{i % 2 === 0 ? label : ""}</div>
          {/if}
          {#each side.slice(labelSpan) as c (c.key)}
            <div class="cell num">{c.kind === "text" ? "" : footer(c, s)[k as "page" | "forward" | "todate"]}</div>
          {/each}
        </div>
      {/each}
      {#if i % 2 === 1}
        <p class="certify">I certify that the entries in this log are true.</p>
      {/if}
    </div>
    <span class="folio">{i + 1}</span>
  </div>
{/snippet}

<section class="logbook">
  <div class="toolbar">
    <h2>Logbook</h2>
    <span class="where">
      {rows.length} {rows.length === 1 ? "entry" : "entries"}{rows.length ? `, pages ${spread * 2 + 1} and ${spread * 2 + 2}` : ""}
    </span>
    <div class="seg" role="group" aria-label="Logbook layout">
      <button class:on={layout === "faa"} onclick={() => setPref("logbook_layout", "faa")}>FAA</button>
      <button class:on={layout === "easa"} onclick={() => setPref("logbook_layout", "easa")}>EASA</button>
    </div>
    <div class="seg" role="group" aria-label="Writing">
      <button class:on={hand} onclick={() => setPref("logbook_hand", "hand")}>Handwritten</button>
      <button class:on={!hand} onclick={() => setPref("logbook_hand", "type")}>Typed</button>
    </div>
  </div>
  {#if loaded && rows.length === 0}
    <p class="note">Your logbook is empty. Each flight writes its own line when you park. Settings has sample flights if you want to see it filled in.</p>
  {/if}
  <div class="stage">
    {#if loaded}
      <Book bind:this={book} bind:spread pageCount={spreads * 2} pageWidth={W} pageHeight={H} page={pageView} label="Pilot logbook" />
    {/if}
  </div>
  {#if rows.length}
    <p class="tip">Click any line to see the flight, edit it or delete it.</p>
  {/if}
</section>

{#if opened}
  <EntryCard row={opened} onclose={() => (opened = null)} onchanged={() => load()} onfull={onflight} />
{/if}

<style>
  .logbook {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: linear-gradient(180deg, #14212b, #101a22);
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 20px 0;
    flex-wrap: wrap;
  }
  h2 {
    margin: 0;
    font-family: var(--cond);
    font-weight: 600;
    font-size: 26px;
  }
  .where,
  .tip,
  .note {
    color: var(--ink-dim);
    font-size: 13px;
  }
  .note {
    margin: 8px 20px 0;
  }
  .tip {
    text-align: center;
    margin: 0 0 12px;
  }
  .seg {
    display: inline-flex;
    border: 1px solid var(--rule);
    border-radius: 6px;
    overflow: hidden;
  }
  .seg button {
    border: none;
    background: transparent;
    color: var(--ink-dim);
    padding: 5px 10px;
    font-size: 13px;
    font-weight: 600;
  }
  .seg button.on {
    background: var(--panel-3);
    color: var(--amber);
  }
  .stage {
    flex: 1;
    min-height: 0;
  }

  /* The paper ---------------------------------------------------------- */
  .page {
    --rule-ink: #9cb9a9;
    --head: #e1ebe2;
    --print: #3d5a4b;
    position: relative;
    height: 100%;
    background: #f7f4e8;
    color: #1f3a8a;
    padding: 10px 14px 0 18px;
    font-family: "Source Serif 4", Georgia, serif;
    display: flex;
    flex-direction: column;
  }
  .page.lp {
    padding: 10px 18px 0 14px;
  }
  .top {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: var(--print);
    letter-spacing: 0.08em;
    height: 16px;
  }
  .yr {
    font-family: "Caveat", cursive;
    font-size: 18px;
    color: #1f3a8a;
    letter-spacing: 0;
    line-height: 0.8;
  }
  .page:not(.hand) .yr {
    font-family: "Special Elite", "Courier New", monospace;
    font-size: 12px;
    color: #222;
  }
  .hdr {
    display: grid;
    grid-template-rows: 18px 40px;
    background: var(--head);
    border: 1px solid var(--rule-ink);
    border-bottom-width: 2px;
  }
  .grp,
  .lbl {
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 1px 3px;
    color: var(--print);
    font-size: 8.4px;
    line-height: 1.12;
    text-transform: uppercase;
    letter-spacing: 0;
    border-left: 1px solid var(--rule-ink);
    overflow: hidden;
  }
  .grp {
    border-bottom: 1px solid var(--rule-ink);
    font-weight: 600;
  }
  .lbl.tall {
    font-weight: 600;
  }
  .row {
    display: grid;
    height: 24px;
    border-left: 1px solid var(--rule-ink);
    border-right: 1px solid var(--rule-ink);
    border-bottom: 1px solid #d3dfd6;
  }
  .row.filled {
    cursor: pointer;
  }
  .row.hover,
  .row.filled:focus-visible {
    background: rgba(240, 178, 58, 0.16);
    outline: none;
  }
  .cell {
    border-left: 1px solid var(--rule-ink);
    padding: 0 4px;
    display: flex;
    align-items: center;
    overflow: hidden;
    white-space: nowrap;
    font-family: "Special Elite", "Courier New", monospace;
    font-size: 11.5px;
    color: #222;
  }
  .cell:first-child {
    border-left: none;
  }
  .hand .cell {
    font-family: "Caveat", cursive;
    font-size: 17px;
    color: #1f3a8a;
    padding-top: 2px;
  }
  .cell.num {
    justify-content: flex-end;
  }
  .cell.flex {
    text-overflow: ellipsis;
    display: block;
    line-height: 23px;
  }
  .foot {
    margin-top: 0;
  }
  .row.total {
    background: var(--head);
    border-bottom-color: var(--rule-ink);
    cursor: default;
  }
  .row.total:first-child {
    border-top: 2px solid var(--rule-ink);
  }
  .flabel {
    font-family: "Source Serif 4", Georgia, serif !important;
    font-size: 9px !important;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--print) !important;
    justify-content: flex-end;
    padding-right: 8px;
  }
  .row.total .cell.num {
    font-weight: 600;
  }
  .certify {
    margin: 6px 0 0;
    font-size: 9.5px;
    font-style: italic;
    color: var(--print);
  }
  .folio {
    position: absolute;
    bottom: 8px;
    left: 0;
    right: 0;
    text-align: center;
    font-size: 10px;
    color: var(--print);
  }

</style>
