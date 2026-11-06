<svelte:options namespace="svg" />

<script lang="ts">
  // Pilot wings, as on a cast metal badge: a plain polished disc with a
  // wing either side. Each wing is a curved shoulder of short feathers over
  // the disc and five rows of long feathers fanning outwards, the top row
  // longest, every feather engraved with fine barbs. Drawn about 124 x 34,
  // centered on 0,0, for use inside an SVG.
  let { metal, id }: { metal: [string, string, string]; id: string } = $props();

  /** One row of long feathers (right wing): from behind the disc out to a
   * swept-up point, as a band with a scalloped lower edge. */
  function row(i: number): string {
    const y = -9.6 + i * 4.3; // top edge at the disc
    const t = 4.6; // thickness
    const tip = 62 - i * 7.2;
    const lift = 4.2 - i * 0.7; // how far the tip sweeps up
    const tipY = y - lift + 0.8;
    // Lower edge: three feather ends, each a shallow scallop.
    const n = 3;
    let lower = "";
    for (let k = n; k >= 1; k--) {
      const x0 = 12 + ((tip - 16) * k) / n;
      const x1 = 12 + ((tip - 16) * (k - 1)) / n;
      const yy0 = y + t - (lift * k) / n;
      const yy1 = y + t - (lift * (k - 1)) / n;
      lower += ` Q${(x0 + x1) / 2 + 1.5} ${(yy0 + yy1) / 2 + 1.4} ${x1} ${yy1}`;
    }
    return `M10 ${y} C${tip * 0.45} ${y - lift * 0.4}, ${tip - 8} ${tipY - 1.2}, ${tip} ${tipY} Q${tip - 2} ${tipY + 2.6} ${tip - 4} ${y + t - lift}${lower} L10 ${y + t} Z`;
  }
  const ROWS = Array.from({ length: 5 }, (_, i) => row(i));
  // The quill along each row, for a highlight.
  const QUILLS = Array.from({ length: 5 }, (_, i) => {
    const y = -9.6 + i * 4.3 + 2.1;
    const tip = 62 - i * 7.2;
    const lift = 4.2 - i * 0.7;
    return `M16 ${y} C${tip * 0.45} ${y - lift * 0.45}, ${tip - 10} ${y - lift + 0.4}, ${tip - 3} ${y - lift + 0.6}`;
  });
  // The shoulder: short feathers arching up from the disc.
  const SHOULDER = "M6 -13.6 C10 -18.4, 20 -18.2, 30 -14.2 C33 -13, 34 -11.4, 33 -10.4 C25 -12.6, 15 -12.4, 8 -9.6 Z";
  const SHOULDER_LINES = [12, 16, 20, 24, 28];
</script>

<defs>
  <linearGradient id="{id}-metal" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0" stop-color={metal[0]} />
    <stop offset="0.5" stop-color={metal[1]} />
    <stop offset="1" stop-color={metal[2]} />
  </linearGradient>
  <!-- Polished disc: bright where the light falls, darker round the rim. -->
  <radialGradient id="{id}-disc" cx="0.4" cy="0.32" r="0.75">
    <stop offset="0" stop-color="#ffffff" stop-opacity="0.95" />
    <stop offset="0.35" stop-color={metal[0]} />
    <stop offset="0.8" stop-color={metal[1]} />
    <stop offset="1" stop-color={metal[2]} />
  </radialGradient>
  <!-- Engraved barbs: fine slanted lines across the feathers. -->
  <pattern id="{id}-barbs" width="1.5" height="4" patternUnits="userSpaceOnUse" patternTransform="rotate(-58)">
    <line x1="0.4" y1="0" x2="0.4" y2="4" stroke={metal[2]} stroke-width="0.42" stroke-opacity="0.75" />
  </pattern>
</defs>

{#each [1, -1] as side (side)}
  <g transform="scale({side} 1)" stroke={metal[2]} stroke-width="0.5" stroke-linejoin="round">
    <!-- Bottom row first, so each row lies over the one below. -->
    {#each [...ROWS].reverse() as d, i (i)}
      <path {d} fill="url(#{id}-metal)" />
      <path {d} fill="url(#{id}-barbs)" stroke="none" />
    {/each}
    {#each QUILLS as d, i (i)}
      <path {d} fill="none" stroke={metal[0]} stroke-width="0.5" opacity="0.8" />
    {/each}
    <path d={SHOULDER} fill="url(#{id}-metal)" />
    <path d={SHOULDER} fill="url(#{id}-barbs)" stroke="none" />
    {#each SHOULDER_LINES as x (x)}
      <path d="M{x} {-17.4 + Math.abs(x - 19) * 0.12} Q{x + 1.6} {-14.4} {x - 1} {-11.6 + (x - 8) * 0.02}" fill="none" stroke-width="0.45" />
    {/each}
    <path d="M8 -14.6 C13 -17.6, 21 -17.4, 29 -14.6" fill="none" stroke={metal[0]} stroke-width="0.5" opacity="0.85" />
  </g>
{/each}
<!-- The disc: plain and polished. -->
<circle r="15.2" fill="url(#{id}-disc)" stroke={metal[2]} stroke-width="0.7" />
<circle r="14.4" fill="none" stroke="#ffffff" stroke-width="0.5" opacity="0.35" />
