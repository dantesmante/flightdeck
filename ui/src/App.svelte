<script lang="ts">
  import { onMount } from "svelte";
  import { api, on, type FlightDetail, type Route, type Status } from "./lib/api";
  import FlightsPanel from "./lib/FlightsPanel.svelte";
  import HangarView from "./lib/hangar/HangarView.svelte";
  import LivePanel from "./lib/LivePanel.svelte";
  import LogbookView from "./lib/logbook/LogbookView.svelte";
  import Map from "./lib/Map.svelte";
  import PassportView from "./lib/passport/PassportView.svelte";
  import { loadPrefs } from "./lib/prefs.svelte";
  import SettingsView from "./lib/SettingsView.svelte";
  import UpdateBanner from "./lib/UpdateBanner.svelte";
  import { fromTrack, type Pt } from "./lib/track";
  import { checkForUpdates } from "./lib/updates.svelte";

  type View = "live" | "flights" | "logbook" | "passport" | "hangar" | "settings";
  let view = $state<View>("live");
  let status = $state<Status | null>(null);
  let liveTrack = $state<Pt[]>([]);
  let history = $state<Pt[] | null>(null);
  let refreshKey = $state(0);
  let toast = $state<string | null>(null);
  let completed = $state<FlightDetail | null>(null);
  let routes = $state<Route[]>([]);
  let selectedFlight = $state<number | null>(null);
  let openRequest = $state<{ id: number; n: number } | null>(null);

  async function loadRoutes() {
    routes = (await api.routes()) ?? [];
  }

  /** A route on the map was clicked: show that flight. */
  function showFlight(id: number) {
    if (full) loadRoutes();
    view = "flights";
    openRequest = { id, n: (openRequest?.n ?? 0) + 1 };
  }
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

  const full = $derived(view !== "live" && view !== "flights");

  onMount(() => {
    loadPrefs();
    loadRoutes();
    checkForUpdates();
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
        loadRoutes();
        show(`Flight saved: ${f.dep ?? "unknown"} to ${f.arr ?? "unknown"}`);
        api.flight(f.id).then((d) => {
          completed = d;
          if (d && view === "live") history = fromTrack(d.track);
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
    if ((v === "live" || v === "flights") && view !== "live" && view !== "flights") loadRoutes();
    if (v === "live") selectedFlight = null;
    view = v;
    history = v === "live" && completed ? fromTrack(completed.track) : null;
  }

  const NAV: { id: View; label: string; icon: string }[] = [
    { id: "live", label: "Map", icon: "M3 6l6-2 6 2 6-2v14l-6 2-6-2-6 2zM9 4v14M15 6v14" },
    { id: "flights", label: "Flights", icon: "M5 4h14v16H5zM8 8h8M8 12h8M8 16h5" },
    { id: "logbook", label: "Logbook", icon: "M4 5h7a2 2 0 012 2v13a2 2 0 00-2-2H4zM20 5h-7a2 2 0 00-2 2v13a2 2 0 012-2h7z" },
    { id: "passport", label: "Passport", icon: "M6 3h11a1 1 0 011 1v16a1 1 0 01-1 1H6zM12 9a3 3 0 100 6 3 3 0 000-6zM9 18h6" },
    { id: "hangar", label: "Hangar", icon: "M3 20V9l9-5 9 5v11M7 20v-6h10v6M3 20h18" },
  ];

  const linkText = $derived(
    !status || status.link === "waiting"
      ? "Waiting for a simulator"
      : status.link === "connecting"
        ? (status.notice ?? "Connecting")
        : `Connected to ${status.sim_label}${status.sim_version && !status.demo ? ` ${status.sim_version}` : ""}`,
  );
</script>

<div class="shell" class:full>
  <nav class="rail" aria-label="Main">
    <div class="mark" aria-hidden="true">
      <svg viewBox="0 0 1024 1024" width="30" height="30">
        <circle cx="512" cy="512" r="330" fill="none" stroke="#2E4658" stroke-width="60" />
        <path d="M190 700 C 360 520, 560 640, 820 330" fill="none" stroke="#E2468F" stroke-width="70" stroke-linecap="round" />
      </svg>
    </div>
    {#each NAV as item (item.id)}
      <button class:on={view === item.id} onclick={() => go(item.id)} aria-current={view === item.id ? "page" : undefined}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d={item.icon} /></svg>
        {item.label}
      </button>
    {/each}
    <span class="grow"></span>
    <button class:on={view === "settings"} onclick={() => go("settings")} aria-current={view === "settings" ? "page" : undefined}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 15a3 3 0 100-6 3 3 0 000 6zM19.4 13.5l1.6 1.2-2 3.4-1.9-.7a7 7 0 01-2 1.2L14.8 21h-4l-.3-2a7 7 0 01-2-1.2l-1.9.7-2-3.4 1.6-1.2a7 7 0 010-2.6L4.6 9.9l2-3.4 1.9.7a7 7 0 012-1.2l.3-2h4l.3 2a7 7 0 012 1.2l1.9-.7 2 3.4-1.6 1.2a7 7 0 010 2.6z" /></svg>
      Settings
    </button>
  </nav>

  <main class="stage" class:hidden={full}>
    <Map live={liveTrack} {aircraft} {origin} {history} {routes} selected={selectedFlight} onroute={showFlight} />
    <footer class="status-line">
      <span class="lamp {status?.link ?? 'waiting'}"></span>
      <span>{linkText}</span>
    </footer>
  </main>

  {#if full}
    <div class="fullview">
      {#if view === "logbook"}
        {#key refreshKey}<LogbookView onflight={showFlight} />{/key}
      {:else if view === "passport"}
        {#key refreshKey}<PassportView onsettings={() => go("settings")} />{/key}
      {:else if view === "hangar"}
        {#key refreshKey}<HangarView onsettings={() => go("settings")} />{/key}
      {:else}
        <SettingsView />
      {/if}
    </div>
  {/if}

  {#if toast}
    <div class="toast" role="status">{toast}</div>
  {/if}
  <UpdateBanner />

  <aside class="panel" class:hidden={full}>
    {#if view === "live"}
      <LivePanel {status} {completed} track={liveTrack} ondismiss={dismiss} />
    {:else}
      <FlightsPanel
        {refreshKey}
        request={openRequest}
        onselect={(t, id) => ((history = t), (selectedFlight = id))}
        ondelete={loadRoutes}
      />
    {/if}
  </aside>
</div>

<style>
  .shell {
    position: relative;
    display: grid;
    grid-template-columns: 76px 1fr 380px;
    height: 100%;
  }
  .shell.full {
    grid-template-columns: 76px 1fr;
  }
  .hidden {
    display: none;
  }
  .fullview {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .grow {
    flex: 1;
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
    z-index: 30;
    left: calc(50% + 38px);
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
