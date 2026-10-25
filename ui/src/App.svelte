<script lang="ts">
  import { onMount } from "svelte";
  import { api, on, type FlightDetail, type Status } from "./lib/api";
  import FlightsPanel from "./lib/FlightsPanel.svelte";
  import LivePanel from "./lib/LivePanel.svelte";
  import Map from "./lib/Map.svelte";

  type View = "live" | "flights";
  let view = $state<View>("live");
  let status = $state<Status | null>(null);
  let liveTrack = $state<[number, number][]>([]);
  let history = $state<[number, number][] | null>(null);
  let refreshKey = $state(0);
  let toast = $state<string | null>(null);
  let completed = $state<FlightDetail | null>(null);
  let session = -1;

  const aircraft = $derived(
    status?.telemetry && status.live
      ? { lon: status.telemetry.lon, lat: status.telemetry.lat, hdg: status.telemetry.hdg_true_deg }
      : null,
  );
  const origin = $derived<[number, number] | null>(status?.live ? [status.live.dep_lon, status.live.dep_lat] : null);

  function show(msg: string) {
    toast = msg;
    setTimeout(() => (toast = null), 5000);
  }

  onMount(() => {
    const offs: Promise<() => void>[] = [];
    api.status().then((s) => {
      if (!s) return;
      status = s;
      session = s.session;
      liveTrack = s.track;
    });
    offs.push(
      on<Status>("status", (s) => {
        // A new flight starts with an empty track.
        // A new flight (or none) starts a fresh track; never join flights up.
        if (!s.live) liveTrack = [];
        else if (s.session !== session) liveTrack = [...s.track_tail];
        else if (s.track_tail.length) liveTrack = [...liveTrack, ...s.track_tail];
        session = s.session;
        // A new flight has begun: put the last result away.
        if (s.live && completed && s.live.start_ms > completed.flight.end_ms) dismiss();
        status = s;
      }),
    );
    offs.push(
      on<{ id: number; dep: string | null; arr: string | null }>("flight-saved", (f) => {
        refreshKey++;
        liveTrack = [];
        show(`Flight saved: ${f.dep ?? "unknown"} to ${f.arr ?? "unknown"}`);
        api.flight(f.id).then((d) => {
          completed = d;
          if (d && view === "live") history = d.track.map((p) => [p.lon, p.lat]);
        });
      }),
    );
    return () => offs.forEach((p) => p.then((off) => off()));
  });

  function dismiss() {
    completed = null;
    if (view === "live") history = null;
  }

  function go(v: View) {
    view = v;
    history = v === "live" && completed ? completed.track.map((p) => [p.lon, p.lat]) : null;
  }

  const linkText = $derived(
    !status || status.link === "waiting"
      ? "Waiting for a simulator"
      : status.link === "connecting"
        ? (status.notice ?? "Connecting")
        : `Connected to ${status.sim_label}${status.sim_version && !status.demo ? ` ${status.sim_version}` : ""}`,
  );
</script>

<div class="shell">
  <nav class="rail" aria-label="Main">
    <div class="mark" aria-hidden="true">
      <svg viewBox="0 0 1024 1024" width="30" height="30">
        <circle cx="512" cy="512" r="330" fill="none" stroke="#2E4658" stroke-width="60" />
        <path d="M190 700 C 360 520, 560 640, 820 330" fill="none" stroke="#E2468F" stroke-width="70" stroke-linecap="round" />
      </svg>
    </div>
    <button class:on={view === "live"} onclick={() => go("live")} aria-current={view === "live" ? "page" : undefined}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 6l6-2 6 2 6-2v14l-6 2-6-2-6 2zM9 4v14M15 6v14" /></svg>
      Map
    </button>
    <button class:on={view === "flights"} onclick={() => go("flights")} aria-current={view === "flights" ? "page" : undefined}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 4h14v16H5zM8 8h8M8 12h8M8 16h5" /></svg>
      Flights
    </button>
    <button disabled title="The passport book arrives in Milestone 4">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 3h11a1 1 0 011 1v16a1 1 0 01-1 1H6zM12 9a3 3 0 100 6 3 3 0 000-6zM9 18h6" /></svg>
      Passport
    </button>
    <button disabled title="The paper logbook arrives in Milestone 3">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 5h7a2 2 0 012 2v13a2 2 0 00-2-2H4zM20 5h-7a2 2 0 00-2 2v13a2 2 0 012-2h7z" /></svg>
      Logbook
    </button>
  </nav>

  <main class="stage">
    <Map live={liveTrack} {aircraft} {origin} {history} />
    <footer class="status-line">
      <span class="lamp {status?.link ?? 'waiting'}"></span>
      <span>{linkText}</span>
    </footer>
    {#if toast}
      <div class="toast" role="status">{toast}</div>
    {/if}
  </main>

  <aside class="panel">
    {#if view === "live"}
      <LivePanel {status} {completed} ondismiss={dismiss} />
    {:else}
      <FlightsPanel {refreshKey} onselect={(t) => (history = t)} />
    {/if}
  </aside>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 76px 1fr 380px;
    height: 100%;
  }
  .rail {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 4px;
    padding: 14px 6px;
    background: var(--panel);
    border-right: 1px solid var(--rule);
  }
  .mark {
    display: flex;
    justify-content: center;
    padding-bottom: 14px;
  }
  .rail button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 9px 2px 7px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--ink-dim);
    font-size: 12px;
  }
  .rail svg {
    width: 22px;
    height: 22px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .rail button:hover:not(:disabled) {
    color: var(--ink);
    background: var(--panel-2);
  }
  .rail button.on {
    color: var(--amber);
    background: var(--panel-2);
  }
  .rail button:disabled {
    color: var(--ink-faint);
    opacity: 0.55;
    cursor: default;
  }
  .stage {
    position: relative;
    overflow: hidden;
  }
  .status-line {
    position: absolute;
    left: 12px;
    bottom: 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 11px;
    border-radius: 6px;
    background: rgba(23, 36, 48, 0.92);
    border: 1px solid var(--rule);
    font-size: 13px;
    color: var(--ink-dim);
  }
  .lamp {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--ink-faint);
  }
  .lamp.connected {
    background: var(--good);
    box-shadow: 0 0 8px var(--good);
  }
  .lamp.connecting {
    background: var(--amber);
  }
  .toast {
    position: absolute;
    left: 50%;
    bottom: 18px;
    transform: translateX(-50%);
    padding: 10px 16px;
    border-radius: 6px;
    background: var(--strip);
    color: var(--strip-ink);
    font-weight: 600;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.35);
  }
  .panel {
    overflow-y: auto;
    padding: 22px 20px 28px;
    background: var(--panel);
    border-left: 1px solid var(--rule);
  }
</style>
