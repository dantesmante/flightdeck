<svelte:options namespace="svg" />

<script lang="ts">
  // Pilot wings: a round matte badge with a wing either side, each a fan
  // of upright feathers by the badge and long swept feathers stacked
  // outwards. Drawn about 124 x 34, centered on 0,0, for use inside an SVG.
  let { metal, id }: { metal: [string, string, string]; id: string } = $props();

  // Long feathers, top to bottom: each shorter, its tip swept up.
  const FEATHERS = Array.from({ length: 5 }, (_, i) => {
    const top = -10.5 + i * 3.9;
    const h = 3.3;
    const end = 62 - i * 7.5;
    const lift = 3.4 - i * 0.55;
    return `M17 ${top} L${end - 5} ${top - lift + 0.6} Q${end + 1} ${top - lift} ${end} ${top - lift + 1.6} L${end - 3} ${top + h - lift + 0.4} L17 ${top + h} Z`;
  });
  // The fan of short upright feathers next to the badge.
  const FAN = "M12 -11.5 L28 -12.5 Q31 -2 27 9.5 L12 10.5 Z";
  const FAN_LINES = [15, 18, 21, 24, 26.6];
</script>

<defs>
  <linearGradient id="{id}-metal" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color={metal[0]} />
    <stop offset="0.45" stop-color={metal[1]} />
    <stop offset="1" stop-color={metal[2]} />
  </linearGradient>
  <radialGradient id="{id}-face" cx="0.45" cy="0.4" r="0.7">
    <stop offset="0" stop-color={metal[1]} />
    <stop offset="1" stop-color={metal[2]} />
  </radialGradient>
</defs>

{#each [1, -1] as side (side)}
  <g transform="scale({side} 1)" stroke={metal[2]} stroke-width="0.55" stroke-linejoin="round" fill="url(#{id}-metal)">
    {#each [...FEATHERS].reverse() as d, i (i)}
      <path {d} />
    {/each}
    <path d={FAN} />
    {#each FAN_LINES as x (x)}
      <path d="M{x} {-11.6 - (x - 12) * 0.06} Q{x + 1.2} 0 {x - 0.4} {10.4 - (x - 12) * 0.05}" fill="none" stroke-width="0.7" />
    {/each}
    <!-- A highlight along each long feather. -->
    {#each FEATHERS as _, i (i)}
      <path d="M19 {-9.9 + i * 3.9} L{50 - i * 7.5} {-9.9 + i * 3.9 - (3.4 - i * 0.55) + 0.9}" stroke={metal[0]} stroke-width="0.45" opacity="0.7" fill="none" />
    {/each}
  </g>
{/each}
<!-- The badge. -->
<circle r="15" fill="url(#{id}-metal)" stroke={metal[2]} stroke-width="0.8" />
<circle r="12.6" fill="url(#{id}-face)" stroke={metal[2]} stroke-width="0.6" />
<circle r="12.6" fill="none" stroke={metal[0]} stroke-width="0.5" opacity="0.6" transform="translate(-0.4 -0.4)" />
