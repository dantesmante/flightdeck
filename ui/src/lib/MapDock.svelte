<script lang="ts">
  // The card over the bottom right of the map. Between flights it is an
  // amber "Start a flight" button that opens the card; during a flight and
  // after landing the card shows the flight.
  import { cubicOut } from "svelte/easing";
  import { pfdState } from "./pfd/store.svelte";
  import { untrack } from "svelte";
  import { briefing, openBriefing } from "./briefingStore.svelte";
  import { fly as slide } from "svelte/transition";
  import type { FlightDetail, Status } from "./api";
  import { fly, pick, planClear } from "./flyStore.svelte";
  import LivePanel from "./LivePanel.svelte";
  import Logo from "./Logo.svelte";
  import { screen } from "./phone.svelte";
  import type { Pt } from "./track";

  interface Props {
    status: Status | null;
    completed: FlightDetail | null;
    track?: Pt[];
    ondismiss: () => void;
    onsettings: () => void;
    /** The room the dock takes over the map, so the map can keep clear of it. */
    oninset?: (left: number, bottom: number) => void;
  }
  let { status, completed, track = [], ondismiss, onsettings, oninset }: Props = $props();

  /** Start a flight was pressed (or an aircraft was picked on the map). */
  let open = $state(false);
  /** The flight card was put away to see more map. */
  let tucked = $state(false);

  const flying = $derived(!!status?.live);
  const busy = $derived(flying || !!completed);
  const shown = $derived(busy ? !tucked : open);

  // Split screen puts the card away to give the map its half; leaving it
  // brings the card back.
  let splitWas = false;
  let openBeforeSplit = false;
  $effect(() => {
    const split = pfdState.split && pfdState.open;
    untrack(() => {
      if (split && !splitWas) {
        openBeforeSplit = open;
        tucked = true;
        open = false;
      } else if (!split && splitWas) {
        tucked = false;
        open = openBeforeSplit;
      }
      splitWas = split;
    });
  });

  // "Fly this aircraft" on the map opens the card with that aircraft.
  $effect(() => {
    if (fly.item) open = true;
  });
  // An airport sent from the map's right-click menu opens the planner.
  $effect(() => {
    pick.locked = busy || !!status?.armed;
  });
  $effect(() => {
    if (pick.spot && !pick.locked) open = true;
  });
  // A flight started (or a plan loaded) elsewhere, like the EFB or a phone:
  // show it.
  const ready = $derived(!!(status?.armed || status?.plan) && !busy);
  let wasReady = false;
  $effect(() => {
    if (ready && !wasReady) open = true;
    wasReady = ready;
  });
  // A new flight or result comes out of its pocket.
  let wasBusy = false;
  $effect(() => {
    if (busy && !wasBusy) tucked = false;
    if (!busy && wasBusy) open = !!status?.armed;
    wasBusy = busy;
  });

  $effect(() => {
    // On a phone the card is a sheet across the bottom instead.
    if (screen.phone) oninset?.(0, shown ? Math.round(window.innerHeight * 0.4) : 0);
    else oninset?.(shown ? 380 : 0, 0);
  });

  function close() {
    if (busy) {
      tucked = true;
      return;
    }
    open = false;
    fly.item = null;
  }

  let cardEl = $state<HTMLElement>();
  /** The tear line's notches are real holes: the card is cut there (a mask), so the map shows through. */
  function punchHoles(card: HTMLElement) {
    let raf = 0;
    const cut = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(() => {
        const r = card.getBoundingClientRect();
        const ys = [...card.querySelectorAll<HTMLElement>(".perf")].map((p) => {
          const q = p.getBoundingClientRect();
          return Math.round(q.top - r.top + q.height / 2);
        });
        const layers = ys.flatMap((y) => [`radial-gradient(circle at 0 ${y}px, transparent 7px, #000 7.6px)`, `radial-gradient(circle at 100% ${y}px, transparent 7px, #000 7.6px)`]);
        const m = layers.join(", ");
        card.style.maskImage = m;
        card.style.setProperty("-webkit-mask-image", m);
        card.style.maskComposite = layers.length > 1 ? "intersect" : "";
        card.style.setProperty("-webkit-mask-composite", layers.length > 1 ? "source-in" : "");
      });
    };
    const ro = new ResizeObserver(cut);
    ro.observe(card);
    const mo = new MutationObserver(cut);
    mo.observe(card, { childList: true, subtree: true });
    card.addEventListener("scroll", cut, { passive: true });
    window.addEventListener("resize", cut);
    cut();
    return {
      destroy() {
        ro.disconnect();
        mo.disconnect();
        card.removeEventListener("scroll", cut);
        window.removeEventListener("resize", cut);
        cancelAnimationFrame(raf);
      },
    };
  }
  const label = $derived(busy ? (flying ? (status?.phase_label ?? "Show flight") : "Flight complete") : status?.armed ? "Flight started" : status?.plan ? "Flight plan ready" : "Start a flight");
</script>

{#if shown}
  <section
    bind:this={cardEl}
    use:punchHoles
    class="card"
    aria-label={busy ? "Your flight" : "Start a flight"}
    transition:slide={{ y: 60, duration: 260, easing: cubicOut, opacity: 0 }}
  >
    <div class="top" class:flat={busy}>
      {#if !busy}<h2>Start a flight</h2>{/if}
      {#if busy && status?.plan?.source === "simbrief"}
        <button class="ofp" onclick={() => status?.plan && openBriefing(status.plan)} disabled={briefing.loading} title="SimBrief's flight plan (OFP)">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 3h7l4 4v14H7zM14 3v4h4M10 11h5M10 14h5M10 17h3" /></svg>
          {briefing.loading ? "Opening…" : "OFP"}
        </button>
      {/if}
      {#if !busy}
        <button class="clear" onclick={() => planClear.n++} title="Clear the flight plan and start over">Clear</button>
      {/if}
      <button class="x" aria-label={busy ? "Put the card away" : "Close"} title={busy ? "Hide" : "Close"} onclick={close}>
        {#if busy}
          <svg viewBox="0 0 24 24"><path d="M6 9l6 6 6-6" /></svg>
        {:else}
          <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
        {/if}
      </button>
    </div>
    <LivePanel {status} {completed} {track} {ondismiss} {onsettings} />
  </section>
{:else}
  <button class="start" class:quiet={busy} onclick={() => (busy ? (tucked = false) : (open = true))} transition:slide={{ y: 20, duration: 180 }}>
    {#if busy}
      <svg viewBox="0 0 24 24" aria-hidden="true" class="chev"><path d="M6 15l6-6 6 6" /></svg>
    {:else}
      <Logo size={30} color="#1b1406" />
    {/if}
    {label}
  </button>
{/if}

<style>
  .card {
    position: absolute;
    z-index: 5;
    left: 16px;
    bottom: 16px;
    width: 356px;
    max-width: calc(100% - 32px);
    max-height: calc(100% - 32px);
    /* Scrolls when the window is short, without showing a scroll bar; the
       tear-line notches are cut off at the edges, as they should be. */
    overflow-x: hidden;
    overflow-y: auto;
    scrollbar-width: none;
    padding: 16px 18px 18px;
    background: rgba(28, 30, 33, 0.86);
    border: 1px solid var(--rule);
    border-radius: 14px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
  }
  .top {
    position: relative;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: -16px -18px 6px;
    padding: 14px 14px 6px 18px;
  }
  .card::-webkit-scrollbar {
    display: none;
  }
  .top.flat {
    position: absolute;
    top: 0;
    right: 0;
    margin: 0;
    padding: 10px;
    gap: 6px;
    background: none;
  }
  .ofp {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 30px;
    padding: 0 10px;
    border-radius: 8px;
    border: 1px solid #6d5bd0;
    background: rgba(28, 30, 33, 0.9);
    color: #c4b5fd;
    font-size: 12.5px;
    font-weight: 700;
    letter-spacing: 0.04em;
  }
  .ofp:hover:not(:disabled) {
    background: #2b2840;
  }
  .ofp svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  h2 {
    margin: 0;
    font-family: var(--sans);
    font-size: 21px;
    font-weight: 800;
    letter-spacing: -0.01em;
  }
  .clear {
    margin-left: auto;
    margin-right: 6px;
    height: 28px;
    padding: 0 12px;
    border: 1px solid var(--rule);
    border-radius: 7px;
    background: var(--panel-2);
    color: var(--ink);
    font: inherit;
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .clear:hover {
    border-color: var(--amber);
    color: var(--amber);
  }
  .x {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--ink-dim);
  }
  .x:hover {
    color: var(--ink);
    background: var(--panel-2);
  }
  .x svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .start {
    position: absolute;
    z-index: 5;
    left: 16px;
    bottom: 16px;
    min-width: 210px;
    max-width: calc(100% - 32px);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 11px 24px;
    border: none;
    border-radius: 10px;
    background: var(--amber);
    color: #1b1406;
    font-family: var(--cond);
    font-size: 19px;
    font-weight: 600;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4), 0 6px 20px rgba(240, 178, 58, 0.18);
    transition: transform 0.15s, box-shadow 0.15s;
  }
  .start:hover {
    transform: translateY(-1px);
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.45), 0 10px 26px rgba(240, 178, 58, 0.28);
  }
  .start .chev {
    width: 22px;
    height: 22px;
  }
  .start .chev {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .start.quiet {
    background: rgba(28, 30, 33, 0.95);
    color: var(--amber);
    border: 1px solid var(--rule);
  }
  /* Phones: the card is a sheet across the bottom of the screen. */
  @media (max-width: 640px) {
    .card {
      left: 10px;
      right: 10px;
      bottom: 10px;
      width: auto;
      max-width: none;
      max-height: 55%;
    }
    .start {
      left: 10px;
      bottom: 12px;
      min-width: 0;
      padding: 10px 18px;
      font-size: 17px;
    }
  }
</style>
