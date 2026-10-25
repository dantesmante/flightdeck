<script lang="ts">
  import { onMount } from "svelte";
  import { api, type LogPatch, type LogRow } from "../api";
  import Book from "../Book.svelte";
  import { prefs, setPref } from "../prefs.svelte";
  import { groups, LAYOUTS, printValue, type Col, type Layout } from "./columns";

  const W = 560;
  const H = 470;
  const ROWS = 12;

  let rows = $state<LogRow[]>([]);
  let loaded = $state(false);
  let spread = $state(0);
  let book = $state<Book | null>(null);
  let hover = $state<number | null>(null);
  let editing = $state<LogRow | null>(null);

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

  // --- editing -----------------------------------------------------------
  type Field = { key: keyof LogPatch; label: string; kind: "time" | "count" | "text" };
  const FIELDS: Field[] = [
    { key: "total_min", label: "Total time", kind: "time" },
    { key: "pic_min", label: "Pilot in command", kind: "time" },
    { key: "dual_min", label: "Dual received", kind: "time" },
    { key: "night_min", label: "Night", kind: "time" },
    { key: "xc_min", label: "Cross-country", kind: "time" },
    { key: "ifr_min", label: "IFR", kind: "time" },
    { key: "act_inst_min", label: "Actual instrument", kind: "time" },
    { key: "sim_inst_min", label: "Simulated instrument", kind: "time" },
    { key: "day_landings", label: "Day landings", kind: "count" },
    { key: "night_landings", label: "Night landings", kind: "count" },
    { key: "approaches", label: "Instrument approaches", kind: "count" },
  ];
  let draft = $state<Record<string, string>>({});
  let error = $state<string | null>(null);

  const asTime = (m: number) => `${Math.floor(m / 60)}:${String(m % 60).padStart(2, "0")}`;

  function edit(r: LogRow) {
    editing = r;
    error = null;
    const d: Record<string, string> = { remarks: r.remarks };
    for (const f of FIELDS) {
      const v = Number(r[f.key as keyof LogRow]) || 0;
      d[f.key] = f.kind === "time" ? asTime(v) : String(v);
    }
    draft = d;
  }

  /** Accepts "1:24", "1.4" (hours) or "84m". */
  function parseTime(s: string): number | null {
    const t = s.trim();
    if (!t) return 0;
    let m = t.match(/^(\d+):(\d{1,2})$/);
    if (m) return +m[1] * 60 + +m[2];
    m = t.match(/^(\d+)m$/);
    if (m) return +m[1];
    if (/^\d+(\.\d+)?$/.test(t)) return Math.round(parseFloat(t) * 60);
    return null;
  }

  async function save() {
    if (!editing) return;
    const patch: LogPatch = { remarks: draft.remarks };
    for (const f of FIELDS) {
      const raw = draft[f.key] ?? "";
      const v = f.kind === "time" ? parseTime(raw) : /^\d+$/.test(raw.trim()) ? +raw : raw.trim() === "" ? 0 : null;
      if (v == null) {
        error = `${f.label}: use hours and minutes like 1:25, or decimal hours like 1.4.`;
        return;
      }
      (patch as Record<string, unknown>)[f.key] = v;
    }
    await api.updateLogEntry(editing.flight_id, patch);
    editing = null;
    await load();
  }

  function key(e: KeyboardEvent) {
    if (editing && e.key === "Escape") editing = null;
  }

  const year = (s: number) => {
    const r = rows[s * ROWS];
    return r ? new Date(r.date_ms).getUTCFullYear() : new Date().getUTCFullYear();
  };
</script>

<svelte:window onkeydown={key} />

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
          aria-label={r ? `Edit entry ${r.from_icao} to ${r.to_icao}` : undefined}
          onmouseenter={() => r && (hover = r.flight_id)}
          onmouseleave={() => (hover = null)}
          onclick={() => r && edit(r)}
          onkeydown={(e) => r && (e.key === "Enter" || e.key === " ") && (e.preventDefault(), edit(r))}
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
    <span class="nav">
      <button class="ghost" onclick={() => book?.prev()} disabled={spread === 0}>Previous</button>
      <button class="ghost" onclick={() => book?.next()} disabled={spread >= spreads - 1}>Next</button>
    </span>
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
    <p class="tip">Click any line to correct it. Approaches, IFR and instrument time are yours to fill in.</p>
  {/if}
</section>

{#if editing}
  {@const e = editing}
  <div class="scrim" role="presentation" onclick={() => (editing = null)}></div>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="edit-title">
    <h3 id="edit-title">{e.from_icao || "????"} to {e.to_icao || "????"}, {new Date(e.date_ms).toUTCString().slice(5, 16)}</h3>
    <p class="sub">{e.aircraft_type} {e.ident}</p>
    <div class="fields">
      {#each FIELDS as f (f.key)}
        <label>
          <span>{f.label}</span>
          <input bind:value={draft[f.key]} inputmode={f.kind === "count" ? "numeric" : "text"} placeholder={f.kind === "time" ? "0:00" : "0"} />
        </label>
      {/each}
      <label class="wide">
        <span>Remarks</span>
        <input bind:value={draft.remarks} maxlength="120" />
      </label>
    </div>
    {#if error}<p class="error">{error}</p>{/if}
    <div class="actions">
      <button class="primary" onclick={save}>Save entry</button>
      <button class="ghost" onclick={() => (editing = null)}>Cancel</button>
    </div>
  </div>
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
  .nav {
    margin-left: auto;
    display: flex;
    gap: 8px;
  }
  .ghost {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    border-radius: 6px;
    padding: 6px 12px;
    font-weight: 600;
  }
  .ghost:disabled {
    opacity: 0.4;
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

  /* Edit dialog -------------------------------------------------------- */
  .scrim {
    position: fixed;
    inset: 0;
    background: rgba(5, 10, 14, 0.55);
    z-index: 20;
  }
  .dialog {
    position: fixed;
    z-index: 21;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(560px, 92vw);
    background: var(--panel);
    border: 1px solid var(--rule);
    border-radius: 10px;
    padding: 20px 22px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }
  .dialog h3 {
    margin: 0;
    font-family: var(--cond);
    font-size: 24px;
    font-weight: 600;
  }
  .sub {
    margin: 2px 0 14px;
    color: var(--ink-dim);
  }
  .fields {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px 12px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--ink-dim);
  }
  label.wide {
    grid-column: 1 / -1;
  }
  input {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
    font: inherit;
    font-size: 14px;
    color: var(--ink);
    background: var(--panel-2);
    border: 1px solid var(--rule);
    border-radius: 6px;
    padding: 6px 8px;
  }
  .error {
    color: var(--bad);
    margin: 10px 0 0;
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 16px;
  }
  .primary {
    border: none;
    background: var(--amber);
    color: #1b1406;
    border-radius: 6px;
    padding: 8px 14px;
    font-weight: 600;
  }
</style>
