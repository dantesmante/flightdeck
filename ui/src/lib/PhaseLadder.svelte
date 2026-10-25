<script lang="ts">
  import type { Phase } from "./api";
  import { PHASES } from "./format";

  let { phase }: { phase: Phase | null } = $props();
  const current = $derived(PHASES.findIndex((p) => p.id === phase));
</script>

<ol class="ladder" aria-label="Flight phase">
  {#each PHASES as p, i (p.id)}
    <li class:done={current >= 0 && i < current} class:now={i === current} aria-current={i === current ? "step" : undefined}>
      <span class="dot"></span>{p.label}
    </li>
  {/each}
</ol>

<style>
  .ladder {
    list-style: none;
    margin: 0;
    padding: 0;
    position: relative;
  }
  .ladder::before {
    content: "";
    position: absolute;
    left: 5px;
    top: 10px;
    bottom: 10px;
    width: 2px;
    background: var(--rule);
  }
  li {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 3px 0;
    font-size: 14px;
    color: var(--ink-faint);
  }
  .dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid var(--rule);
    background: var(--panel);
    z-index: 1;
    flex: none;
  }
  .done {
    color: var(--ink-dim);
  }
  .done .dot {
    background: var(--ink-dim);
    border-color: var(--ink-dim);
  }
  .now {
    color: var(--amber);
    font-weight: 600;
  }
  .now .dot {
    background: var(--amber);
    border-color: var(--amber);
    box-shadow: 0 0 0 5px var(--amber-glow);
  }
</style>
