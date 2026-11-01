<script lang="ts">
  // A score out of 100 as a ring that fills.
  let { value, label, size = 92 }: { value: number | null; label: string; size?: number } = $props();
  const R = 40;
  const C = 2 * Math.PI * R;
  const tone = $derived(value == null ? "dim" : value >= 85 ? "good" : value >= 65 ? "warn" : "bad");
</script>

<figure class="ring {tone}" style="width:{size}px">
  <svg viewBox="0 0 100 100" width={size} height={size} aria-hidden="true">
    <circle cx="50" cy="50" r={R} class="track" />
    {#if value != null}
      <circle cx="50" cy="50" r={R} class="fill" stroke-dasharray="{(C * Math.max(0, Math.min(100, value))) / 100} {C}" />
    {/if}
  </svg>
  <span class="num value">{value == null ? "–" : Math.round(value)}</span>
  <figcaption>{label}</figcaption>
</figure>

<style>
  .ring {
    position: relative;
    margin: 0;
    display: grid;
    justify-items: center;
    gap: 6px;
  }
  svg {
    transform: rotate(-90deg);
  }
  circle {
    fill: none;
    stroke-width: 7;
  }
  .track {
    stroke: var(--panel-3);
  }
  .fill {
    stroke-linecap: round;
    transition: stroke-dasharray 0.6s ease;
  }
  .good .fill {
    stroke: var(--good);
  }
  .warn .fill {
    stroke: var(--amber);
  }
  .bad .fill {
    stroke: var(--bad);
  }
  .value {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: var(--h, auto);
    aspect-ratio: 1;
    display: grid;
    place-items: center;
    font-size: 28px;
    font-weight: 600;
    color: var(--ink);
  }
  figcaption {
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-dim);
    text-align: center;
  }
</style>
