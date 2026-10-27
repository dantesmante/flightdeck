<script lang="ts">
  // A two-page book that turns its pages in 3D.
  //
  // Pages are numbered from 0; spread `s` shows page 2s on the left and
  // 2s+1 on the right. While a page turns, a "leaf" with the turning page
  // on its front and the next page on its back swings over the spine,
  // revealing the page underneath.
  //
  // With a `cover`, the book starts closed (spread -1): only the cover shows,
  // centred. Opening turns the cover like any other leaf, with page 0 printed
  // on its inside, while the book slides across to sit open in the middle.
  import type { Snippet } from "svelte";

  interface Props {
    pageCount: number;
    pageWidth: number;
    pageHeight: number;
    page: Snippet<[number]>;
    /** Front cover. When given, the book can be closed (spread -1). */
    cover?: Snippet;
    spread?: number;
    /** Extra classes for the book (paper colour etc. come from the pages). */
    label?: string;
    onturn?: (spread: number) => void;
  }
  let { pageCount, pageWidth, pageHeight, page, cover, spread = $bindable(0), label = "Book", onturn }: Props = $props();

  const spreads = $derived(Math.max(1, Math.ceil(pageCount / 2)));
  const first = $derived(cover ? -1 : 0);
  let turning = $state<"next" | "prev" | null>(null);
  /** 0 = flat, 1 = standing on edge (front shown), 2 = landed (back shown). */
  let stage = $state(0);
  let wrapW = $state(0);
  let wrapH = $state(0);

  /** Room kept on each side for the page arrows. */
  const SIDE = 76;
  const scale = $derived(
    wrapW && wrapH ? Math.min((wrapW - 2 * SIDE) / (pageWidth * 2), (wrapH - 24) / pageHeight, 1.6) : 1,
  );
  /** Which arrow the pointer is over: the matching page corner lifts. */
  let peek = $state<"next" | "prev" | null>(null);

  const reduced =
    typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

  const left = $derived(turning === "prev" ? 2 * spread - 2 : 2 * spread);
  const right = $derived(turning === "next" ? 2 * spread + 3 : 2 * spread + 1);

  function commit() {
    if (turning === "next") spread = Math.min(spreads - 1, spread + 1);
    if (turning === "prev") spread = Math.max(first, spread - 1);
    turning = null;
    stage = 0;
    onturn?.(spread);
  }

  export function next() {
    if (turning || spread >= spreads - 1) return;
    start("next");
  }

  export function prev() {
    if (turning || spread <= first) return;
    start("prev");
  }

  export function goTo(s: number) {
    if (turning) return;
    spread = Math.max(first, Math.min(spreads - 1, s));
    onturn?.(spread);
  }

  /** Close the book: jump back to the first spread, then turn the cover. */
  export function close() {
    if (turning || !cover || spread < 0) return;
    spread = 0;
    requestAnimationFrame(() => start("prev"));
  }

  /** Closed, or about to be: the cover sits centred. */
  const shut = $derived(
    !!cover &&
      ((spread === -1 && !(turning === "next" && stage > 0)) || (spread === 0 && turning === "prev" && stage > 0)),
  );

  function start(dir: "next" | "prev") {
    turning = dir;
    if (reduced) {
      commit();
      return;
    }
    // Two frames so the starting position paints before the transition.
    stage = 0;
    requestAnimationFrame(() => requestAnimationFrame(() => (stage = 1)));
  }

  /** First half done: show the back of the leaf for the second half. */
  function halfway() {
    if (stage === 1) stage = 2;
    else if (stage === 2) commit();
  }

  const angle = $derived((turning === "next" ? -1 : 1) * (stage === 0 ? 0 : stage === 1 ? 90 : 180));

  function key(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    if (t && (t.closest("input, textarea, select") || t.isContentEditable)) return;
    if (e.key === "ArrowRight" || e.key === "PageDown") next();
    if (e.key === "ArrowLeft" || e.key === "PageUp") prev();
  }
</script>

<svelte:window onkeydown={key} />

<div class="wrap" bind:clientWidth={wrapW} bind:clientHeight={wrapH}>
  <button
    class="arrow prev-arrow"
    class:hidden={spread <= first}
    disabled={spread <= first}
    aria-label="Previous page"
    onclick={prev}
    onmouseenter={() => (peek = "prev")}
    onmouseleave={() => (peek = null)}
    onfocus={() => (peek = "prev")}
    onblur={() => (peek = null)}
  >
    <span class="disc"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg></span>
    <span class="hint">Previous</span>
  </button>
  <button
    class="arrow next-arrow"
    class:hidden={spread >= spreads - 1}
    disabled={spread >= spreads - 1}
    aria-label="Next page"
    onclick={next}
    onmouseenter={() => (peek = "next")}
    onmouseleave={() => (peek = null)}
    onfocus={() => (peek = "next")}
    onblur={() => (peek = null)}
  >
    <span class="disc"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5l7 7-7 7" /></svg></span>
    <span class="hint">{spread < 0 ? "Open" : "Next"}</span>
  </button>
  <div
    class="book"
    role="region"
    aria-label={label}
    style:width="{pageWidth * 2}px"
    style:height="{pageHeight}px"
    class:shut
    style:transform="scale({scale}) translateX({shut ? -pageWidth / 2 : 0}px)"
  >
    <div class="page left" style:width="{pageWidth}px">
      {#if left >= 0 && left < pageCount}{@render page(left)}{/if}
      {#if left >= 0}<div class="spine-shade left-shade"></div>{/if}
    </div>
    <div class="page right" class:cover-page={right === -1} style:left="{pageWidth}px" style:width="{pageWidth}px">
      {#if right === -1}
        {@render cover?.()}
      {:else if right >= 0 && right < pageCount}
        {@render page(right)}
        <div class="spine-shade right-shade"></div>
      {/if}
    </div>

    {#if turning}
      <div
        class="leaf {turning}"
        class:rising={stage === 1}
        class:falling={stage === 2}
        style:width="{pageWidth}px"
        style:left={turning === "next" ? `${pageWidth}px` : "0px"}
        style:transform="rotateY({angle}deg)"
        ontransitionend={(e) => e.target === e.currentTarget && halfway()}
      >
        {#if stage < 2}
          {@const f = turning === "next" ? 2 * spread + 1 : 2 * spread}
          <div class="face" class:cover-page={f === -1}>
            {#if f === -1}{@render cover?.()}{:else}{@render page(f)}{/if}
            <div class="turn-shade"></div>
          </div>
        {:else}
          {@const b = turning === "next" ? 2 * spread + 2 : 2 * spread - 1}
          <div class="face mirrored" class:cover-page={b === -1}>
            {#if b === -1}
              {@render cover?.()}
            {:else if b < pageCount}
              {@render page(b)}
            {/if}
            <div class="turn-shade"></div>
          </div>
        {/if}
      </div>
    {/if}

    {#if spread > first && !shut}
      <button class="corner prev-corner" class:peek={peek === "prev"} tabindex="-1" aria-hidden="true" onclick={prev}></button>
    {/if}
    {#if spread < spreads - 1}
      <button class="corner next-corner" class:peek={peek === "next"} tabindex="-1" aria-hidden="true" onclick={next}></button>
    {/if}
  </div>
</div>

<style>
  .wrap {
    position: relative;
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    overflow: hidden;
  }
  .book {
    position: relative;
    perspective: 2600px;
    transform-origin: center center;
    filter: drop-shadow(0 18px 30px rgba(0, 0, 0, 0.45));
    transition: transform 0.68s cubic-bezier(0.45, 0.05, 0.3, 1);
  }
  .page.cover-page,
  .face.cover-page {
    border-radius: 6px 14px 14px 6px;
  }
  .page {
    position: absolute;
    top: 0;
    height: 100%;
    overflow: hidden;
  }
  .left {
    left: 0;
    border-radius: 6px 0 0 6px;
  }
  .right {
    border-radius: 0 6px 6px 0;
  }
  .spine-shade {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 46px;
    pointer-events: none;
  }
  .left-shade {
    right: 0;
    background: linear-gradient(to left, rgba(0, 0, 0, 0.16), rgba(0, 0, 0, 0));
  }
  .right-shade {
    left: 0;
    background: linear-gradient(to right, rgba(0, 0, 0, 0.18), rgba(0, 0, 0, 0));
  }
  .leaf {
    position: absolute;
    top: 0;
    height: 100%;
    z-index: 5;
  }
  .leaf.next {
    transform-origin: left center;
  }
  .leaf.prev {
    transform-origin: right center;
  }
  .leaf.rising {
    transition: transform 0.34s cubic-bezier(0.5, 0, 0.9, 0.6);
  }
  .leaf.falling {
    transition: transform 0.34s cubic-bezier(0.1, 0.4, 0.5, 1);
  }
  .face {
    position: absolute;
    inset: 0;
    overflow: hidden;
  }
  /* Past 90 degrees the leaf is seen from behind: flip its content back. */
  .face.mirrored {
    transform: scaleX(-1);
  }
  .turn-shade {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: linear-gradient(to right, rgba(0, 0, 0, 0.16), rgba(255, 255, 255, 0.05) 40%, rgba(0, 0, 0, 0.12));
  }
  .corner {
    position: absolute;
    bottom: 0;
    width: 64px;
    height: 64px;
    border: none;
    background: transparent;
    z-index: 6;
    cursor: pointer;
  }
  .corner::after {
    content: "";
    position: absolute;
    bottom: 0;
    width: 0;
    height: 0;
    transition: width 0.15s, height 0.15s;
  }
  .prev-corner {
    left: 0;
  }
  .next-corner {
    right: 0;
  }
  .next-corner::after {
    right: 0;
    background: linear-gradient(135deg, transparent 50%, rgba(0, 0, 0, 0.18) 50%);
  }
  .prev-corner::after {
    left: 0;
    background: linear-gradient(225deg, transparent 50%, rgba(0, 0, 0, 0.18) 50%);
  }
  .corner:hover::after,
  .corner.peek::after {
    width: 34px;
    height: 34px;
  }
  .corner.peek::after {
    width: 52px;
    height: 52px;
  }

  /* Side arrows: a disc that slides toward the way the page will turn,
     lights up amber, and lifts the matching page corner. */
  .arrow {
    position: absolute;
    top: 50%;
    z-index: 8;
    width: 64px;
    height: 120px;
    margin-top: -60px;
    border: none;
    background: transparent;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 0;
    cursor: pointer;
    transition: opacity 0.25s;
  }
  .prev-arrow {
    left: 6px;
  }
  .next-arrow {
    right: 6px;
  }
  .arrow.hidden {
    opacity: 0;
    pointer-events: none;
  }
  .disc {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    border: 1.5px solid var(--rule);
    background: rgba(23, 36, 48, 0.75);
    color: var(--ink-dim);
    transition:
      transform 0.25s cubic-bezier(0.3, 0.7, 0.4, 1.4),
      background 0.2s,
      color 0.2s,
      border-color 0.2s,
      box-shadow 0.25s;
  }
  .disc svg {
    width: 24px;
    height: 24px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform 0.25s cubic-bezier(0.3, 0.7, 0.4, 1.4);
  }
  .hint {
    font-size: 12px;
    font-weight: 600;
    color: var(--amber);
    opacity: 0;
    transform: translateY(-4px);
    transition: opacity 0.2s, transform 0.2s;
  }
  .arrow:hover .disc,
  .arrow:focus-visible .disc {
    background: var(--amber);
    border-color: var(--amber);
    color: #1b1406;
    box-shadow: 0 0 0 6px var(--amber-glow), 0 8px 24px rgba(0, 0, 0, 0.4);
  }
  .prev-arrow:hover .disc,
  .prev-arrow:focus-visible .disc {
    transform: translateX(-4px) scale(1.08);
  }
  .next-arrow:hover .disc,
  .next-arrow:focus-visible .disc {
    transform: translateX(4px) scale(1.08);
  }
  .prev-arrow:hover svg {
    transform: translateX(-2px);
  }
  .next-arrow:hover svg {
    transform: translateX(2px);
  }
  .arrow:hover .hint,
  .arrow:focus-visible .hint {
    opacity: 1;
    transform: none;
  }
  .arrow:active .disc {
    transform: scale(0.94);
  }
  .arrow:focus-visible {
    outline: none;
  }
</style>
