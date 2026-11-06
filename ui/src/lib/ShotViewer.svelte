<script lang="ts">
  // A screenshot full size over the app, with where and when it was taken,
  // and the others of the flight a key or click away.
  import { deleteShot, shotUrl, shots } from "./shots.svelte";
  import { inRemote, session } from "./remote.svelte";

  const v = $derived(shots.viewer);
  const cur = $derived(v ? v.list[v.index] : null);
  let broken = $state(false);
  /** Asking how to delete the picture shown. */
  let asking = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  $effect(() => {
    void cur;
    broken = false;
    asking = false;
    error = null;
  });
  /** A device allowed to make changes, or the PC itself. */
  const canDelete = $derived(!inRemote || session.canEdit);
  async function remove(fileToo: boolean) {
    if (!cur) return;
    busy = true;
    error = null;
    try {
      await deleteShot(cur.id, fileToo);
      asking = false;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function step(d: number) {
    if (!v) return;
    shots.viewer = { list: v.list, index: (v.index + d + v.list.length) % v.list.length };
  }
  function key(e: KeyboardEvent) {
    if (!v) return;
    if (e.key === "Escape") {
      if (asking) asking = false;
      else shots.viewer = null;
    } else if (asking) return;
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
        {#if canDelete}
          <span class="acts">
            <button class="del" onclick={() => (asking = true)}>
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 7h14M10 7V4h4v3M7 7l1 13h8l1-13M10 11v6M14 11v6" /></svg>Delete
            </button>
            <button class="close" onclick={() => (shots.viewer = null)}>Close</button>
          </span>
        {/if}
      </figcaption>
    </figure>
      {#if asking}
        <div class="ask" role="alertdialog" aria-label="Delete this screenshot">
          <b>Delete this screenshot?</b>
          <span>Moving it to the Recycle Bin removes the picture from your PC (you can restore it from there). Or FlightDeck can just stop showing it, leaving the file where it is.</span>
          {#if error}<em>{error}</em>{/if}
          <div class="row">
            <button class="danger" disabled={busy} onclick={() => remove(true)}>Move to Recycle Bin</button>
            <button disabled={busy} onclick={() => remove(false)}>Remove from FlightDeck only</button>
            <button disabled={busy} onclick={() => (asking = false)}>Cancel</button>
          </div>
        </div>
      {/if}
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
    align-items: center;
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
  .acts {
    display: flex;
    gap: 6px;
    margin-left: auto;
    pointer-events: auto;
  }
  .acts button,
  .ask button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 7px;
    border: 1px solid rgba(255, 255, 255, 0.18);
    background: rgba(255, 255, 255, 0.06);
    color: var(--ink);
    font-size: 13px;
  }
  .acts button:hover,
  .ask button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.14);
  }
  .acts .del:hover {
    border-color: var(--bad);
    color: #ffb4a8;
  }
  .acts svg {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .ask {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    display: grid;
    gap: 10px;
    width: min(460px, calc(100% - 24px));
    padding: 18px;
    border-radius: 12px;
    background: var(--panel);
    border: 1px solid var(--rule);
    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
    pointer-events: auto;
    color: var(--ink-dim);
    font-size: 13.5px;
  }
  .ask b {
    color: var(--ink);
    font-size: 16px;
  }
  .ask em {
    font-style: normal;
    color: var(--bad);
  }
  .ask .row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .ask .danger {
    background: var(--bad);
    border-color: var(--bad);
    color: #fff;
    font-weight: 600;
  }
  .ask button:disabled {
    opacity: 0.5;
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
