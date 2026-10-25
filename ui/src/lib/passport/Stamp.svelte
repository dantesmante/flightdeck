<script lang="ts">
  // One rubber stamp, drawn as SVG from its seed: the shape, ink, tilt and
  // wear are random but stable, so a stamp always looks the same.
  import type { Stamp } from "../api";

  let { stamp, size = 150 }: { stamp: Stamp; size?: number } = $props();

  function rng(seed: number) {
    let a = seed >>> 0;
    return () => {
      a = (a + 0x6d2b79f5) | 0;
      let t = Math.imul(a ^ (a >>> 15), 1 | a);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  const INKS = ["#4b2f8f", "#a8232b", "#1d6b44", "#1f4a94", "#2b2b2b", "#86471a"];
  type Shape = "circle" | "double" | "rect" | "rounded" | "oval" | "hex" | "seal";
  const months = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];

  const d = $derived.by(() => {
    const r = rng(stamp.seed);
    const pick = <T,>(xs: T[]) => xs[Math.floor(r() * xs.length) % xs.length];
    const shape: Shape =
      stamp.kind === "first_entry" ? "seal" : pick<Shape>(["circle", "double", "rect", "rounded", "oval", "hex"]);
    const date = new Date(stamp.t_ms);
    return {
      shape,
      ink: pick(INKS),
      tilt: (r() - 0.5) * 30,
      opacity: 0.8 + r() * 0.14,
      wear: Math.floor(r() * 900),
      date: `${String(date.getUTCDate()).padStart(2, "0")} ${months[date.getUTCMonth()]} ${date.getUTCFullYear()}`,
    };
  });

  const id = $derived(`s${stamp.seed}`);
  const upper = (s: string) => s.toLocaleUpperCase("en-US");
  const place = $derived(upper(stamp.country_name || stamp.country));
  const word = $derived({ departure: "DEPARTED", arrival: "ADMITTED", first_entry: "FIRST ENTRY" }[stamp.kind]);
  const going = $derived(stamp.kind === "departure");
  /** Long names get smaller letters so they still fit. */
  const fit = (s: string, base: number, room: number) => Math.min(base, (room / Math.max(s.length, 1)) * 1.75);
  const wide = $derived(d.shape === "rect" || d.shape === "rounded" || d.shape === "oval");
</script>

{#snippet plane(x: number, y: number, s: number)}
  <g transform="translate({x} {y}) rotate({going ? -22 : 22}) scale({s})">
    <path d="M-20 0 L-6 -2.5 L2 -15 L6 -15 L2 -2.5 L14 -2.5 L18 -7 L21 -7 L19 0 L21 7 L18 7 L14 2.5 L2 2.5 L6 15 L2 15 L-6 2.5 Z" />
  </g>
{/snippet}

<svg
  class="stamp"
  viewBox={wide ? "0 0 240 160" : "0 0 220 220"}
  width={size}
  height={wide ? size * (160 / 240) : size}
  style:transform="rotate({d.tilt}deg)"
  style:opacity={d.opacity}
  role="img"
  aria-label="{stamp.kind.replace('_', ' ')} stamp, {stamp.country_name}, {d.date}"
>
  <defs>
    <filter id="ink-{id}" x="-8%" y="-8%" width="116%" height="116%">
      <feTurbulence type="fractalNoise" baseFrequency="0.85" numOctaves="2" seed={d.wear} result="grain" />
      <feDisplacementMap in="SourceGraphic" in2="grain" scale="2.4" xChannelSelector="R" yChannelSelector="G" result="rough" />
      <feTurbulence type="fractalNoise" baseFrequency="0.03" numOctaves="3" seed={d.wear + 7} result="blot" />
      <feColorMatrix in="blot" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  -1.7 0 0 0 1.6" result="press" />
      <feComposite in="rough" in2="press" operator="in" />
    </filter>
    <path id="top-{id}" d="M 30 110 A 80 80 0 0 1 190 110" />
    <path id="bottom-{id}" d="M 36 110 A 74 74 0 0 0 184 110" />
  </defs>

  <g filter="url(#ink-{id})" fill={d.ink} stroke={d.ink} font-family="Barlow Condensed, sans-serif" font-weight="600">
    {#if d.shape === "circle" || d.shape === "double" || d.shape === "seal"}
      <circle cx="110" cy="110" r="100" fill="none" stroke-width={d.shape === "seal" ? 3 : 5} />
      {#if d.shape !== "circle"}<circle cx="110" cy="110" r="91" fill="none" stroke-width="1.8" />{/if}
      {#if d.shape === "seal"}
        {#each Array(48) as _, i (i)}
          <line x1="110" y1="4" x2="110" y2="12" stroke-width="2.4" transform="rotate({i * 7.5} 110 110)" />
        {/each}
      {/if}
      <circle cx="110" cy="110" r="58" fill="none" stroke-width="1.6" />
      <text stroke="none" font-size={fit(place, 22, 150)} letter-spacing="2" text-anchor="middle">
        <textPath href="#top-{id}" startOffset="50%">{place}</textPath>
      </text>
      <text stroke="none" font-size="15" letter-spacing="3" text-anchor="middle">
        <textPath href="#bottom-{id}" startOffset="50%">{stamp.icao}{stamp.aircraft_type ? `  ${upper(stamp.aircraft_type)}` : ""}</textPath>
      </text>
      <g stroke="none">{@render plane(110, 86, 0.95)}</g>
      <text x="110" y="122" stroke="none" font-size={fit(word, 17, 100)} letter-spacing="1.5" text-anchor="middle">{word}</text>
      <text x="110" y="144" stroke="none" font-size="15" letter-spacing="1" text-anchor="middle">{d.date}</text>
    {:else if d.shape === "hex"}
      <polygon points="110,8 200,58 200,162 110,212 20,162 20,58" fill="none" stroke-width="5" />
      <polygon points="110,20 190,65 190,155 110,200 30,155 30,65" fill="none" stroke-width="1.6" />
      <text x="110" y="62" stroke="none" font-size={fit(place, 20, 120)} letter-spacing="1.5" text-anchor="middle">{place}</text>
      <g stroke="none">{@render plane(110, 92, 0.9)}</g>
      <text x="110" y="128" stroke="none" font-size="17" letter-spacing="1.5" text-anchor="middle">{word}</text>
      <text x="110" y="150" stroke="none" font-size="15" text-anchor="middle">{d.date}</text>
      <text x="110" y="174" stroke="none" font-size="15" letter-spacing="3" text-anchor="middle">{stamp.icao}</text>
    {:else}
      {#if d.shape === "oval"}
        <ellipse cx="120" cy="80" rx="114" ry="74" fill="none" stroke-width="5" />
        <ellipse cx="120" cy="80" rx="104" ry="65" fill="none" stroke-width="1.6" />
      {:else}
        <rect x="6" y="6" width="228" height="148" rx={d.shape === "rounded" ? 22 : 3} fill="none" stroke-width="5" />
        <rect x="15" y="15" width="210" height="130" rx={d.shape === "rounded" ? 15 : 1} fill="none" stroke-width="1.6" />
      {/if}
      <text x="120" y={d.shape === "oval" ? 50 : 44} stroke="none" font-size={fit(place, 22, 170)} letter-spacing="1.5" text-anchor="middle">{place}</text>
      <line x1="40" x2="200" y1={d.shape === "oval" ? 58 : 53} y2={d.shape === "oval" ? 58 : 53} stroke-width="1.4" />
      <g stroke="none">{@render plane(70, 82, 0.85)}</g>
      <text x="96" y="88" stroke="none" font-size="19" letter-spacing="1.5">{word}</text>
      <text x="120" y="114" stroke="none" font-size="16" letter-spacing="1" text-anchor="middle">{d.date}</text>
      <text x="120" y={d.shape === "oval" ? 134 : 137} stroke="none" font-size="14" letter-spacing="3" text-anchor="middle">
        {stamp.icao}{stamp.aircraft_type ? `  ${upper(stamp.aircraft_type)}` : ""}
      </text>
    {/if}
  </g>
</svg>

<style>
  .stamp {
    display: block;
    mix-blend-mode: multiply;
    overflow: visible;
  }
</style>
