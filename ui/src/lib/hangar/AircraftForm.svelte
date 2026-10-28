<script lang="ts">
  // Add an aircraft to the hangar, or edit one's details: registration,
  // model and airline. Statistics are never edited here.
  import { untrack } from "svelte";
  import { api, type HangarItem } from "../api";
  import { AIRLINES, airlineLook } from "./airlines";
  import { cleanType, codeFor, modelName, TYPES } from "./names";

  interface Props {
    /** The aircraft to edit; leave out to add a new one. */
    item?: HangarItem | null;
    onclose: () => void;
    /** Saved: the aircraft's id. */
    onsaved: (id: number) => void;
  }
  let { item = null, onclose, onsaved }: Props = $props();

  // The form starts from the aircraft as it was when opened.
  const start = untrack(() => item);
  const editing = !!start;
  let registration = $state(start?.registration ?? "");
  let model = $state(start ? modelName(start) : "");
  let airline = $state(start?.airline ?? "");
  let icao = $state(start ? cleanType(start.icao_type) : "");
  let engineType = $state("jet");
  let engineCount = $state(2);
  let typedCode = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);

  // Adding: fill the type code and engines from a known model name.
  $effect(() => {
    if (editing || typedCode) return;
    const c = codeFor(model);
    if (c) {
      icao = c;
      const name = TYPES[c] ?? "";
      if (/H1[2-6]5|Bell|Robinson|Sikorsky|Cabri|AW1|Chinook|H160|AS3/.test(name)) {
        engineType = "helo";
        engineCount = 1;
      } else if (/Schempp|Schleicher|DG Flug|Glider/.test(name)) {
        engineType = "glider";
        engineCount = 0;
      } else if (/Airbus A|Boeing [7-9]|Embraer E|CRJ|Citation|Gulfstream|Learjet|Falcon|Challenger|Global|Phenom|Praetor|HondaJet|PC-24|MD-|DC-10|Concorde/.test(name)) {
        engineType = "jet";
        engineCount = /A340|A380|747|A400/.test(name) ? 4 : /MD-11|DC-10|Falcon (7X|8X|50|900)/.test(name) ? 3 : name.includes("Vision Jet") ? 1 : 2;
      } else if (/ATR|Dash 8|King Air|PC-12|TBM|Caravan|Kodiak|Twin Otter|Saab|Jetstream|M600|Malibu Meridian|1900|SkyCourier|Porter/.test(name)) {
        engineType = "turboprop";
        engineCount = /ATR|Dash 8|King Air|Twin Otter|Saab|Jetstream|1900|SkyCourier/.test(name) ? 2 : 1;
      } else {
        engineType = "piston";
        engineCount = /Baron|Seneca|Seminole|Duchess|DA42|DA62|310|337|340|414|421|Navajo|Model 18|Twin/.test(name) ? 2 : 1;
      }
    }
  });

  const look = $derived(airlineLook(airline));

  async function save() {
    error = null;
    busy = true;
    try {
      if (item) {
        await api.updateAircraft(item.id, { registration, model: model === modelName({ ...item, model: null }) ? "" : model, airline });
        onsaved(item.id);
      } else {
        const id = await api.addAircraft({
          registration,
          model,
          icao_type: icao,
          engine_type: engineType,
          engine_count: Number(engineCount) || 0,
          airline: airline || null,
        });
        onsaved(id);
      }
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function remove() {
    if (!item) return;
    busy = true;
    try {
      await api.removeAircraft(item.id);
      onsaved(-1);
    } finally {
      busy = false;
    }
  }

  function key(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={key} />

<datalist id="fd-models">
  {#each Object.entries(TYPES) as [code, name] (code)}
    <option value={name}></option>
  {/each}
</datalist>
<datalist id="fd-airlines">
  {#each AIRLINES as a (a.icao + a.name)}
    <option value={a.icao}>{a.name}</option>
  {/each}
</datalist>

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <form
    class="sheet"
    aria-label={editing ? "Edit aircraft" : "Add aircraft"}
    onsubmit={(e) => {
      e.preventDefault();
      save();
    }}
  >
    <header>
      <h3>{editing ? "Edit aircraft" : "Add aircraft"}</h3>
      <button type="button" class="x" aria-label="Close" onclick={onclose}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg></button>
    </header>

    <div class="strip" style:--a={look?.colors[0] ?? "#2b2e32"} style:--b={look?.colors[1] ?? "#2b2e32"}>
      <span class="reg num">{registration.trim().toUpperCase() || "REGISTRATION"}</span>
      {#if look}<span class="al">{look.name}</span>{/if}
    </div>

    <label>
      <span>Registration</span>
      <input bind:value={registration} placeholder="e.g. N172SP or D-AINA" autocomplete="off" spellcheck="false" required />
    </label>
    {#if editing && item && item.sim_registration && item.sim_registration !== registration.trim().toUpperCase()}
      <p class="hint">The sim reports this aircraft as {item.sim_registration}. Flights it reports either way are logged here.</p>
    {:else if !editing}
      <p class="hint">Flights the sim reports with this registration are logged to this aircraft.</p>
    {/if}

    <label>
      <span>Model</span>
      <input bind:value={model} list="fd-models" placeholder="e.g. Boeing 737-800" autocomplete="off" />
    </label>

    {#if !editing}
      <div class="row">
        <label>
          <span>Type code (ICAO)</span>
          <input bind:value={icao} oninput={() => (typedCode = true)} placeholder="B738" maxlength="4" autocomplete="off" spellcheck="false" />
        </label>
        <label>
          <span>Engines</span>
          <select bind:value={engineType}>
            <option value="piston">Piston</option>
            <option value="turboprop">Turboprop</option>
            <option value="jet">Jet</option>
            <option value="helo">Helicopter</option>
            <option value="glider">Glider</option>
          </select>
        </label>
        <label class="n">
          <span>Number</span>
          <input type="number" min="0" max="8" bind:value={engineCount} />
        </label>
      </div>
    {/if}

    <label>
      <span>Airline <em>(optional)</em></span>
      <input bind:value={airline} list="fd-airlines" placeholder="Code or name, e.g. DLH or Lufthansa" autocomplete="off" />
    </label>

    <footer>
      {#if error}<span class="err">{error}</span>{/if}
      {#if editing && item?.manual && item.flights === 0}
        <button type="button" class="ghost danger" onclick={remove} disabled={busy}>Remove aircraft</button>
      {/if}
      <span class="sp"></span>
      <button type="button" class="ghost" onclick={onclose}>Cancel</button>
      <button type="submit" class="primary" disabled={busy || !registration.trim() || !(model.trim() || icao.trim())}>
        {editing ? "Save" : "Add aircraft"}
      </button>
    </footer>
  </form>
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
    width: min(460px, 100%);
    max-height: 100%;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 18px;
    border-radius: 12px;
    background: var(--panel);
    border: 1px solid var(--rule);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h3 {
    margin: 0;
    font-family: var(--cond);
    font-size: 22px;
  }
  .x {
    border: none;
    background: transparent;
    color: var(--ink-dim);
    padding: 2px;
    display: grid;
  }
  .x svg {
    width: 20px;
    height: 20px;
    stroke: currentColor;
    stroke-width: 2;
  }
  .strip {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 14px;
    border-radius: 8px;
    color: #fff;
    background: linear-gradient(100deg, var(--a) 0 70%, var(--b) 70% 76%, var(--a) 76%);
  }
  .strip .reg {
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 24px;
    font-weight: 600;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.45);
  }
  .al {
    font-size: 12px;
    font-weight: 600;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.6);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }
  label > span {
    font-size: 12px;
    color: var(--ink-dim);
  }
  em {
    font-style: normal;
    opacity: 0.7;
  }
  input,
  select {
    font: inherit;
    color: var(--ink);
    background: var(--panel-2);
    border: 1px solid var(--rule);
    border-radius: 6px;
    padding: 7px 10px;
    min-width: 0;
  }
  input:focus,
  select:focus {
    outline: none;
    border-color: var(--amber);
  }
  .row {
    display: flex;
    gap: 10px;
  }
  .row .n {
    flex: 0 0 72px;
  }
  .hint {
    margin: -6px 0 0;
    font-size: 12px;
    color: var(--ink-dim);
  }
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }
  .sp {
    flex: 1;
  }
  .err {
    color: var(--bad);
    font-size: 13px;
  }
  .primary {
    border: none;
    border-radius: 6px;
    padding: 8px 14px;
    background: var(--amber);
    color: #1b1406;
    font-weight: 600;
  }
  .primary:disabled {
    opacity: 0.5;
  }
  .ghost {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    border-radius: 6px;
    padding: 7px 12px;
    font-weight: 600;
  }
  .ghost:hover {
    color: var(--ink);
  }
  .danger:hover {
    color: var(--bad);
    border-color: var(--bad);
  }
</style>
