<script lang="ts">
  // Before a bulk import: every flight found in the chosen files, ticked to
  // import. Flights FlightDeck seems to have already are shown in red and
  // left unticked; files that cannot be read are listed but cannot be ticked.
  import { untrack } from "svelte";
  import type { ImportPreview } from "./api";
  import { day, hhmm } from "./format";

  interface Props {
    title: string;
    items: ImportPreview[];
    onimport: (names: string[]) => void;
    onclose: () => void;
  }
  let { title, items, onimport, onclose }: Props = $props();

  let picked = $state(new Set(untrack(() => items).filter((i) => !i.error && !i.duplicate).map((i) => i.name)));
  let onlyDupes = $state(false);

  const ok = $derived(items.filter((i) => !i.error));
  const dupes = $derived(items.filter((i) => !i.error && i.duplicate));
  const unreadable = $derived(items.filter((i) => i.error));
  const shown = $derived(onlyDupes ? dupes : items);
  const allPicked = $derived(ok.length > 0 && ok.every((i) => picked.has(i.name)));

  function toggle(name: string) {
    const n = new Set(picked);
    if (n.has(name)) n.delete(name);
    else n.add(name);
    picked = n;
  }
  function setAll(on: boolean) {
    picked = new Set(on ? ok.map((i) => i.name) : []);
  }
  function key(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={key} />

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="sheet" role="dialog" aria-modal="true" aria-label={title}>
    <header>
      <div>
        <h3>{title}</h3>
        <p>
          {ok.length} {ok.length === 1 ? "flight" : "flights"} found.
          {#if dupes.length}
            <span class="bad">{dupes.length} {dupes.length === 1 ? "looks" : "look"} like {dupes.length === 1 ? "a flight" : "flights"} you already have</span>, shown in red and unticked. Tick one to import it anyway.
          {/if}
          {#if unreadable.length}{unreadable.length} {unreadable.length === 1 ? "file" : "files"} could not be read.{/if}
        </p>
      </div>
      <button class="x" aria-label="Close" onclick={onclose}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg></button>
    </header>
    <div class="tools">
      <button class="ghost" onclick={() => setAll(!allPicked)}>{allPicked ? "Select none" : "Select all"}</button>
      {#if dupes.length}
        <button class="ghost" onclick={() => (picked = new Set([...picked].filter((n) => !dupes.some((d) => d.name === n))))}>Untick duplicates</button>
        <label class="only"><input type="checkbox" bind:checked={onlyDupes} /> Show duplicates only</label>
      {/if}
    </div>
    <ul class="list">
      {#each shown as it (it.name)}
        <li>
          <label class:on={picked.has(it.name)} class:dupe={!!it.duplicate} class:broken={!!it.error}>
            <input type="checkbox" checked={picked.has(it.name)} disabled={!!it.error} onchange={() => toggle(it.name)} />
            <span class="route num">{it.error ? "–" : `${it.dep ?? "????"} → ${it.arr ?? "????"}`}</span>
            <span class="what">
              {#if it.error}
                <span class="file">{it.name}</span><span class="why">{it.error}</span>
              {:else}
                <span>{[it.callsign, it.icao_type, it.registration].filter(Boolean).join(" · ") || it.name}</span>
                {#if it.duplicate}<span class="why">{it.duplicate}</span>{/if}
              {/if}
            </span>
            <span class="when num">{it.error ? "" : day(it.start_ms)}</span>
            <span class="dur num">{it.error ? "" : hhmm(it.minutes)}</span>
          </label>
        </li>
      {/each}
    </ul>
    <footer>
      <span>{picked.size} of {ok.length} selected</span>
      <button class="ghost" onclick={onclose}>Cancel</button>
      <button class="primary" onclick={() => onimport([...picked])} disabled={picked.size === 0}>
        Import {picked.size} {picked.size === 1 ? "flight" : "flights"}
      </button>
    </footer>
  </div>
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
    width: min(820px, 100%);
    max-height: 100%;
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    background: var(--panel);
    border: 1px solid var(--rule);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }
  header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 18px 10px;
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
  .bad {
    color: #f07a72;
    font-weight: 600;
  }
  .x {
    border: none;
    background: transparent;
    color: var(--ink-dim);
    align-self: flex-start;
    padding: 2px;
    display: grid;
  }
  .x svg {
    width: 20px;
    height: 20px;
    stroke: currentColor;
    stroke-width: 2;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 18px 10px;
  }
  .only {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
    font-size: 13px;
    color: var(--ink-dim);
    cursor: pointer;
  }
  .only input {
    accent-color: var(--amber);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0 10px;
    overflow-y: auto;
    flex: 1;
    min-height: 120px;
    border-top: 1px solid var(--rule);
    border-bottom: 1px solid var(--rule);
  }
  label:not(.only) {
    display: grid;
    grid-template-columns: 22px 120px 1fr 96px 46px;
    align-items: center;
    gap: 10px;
    padding: 8px;
    margin: 2px 0;
    border-radius: 6px;
    border: 1px solid transparent;
    cursor: pointer;
    font-size: 14px;
  }
  label:not(.only):hover {
    background: var(--panel-2);
  }
  label input {
    width: 16px;
    height: 16px;
    accent-color: var(--amber);
  }
  label.dupe {
    background: rgba(226, 69, 60, 0.1);
    border-color: rgba(226, 69, 60, 0.45);
  }
  label.dupe .route,
  label.dupe .why {
    color: #f07a72;
  }
  label.broken {
    opacity: 0.6;
    cursor: default;
  }
  .what {
    display: grid;
    min-width: 0;
    color: var(--ink-dim);
  }
  .what > span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .why {
    font-size: 12px;
  }
  .file {
    color: var(--ink);
  }
  .when,
  .dur {
    color: var(--ink-dim);
    text-align: right;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 18px;
  }
  footer span {
    margin-right: auto;
    color: var(--ink-dim);
    font-size: 13px;
  }
  button {
    border-radius: 6px;
    padding: 7px 14px;
    font-weight: 600;
  }
  .primary {
    border: none;
    background: var(--amber);
    color: #1b1406;
  }
  .primary:disabled {
    opacity: 0.5;
  }
  .ghost {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
  }
  .ghost:hover {
    color: var(--ink);
  }
</style>
