<script lang="ts" module>
  /** An airport in the plan: enough to show it and draw it on the map. */
  export interface Spot {
    icao: string;
    name: string;
    lat: number;
    lon: number;
  }
</script>

<script lang="ts">
  // One airport line of the flight plan: shows the chosen airport, or a
  // field that finds airports by code or name as you type.
  import { api, type AirportHit } from "../api";

  interface Props {
    value: Spot | null;
    placeholder: string;
    label: string;
    onchange: (v: Spot | null) => void;
  }
  let { value, placeholder, label, onchange }: Props = $props();

  let editing = $state(false);
  let text = $state("");
  let hits = $state<AirportHit[]>([]);
  let active = $state(0);
  let input = $state<HTMLInputElement | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;

  function edit() {
    editing = true;
    text = value?.icao ?? "";
    hits = [];
    queueMicrotask(() => input?.select());
  }

  function typed() {
    clearTimeout(timer);
    const q = text.trim();
    if (!q) {
      hits = [];
      return;
    }
    const mine = ++seq;
    timer = setTimeout(async () => {
      const r = (await api.searchAirports(q).catch(() => [])) ?? [];
      if (mine === seq) {
        hits = r;
        active = 0;
      }
    }, 120);
  }

  function pick(h: AirportHit) {
    onchange({ icao: h.icao, name: h.name, lat: h.lat, lon: h.lon });
    editing = false;
    hits = [];
  }

  function done() {
    // Leaving the field: a typed code that matches an airport counts.
    const q = text.trim().toUpperCase();
    const exact = hits.find((h) => h.icao === q);
    if (exact) pick(exact);
    else if (!q) {
      onchange(null);
      editing = false;
    } else editing = false;
  }

  function key(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      active = Math.min(active + 1, hits.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      active = Math.max(active - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (hits[active]) pick(hits[active]);
      else done();
    } else if (e.key === "Escape") {
      e.stopPropagation();
      editing = false;
      hits = [];
    }
  }
</script>

<div class="field">
  {#if editing}
    <input
      bind:this={input}
      bind:value={text}
      oninput={typed}
      onkeydown={key}
      onblur={() => setTimeout(done, 120)}
      placeholder="Code or name, e.g. KABQ"
      aria-label={label}
      autocomplete="off"
      spellcheck="false"
    />
    {#if hits.length}
      <ul class="hits" role="listbox" aria-label="{label} suggestions">
        {#each hits as h, i (h.icao)}
          <li role="option" aria-selected={i === active}>
            <button type="button" class:on={i === active} onmousedown={(e) => (e.preventDefault(), pick(h))}>
              <strong class="num">{h.icao}</strong>
              <span>{h.name}</span>
              <small>{h.country}</small>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {:else if value}
    <button type="button" class="shown" onclick={edit} title="Change {label.toLowerCase()}">
      <strong class="num">{value.icao}</strong>
      <span>{value.name}</span>
    </button>
    <button type="button" class="clear" aria-label="Remove {label.toLowerCase()}" onclick={() => onchange(null)}>
      <svg viewBox="0 0 24 24"><path d="M7 7l10 10M17 7L7 17" /></svg>
    </button>
  {:else}
    <button type="button" class="empty" onclick={edit}>{placeholder}</button>
  {/if}
</div>

<style>
  .field {
    position: relative;
    display: flex;
    align-items: center;
    min-width: 0;
    flex: 1;
  }
  input {
    width: 100%;
    font: inherit;
    font-size: 15px;
    color: var(--ink);
    background: var(--panel-2);
    border: 1px solid var(--amber);
    border-radius: 6px;
    padding: 6px 8px;
    outline: none;
  }
  .empty,
  .shown {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 8px;
    border: none;
    background: transparent;
    padding: 6px 0;
    text-align: left;
    font-size: 15px;
  }
  .empty {
    color: var(--ink-faint);
  }
  .empty:hover {
    color: var(--ink-dim);
  }
  .shown strong {
    font-size: 17px;
    color: var(--ink);
  }
  .shown span {
    color: var(--ink-dim);
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .shown:hover strong {
    color: var(--amber);
  }
  .clear {
    flex: none;
    border: none;
    background: transparent;
    color: var(--ink-faint);
    padding: 4px;
    display: grid;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .field:hover .clear,
  .clear:focus-visible {
    opacity: 1;
  }
  .clear:hover {
    color: var(--ink);
  }
  .clear svg {
    width: 14px;
    height: 14px;
    stroke: currentColor;
    stroke-width: 2;
  }
  .hits {
    position: absolute;
    z-index: 20;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--panel-3);
    border: 1px solid var(--rule);
    border-radius: 8px;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.45);
  }
  .hits button {
    width: 100%;
    display: grid;
    grid-template-columns: 46px 1fr;
    grid-template-areas: "code name" "code country";
    column-gap: 8px;
    align-items: center;
    border: none;
    border-radius: 5px;
    background: transparent;
    padding: 5px 6px;
    text-align: left;
    color: var(--ink);
  }
  .hits button.on {
    background: rgba(240, 178, 58, 0.14);
  }
  .hits strong {
    grid-area: code;
  }
  .hits span {
    grid-area: name;
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hits small {
    grid-area: country;
    font-size: 11px;
    color: var(--ink-dim);
  }
</style>
