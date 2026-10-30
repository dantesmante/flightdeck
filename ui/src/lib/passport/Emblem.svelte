<script lang="ts">
  // FlightDeck's own emblem: the origami bird logo inside a ring of laurel.
  // Used on the cover in gold and as a watermark.
  import { BIRD } from "../Logo.svelte";
  let { size = 120, color = "currentColor" }: { size?: number; color?: string } = $props();
  const leaves = Array.from({ length: 11 }, (_, i) => i);
</script>

<svg viewBox="-100 -100 200 200" width={size} height={size} aria-hidden="true" fill={color} stroke={color}>
  <circle r="62" fill="none" stroke-width="3" />
  <circle r="54" fill="none" stroke-width="1.2" />
  {#each [-1, 1] as side (side)}
    {#each leaves as i (i)}
      <ellipse
        cx="0"
        cy="-80"
        rx="5"
        ry="11"
        stroke="none"
        transform="rotate({side * (32 + i * 12)}) rotate({side * 30} 0 -80)"
      />
    {/each}
  {/each}
  <g transform="translate(-52.9 -37.1) scale(0.056)" stroke="none">
    <path d={BIRD.back} />
    <path d={BIRD.wing} />
    <path d={BIRD.fold} />
    <path d={BIRD.head + BIRD.eye} fill-rule="evenodd" />
    <path d={BIRD.beak} />
    <path d={BIRD.tail} />
  </g>
</svg>
