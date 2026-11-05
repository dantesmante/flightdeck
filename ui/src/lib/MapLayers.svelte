<script lang="ts">
  // The map's Layers panel: picture cards you switch on and off, grouped
  // into the view, your own flights and weather.
  import type { Snippet } from "svelte";
  import { WIND_LEVELS } from "./wind/wind";

  interface Props {
    three: boolean;
    on: Set<string>;
    onthree: (v: boolean) => void;
    ontoggle: (key: string) => void;
    /** Time of the radar picture shown, Unix seconds, or null. */
    radarTime: number | null;
    radarError: boolean;
    /** Offer the Parked aircraft layer (the main map only). */
    parkedLayer?: boolean;
    /** World airports detail: 1 major, 2 + regional, 3 + small. */
    airportDetail?: number;
    ondetail?: (d: number) => void;
    /** Wind level: "auto" (follows the aircraft) or a level id. */
    windLevel?: string;
    onwindlevel?: (l: string) => void;
  }
  let { three, on, onthree, ontoggle, radarTime, radarError, parkedLayer = false, airportDetail = 1, ondetail, windLevel = "auto", onwindlevel }: Props = $props();
  const DETAIL = ["Major airports only", "Major and regional airports", "All airports and airstrips"];

  let open = $state(false);

  const zulu = (s: number) => {
    const d = new Date(s * 1000);
    return `${String(d.getUTCHours()).padStart(2, "0")}:${String(d.getUTCMinutes()).padStart(2, "0")}Z`;
  };
</script>

{#snippet art(kind: string)}
  <svg viewBox="0 0 120 64" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
    <rect width="120" height="64" fill="#1b1c1e" />
    {#if kind === "3d"}
      <defs>
        <linearGradient id="la-3d" x1="0" x2="1" y1="0" y2="0">
          <stop offset="0" stop-color="#ef4b4b" /><stop offset="0.45" stop-color="#f6b73c" /><stop offset="1" stop-color="#34c98b" />
        </linearGradient>
      </defs>
      <path d="M8 54 L40 44 L80 40 L112 30 L112 12 L80 18 L40 28 L8 50 Z" fill="url(#la-3d)" opacity="0.75" />
      <path d="M8 54 L40 44 L80 40 L112 30" fill="none" stroke="url(#la-3d)" stroke-width="2" />
    {:else if kind === "night"}
      <path d="M0 0 H70 C 58 16, 58 48, 70 64 H0 Z" fill="#000" opacity="0.45" />
      <path d="M70 0 C 58 16, 58 48, 70 64" fill="none" stroke="#f0b23a" stroke-width="1.4" opacity="0.8" />
      <circle cx="96" cy="20" r="7" fill="#f6c453" opacity="0.9" />
      <circle cx="28" cy="20" r="1" fill="#fff" /><circle cx="44" cy="34" r="1" fill="#fff" /><circle cx="18" cy="44" r="1" fill="#fff" />
    {:else if kind === "satellite"}
      <rect width="120" height="64" fill="#2f3a2a" />
      <path d="M0 40 C 20 30, 40 46, 64 36 C 84 28, 100 40, 120 30 V64 H0 Z" fill="#4a5a38" />
      <path d="M0 18 C 18 10, 34 22, 56 14 C 70 9, 86 18, 120 8 V0 H0 Z" fill="#7c7158" opacity="0.8" />
      <path d="M58 64 C 62 50, 76 44, 70 30 C 66 20, 80 14, 88 0" fill="none" stroke="#2c4a5e" stroke-width="4" />
      <path d="M20 54 L44 34" stroke="#b9b2a0" stroke-width="2" />
    {:else if kind === "terrain"}
      <defs>
        <linearGradient id="la-ridge" x1="0" x2="1" y1="0" y2="1">
          <stop offset="0" stop-color="#d9dde2" /><stop offset="1" stop-color="#2a2d31" />
        </linearGradient>
      </defs>
      <path d="M0 64 L22 30 L34 42 L56 12 L74 36 L88 26 L120 58 V64 Z" fill="url(#la-ridge)" opacity="0.85" />
      <path d="M0 64 L30 46 L48 54 L70 40 L96 52 L120 46 V64 Z" fill="#3a3e43" />
    {:else if kind === "borders"}
      <path d="M0 22 L30 26 L44 10 L60 18 L82 8 L120 16 M44 10 L50 40 L36 64 M50 40 L84 44 L96 64 M84 44 L120 38" fill="none" stroke="#9aa0a8" stroke-width="1.6" />
    {:else if kind === "routes"}
      <path d="M12 50 Q 50 4, 104 30 M20 20 Q 60 50, 108 52 M10 36 Q 40 22, 70 14" fill="none" stroke="#c4c8ce" stroke-width="1.4" opacity="0.8" />
    {:else if kind === "airports"}
      <path d="M12 50 Q 50 4, 104 30" fill="none" stroke="#c4c8ce" stroke-width="1" opacity="0.4" />
      {#each [[12, 50], [104, 30], [60, 44], [34, 18], [86, 54]] as [x, y] (x)}
        <circle cx={x} cy={y} r="3.4" fill="#f0b23a" stroke="#1b1c1e" stroke-width="1.4" />
      {/each}
    {:else if kind === "aeroway"}
      <rect x="0" y="0" width="120" height="64" fill="#26282c" />
      <path d="M-4 48 L124 18" stroke="#5d6168" stroke-width="12" />
      <path d="M-4 48 L124 18" stroke="#e8eaed" stroke-width="1" stroke-dasharray="5 4" />
      <path d="M20 58 C 30 40, 40 36, 52 34 M70 64 C 76 46, 86 30, 96 6" fill="none" stroke="#d4b24a" stroke-width="1.6" />
      <text x="44" y="47" font-size="8" font-weight="700" fill="#1b1406" stroke="#e2c15a" stroke-width="2.4" paint-order="stroke">B</text>
      <text x="82" y="26" font-size="8" font-weight="700" fill="#1b1406" stroke="#e2c15a" stroke-width="2.4" paint-order="stroke">A</text>
    {:else if kind === "world"}
      {#each [[18, 16, 4], [70, 40, 4], [100, 14, 3], [40, 50, 2.4], [56, 22, 2.4], [88, 54, 2.4], [24, 36, 1.8], [108, 38, 1.8], [64, 8, 1.8], [10, 56, 1.8]] as [x, y, r] (`${x},${y}`)}
        <circle cx={x} cy={y} r={r} fill={r > 3 ? "#4f9cf0" : r > 2 ? "#5cc08a" : "#a98be0"} stroke="#111214" stroke-width="1" />
      {/each}
    {:else if kind === "vatsim-atc" || kind === "ivao-atc"}
      {@const c = kind === "vatsim-atc" ? "#5cc08a" : "#5aa0f0"}
      <path d="M14 12 L62 8 L80 30 L58 56 L18 50 Z" fill={c} fill-opacity="0.12" stroke={c} stroke-width="1.4" />
      <text x="40" y="34" text-anchor="middle" font-size="9" font-family="monospace" fill="#eef1f4">CTR</text>
      <circle cx="96" cy="22" r="13" fill={c} fill-opacity="0.08" stroke={c} stroke-width="1.2" />
      <circle cx="96" cy="22" r="3" fill={c} />
      <rect x="86" y="44" width="9" height="9" rx="2" fill="#d6453d" /><rect x="98" y="44" width="9" height="9" rx="2" fill="#2fae73" />
    {:else if kind === "vatsim-pilots" || kind === "ivao-pilots"}
      {@const c = kind === "vatsim-pilots" ? "#5cc08a" : "#5aa0f0"}
      {#each [[22, 20, 40], [60, 40, 120], [96, 18, 210], [40, 50, 300], [84, 50, 80], [104, 38, 160]] as [x, y, r] (`${x}${y}`)}
        <g transform="translate({x} {y}) rotate({r}) scale(0.32)"><path d="M0 -20 L3.4 -6 L19 1.5 L19 5.5 L3.4 2.6 L2.4 13 L8 17 L8 19.6 L0 17.8 L-8 19.6 L-8 17 L-2.4 13 L-3.4 2.6 L-19 5.5 L-19 1.5 L-3.4 -6 Z" fill={c} /></g>
      {/each}
    {:else if kind === "wx-cat"}
      {#each [[18, 18, "#2fbf71"], [48, 40, "#4f8fe8"], [80, 20, "#2fbf71"], [100, 46, "#e2453c"], [30, 50, "#cf5bd6"], [64, 14, "#2fbf71"]] as [x, y, c] (`${x}${y}`)}
        <circle cx={x} cy={y} r="6" fill={String(c)} fill-opacity="0.35" stroke={String(c)} stroke-width="2" />
      {/each}
    {:else if kind === "wx-adv"}
      <path d="M8 10 L56 6 L62 34 L20 44 Z" fill="#e2453c" fill-opacity="0.15" stroke="#e2453c" stroke-width="1.6" />
      <path d="M60 30 L110 22 L114 56 L66 58 Z" fill="#f0a03a" fill-opacity="0.12" stroke="#f0a03a" stroke-width="1.4" stroke-dasharray="4 3" />
      <path d="M78 6 L112 8 L106 20 L80 18 Z" fill="#5cc8e8" fill-opacity="0.15" stroke="#5cc8e8" stroke-width="1.3" stroke-dasharray="4 3" />
    {:else if kind === "parked"}
      {#each [[22, 22], [64, 42], [92, 18]] as [x, y] (x)}
        <g transform="translate({x} {y}) scale(0.32)"><path d="M0 -20 L3.4 -6 L19 1.5 L19 5.5 L3.4 2.6 L2.4 13 L8 17 L8 19.6 L0 17.8 L-8 19.6 L-8 17 L-2.4 13 L-3.4 2.6 L-19 5.5 L-19 1.5 L-3.4 -6 Z" fill="#c4c8ce" /></g>
        <rect x={x + 8} y={y - 4} width="18" height="8" rx="2" fill="#2e3135" stroke="#4a4e54" stroke-width="0.8" />
      {/each}
    {:else if kind === "wind"}
      {#each [[6, 14, "#4fb0c6"], [20, 30, "#49c27a"], [2, 46, "#c9d94a"], [44, 10, "#f0b23a"], [60, 40, "#ec6a3a"], [30, 54, "#49c27a"], [76, 22, "#e2453c"]] as [x, y, c] (`${x}${y}`)}
        <path d="M{x} {y} q 14 -6 28 0 t 28 0" fill="none" stroke={String(c)} stroke-width="1.6" stroke-linecap="round" opacity="0.85" />
      {/each}
    {:else if kind === "wx-wind"}
      {#each [[24, 32, 60, "#49c27a"], [64, 22, 200, "#4fb0c6"], [98, 40, 120, "#f0b23a"]] as [x, y, r, c] (`${x}${y}`)}
        <g transform="translate({x} {y}) rotate({r})">
          <path d="M0 13 V-9 M-5 -4 L0 -12 L5 -4 Z M-3 13 H3" fill={String(c)} stroke={String(c)} stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
        </g>
        <circle cx={x} cy={y} r="2" fill="#eef1f4" />
      {/each}
    {:else if kind === "radar"}
      <path d="M10 40 C 20 20, 44 18, 52 30 C 60 42, 40 56, 22 52 C 12 50, 6 46, 10 40 Z" fill="#3aa0ff" opacity="0.55" />
      <path d="M22 40 C 26 32, 38 30, 42 36 C 46 44, 32 48, 26 46 Z" fill="#7ce0ff" opacity="0.8" />
      <path d="M70 18 C 82 6, 108 10, 112 24 C 116 38, 92 44, 80 38 C 70 34, 64 26, 70 18 Z" fill="#3aa0ff" opacity="0.5" />
      <path d="M84 22 C 90 16, 102 18, 102 26 C 102 32, 92 34, 88 31 Z" fill="#fff36a" opacity="0.75" />
    {/if}
  </svg>
{/snippet}

{#snippet card(key: string, label: string, active: boolean, toggle: () => void, extra?: Snippet)}
  <button class="card" class:active role="switch" aria-checked={active} onclick={toggle}>
    <span class="thumb">{@render art(key)}</span>
    <span class="text">
      <span class="label">{label}</span>
      {#if extra}{@render extra()}{/if}
    </span>
    <span class="tick" aria-hidden="true">
      <svg viewBox="0 0 16 16"><path d="M3.5 8.5l3 3 6-7" /></svg>
    </span>
  </button>
{/snippet}

<!-- A network: one tile, with ATC and pilots switched on their own. -->
{#snippet netCard(net: "vatsim" | "ivao", name: string)}
  {@const atc = on.has(`${net}-atc`)}
  {@const pilots = on.has(`${net}-pilots`)}
  <div class="card net" class:active={atc || pilots}>
    <span class="thumb">{@render art(`${net}-atc`)}</span>
    <span class="text">
      <span class="label">{name}</span>
      <span class="chips">
        <button role="switch" aria-checked={atc} aria-label="{name} ATC" class:on={atc} onclick={() => ontoggle(`${net}-atc`)}>ATC</button>
        <button role="switch" aria-checked={pilots} aria-label="{name} pilots" class:on={pilots} onclick={() => ontoggle(`${net}-pilots`)}>Pilots</button>
      </span>
    </span>
  </div>
{/snippet}

{#snippet aerowayNote()}
  <span class="note">Zoom in on an airport</span>
{/snippet}

{#snippet windNote()}
  <span class="note">{on.has("wind") ? (WIND_LEVELS.find((l) => l.id === windLevel)?.label ?? "Auto") : "Moving flow"}</span>
{/snippet}

{#snippet radarNote()}
  <span class="note">
    {#if !on.has("radar")}Rain and snow{:else if radarError}Not reachable{:else if radarTime}{zulu(radarTime)}{:else}Loading…{/if}
  </span>
{/snippet}

<div class="rail">
  <div class="seg" role="radiogroup" aria-label="Map view">
    <button role="radio" aria-checked={!three} class:on={!three} onclick={() => onthree(false)}>2D</button>
    <button role="radio" aria-checked={three} class:on={three} onclick={() => onthree(true)}>3D</button>
  </div>
  <button class="layers-btn" class:open aria-expanded={open} onclick={() => (open = !open)}>
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3l9 5-9 5-9-5 9-5z" /><path d="M3 13l9 5 9-5" /></svg>
    <span>Layers</span>
  </button>
</div>

{#if open}
  <div class="panel" role="dialog" aria-label="Map layers">
    <header>
      <h3>Map layers</h3>
      <button class="x" aria-label="Close" onclick={() => (open = false)}>
        <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
      </button>
    </header>
    <h4>Map</h4>
    <div class="grid">
      {@render card("3d", "3D flight", three, () => onthree(!three))}
      {@render card("night", "Day / night", on.has("night"), () => ontoggle("night"))}
      {@render card("satellite", "Satellite", on.has("satellite"), () => ontoggle("satellite"))}
      {@render card("terrain", "Topographic", on.has("terrain"), () => ontoggle("terrain"))}
      {@render card("borders", "Strong borders", on.has("borders"), () => ontoggle("borders"))}
    </div>
    <h4>Weather</h4>
    <div class="grid">
      {@render card("radar", "Rain radar", on.has("radar"), () => ontoggle("radar"), radarNote)}
      {@render card("wx-cat", "Flight category", on.has("wx-cat"), () => ontoggle("wx-cat"))}
      {@render card("wx-adv", "SIGMET / AIRMET", on.has("wx-adv"), () => ontoggle("wx-adv"))}
      {@render card("wind", "Wind", on.has("wind"), () => ontoggle("wind"), windNote)}
      {@render card("wx-wind", "Airport wind", on.has("wx-wind"), () => ontoggle("wx-wind"))}
    </div>
    {#if on.has("wind")}
      <div class="levels" role="radiogroup" aria-label="Wind level">
        {#each [{ id: "auto", label: "Auto" }, ...WIND_LEVELS.map((l) => ({ id: l.id, label: l.id === "sfc" ? "SFC" : l.label }))] as l (l.id)}
          <button role="radio" aria-checked={windLevel === l.id} class:on={windLevel === l.id} onclick={() => onwindlevel?.(l.id)}>{l.label}</button>
        {/each}
      </div>
    {/if}
    {#if on.has("wx-cat")}
      <p class="legend">
        <span style:--c="#2fbf71">VFR</span><span style:--c="#4f8fe8">MVFR</span><span style:--c="#e2453c">IFR</span><span style:--c="#cf5bd6">LIFR</span>
      </p>
    {/if}
    <h4>Airports and flights</h4>
    <div class="grid">
      {@render card("world", "World airports", on.has("world"), () => ontoggle("world"))}
      {@render card("airports", "Your airports", on.has("airports"), () => ontoggle("airports"))}
      {@render card("routes", "Past routes", on.has("routes"), () => ontoggle("routes"))}
      {@render card("aeroway", "Runways and taxiways", !on.has("aeroway-off"), () => ontoggle("aeroway-off"), aerowayNote)}
      {#if parkedLayer}
        {@render card("parked", "Parked aircraft", on.has("parked"), () => ontoggle("parked"))}
      {/if}
    </div>
    {#if on.has("world")}
      <div class="detail">
        <label for="ap-detail">Airport detail <span>{DETAIL[airportDetail - 1]}</span></label>
        <input
          id="ap-detail"
          type="range"
          min="1"
          max="3"
          step="1"
          value={airportDetail}
          oninput={(e) => ondetail?.(Number((e.currentTarget as HTMLInputElement).value))}
        />
        <div class="ticks"><span>Major</span><span>Regional</span><span>All</span></div>
      </div>
    {/if}
    <h4>Online networks</h4>
    <div class="grid two">
      {@render netCard("vatsim", "VATSIM")}
      {@render netCard("ivao", "IVAO")}
    </div>
    <p class="credit">
      Radar RainViewer · METARs, SIGMETs and AIRMETs NOAA Aviation Weather Center (every 10 min while on) · Wind
      forecast NOAA GFS (one level, about 100 KB, every 3 h while on; drawn on the processor, not the graphics card) · Satellite Esri ·
      Terrain AWS Terrain Tiles · Live traffic VATSIM and IVAO (every 30 s while on) · ATC
      areas VAT-Spy Data Project (CC BY-SA 4.0)
    </p>
  </div>
{/if}

<style>
  .detail {
    margin: 8px 2px 0;
  }
  .detail label {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
    color: var(--ink);
  }
  .detail label span {
    color: var(--ink-dim);
  }
  .detail input {
    width: 100%;
    margin: 4px 0 0;
    accent-color: var(--amber);
  }
  .ticks {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--ink-faint);
  }
  .rail {
    position: absolute;
    top: 16px;
    right: 16px;
    display: flex;
    gap: 8px;
    z-index: 3;
  }
  .seg,
  .layers-btn {
    border-radius: 8px;
    background: rgba(28, 30, 33, 0.92);
    border: 1px solid var(--rule);
    backdrop-filter: blur(6px);
  }
  .seg {
    display: flex;
    padding: 3px;
    gap: 2px;
  }
  .seg button {
    border: none;
    background: transparent;
    color: var(--ink-dim);
    font-family: var(--cond);
    font-weight: 600;
    font-size: 15px;
    padding: 5px 12px;
    border-radius: 6px;
  }
  .seg button:hover {
    color: var(--ink);
  }
  .seg button.on {
    background: var(--amber);
    color: #1b1406;
  }
  .layers-btn {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 12px;
    color: var(--ink-dim);
    font-family: var(--cond);
    font-weight: 600;
    font-size: 15px;
  }
  .layers-btn svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linejoin: round;
  }
  .layers-btn:hover,
  .layers-btn.open {
    color: var(--ink);
    border-color: var(--amber);
  }

  .panel {
    position: absolute;
    top: 62px;
    right: 16px;
    width: 360px;
    max-height: calc(100% - 130px);
    overflow-y: auto;
    overflow-x: hidden;
    z-index: 6;
    padding: 14px 16px 12px;
    border-radius: 12px;
    background: rgba(28, 30, 33, 0.94);
    border: 1px solid var(--rule);
    box-shadow: 0 18px 40px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(8px);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h3 {
    margin: 0;
    font-family: var(--cond);
    font-weight: 600;
    font-size: 19px;
  }
  .x {
    border: none;
    background: transparent;
    color: var(--ink-dim);
    padding: 4px;
    display: grid;
  }
  .x svg {
    width: 18px;
    height: 18px;
    stroke: currentColor;
    stroke-width: 2;
  }
  .x:hover {
    color: var(--ink);
  }
  h4 {
    margin: 12px 0 6px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--ink-faint);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 7px;
  }
  /* Tiles: a short picture strip over the name, a tick when on. */
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 0;
    border-radius: 8px;
    overflow: hidden;
    border: 1.5px solid var(--rule);
    background: var(--panel-2);
    text-align: left;
    transition: border-color 0.15s, background 0.15s;
  }
  .card:hover {
    border-color: #555a61;
  }
  .card.active {
    border-color: var(--amber);
    background: rgba(240, 178, 58, 0.07);
  }
  .thumb {
    display: block;
    height: 30px;
    overflow: hidden;
  }
  .thumb :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }
  .text {
    display: grid;
    padding: 5px 7px 6px;
  }
  .label {
    font-size: 12px;
    font-weight: 600;
    line-height: 1.15;
  }
  .note {
    font-size: 10.5px;
    color: var(--ink-dim);
  }
  .tick {
    position: absolute;
    top: 5px;
    right: 5px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--amber);
    opacity: 0;
    transform: scale(0.6);
    transition: opacity 0.15s, transform 0.15s;
  }
  .tick svg {
    width: 11px;
    height: 11px;
    fill: none;
    stroke: #1b1406;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .card.active .tick {
    opacity: 1;
    transform: none;
  }
  .grid.two {
    grid-template-columns: 1fr 1fr;
  }
  .card.net .text {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }
  .chips {
    display: flex;
    gap: 4px;
  }
  .chips button {
    padding: 2px 8px;
    border: 1px solid var(--rule);
    border-radius: 999px;
    background: transparent;
    color: var(--ink-dim);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }
  .chips button:hover {
    border-color: #6a6f76;
    color: var(--ink);
  }
  .chips button.on {
    border-color: var(--amber);
    background: var(--amber);
    color: #1b1406;
  }
  .levels {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin: 8px 2px 0;
  }
  .levels button {
    padding: 2px 7px;
    border: 1px solid var(--rule);
    border-radius: 999px;
    background: transparent;
    color: var(--ink-dim);
    font: 600 11px "IBM Plex Mono", ui-monospace, monospace;
    cursor: pointer;
  }
  .levels button:hover {
    color: var(--ink);
    border-color: #6a6f76;
  }
  .levels button.on {
    border-color: var(--amber);
    background: var(--amber);
    color: #1b1406;
  }
  .legend {
    display: flex;
    gap: 12px;
    margin: 7px 2px 0;
    font-size: 11px;
    color: var(--ink-dim);
  }
  .legend span::before {
    content: "";
    display: inline-block;
    width: 8px;
    height: 8px;
    margin-right: 5px;
    border-radius: 50%;
    border: 2px solid var(--c);
    background: color-mix(in srgb, var(--c) 35%, transparent);
    vertical-align: -1px;
  }
  .credit {
    margin: 10px 0 0;
    font-size: 10.5px;
    line-height: 1.4;
    color: var(--ink-faint);
  }
</style>
