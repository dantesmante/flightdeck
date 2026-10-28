<script lang="ts">
  // The hangar: a card for every aircraft flown, favourites first, then by
  // hours flown. Filter by category; hover the picture to upload a photo;
  // click anywhere else on a card to open the aircraft and its flights.
  import { onMount } from "svelte";
  import { api, type HangarItem } from "../api";
  import { hoursMinutes, int } from "../format";
  import { shrinkImage } from "../image";
  import AircraftDetail from "./AircraftDetail.svelte";
  import AircraftPicture from "./AircraftPicture.svelte";
  import { CATEGORIES, category, type Category } from "./kind";

  let { onsettings, onflight }: { onsettings: () => void; onflight?: (id: number) => void } = $props();

  let items = $state<HangarItem[]>([]);
  let loaded = $state(false);
  let photos = $state<Record<number, string>>({});
  let message = $state<string | null>(null);
  let openId = $state<number | null>(null);

  async function load() {
    items = (await api.hangar()) ?? [];
    loaded = true;
    for (const it of items.filter((i) => i.has_photo && !photos[i.id])) {
      const p = await api.aircraftPhoto(it.id);
      if (p) photos = { ...photos, [it.id]: p };
    }
  }
  onMount(load);

  let filter = $state<Category | null>(null);
  const counts = $derived.by(() => {
    const c: Record<string, number> = {};
    for (const i of items) c[category(i)] = (c[category(i)] ?? 0) + 1;
    return c;
  });
  const shown = $derived(filter ? items.filter((i) => category(i) === filter) : items);
  const totalMinutes = $derived(items.reduce((a, i) => a + i.minutes, 0));
  const opened = $derived(items.find((i) => i.id === openId) ?? null);

  const hours = (min: number) => (min / 60).toLocaleString("en-US", { maximumFractionDigits: 1, minimumFractionDigits: 1 });

  /** A friendlier aircraft name: the ICAO type spelled out when known. */
  const NAMES: Record<string, string> = {
    A20N: "Airbus A320neo", A21N: "Airbus A321neo", A319: "Airbus A319", A320: "Airbus A320", A321: "Airbus A321",
    A332: "Airbus A330-200", A333: "Airbus A330-300", A339: "Airbus A330-900", A359: "Airbus A350-900", A35K: "Airbus A350-1000",
    A388: "Airbus A380-800", B738: "Boeing 737-800", B38M: "Boeing 737 MAX 8", B739: "Boeing 737-900", B39M: "Boeing 737 MAX 9",
    B744: "Boeing 747-400", B748: "Boeing 747-8", B77W: "Boeing 777-300ER", B772: "Boeing 777-200", B788: "Boeing 787-8",
    B789: "Boeing 787-9", B78X: "Boeing 787-10", C172: "Cessna 172 Skyhawk", C152: "Cessna 152", C208: "Cessna 208 Caravan",
    C150: "Cessna 150", DH8D: "De Havilland Dash 8-400", AT76: "ATR 72-600", E190: "Embraer E190", E195: "Embraer E195",
    CRJ7: "Bombardier CRJ700", CRJ9: "Bombardier CRJ900", PC12: "Pilatus PC-12", TBM9: "Daher TBM 930", SR22: "Cirrus SR22",
    DA40: "Diamond DA40", DA62: "Diamond DA62", BE58: "Beechcraft Baron 58", PA28: "Piper PA-28", PA24: "Piper PA-24 Comanche",
  };
  const name = (it: HangarItem) => NAMES[it.icao_type] ?? (it.title || it.icao_type || "Unknown aircraft");

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
    it.favourite = !it.favourite;
    await api.setAircraftFavourite(it.id, it.favourite);
    // Favourites move to the top, then everything by hours flown.
    items = [...items].sort((a, b) => Number(b.favourite) - Number(a.favourite) || b.minutes - a.minutes);
  }
</script>

{#if opened}
  <AircraftDetail
    item={opened}
    photo={photos[opened.id] ?? null}
    displayName={name(opened)}
    onback={() => ((openId = null), load())}
    onupload={(e) => pick(opened, e)}
    {onflight}
  />
{:else}
  <section class="hangar">
    <header>
      <h2>Hangar</h2>
      {#if items.length}
        <p>{items.length} aircraft, {hoursMinutes(totalMinutes)} flown in total. Click a card to see its flights.</p>
      {/if}
    </header>
    {#if items.length}
      <div class="chips" role="group" aria-label="Filter by aircraft type">
        <button class:on={filter === null} onclick={() => (filter = null)}>All <span>{items.length}</span></button>
        {#each CATEGORIES as c (c.id)}
          <button class:on={filter === c.id} disabled={!counts[c.id]} onclick={() => (filter = filter === c.id ? null : (c.id as Category))}>
            {c.label} <span>{counts[c.id] ?? 0}</span>
          </button>
        {/each}
      </div>
    {/if}

    {#if loaded && items.length === 0}
      <div class="empty">
        <p>Your hangar is empty. Every aircraft you fly parks here, with its hours, landings and where you left it.</p>
        <button class="primary" onclick={onsettings}>Add sample flights</button>
      </div>
    {:else}
      <div class="grid">
        {#each shown as it (it.id)}
          <article class="card">
            <button class="open" onclick={() => (openId = it.id)} aria-label="Open {it.registration || name(it)}"></button>
            <h3 class="reg">{it.registration || it.icao_type || "Unregistered"}</h3>
            <div class="picture">
              <AircraftPicture item={it} own={photos[it.id] ?? null} onpick={(e) => pick(it, e)} onremove={() => removePhoto(it)} />
              <button class="star" class:on={it.favourite} onclick={() => star(it)} aria-pressed={it.favourite} aria-label={it.favourite ? "Remove from favourites" : "Add to favourites"}>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3.2l2.7 5.6 6.1.8-4.5 4.2 1.1 6.1L12 17l-5.4 2.9 1.1-6.1-4.5-4.2 6.1-.8z" /></svg>
              </button>
            </div>
            <p class="name">{name(it)}</p>
            <div class="where">
              <span class="lbl">Current aircraft location</span>
              <span class="icao num">{it.last_icao ?? "—"}</span>
              <span class="apt">{it.last_name ?? "Last position unknown"}</span>
            </div>
            <dl>
              <div><dt>Flights flown</dt><dd class="num">{it.flights}</dd></div>
              <div><dt>Hours flown</dt><dd class="num">{hours(it.minutes)}</dd></div>
              <div><dt>Avg. landing</dt><dd class="num">{it.avg_landing_fpm != null ? `${int(it.avg_landing_fpm)} ft/min` : "–"}</dd></div>
            </dl>
          </article>
        {/each}
        {#if shown.length === 0}<p class="none">No aircraft in this category yet.</p>{/if}
      </div>
    {/if}
    {#if message}<p class="msg" role="status">{message}</p>{/if}
  </section>
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
    transition: transform 0.18s, box-shadow 0.18s;
  }
  .card:hover {
    transform: translateY(-3px);
    box-shadow: 0 18px 38px rgba(0, 0, 0, 0.45);
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
    margin: 0;
    padding: 10px 14px 8px;
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-size: 27px;
    font-weight: 600;
    letter-spacing: 0.02em;
    background: #2b2e32;
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
