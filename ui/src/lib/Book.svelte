<script lang="ts">
  // A two-page book that turns its pages in 3D.
  //
  // Pages are numbered from 0; spread `s` shows page 2s on the left and
  // 2s+1 on the right. While a page turns, a "leaf" with the turning page
  // on its front and the next page on its back swings over the spine,
  // revealing the page underneath.
  import type { Snippet } from "svelte";

  interface Props {
    pageCount: number;
    pageWidth: number;
    pageHeight: number;
    page: Snippet<[number]>;
    spread?: number;
    /** Extra classes for the book (paper colour etc. come from the pages). */
    label?: string;
    onturn?: (spread: number) => void;
  }
  let { pageCount, pageWidth, pageHeight, page, spread = $bindable(0), label = "Book", onturn }: Props = $props();

  const spreads = $derived(Math.max(1, Math.ceil(pageCount / 2)));
  let turning = $state<"next" | "prev" | null>(null);
  /** 0 = flat, 1 = standing on edge (front shown), 2 = landed (back shown). */
  let stage = $state(0);
  let wrapW = $state(0);
  let wrapH = $state(0);

  const scale = $derived(
    wrapW && wrapH ? Math.min((wrapW - 24) / (pageWidth * 2), (wrapH - 24) / pageHeight, 1.6) : 1,
  );

  const reduced =
    typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

  const left = $derived(turning === "prev" ? 2 * spread - 2 : 2 * spread);
  const right = $derived(turning === "next" ? 2 * spread + 3 : 2 * spread + 1);

  function commit() {
    if (turning === "next") spread = Math.min(spreads - 1, spread + 1);
    if (turning === "prev") spread = Math.max(0, spread - 1);
    turning = null;
    stage = 0;
    onturn?.(spread);
  }

  export function next() {
    if (turning || spread >= spreads - 1) return;
    start("next");
  }

  export function prev() {
    if (turning || spread <= 0) return;
    start("prev");
  }

  export function goTo(s: number) {
    if (turning) return;
    spread = Math.max(0, Math.min(spreads - 1, s));
    onturn?.(spread);
  }

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
  <div
    class="book"
    role="region"
    aria-label={label}
    style:width="{pageWidth * 2}px"
    style:height="{pageHeight}px"
    style:transform="scale({scale})"
  >
    <div class="page left" style:width="{pageWidth}px">
      {#if left >= 0 && left < pageCount}{@render page(left)}{/if}
      <div class="spine-shade left-shade"></div>
    </div>
    <div class="page right" style:left="{pageWidth}px" style:width="{pageWidth}px">
      {#if right < pageCount}{@render page(right)}{/if}
      <div class="spine-shade right-shade"></div>
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
          <div class="face">
            {@render page(turning === "next" ? 2 * spread + 1 : 2 * spread)}
            <div class="turn-shade"></div>
          </div>
        {:else}
          <div class="face mirrored">
            {#if (turning === "next" ? 2 * spread + 2 : 2 * spread - 1) < pageCount}
              {@render page(turning === "next" ? 2 * spread + 2 : 2 * spread - 1)}
            {/if}
            <div class="turn-shade"></div>
          </div>
        {/if}
      </div>
    {/if}

    {#if spread > 0}
      <button class="corner prev-corner" aria-label="Previous page" onclick={prev}></button>
    {/if}
    {#if spread < spreads - 1}
      <button class="corner next-corner" aria-label="Next page" onclick={next}></button>
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
  .corner:focus-visible::after {
    width: 34px;
    height: 34px;
  }
</style>
