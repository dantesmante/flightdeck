<script lang="ts">
  // The Photos tab's card: every screenshot FlightDeck knows, two to a row,
  // grouped by flight, newest first. Pointing at one brings out its pin on
  // the map; clicking opens it full size.
  import type { Photo } from "./api";
  import { shots, thumbUrl, type PlacedShot } from "./shots.svelte";

  let {
    list,
    hot = null,
    loading = false,
    hasFolder = true,
    onhover,
    onsettings,
  }: {
    list: Photo[];
    /** A screenshot whose pin is under the pointer. */
    hot?: number | null;
    loading?: boolean;
    hasFolder?: boolean;
    onhover: (id: number | null) => void;
    onsettings: () => void;
  } = $props();

  const placed = $derived<PlacedShot[]>(list.map((p) => ({ id: p.id, path: p.path, taken_ms: p.taken_ms, lat: p.lat, lon: p.lon, alt: p.alt_ft })));
  /** One group per flight, in the order of the list (newest first). */
  const groups = $derived.by(() => {
    const out: { key: string; title: string; when: number; items: { p: Photo; i: number }[] }[] = [];
    list.forEach((p, i) => {
      if (p.flight_id == null) return;
      const key = String(p.flight_id);
      let g = out.find((x) => x.key === key);
      if (!g) {
        g = { key, title: p.dep || p.arr ? `${p.dep ?? "?"} → ${p.arr ?? "?"}` : "Flight", when: p.taken_ms, items: [] };
        out.push(g);
      }
      g.items.push({ p, i });
    });
    return out;
  });
  const loose = $derived(list.flatMap((p, i) => (p.flight_id == null ? [{ p, i }] : [])));
  const inFlights = $derived(list.length - loose.length);

  // The two sections fold away; how they were is remembered on this device.
  const FOLD_KEY = "fd.photosFolded";
  let folded = $state<{ loose: boolean; past: boolean }>(
    (() => {
      try {
        return { loose: false, past: false, ...JSON.parse(localStorage.getItem(FOLD_KEY) ?? "{}") };
      } catch {
        return { loose: false, past: false };
      }
    })(),
  );
  function fold(k: "loose" | "past") {
    folded = { ...folded, [k]: !folded[k] };
    try {
      localStorage.setItem(FOLD_KEY, JSON.stringify(folded));
    } catch {
      /* not remembered */
    }
  }

  // A pin pointed at on the map: its picture shows and stands out.
  let thumbs = new Map<number, HTMLElement>();
  function track(el: HTMLElement, id: number) {
    thumbs.set(id, el);
    return { destroy: () => thumbs.delete(id) };
  }
  $effect(() => {
    if (hot != null) thumbs.get(hot)?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  });
  let broken = $state(new Set<number>());
  const day = (ms: number) => new Date(ms).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
</script>

{#snippet pic(p: Photo, i: number)}
  <button
    class="pic"
    class:hot={hot === p.id}
    title={p.path}
    use:track={p.id}
    onmouseenter={() => onhover(p.id)}
    onmouseleave={() => onhover(null)}
    onfocus={() => onhover(p.id)}
    onblur={() => onhover(null)}
    onclick={() => (shots.viewer = { list: placed, index: i })}
  >
    {#if broken.has(p.id)}
      <span class="gone">Moved or deleted</span>
    {:else}
      <img src={thumbUrl(p.id)} alt="Screenshot" loading="lazy" decoding="async" onerror={() => (broken = new Set([...broken, p.id]))} />
    {/if}
    {#if p.lat == null}<span class="nomap" title="Not taken during a recorded flight, so not on the map">No place</span>{/if}
  </button>
{/snippet}

<header>
  <h2>Photos</h2>
  {#if list.length}<span class="count num">{list.length}</span>{/if}
</header>

{#if loading && !list.length}
  <p class="empty">Loading your screenshots…</p>
{:else if !list.length}
  <div class="empty">
    {#if hasFolder}
      <p>No screenshots yet. Take one in the sim during a flight and it shows up here, and on the map where you took it.</p>
    {:else}
      <p>Choose the folder your screenshots are saved to, and the ones you take during flights show up here, and on the map where you took them.</p>
      <button class="link" onclick={onsettings}>Choose a folder in Settings</button>
    {/if}
  </div>
{:else}
  {#if loose.length}
    <section class="sect">
      <button class="sect-head" aria-expanded={!folded.loose} onclick={() => fold("loose")}>
        <svg viewBox="0 0 24 24" aria-hidden="true" class:shut={folded.loose}><path d="M6 9l6 6 6-6" /></svg>
        Not in a flight <span class="n num">{loose.length}</span>
      </button>
      {#if !folded.loose}
        <div class="grid">
          {#each loose as { p, i } (p.id)}{@render pic(p, i)}{/each}
        </div>
      {/if}
    </section>
  {/if}
  {#if groups.length}
    <section class="sect">
      <button class="sect-head" aria-expanded={!folded.past} onclick={() => fold("past")}>
        <svg viewBox="0 0 24 24" aria-hidden="true" class:shut={folded.past}><path d="M6 9l6 6 6-6" /></svg>
        Past flights <span class="n num">{inFlights}</span>
      </button>
      {#if !folded.past}
        {#each groups as g (g.key)}
          <section class="group">
            <h3>
              <span class="num">{g.title}</span>
              <span class="when">{day(g.when)}</span>
            </h3>
            <div class="grid">
              {#each g.items as { p, i } (p.id)}{@render pic(p, i)}{/each}
            </div>
          </section>
        {/each}
      {/if}
    </section>
  {/if}
{/if}

<style>
  header {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin: 0 30px 12px 0;
  }
  h2 {
    margin: 0;
    font-family: var(--cond);
    font-size: 24px;
    font-weight: 600;
  }
  .count {
    color: var(--amber);
    font-size: 15px;
  }
  .empty {
    margin: 4px 0 0;
    color: var(--ink-dim);
    font-size: 14px;
  }
  .empty p {
    margin: 0 0 10px;
  }
  .link {
    padding: 7px 12px;
    border-radius: 8px;
    border: 1px solid var(--amber);
    background: transparent;
    color: var(--amber);
    font-size: 13px;
  }
  .sect + .sect {
    margin-top: 14px;
  }
  .sect-head {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    margin: 0 0 8px;
    padding: 6px 2px;
    border: none;
    border-bottom: 1px solid var(--rule);
    background: transparent;
    color: var(--ink);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    text-align: left;
  }
  .sect-head:hover {
    color: var(--amber);
  }
  .sect-head svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform 0.15s;
  }
  .sect-head svg.shut {
    transform: rotate(-90deg);
  }
  .sect-head .n {
    margin-left: auto;
    color: var(--amber);
    letter-spacing: 0;
  }
  .group + .group {
    margin-top: 14px;
  }
  h3 {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin: 0 0 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--ink);
  }
  h3 .when {
    font-weight: 400;
    font-size: 12px;
    color: var(--ink-faint);
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .pic {
    position: relative;
    aspect-ratio: 16 / 9;
    padding: 0;
    border: 1px solid var(--rule);
    border-radius: 8px;
    overflow: hidden;
    background: #111214;
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .pic.hot {
    border-color: var(--amber);
    box-shadow: 0 0 0 2px rgba(240, 178, 58, 0.45);
  }
  .pic:hover,
  .pic:focus-visible {
    border-color: var(--amber);
    outline: none;
  }
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .gone {
    display: grid;
    place-items: center;
    height: 100%;
    font-size: 11px;
    color: var(--ink-faint);
  }
  .nomap {
    position: absolute;
    left: 5px;
    bottom: 5px;
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(17, 18, 20, 0.8);
    color: var(--ink-dim);
    font-size: 10px;
  }
</style>
