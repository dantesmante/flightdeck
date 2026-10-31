<script lang="ts">
  import { api, type FlightDetail, type Status } from "./api";
  import AltSpeedGraph from "./AltSpeedGraph.svelte";
  import PlanCard from "./PlanCard.svelte";
  import StartFlight from "./StartFlight.svelte";
  import { fromTrack, type Pt } from "./track";
  import { hhmm } from "./format";
  import { distance } from "./prefs.svelte";
  import BoardingPass from "./BoardingPass.svelte";
  import PlanForm from "./plan/PlanForm.svelte";
  import LandingCard from "./LandingCard.svelte";
  import PhaseLadder from "./PhaseLadder.svelte";
  import Readouts from "./Readouts.svelte";

  interface Props {
    status: Status | null;
    completed: FlightDetail | null;
    /** The live track so far, for the altitude & speed graph. */
    track?: Pt[];
    ondismiss: () => void;
    onsettings?: () => void;
  }
  let { status, completed, track = [], ondismiss, onsettings = () => {} }: Props = $props();

  /** Mid-flight: the flight plan form, to add a plan to the flight. */
  let addingPlan = $state(false);
  const finalLanding = $derived(completed?.landings.filter((l) => !l.touch_and_go).at(-1) ?? null);

  let routes = $state<string[]>([]);
  let route = $state(-1);
  let busy = $state(false);
  let confirmEnd = $state(false);
  let confirmCancel = $state(false);
  /** Keep a confirmation in view when the panel is short. */
  const reveal = (node: HTMLElement) => node.scrollIntoView({ block: "nearest", behavior: "smooth" });

  $effect(() => {
    api.demoRoutes().then((r) => (routes = r ?? []));
  });

  const connected = $derived(status?.link === "connected");
  const flying = $derived(!!status?.live);

  async function startDemo() {
    busy = true;
    await api.startDemo(route < 0 ? null : route);
    busy = false;
  }

  async function endFlight() {
    confirmEnd = false;
    await api.endFlight();
  }
</script>

<div class="live">
  {#if flying && status}
    <header>
      <h2>{status.phase_label}</h2>
      <p class="sub">
        {status.demo ? "Demo flight" : status.sim_label}{status.telemetry?.paused ? ", paused" : ""}
      </p>
    </header>
    <BoardingPass {status} />
    <Readouts telemetry={status.telemetry} live={status.live} />
    {#if track.length > 1}
      <AltSpeedGraph pts={track} height={130} />
    {/if}
    {#if status.plan}
      <PlanCard plan={status.plan} onremove={() => api.setPlan(null)} />
    {:else if !status.demo}
      {#if addingPlan}
        <div class="add-sheet">
          <div class="add-title">
            <span>Add a flight plan</span>
            <button class="x" aria-label="Close" onclick={() => (addingPlan = false)}>
              <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
            </button>
          </div>
          <PlanForm mode="attach" {onsettings} ondone={() => (addingPlan = false)} />
        </div>
      {:else}
        <button class="ghost add-plan" onclick={() => (addingPlan = true)}>Add a flight plan</button>
      {/if}
    {/if}
    {#if status.last_landing}
      <LandingCard landing={status.last_landing} />
    {/if}
    <PhaseLadder phase={status.phase} />
    <div class="actions">
      {#if confirmCancel}
        <div class="row inline" use:reveal title="The flight is thrown away and nothing is saved.">
          <span class="ask">Discard this flight?</span>
          <button class="danger" onclick={() => ((confirmCancel = false), api.cancelFlight())}>Discard</button>
          <button class="ghost" onclick={() => (confirmCancel = false)}>Keep it</button>
        </div>
      {:else if confirmEnd}
        <div class="row inline" use:reveal title="The flight is saved as partial.">
          <span class="ask">End and save as partial?</span>
          <button class="danger" onclick={endFlight}>End</button>
          <button class="ghost" onclick={() => (confirmEnd = false)}>Keep flying</button>
        </div>
      {:else}
        <button class="ghost" onclick={() => (confirmEnd = true)}>End flight</button>
        <button class="ghost" onclick={() => (confirmCancel = true)}>Cancel flight</button>
        {#if status.demo}
          <button class="ghost" onclick={() => api.stopDemo()}>Stop demo</button>
        {/if}
      {/if}
    </div>
  {:else if completed}
    {@const f = completed.flight}
    <header>
      <h2>Flight complete</h2>
      <p class="sub">
        {f.dep_icao ?? "????"} to {f.arr_icao ?? "????"} in {hhmm(f.total_min)}, {distance(f.distance_nm).value} {distance(f.distance_nm).unit}.
        Saved to your flights{f.status !== "complete" ? ` as ${f.status}` : ""}.
      </p>
    </header>
    {#if completed.track.length > 1}
      <AltSpeedGraph pts={fromTrack(completed.track)} height={150} />
    {/if}
    {#if finalLanding}
      <LandingCard landing={finalLanding} />
    {/if}
    <div class="actions">
      <button class="primary" onclick={ondismiss}>Done</button>
    </div>
  {:else if connected && status}
    <header>
      <h2><span class="good">Connected</span> to {status.sim_label}</h2>
      <p class="sub">
        {#if status.armed}
          Flight started. Tracking begins the moment you are in the cockpit.
        {:else}
          Get in the aircraft: tracking starts when the battery is on or an engine is running. Or press Start a flight.
        {/if}
      </p>
    </header>
    {#if status.plan}
      <PlanCard plan={status.plan} onremove={() => api.setPlan(null)} />
    {/if}
    {#if !status.armed}
      <StartFlight {onsettings} />
    {/if}
  {:else}
    <header>
      <h2>Ready when you are</h2>
      <p class="sub">
        Start MSFS 2020 or 2024 and FlightDeck connects by itself. Tracking starts when you power up the aircraft, or press Start a flight to load your SimBrief plan first.
      </p>
    </header>
    {#if status?.notice}
      <p class="notice">{status.notice}</p>
    {/if}
    {#if status?.plan}
      <PlanCard plan={status.plan} onremove={() => api.setPlan(null)} />
    {:else}
      <StartFlight {onsettings} />
    {/if}
    <div class="demo">
      <h3>No sim handy?</h3>
      <p>Fly a short demo flight to see tracking, the map and the landing report. It runs 30 times faster than real time.</p>
      <label>
        Route
        <select bind:value={route}>
          <option value={-1}>Surprise me</option>
          {#each routes as r, i (r)}
            <option value={i}>{r}</option>
          {/each}
        </select>
      </label>
      <button class="primary" onclick={startDemo} disabled={busy}>Fly a demo flight</button>
    </div>
  {/if}
</div>

<style>
  .live {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  header h2 {
    margin: 0;
    font-family: var(--cond);
    font-weight: 600;
    font-size: 28px;
    line-height: 1.1;
  }
  .sub {
    margin: 4px 0 0;
    color: var(--ink-dim);
    max-width: 34ch;
  }
  .notice {
    margin: 0;
    padding: 10px 12px;
    border-left: 3px solid var(--amber);
    background: var(--panel-2);
    color: var(--ink);
  }
  .demo {
    border-top: 1px solid var(--rule);
    padding-top: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .demo h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .demo p {
    margin: 0;
    color: var(--ink-dim);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    color: var(--ink-dim);
  }
  select {
    appearance: none;
    background-image: linear-gradient(45deg, transparent 50%, var(--ink-dim) 50%),
      linear-gradient(135deg, var(--ink-dim) 50%, transparent 50%);
    background-position: calc(100% - 16px) 50%, calc(100% - 11px) 50%;
    background-size: 5px 5px;
    background-repeat: no-repeat;
    font: inherit;
    color: var(--ink);
    background-color: var(--panel-2);
    border: 1px solid var(--rule);
    border-radius: 6px;
    padding: 7px 28px 7px 8px;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .row.inline {
    align-items: center;
    width: 100%;
  }
  .ask {
    margin-right: auto;
    color: var(--ink);
    font-weight: 600;
  }
  button {
    border-radius: 6px;
    padding: 8px 14px;
    font-weight: 600;
  }
  .primary {
    border: none;
    background: var(--amber);
    color: #1b1406;
    align-self: flex-start;
  }
  .primary:disabled {
    opacity: 0.6;
  }
  .ghost {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
  }
  .ghost:hover {
    color: var(--ink);
    border-color: var(--ink-faint);
  }
  .danger {
    border: none;
    background: var(--bad);
    color: #fff;
  }
  .add-plan {
    align-self: flex-start;
  }
  .add-sheet {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border-radius: 12px;
    border: 1px solid var(--rule);
  }
  .add-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-family: var(--cond);
    font-size: 18px;
    font-weight: 600;
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
  .good {
    color: var(--good);
  }
</style>
