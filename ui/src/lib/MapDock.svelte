<script lang="ts">
  // The card over the bottom right of the map. Between flights it is an
  // amber "Start a flight" button that opens the card; during a flight and
  // after landing the card shows the flight.
  import { cubicOut } from "svelte/easing";
  import { fly as slide } from "svelte/transition";
  import type { FlightDetail, Status } from "./api";
  import { fly } from "./flyStore.svelte";
  import LivePanel from "./LivePanel.svelte";
  import Logo from "./Logo.svelte";
  import type { Pt } from "./track";

  interface Props {
    status: Status | null;
    completed: FlightDetail | null;
    track?: Pt[];
    ondismiss: () => void;
    onsettings: () => void;
    /** The room the dock takes over the map, so the map can keep clear of it. */
    oninset?: (right: number, bottom: number) => void;
  }
  let { status, completed, track = [], ondismiss, onsettings, oninset }: Props = $props();

  /** Start a flight was pressed (or an aircraft was picked on the map). */
  let open = $state(false);
  /** The flight card was put away to see more map. */
  let tucked = $state(false);

  const flying = $derived(!!status?.live);
  const busy = $derived(flying || !!completed);
  const shown = $derived(busy ? !tucked : open);

  // "Fly this aircraft" on the map opens the card with that aircraft.
  $effect(() => {
    if (fly.item) open = true;
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
    oninset?.(shown ? 380 : 0, shown ? 0 : 66);
  });

  function close() {
    if (busy) {
      tucked = true;
      return;
    }
    open = false;
    fly.item = null;
  }

  const label = $derived(busy ? (flying ? (status?.phase_label ?? "Show flight") : "Flight complete") : status?.armed ? "Flight started" : status?.plan ? "Flight plan ready" : "Start a flight");
</script>

{#if shown}
  <section
    class="card"
    aria-label={busy ? "Your flight" : "Start a flight"}
    transition:slide={{ y: 60, duration: 260, easing: cubicOut, opacity: 0 }}
  >
    <div class="top" class:flat={busy}>
      {#if !busy}<h2>Start a flight</h2>{/if}
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
    right: 16px;
    bottom: 16px;
    width: 356px;
    max-width: calc(100% - 32px);
    max-height: calc(100% - 80px);
    overflow-y: auto;
    scrollbar-width: thin;
    padding: 16px 18px 20px;
    background: rgba(28, 30, 33, 0.97);
    border: 1px solid var(--rule);
    border-radius: 14px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
  }
  .top {
    position: sticky;
    top: -16px;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: -16px -18px 6px;
    padding: 14px 14px 6px 18px;
    background: rgba(28, 30, 33, 0.97);
  }
  .top.flat {
    position: absolute;
    top: 0;
    right: 0;
    margin: 0;
    padding: 10px;
    background: none;
  }
  h2 {
    margin: 0;
    font-family: var(--cond);
    font-size: 22px;
    font-weight: 600;
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
    right: 16px;
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
</style>
