<script lang="ts">
  // FlightDeck's own emblem: a compass rose inside a ring of laurel, with an
  // aircraft crossing it. Used on the cover in gold and as a watermark.
  let { size = 120, color = "currentColor" }: { size?: number; color?: string } = $props();
  const leaves = Array.from({ length: 11 }, (_, i) => i);
</script>

<svg viewBox="-100 -100 200 200" width={size} height={size} aria-hidden="true" fill={color} stroke={color}>
  <circle r="62" fill="none" stroke-width="3" />
  <circle r="54" fill="none" stroke-width="1.2" />
  {#each [0, 45, 90, 135, 180, 225, 270, 315] as a (a)}
    <path d="M0 -50 L6 -8 L0 0 L-6 -8 Z" transform="rotate({a})" stroke="none" opacity={a % 90 === 0 ? 1 : 0.65} />
  {/each}
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
  <g transform="rotate(-35) translate(0 0) scale(1.5)" stroke="none">
    <path d="M0 -26 L3.6 -8 L22 1 L22 6 L3.6 2.6 L2.4 15 L8.5 19.5 L8.5 23 L0 21 L-8.5 23 L-8.5 19.5 L-2.4 15 L-3.6 2.6 L-22 6 L-22 1 L-3.6 -8 Z" />
  </g>
</svg>
