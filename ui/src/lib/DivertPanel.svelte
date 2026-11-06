<script lang="ts">
  // Divert: the nearest airports fit to land at (large and regional
  // airports with a hard runway of at least 2,000 ft, no grass strips), or
  // any airport typed in. Choosing one makes it the flight's destination.
  import { onMount } from "svelte";
  import { api, type DivertOption } from "./api";
  import AirportField, { type Spot } from "./plan/AirportField.svelte";
  import { distance } from "./prefs.svelte";

  interface Props {
    lat: number;
    lon: number;
    /** The destination planned now, left out of the list. */
    planned: string;
    onclose: () => void;
  }
  let { lat, lon, planned, onclose }: Props = $props();

  let options = $state<DivertOption[] | null>(null);
  let other = $state<Spot | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  // Looked up once when opened, from where the aircraft is then.
  onMount(() => {
    api.divertOptions(lat, lon, planned).then(
      (o) => (options = o ?? []),
      () => (options = []),
    );
  });

  const card = (b: number) => ["N", "NE", "E", "SE", "S", "SW", "W", "NW"][Math.round(((b % 360) + 360) % 360 / 45) % 8];

  async function go(icao: string) {
    busy = true;
    error = null;
    try {
      await api.divert(icao);
      onclose();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="divert" role="region" aria-label="Divert">
  <div class="head">
    <h3>Divert to</h3>
    <button class="x" aria-label="Close" onclick={onclose}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg></button>
  </div>
  <p class="sub">Nearest airports with a hard runway of 2,000 ft or more.</p>
  {#if options === null}
    <p class="sub">Looking…</p>
  {:else if !options.length}
    <p class="sub">None found nearby. Enter an airport below.</p>
  {:else}
    <ul>
      {#each options as o (o.ident)}
        {@const d = distance(o.distance_nm)}
        <li>
          <button disabled={busy} onclick={() => go(o.ident)}>
            <span class="id num">{o.ident}</span>
            <span class="nm">{o.name}{o.city ? `, ${o.city}` : ""}</span>
            <span class="far num">{d.value} {d.unit} {card(o.bearing)}</span>
            <span class="rw">Longest runway {o.longest_ft.toLocaleString("en-US")} ft{o.surface ? `, ${o.surface.toLowerCase()}` : ""}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  <div class="other">
    <span class="lbl">Or another airport</span>
    <AirportField label="Diversion airport" value={other} placeholder="ICAO or name" onchange={(v) => (other = v)} />
    <button class="primary" disabled={!other || busy} onclick={() => other && go(other.icao)}>Divert to {other?.icao ?? "…"}</button>
  </div>
  {#if error}<p class="err">{error}</p>{/if}
</div>

<style>
  .divert {
    margin-top: 10px;
    padding: 12px;
    border-radius: 10px;
    border: 1px solid rgba(240, 178, 58, 0.45);
    background: rgba(240, 178, 58, 0.05);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h3 {
    margin: 0;
    font-family: var(--cond);
    font-size: 19px;
    color: var(--amber);
  }
  .x {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--ink-dim);
  }
  .x svg {
    width: 17px;
    height: 17px;
    stroke: currentColor;
    stroke-width: 2;
    fill: none;
  }
  .sub {
    margin: 2px 0 8px;
    color: var(--ink-dim);
    font-size: 12.5px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  li button {
    width: 100%;
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 1px 10px;
    text-align: left;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid var(--rule);
    background: var(--panel-2);
    color: var(--ink);
    cursor: pointer;
  }
  li button:hover {
    border-color: var(--amber);
  }
  .id {
    font-size: 16px;
    font-weight: 700;
    color: var(--amber);
  }
  .nm {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    align-self: center;
  }
  .far {
    font-size: 13px;
    font-weight: 600;
    align-self: center;
  }
  .rw {
    grid-column: 1 / -1;
    font-size: 11.5px;
    color: var(--ink-dim);
  }
  .other {
    display: grid;
    gap: 6px;
    margin-top: 10px;
  }
  .lbl {
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-faint);
  }
  .primary {
    justify-self: start;
    border: none;
    border-radius: 6px;
    padding: 7px 14px;
    background: var(--amber);
    color: #1b1406;
    font-weight: 700;
  }
  .primary:disabled {
    opacity: 0.5;
  }
  .err {
    margin: 8px 0 0;
    color: #f07a72;
    font-size: 13px;
  }
</style>
