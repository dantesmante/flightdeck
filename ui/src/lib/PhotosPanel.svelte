<script lang="ts">
  // The Photos tab's card: every screenshot FlightDeck knows, two to a row,
  // grouped by flight, newest first. Pointing at one brings out its pin on
  // the map; clicking opens it full size.
  import type { Photo } from "./api";
  import { shots, thumbUrl, type PlacedShot } from "./shots.svelte";

  let {
    list,
    loading = false,
    hasFolder = true,
    onhover,
    onsettings,
  }: {
    list: Photo[];
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
      const key = p.flight_id == null ? "none" : String(p.flight_id);
      let g = out.find((x) => x.key === key);
      if (!g) {
        g = { key, title: p.flight_id == null ? "Not in a flight" : p.dep || p.arr ? `${p.dep ?? "?"} → ${p.arr ?? "?"}` : "Flight", when: p.taken_ms, items: [] };
        out.push(g);
      }
      g.items.push({ p, i });
    });
    return out;
  });
  let broken = $state(new Set<number>());
  const day = (ms: number) => new Date(ms).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
</script>

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
  {#each groups as g (g.key)}
    <section class="group">
      <h3>
        <span class="num">{g.title}</span>
        <span class="when">{day(g.when)}</span>
      </h3>
      <div class="grid">
        {#each g.items as { p, i } (p.id)}
          <button
            class="pic"
            title={p.path}
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
        {/each}
      </div>
    </section>
  {/each}
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
  .group + .group {
    margin-top: 16px;
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
    cursor: zoom-in;
    transition: border-color 0.15s;
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
