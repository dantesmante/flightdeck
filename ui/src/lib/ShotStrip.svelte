<script lang="ts">
  // A flight's screenshots as a row of small pictures; one click opens the
  // viewer. Shown only when the flight has some.
  import { shots, thumbUrl, type PlacedShot } from "./shots.svelte";

  let { list, title = "Screenshots" }: { list: PlacedShot[]; title?: string } = $props();
  let broken = $state(new Set<number>());
</script>

{#if list.length}
  <section class="strip" aria-label={title}>
    <h4>{title} <span>{list.length}</span></h4>
    <div class="row">
      {#each list as s, i (s.id)}
        <button class="pic" onclick={() => (shots.viewer = { list, index: i })} title={s.path}>
          {#if broken.has(s.id)}
            <span class="gone">Moved or deleted</span>
          {:else}
            <img src={thumbUrl(s.id)} alt="Screenshot {i + 1}" loading="lazy" decoding="async" onerror={() => (broken = new Set([...broken, s.id]))} />
          {/if}
        </button>
      {/each}
    </div>
  </section>
{/if}

<style>
  .strip {
    margin-top: 14px;
  }
  h4 {
    margin: 0 0 8px;
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-dim);
  }
  h4 span {
    margin-left: 4px;
    color: var(--amber);
  }
  .row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }
  .pic {
    aspect-ratio: 16 / 9;
    padding: 0;
    border: 1px solid var(--rule);
    border-radius: 6px;
    overflow: hidden;
    background: #111214;
    cursor: zoom-in;
  }
  .pic:hover {
    border-color: var(--amber);
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
    padding: 4px;
    font-size: 10.5px;
    color: var(--ink-faint);
    text-align: center;
  }
</style>
