<script lang="ts">
  // The hangar: a card for every aircraft, favorites first, then by hours
  // flown. Show all or only favorites; add an aircraft by hand; hover the
  // picture to upload a photo; click anywhere else on a card to open the
  // aircraft and its flights.
  import { onMount } from "svelte";
  import { api, type HangarItem } from "../api";
  import { hoursMinutes, int } from "../format";
  import { shrinkImage } from "../image";
  import AircraftDetail from "./AircraftDetail.svelte";
  import AircraftForm from "./AircraftForm.svelte";
  import AircraftPicture from "./AircraftPicture.svelte";
  import { airlineLook } from "./airlines";
  import { modelName } from "./names";

  let { onsettings, onflight }: { onsettings: () => void; onflight?: (id: number) => void } = $props();

  let items = $state<HangarItem[]>([]);
  let loaded = $state(false);
  let photos = $state<Record<number, string>>({});
  let message = $state<string | null>(null);
  let openId = $state<number | null>(null);
  let onlyFavorites = $state(false);
  /** Favorites as they were when the list was last ordered: starring a
   * card leaves it where it is until the hangar is opened again. */
  let favSnapshot = $state<Set<number>>(new Set());
  /** The add/edit form: "new", an aircraft, or closed. */
  let form = $state<HangarItem | "new" | null>(null);

  async function load() {
    items = (await api.hangar()) ?? [];
    favSnapshot = new Set(items.filter((i) => i.favorite).map((i) => i.id));
    loaded = true;
    for (const it of items.filter((i) => i.has_photo && !photos[i.id])) {
      const p = await api.aircraftPhoto(it.id);
      if (p) photos = { ...photos, [it.id]: p };
    }
  }
  onMount(load);

  const favCount = $derived(items.filter((i) => i.favorite).length);
  const shown = $derived(onlyFavorites ? items.filter((i) => favSnapshot.has(i.id)) : items);
  const totalMinutes = $derived(items.reduce((a, i) => a + i.minutes, 0));
  const opened = $derived(items.find((i) => i.id === openId) ?? null);

  const hours = (min: number) => (min / 60).toLocaleString("en-US", { maximumFractionDigits: 1, minimumFractionDigits: 1 });
  const name = (it: HangarItem) => modelName(it);

  function showFavorites(on: boolean) {
    favSnapshot = new Set(items.filter((i) => i.favorite).map((i) => i.id));
    onlyFavorites = on;
  }

  async function pick(it: HangarItem, e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    try {
      const data = await shrinkImage(file);
      await api.setAircraftPhoto(it.id, data);
      photos = { ...photos, [it.id]: data };
      it.has_photo = true;
      message = `Photo saved for ${it.registration || it.icao_type || "this aircraft"}.`;
    } catch (err) {
      message = String(err instanceof Error ? err.message : err);
    }
  }

  async function removePhoto(it: HangarItem) {
    await api.setAircraftPhoto(it.id, null);
    const { [it.id]: _gone, ...rest } = photos;
    photos = rest;
    it.has_photo = false;
  }

  async function star(it: HangarItem) {
    it.favorite = !it.favorite;
    await api.setAircraftFavorite(it.id, it.favorite);
  }

  async function saved(id: number) {
    const wasNew = form === "new";
    form = null;
    await load();
    if (wasNew && id > 0) message = "Aircraft added to your hangar.";
  }
</script>

{#if opened}
  <AircraftDetail
    item={opened}
    photo={photos[opened.id] ?? null}
    displayName={name(opened)}
    onback={() => ((openId = null), load())}
    onupload={(e) => pick(opened, e)}
    onedit={() => (form = opened)}
    {onflight}
  />
{:else}
  <section class="hangar">
    <header>
      <h2>Hangar</h2>
      {#if items.length}
        <p>{items.length} aircraft, {hoursMinutes(totalMinutes)} flown in total. Click a card to see its flights.</p>
      {/if}
      <button class="add" onclick={() => (form = "new")}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
        Add aircraft
      </button>
    </header>
    {#if items.length}
      <div class="chips" role="group" aria-label="Show">
        <button class:on={!onlyFavorites} onclick={() => showFavorites(false)}>All <span>{items.length}</span></button>
        <button class:on={onlyFavorites} onclick={() => showFavorites(true)}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3.2l2.7 5.6 6.1.8-4.5 4.2 1.1 6.1L12 17l-5.4 2.9 1.1-6.1-4.5-4.2 6.1-.8z" /></svg>
          Favorites <span>{favCount}</span>
        </button>
      </div>
    {/if}

    {#if loaded && items.length === 0}
      <div class="empty">
        <p>Your hangar is empty. Every aircraft you fly parks here, with its hours, landings and where you left it. You can also add your aircraft by hand.</p>
        <button class="primary" onclick={onsettings}>Add sample flights</button>
      </div>
    {:else}
      <div class="grid">
        {#each shown as it (it.id)}
          {@const look = airlineLook(it.airline)}
          <article class="card">
            <button class="open" onclick={() => (openId = it.id)} aria-label="Open {it.registration || name(it)}"></button>
            <div class="reg" class:airline={!!look} style:--a={look?.colors[0]} style:--b={look?.colors[1]}>
              <h3 class="num">{it.registration || it.icao_type || "Unregistered"}</h3>
              {#if look}<span class="al">{look.name}</span>{/if}
              <button class="edit" onclick={() => (form = it)} aria-label="Edit {it.registration || 'aircraft'}" title="Edit registration, model and airline">
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16v4zM13.5 6.5l4 4" /></svg>
              </button>
            </div>
            <div class="picture">
              <AircraftPicture item={it} own={photos[it.id] ?? null} onpick={(e) => pick(it, e)} onremove={() => removePhoto(it)} />
              <button class="star" class:on={it.favorite} onclick={() => star(it)} aria-pressed={it.favorite} aria-label={it.favorite ? "Remove from favorites" : "Add to favorites"}>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3.2l2.7 5.6 6.1.8-4.5 4.2 1.1 6.1L12 17l-5.4 2.9 1.1-6.1-4.5-4.2 6.1-.8z" /></svg>
              </button>
            </div>
            <p class="name">{name(it)}</p>
            <div class="where">
              <span class="lbl">Current aircraft location</span>
              <span class="icao num">{it.last_icao ?? "—"}</span>
              <span class="apt">{it.last_name ?? (it.flights ? "Last position unknown" : "Not flown yet")}</span>
            </div>
            <dl>
              <div><dt>Flights flown</dt><dd class="num">{it.flights}</dd></div>
              <div><dt>Hours flown</dt><dd class="num">{hours(it.minutes)}</dd></div>
              <div><dt>Avg. landing</dt><dd class="num">{it.avg_landing_fpm != null ? `${int(it.avg_landing_fpm)} ft/min` : "–"}</dd></div>
            </dl>
          </article>
        {/each}
        {#if shown.length === 0}<p class="none">No favorites yet. Star an aircraft to keep it here.</p>{/if}
      </div>
    {/if}
    {#if message}<p class="msg" role="status">{message}</p>{/if}
  </section>
{/if}

{#if form}
  <AircraftForm item={form === "new" ? null : form} onclose={() => (form = null)} onsaved={saved} />
{/if}

<style>
  .hangar {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    background:
      radial-gradient(ellipse 50% 30% at 50% 0%, rgba(255, 255, 255, 0.04), transparent 70%),
      linear-gradient(180deg, #1c1e21, #17181a);
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 14px;
    flex-wrap: wrap;
    padding: 20px 28px 0;
  }
  h2 {
    margin: 0;
    font-family: var(--cond);
    font-weight: 600;
    font-size: 28px;
  }
  header p {
    margin: 0;
    color: var(--ink-dim);
  }
  .add {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--rule);
    background: #26282c;
    color: var(--ink);
    border-radius: 8px;
    padding: 7px 14px;
    font-weight: 600;
    align-self: center;
  }
  .add:hover {
    border-color: var(--amber);
    color: var(--amber);
  }
  .add svg,
  .chips svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .chips svg {
    vertical-align: -2px;
    margin-right: 2px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 14px 28px 0;
  }
  .chips button {
    border: 1px solid var(--rule);
    background: #26282c;
    color: var(--ink-dim);
    border-radius: 999px;
    padding: 6px 14px;
    font-weight: 600;
    font-size: 13px;
    transition: background 0.15s, color 0.15s, border-color 0.15s;
  }
  .chips button span {
    margin-left: 4px;
    font-weight: 500;
    opacity: 0.7;
  }
  .chips button:hover:not(:disabled) {
    color: var(--ink);
    background: #2e3135;
  }
  .chips button.on {
    color: #1b1406;
    background: var(--amber);
    border-color: var(--amber);
  }
  .chips button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .empty {
    margin: 80px auto;
    max-width: 44ch;
    text-align: center;
    color: var(--ink-dim);
  }
  .none {
    grid-column: 1 / -1;
    color: var(--ink-dim);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 26px;
    padding: 20px 28px 40px;
  }
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    border-radius: 14px;
    overflow: hidden;
    color: var(--ink);
    background: #26282c;
    border: 1px solid rgba(255, 255, 255, 0.06);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.35);
    transition: box-shadow 0.3s, border-color 0.3s;
  }
  /* Hover: a soft amber glow, as if lit from behind. */
  .card:hover {
    border-color: rgba(240, 178, 58, 0.35);
    box-shadow:
      0 0 0 1px rgba(240, 178, 58, 0.12),
      0 0 36px 6px rgba(240, 178, 58, 0.3),
      0 12px 30px rgba(0, 0, 0, 0.35);
  }
  /* The whole card opens the aircraft; the picture, star and upload sit on top. */
  .open {
    position: absolute;
    inset: 0;
    z-index: 1;
    border: none;
    background: transparent;
    padding: 0;
    cursor: pointer;
  }
  .reg {
    position: relative;
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 10px 44px 8px 14px;
    background: #2b2e32;
  }
  .reg h3 {
    margin: 0;
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 27px;
    font-weight: 600;
    letter-spacing: 0.02em;
    white-space: nowrap;
  }
  /* An airline's colors behind the registration: its main color with a
     cheat-line stripe in the accent color. */
  .reg.airline {
    color: #fff;
    background:
      linear-gradient(100deg, transparent 0 68%, var(--b) 68% 73%, transparent 73% 76%, var(--b) 76% 78%, transparent 78%),
      linear-gradient(180deg, rgba(255, 255, 255, 0.08), rgba(0, 0, 0, 0.18)),
      var(--a);
  }
  .reg.airline h3 {
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.45);
  }
  .al {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11.5px;
    font-weight: 600;
    opacity: 0.95;
    text-shadow: 0 1px 3px rgba(0, 0, 0, 0.6);
  }
  .edit {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    z-index: 2;
    width: 30px;
    height: 30px;
    border: none;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.25);
    color: rgba(255, 255, 255, 0.85);
    display: grid;
    place-items: center;
    opacity: 0;
    transition: opacity 0.2s, background 0.2s;
  }
  .card:hover .edit,
  .edit:focus-visible {
    opacity: 1;
  }
  .edit:hover {
    background: rgba(0, 0, 0, 0.45);
  }
  .edit svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linejoin: round;
  }
  .picture {
    position: relative;
    z-index: 2;
    aspect-ratio: 5 / 4;
    overflow: hidden;
  }
  .star {
    position: absolute;
    right: 8px;
    top: 8px;
    z-index: 4;
    width: 34px;
    height: 34px;
    padding: 4px;
    border: none;
    border-radius: 50%;
    background: rgba(20, 22, 25, 0.45);
    display: grid;
    place-items: center;
  }
  .star svg {
    width: 22px;
    height: 22px;
    fill: transparent;
    stroke: rgba(255, 255, 255, 0.9);
    stroke-width: 1.6;
    stroke-linejoin: round;
    transition: fill 0.15s, transform 0.15s;
  }
  .star:hover svg {
    transform: scale(1.12);
  }
  .star.on svg {
    fill: var(--amber);
    stroke: #8a5d0c;
  }
  .name {
    margin: 0;
    padding: 9px 12px 7px;
    font-size: 17px;
    border-bottom: 1px solid var(--rule);
  }
  .where {
    display: flex;
    flex-direction: column;
    padding: 8px 12px;
    border-bottom: 1px solid var(--rule);
  }
  .lbl,
  dt {
    font-size: 10px;
    white-space: nowrap;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--ink-dim);
  }
  .icao {
    font-size: 19px;
    font-weight: 600;
    line-height: 1.15;
  }
  .apt {
    font-size: 12px;
    color: var(--ink-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    padding: 8px 6px 14px;
    text-align: center;
  }
  dd {
    margin: 2px 0 0;
    font-size: 14px;
    font-weight: 600;
  }
  .msg {
    position: fixed;
    left: 50%;
    bottom: 18px;
    transform: translateX(-50%);
    margin: 0;
    padding: 8px 14px;
    border-radius: 6px;
    background: var(--strip);
    color: var(--strip-ink);
    font-weight: 600;
  }
</style>
