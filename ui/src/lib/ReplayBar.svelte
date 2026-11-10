<script lang="ts" module>
  /** 1X: a minute of the flight plays in 5 seconds (12 times real time). */
  export const REPLAY_BASE = 12;
  export const REPLAY_SPEEDS = [1, 2, 5, 10, 20, 30];
</script>

<script lang="ts">
  // The replay controls for a past flight: play or pause, speed, and a
  // progress bar whose little plane can be dragged to any moment.
  interface Props {
    /** Flight time now, and the flight's start and end (ms). */
    t: number;
    start: number;
    end: number;
    playing: boolean;
    speed: number;
    /** Altitude (ft) and ground speed (kt) now. */
    alt: number;
    gs: number;
    onplay: () => void;
    onseek: (t: number, done: boolean) => void;
    onspeed: (s: number) => void;
    onclose: () => void;
  }
  let { t, start, end, playing, speed, alt, gs, onplay, onseek, onspeed, onclose }: Props = $props();

  let bar = $state<HTMLDivElement>();
  let dragging = $state(false);
  const frac = $derived(end > start ? Math.min(1, Math.max(0, (t - start) / (end - start))) : 0);

  function hm(ms: number): string {
    const m = Math.max(0, Math.round(ms / 60_000));
    return `${Math.floor(m / 60)}:${String(m % 60).padStart(2, "0")}`;
  }
  const clock = $derived(new Date(t).toISOString().slice(11, 16) + "Z");

  function at(e: PointerEvent): number {
    const r = bar!.getBoundingClientRect();
    return start + Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * (end - start);
  }
  function down(e: PointerEvent) {
    dragging = true;
    bar!.setPointerCapture(e.pointerId);
    onseek(at(e), false);
  }
  function move(e: PointerEvent) {
    if (dragging) onseek(at(e), false);
  }
  function up(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    onseek(at(e), true);
  }
  function key(e: KeyboardEvent) {
    const step = (end - start) / 100;
    if (e.key === "ArrowRight") onseek(Math.min(end, t + step), true);
    else if (e.key === "ArrowLeft") onseek(Math.max(start, t - step), true);
    else if (e.key === " ") onplay();
    else return;
    e.preventDefault();
  }
</script>

<div class="replay" role="group" aria-label="Flight replay">
  <div class="row">
    <button class="play" onclick={onplay} aria-label={playing ? "Pause" : "Play"}>
      {#if playing}
        <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true"><rect x="3" y="2" width="3.6" height="12" rx="1" /><rect x="9.4" y="2" width="3.6" height="12" rx="1" /></svg>
      {:else}
        <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true"><path d="M4 2.2v11.6c0 .6.7 1 1.2.7l9.2-5.8a.8.8 0 0 0 0-1.4L5.2 1.5C4.7 1.2 4 1.6 4 2.2z" /></svg>
      {/if}
    </button>
    <div class="speeds" role="radiogroup" aria-label="Replay speed">
      {#each REPLAY_SPEEDS as s (s)}
        <button role="radio" aria-checked={speed === s} class:on={speed === s} onclick={() => onspeed(s)}>{s}X</button>
      {/each}
    </div>
    <span class="read"><b>{Math.round(alt).toLocaleString()}</b> ft · <b>{Math.round(gs)}</b> kt</span>
    <button class="x" onclick={onclose} aria-label="Close replay">×</button>
  </div>
  <div class="row">
    <span class="time">{hm(t - start)}</span>
    <div
      class="bar"
      bind:this={bar}
      role="slider"
      tabindex="0"
      aria-label="Replay position"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(frac * 100)}
      aria-valuetext="{hm(t - start)} of {hm(end - start)}"
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={up}
      onkeydown={key}
    >
      <span class="fill" style:width="{frac * 100}%"></span>
      <span class="thumb" class:dragging style:left="{frac * 100}%">
        <svg viewBox="-24 -24 48 48" width="22" height="22" aria-hidden="true"><path transform="rotate(90)" d="M0 -20 L3.4 -6 L19 1.5 L19 5.5 L3.4 2.6 L2.4 13 L8 17 L8 19.6 L0 17.8 L-8 19.6 L-8 17 L-2.4 13 L-3.4 2.6 L-19 5.5 L-19 1.5 L-3.4 -6 Z" fill="#F0B23A" stroke="#111214" stroke-width="2" stroke-linejoin="round" /></svg>
      </span>
    </div>
    <span class="time">{hm(end - start)}</span>
    <span class="clock">{clock}</span>
  </div>
</div>

<style>
  .replay {
    display: grid;
    gap: 6px;
    padding: 9px 12px 10px;
    background: color-mix(in srgb, var(--panel) 94%, transparent);
    border: 1px solid var(--rule);
    border-radius: 12px;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(6px);
    pointer-events: auto;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
  }
  .play {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    border: 0;
    display: grid;
    place-items: center;
    background: var(--accent, #f0b23a);
    color: #111214;
    fill: currentColor;
    cursor: pointer;
    flex: none;
  }
  .speeds {
    display: flex;
    gap: 3px;
    min-width: 0;
    overflow-x: auto;
  }
  .speeds button {
    border: 1px solid var(--rule);
    background: transparent;
    color: var(--ink-dim);
    border-radius: 999px;
    padding: 3px 8px;
    font-size: 11.5px;
    font-family: "IBM Plex Mono", monospace;
    cursor: pointer;
  }
  .speeds button.on {
    background: var(--ink);
    color: var(--panel);
    border-color: var(--ink);
  }
  .read {
    margin-left: auto;
    font-size: 12px;
    color: var(--ink-dim);
    white-space: nowrap;
  }
  .read b {
    color: var(--ink);
    font-weight: 600;
  }
  .x {
    border: 0;
    background: none;
    color: var(--ink-dim);
    font-size: 19px;
    line-height: 1;
    cursor: pointer;
    padding: 0 2px;
  }
  .time,
  .clock {
    font-family: "IBM Plex Mono", monospace;
    font-size: 11px;
    color: var(--ink-dim);
    flex: none;
  }
  .bar {
    position: relative;
    flex: 1;
    height: 22px;
    cursor: pointer;
    touch-action: none;
    outline: none;
  }
  .bar::before {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 4px;
    margin-top: -2px;
    border-radius: 2px;
    background: var(--rule);
  }
  .fill {
    position: absolute;
    left: 0;
    top: 50%;
    height: 4px;
    margin-top: -2px;
    border-radius: 2px;
    background: #f0b23a;
  }
  .thumb {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    display: grid;
    place-items: center;
    cursor: grab;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.6));
  }
  .thumb.dragging {
    cursor: grabbing;
    transform: translate(-50%, -50%) scale(1.15);
  }
  .bar:focus-visible .thumb {
    outline: 2px solid var(--ink);
    border-radius: 50%;
  }
  @media (max-width: 560px) {
    .clock {
      display: none;
    }
  }
</style>
