<script lang="ts">
  import { onMount } from "svelte";
  import { api, type LogRow } from "../api";
  import Book from "../Book.svelte";
  import { prefs, setPref } from "../prefs.svelte";
  import { groups, LAYOUTS, LIST_COLS, printValue, type Col, type Layout } from "./columns";
  import EntryCard from "./EntryCard.svelte";
  import ManualEntry from "./ManualEntry.svelte";
  import Duplicates from "./Duplicates.svelte";

  let { onflight }: { onflight?: (flightId: number) => void } = $props();

  const W = 560;
  const H = 542;
  const ROWS = 15;

  let rows = $state<LogRow[]>([]);
  let loaded = $state(false);
  let spread = $state(0);
  let book = $state<Book | null>(null);
  let hover = $state<number | null>(null);
  let opened = $state<LogRow | null>(null);
  let adding = $state(false);
  let dupes = $state(false);

  /** A new manual entry: show the page it landed on. */
  async function added(id: number) {
    adding = false;
    await load();
    const i = rows.findIndex((r) => r.flight_id === id);
    if (i >= 0) book?.goTo(Math.floor(i / ROWS));
  }

  const layout = $derived((prefs.s.logbook_layout as Layout) ?? "faa");
  const hand = $derived(prefs.s.logbook_hand !== "type");
  const dark = $derived(prefs.s.logbook_theme === "dark");
  const list = $derived(prefs.s.logbook_view === "list");
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

  // The simple list: the main columns in one table, newest flight first,
  // with the full date. FAA/EASA still sets how times are written.
  const tableCols = LIST_COLS;
  const newestFirst = $derived([...rows].reverse());
  const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  const fullDate = (ms: number) => {
    const d = new Date(ms);
    return `${d.getUTCDate()} ${MONTHS[d.getUTCMonth()]} ${d.getUTCFullYear()}`;
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
  <div class="page" class:lp={i % 2 === 0} class:hand class:dark>
    <div class="top">
      {#if i % 2 === 0}
        <span>Pilot logbook</span><span></span>
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
      {rows.length} {rows.length === 1 ? "entry" : "entries"}
    </span>
    <div class="seg" role="group" aria-label="Logbook view">
      <button class:on={!list} onclick={() => setPref("logbook_view", "book")} title="A paper pilot logbook">Logbook</button>
      <button class:on={list} onclick={() => setPref("logbook_view", "list")} title="A plain list of every flight">List</button>
    </div>
    <div class="seg" role="group" aria-label="Logbook layout">
      <button class:on={layout === "faa"} onclick={() => setPref("logbook_layout", "faa")}>FAA</button>
      <button class:on={layout === "easa"} onclick={() => setPref("logbook_layout", "easa")}>EASA</button>
    </div>
    {#if !list}
      <div class="seg" role="group" aria-label="Writing">
        <button class:on={hand} onclick={() => setPref("logbook_hand", "hand")}>Handwritten</button>
        <button class:on={!hand} onclick={() => setPref("logbook_hand", "type")}>Typed</button>
      </div>
      <div class="seg" role="group" aria-label="Paper">
        <button class:on={!dark} onclick={() => setPref("logbook_theme", "light")} aria-label="Light paper" title="Light paper">
          <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="4.2" /><path d="M12 2.5v2.5M12 19v2.5M2.5 12H5M19 12h2.5M5.3 5.3l1.8 1.8M16.9 16.9l1.8 1.8M5.3 18.7l1.8-1.8M16.9 7.1l1.8-1.8" /></svg>
        </button>
        <button class:on={dark} onclick={() => setPref("logbook_theme", "dark")} aria-label="Dark paper" title="Dark paper">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M19.5 14.6A8 8 0 0 1 9.4 4.5a8 8 0 1 0 10.1 10.1z" /></svg>
        </button>
      </div>
    {/if}
    <button class="add dup" onclick={() => (dupes = true)} title="Find flights logged more than once">
      <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="8" y="8" width="12" height="12" rx="2" /><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" /></svg>
      Find duplicates
    </button>
    <button class="add" onclick={() => (adding = true)}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
      Add manual entry
    </button>
  </div>
  {#if loaded && rows.length === 0}
    <p class="note">Your logbook is empty. Each flight writes its own line when you park. Settings has sample flights if you want to see it filled in.</p>
  {/if}
  {#if list}
    <div class="table-wrap">
      {#if rows.length}
        <table class="flat">
          <thead>
            <tr>
              <th>Date</th>
              {#each tableCols as c (c.key)}
                <th class:num={c.kind !== "text"}>{c.label}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each newestFirst as r (r.flight_id)}
              <tr tabindex="0" onclick={() => (opened = r)} onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), (opened = r))}>
                <td class="nowrap">{fullDate(r.date_ms)}</td>
                {#each tableCols as c (c.key)}
                  <td class:num={c.kind !== "text"} class:remarks={c.flex} class:ac={c.key === "type"} title={c.flex || c.key === "type" ? String(c.value(r)) : undefined}>{printValue(c, c.value(r), layout)}</td>
                {/each}
              </tr>
            {/each}
          </tbody>
          <tfoot>
            <tr>
              <td>Totals</td>
              {#each tableCols as c (c.key)}
                <td class:num={c.kind !== "text"}>{c.kind === "text" ? "" : printValue(c, sum(c, rows), layout)}</td>
              {/each}
            </tr>
          </tfoot>
        </table>
      {/if}
    </div>
  {:else}
    <div class="stage">
      {#if loaded}
        <Book bind:this={book} bind:spread pageCount={spreads * 2} pageWidth={W} pageHeight={H} page={pageView} label="Pilot logbook" />
      {/if}
    </div>
  {/if}
  {#if rows.length}
    <p class="tip">Click any {list ? "flight" : "line"} to see the flight, edit it or delete it.</p>
  {/if}
</section>

{#if dupes}
  <Duplicates onclose={() => (dupes = false)} ondone={() => load()} />
{/if}

{#if adding}
  <ManualEntry onclose={() => (adding = false)} onsaved={added} />
{/if}

{#if opened}
  <EntryCard row={opened} onclose={() => (opened = null)} onchanged={() => load()} onfull={onflight} />
{/if}

<style>
  .logbook {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: linear-gradient(180deg, #1d1f22, #141517);
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
  .add {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--rule);
    background: #26282c;
    color: var(--ink);
    border-radius: 6px;
    padding: 5px 12px;
    font-size: 13px;
    font-weight: 600;
  }
  .add.dup + .add {
    margin-left: 0;
  }
  .add svg rect,
  .add svg path {
    fill: none;
  }
  .add:hover {
    border-color: var(--amber);
    color: var(--amber);
  }
  .add svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.2;
    stroke-linecap: round;
  }
  .seg svg {
    width: 15px;
    height: 15px;
    display: block;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  /* The simple list ------------------------------------------------------ */
  .table-wrap {
    flex: 1;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    margin: 14px 20px 10px;
    border: 1px solid var(--rule);
    border-radius: 10px;
    background: var(--panel);
  }
  .flat {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .flat th {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--panel-2);
    color: var(--ink-dim);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    text-align: left;
    vertical-align: bottom;
    padding: 8px 10px;
    border-bottom: 1px solid var(--rule);
    white-space: nowrap;
  }
  .flat td {
    padding: 8px 10px;
    border-bottom: 1px solid var(--rule);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .flat .num {
    text-align: right;
  }
  .flat td.remarks {
    width: 40%;
    max-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ink-dim);
  }
  .flat td.ac {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .flat tbody tr {
    cursor: pointer;
  }
  .flat tbody tr:hover,
  .flat tbody tr:focus-visible {
    background: rgba(240, 178, 58, 0.08);
    outline: none;
  }
  .flat tfoot td {
    position: sticky;
    bottom: 0;
    background: var(--panel-2);
    font-weight: 700;
    border-bottom: none;
    border-top: 1px solid var(--rule);
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
    --paper: #f7f4e8;
    --pen: #1f3a8a;
    --type: #222;
    --line: #d3dfd6;
    position: relative;
    height: 100%;
    background: var(--paper);
    color: var(--pen);
    padding: 10px 14px 0 18px;
    font-family: "Source Serif 4", Georgia, serif;
    display: flex;
    flex-direction: column;
  }
  /* Dark paper: charcoal pages, pale green print, light-blue ink. */
  .page.dark {
    --rule-ink: #4f5e57;
    --head: #2b302f;
    --print: #9db5a8;
    --paper: #212426;
    --pen: #9cc0ff;
    --type: #dcdfe2;
    --line: #353b3c;
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
    border-bottom: 1px solid var(--line);
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
    color: var(--type);
  }
  .cell:first-child {
    border-left: none;
  }
  .hand .cell {
    font-family: "Caveat", cursive;
    font-size: 17px;
    color: var(--pen);
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
