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
      {#each list as r (r.ident)}
        <button type="button" role="radio" aria-checked={value === r.ident} class:on={value === r.ident} class:wind={r.wind} title={windNote ? fmtWind(r) : ""} onclick={() => onchange(value === r.ident ? "" : r.ident)}>
          {r.ident}
        </button>
      {/each}
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
    gap: 4px;
  }
  .chips button {
    min-width: 40px;
    padding: 3px 8px;
    border: 1px solid var(--rule, #34373c);
    border-radius: 6px;
    background: transparent;
    color: var(--ink-dim, #a8adb4);
    font: 600 12px "IBM Plex Mono", ui-monospace, monospace;
    cursor: pointer;
  }
  .chips button.wind {
    border-color: rgba(90, 169, 255, 0.6);
    color: var(--ink, #eceef1);
  }
  .chips button.wind::after {
    content: " ↑";
    color: #5aa9ff;
  }
  .chips button.on {
    border-color: var(--amber, #f0b23a);
    background: rgba(240, 178, 58, 0.18);
    color: var(--amber, #f0b23a);
  }
  .note {
    font-size: 11px;
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
