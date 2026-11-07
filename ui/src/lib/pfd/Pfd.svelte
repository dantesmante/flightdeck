<script lang="ts">
  // A primary flight display drawn from the sim's numbers: attitude with
  // pitch ladder and bank scale, speed and altitude tapes, vertical speed,
  // and the heading rose, with the autopilot's selected values. Plain SVG,
  // drawn by the processor; the graphics card does next to nothing.
  //
  // Laid out like the G1000 in MSFS: the attitude centre sits about a third
  // of the way down, the tapes start just under the top edge, and the
  // heading rose fills the lower part. The frame is anchored to the top of
  // the window, so a taller window shows more ground and rose, never more
  // empty sky.
  //
  // With `svt` the sky and ground are left out, for the terrain view drawn
  // beneath it. Pitch is drawn at the terrain camera's scale (36.87° across
  // 500 units), with the camera's centre moved up to CY (Svt.svelte), so the
  // horizon lines up with the terrain's.
  import type { Pfd } from "../api";
  import { PFD_CY, PFD_H, PFD_K, PFD_W } from "./layout";

  let { d, svt = false }: { d: Pfd | null; svt?: boolean } = $props();

  const W = PFD_W;
  const H = PFD_H;
  const CX = 400;
  const CY = PFD_CY;
  /** Units per degree of pitch. */
  const K = PFD_K;

  const pitch = $derived(d?.pitch_deg ?? 0);
  const bank = $derived(d?.bank_deg ?? 0);
  const ias = $derived(Math.max(0, d?.ias_kt ?? 0));
  const alt = $derived(d?.alt_ft ?? 0);
  const vs = $derived(d?.vs_fpm ?? 0);
  const hdg = $derived(((d?.hdg_deg ?? 0) % 360 + 360) % 360);
  /** Magnetic variation, to show the true track on a magnetic rose. */
  const vari = $derived(d ? d.hdg_deg - d.hdg_true_deg : 0);
  const track = $derived(d?.track_true_deg != null && d.gs_kt > 30 ? (((d.track_true_deg + vari) % 360) + 360) % 360 : null);

  // --- Pitch ladder ---------------------------------------------------------
  const ladder = $derived.by(() => {
    const out: { deg: number; w: number; label: boolean }[] = [];
    const from = Math.floor((pitch - 20) / 2.5) * 2.5;
    for (let p = from; p <= pitch + 20; p += 2.5) {
      if (p === 0 || Math.abs(p) > 90) continue;
      const major = Math.abs(p % 10) < 0.01;
      const mid = Math.abs(p % 5) < 0.01;
      out.push({ deg: p, w: major ? 110 : mid ? 60 : 30, label: major });
    }
    return out;
  });

  // --- Tapes ------------------------------------------------------------------
  const SPD_K = 4; // units per knot
  const spdTicks = $derived.by(() => {
    const out: { v: number; y: number; major: boolean }[] = [];
    for (let v = Math.floor((ias - 40) / 5) * 5; v <= ias + 40; v += 5) {
      if (v < 0) continue;
      out.push({ v, y: CY - (v - ias) * SPD_K, major: v % 10 === 0 });
    }
    return out;
  });
  const ALT_K = 0.4; // units per foot
  const altTicks = $derived.by(() => {
    const out: { v: number; y: number; major: boolean }[] = [];
    for (let v = Math.floor((alt - 400) / 20) * 20; v <= alt + 400; v += 20) {
      out.push({ v, y: CY - (v - alt) * ALT_K, major: v % 100 === 0 });
    }
    return out;
  });
  /** Vertical speed to a scale offset: finer near zero. */
  function vsY(v: number) {
    const a = Math.abs(v);
    const off = a <= 1000 ? a * 0.07 : a <= 2000 ? 70 + (a - 1000) * 0.04 : 110 + Math.min(a - 2000, 2000) * 0.01;
    return CY - Math.sign(v) * off;
  }

  // --- Heading rose ----------------------------------------------------------
  const RX = CX;
  const RY = CY + 310;
  const RR = 150;
  const rose = Array.from({ length: 72 }, (_, i) => i * 5);
  const LABELS: Record<number, string> = { 0: "N", 90: "E", 180: "S", 270: "W" };
  const rad = (a: number) => (a * Math.PI) / 180;
  const at = (a: number, r: number) => [RX + r * Math.sin(rad(a)), RY - r * Math.cos(rad(a))];

  const fmt = (n: number) => Math.round(n).toLocaleString("en-US");
  const hdg3 = (h: number) => String(Math.round(h) % 360 || 360).padStart(3, "0");
  const clamp = (v: number, a: number, b: number) => Math.min(b, Math.max(a, v));
  const altHundreds = $derived(Math.floor(Math.abs(alt) / 100));
  const altTens = $derived(String(Math.round((Math.abs(alt) % 100) / 20) * 20 % 100).padStart(2, "0"));
</script>

<svg viewBox="0 0 {W} {H}" preserveAspectRatio="xMidYMin meet" class="pfd" class:svt role="img" aria-label="Primary flight display" overflow="visible">
  <defs>
    <linearGradient id="pfd-sky" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#0b3fd6" />
      <stop offset="1" stop-color="#5b8ef0" />
    </linearGradient>
    <linearGradient id="pfd-ground" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#8a5a1c" />
      <stop offset="1" stop-color="#4a2e0c" />
    </linearGradient>
    <!-- Sky and ground reach past the frame, to fill a taller or wider
         window around the instruments. -->
    <clipPath id="pfd-att"><rect x={-W * 2} y={-H * 3} width={W * 5} height={H * 7} /></clipPath>
    <clipPath id="pfd-ladder"><rect x={CX - 140} y={CY - 150} width="280" height="245" /></clipPath>
    <clipPath id="pfd-spd"><rect x="110" y={CY - 150} width="92" height="300" /></clipPath>
    <clipPath id="pfd-alt"><rect x="598" y={CY - 150} width="92" height="300" /></clipPath>
  </defs>

  <!-- Attitude: the world turns and moves; the aircraft stays. -->
  <g clip-path="url(#pfd-att)">
    <g transform="rotate({-bank} {CX} {CY}) translate(0 {pitch * K})">
      {#if !svt}
        <rect x={-W * 3} y={CY - 4000} width={W * 7} height="4000" fill="url(#pfd-sky)" />
        <rect x={-W * 3} y={CY} width={W * 7} height="4000" fill="url(#pfd-ground)" />
      {/if}
      <line x1={-W * 3} x2={W * 4} y1={CY} y2={CY} stroke="#fff" stroke-width="2" />
      <g clip-path="url(#pfd-ladder)">
        {#each ladder as l (l.deg)}
          {@const y = CY - l.deg * K}
          <line x1={CX - l.w / 2} x2={CX + l.w / 2} y1={y} y2={y} stroke="#fff" stroke-width="2" class="shadow" />
          {#if l.label}
            <text x={CX - l.w / 2 - 8} y={y + 6} text-anchor="end" class="lad">{Math.abs(l.deg)}</text>
            <text x={CX + l.w / 2 + 8} y={y + 6} class="lad">{Math.abs(l.deg)}</text>
          {/if}
        {/each}
      </g>
    </g>
  </g>

  <!-- Bank scale turns with the horizon; the pointer at the top stays. -->
  <g transform="rotate({-bank} {CX} {CY})">
    <path d="M {CX - 150 * Math.sin(rad(60))} {CY - 150 * Math.cos(rad(60))} A 150 150 0 0 1 {CX + 150 * Math.sin(rad(60))} {CY - 150 * Math.cos(rad(60))}" fill="none" stroke="#fff" stroke-width="2" class="shadow" />
    {#each [-60, -45, -30, -20, -10, 10, 20, 30, 45, 60] as b (b)}
      {@const len = Math.abs(b) === 30 || Math.abs(b) === 60 ? 18 : 10}
      <line x1={CX + 150 * Math.sin(rad(b))} y1={CY - 150 * Math.cos(rad(b))} x2={CX + (150 + len) * Math.sin(rad(b))} y2={CY - (150 + len) * Math.cos(rad(b))} stroke="#fff" stroke-width="2" />
    {/each}
    <path d="M {CX} {CY - 150} l -9 -14 h 18 z" fill="#fff" />
  </g>
  <path d="M {CX} {CY - 148} l -10 16 h 20 z" fill="#fff" stroke="#000" stroke-width="1" />
  {#if d?.ball != null}
    <rect x={CX - 11 + d.ball * 24} y={CY - 128} width="22" height="5" fill="#fff" stroke="#000" stroke-width="1" />
  {/if}

  <!-- The aircraft. -->
  <g class="ac">
    <path d="M {CX} {CY} l -95 26 l 0 -6 z M {CX} {CY} l 95 26 l 0 -6 z" fill="#ffd400" stroke="#000" stroke-width="1.2" />
    <path d="M {CX} {CY} l -95 26 l 34 0 z M {CX} {CY} l 95 26 l -34 0 z" fill="#c9a400" stroke="#000" stroke-width="1" />
    <rect x={CX - 210} y={CY - 3} width="44" height="7" rx="2" fill="#ffd400" stroke="#000" />
    <rect x={CX + 166} y={CY - 3} width="44" height="7" rx="2" fill="#ffd400" stroke="#000" />
  </g>

  <!-- Speed. -->
  <rect x="110" y={CY - 150} width="92" height="300" class="tape" />
  <g clip-path="url(#pfd-spd)">
    {#each spdTicks as t (t.v)}
      <line x1={t.major ? 184 : 192} x2="202" y1={t.y} y2={t.y} stroke="#fff" stroke-width="2" />
      {#if t.major}<text x="178" y={t.y + 7} text-anchor="end" class="num">{t.v}</text>{/if}
    {/each}
    {#if d?.sel_spd_kt != null}
      <path d="M 202 {clamp(CY - (d.sel_spd_kt - ias) * SPD_K, CY - 150, CY + 150)} l -10 -9 v 18 z" fill="#ff3dd1" />
    {/if}
  </g>
  <path d="M 108 {CY - 20} h 72 l 14 20 l -14 20 h -72 z" class="box" />
  <text x="172" y={CY + 10} text-anchor="end" class="big">{Math.round(ias)}</text>
  {#if d?.sel_spd_kt != null}
    <text x="114" y={CY - 158} class="sel mag">▸ {Math.round(d.sel_spd_kt)}<tspan class="unit">KT</tspan></text>
  {/if}

  <!-- Altitude and vertical speed. -->
  <rect x="598" y={CY - 150} width="92" height="300" class="tape" />
  <g clip-path="url(#pfd-alt)">
    {#each altTicks as t (t.v)}
      <line x1="598" x2={t.major ? 614 : 606} y1={t.y} y2={t.y} stroke="#fff" stroke-width="2" />
      {#if t.major}<text x="684" y={t.y + 7} text-anchor="end" class="num">{t.v}</text>{/if}
    {/each}
    {#if d?.sel_alt_ft != null}
      <path d="M 598 {clamp(CY - (d.sel_alt_ft - alt) * ALT_K, CY - 150, CY + 150)} m 0 -12 h 8 v 7 l 6 5 l -6 5 v 7 h -8 z" fill="#3fe0ff" />
    {/if}
  </g>
  <path d="M 612 {CY - 22} h 82 v 44 h -82 l -14 -22 z" class="box" />
  <text x="660" y={CY + 10} text-anchor="end" class="big">{alt < 0 ? "-" : ""}{altHundreds}</text>
  <text x="663" y={CY + 9} class="tens">{altTens}</text>
  {#if d?.sel_alt_ft != null}
    <text x="690" y={CY - 158} text-anchor="end" class="sel cyan">{fmt(d.sel_alt_ft)}<tspan class="unit">FT</tspan></text>
  {/if}
  <rect x="598" y={CY + 152} width="92" height="28" class="tag" />
  <text x="644" y={CY + 173} text-anchor="middle" class="sel cyan">
    {#if d?.baro_inhg != null}{Math.abs(d.baro_inhg - 29.92) < 0.005 && d.alt_ft > 18000 ? "STD" : d.baro_inhg.toFixed(2)}<tspan class="unit">IN</tspan>{:else}–{/if}
  </text>

  <rect x="696" y={CY - 130} width="34" height="260" class="tape thin" />
  {#each [-2000, -1000, -500, 500, 1000, 2000] as m (m)}
    <line x1="696" x2={Math.abs(m) === 500 ? 703 : 708} y1={vsY(m)} y2={vsY(m)} stroke="#fff" stroke-width="2" />
    {#if Math.abs(m) !== 500}<text x="726" y={vsY(m) + 6} text-anchor="end" class="small">{Math.abs(m) / 1000}</text>{/if}
  {/each}
  {#if d?.sel_vs_fpm != null && Math.abs(d.sel_vs_fpm) > 50}
    <path d="M 696 {vsY(d.sel_vs_fpm)} m 0 -7 h 6 l 5 7 l -5 7 h -6 z" fill="#3fe0ff" />
  {/if}
  <path d="M 696 {vsY(vs)} l 14 -11 h 46 v 22 h -46 z" class="box" />
  <text x="752" y={vsY(vs) + 7} text-anchor="end" class="small">{Math.abs(vs) >= 100 ? Math.round(vs / 50) * 50 : 0}</text>

  <!-- Heading. -->
  <circle cx={RX} cy={RY} r={RR} class="rose" />
  <g transform="rotate({-hdg} {RX} {RY})">
    {#each rose as a (a)}
      {@const [x1, y1] = at(a, RR)}
      {@const [x2, y2] = at(a, RR - (a % 10 === 0 ? 14 : 8))}
      <line {x1} {y1} {x2} {y2} stroke="#fff" stroke-width="2" />
      {#if a % 30 === 0}
        {@const [tx, ty] = at(a, RR - 30)}
        <text x={tx} y={ty + 8} text-anchor="middle" transform="rotate({a} {tx} {ty})" class="rose-l">{LABELS[a] ?? a / 10}</text>
      {/if}
    {/each}
    {#if d?.sel_hdg_deg != null}
      {@const [bx, by] = at(d.sel_hdg_deg, RR)}
      <path d="M {bx} {by} m -9 -9 h 18 v 9 l -4 0 l -5 6 l -5 -6 l -4 0 z" fill="#3fe0ff" transform="rotate({d.sel_hdg_deg} {bx} {by})" />
    {/if}
    {#if track != null}
      {@const [kx, ky] = at(track, RR - 4)}
      <path d="M {kx} {ky} l -6 8 l 6 8 l 6 -8 z" fill="#ff3dd1" transform="rotate({track} {kx} {ky})" />
    {/if}
  </g>
  <path d="M {RX} {RY - RR - 2} l -9 -12 h 18 z" fill="#fff" />
  <rect x={RX - 40} y={RY - RR - 50} width="80" height="36" rx="4" class="box" />
  <text x={RX} y={RY - RR - 22} text-anchor="middle" class="big">{hdg3(hdg)}°</text>
  {#if d?.sel_hdg_deg != null}
    <text x={RX - 170} y={RY - RR + 6} class="sel"><tspan class="lbl">HDG</tspan> <tspan class="cyan">{hdg3(d.sel_hdg_deg)}°</tspan></text>
  {/if}
  <path d="M {RX} {RY - 22} v 44 M {RX - 22} {RY - 2} h 44 M {RX - 10} {RY + 18} h 20" stroke="#fff" stroke-width="4" stroke-linecap="round" />

  <!-- Speeds, height above ground, autopilot. -->
  <text x="14" y={H - 40} class="sel"><tspan class="lbl">TAS</tspan> {d?.tas_kt != null ? Math.round(d.tas_kt) : "–"}<tspan class="unit">KT</tspan></text>
  <text x="14" y={H - 14} class="sel"><tspan class="lbl">GS</tspan> {d ? Math.round(d.gs_kt) : "–"}<tspan class="unit">KT</tspan></text>
  {#if d && !d.on_ground && d.agl_ft < 2500}
    <rect x="520" y={CY + 104} width="72" height="28" rx="3" class="tag" />
    <text x="556" y={CY + 125} text-anchor="middle" class="sel"><tspan class="lbl">RA</tspan> {Math.round(d.agl_ft / 10) * 10}</text>
  {/if}
  {#if d?.ap_on}
    <rect x={CX - 24} y="8" width="48" height="26" rx="3" class="tag" />
    <text x={CX} y="28" text-anchor="middle" class="sel green">AP</text>
  {/if}

  {#if !d}
    <text x={CX} y={CY - 40} text-anchor="middle" class="note">Waiting for the simulator</text>
  {:else if d.paused}
    <text x={CX} y={CY - 40} text-anchor="middle" class="note">Paused</text>
  {/if}
</svg>

<style>
  .pfd {
    display: block;
    width: 100%;
    height: 100%;
    background: #000;
    font-family: "Inter", system-ui, sans-serif;
    user-select: none;
  }
  .pfd.svt {
    background: transparent;
  }
  text {
    fill: #fff;
    font-weight: 600;
  }
  .num {
    font-size: 20px;
  }
  .lad {
    font-size: 17px;
    paint-order: stroke;
    stroke: rgba(0, 0, 0, 0.6);
    stroke-width: 3px;
  }
  .shadow {
    filter: drop-shadow(0 0 1px rgba(0, 0, 0, 0.8));
  }
  .big {
    font-size: 27px;
    font-weight: 700;
  }
  .tens {
    font-size: 19px;
    font-weight: 700;
  }
  .small {
    font-size: 15px;
  }
  .sel {
    font-size: 18px;
  }
  .unit {
    font-size: 12px;
  }
  .lbl {
    fill: #fff;
    font-size: 14px;
  }
  .mag {
    fill: #ff3dd1;
  }
  .cyan,
  .cyan :global(tspan) {
    fill: #3fe0ff;
  }
  .green {
    fill: #29e05a;
  }
  .tape {
    fill: rgba(30, 40, 70, 0.45);
    stroke: rgba(255, 255, 255, 0.25);
  }
  .tape.thin {
    fill: rgba(30, 40, 70, 0.35);
  }
  .box,
  .tag {
    fill: #000;
    stroke: #fff;
    stroke-width: 1.5;
  }
  .tag {
    fill: rgba(0, 0, 0, 0.75);
    stroke: rgba(255, 255, 255, 0.4);
  }
  .rose {
    fill: rgba(30, 25, 20, 0.55);
    stroke: rgba(255, 255, 255, 0.4);
  }
  .rose-l {
    font-size: 21px;
  }
  .note {
    font-size: 26px;
    paint-order: stroke;
    stroke: #000;
    stroke-width: 4px;
  }
</style>
