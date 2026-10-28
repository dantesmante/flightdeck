<script lang="ts">
  import { onMount } from "svelte";
  import { api, type FlightPlan, type Pilot } from "./api";
  import { shrinkImage } from "./image";
  import { prefs, setPref } from "./prefs.svelte";
  import ExportDialog from "./ExportDialog.svelte";
  import { checkForUpdates, installUpdate, updates } from "./updates.svelte";

  let pilot = $state<Pilot | null>(null);
  let version = $state("");
  let saved = $state<string | null>(null);
  let busy = $state(false);
  let sampleMsg = $state<string | null>(null);

  onMount(async () => {
    pilot = await api.pilot();
    version = await api.version();
  });

  async function savePilot() {
    if (!pilot) return;
    pilot.nationality = pilot.nationality.trim().toUpperCase().slice(0, 3);
    pilot.home_icao = pilot.home_icao.trim().toUpperCase();
    await api.setPilot(pilot);
    saved = "Details saved. They appear on your passport's data page.";
  }

  async function pickPhoto(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file || !pilot) return;
    try {
      pilot.photo_url = await shrinkImage(file, 600, 0.85);
      await savePilot();
    } catch (err) {
      saved = String(err instanceof Error ? err.message : err);
    }
  }

  async function addSamples() {
    busy = true;
    sampleMsg = null;
    const n = await api.addSampleFlights();
    busy = false;
    sampleMsg = `Added ${n} sample flights. Open the passport, logbook or hangar to see them.`;
  }

  async function removeSamples() {
    busy = true;
    const n = await api.removeSampleFlights();
    busy = false;
    sampleMsg = n ? `Removed ${n} sample and demo flights.` : "There were no sample or demo flights to remove.";
  }

  const units = [
    { id: "nm", label: "Nautical miles" },
    { id: "mi", label: "Statute miles" },
    { id: "km", label: "Kilometers" },
  ];

  // SimBrief: the Pilot ID (or username) used to fetch the latest plan.
  let sbId = $state("");
  let sbBusy = $state(false);
  let sbPlan = $state<FlightPlan | null>(null);
  let sbError = $state<string | null>(null);
  $effect(() => {
    if (prefs.loaded && !sbId) sbId = prefs.s.simbrief_id ?? "";
  });
  async function saveSimbrief() {
    sbBusy = true;
    sbError = null;
    sbPlan = null;
    try {
      await setPref("simbrief_id", sbId.trim());
      if (sbId.trim()) sbPlan = await api.simbriefLatest();
    } catch (e) {
      sbError = String(e);
    } finally {
      sbBusy = false;
    }
  }

  // Volanta: import its flight files, export ours in the same format.
  let vBusy = $state(false);
  let vMsg = $state<string | null>(null);
  let vErr = $state<string | null>(null);
  let vFailed = $state<[string, string][]>([]);
  async function importFiles(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const list = [...(input.files ?? [])];
    input.value = "";
    if (!list.length) return;
    vBusy = true;
    vMsg = vErr = null;
    vFailed = [];
    let imported = 0;
    let dupes = 0;
    try {
      // A few files at a time, so hundreds of flights do not arrive at once.
      for (let i = 0; i < list.length; i += 20) {
        const batch = await Promise.all(list.slice(i, i + 20).map(async (f) => ({ name: f.name, text: await f.text() })));
        const r = await api.importVolanta(batch);
        imported += r.imported.length;
        dupes += r.duplicates.length;
        vFailed = [...vFailed, ...r.failed];
        vMsg = `Importing… ${Math.min(i + 20, list.length)} of ${list.length}`;
      }
      vMsg =
        `Imported ${imported} ${imported === 1 ? "flight" : "flights"}.` +
        (dupes ? ` ${dupes} already in FlightDeck, skipped.` : "") +
        (vFailed.length ? ` ${vFailed.length} could not be read.` : "");
    } catch (e) {
      vErr = String(e);
    } finally {
      vBusy = false;
    }
  }
  let exporting = $state(false);


  // Diagnostics: a report of recent flights and the log, to send when
  // something does not behave.
  let dBusy = $state(false);
  let dMsg = $state<string | null>(null);
  let dErr = $state(false);
  async function copyDiagnostics() {
    dBusy = true;
    dMsg = null;
    dErr = false;
    try {
      const text = await api.diagnostics();
      try {
        await navigator.clipboard.writeText(text);
      } catch {
        // Older clipboard route.
        const ta = document.createElement("textarea");
        ta.value = text;
        document.body.appendChild(ta);
        ta.select();
        document.execCommand("copy");
        ta.remove();
      }
      dMsg = `Copied (${Math.round(text.length / 1024)} KB). Paste it into the chat.`;
    } catch (e) {
      dMsg = String(e);
      dErr = true;
    } finally {
      dBusy = false;
    }
  }
  async function saveDiagnostics() {
    dBusy = true;
    dMsg = null;
    dErr = false;
    try {
      dMsg = `Saved to ${await api.saveDiagnostics()}`;
    } catch (e) {
      dMsg = String(e);
      dErr = true;
    } finally {
      dBusy = false;
    }
  }
</script>

<section class="settings"><div class="inner">
  <h2>Settings</h2>

  <div class="group">
    <h3>Pilot</h3>
    <p class="help">Shown on your passport's data page and logbook.</p>
    {#if pilot}
      <div class="pilot">
        <div class="photo">
          {#if pilot.photo_url}
            <img src={pilot.photo_url} alt="You" />
          {:else}
            <svg viewBox="0 0 100 120" aria-hidden="true"><circle cx="50" cy="42" r="22" /><path d="M10 120 C 12 82, 88 82, 90 120 Z" /></svg>
          {/if}
          <label class="ghost file">
            {pilot.photo_url ? "Change photo" : "Add photo"}
            <input type="file" accept="image/*" onchange={pickPhoto} />
          </label>
          {#if pilot.photo_url}
            <button class="link" onclick={() => pilot && ((pilot.photo_url = null), savePilot())}>Remove</button>
          {/if}
        </div>
        <div class="fields">
          <label>Full name<input bind:value={pilot.name} placeholder="Given names and surname" /></label>
          <label>Pilot ID or callsign<input bind:value={pilot.pilot_id} placeholder="Optional" /></label>
          <label>Nationality code<input bind:value={pilot.nationality} maxlength="3" placeholder="USA" /></label>
          <label>Home airport<input bind:value={pilot.home_icao} maxlength="8" placeholder="ICAO, e.g. KC81" /></label>
          <div class="row">
            <button class="primary" onclick={savePilot}>Save details</button>
            {#if saved}<span class="ok" role="status">{saved}</span>{/if}
          </div>
        </div>
      </div>
    {/if}
  </div>

  <div class="group">
    <h3>SimBrief</h3>
    <p class="help">
      Your SimBrief Pilot ID lets FlightDeck load the flight you last dispatched: press Start a flight on the map.
      Find it on simbrief.com under Account, Settings (a number). Your SimBrief username works too.
    </p>
    <div class="row">
      <label class="sb">Pilot ID<input bind:value={sbId} placeholder="e.g. 123456" onkeydown={(e) => e.key === "Enter" && saveSimbrief()} /></label>
      <button class="primary" onclick={saveSimbrief} disabled={sbBusy}>{sbBusy ? "Checking…" : "Save and check"}</button>
    </div>
    {#if sbPlan}
      <p class="ok" role="status">
        Connected. Latest flight: {sbPlan.callsign} ({sbPlan.origin.icao} to {sbPlan.destination.icao}){sbPlan.aircraft_icao ? `, ${sbPlan.aircraft_icao}` : ""}.
      </p>
    {:else if sbError}
      <p class="err" role="alert">{sbError}</p>
    {/if}
  </div>

  <div class="group">
    <h3>Volanta</h3>
    <p class="help">
      Bring your Volanta history across: select Volanta flight files (.json), one or many at once. Each becomes a
      FlightDeck flight on the map, in the logbook, passport and hangar, and the original file is kept so it can be
      exported again unchanged. Export lets you choose flights and saves them as Volanta-format files in your Downloads folder.
    </p>
    <div class="row">
      <label class="primary file" class:disabled={vBusy}>
        Import Volanta flights
        <input type="file" accept=".json,application/json" multiple onchange={importFiles} disabled={vBusy} />
      </label>
      <button class="ghost" onclick={() => (exporting = true)} disabled={vBusy}>Export…</button>
    </div>
    {#if vMsg}<p class="ok" role="status">{vMsg}</p>{/if}
    {#if vErr}<p class="err" role="alert">{vErr}</p>{/if}
    {#if vFailed.length}
      <ul class="failed">
        {#each vFailed as [name, why] (name)}<li><strong>{name}</strong>: {why}</li>{/each}
      </ul>
    {/if}
  </div>

  <div class="group">
    <h3>Units</h3>
    <div class="seg" role="radiogroup" aria-label="Distance">
      {#each units as u (u.id)}
        <button role="radio" aria-checked={prefs.s.distance_unit === u.id} class:on={prefs.s.distance_unit === u.id} onclick={() => setPref("distance_unit", u.id)}>
          {u.label}
        </button>
      {/each}
    </div>
  </div>

  <div class="group">
    <h3>Logbook</h3>
    <div class="seg" role="radiogroup" aria-label="Logbook layout">
      <button role="radio" aria-checked={prefs.s.logbook_layout === "faa"} class:on={prefs.s.logbook_layout === "faa"} onclick={() => setPref("logbook_layout", "faa")}>FAA layout</button>
      <button role="radio" aria-checked={prefs.s.logbook_layout === "easa"} class:on={prefs.s.logbook_layout === "easa"} onclick={() => setPref("logbook_layout", "easa")}>EASA layout</button>
    </div>
    <div class="seg" role="radiogroup" aria-label="Writing">
      <button role="radio" aria-checked={prefs.s.logbook_hand !== "type"} class:on={prefs.s.logbook_hand !== "type"} onclick={() => setPref("logbook_hand", "hand")}>Handwritten</button>
      <button role="radio" aria-checked={prefs.s.logbook_hand === "type"} class:on={prefs.s.logbook_hand === "type"} onclick={() => setPref("logbook_hand", "type")}>Typed</button>
    </div>
    <div class="seg" role="radiogroup" aria-label="Paper">
      <button role="radio" aria-checked={prefs.s.logbook_theme !== "dark"} class:on={prefs.s.logbook_theme !== "dark"} onclick={() => setPref("logbook_theme", "light")}>Light paper</button>
      <button role="radio" aria-checked={prefs.s.logbook_theme === "dark"} class:on={prefs.s.logbook_theme === "dark"} onclick={() => setPref("logbook_theme", "dark")}>Dark paper</button>
    </div>
    <div class="seg" role="radiogroup" aria-label="Logbook view">
      <button role="radio" aria-checked={prefs.s.logbook_view !== "list"} class:on={prefs.s.logbook_view !== "list"} onclick={() => setPref("logbook_view", "book")}>Logbook pages</button>
      <button role="radio" aria-checked={prefs.s.logbook_view === "list"} class:on={prefs.s.logbook_view === "list"} onclick={() => setPref("logbook_view", "list")}>Simple list</button>
    </div>
  </div>

  <div class="group">
    <h3>Diagnostics</h3>
    <p class="help">
      If a flight starts, ends or records something wrongly, this report shows what FlightDeck saw: your last five
      flights step by step and the recent log. It stays on your PC until you send it. Saving it as a file is easiest
      for long reports.
    </p>
    <div class="row">
      <button class="primary" onclick={saveDiagnostics} disabled={dBusy}>Save diagnostics file</button>
      <button class="ghost" onclick={copyDiagnostics} disabled={dBusy}>Copy diagnostics</button>
    </div>
    {#if dMsg}<p class={dErr ? "err" : "ok"} role="status">{dMsg}</p>{/if}
  </div>

  <div class="group">
    <h3>Updates</h3>
    <p class="help">You have FlightDeck {version || "…"}. New versions install in one click and keep all your flights.</p>
    <div class="row">
      {#if updates.info?.available}
        <button class="primary" onclick={installUpdate} disabled={updates.installing}>
          {updates.installing ? "Downloading…" : `Install version ${updates.info.latest}`}
        </button>
      {:else}
        <button class="ghost" onclick={checkForUpdates} disabled={updates.checking}>
          {updates.checking ? "Checking…" : "Check for updates"}
        </button>
        {#if updates.info && !updates.checking}<span class="ok">You have the latest version.</span>{/if}
      {/if}
    </div>
    {#if updates.error}<p class="err">{updates.error}</p>{/if}
  </div>

  <div class="group">
    <h3>Sample flights</h3>
    <p class="help">
      Five ready-made flights across the US, Canada and Europe in five different aircraft, one of them at night. Use them to
      try the map's route lines, the passport, the logbook and the hangar. Removing them also removes demo flights you flew from the map.
    </p>
    <div class="row">
      <button class="primary" onclick={addSamples} disabled={busy}>Add sample flights</button>
      <button class="ghost" onclick={removeSamples} disabled={busy}>Remove sample and demo flights</button>
    </div>
    {#if sampleMsg}<p class="ok" role="status">{sampleMsg}</p>{/if}
  </div>
</div></section>

{#if exporting}
  <ExportDialog onclose={() => (exporting = false)} />
{/if}

<style>
  .settings {
    height: 100%;
    overflow-y: auto;
  }
  .inner {
    padding: 22px 32px 40px;
    max-width: 820px;
  }
  h2 {
    margin: 0 0 8px;
    font-family: var(--cond);
    font-weight: 600;
    font-size: 26px;
  }
  .group {
    padding: 18px 0;
    border-bottom: 1px solid var(--rule);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }
  .help {
    margin: -4px 0 0;
    color: var(--ink-dim);
    max-width: 62ch;
  }
  .pilot {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 22px;
  }
  .photo {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }
  .photo img,
  .photo svg {
    width: 120px;
    height: 150px;
    object-fit: cover;
    border-radius: 6px;
    background: var(--panel-2);
  }
  .photo svg {
    fill: var(--panel-3);
  }
  .fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px 16px;
    align-content: start;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    color: var(--ink-dim);
  }
  input:not([type]) {
    font: inherit;
    font-size: 15px;
    color: var(--ink);
    background: var(--panel-2);
    border: 1px solid var(--rule);
    border-radius: 6px;
    padding: 7px 9px;
  }
  .sb {
    width: 220px;
  }
  .row {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .row:has(.sb) {
    align-items: flex-end;
  }
  .err {
    color: var(--bad);
    margin: 0;
  }
  .ok {
    color: var(--good);
    margin: 0;
    font-size: 14px;
  }
  .seg {
    display: inline-flex;
    align-self: flex-start;
    border: 1px solid var(--rule);
    border-radius: 6px;
    overflow: hidden;
  }
  .seg button {
    border: none;
    background: transparent;
    color: var(--ink-dim);
    padding: 7px 14px;
    font-weight: 600;
  }
  .seg button.on {
    background: var(--panel-3);
    color: var(--amber);
  }
  .primary {
    border: none;
    background: var(--amber);
    color: #1b1406;
    border-radius: 6px;
    padding: 8px 14px;
    font-weight: 600;
  }
  .primary:disabled {
    opacity: 0.6;
  }
  .ghost {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    border-radius: 6px;
    padding: 7px 12px;
    font-weight: 600;
    font-size: 14px;
    cursor: pointer;
  }
  .file {
    position: relative;
  }
  .file input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
  }
  .link {
    border: none;
    background: none;
    color: var(--ink-dim);
    padding: 0;
    font-size: 13px;
    text-decoration: underline;
  }
  label.primary.file {
    flex-direction: row;
    align-items: center;
    font-size: 15px;
    color: #1b1406;
    background: var(--amber);
    border-radius: 6px;
    padding: 8px 14px;
    font-weight: 600;
    cursor: pointer;
  }
  label.primary.file.disabled {
    opacity: 0.6;
    pointer-events: none;
  }
  label.file input {
    display: none;
  }
  .failed {
    margin: 0;
    padding-left: 18px;
    color: var(--ink-dim);
    font-size: 13px;
  }
</style>
