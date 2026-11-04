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

  // Hover: the card tilts towards the pointer, with a holographic sheen and
  // a glare that follow it. Only CSS transforms, and only while hovered.
  const still = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  function tilt(e: PointerEvent) {
    const el = e.currentTarget as HTMLElement;
    // Over the picture (where a photo is changed) or the edit button: no effect.
    const calm = !!(e.target as HTMLElement | null)?.closest?.(".picture, .edit");
    el.classList.toggle("calm", calm);
    if (calm) {
      el.style.setProperty("--rx", "0deg");
      el.style.setProperty("--ry", "0deg");
      return;
    }
    const r = el.getBoundingClientRect();
    const x = (e.clientX - r.left) / r.width;
    const y = (e.clientY - r.top) / r.height;
    el.style.setProperty("--mx", `${(x * 100).toFixed(1)}%`);
    el.style.setProperty("--my", `${(y * 100).toFixed(1)}%`);
    if (!still) {
      el.style.setProperty("--rx", `${((0.5 - y) * 12).toFixed(2)}deg`);
      el.style.setProperty("--ry", `${((x - 0.5) * 14).toFixed(2)}deg`);
    }
  }
  function untilt(e: PointerEvent) {
    const el = e.currentTarget as HTMLElement;
    el.classList.remove("calm");
    el.style.setProperty("--rx", "0deg");
    el.style.setProperty("--ry", "0deg");
  }

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
          <article
            class="card"
            class:airline={!!look}
            style:--a={look?.colors[0]}
            style:--b={look?.colors[1]}
            style:--c={look?.colors[2] ?? look?.colors[1]}
            onpointermove={tilt}
            onpointerleave={untilt}
          >
            <button class="open" onclick={() => (openId = it.id)} aria-label="Open {it.registration || name(it)}"></button>
            <div class="banner">
              <h3>{name(it)}</h3>
              {#if look}
                <span class="al">{look.name}</span>
                <!-- The livery's tail swoosh in the corner: a broad band of the third
                     colour (or the accent) sweeping down to the left, the accent in
                     the corner beneath it. -->
                <svg class="swoosh" viewBox="0 0 30 40" preserveAspectRatio="none" aria-hidden="true">
                  <path d="M30 4 C 28 18, 25 30, 20 40 L 30 40 Z" fill={look.colors[1]} />
                  <path d="M21 0 L30 0 L30 4 C 28 18, 25 30, 20 40 L0 40 C 10 33, 18 20, 21 0 Z" fill={look.colors[2] ?? look.colors[1]} />
                </svg>
              {/if}
            </div>
            <div class="picture">
              <AircraftPicture item={it} own={photos[it.id] ?? null} onpick={(e) => pick(it, e)} onremove={() => removePhoto(it)} />
              <span class="reg num">{it.registration || it.icao_type || "No reg."}</span>
              <button class="edit" onclick={() => (form = it)} aria-label="Edit {it.registration || 'aircraft'}" title="Edit registration, model and airline">
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16v4zM13.5 6.5l4 4" /></svg>
              </button>
            </div>
            <div class="where">
              <span class="lbl">Current aircraft location</span>
              <span class="icao num">{it.last_icao ?? "—"}</span>
              <span class="apt">{it.last_name ?? (it.flights ? "Last position unknown" : "Not flown yet")}</span>
            </div>
            <div class="foot">
              <dl>
                <div><dt>Flights</dt><dd class="num">{it.flights}</dd></div>
                <div><dt>Hours</dt><dd class="num">{hours(it.minutes)}</dd></div>
                <div><dt>Avg. landing</dt><dd class="num">{it.avg_landing_fpm != null ? `${int(it.avg_landing_fpm)} ft/min` : "–"}</dd></div>
              </dl>
              <button class="star" class:on={it.favorite} onclick={() => star(it)} aria-pressed={it.favorite} aria-label={it.favorite ? "Remove from favorites" : "Add to favorites"}>
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3.2l2.7 5.6 6.1.8-4.5 4.2 1.1 6.1L12 17l-5.4 2.9 1.1-6.1-4.5-4.2 6.1-.8z" /></svg>
              </button>
            </div>
            <span class="holo" aria-hidden="true"></span>
            <span class="glare" aria-hidden="true"></span>
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
    gap: 18px;
    padding: 20px 28px 40px;
  }
  .card {
    --rx: 0deg;
    --ry: 0deg;
    --mx: 50%;
    --my: 50%;
    position: relative;
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    overflow: hidden;
    color: var(--ink);
    background: #26282c;
    border: 1px solid rgba(255, 255, 255, 0.07);
    box-shadow: 0 10px 26px rgba(0, 0, 0, 0.35);
    transform: perspective(900px) rotateX(var(--rx)) rotateY(var(--ry));
    transition:
      transform 0.5s cubic-bezier(0.2, 0.8, 0.2, 1),
      box-shadow 0.3s;
  }
  .card:hover {
    transform: perspective(900px) rotateX(var(--rx)) rotateY(var(--ry)) scale(1.02);
    transition:
      transform 0.12s ease-out,
      box-shadow 0.3s;
    box-shadow:
      0 0 0 1px rgba(240, 178, 58, 0.18),
      0 18px 40px rgba(0, 0, 0, 0.5);
  }
  /* The holographic film and the glare, over everything, never in the way. */
  .holo,
  .glare {
    position: absolute;
    inset: 0;
    z-index: 5;
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.35s;
    border-radius: inherit;
  }
  /* Brushed metal: fine horizontal grain, with a band of silver light that
     moves with the pointer, and a soft glare where it is. */
  .holo {
    background:
      repeating-linear-gradient(0deg, rgba(255, 255, 255, 0.05) 0 1px, rgba(0, 0, 0, 0.04) 1px 2px, transparent 2px 3px),
      linear-gradient(
        105deg,
        transparent 20%,
        rgba(200, 206, 214, 0.25) 34%,
        rgba(255, 255, 255, 0.55) 44%,
        rgba(176, 184, 194, 0.3) 50%,
        rgba(240, 243, 247, 0.5) 57%,
        rgba(160, 168, 178, 0.2) 66%,
        transparent 80%
      );
    background-size:
      100% 100%,
      280% 280%;
    background-position:
      0 0,
      var(--mx) var(--my);
    mix-blend-mode: soft-light;
  }
  .glare {
    background: radial-gradient(circle at var(--mx) var(--my), rgba(255, 255, 255, 0.3) 0%, rgba(255, 255, 255, 0.06) 32%, transparent 55%);
    mix-blend-mode: overlay;
  }
  .card:hover .holo {
    opacity: 0.9;
  }
  .card:hover .glare {
    opacity: 1;
  }
  .card:global(.calm) .holo,
  .card:global(.calm) .glare {
    opacity: 0;
  }
  .card:global(.calm):hover {
    transform: perspective(900px) rotateX(0deg) rotateY(0deg);
  }
  @media (prefers-reduced-motion: reduce) {
    .card,
    .card:hover {
      transform: none;
    }
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
  /* The banner: the model, and the airline in its livery colours. */
  .banner {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 40px;
    padding: 0 12px;
    background: #2b2e32;
  }
  .card.airline .banner {
    background: var(--a);
    color: #fff;
    padding-right: 36px;
  }
  .banner h3 {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 15.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .card.airline .banner h3 {
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.35);
  }
  .al {
    flex: none;
    max-width: 45%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 10.5px;
    font-weight: 700;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.45);
  }
  .swoosh {
    position: absolute;
    top: 0;
    right: 0;
    width: 30px;
    height: 100%;
  }
  .picture {
    position: relative;
    z-index: 2;
    aspect-ratio: 2 / 1;
    overflow: hidden;
  }
  /* The registration, as a tag on the picture in the airline's colour. */
  .reg {
    position: absolute;
    top: 7px;
    right: 7px;
    z-index: 3;
    padding: 3px 11px;
    border-radius: 999px;
    background: #4a4d52;
    color: #fff;
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.02em;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
    pointer-events: none;
  }
  .card.airline .reg {
    background: var(--a);
  }
  .edit {
    position: absolute;
    left: 7px;
    top: 7px;
    z-index: 4;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.45);
    color: rgba(255, 255, 255, 0.9);
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
    background: rgba(0, 0, 0, 0.65);
  }
  .edit svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linejoin: round;
  }
  .where {
    display: flex;
    flex-direction: column;
    padding: 8px 10px 7px;
    border-bottom: 1px solid var(--rule);
  }
  .lbl,
  dt {
    font-size: 9.5px;
    white-space: nowrap;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--ink-dim);
  }
  .icao {
    font-size: 18px;
    font-weight: 600;
    line-height: 1.2;
  }
  .apt {
    font-size: 11.5px;
    color: var(--ink-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .foot {
    display: flex;
    align-items: stretch;
  }
  dl {
    flex: 1;
    margin: 0;
    display: grid;
    grid-template-columns: 0.8fr 0.8fr 1.3fr;
    padding: 7px 4px 8px;
    text-align: center;
  }
  dd {
    margin: 2px 0 0;
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
  }
  .star {
    position: relative;
    z-index: 2;
    flex: none;
    width: 52px;
    padding: 0;
    border: none;
    border-left: 1px solid var(--rule);
    background: transparent;
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .star svg {
    width: 22px;
    height: 22px;
    fill: transparent;
    stroke: rgba(255, 255, 255, 0.55);
    stroke-width: 1.5;
    stroke-linejoin: round;
    transition: fill 0.15s, transform 0.15s, stroke 0.15s;
  }
  .star:hover svg {
    transform: scale(1.12);
    stroke: rgba(255, 255, 255, 0.9);
  }
  .star.on svg {
    fill: var(--amber);
    stroke: #8a5d0c;
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
