<script lang="ts">
  // A screenshot full size over the app, with where and when it was taken,
  // and the others of the flight a key or click away.
  import { shotUrl, shots } from "./shots.svelte";

  const v = $derived(shots.viewer);
  const cur = $derived(v ? v.list[v.index] : null);
  let broken = $state(false);
  $effect(() => {
    void cur;
    broken = false;
  });

  function step(d: number) {
    if (!v) return;
    shots.viewer = { list: v.list, index: (v.index + d + v.list.length) % v.list.length };
  }
  function key(e: KeyboardEvent) {
    if (!v) return;
    if (e.key === "Escape") shots.viewer = null;
    else if (e.key === "ArrowRight") step(1);
    else if (e.key === "ArrowLeft") step(-1);
  }
  const zulu = (ms: number) => {
    const d = new Date(ms);
    return `${d.toISOString().slice(0, 10)} ${String(d.getUTCHours()).padStart(2, "0")}${String(d.getUTCMinutes()).padStart(2, "0")}Z`;
  };
  const alt = (ft: number | null) => (ft == null ? "" : ft >= 18000 ? `FL${String(Math.round(ft / 100)).padStart(3, "0")}` : `${Math.round(ft / 10) * 10} ft`);
</script>

<svelte:window onkeydown={key} />

{#if v && cur}
  <div class="viewer" role="dialog" aria-modal="true" aria-label="Screenshot">
    <button class="scrim" aria-label="Close" onclick={() => (shots.viewer = null)}></button>
    <figure>
      {#if broken}
        <div class="gone">
          <b>This picture is no longer where it was.</b>
          <span>It was moved, renamed or deleted on the PC. FlightDeck only links to your screenshots, so it cannot show it any more.</span>
          <code>{cur.path}</code>
        </div>
      {:else}
        <img src={shotUrl(cur.id)} alt="Screenshot" onerror={() => (broken = true)} />
      {/if}
      <figcaption>
        <span class="num">{zulu(cur.taken_ms)}</span>
        {#if cur.alt != null}<span class="num">{alt(cur.alt)}</span>{/if}
        {#if v.list.length > 1}<span class="num">{v.index + 1} of {v.list.length}</span>{/if}
        <span class="path" title={cur.path}>{cur.path}</span>
      </figcaption>
    </figure>
    {#if v.list.length > 1}
      <button class="nav prev" aria-label="Previous" onclick={() => step(-1)}><svg viewBox="0 0 24 24"><path d="M15 5l-7 7 7 7" /></svg></button>
      <button class="nav next" aria-label="Next" onclick={() => step(1)}><svg viewBox="0 0 24 24"><path d="M9 5l7 7-7 7" /></svg></button>
    {/if}
    <button class="x" aria-label="Close" onclick={() => (shots.viewer = null)}><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg></button>
  </div>
{/if}

<style>
  .viewer {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
    padding: 40px 70px;
  }
  .scrim {
    position: absolute;
    inset: 0;
    border: none;
    padding: 0;
    background: rgba(8, 9, 10, 0.88);
    cursor: zoom-out;
  }
  figure {
    position: relative;
    margin: 0;
    max-width: 100%;
    max-height: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }
  img {
    max-width: 100%;
    max-height: calc(100vh - 130px);
    object-fit: contain;
    border-radius: 6px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.6);
    pointer-events: auto;
  }
  figcaption {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    color: var(--ink-dim);
    font-size: 13px;
  }
  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ink-faint);
    font-size: 12px;
  }
  .gone {
    display: grid;
    gap: 8px;
    max-width: 520px;
    padding: 24px;
    border-radius: 10px;
    background: var(--panel);
    border: 1px solid var(--rule);
    color: var(--ink-dim);
  }
  .gone b {
    color: var(--ink);
  }
  .gone code {
    font-size: 12px;
    word-break: break-all;
    color: var(--ink-faint);
  }
  .nav,
  .x {
    position: absolute;
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }
  .nav:hover,
  .x:hover {
    background: rgba(255, 255, 255, 0.18);
  }
  .prev {
    left: 14px;
    top: 50%;
  }
  .next {
    right: 14px;
    top: 50%;
  }
  .x {
    top: 14px;
    right: 14px;
  }
  svg {
    width: 22px;
    height: 22px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  @media (max-width: 640px) {
    .viewer {
      padding: 60px 10px 20px;
    }
  }
</style>
