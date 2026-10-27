<script lang="ts">
  // The map's Layers panel: picture cards you switch on and off, grouped
  // into the view, your own flights and weather.
  import type { Snippet } from "svelte";

  interface Props {
    three: boolean;
    on: Set<string>;
    onthree: (v: boolean) => void;
    ontoggle: (key: string) => void;
    /** Time of the radar picture shown, Unix seconds, or null. */
    radarTime: number | null;
    radarError: boolean;
  }
  let { three, on, onthree, ontoggle, radarTime, radarError }: Props = $props();

  let open = $state(false);

  const zulu = (s: number) => {
    const d = new Date(s * 1000);
    return `${String(d.getUTCHours()).padStart(2, "0")}:${String(d.getUTCMinutes()).padStart(2, "0")}Z`;
  };
</script>

{#snippet art(kind: string)}
  <svg viewBox="0 0 120 64" aria-hidden="true">
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
    {:else if kind === "borders"}
      <path d="M0 22 L30 26 L44 10 L60 18 L82 8 L120 16 M44 10 L50 40 L36 64 M50 40 L84 44 L96 64 M84 44 L120 38" fill="none" stroke="#9aa0a8" stroke-width="1.6" />
    {:else if kind === "routes"}
      <path d="M12 50 Q 50 4, 104 30 M20 20 Q 60 50, 108 52 M10 36 Q 40 22, 70 14" fill="none" stroke="#c4c8ce" stroke-width="1.4" opacity="0.8" />
    {:else if kind === "airports"}
      <path d="M12 50 Q 50 4, 104 30" fill="none" stroke="#c4c8ce" stroke-width="1" opacity="0.4" />
      {#each [[12, 50], [104, 30], [60, 44], [34, 18], [86, 54]] as [x, y] (x)}
        <circle cx={x} cy={y} r="3.4" fill="#f0b23a" stroke="#1b1c1e" stroke-width="1.4" />
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
    {@render art(key)}
    <span class="label">{label}</span>
    {#if extra}{@render extra()}{/if}
    <span class="tick" aria-hidden="true">
      <svg viewBox="0 0 16 16"><path d="M3.5 8.5l3 3 6-7" /></svg>
    </span>
  </button>
{/snippet}

{#snippet radarNote()}
  <span class="note">
    {#if !on.has("radar")}Live rain and snow{:else if radarError}Could not reach RainViewer{:else if radarTime}Radar {zulu(radarTime)}{:else}Loading…{/if}
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
    <h4>View</h4>
    <div class="grid">
      {@render card("3d", "3D flight", three, () => onthree(!three))}
      {@render card("night", "Day / night", on.has("night"), () => ontoggle("night"))}
      {@render card("borders", "Strong borders", on.has("borders"), () => ontoggle("borders"))}
    </div>
    <h4>Your flights</h4>
    <div class="grid">
      {@render card("routes", "Past routes", on.has("routes"), () => ontoggle("routes"))}
      {@render card("airports", "Airports", on.has("airports"), () => ontoggle("airports"))}
    </div>
    <h4>Weather</h4>
    <div class="grid">
      {@render card("radar", "Rain radar", on.has("radar"), () => ontoggle("radar"), radarNote)}
    </div>
    <p class="credit">Radar by RainViewer, updated every 10 minutes.</p>
  </div>
{/if}

<style>
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
    width: 340px;
    max-height: calc(100% - 130px);
    overflow-y: auto;
    overflow-x: hidden;
    z-index: 3;
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
    margin: 14px 0 7px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--ink-faint);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 0;
    border-radius: 9px;
    overflow: hidden;
    border: 1.5px solid var(--rule);
    background: var(--panel-2);
    text-align: left;
    transition: border-color 0.15s, transform 0.15s;
  }
  .card:hover {
    border-color: #555a61;
    transform: translateY(-1px);
  }
  .card.active {
    border-color: var(--amber);
  }
  .card svg {
    display: block;
    width: 100%;
    height: auto;
  }
  .label {
    padding: 6px 8px 7px;
    font-size: 12.5px;
    font-weight: 600;
    line-height: 1.2;
  }
  .note {
    padding: 0 8px 7px;
    margin-top: -4px;
    font-size: 10.5px;
    color: var(--ink-dim);
  }
  .tick {
    position: absolute;
    top: 6px;
    right: 6px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--amber);
    opacity: 0;
    transform: scale(0.6);
    transition: opacity 0.15s, transform 0.15s;
  }
  .tick svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: #1b1406;
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .card.active .tick {
    opacity: 1;
    transform: none;
  }
  .credit {
    margin: 12px 0 0;
    font-size: 11px;
    color: var(--ink-faint);
  }
</style>
