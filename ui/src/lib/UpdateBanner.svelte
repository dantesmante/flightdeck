<script lang="ts">
  import { installUpdate, updates } from "./updates.svelte";
</script>

{#if updates.info?.available && !updates.dismissed}
  <div class="banner" role="status">
    <span>
      {#if updates.installing}
        Downloading FlightDeck {updates.info.latest}… FlightDeck closes, installs the update and opens again by itself. Your flights are kept.
      {:else}
        FlightDeck {updates.info.latest} is ready to install.
      {/if}
    </span>
    {#if !updates.installing}
      <button class="primary" onclick={installUpdate}>Update now</button>
      <button class="ghost" onclick={() => (updates.dismissed = true)}>Later</button>
    {/if}
    {#if updates.error}<span class="err">{updates.error}</span>{/if}
  </div>
{/if}

<style>
  .banner {
    position: absolute;
    z-index: 40;
    top: 12px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px 8px 16px;
    border-radius: 8px;
    background: var(--panel-2);
    border: 1px solid var(--amber);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
    max-width: 90%;
  }
  .primary {
    border: none;
    background: var(--amber);
    color: #1b1406;
    border-radius: 6px;
    padding: 6px 12px;
    font-weight: 600;
  }
  .ghost {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    border-radius: 6px;
    padding: 6px 10px;
    font-weight: 600;
  }
  .err {
    color: var(--bad);
  }
</style>
