<script lang="ts">
  // Add a logbook entry by hand, for a flight FlightDeck did not record.
  // It has no track: the map draws a line from departure to arrival.
  import { onMount } from "svelte";
  import { api, type HangarItem } from "../api";
  import { modelName } from "../hangar/names";

  let { onclose, onsaved }: { onclose: () => void; onsaved: (id: number) => void } = $props();

  const today = new Date().toISOString().slice(0, 10);
  let date = $state(today);
  let from = $state("");
  let to = $state("");
  let out = $state("");
  let inn = $state("");
  let total = $state("");
  let night = $state("");
  let ifr = $state("");
  let dayLdg = $state(1);
  let nightLdg = $state(0);
  let callsign = $state("");
  let remarks = $state("");
  let aircraft = $state<string>("");
  let otherType = $state("");
  let otherReg = $state("");
  let fleet = $state<HangarItem[]>([]);
  let busy = $state(false);
  let error = $state<string | null>(null);

  onMount(async () => {
    fleet = (await api.hangar()) ?? [];
    aircraft = fleet[0] ? String(fleet[0].id) : "other";
  });

  /** "1:25", "1h25", "85" (minutes) or "1.4" (hours) as minutes. */
  function minutes(s: string): number | null {
    const t = s.trim();
    if (!t) return null;
    let m = t.match(/^(\d+)\s*[:h]\s*(\d{1,2})\s*m?$/i);
    if (m) return Number(m[1]) * 60 + Number(m[2]);
    m = t.match(/^(\d+(?:\.\d+)?)$/);
    if (m) return t.includes(".") ? Math.round(Number(t) * 60) : Number(t) > 24 ? Number(t) : Number(t) * 60;
    return NaN;
  }
  /** "14:05" or "1405" as minutes after midnight. */
  function clock(s: string): number | null {
    const t = s.trim();
    if (!t) return null;
    const m = t.match(/^(\d{1,2}):?(\d{2})$/);
    if (!m || Number(m[1]) > 23 || Number(m[2]) > 59) return NaN;
    return Number(m[1]) * 60 + Number(m[2]);
  }

  const outMin = $derived(clock(out));
  const inMin = $derived(clock(inn));
  const blockMin = $derived(
    outMin != null && inMin != null && !isNaN(outMin) && !isNaN(inMin) ? (inMin - outMin + 1440) % 1440 : null,
  );
  const totalMin = $derived(minutes(total));
  const hm = (m: number) => `${Math.floor(m / 60)}:${String(m % 60).padStart(2, "0")}`;

  async function save() {
    error = null;
    if (outMin != null && isNaN(outMin)) return (error = "Write the out time as HH:MM (UTC).");
    if (inMin != null && isNaN(inMin)) return (error = "Write the in time as HH:MM (UTC).");
    for (const [label, v] of [["total", totalMin], ["night", minutes(night)], ["instrument", minutes(ifr)]] as const) {
      if (v != null && isNaN(v)) return (error = `Write the ${label} time as h:mm, e.g. 1:25.`);
    }
    busy = true;
    try {
      const id = await api.addManualEntry({
        date_ms: Date.parse(`${date}T12:00:00Z`),
        out_min: outMin,
        in_min: inMin,
        from_icao: from,
        to_icao: to,
        aircraft_id: aircraft && aircraft !== "other" ? Number(aircraft) : null,
        aircraft_type: aircraft === "other" ? otherType : "",
        ident: aircraft === "other" ? otherReg : "",
        callsign,
        total_min: totalMin ?? blockMin ?? 0,
        night_min: minutes(night) ?? 0,
        ifr_min: minutes(ifr) ?? 0,
        day_landings: Number(dayLdg) || 0,
        night_landings: Number(nightLdg) || 0,
        remarks,
      });
      onsaved(id);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={key} />

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <form
    class="sheet"
    aria-label="Add a logbook entry"
    onsubmit={(e) => {
      e.preventDefault();
      save();
    }}
  >
    <header>
      <div>
        <h3>Add manual entry</h3>
        <p>For a flight FlightDeck did not record. It shows on the map as a line from departure to arrival.</p>
      </div>
      <button type="button" class="x" aria-label="Close" onclick={onclose}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg></button>
    </header>

    <div class="row">
      <label><span>Date (UTC)</span><input type="date" bind:value={date} max={today} required /></label>
      <label><span>From</span><input class="code" bind:value={from} placeholder="KSEA" maxlength="7" autocomplete="off" spellcheck="false" required /></label>
      <label><span>To</span><input class="code" bind:value={to} placeholder="KPDX" maxlength="7" autocomplete="off" spellcheck="false" required /></label>
    </div>

    <div class="row">
      <label><span>Out (UTC)</span><input bind:value={out} placeholder="14:05" inputmode="numeric" autocomplete="off" /></label>
      <label><span>In (UTC)</span><input bind:value={inn} placeholder="15:30" inputmode="numeric" autocomplete="off" /></label>
      <label>
        <span>Total time</span>
        <input bind:value={total} placeholder={blockMin != null ? hm(blockMin) : "1:25"} autocomplete="off" />
      </label>
    </div>

    <label>
      <span>Aircraft</span>
      <select bind:value={aircraft}>
        {#each fleet as a (a.id)}
          <option value={String(a.id)}>{a.registration || a.icao_type || "Unregistered"} · {modelName(a)}</option>
        {/each}
        <option value="other">Another aircraft…</option>
      </select>
    </label>
    {#if aircraft === "other"}
      <div class="row">
        <label><span>Type code</span><input class="code" bind:value={otherType} placeholder="C172" maxlength="4" autocomplete="off" spellcheck="false" /></label>
        <label><span>Registration</span><input class="code" bind:value={otherReg} placeholder="N172SP" autocomplete="off" spellcheck="false" /></label>
      </div>
    {/if}

    <div class="row">
      <label><span>Landings, day</span><input type="number" min="0" max="99" bind:value={dayLdg} /></label>
      <label><span>Landings, night</span><input type="number" min="0" max="99" bind:value={nightLdg} /></label>
      <label><span>Night time</span><input bind:value={night} placeholder="0:00" autocomplete="off" /></label>
      <label><span>Instrument</span><input bind:value={ifr} placeholder="0:00" autocomplete="off" /></label>
    </div>

    <div class="row">
      <label class="cs"><span>Callsign <em>(optional)</em></span><input class="code" bind:value={callsign} autocomplete="off" spellcheck="false" /></label>
      <label><span>Remarks</span><input bind:value={remarks} autocomplete="off" /></label>
    </div>

    <footer>
      {#if error}<span class="err">{error}</span>{/if}
      <span class="sp"></span>
      <button type="button" class="ghost" onclick={onclose}>Cancel</button>
      <button type="submit" class="primary" disabled={busy || !from.trim() || !to.trim()}>Add entry</button>
    </footer>
  </form>
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
    width: min(560px, 100%);
    max-height: 100%;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 18px;
    border-radius: 12px;
    background: var(--panel);
    border: 1px solid var(--rule);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
  }
  h3 {
    margin: 0;
    font-family: var(--cond);
    font-size: 22px;
  }
  header p {
    margin: 4px 0 0;
    color: var(--ink-dim);
    font-size: 13px;
  }
  .x {
    border: none;
    background: transparent;
    color: var(--ink-dim);
    padding: 2px;
    display: grid;
    align-self: flex-start;
  }
  .x svg {
    width: 20px;
    height: 20px;
    stroke: currentColor;
    stroke-width: 2;
  }
  .row {
    display: flex;
    gap: 10px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }
  label.cs {
    flex: 0 0 130px;
  }
  label > span {
    font-size: 12px;
    color: var(--ink-dim);
    white-space: nowrap;
  }
  em {
    font-style: normal;
    opacity: 0.7;
  }
  input,
  select {
    font: inherit;
    color: var(--ink);
    background: var(--panel-2);
    border: 1px solid var(--rule);
    border-radius: 6px;
    padding: 7px 10px;
    min-width: 0;
    color-scheme: dark;
  }
  input:focus,
  select:focus {
    outline: none;
    border-color: var(--amber);
  }
  .code {
    text-transform: uppercase;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }
  .sp {
    flex: 1;
  }
  .err {
    color: var(--bad);
    font-size: 13px;
  }
  .primary {
    border: none;
    border-radius: 6px;
    padding: 8px 14px;
    background: var(--amber);
    color: #1b1406;
    font-weight: 600;
  }
  .primary:disabled {
    opacity: 0.5;
  }
  .ghost {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    border-radius: 6px;
    padding: 7px 12px;
    font-weight: 600;
  }
  .ghost:hover {
    color: var(--ink);
  }
</style>
