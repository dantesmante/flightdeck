<script lang="ts">
  // The runway to land on at the destination: one chip per runway end,
  // those most into the wind (from the airport's latest METAR) first and
  // marked, the way ATC usually picks. No free live source of the runways
  // actually in use exists worldwide, so the wind is the suggestion; the
  // pilot picks.
  import { api, type AirportWeather, type RunwayEnds } from "../api";

  let {
    icao,
    value,
    onchange,
    compact = false,
  }: {
    icao: string;
    /** The chosen runway ident ("" for none yet). */
    value: string;
    onchange: (rwy: string) => void;
    /** A single dropdown instead of chips (the profile's header). */
    compact?: boolean;
  } = $props();

  let ends = $state.raw<RunwayEnds | null>(null);
  let wx = $state.raw<AirportWeather | null>(null);
  // By value: a prop like `route.dest.icao` is read again whenever the
  // parent's route object is rebuilt (twice a second in flight), and
  // fetching again each time made the list blink.
  const airport = $derived(icao);
  $effect(() => {
    const id = airport;
    ends = null;
    wx = null;
    if (!id) return;
    let gone = false;
    api.runwayEnds(id).then((r) => !gone && (ends = r), () => {});
    api.airportWeather(id).then((w) => !gone && (wx = w), () => {});
    return () => (gone = true);
  });

  const rad = (d: number) => (d * Math.PI) / 180;
  /** Runway ends with their wind, most headwind first. */
  const list = $derived.by(() => {
    const e = ends?.ends ?? [];
    const dir = wx?.wind_dir;
    const kt = wx?.wind_kt ?? 0;
    const windy = dir != null && kt >= 3;
    const out = e.map((r) => {
      const head = windy ? kt * Math.cos(rad(dir! - r.heading_deg)) : 0;
      const cross = windy ? Math.abs(kt * Math.sin(rad(dir! - r.heading_deg))) : 0;
      return { ident: r.ident, head, cross };
    });
    if (windy) out.sort((a, b) => b.head - a.head || a.ident.localeCompare(b.ident));
    else out.sort((a, b) => a.ident.localeCompare(b.ident, undefined, { numeric: true }));
    const best = windy && out.length ? out[0].head : null;
    return out.map((r) => ({ ...r, wind: best != null && r.head >= best - 1 && r.head > 0 }));
  });
  const windNote = $derived(
    wx?.wind_dir != null && (wx.wind_kt ?? 0) >= 3 ? `Wind ${String(Math.round(wx.wind_dir)).padStart(3, "0")}° ${Math.round(wx.wind_kt!)} kt` : wx ? "Wind calm" : "",
  );
  /** One row of chips: the four best (the chosen one kept), the rest on demand. */
  let more = $state(false);
  const shown = $derived.by(() => {
    if (more || list.length <= 5) return list;
    const top = list.slice(0, 4);
    const pick = list.find((r) => r.ident === value);
    if (pick && !top.includes(pick)) top[3] = pick;
    return top;
  });
  const fmtWind = (r: { head: number; cross: number }) =>
    `${r.head >= 0 ? "Headwind" : "Tailwind"} ${Math.abs(Math.round(r.head))} kt · crosswind ${Math.round(r.cross)} kt`;
</script>

{#if compact}
  <label class="drop" title={windNote}>
    <span class="k">Runway</span>
    <select value={value} onchange={(e) => onchange((e.currentTarget as HTMLSelectElement).value)}>
      {#if !value}<option value="">Pick…</option>{/if}
      {#each list as r (r.ident)}
        <option value={r.ident}>{r.ident}{r.wind ? " · into wind" : ""}</option>
      {/each}
    </select>
  </label>
{:else if list.length}
  <div class="pick">
    <div class="chips" role="radiogroup" aria-label="Landing runway at {icao}">
      {#each shown as r (r.ident)}
        <button type="button" role="radio" aria-checked={value === r.ident} class:on={value === r.ident} class:wind={r.wind} title={windNote ? fmtWind(r) : ""} onclick={() => onchange(value === r.ident ? "" : r.ident)}>
          {r.ident}{#if r.wind}<svg viewBox="0 0 10 12" aria-hidden="true"><path d="M5 1v9M1.5 6.5 5 10l3.5-3.5" /></svg>{/if}
        </button>
      {/each}
      {#if shown.length < list.length}
        <button type="button" class="more" onclick={() => (more = true)} title="Show every runway">+{list.length - shown.length}</button>
      {:else if more && list.length > 5}
        <button type="button" class="more" onclick={() => (more = false)} title="Show fewer">Less</button>
      {/if}
    </div>
    {#if windNote}<span class="note">{windNote}{list.some((r) => r.wind) ? " · into wind marked" : ""}</span>{/if}
  </div>
{/if}

<style>
  .pick {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chips button {
    min-width: 50px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 3px 8px;
    border: 1.5px solid #4a4f56;
    border-radius: 5px;
    background: transparent;
    color: #c9ccd1;
    font: 600 13px "IBM Plex Mono", ui-monospace, monospace;
    cursor: pointer;
  }
  .chips button:hover {
    border-color: #6b7078;
  }
  .chips button svg {
    width: 9px;
    height: 11px;
    fill: none;
    stroke: #5aa9ff;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .chips button.on {
    border-color: var(--amber, #f0b23a);
    background: rgba(240, 178, 58, 0.22);
    color: #d9dce0;
  }
  .chips button.more {
    min-width: 0;
    border-style: dashed;
    color: var(--ink-dim, #a8adb4);
    font-size: 12px;
  }
  .note {
    font-size: 12px;
    color: var(--ink-faint, #7d828a);
  }
  .drop {
    display: inline-flex;
    flex-direction: column;
    line-height: 1.15;
  }
  .drop .k {
    font-size: 10.5px;
    color: var(--ink-faint, #7d828a);
  }
  .drop select {
    margin-top: 1px;
    padding: 1px 4px;
    border: 1px solid var(--rule, #34373c);
    border-radius: 5px;
    background: var(--panel, #1c1e21);
    color: #5aa9ff;
    font: 600 13px "IBM Plex Mono", ui-monospace, monospace;
  }
</style>
