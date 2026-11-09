<script lang="ts">
  // The PFD in a window of its own over the app: moved by its title bar,
  // sized from its corner (it keeps the PFD's shape), both remembered on
  // this device. Full screen on a phone. On phones and tablets the terrain
  // ahead is drawn behind it (synthetic vision); on the simulator PC it is
  // plain sky and ground, to leave the graphics card to the sim.
  import { onDestroy, onMount } from "svelte";
  import { api, inRemote, on, type Pfd as PfdData } from "../api";
  import Pfd from "./Pfd.svelte";
  import Svt from "./Svt.svelte";
  import { pfdState } from "./store.svelte";

  let d = $state<PfdData | null>(null);
  let lastAt = 0;
  const KEY = "fd.pfdWindow";
  const SVT_KEY = "fd.pfdSvt";
  let box = $state(load());
  /** Terrain only where it costs the sim nothing: on devices (and in the
   * development preview with ?svt). */
  const svtOffered = inRemote || (import.meta.env.DEV && location.search.includes("svt"));
  let svt = $state(svtOffered && readSvt());

  function load(): { x: number; y: number; w: number } {
    try {
      const v = JSON.parse(localStorage.getItem(KEY) ?? "null");
      if (v && typeof v.x === "number") return v;
    } catch {
      /* default */
    }
    return { x: Math.max(16, window.innerWidth - 560), y: 120, w: 480 };
  }
  function readSvt() {
    try {
      return localStorage.getItem(SVT_KEY) !== "0";
    } catch {
      return true;
    }
  }
  function save() {
    try {
      localStorage.setItem(KEY, JSON.stringify(box));
    } catch {
      /* not remembered */
    }
  }
  function toggleSvt() {
    svt = !svt;
    try {
      localStorage.setItem(SVT_KEY, svt ? "1" : "0");
    } catch {
      /* not remembered */
    }
  }

  // Keep the feed coming while the window is open.
  let off: (() => void) | undefined;
  let timer: ReturnType<typeof setInterval> | undefined;
  onMount(() => {
    let gone = false;
    on<PfdData>("pfd", (p) => {
      d = p;
      lastAt = Date.now();
    }).then((u) => (gone ? u() : (off = u)));
    const ask = () => {
      api.pfdWatch().catch(() => {});
      if (Date.now() - lastAt > 4000) d = null;
    };
    ask();
    timer = setInterval(ask, 3000);
    return () => {
      gone = true;
    };
  });
  onDestroy(() => {
    off?.();
    clearInterval(timer);
  });

  // Moving and sizing.
  let drag: { kind: "move" | "size"; sx: number; sy: number; b: { x: number; y: number; w: number } } | null = null;
  function down(kind: "move" | "size", e: PointerEvent) {
    if (pfdState.split || pfdState.full) return;
    if ((e.target as HTMLElement).closest("button") && kind === "move") return;
    drag = { kind, sx: e.clientX, sy: e.clientY, b: { ...box } };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    const dx = e.clientX - drag.sx;
    const dy = e.clientY - drag.sy;
    if (drag.kind === "move") {
      box = {
        ...box,
        x: Math.min(Math.max(drag.b.x + dx, 60 - box.w), window.innerWidth - 60),
        y: Math.min(Math.max(drag.b.y + dy, 0), window.innerHeight - 40),
      };
    } else {
      box = { ...box, w: Math.min(Math.max(drag.b.w + dx, 300), window.innerWidth - 20) };
    }
  }
  function up() {
    if (drag) save();
    drag = null;
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && pfdState.full && (pfdState.full = false)} />
<div
  class="win"
  class:dock={pfdState.split}
  class:full={pfdState.full}
  style:left={pfdState.split || pfdState.full ? null : `${box.x}px`}
  style:top={pfdState.split || pfdState.full ? null : `${box.y}px`}
  style:width={pfdState.split || pfdState.full ? null : `${box.w}px`}
  role="dialog"
  aria-label="PFD"
>
  <header role="toolbar" tabindex="-1" aria-label="PFD window: drag to move" onpointerdown={(e) => down("move", e)} onpointermove={move} onpointerup={up}>
    <strong>PFD</strong>
    {#if d && !d.paused}<span class="live">Live</span>{/if}
    <span class="grow"></span>
    {#if svtOffered}
      <button class="svt-btn" class:on={svt} onclick={toggleSvt} title="Synthetic vision: the terrain ahead">SVT</button>
    {:else}
      <span class="hint" title="Synthetic vision draws the terrain ahead. It runs on phones and tablets connected to FlightDeck, so this PC keeps its graphics card for the sim.">SVT on phones &amp; tablets</span>
    {/if}
    <button class="svt-btn split" class:on={pfdState.split} onclick={() => ((pfdState.split = !pfdState.split), (pfdState.full = false))} title={pfdState.split ? "Back to a floating window" : "Split screen: map on the left, PFD on the right"}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="4" width="18" height="16" rx="2" /><path d="M12 4v16" /></svg>
      Split
    </button>
    <button class="svt-btn split" class:on={pfdState.full} onclick={() => ((pfdState.full = !pfdState.full), (pfdState.split = false))} title={pfdState.full ? "Back to a floating window (Esc)" : "Full window: the PFD fills FlightDeck's window"}>
      <svg viewBox="0 0 24 24" aria-hidden="true">
        {#if pfdState.full}<path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5" />{:else}<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" />{/if}
      </svg>
      {pfdState.full ? "Exit full" : "Full window"}
    </button>
    <button class="x" aria-label="Close the PFD" onclick={() => ((pfdState.open = false), (pfdState.split = false), (pfdState.full = false))}>
      <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
    </button>
  </header>
  <div class="screen">
    {#if svt}<Svt {d} />{/if}
    <div class="over"><Pfd {d} {svt} /></div>
  </div>
  {#if !pfdState.split && !pfdState.full}<div class="grip" role="presentation" onpointerdown={(e) => down("size", e)} onpointermove={move} onpointerup={up}></div>{/if}
</div>

<style>
  .win {
    position: fixed;
    z-index: 60;
    border-radius: 12px;
    overflow: hidden;
    background: #000;
    border: 1px solid var(--rule);
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.55);
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 6px 0 12px;
    background: var(--panel);
    border-bottom: 1px solid var(--rule);
    cursor: move;
    touch-action: none;
    user-select: none;
  }
  header strong {
    font-family: var(--cond);
    font-size: 16px;
    letter-spacing: 0.04em;
  }
  .live {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--good, #4ade80);
  }
  .grow {
    flex: 1;
  }
  .svt-btn {
    padding: 3px 9px;
    border-radius: 6px;
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    font-size: 12px;
    font-weight: 700;
  }
  .hint {
    font-size: 11.5px;
    color: var(--ink-faint);
    white-space: nowrap;
    cursor: help;
  }
  .split {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .split svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }
  /* Split screen: the right half of the Map tab, the PFD centred in it. */
  .win.dock {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 50%;
    border-radius: 0;
    border: none;
    border-left: 1px solid var(--rule);
    box-shadow: none;
    display: flex;
    flex-direction: column;
    z-index: 8;
  }
  .win.dock header {
    cursor: default;
  }
  /* Full window: the PFD over all of FlightDeck, centred, as large as fits. */
  .win.full {
    position: fixed;
    inset: 0;
    width: auto;
    border-radius: 0;
    border: none;
    box-shadow: none;
    display: flex;
    flex-direction: column;
    z-index: 200;
  }
  .win.full header {
    cursor: default;
  }
  .win.full .screen {
    flex: 1;
    aspect-ratio: auto;
  }
  .win.dock .screen {
    flex: 1;
    aspect-ratio: auto;
  }
  .svt-btn.on {
    border-color: var(--amber);
    color: var(--amber);
  }
  .x {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--ink-dim);
  }
  .x:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--ink);
  }
  .x svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
  .screen {
    position: relative;
    aspect-ratio: 20 / 17;
    overflow: hidden;
  }
  .over {
    position: absolute;
    inset: 0;
  }
  .grip {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 22px;
    height: 22px;
    cursor: nwse-resize;
    touch-action: none;
    background: linear-gradient(135deg, transparent 55%, rgba(255, 255, 255, 0.35) 55% 62%, transparent 62% 72%, rgba(255, 255, 255, 0.35) 72% 79%, transparent 79%);
  }
  @media (max-width: 640px) {
    .win {
      left: 0 !important;
      top: 0 !important;
      width: 100% !important;
      height: 100%;
      border-radius: 0;
      display: flex;
      flex-direction: column;
    }
    .screen {
      flex: 1;
      aspect-ratio: auto;
    }
    .grip,
    .split,
    .hint {
      display: none;
    }
  }
</style>
