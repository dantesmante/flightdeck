<script lang="ts">
  // A pilot's shoulder board for a rank: woven navy cloth with a padded
  // edge, embroidered gold braid stripes at the shoulder end, wings or an
  // oak leaf in the middle, a button at the tip.
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
    <!-- Twill weave: fine diagonal threads across the cloth. -->
    <pattern id="{uid}-twill" width="3" height="3" patternUnits="userSpaceOnUse" patternTransform="rotate(-35)">
      <rect width="3" height="3" fill="none" />
      <line x1="0" y1="0.6" x2="3" y2="0.6" stroke="#ffffff" stroke-opacity="0.07" stroke-width="0.9" />
      <line x1="0" y1="2.1" x2="3" y2="2.1" stroke="#000000" stroke-opacity="0.22" stroke-width="0.8" />
    </pattern>
    <!-- Braid: woven gold threads across each stripe. -->
    <pattern id="{uid}-braid" width="4" height="2.4" patternUnits="userSpaceOnUse" patternTransform="rotate(28)">
      <line x1="0" y1="0.5" x2="4" y2="0.5" stroke="#fff3c4" stroke-opacity="0.45" stroke-width="0.6" />
      <line x1="0" y1="1.7" x2="4" y2="1.7" stroke="#6e4a12" stroke-opacity="0.45" stroke-width="0.7" />
    </pattern>
    <!-- Fabric: a soft, uneven fibre texture. -->
    <filter id="{uid}-fibre" x="0" y="0" width="100%" height="100%">
      <feTurbulence type="fractalNoise" baseFrequency="0.9 0.45" numOctaves="2" seed="4" />
      <feColorMatrix values="0 0 0 0 0.6  0 0 0 0 0.65  0 0 0 0 0.8  0 0 0 0.35 -0.08" />
    </filter>
    <!-- Padding: the board is lit on top, shaded at the edges. -->
    <linearGradient id="{uid}-pad" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.14" />
      <stop offset="0.18" stop-color="#ffffff" stop-opacity="0.03" />
      <stop offset="0.75" stop-color="#000000" stop-opacity="0" />
      <stop offset="1" stop-color="#000000" stop-opacity="0.35" />
    </linearGradient>
    <clipPath id="{uid}-clip"><path d="M8 4 H168 L214 32 L168 60 H8 Q3 60 3 55 V9 Q3 4 8 4 Z" /></clipPath>
  </defs>
  <path d="M8 4 H168 L214 32 L168 60 H8 Q3 60 3 55 V9 Q3 4 8 4 Z" fill="url(#{uid}-cloth)" />
  <g clip-path="url(#{uid}-clip)">
    <rect x="0" y="0" width="220" height="64" fill="url(#{uid}-twill)" />
    <rect x="0" y="0" width="220" height="64" filter="url(#{uid}-fibre)" />
    {#each stripes as s, i (i)}
      <rect x={s.x} y="4" width={s.w} height="56" fill="url(#{uid}-gold)" />
      <rect x={s.x} y="4" width={s.w} height="56" fill="url(#{uid}-braid)" />
      <!-- Sewn on: a dark seam either side of the braid. -->
      <rect x={s.x - 0.6} y="4" width="0.8" height="56" fill="#0a0d14" opacity="0.55" />
      <rect x={s.x + s.w - 0.2} y="4" width="0.8" height="56" fill="#0a0d14" opacity="0.55" />
    {/each}
    <path d="M8 4 H168 L214 32 L168 60 H8 Q3 60 3 55 V9 Q3 4 8 4 Z" fill="url(#{uid}-pad)" />
    <!-- Stitching just inside the edge, in thread a shade lighter than the cloth. -->
    <path d="M10 8.5 H166 L207 32 L166 55.5 H10" fill="none" stroke="#56648a" stroke-width="0.7" stroke-dasharray="2.2 1.6" stroke-linecap="round" />
  </g>
  <!-- The rolled edge. -->
  <path d="M8 4 H168 L214 32 L168 60 H8 Q3 60 3 55 V9 Q3 4 8 4 Z" fill="none" stroke="#0b0f18" stroke-width="2.2" />
  <path d="M8 5.2 H167.6 L212.6 32" fill="none" stroke="#4a5778" stroke-width="0.7" opacity="0.8" />
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
