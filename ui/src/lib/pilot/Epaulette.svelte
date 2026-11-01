<script lang="ts">
  // A pilot's shoulder board for a rank: thin or full gold stripes at the
  // shoulder end, wings or an oak leaf in the middle, a button at the tip.
  import type { Insignia } from "../api";
  import Wings from "./Wings.svelte";

  let { insignia, width = 220, dim = false }: { insignia: Insignia; width?: number; dim?: boolean } = $props();
  const uid = $props.id();

  const METAL: Record<string, [string, string, string]> = {
    gold: ["#fbe39a", "#e2b04a", "#9c6b1c"],
    silver: ["#ffffff", "#c9d0d8", "#7d8792"],
    bronze: ["#f2c08d", "#c47f45", "#7a4520"],
  };
  const wing = $derived(insignia.wings ? METAL[insignia.wings] : null);

  // Stripes from the shoulder end: full 9 wide, thin 4, 5 apart.
  const stripes = $derived.by(() => {
    const w = insignia.thin ? 4 : 9;
    return Array.from({ length: insignia.stripes }, (_, i) => ({ x: 16 + i * (w + 6), w }));
  });
  const middle = $derived(stripes.length ? 16 + stripes.length * ((insignia.thin ? 4 : 9) + 6) + 50 : 104);

  const OAK =
    "M0 -12 C3 -10 6 -10 5 -7 C9 -7 10 -4 7 -2 C10 0 9 3 6 3 C7 6 4 8 2 6 L0 12 L-2 6 C-4 8 -7 6 -6 3 C-9 3 -10 0 -7 -2 C-10 -4 -9 -7 -5 -7 C-6 -10 -3 -10 0 -12 Z";
</script>

<svg viewBox="0 0 220 64" {width} height={(width * 64) / 220} class:dim role="img" aria-label="Shoulder board">
  <defs>
    <linearGradient id="{uid}-cloth" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#28324a" />
      <stop offset="0.5" stop-color="#18202f" />
      <stop offset="1" stop-color="#0f141e" />
    </linearGradient>
    <linearGradient id="{uid}-gold" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color={METAL.gold[0]} />
      <stop offset="0.45" stop-color={METAL.gold[1]} />
      <stop offset="1" stop-color={METAL.gold[2]} />
    </linearGradient>
    <clipPath id="{uid}-clip"><path d="M8 4 H168 L214 32 L168 60 H8 Q3 60 3 55 V9 Q3 4 8 4 Z" /></clipPath>
  </defs>
  <path d="M8 4 H168 L214 32 L168 60 H8 Q3 60 3 55 V9 Q3 4 8 4 Z" fill="url(#{uid}-cloth)" stroke="#3a455c" stroke-width="1.2" />
  <g clip-path="url(#{uid}-clip)">
    <!-- Stitching just inside the edge. -->
    <path d="M10 8.5 H166 L207 32 L166 55.5 H10" fill="none" stroke="#3b4762" stroke-width="0.8" stroke-dasharray="2.5 2" />
    {#each stripes as s, i (i)}
      <rect x={s.x} y="4" width={s.w} height="56" fill="url(#{uid}-gold)" />
      <rect x={s.x} y="4" width="1" height="56" fill="#fff6d0" opacity="0.5" />
    {/each}
  </g>
  {#if wing}
    <g transform="translate({middle} 32) scale(0.72)">
      <Wings metal={wing} id="{uid}-w" />
    </g>
  {:else if insignia.oak_leaf}
    <g transform="translate({middle} 32)" fill="url(#{uid}-gold)" stroke="#9c6b1c" stroke-width="0.6" stroke-linejoin="round">
      <path d={OAK} transform="rotate(-38) translate(0 -4) scale(1.25)" />
      <path d={OAK} transform="rotate(38) translate(0 -4) scale(1.25)" />
      <path d={OAK} transform="translate(0 -6) scale(1.35)" />
      <ellipse cy="13" rx="4.2" ry="5" />
      <path d="M-4.6 11 Q0 7.5 4.6 11 L4.2 12.6 Q0 10 -4.2 12.6 Z" fill="#7a5418" stroke="none" />
    </g>
  {/if}
  <!-- The button at the collar end. -->
  <circle cx="190" cy="32" r="7" fill="url(#{uid}-gold)" stroke="#8a5c16" stroke-width="0.8" />
  <circle cx="188" cy="30" r="2.2" fill="#fff6d0" opacity="0.6" />
</svg>

<style>
  svg {
    display: block;
    filter: drop-shadow(0 4px 10px rgba(0, 0, 0, 0.45));
  }
  .dim {
    opacity: 0.38;
    filter: grayscale(0.6);
  }
</style>
