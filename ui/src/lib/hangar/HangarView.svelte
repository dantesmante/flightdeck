<script lang="ts">
  import { onMount } from "svelte";
  import { api, type HangarItem } from "../api";
  import { day, gradeTone, hoursMinutes, int } from "../format";
  import { shrinkImage } from "../image";
  import { distance } from "../prefs.svelte";
  import AircraftArt from "./AircraftArt.svelte";

  let { onsettings }: { onsettings: () => void } = $props();

  let items = $state<HangarItem[]>([]);
  let loaded = $state(false);
  let photos = $state<Record<number, string>>({});
  let active = $state(0);
  let strip = $state<HTMLDivElement | null>(null);
  let message = $state<string | null>(null);

  onMount(async () => {
    items = (await api.hangar()) ?? [];
    loaded = true;
    for (const it of items.filter((i) => i.has_photo)) {
      const p = await api.aircraftPhoto(it.id);
      if (p) photos = { ...photos, [it.id]: p };
    }
  });

  const totalMinutes = $derived(items.reduce((a, i) => a + i.minutes, 0));

  function grade(v: number | null) {
    if (v == null) return null;
    const a = Math.abs(v);
    return a <= 60 ? "Butter" : a <= 180 ? "Smooth" : a <= 300 ? "Firm" : a <= 600 ? "Hard" : "Very hard";
  }

  function onScroll() {
    if (!strip) return;
    const mid = strip.scrollLeft + strip.clientWidth / 2;
    let best = 0;
    let bestD = Infinity;
    strip.querySelectorAll<HTMLElement>(".card").forEach((el, i) => {
      const d = Math.abs(el.offsetLeft + el.offsetWidth / 2 - mid);
      if (d < bestD) {
        bestD = d;
        best = i;
      }
    });
    active = best;
  }

  function show(i: number) {
    const el = strip?.querySelectorAll<HTMLElement>(".card")[i];
    el?.scrollIntoView({ behavior: "smooth", inline: "center", block: "nearest" });
  }

  function key(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    if (t?.closest("input, textarea, select")) return;
    if (e.key === "ArrowRight") show(Math.min(items.length - 1, active + 1));
    if (e.key === "ArrowLeft") show(Math.max(0, active - 1));
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

  async function useDrawing(it: HangarItem) {
    await api.setAircraftPhoto(it.id, null);
    const { [it.id]: _gone, ...rest } = photos;
    photos = rest;
    it.has_photo = false;
  }

  const place = (it: HangarItem) =>
    [it.last_region_name, it.last_country_name].filter((x, i, xs) => x && xs.indexOf(x) === i).join(", ");
</script>

<svelte:window onkeydown={key} />

<section class="hangar">
  <header>
    <h2>Hangar</h2>
    {#if items.length}
      <p>{items.length} {items.length === 1 ? "aircraft" : "aircraft"}, {hoursMinutes(totalMinutes)} flown in total</p>
    {/if}
    {#if items.length > 1}
      <span class="nav">
        <button class="ghost" onclick={() => show(Math.max(0, active - 1))} disabled={active === 0} aria-label="Previous aircraft">Previous</button>
        <button class="ghost" onclick={() => show(Math.min(items.length - 1, active + 1))} disabled={active >= items.length - 1} aria-label="Next aircraft">Next</button>
      </span>
    {/if}
  </header>

  {#if loaded && items.length === 0}
    <div class="empty">
      <p>Your hangar is empty. Every aircraft you fly parks here, with its hours, distance and where you left it.</p>
      <button class="primary" onclick={onsettings}>Add sample flights</button>
    </div>
  {:else}
    <div class="strip" bind:this={strip} onscroll={onScroll}>
      {#each items as it, i (it.id)}
        {@const d = distance(it.distance_nm)}
        {@const best = grade(it.best_landing_fpm)}
        <article class="card" class:active={i === active} aria-label={it.title || it.icao_type}>
          <div class="picture">
            {#if photos[it.id]}
              <img src={photos[it.id]} alt={it.registration || it.title} />
            {:else}
              <AircraftArt
                icao_type={it.icao_type}
                engine_type={it.engine_type}
                engine_count={it.engine_count}
                title={it.title}
                registration={it.registration}
                tone={it.id}
              />
            {/if}
            <span class="reg num">{it.registration || it.icao_type || "Unregistered"}</span>
          </div>
          <div class="info">
            <div class="name">
              <h3>{it.title || it.icao_type || "Unknown aircraft"}</h3>
              {#if it.icao_type}<span class="type num">{it.icao_type}</span>{/if}
            </div>
            <dl>
              <div><dt>Flight time</dt><dd class="num">{hoursMinutes(it.minutes)}</dd></div>
              <div><dt>Distance</dt><dd class="num">{d.value} <small>{d.unit}</small></dd></div>
              <div><dt>Flights</dt><dd class="num">{it.flights}</dd></div>
              <div><dt>Landings</dt><dd class="num">{it.landings}</dd></div>
              <div>
                <dt>Best landing</dt>
                <dd class="num {gradeTone(best)}">{it.best_landing_fpm != null ? `${int(it.best_landing_fpm)} fpm` : "–"}</dd>
              </div>
            </dl>
            <p class="where">
              {#if it.last_icao}
                Last parked at <strong>{it.last_icao}</strong>{it.last_name ? `, ${it.last_name}` : ""}{place(it) ? `, ${place(it)}` : ""}.
                <span class="when">{day(it.last_flight_ms)}</span>
              {:else}
                Last position unknown.
              {/if}
            </p>
            <div class="actions">
              <label class="ghost file">
                {photos[it.id] ? "Change photo" : "Use my photo"}
                <input type="file" accept="image/*" onchange={(e) => pick(it, e)} />
              </label>
              {#if photos[it.id]}
                <button class="ghost" onclick={() => useDrawing(it)}>Use the drawing</button>
              {/if}
            </div>
          </div>
        </article>
      {/each}
    </div>
    {#if items.length > 1}
      <div class="dots" role="tablist" aria-label="Aircraft">
        {#each items as it, i (it.id)}
          <button class:on={i === active} role="tab" aria-selected={i === active} aria-label={it.registration || it.title} onclick={() => show(i)}></button>
        {/each}
      </div>
    {/if}
  {/if}
  {#if message}<p class="msg" role="status">{message}</p>{/if}
</section>

<style>
  .hangar {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    /* Hangar floor and roof lights */
    background:
      radial-gradient(ellipse 40% 30% at 25% 0%, rgba(255, 244, 214, 0.08), transparent 70%),
      radial-gradient(ellipse 40% 30% at 75% 0%, rgba(255, 244, 214, 0.08), transparent 70%),
      linear-gradient(180deg, #15222c 0%, #121c24 62%, #0e161c 62%, #172530 100%);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 14px;
    padding: 18px 24px 0;
  }
  h2 {
    margin: 0;
    font-family: var(--cond);
    font-weight: 600;
    font-size: 26px;
  }
  header p {
    margin: 0;
    color: var(--ink-dim);
  }
  .nav {
    margin-left: auto;
    display: flex;
    gap: 8px;
  }
  .strip {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    gap: 28px;
    padding: 10px calc(50% - 280px);
    overflow-x: auto;
    scroll-snap-type: x mandatory;
    scrollbar-width: none;
  }
  .strip::-webkit-scrollbar {
    display: none;
  }
  .card {
    flex: none;
    width: 560px;
    scroll-snap-align: center;
    background: var(--panel-2);
    border: 1px solid var(--rule);
    border-radius: 10px;
    overflow: hidden;
    transform: scale(0.92);
    opacity: 0.6;
    transition: transform 0.35s ease, opacity 0.35s ease;
    box-shadow: 0 24px 50px rgba(0, 0, 0, 0.45);
  }
  .card.active {
    transform: scale(1);
    opacity: 1;
  }
  .picture {
    position: relative;
    aspect-ratio: 16 / 9;
    background: #0d1820;
  }
  .picture img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .reg {
    position: absolute;
    left: 12px;
    top: 10px;
    padding: 2px 9px;
    border-radius: 4px;
    background: rgba(13, 24, 32, 0.72);
    color: #fff;
    font-size: 20px;
    font-weight: 600;
    letter-spacing: 0.04em;
  }
  .info {
    padding: 14px 18px 16px;
  }
  .name {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  h3 {
    margin: 0;
    font-size: 19px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .type {
    font-size: 15px;
    color: var(--amber);
    border: 1px solid var(--amber);
    border-radius: 4px;
    padding: 0 6px;
  }
  dl {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 8px;
    margin: 12px 0 0;
  }
  dt {
    font-size: 12px;
    color: var(--ink-dim);
  }
  dd {
    margin: 0;
    font-size: 20px;
    font-weight: 500;
  }
  dd small {
    font-size: 12px;
    color: var(--ink-faint);
  }
  dd.good {
    color: var(--good);
  }
  dd.warn {
    color: var(--warn);
  }
  dd.bad {
    color: var(--bad);
  }
  .where {
    margin: 12px 0 0;
    color: var(--ink-dim);
  }
  .where strong {
    color: var(--ink);
  }
  .when {
    display: block;
    font-size: 12px;
    color: var(--ink-faint);
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  .ghost {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    border-radius: 6px;
    padding: 6px 12px;
    font-weight: 600;
    font-size: 14px;
    cursor: pointer;
  }
  .ghost:hover:not(:disabled) {
    color: var(--ink);
  }
  .ghost:disabled {
    opacity: 0.4;
  }
  .file input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
  }
  .file:focus-within {
    outline: 2px solid var(--amber);
  }
  .dots {
    display: flex;
    justify-content: center;
    gap: 8px;
    padding: 4px 0 18px;
  }
  .dots button {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    border: none;
    padding: 0;
    background: var(--rule);
  }
  .dots button.on {
    background: var(--amber);
  }
  .empty {
    margin: auto;
    max-width: 40ch;
    text-align: center;
    color: var(--ink-dim);
  }
  .primary {
    border: none;
    background: var(--amber);
    color: #1b1406;
    border-radius: 6px;
    padding: 8px 14px;
    font-weight: 600;
  }
  .msg {
    position: absolute;
    bottom: 16px;
    left: 50%;
    transform: translateX(-50%);
    margin: 0;
    padding: 8px 14px;
    border-radius: 6px;
    background: var(--strip);
    color: var(--strip-ink);
    font-weight: 600;
  }
</style>
