<script lang="ts" module>
  // The FlightDeck logo: an origami bird, traced from the 2000×1400
  // artwork (the app icon, src-tauri/icons/logo.svg, uses the same shapes). The eye is a hole in
  // the head, so the one-color version shows the background through it.
  export const BIRD = {
    back: "M540 100 L972 492 L912 545 L668 395 Z",
    fold: "M263 249 L650 416 L912 578 L1312 910 Z",
    wing: "M263 249 L1312 910 L737 733 Z",
    head: "M933 562 L1462 100 L1717 140 L1327 892 Z",
    eye: "M1580 193 a27 27 0 1 0 54 0 a27 27 0 1 0 -54 0 Z",
    beak: "M1734 160 L1795 642 L1622 375 Z",
    tail: "M712 755 L1295 933 L95 1297 Z",
  };
</script>

<script lang="ts">
  /** Full-color logo, or one color when `color` is given. */
  let { size = 26, color }: { size?: number; color?: string } = $props();
  const id = `fdl${Math.random().toString(36).slice(2, 8)}`;
  const light = $derived(color ?? `url(#${id}l)`);
  const dark = $derived(color ?? `url(#${id}d)`);
</script>

<svg viewBox="60 60 1780 1280" width={size} height={size * (1280 / 1780)} aria-hidden="true">
  {#if !color}
    <defs>
      <linearGradient id="{id}l" gradientUnits="userSpaceOnUse" x1="95" y1="0" x2="1795" y2="0">
        <stop offset="0" stop-color="#e46f6a" />
        <stop offset="0.36" stop-color="#e8855a" />
        <stop offset="0.6" stop-color="#eb954f" />
        <stop offset="1" stop-color="#f0b03c" />
      </linearGradient>
      <linearGradient id="{id}d" gradientUnits="userSpaceOnUse" x1="263" y1="0" x2="1312" y2="0">
        <stop offset="0" stop-color="#df534b" />
        <stop offset="1" stop-color="#e57733" />
      </linearGradient>
    </defs>
  {/if}
  <path d={BIRD.back} fill={dark} />
  <path d={BIRD.wing} fill={dark} />
  <path d={BIRD.fold} fill={light} />
  <path d={BIRD.head + BIRD.eye} fill={light} fill-rule="evenodd" />
  {#if !color}<path d={BIRD.eye} fill="#000" />{/if}
  <path d={BIRD.beak} fill={light} />
  <path d={BIRD.tail} fill={light} />
</svg>
