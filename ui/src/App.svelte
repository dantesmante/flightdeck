<script lang="ts">
  import { onMount } from "svelte";
  import { fly as slide } from "svelte/transition";
  import { draft, fly } from "./lib/flyStore.svelte";
  import { api, inRemote, on, type FlightDetail, type FlightPlan, type HangarItem, type Photo, type Route, type Status } from "./lib/api";
  import FlightsPanel from "./lib/FlightsPanel.svelte";
  import PhotosPanel from "./lib/PhotosPanel.svelte";
  import PfdWindow from "./lib/pfd/PfdWindow.svelte";
  import { pfdState } from "./lib/pfd/store.svelte";
  import { prefs } from "./lib/prefs.svelte";
  import HangarView from "./lib/hangar/HangarView.svelte";
  import MapDock from "./lib/MapDock.svelte";
  import ShotViewer from "./lib/ShotViewer.svelte";
  import { place, shots } from "./lib/shots.svelte";
  import { screen } from "./lib/phone.svelte";
  import LogbookView from "./lib/logbook/LogbookView.svelte";
  import Map from "./lib/Map.svelte";
  import PassportView from "./lib/passport/PassportView.svelte";
  import PilotView from "./lib/pilot/PilotView.svelte";
  import { loadPrefs } from "./lib/prefs.svelte";
  import SettingsView from "./lib/SettingsView.svelte";
  import UpdateBanner from "./lib/UpdateBanner.svelte";
  import Briefing from "./lib/Briefing.svelte";
  import TitleBar from "./lib/TitleBar.svelte";
  import { briefing, closeBriefing } from "./lib/briefingStore.svelte";
  import { fromTrack, type Pt } from "./lib/track";
  import { checkForUpdates } from "./lib/updates.svelte";

  type View = "live" | "flights" | "photos" | "logbook" | "passport" | "hangar" | "pilot" | "settings";
  let view = $state<View>("live");
  let status = $state<Status | null>(null);
  let liveTrack = $state<Pt[]>([]);
  let history = $state<Pt[] | null>(null);
  /** The flight plan of the saved flight shown, if it had one. */
  let historyPlan = $state<FlightPlan | null>(null);
  let refreshKey = $state(0);
  let toast = $state<string | null>(null);
  let completed = $state<FlightDetail | null>(null);
  let routes = $state<Route[]>([]);
  let selectedFlight = $state<number | null>(null);
  let hoverFlight = $state<number | null>(null);

  /** "Fly this aircraft" on a parked aircraft: open Start a flight with it. */
  function flyThis(item: HangarItem) {
    fly.item = item;
    if (view !== "live") go("live");
  }
  /** Room the map's card takes, for the map to keep clear of. */
  let inset = $state({ l: 0, b: 0 });
  const draftRoute = $derived(view === "live" && draft.origin && draft.dest ? { origin: draft.origin, dest: draft.dest, fixes: draft.fixes } : null);
  let openRequest = $state<{ id: number; n: number } | null>(null);

  let parked = $state<HangarItem[]>([]);
  // The aircraft being flown is on the map already; don't park it too.
  const parkedShown = $derived(
    status?.live && status.aircraft?.registration ? parked.filter((p) => p.registration !== status!.aircraft!.registration) : parked,
  );
  async function loadRoutes() {
    routes = (await api.routes()) ?? [];
    parked = (await api.hangar()) ?? [];
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

  const full = $derived(view !== "live" && view !== "flights" && view !== "photos");

  onMount(() => {
    loadPrefs();
    loadRoutes();
    // Updates are installed on the flight sim PC, not from other devices.
    if (!inRemote) checkForUpdates();
    const offs: Promise<() => void>[] = [];
    const fullStatus = () =>
      api.status().then((s) => {
        if (!s) return;
        status = s;
        session = s.session;
        liveTrack = s.track;
      });
    fullStatus();
    // Another device back online after a gap: catch up on everything.
    offs.push(
      on("reconnected", () => {
        fullStatus();
        loadRoutes();
        refreshKey++;
      }),
    );
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
      on<number>("flights-changed", () => {
        refreshKey++;
        loadRoutes();
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
          if (d && view === "live") {
            history = fromTrack(d.track);
            historyPlan = d.plan;
          }
        });
      }),
    );
    return () => offs.forEach((p) => p.then((off) => off()));
  });

  function dismiss() {
    completed = null;
    if (view === "live") {
      history = null;
      historyPlan = null;
    }
  }

  function go(v: View) {
    if ((v === "live" || v === "flights") && view !== "live" && view !== "flights") loadRoutes();
    if (v === "live") selectedFlight = null;
    menuOpen = false;
    // Coming to Flights from another page opens on the list, not on the
    // last flight shown (a route clicked on the map still opens its flight).
    if (v === "flights" && view !== "flights") {
      openRequest = null;
      selectedFlight = null;
    }
    // The flights list is a card over the map: bring it out if put away.
    if ((v === "flights" || v === "photos") && collapsed) togglePanel();
    view = v;
    history = v === "live" && completed ? fromTrack(completed.track) : null;
    historyPlan = v === "live" && completed ? completed.plan : null;
  }

  const NAV: { id: View; label: string; icon: string }[] = [
    { id: "live", label: "Map", icon: "M4 5.5A2.5 2.5 0 016.5 3H19a2 2 0 012 2v14a2 2 0 01-2 2H6.5A2.5 2.5 0 014 18.5zM4 18.5A2.5 2.5 0 016.5 16H7V3M11.4 7.6a1.3 1.3 0 11-2.6 0 1.3 1.3 0 012.6 0zM10.3 10.5v.01M11.4 12.5v.01M13.1 14v.01M15 15v.01M16.6 16.6l2 2M18.6 16.6l-2 2M15 3c.2 1.6-1 2.6.4 4S19 8.2 21 9.4" },
    { id: "flights", label: "Flights", icon: "M5 4h14v16H5zM8 8h8M8 12h8M8 16h5" },
    { id: "logbook", label: "Logbook", icon: "M4 5h7a2 2 0 012 2v13a2 2 0 00-2-2H4zM20 5h-7a2 2 0 00-2 2v13a2 2 0 012-2h7z" },
    { id: "hangar", label: "Hangar", icon: "M3 20V9l9-5 9 5v11M7 20v-6h10v6M3 20h18" },
    { id: "passport", label: "Passport", icon: "M6 3h11a1 1 0 011 1v16a1 1 0 01-1 1H6zM12 9a3 3 0 100 6 3 3 0 000-6zM9 18h6" },
    { id: "photos", label: "Photos", icon: "M3 7.5h3.5L8.5 5h7l2 2.5H21V19H3zM12 9.5a3.5 3.5 0 100 7 3.5 3.5 0 000-7z" },
  ];

  /** The flights card can be put away to see the whole map. */
  const PANEL_KEY = "fd.panelCollapsed";
  let collapsed = $state((() => {
    try {
      return localStorage.getItem(PANEL_KEY) === "1";
    } catch {
      return false;
    }
  })());
  function togglePanel() {
    collapsed = !collapsed;
    try {
      localStorage.setItem(PANEL_KEY, collapsed ? "1" : "0");
    } catch {
      /* not saved, still works */
    }
  }

  /** Phones: the page menu is hidden until the menu button opens it. */
  let menuOpen = $state(false);
  /** Screenshots on the map: the open flight's, or the flight under way's. */
  const mapShots = $derived(
    view === "flights" && history ? place(shots.flight, history) : view === "live" && status?.live ? place(shots.live, liveTrack) : [],
  );
  // Photos: every screenshot, read again when one is taken or deleted.
  let photoList = $state<Photo[]>([]);
  let photosLoading = $state(false);
  let hoverShot = $state<number | null>(null);
  function loadPhotos() {
    photosLoading = true;
    api.photos()
      .then((l) => (photoList = l))
      .catch(() => {})
      .finally(() => (photosLoading = false));
  }
  $effect(() => {
    void shots.deleted;
    if (view === "photos") loadPhotos();
  });
  onMount(() => {
    let off: (() => void) | undefined;
    let gone = false;
    on("screenshot", () => {
      if (view === "photos") loadPhotos();
    }).then((u) => (gone ? u() : (off = u)));
    return () => {
      gone = true;
      off?.();
    };
  });
  const photoPins = $derived(photoList.map((p) => ({ id: p.id, path: p.path, taken_ms: p.taken_ms, lat: p.lat, lon: p.lon, alt: p.alt_ft })));
  const FLIGHTS_LAYERS = ["routes", "airports"];
  const NO_LAYERS: string[] = [];
  const NO_TRACK: Pt[] = [];
  const NO_ROUTES: never[] = [];
  const ph = $derived(view === "photos");
  /** Split screen: the PFD on the right half of the Map tab, the map on the left. */
  const splitOn = $derived(pfdState.open && pfdState.split && view === "live" && !screen.phone);

  const linkText = $derived(
    !status || status.link === "waiting"
      ? "Waiting for a simulator"
      : status.link === "connecting"
        ? (status.notice ?? "Connecting")
        : `Connected to ${status.sim_label}${status.sim_version && !status.demo ? ` ${status.sim_version}` : ""}`,
  );
</script>

<div class="app">
<ShotViewer />
<TitleBar {linkText} link={status?.link ?? "waiting"} onmenu={() => (menuOpen = !menuOpen)} {menuOpen} />
<div class="shell" class:full class:menu-open={menuOpen}>
  {#if menuOpen}<button class="scrim" aria-label="Close menu" onclick={() => (menuOpen = false)}></button>{/if}
  <nav class="rail" aria-label="Main">
    {#each NAV as item (item.id)}
      <button class:on={view === item.id} onclick={() => go(item.id)} aria-current={view === item.id ? "page" : undefined}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d={item.icon} /></svg>
        {item.label}
      </button>
    {/each}
    <span class="grow"></span>
    <button class:on={view === "pilot"} onclick={() => go("pilot")} aria-current={view === "pilot" ? "page" : undefined}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 8.5c0-1.5 2.7-3.5 6-3.5s6 2 6 3.5H6zM7.2 8.5v1a4.8 4.8 0 009.6 0v-1M12 6.4v1.2M4.5 20.5c.6-3.6 3.6-5.5 7.5-5.5s6.9 1.9 7.5 5.5" /></svg>
      Pilot
    </button>
    <button class:on={view === "settings"} onclick={() => go("settings")} aria-current={view === "settings" ? "page" : undefined}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 15a3 3 0 100-6 3 3 0 000 6zM19.4 13.5l1.6 1.2-2 3.4-1.9-.7a7 7 0 01-2 1.2L14.8 21h-4l-.3-2a7 7 0 01-2-1.2l-1.9.7-2-3.4 1.6-1.2a7 7 0 010-2.6L4.6 9.9l2-3.4 1.9.7a7 7 0 012-1.2l.3-2h4l.3 2a7 7 0 012 1.2l1.9-.7 2 3.4-1.6 1.2a7 7 0 010 2.6z" /></svg>
      Settings
    </button>
  </nav>

  <main class="stage" class:hidden={full} class:split={splitOn}>
    <div class="mapside">
    <!-- One map for the Map, Flights and Photos tabs; in Photos it shows
         nothing but the screenshot pins. -->
    <Map live={ph ? NO_TRACK : liveTrack} aircraft={ph ? null : aircraft} origin={ph ? null : origin} history={ph ? null : history} routes={ph ? NO_ROUTES : routes} parked={ph ? NO_ROUTES : parkedShown} highlight={view === "flights" ? hoverFlight : null} onfly={status?.live || ph ? undefined : flyThis} selected={ph ? null : selectedFlight} onroute={showFlight} plan={ph ? null : history ? historyPlan : (status?.plan ?? null)} draft={ph ? null : draftRoute} insetLeft={view === "live" ? inset.l : (view === "flights" || ph) && !collapsed && !screen.phone ? 396 : 0} insetBottom={view === "live" ? inset.b : 0} initialOn={view === "flights" ? FLIGHTS_LAYERS : NO_LAYERS} shots={ph ? photoPins : mapShots} planner={view === "live"} bare={ph} shotAccent={ph ? hoverShot : null} onshothover={ph ? (id) => (hoverShot = id) : undefined} acAltFt={ph ? null : (status?.telemetry?.alt_msl_ft ?? null)} acOnGround={status?.telemetry?.on_ground ?? false} acPos={!ph && status?.telemetry ? [status.telemetry.lon, status.telemetry.lat] : null} />
    {#if view === "live"}
      <MapDock {status} {completed} track={liveTrack} ondismiss={dismiss} onsettings={() => go("settings")} oninset={(l, b) => (inset = { l, b })} />
    {:else if view === "flights"}
      {#if collapsed}
        <button class="flights-pill" onclick={togglePanel} transition:slide={{ x: -30, duration: 180 }}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 4h14v16H5zM8 8h8M8 12h8M8 16h5" /></svg>
          Show flights
        </button>
      {:else}
        <section class="flights-card" aria-label="Flights" transition:slide={{ x: -40, duration: 220 }}>
          <button class="tuck" onclick={togglePanel} aria-label="Put the flights card away" title="Hide">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
          </button>
          <FlightsPanel
            {refreshKey}
            request={openRequest}
            onhover={(id) => (hoverFlight = id)}
            onselect={(t, id, plan) => ((history = t), (selectedFlight = id), (historyPlan = plan ?? null))}
            ondelete={loadRoutes}
          />
        </section>
      {/if}
    {:else if view === "photos"}
      {#if collapsed}
        <button class="flights-pill" onclick={togglePanel} transition:slide={{ x: -30, duration: 180 }}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7.5h3.5L8.5 5h7l2 2.5H21V19H3zM12 9.5a3.5 3.5 0 100 7 3.5 3.5 0 000-7z" /></svg>
          Show photos
        </button>
      {:else}
        <section class="flights-card" aria-label="Photos" transition:slide={{ x: -40, duration: 220 }}>
          <button class="tuck" onclick={togglePanel} aria-label="Put the photos card away" title="Hide">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
          </button>
          <PhotosPanel list={photoList} hot={hoverShot} loading={photosLoading} hasFolder={!!prefs.s.screenshot_dir} onhover={(id) => (hoverShot = id)} onsettings={() => go("settings")} />
        </section>
      {/if}
    {/if}
    </div>
    {#if splitOn}<PfdWindow />{/if}
  </main>

  {#if full}
    <div class="fullview">
      {#if view === "logbook"}
        {#key refreshKey}<LogbookView onflight={showFlight} />{/key}
      {:else if view === "passport"}
        {#key refreshKey}<PassportView onsettings={() => go("settings")} />{/key}
      {:else if view === "pilot"}
        {#key refreshKey}<PilotView onflight={showFlight} onsettings={() => go("settings")} />{/key}
      {:else if view === "hangar"}
        {#key refreshKey}<HangarView onsettings={() => go("settings")} onflight={showFlight} />{/key}
      {:else}
        <SettingsView />
      {/if}
    </div>
  {/if}

  {#if toast}
    <div class="toast" role="status">{toast}</div>
  {/if}
  <UpdateBanner />
  {#if pfdState.open && view === "live" && !pfdState.split}<PfdWindow />{/if}
  {#if briefing.plan && briefing.ofp}
    <Briefing plan={briefing.plan} ofp={briefing.ofp} onclose={closeBriefing} />
  {/if}

</div>
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .shell {
    position: relative;
    display: grid;
    grid-template-columns: 76px 1fr;
    flex: 1;
    min-height: 0;
  }

  .hidden {
    display: none;
  }
  .scrim {
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
  /* Split screen: the map and its cards keep to the left half. */
  .mapside {
    position: absolute;
    inset: 0;
  }
  .stage.split .mapside {
    right: 50%;
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
  /* The flights list, floating over the left of the map, the map showing
     faintly through it. */
  .flights-card {
    position: absolute;
    z-index: 5;
    top: 16px;
    left: 16px;
    max-height: calc(100% - 32px);
    width: 380px;
    max-width: calc(100% - 32px);
    overflow-y: auto;
    scrollbar-width: thin;
    padding: 18px 18px 24px;
    background: rgba(28, 30, 33, 0.86);
    border: 1px solid var(--rule);
    border-radius: 14px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5);
  }
  .tuck {
    position: absolute;
    top: 10px;
    right: 10px;
    z-index: 2;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--ink-dim);
  }
  .tuck:hover {
    color: var(--ink);
    background: var(--panel-2);
  }
  .tuck svg,
  .flights-pill svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .flights-pill {
    position: absolute;
    z-index: 5;
    top: 16px;
    left: 16px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 14px;
    border-radius: 10px;
    border: 1px solid var(--rule);
    background: rgba(28, 30, 33, 0.95);
    color: var(--amber);
    font-weight: 600;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }
  /* Phones: the page menu slides in over the page from the menu button, and
     the map's own cards take the full width. */
  @media (max-width: 640px) {
    .shell {
      grid-template-columns: 1fr;
    }
    .rail {
      position: absolute;
      z-index: 40;
      top: 0;
      bottom: 0;
      left: 0;
      width: 92px;
      overflow-y: auto;
      transform: translateX(-102%);
      transition: transform 0.22s ease;
      box-shadow: 8px 0 24px rgba(0, 0, 0, 0.45);
    }
    .shell.menu-open .rail {
      transform: none;
    }
    .scrim {
      display: block;
      position: absolute;
      inset: 0;
      z-index: 39;
      border: none;
      padding: 0;
      background: rgba(0, 0, 0, 0.45);
    }
    .flights-card {
      top: 62px;
      left: 10px;
      right: 10px;
      width: auto;
      max-width: none;
      max-height: 60%;
      padding: 14px 14px 20px;
    }
    .flights-pill {
      top: 62px;
      left: 10px;
    }
  }
</style>
