<script lang="ts">
  // The pen palette shared by the scratch pad and map markup: pen or
  // eraser, colour, a size slider (2 to 10 px), undo, redo and clear.
  import type { Snippet } from "svelte";
  import { PEN_COLORS, pen } from "./store.svelte";

  interface Props {
    canUndo: boolean;
    canRedo: boolean;
    onundo: () => void;
    onredo: () => void;
    onclear: () => void;
    /** Extra buttons at the start (markup: draw or move the map). */
    lead?: Snippet;
    /** Extra buttons at the end (markup: Done). */
    tail?: Snippet;
    /** Little room: the colours fold into one swatch that opens them. */
    compact?: boolean;
  }
  let { canUndo, canRedo, onundo, onredo, onclear, lead, tail, compact = false }: Props = $props();

  let colorsOpen = $state(false);
  let swatch = $state<HTMLDivElement>();
  function outside(e: PointerEvent) {
    if (colorsOpen && swatch && !swatch.contains(e.target as Node)) colorsOpen = false;
  }
</script>

<svelte:window onpointerdown={outside} />

<div class="palette" class:compact role="toolbar" aria-label="Pen">
  {#if lead}{@render lead()}<span class="sep"></span>{/if}
  <div class="pair" role="radiogroup" aria-label="Pen or eraser">
    <button role="radio" aria-checked={!pen.erasing} class:on={!pen.erasing} title="Pen" onclick={() => (pen.erasing = false)}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20l1.2-4.2L15.5 5.5a2.1 2.1 0 0 1 3 3L8.2 18.8z" /><path d="M13.5 7.5l3 3" /></svg>
    </button>
    <button role="radio" aria-checked={pen.erasing} class:on={pen.erasing} title="Eraser" onclick={() => (pen.erasing = true)}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 20h11" /><path d="M5.6 14.4l8.8-8.8a2 2 0 0 1 2.8 0l2.2 2.2a2 2 0 0 1 0 2.8L11 19H8.4z" /><path d="M9.5 10.5l5 5" /></svg>
    </button>
  </div>
  {#if pen.erasing}
    <!-- What the eraser takes: just where it rubs, or whole lines it touches. -->
    <div class="pair modes" role="radiogroup" aria-label="Eraser">
      <button role="radio" aria-checked={pen.eraseMode === "area"} class:on={pen.eraseMode === "area"} title="Erase where the eraser passes" onclick={() => (pen.eraseMode = "area")}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="6.5" stroke-dasharray="2.6 2.2" /></svg>
      </button>
      <button role="radio" aria-checked={pen.eraseMode === "line"} class:on={pen.eraseMode === "line"} title="Erase whole lines" onclick={() => (pen.eraseMode = "line")}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 16c3-6 6-6 9 0s6 6 9 0" /><path d="M6 6l12 12" /></svg>
      </button>
    </div>
  {/if}
  <span class="sep"></span>
  {#snippet dots()}
    {#each PEN_COLORS as c (c)}
      <button
        role="radio"
        aria-checked={pen.color === c && !pen.erasing}
        class="dot"
        class:on={pen.color === c && !pen.erasing}
        style:--c={c}
        aria-label="Colour"
        onclick={() => ((pen.color = c), (pen.erasing = false), (colorsOpen = false))}
      ></button>
    {/each}
  {/snippet}
  {#if compact}
    <div class="swatch" bind:this={swatch}>
      <button class="dot on" style:--c={pen.color} aria-label="Choose a colour" aria-expanded={colorsOpen} onclick={() => (colorsOpen = !colorsOpen)}></button>
      {#if colorsOpen}
        <div class="pop" role="radiogroup" aria-label="Colour">{@render dots()}</div>
      {/if}
    </div>
  {:else}
    <div class="colors" role="radiogroup" aria-label="Colour">{@render dots()}</div>
  {/if}
  <span class="sep"></span>
  <label class="size" title="Line width">
    <span class="prev" aria-hidden="true"><i style:width="{pen.size}px" style:height="{pen.size}px" style:background={pen.erasing ? "var(--ink-dim)" : pen.color}></i></span>
    <input type="range" min="2" max="10" step="1" bind:value={pen.size} aria-label="Line width" />
    <span class="px">{pen.size} px</span>
  </label>
  <span class="sep"></span>
  <button class="ic" disabled={!canUndo} onclick={onundo} title="Undo" aria-label="Undo">
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 14L4 9l5-5" /><path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11" /></svg>
  </button>
  <button class="ic" disabled={!canRedo} onclick={onredo} title="Redo" aria-label="Redo">
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 14l5-5-5-5" /><path d="M20 9H9.5a5.5 5.5 0 0 0 0 11H13" /></svg>
  </button>
  <button class="txt" onclick={onclear} title="Clear everything">Clear</button>
  {#if tail}<span class="sep"></span>{@render tail()}{/if}
</div>

<style>
  .palette {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    background: rgba(28, 30, 33, 0.96);
    border: 1px solid var(--rule);
    border-radius: 14px;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(6px);
    max-width: 100%;
  }
  .sep {
    width: 1px;
    align-self: stretch;
    margin: 4px 2px;
    background: var(--rule);
    flex: none;
  }
  .palette :global(button) {
    height: 34px;
    min-width: 34px;
    border-radius: 9px;
    border: 0;
    background: transparent;
    color: var(--ink-dim);
    display: inline-grid;
    place-items: center;
    cursor: pointer;
    flex: none;
    font: inherit;
  }
  .palette :global(button:hover:not(:disabled)) {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.05);
  }
  .palette :global(button:disabled) {
    opacity: 0.3;
    cursor: default;
  }
  .palette :global(button.on) {
    background: var(--amber);
    color: #1b1406;
  }
  .palette :global(svg) {
    width: 19px;
    height: 19px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .pair {
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 11px;
    background: rgba(0, 0, 0, 0.25);
  }
  .colors {
    display: flex;
    gap: 3px;
  }
  .dot {
    min-width: 0 !important;
    width: 26px;
    height: 26px !important;
    border-radius: 50% !important;
  }
  .dot::after {
    content: "";
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--c);
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.35);
  }
  .dot.on {
    background: transparent !important;
    box-shadow: 0 0 0 2px var(--amber);
  }
  .size {
    display: flex;
    align-items: center;
    gap: 7px;
    flex: none;
    color: var(--ink-dim);
    font-size: 11.5px;
    font-family: "IBM Plex Mono", monospace;
  }
  .prev {
    width: 14px;
    display: grid;
    place-items: center;
  }
  .prev i {
    border-radius: 50%;
  }
  .size input {
    width: 92px;
    accent-color: var(--amber);
  }
  .px {
    width: 34px;
  }
  .swatch {
    position: relative;
  }
  .pop {
    position: absolute;
    bottom: calc(100% + 14px);
    left: -12px;
    display: flex;
    gap: 3px;
    padding: 6px;
    background: rgba(28, 30, 33, 0.98);
    border: 1px solid var(--rule);
    border-radius: 12px;
    box-shadow: 0 10px 26px rgba(0, 0, 0, 0.5);
    z-index: 10;
  }
  .compact .size input {
    width: 64px;
  }
  .compact .px {
    display: none;
  }
  .txt {
    padding: 0 10px;
    font-size: 13px !important;
    font-weight: 600;
    color: var(--ink) !important;
  }
</style>
