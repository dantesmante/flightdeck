<script lang="ts">
  // "Start a flight": load the latest SimBrief plan (or fly without one) and
  // tell FlightDeck to begin tracking as soon as you are in the cockpit.
  import { api, type FlightPlan } from "./api";
  import { fly } from "./flyStore.svelte";
  import { modelName } from "./hangar/names";
  import { prefs } from "./prefs.svelte";

  let { onsettings }: { onsettings: () => void } = $props();

  let open = $state(false);
  let loading = $state(false);
  let latest = $state<FlightPlan | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  const hasId = $derived(!!prefs.s.simbrief_id?.trim());

  async function show() {
    open = true;
    error = null;
    latest = null;
    if (!hasId) return;
    loading = true;
    try {
      latest = await api.simbriefLatest();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  // "Fly this aircraft" from the map opens this card with that aircraft.
  $effect(() => {
    if (fly.item && !open) show();
  });

  function close() {
    open = false;
    fly.item = null;
  }

  async function start(plan: FlightPlan | null) {
    busy = true;
    try {
      await api.startFlight(plan, fly.item?.id ?? null);
      open = false;
      fly.item = null;
    } finally {
      busy = false;
    }
  }

  const ago = (ms: number) => {
    if (!ms) return "";
    const m = Math.round((Date.now() - ms) / 60000);
    if (m < 2) return "dispatched just now";
    if (m < 90) return `dispatched ${m} min ago`;
    const h = Math.round(m / 60);
    if (h < 36) return `dispatched ${h} h ago`;
    return `dispatched ${Math.round(h / 24)} days ago`;
  };
</script>

{#if !open}
  <button class="start" onclick={show}>
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

    {#if fly.item}
      <div class="chosen">
        <span>
          <strong class="num">{fly.item.registration || fly.item.icao_type}</strong>
          {modelName(fly.item)}
        </span>
        <small>This flight is logged to this aircraft, whatever the sim shows.</small>
        <button class="clear" onclick={() => (fly.item = null)}>Use the sim's aircraft</button>
      </div>
    {/if}

    {#if !hasId}
      <button class="opt" onclick={onsettings}>
        <strong>Import from SimBrief</strong>
        <span>Add your SimBrief Pilot ID in Settings first</span>
      </button>
    {:else if loading}
      <div class="opt wait"><strong>Import from SimBrief</strong><span>Looking for your latest flight…</span></div>
    {:else if latest}
      <button class="opt sb" onclick={() => start(latest)} disabled={busy}>
        <strong>Import from SimBrief</strong>
        <span class="big num">{latest.callsign} ({latest.origin.icao}–{latest.destination.icao})</span>
        <span>{[latest.aircraft_icao, ago(latest.generated_ms)].filter(Boolean).join(", ")}</span>
      </button>
    {:else}
      <button class="opt" onclick={show}>
        <strong>Import from SimBrief</strong>
        <span class="err">{error ?? "No flight found"}</span>
        <span>Click to try again</span>
      </button>
    {/if}

    <div class="or">or</div>

    <button class="opt" onclick={() => start(null)} disabled={busy}>
      <strong>Fly without a flight plan</strong>
      <span>Tracking starts as soon as you are in the cockpit</span>
    </button>
    <div class="opt soon" aria-disabled="true">
      <strong>Enter a text route</strong>
      <span>Coming in a later version</span>
    </div>
  </div>
{/if}

<style>
  .chosen {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid rgba(240, 178, 58, 0.45);
    background: rgba(240, 178, 58, 0.08);
  }
  .chosen strong {
    font-size: 17px;
    margin-right: 6px;
    color: var(--amber);
  }
  .chosen small {
    color: var(--ink-dim);
    font-size: 12px;
  }
  .clear {
    align-self: flex-start;
    margin-top: 2px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--ink-dim);
    font-size: 12px;
    text-decoration: underline;
  }
  .clear:hover {
    color: var(--ink);
  }
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
    gap: 8px;
    padding: 12px;
    border-radius: 12px;
    border: 1px dashed #464a50;
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
  .opt {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 12px 14px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: var(--panel-3);
    color: var(--ink);
    text-align: center;
    font-size: 13px;
    transition: border-color 0.15s, background 0.15s;
  }
  button.opt:hover:not(:disabled) {
    border-color: var(--amber);
    background: #33373c;
  }
  .opt strong {
    font-size: 15px;
  }
  .opt span {
    color: var(--ink-dim);
  }
  .opt .big {
    color: var(--ink);
    font-size: 17px;
  }
  .opt.sb {
    border-color: rgba(167, 139, 250, 0.5);
  }
  .opt .err {
    color: var(--bad);
  }
  .opt.soon,
  .opt.wait {
    background: var(--panel-2);
    opacity: 0.6;
  }
  .or {
    text-align: center;
    color: var(--ink-faint);
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }
</style>
