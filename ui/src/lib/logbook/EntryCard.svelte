<script lang="ts">
  // A logbook line opened up: a mini map of the route and the flight's
  // details, with Edit entry and Delete entry.
  import { onMount } from "svelte";
  import { api, type FlightDetail, type LogPatch, type LogRow } from "../api";
  import { gradeTone, hhmm, int } from "../format";
  import AltSpeedGraph from "../AltSpeedGraph.svelte";
  import MiniMap from "../MiniMap.svelte";
  import { fromTrack, type Pt } from "../track";
  import { distance } from "../prefs.svelte";

  interface Props {
    row: LogRow;
    onclose: () => void;
    /** Something changed (edited or deleted): reload the logbook. */
    onchanged: () => void;
    /** Open this flight's full details on the Flights page. */
    onfull?: (flightId: number) => void;
  }
  let { row, onclose, onchanged, onfull }: Props = $props();

  let detail = $state<FlightDetail | null>(null);
  let mode = $state<"view" | "edit" | "delete">("view");
  let error = $state<string | null>(null);
  let busy = $state(false);

  onMount(async () => {
    detail = await api.flight(row.flight_id);
  });

  const track = $derived.by<Pt[]>(() => (detail ? fromTrack(detail.track) : []));

  const months = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
  const longDate = (ms: number) => {
    const d = new Date(ms);
    return `${d.getUTCDate()} ${months[d.getUTCMonth()]} ${d.getUTCFullYear()}`;
  };
  const zulu = (ms: number | null) => {
    if (ms == null) return "–";
    const d = new Date(ms);
    return `${String(d.getUTCHours()).padStart(2, "0")}:${String(d.getUTCMinutes()).padStart(2, "0")}Z`;
  };
  const isoDay = (ms: number) => new Date(ms).toISOString().slice(0, 10);
  const hm = (ms: number | null) => (ms == null ? "" : new Date(ms).toISOString().slice(11, 16));
  const asTime = (m: number) => `${Math.floor(m / 60)}:${String(m % 60).padStart(2, "0")}`;

  const landing = $derived(detail?.landings.filter((l) => !l.touch_and_go).at(-1) ?? null);
  const grade = $derived(detail?.flight.landing_grade ?? null);

  // --- edit ----------------------------------------------------------------
  type Field = { key: keyof LogPatch; label: string; kind: "time" | "count" };
  const TIMES: Field[] = [
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
  /** Category and class for this line (the aircraft's own unless changed). */
  const CLASSES = [
    { id: "SEL", label: "SEL", title: "Single engine land" },
    { id: "MEL", label: "MEL", title: "Multi engine land" },
    { id: "SES", label: "SES", title: "Single engine sea" },
    { id: "MES", label: "MES", title: "Multi engine sea" },
  ];
  let cls = $state("");

  function startEdit() {
    error = null;
    const d: Record<string, string> = {
      date: isoDay(row.date_ms),
      out: hm(row.out_ms),
      in: hm(row.in_ms),
      from: row.from_icao,
      to: row.to_icao,
      type: row.aircraft_type,
      ident: row.ident,
      remarks: row.remarks,
    };
    for (const f of TIMES) {
      const v = Number(row[f.key as keyof LogRow]) || 0;
      d[f.key] = f.kind === "time" ? asTime(v) : String(v);
    }
    draft = d;
    cls = row.category_class;
    mode = "edit";
  }

  /** "1:24", "1.4" (hours) or "84m" -> minutes. */
  function parseDuration(s: string): number | null {
    const t = s.trim();
    if (!t) return 0;
    let m = t.match(/^(\d+):(\d{1,2})$/);
    if (m && +m[2] < 60) return +m[1] * 60 + +m[2];
    m = t.match(/^(\d+)m$/);
    if (m) return +m[1];
    if (/^\d+(\.\d+)?$/.test(t)) return Math.round(parseFloat(t) * 60);
    return null;
  }

  /** "14:05" or "1405" -> minutes after midnight. */
  function parseClock(s: string): number | null | undefined {
    const t = s.trim().replace(/z$/i, "");
    if (!t) return undefined;
    const m = t.match(/^(\d{1,2}):?(\d{2})$/);
    if (!m || +m[1] > 23 || +m[2] > 59) return null;
    return +m[1] * 60 + +m[2];
  }

  async function save() {
    error = null;
    const log: LogPatch = { remarks: draft.remarks.trim() };
    if (cls !== row.category_class && CLASSES.some((c) => c.id === cls)) log.class_override = cls;
    for (const f of TIMES) {
      const raw = draft[f.key] ?? "";
      const v = f.kind === "time" ? parseDuration(raw) : /^\d+$/.test(raw.trim()) ? +raw : raw.trim() === "" ? 0 : null;
      if (v == null) {
        error = f.kind === "time" ? `${f.label}: use hours and minutes like 1:25, or decimal hours like 1.4.` : `${f.label}: use a whole number.`;
        return;
      }
      (log as Record<string, unknown>)[f.key] = v;
    }
    const out = parseClock(draft.out);
    const inn = parseClock(draft.in);
    if (out === null || inn === null) {
      error = "Out and In times: use 24-hour UTC time like 14:05.";
      return;
    }
    const edit: Record<string, unknown> = {};
    if (draft.date && draft.date !== isoDay(row.date_ms)) edit.date_ms = Date.parse(`${draft.date}T12:00:00Z`);
    if (draft.out !== hm(row.out_ms) && out !== undefined) edit.out_min = out;
    if (draft.in !== hm(row.in_ms) && inn !== undefined) edit.in_min = inn;
    if (draft.from.trim().toUpperCase() !== row.from_icao) edit.from_icao = draft.from.trim();
    if (draft.to.trim().toUpperCase() !== row.to_icao) edit.to_icao = draft.to.trim();
    if (draft.type.trim().toUpperCase() !== row.aircraft_type.toUpperCase()) edit.aircraft_type = draft.type.trim();
    if (draft.ident.trim().toUpperCase() !== row.ident.toUpperCase()) edit.ident = draft.ident.trim();
    busy = true;
    try {
      if (Object.keys(edit).length) await api.updateFlight(row.flight_id, edit);
      await api.updateLogEntry(row.flight_id, log);
      onchanged();
      onclose();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function remove() {
    busy = true;
    try {
      await api.deleteFlight(row.flight_id);
      onchanged();
      onclose();
    } catch (e) {
      error = String(e);
      busy = false;
    }
  }

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") mode === "view" ? onclose() : (mode = "view");
  }
</script>

<svelte:window onkeydown={key} />

<div class="scrim" role="presentation" onclick={onclose}></div>
<div class="card" role="dialog" aria-modal="true" aria-labelledby="entry-title">
  <button class="close" onclick={onclose} aria-label="Close">
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
  </button>

  {#if mode === "edit"}
    <h3 id="entry-title">Edit entry</h3>
    <p class="sub">Times of day are UTC, as in the logbook.</p>
    <div class="fields flight">
      <label><span>Date</span><input type="date" bind:value={draft.date} /></label>
      <label><span>Out (UTC)</span><input bind:value={draft.out} placeholder="14:05" maxlength="6" /></label>
      <label><span>In (UTC)</span><input bind:value={draft.in} placeholder="15:20" maxlength="6" /></label>
      <label><span>From</span><input bind:value={draft.from} placeholder="ICAO" maxlength="8" class="caps" /></label>
      <label><span>To</span><input bind:value={draft.to} placeholder="ICAO" maxlength="8" class="caps" /></label>
      <label><span>Aircraft type</span><input bind:value={draft.type} placeholder="C172" maxlength="12" class="caps" /></label>
      <label><span>Registration</span><input bind:value={draft.ident} placeholder="N12345" maxlength="12" class="caps" /></label>
      <div class="cls">
        <span>Class</span>
        <div class="seg" role="radiogroup" aria-label="Category and class">
          {#each CLASSES as c (c.id)}
            <button type="button" role="radio" aria-checked={cls === c.id} class:on={cls === c.id} title={c.title} onclick={() => (cls = c.id)}>{c.label}</button>
          {/each}
        </div>
      </div>
    </div>
    <div class="fields">
      {#each TIMES as f (f.key)}
        <label>
          <span>{f.label}</span>
          <input bind:value={draft[f.key]} inputmode={f.kind === "count" ? "numeric" : "text"} placeholder={f.kind === "time" ? "0:00" : "0"} />
        </label>
      {/each}
      <label class="wide"><span>Remarks</span><input bind:value={draft.remarks} maxlength="120" /></label>
    </div>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="actions">
      <button class="primary" onclick={save} disabled={busy}>{busy ? "Saving…" : "Save changes"}</button>
      <button class="ghost" onclick={() => (mode = "view")} disabled={busy}>Cancel</button>
    </div>
  {:else}
    <div class="head">
      <h3 id="entry-title" class="num">{row.from_icao || "????"} <span class="to">to</span> {row.to_icao || "????"}</h3>
      <p class="sub">
        {longDate(row.date_ms)}{row.aircraft_type ? `, ${row.aircraft_type}` : ""}{row.ident ? ` ${row.ident}` : ""}
        {#if row.status !== "complete"}<span class="tag">{row.status}</span>{/if}
      </p>
      {#if detail}
        <p class="names">{detail.flight.dep_name ?? "Unknown departure"}<br />{detail.flight.arr_name ?? "Unknown arrival"}</p>
      {/if}
    </div>

    <div class="map">
      {#if detail}
        {#if track.length}
          <MiniMap {track} height={170} interactive />
        {:else}
          <div class="nomap">No track was recorded for this flight.</div>
        {/if}
      {:else}
        <div class="nomap">Loading the route…</div>
      {/if}
    </div>

    <dl class="facts">
      <div><dt>Out</dt><dd class="num">{zulu(row.out_ms)}</dd></div>
      <div><dt>In</dt><dd class="num">{zulu(row.in_ms)}</dd></div>
      <div><dt>Total</dt><dd class="num">{hhmm(row.total_min)}</dd></div>
      <div><dt>Night</dt><dd class="num">{row.night_min ? hhmm(row.night_min) : "–"}</dd></div>
      <div><dt>Distance</dt><dd class="num">{detail ? `${distance(detail.flight.distance_nm).value} ${distance(detail.flight.distance_nm).unit}` : "–"}</dd></div>
      <div><dt>Max altitude</dt><dd class="num">{detail ? `${int(detail.flight.max_alt_ft)} ft` : "–"}</dd></div>
      <div><dt>Landings</dt><dd class="num">{row.day_landings + row.night_landings}</dd></div>
      <div>
        <dt>Landing rate</dt>
        <dd class="num {gradeTone(grade)}">{landing ? `${int(landing.vs_fpm)} fpm` : "–"}</dd>
      </div>
    </dl>
    {#if track.length > 1}
      <div class="graph"><AltSpeedGraph pts={track} height={128} /></div>
    {/if}
    {#if row.remarks}<p class="remarks">{row.remarks}</p>{/if}

    {#if mode === "delete"}
      <div class="confirm" role="alert">
        <p>
          Delete this flight? It is removed from the logbook, the map, your passport and the hangar. This cannot be undone.
        </p>
        <div class="actions">
          <button class="danger" onclick={remove} disabled={busy}>{busy ? "Deleting…" : "Delete flight"}</button>
          <button class="ghost" onclick={() => (mode = "view")} disabled={busy}>Keep it</button>
        </div>
      </div>
    {:else}
      <div class="actions">
        <button class="primary" onclick={startEdit}>Edit entry</button>
        <button class="ghost danger-text" onclick={() => (mode = "delete")}>Delete entry</button>
        {#if onfull}
          <button class="ghost full" onclick={() => onfull(row.flight_id)}>
            Full flight info
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5l7 7-7 7" /></svg>
          </button>
        {/if}
      </div>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  {/if}
</div>

<style>
  .cls {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .cls > span {
    font-size: 12px;
    color: var(--ink-dim);
  }
  .seg {
    display: inline-flex;
    border: 1px solid var(--rule);
    border-radius: 6px;
    overflow: hidden;
    align-self: flex-start;
  }
  .seg button {
    border: none;
    background: transparent;
    color: var(--ink-dim);
    padding: 6px 10px;
    font-weight: 600;
    font-size: 13px;
  }
  .seg button + button {
    border-left: 1px solid var(--rule);
  }
  .seg button.on {
    background: var(--panel-3);
    color: var(--amber);
  }
  .scrim {
    position: fixed;
    inset: 0;
    background: rgba(5, 10, 14, 0.6);
    z-index: 20;
  }
  .card {
    position: fixed;
    z-index: 21;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(620px, 94vw);
    max-height: 92vh;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--panel);
    border: 1px solid var(--rule);
    border-radius: 12px;
    padding: 20px 22px;
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.55);
  }
  .close {
    position: absolute;
    top: 12px;
    right: 12px;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--ink-dim);
    display: grid;
    place-items: center;
  }
  .close:hover {
    background: var(--panel-2);
    color: var(--ink);
  }
  .close svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
  h3 {
    margin: 0;
    font-family: var(--cond);
    font-weight: 600;
    font-size: 30px;
    line-height: 1.1;
  }
  .to {
    font-family: var(--sans);
    font-size: 16px;
    font-weight: 400;
    color: var(--ink-dim);
  }
  .sub,
  .names {
    margin: 4px 0 0;
    color: var(--ink-dim);
  }
  .names {
    font-size: 13px;
  }
  .tag {
    font-size: 12px;
    color: var(--amber);
    border: 1px solid var(--amber);
    border-radius: 99px;
    padding: 0 7px;
    margin-left: 6px;
  }
  .map {
    margin-top: 14px;
  }
  .nomap {
    height: 220px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: var(--panel-2);
    color: var(--ink-dim);
    border: 1px solid var(--rule);
  }
  .facts {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px 8px;
    margin: 14px 0 0;
    padding: 12px 0;
    border-top: 1px solid var(--rule);
    border-bottom: 1px solid var(--rule);
  }
  dt {
    font-size: 12px;
    color: var(--ink-dim);
  }
  dd {
    margin: 0;
    font-size: 18px;
  }
  dd.good {
    color: var(--good);
  }
  dd.warn {
    color: var(--warn);
  }
  dd.bad {
    color: var(--bad);
  }
  .remarks {
    margin: 10px 0 0;
    color: var(--ink-dim);
    font-style: italic;
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 16px;
    flex-wrap: wrap;
  }
  .confirm {
    margin-top: 16px;
    padding: 12px 14px;
    border: 1px solid var(--bad);
    border-radius: 8px;
    background: rgba(226, 106, 90, 0.08);
  }
  .confirm p {
    margin: 0;
  }
  .confirm .actions {
    margin-top: 10px;
  }
  button.primary,
  button.ghost,
  button.danger {
    border-radius: 6px;
    padding: 8px 14px;
    font-weight: 600;
  }
  .primary {
    border: none;
    background: var(--amber);
    color: #1b1406;
  }
  .ghost {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
  }
  .ghost:hover {
    color: var(--ink);
  }
  .danger-text:hover {
    color: var(--bad);
    border-color: var(--bad);
  }
  .danger {
    border: none;
    background: var(--bad);
    color: #fff;
  }
  button:disabled {
    opacity: 0.6;
  }
  .fields {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px 12px;
    margin-top: 14px;
  }
  .fields.flight {
    grid-template-columns: repeat(4, 1fr);
    padding-bottom: 14px;
    border-bottom: 1px solid var(--rule);
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
    color-scheme: dark;
  }
  input.caps {
    text-transform: uppercase;
  }
  .error {
    color: var(--bad);
    margin: 10px 0 0;
  }
  .full {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .full svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform 0.15s;
  }
  .full:hover svg {
    transform: translateX(2px);
  }
</style>
