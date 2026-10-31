<script lang="ts">
  // "Start a flight": a simple flight plan (typed in or imported from
  // SimBrief), then FlightDeck tracks from the moment you are in the cockpit.
  import { fly } from "./flyStore.svelte";
  import PlanForm from "./plan/PlanForm.svelte";

  let { onsettings }: { onsettings: () => void } = $props();

  let open = $state(false);

  // "Fly this aircraft" from the map opens this card with that aircraft.
  $effect(() => {
    if (fly.item && !open) open = true;
  });

  function close() {
    open = false;
    fly.item = null;
  }
</script>

{#if !open}
  <button class="start" onclick={() => (open = true)}>
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M2.5 13.5l7-1.5 5-8h2.5l-2.5 8 5.5-.5 1.8-2.5H23l-1.2 4 1.2 4h-1.2L20 14.5l-5.5-.5 2.5 8h-2.5l-5-8-7-1.5z" /></svg>
    Start a flight
  </button>
{:else}
  <div class="sheet" role="dialog" aria-label="Start a flight">
    <div class="title">
      <span>Start a flight</span>
      <button class="x" aria-label="Close" onclick={close}>
        <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
      </button>
    </div>
    {#key fly.item?.id}
      <PlanForm mode="start" aircraft={fly.item} {onsettings} ondone={close} />
    {/key}
  </div>
{/if}

<style>
  .start {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    width: 100%;
    padding: 12px 16px;
    border: none;
    border-radius: 10px;
    background: var(--amber);
    color: #1b1406;
    font-family: var(--cond);
    font-size: 19px;
    font-weight: 600;
    box-shadow: 0 6px 20px rgba(240, 178, 58, 0.18);
    transition: transform 0.15s, box-shadow 0.15s;
  }
  .start:hover {
    transform: translateY(-1px);
    box-shadow: 0 10px 26px rgba(240, 178, 58, 0.28);
  }
  .start svg {
    width: 22px;
    height: 22px;
    fill: currentColor;
  }
  .sheet {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border-radius: 12px;
    border: 1px solid var(--rule);
    background: var(--panel);
  }
  .title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-family: var(--cond);
    font-size: 18px;
    font-weight: 600;
    padding: 0 2px 2px;
  }
  .x {
    border: none;
    background: transparent;
    color: var(--ink-dim);
    padding: 2px;
    display: grid;
  }
  .x svg {
    width: 18px;
    height: 18px;
    stroke: currentColor;
    stroke-width: 2;
  }
</style>
