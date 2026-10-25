<script lang="ts">
  // A side-profile illustration of an aircraft, standing on the apron like a
  // spotter photo. One parametric drawing covers every type; the livery
  // colour and the light come from the registration, so each plane is its own.
  import { artKind, hash, type ArtKind } from "./kind";

  interface Props {
    icao_type: string;
    engine_type: string;
    engine_count: number;
    title: string;
    registration: string;
    /** Livery colour slot; the hangar passes the aircraft's id so planes differ. */
    tone?: number;
  }
  let a: Props = $props();

  const W = 640;
  const VH = 360;
  const GROUND = 286;

  const LIVERY = ["#c8102e", "#0b5fa5", "#1b7f5b", "#e0a21a", "#5a2d82", "#e0662a", "#1f2a44", "#0e8a96"];
  const SKIES = [
    ["#7fb6e8", "#d9ecf7"], // clear day
    ["#f2b48a", "#fbe4c6"], // golden hour
    ["#59618f", "#e3a27f"], // dusk
    ["#9aa9b6", "#dfe5ea"], // overcast
    ["#4b8fd1", "#c9e3f5"], // high summer
  ];

  const kind = $derived<ArtKind>(artKind(a));
  const seed = $derived(hash(`${a.registration}|${a.icao_type}|${a.title}`));
  // Integer hash mix (Math.imul keeps all 32 bits; plain * would round).
  const mixed = $derived.by(() => {
    let h = seed;
    h = Math.imul(h ^ (h >>> 16), 0x45d9f3b) >>> 0;
    h = Math.imul(h ^ (h >>> 16), 0x45d9f3b) >>> 0;
    return (h ^ (h >>> 16)) >>> 0;
  });
  const livery = $derived(LIVERY[(a.tone ?? mixed >>> 3) % LIVERY.length]);
  const sky = $derived(SKIES[(mixed >>> 11) % SKIES.length]);
  const uid = $derived(`a${seed}`);

  type Spec = {
    L: number;
    H: number;
    nose: "round" | "pointy" | "prop";
    tail: "low" | "T";
    wing: "low" | "high" | "none";
    engines: "underwing" | "rear" | "wingprops" | "nose" | "none";
    gear: "big" | "small" | "fixed" | "tail" | "skids" | "mono";
    windows: "row" | "ga" | "none";
    finH: number;
  };

  const SPECS: Record<ArtKind, Spec> = {
    widebody: { L: 580, H: 66, nose: "round", tail: "low", wing: "low", engines: "underwing", gear: "big", windows: "row", finH: 84 },
    airliner: { L: 520, H: 52, nose: "round", tail: "low", wing: "low", engines: "underwing", gear: "big", windows: "row", finH: 70 },
    regional: { L: 460, H: 44, nose: "round", tail: "T", wing: "low", engines: "rear", gear: "small", windows: "row", finH: 64 },
    bizjet: { L: 400, H: 38, nose: "pointy", tail: "T", wing: "low", engines: "rear", gear: "small", windows: "ga", finH: 56 },
    "turboprop-twin": { L: 470, H: 48, nose: "round", tail: "T", wing: "high", engines: "wingprops", gear: "small", windows: "row", finH: 70 },
    "turboprop-single": { L: 360, H: 50, nose: "prop", tail: "low", wing: "high", engines: "nose", gear: "fixed", windows: "ga", finH: 60 },
    "piston-twin": { L: 330, H: 40, nose: "pointy", tail: "low", wing: "low", engines: "wingprops", gear: "small", windows: "ga", finH: 48 },
    "piston-high": { L: 300, H: 46, nose: "prop", tail: "low", wing: "high", engines: "nose", gear: "fixed", windows: "ga", finH: 50 },
    "piston-low": { L: 300, H: 40, nose: "prop", tail: "low", wing: "low", engines: "nose", gear: "fixed", windows: "ga", finH: 46 },
    helicopter: { L: 330, H: 64, nose: "round", tail: "low", wing: "none", engines: "none", gear: "skids", windows: "none", finH: 40 },
    glider: { L: 360, H: 30, nose: "pointy", tail: "T", wing: "low", engines: "none", gear: "mono", windows: "none", finH: 50 },
  };

  const g = $derived.by(() => {
    const s = SPECS[kind];
    const strut = s.gear === "big" ? 40 : s.gear === "skids" ? 26 : s.gear === "mono" ? 10 : 30;
    const yc = GROUND - strut - s.H / 2;
    const x0 = (W - s.L) / 2;
    return { s, yc, x0, top: yc - s.H / 2, bot: yc + s.H / 2 };
  });

  const ga = $derived(g.s.nose === "prop");

  // Fuselage outline (nose left, tail right). Light aircraft get a tall
  // cabin and a cone that tapers to the tail; jets a tube whose belly
  // sweeps up into a narrow tail.
  const body = $derived.by(() => {
    const { s, x0, yc, top, bot } = g;
    const L = s.L;
    const H = s.H;
    if (ga) {
      const n = L * 0.1;
      return [
        `M ${x0 + n} ${yc - H * 0.22}`,
        `L ${x0 + L * 0.17} ${yc - H * 0.3}`,
        `L ${x0 + L * 0.26} ${top}`,
        `L ${x0 + L * 0.46} ${top}`,
        `L ${x0 + L * 0.97} ${yc - H * 0.24}`,
        `L ${x0 + L} ${yc - H * 0.14}`,
        `L ${x0 + L * 0.97} ${yc - H * 0.04}`,
        `L ${x0 + L * 0.44} ${bot}`,
        `L ${x0 + n} ${bot - H * 0.04}`,
        `C ${x0 + n * 0.3} ${bot - H * 0.06} ${x0} ${yc + H * 0.2} ${x0} ${yc + H * 0.02}`,
        `C ${x0} ${yc - H * 0.14} ${x0 + n * 0.4} ${yc - H * 0.22} ${x0 + n} ${yc - H * 0.22}`,
        "Z",
      ].join(" ");
    }
    const n = s.nose === "pointy" ? L * 0.16 : L * 0.11;
    const tipY = s.nose === "pointy" ? yc + H * 0.12 : yc + H * 0.04;
    return [
      `M ${x0 + n} ${top}`,
      `L ${x0 + L * 0.88} ${top}`,
      `L ${x0 + L} ${top + H * 0.16}`,
      `L ${x0 + L} ${top + H * 0.3}`,
      `Q ${x0 + L * 0.84} ${yc + H * 0.26} ${x0 + L * 0.68} ${bot}`,
      `L ${x0 + n} ${bot}`,
      `C ${x0 + n * 0.35} ${bot} ${x0} ${yc + H * 0.3} ${x0} ${tipY}`,
      `C ${x0} ${yc - H * 0.3} ${x0 + n * 0.3} ${top} ${x0 + n} ${top}`,
      "Z",
    ].join(" ");
  });

  /** Height of the fuselage's top edge at a point along it (light aircraft taper). */
  function topAt(f: number) {
    const { s, yc, top } = g;
    if (!ga || f <= 0.46) return top;
    const k = (f - 0.46) / (0.97 - 0.46);
    return top + (yc - s.H * 0.24 - top) * Math.min(1, k);
  }

  const fin = $derived.by(() => {
    const { s, x0, yc, top } = g;
    const L = s.L;
    if (ga) {
      const tipY = topAt(0.9) - s.finH * 0.95;
      return `M ${x0 + L * 0.76} ${topAt(0.76) + 1} L ${x0 + L * 0.9} ${tipY} L ${x0 + L * 0.97} ${tipY} L ${x0 + L} ${yc - s.H * 0.14} Z`;
    }
    return `M ${x0 + L * 0.77} ${top + 1} L ${x0 + L * 0.9} ${top - s.finH} L ${x0 + L * 0.965} ${top - s.finH} L ${x0 + L * 0.995} ${top + s.H * 0.2} Z`;
  });

  const windows = $derived.by(() => {
    const { s, x0, yc } = g;
    const out: { x: number; y: number; w: number; h: number }[] = [];
    if (s.windows === "row") {
      const step = Math.max(9, s.H * 0.2);
      for (let x = x0 + s.L * 0.2; x < x0 + s.L * 0.76; x += step) {
        out.push({ x, y: yc - s.H * 0.18, w: s.H * 0.09, h: s.H * 0.14 });
      }
    } else if (s.windows === "ga") {
      const n = kind === "bizjet" ? 5 : kind === "piston-twin" || kind === "turboprop-single" ? 3 : 2;
      const gap = kind === "bizjet" ? 0.075 : 0.085;
      const startX = kind === "bizjet" ? 0.24 : 0.29;
      for (let i = 0; i < n; i++) {
        out.push({ x: x0 + s.L * (startX + i * gap), y: yc - s.H * (ga ? 0.36 : 0.26), w: s.L * 0.055, h: s.H * 0.26 });
      }
    }
    return out;
  });
</script>

<svg viewBox="0 0 {W} {VH}" class="art" role="img" aria-label="Illustration of {a.title || a.icao_type || 'aircraft'}">
  <defs>
    <linearGradient id="sky-{uid}" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color={sky[0]} />
      <stop offset="1" stop-color={sky[1]} />
    </linearGradient>
    <linearGradient id="tarmac-{uid}" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#6c737b" />
      <stop offset="1" stop-color="#3d4248" />
    </linearGradient>
    <linearGradient id="skin-{uid}" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" />
      <stop offset="0.55" stop-color="#eef0f1" />
      <stop offset="1" stop-color="#b9bfc5" />
    </linearGradient>
    <clipPath id="body-{uid}"><path d={body} /></clipPath>
  </defs>

  <!-- Sky, distant tree line and apron -->
  <rect width={W} height={VH} fill="url(#sky-{uid})" />
  <path
    d="M0 {GROUND - 14} C 80 {GROUND - 26} 130 {GROUND - 18} 200 {GROUND - 24} S 330 {GROUND - 16} 420 {GROUND - 26} S 560 {GROUND - 15} 640 {GROUND - 22} V {GROUND} H 0 Z"
    fill="#55705f"
    opacity="0.55"
  />
  <rect y={GROUND - 4} width={W} height={VH - GROUND + 4} fill="url(#tarmac-{uid})" />
  <line x1="0" y1={GROUND + 30} x2={W} y2={GROUND + 30} stroke="#e8c547" stroke-width="3" stroke-dasharray="26 18" opacity="0.8" />
  <ellipse cx={g.x0 + g.s.L * 0.5} cy={GROUND + 6} rx={g.s.L * 0.48} ry="7" fill="#000" opacity="0.25" />

  {#if kind === "helicopter"}
    {@const { x0, yc, s } = g}
    <!-- Helicopter: cabin, tail boom, rotors, skids -->
    <path d="M {x0 + s.L * 0.36} {yc - 6} L {x0 + s.L * 0.95} {yc - 4} L {x0 + s.L * 0.95} {yc + 4} L {x0 + s.L * 0.36} {yc + 10} Z" fill="url(#skin-{uid})" stroke="#5d6670" stroke-width="1" />
    <path d="M {x0 + s.L * 0.9} {yc - 6} L {x0 + s.L * 0.97} {yc - 46} L {x0 + s.L} {yc - 46} L {x0 + s.L} {yc + 2} Z" fill={livery} />
    <circle cx={x0 + s.L * 0.985} cy={yc - 20} r="20" fill="#2b3440" opacity="0.18" />
    <ellipse cx={x0 + s.L * 0.24} cy={yc} rx={s.L * 0.24} ry={s.H * 0.5} fill="url(#skin-{uid})" stroke="#5d6670" stroke-width="1" />
    <path d="M {x0 + s.L * 0.03} {yc} A {s.L * 0.22} {s.H * 0.46} 0 0 1 {x0 + s.L * 0.2} {yc - s.H * 0.44} L {x0 + s.L * 0.2} {yc + 2} Z" fill="#2b3440" opacity="0.85" />
    <rect x={x0} y={yc + s.H * 0.12} width={s.L * 0.48} height="8" fill={livery} clip-path="url(#body-{uid})" />
    <rect x={x0 + s.L * 0.18} y={yc + s.H * 0.08} width={s.L * 0.3} height="9" fill={livery} />
    <rect x={x0 + s.L * 0.22} y={yc - s.H * 0.5 - 14} width="12" height="16" fill="#5d6670" />
    <ellipse cx={x0 + s.L * 0.24} cy={yc - s.H * 0.5 - 16} rx={s.L * 0.62} ry="3.5" fill="#2b3440" opacity="0.8" />
    <line x1={x0 + s.L * 0.1} y1={yc + s.H * 0.5} x2={x0 + s.L * 0.12} y2={GROUND - 4} stroke="#4a525b" stroke-width="4" />
    <line x1={x0 + s.L * 0.36} y1={yc + s.H * 0.5} x2={x0 + s.L * 0.34} y2={GROUND - 4} stroke="#4a525b" stroke-width="4" />
    <path d="M {x0 - 10} {GROUND - 10} Q {x0 - 6} {GROUND - 2} {x0 + 6} {GROUND - 2} L {x0 + s.L * 0.46} {GROUND - 2}" fill="none" stroke="#3b424a" stroke-width="5" stroke-linecap="round" />
  {:else}
    {@const { x0, yc, top, bot, s } = g}
    {@const L = s.L}
    {@const H = s.H}

    <!-- Far-side wing (behind the fuselage) -->
    {#if s.wing === "low"}
      <path d="M {x0 + L * 0.4} {bot - H * 0.15} L {x0 + L * 0.6} {bot - H * 0.32} L {x0 + L * 0.66} {bot - H * 0.32} L {x0 + L * 0.58} {bot - H * 0.12} Z" fill="#9aa3ab" />
    {/if}

    <!-- Tail -->
    <path d={fin} fill={livery} />
    {#if s.tail === "T"}
      <ellipse cx={x0 + L * 0.94} cy={top - s.finH + 3} rx={L * 0.09} ry="4.5" fill="#c9ced3" stroke="#5d6670" stroke-width="0.8" />
    {/if}

    <!-- Rear engines sit beside the fuselage -->
    {#if s.engines === "rear"}
      <path d="M {x0 + L * 0.72} {yc - H * 0.15} L {x0 + L * 0.78} {yc - H * 0.15}" stroke="#7d868f" stroke-width="5" />
      <rect x={x0 + L * 0.66} y={yc - H * 0.62} width={L * 0.16} height={H * 0.46} rx={H * 0.22} fill="url(#skin-{uid})" stroke="#5d6670" stroke-width="1" />
      <ellipse cx={x0 + L * 0.665} cy={yc - H * 0.39} rx="3" ry={H * 0.2} fill="#2b3440" />
    {/if}

    <!-- Fuselage, livery stripe, windows, registration -->
    <path d={body} fill="url(#skin-{uid})" stroke="#5d6670" stroke-width="1" />
    <g clip-path="url(#body-{uid})">
      <rect x={x0} y={yc + H * 0.08} width={L} height={H * 0.14} fill={livery} />
      <rect x={x0} y={yc + H * 0.24} width={L} height={H * 0.04} fill={livery} opacity="0.6" />
      <rect x={x0} y={yc + H * 0.3} width={L} height={H} fill="#aeb5bc" opacity="0.35" />
    </g>
    {#each windows as w, wi (wi)}
      <rect x={w.x} y={w.y} width={w.w} height={w.h} rx={Math.min(w.w, w.h) * 0.4} fill="#26303b" />
    {/each}
    {#if s.nose === "prop"}
      <path d="M {x0 + L * 0.175} {yc - H * 0.28} L {x0 + L * 0.255} {top + 3} L {x0 + L * 0.265} {yc - H * 0.06} L {x0 + L * 0.19} {yc - H * 0.06} Z" fill="#26303b" />
    {:else if s.windows !== "none"}
      <path d="M {x0 + L * 0.035} {yc - H * 0.12} Q {x0 + L * 0.06} {yc - H * 0.34} {x0 + L * 0.1} {yc - H * 0.36} L {x0 + L * 0.105} {yc - H * 0.12} Z" fill="#26303b" />
    {:else}
      <path d="M {x0 + L * 0.12} {top + 2} Q {x0 + L * 0.2} {top - H * 0.55} {x0 + L * 0.32} {top + 2} Z" fill="#3b5168" opacity="0.85" />
    {/if}
    {#if a.registration}
      <text x={x0 + L * (ga ? 0.7 : 0.78)} y={ga ? yc - H * 0.02 : yc + H * 0.02} font-size={Math.max(9, H * 0.22)} font-family="Barlow Condensed, sans-serif" font-weight="600" fill="#2b3440" text-anchor="end">
        {a.registration}
      </text>
    {/if}

    <!-- Stabiliser on a low tail -->
    {#if s.tail === "low"}
      {@const sy = ga ? yc - H * 0.16 : top + H * 0.32}
      <path d="M {x0 + L * 0.84} {sy} L {x0 + L * 0.97} {sy - H * 0.08} L {x0 + L * 0.995} {sy + H * 0.04} L {x0 + L * 0.86} {sy + H * 0.08} Z" fill="#c9ced3" stroke="#5d6670" stroke-width="0.8" />
    {/if}

    <!-- Near-side wing -->
    {#if s.wing === "low"}
      <path d="M {x0 + L * 0.36} {bot - H * 0.08} L {x0 + L * 0.6} {bot + H * 0.12} L {x0 + L * 0.68} {bot + H * 0.12} L {x0 + L * 0.56} {bot - H * 0.04} Z" fill="#d7dbdf" stroke="#5d6670" stroke-width="1" />
    {:else if s.wing === "high"}
      <path d="M {x0 + L * 0.28} {top - 2} L {x0 + L * 0.58} {top - 2} L {x0 + L * 0.58} {top + H * 0.1} L {x0 + L * 0.28} {top + H * 0.12} Z" fill="#d7dbdf" stroke="#5d6670" stroke-width="1" />
      {#if kind === "piston-high" || kind === "turboprop-single"}
        <line x1={x0 + L * 0.42} y1={top + H * 0.1} x2={x0 + L * 0.36} y2={bot - 2} stroke="#7d868f" stroke-width="2.5" />
      {/if}
    {/if}

    <!-- Engines -->
    {#if s.engines === "underwing"}
      <path d="M {x0 + L * 0.41} {bot - H * 0.04} L {x0 + L * 0.43} {bot + H * 0.08}" stroke="#7d868f" stroke-width="7" />
      <rect x={x0 + L * 0.3} y={bot - H * 0.02} width={L * 0.14} height={H * 0.46} rx={H * 0.22} fill="url(#skin-{uid})" stroke="#5d6670" stroke-width="1" />
      <ellipse cx={x0 + L * 0.305} cy={bot + H * 0.21} rx="4.5" ry={H * 0.2} fill="#2b3440" />
      <rect x={x0 + L * 0.32} y={bot + H * 0.17} width={L * 0.1} height={H * 0.06} fill={livery} opacity="0.85" />
    {:else if s.engines === "wingprops"}
      {@const ny = s.wing === "high" ? top + H * 0.1 : bot - H * 0.05}
      <rect x={x0 + L * 0.36} y={ny - H * 0.08} width={L * 0.16} height={H * 0.32} rx={H * 0.14} fill="url(#skin-{uid})" stroke="#5d6670" stroke-width="1" />
      <ellipse cx={x0 + L * 0.355} cy={ny + H * 0.08} rx="3" ry={H * 0.75} fill="#2b3440" opacity="0.22" />
      <circle cx={x0 + L * 0.355} cy={ny + H * 0.08} r="4" fill="#3b424a" />
    {:else if s.engines === "nose"}
      <ellipse cx={x0 - 2} cy={yc + 2} rx="3.5" ry={H * 0.9} fill="#2b3440" opacity="0.22" />
      <path d="M {x0 + 4} {yc - 7} Q {x0 - 10} {yc + 2} {x0 + 4} {yc + 11} Z" fill={livery} />
    {/if}

    <!-- Landing gear -->
    {@const wheel = s.gear === "big" ? 11 : 9}
    {#if s.gear === "mono"}
      <circle cx={x0 + L * 0.3} cy={GROUND - 9} r="9" fill="#23282d" />
    {:else}
      {@const noseX = x0 + L * (s.nose === "prop" ? 0.1 : 0.13)}
      {@const mainX = x0 + L * (s.wing === "high" ? 0.4 : 0.52)}
      <line x1={noseX} y1={bot - 2} x2={noseX} y2={GROUND - wheel} stroke="#7d868f" stroke-width="3" />
      <circle cx={noseX} cy={GROUND - wheel} r={wheel * 0.85} fill="#23282d" />
      <line x1={mainX} y1={bot - 2} x2={mainX + 4} y2={GROUND - wheel} stroke="#7d868f" stroke-width="4" />
      <circle cx={mainX + 4} cy={GROUND - wheel} r={wheel} fill="#23282d" />
      {#if s.gear === "big"}
        <circle cx={mainX + 4 + wheel * 2.1} cy={GROUND - wheel} r={wheel} fill="#23282d" />
      {/if}
    {/if}
  {/if}
</svg>

<style>
  .art {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
