<script lang="ts">
  // The map's menus beside 2D/3D: Airports & Flights, Weather and Layers,
  // each a panel of picture cards you switch on and off.
  import type { Snippet } from "svelte";
  import { WIND_LEVELS } from "./wind/wind";
  import { ICING_ADV, ICING_BANDS, icingSwatch } from "./icingLayer";
  import { NAV_AIDS_ENABLED } from "./navLayer";

  interface Props {
    /** Airspace filters on: "us" US only, "nomr" no MOAs and restricted areas, "noaw" no airways. */
    aspFilter?: Set<string>;
    onaspfilter?: (key: string) => void;
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
    /** The level for wind, temperature and icing: "auto" or a level id ("850"). */
    level?: string;
    /** What Auto means now ("FL240"). */
    levelNow?: string;
    onlevel?: (l: string) => void;
    /** A card is open on the right (airport, network, weather): menus open beside it. */
    cardOpen?: boolean;
    /** The measuring tool is on. Leave out onmeasure to not offer it. */
    measuring?: boolean;
    onmeasure?: () => void;
    /** The US airspace layer's data: loading, ready or not reachable. */
    airspaceState?: "idle" | "loading" | "ok" | "error";
    /** Outside the US (openAIP): "Loading…" or a problem, else empty. */
    airspaceNote?: string;
    /** The PFD window: open, and the button to open or close it. */
    pfdOpen?: boolean;
    onpfd?: () => void;
    padOpen?: boolean;
    onpad?: () => void;
    markupOpen?: boolean;
    onmarkup?: () => void;
    /** The map's width: narrow maps show the tools as icons, or in a list. */
    width?: number;
    /** The layers panel opened or closed (other cards make room for it). */
    onpanel?: (open: boolean) => void;
    obstaclesState?: "idle" | "loading" | "ok" | "error";
    /** A sim is connected: terrain and obstacles can be coloured against it. */
    hasAircraft?: boolean;
    /** Roads' opacity, 0.15 to 1. */
    roadsOpacity?: number;
    /** Range rings: how many, and their spacing ("auto", "5", "10", "15" nm). */
    /** The flight profile under the map: open, and its button (Map tab only). */
    profileOpen?: boolean;
    onprofile?: () => void;
    taxiOpen?: boolean;
    ontaxi?: () => void;
    /** Under the Nav aids card ("12,345 from MSFS", "Needs MSFS"…). */
    navNote?: string;
    /** Under the Temperature card. */
    tempNote?: string;
    icingNote?: string;
    icingInfo?: string;
    icingAdvNote?: string;
    cloudNote?: string;
    visNote?: string;
    ringCount?: number;
    ringSet?: string;
    onrings?: (count: number, set: string) => void;
    onroadsopacity?: (o: number) => void;
  }
  let { aspFilter = new Set(["noaw"]), onaspfilter, three, on, onthree, ontoggle, radarTime, radarError, parkedLayer = false, airportDetail = 1, ondetail, level = "auto", levelNow = "", onlevel, cardOpen = false, measuring = false, onmeasure, airspaceState = "idle", airspaceNote = "", pfdOpen = false, onpfd, padOpen = false, onpad, markupOpen = false, onmarkup, width = 0, onpanel, obstaclesState = "idle", hasAircraft = false, roadsOpacity = 1, onroadsopacity, ringCount = 2, ringSet = "auto", onrings, profileOpen = false, onprofile, taxiOpen = false, ontaxi, navNote = "", tempNote = "", icingNote = "", icingInfo = "", icingAdvNote = "", cloudNote = "", visNote = "" }: Props = $props();
  /** The Tools drawer (Measure, Taxi, Profile, PFD): stays out until closed. */
  const TOOLS_KEY = "fd.toolsOpen";
  let toolsOpen = $state(
    (() => {
      try {
        return localStorage.getItem(TOOLS_KEY) === "1";
      } catch {
        return false;
      }
    })(),
  );
  function toggleTools() {
    toolsOpen = !toolsOpen;
    open = null;
    try {
      localStorage.setItem(TOOLS_KEY, toolsOpen ? "1" : "0");
    } catch {
      /* not remembered */
    }
  }
  const levelLabel = $derived(level === "auto" ? levelNow : WIND_LEVELS.find((l) => l.id === level)?.label);
  const anyTool = $derived(measuring || taxiOpen || profileOpen || pfdOpen || padOpen || markupOpen);
  const hasTools = $derived(!!(onmeasure || ontaxi || onprofile || onpfd || onpad || onmarkup));
  /** Narrow map (split with the PFD, a tablet): icons only. Phone-narrow: the tools drop down as a list. */
  const compact = $derived(width > 0 && width < 1100);
  const stack = $derived(width > 0 && width < 680);

  /** The Layers panel's tabs: airports and flights, weather, the map itself. */
  type Menu = "traffic" | "weather" | "map";
  const TABS: { id: Menu; label: string; title: string; icon: string }[] = [
    {
      id: "traffic",
      label: "Aviation",
      title: "Aviation",
      icon: "M2.5 13.5l7-1.5 5-8h2.5l-2.5 8 5.5-.5 1.8-2.5H23l-1.2 4 1.2 4h-1.2L20 14.5l-5.5-.5 2.5 8h-2.5l-5-8-7-1.5z",
    },
    { id: "weather", label: "Weather", title: "Weather", icon: "M7 18h10a4 4 0 0 0 .6-7.96A6 6 0 0 0 6.1 9.1 4.5 4.5 0 0 0 7 18z" },
    { id: "map", label: "Map", title: "Map layers", icon: "M12 3l9 5-9 5-9-5 9-5zM3 13l9 5 9-5" },
  ];
  /** The layers in each tab, to mark a tab that has something on. */
  const IN_TAB: Record<Menu, string[]> = {
    traffic: ["world", "airports", "routes", "airspace", "navaids", "parked", "vatsim-atc", "vatsim-pilots", "ivao-atc", "ivao-pilots"],
    weather: ["radar", "wx-cat", "wx-adv", "wind", "wx-wind", "temp", "icing", "cloud", "vis"],
    map: ["night", "satellite", "terrain", "borders", "roads", "taws", "obstacles", "rings"],
  };
  const tabOn = (t: Menu) => IN_TAB[t].some((k) => on.has(k)) || (t === "traffic" && !on.has("aeroway-off")) || (t === "map" && three);
  /** The panel is open on this tab. */
  let open = $state<Menu | null>(null);
  $effect(() => {
    onpanel?.(open != null);
  });
  // The panel opens on the tab used last (this device only).
  const TAB_KEY = "fd.layersTab";
  let lastTab: Menu = (() => {
    try {
      const v = localStorage.getItem(TAB_KEY);
      return v === "traffic" || v === "weather" || v === "map" ? v : "traffic";
    } catch {
      return "traffic";
    }
  })();
  function showTab(t: Menu) {
    open = t;
    lastTab = t;
    try {
      localStorage.setItem(TAB_KEY, t);
    } catch {
      /* private window: just not remembered */
    }
  }
  function togglePanel() {
    if (open) open = null;
    else showTab(lastTab);
  }

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
    {:else if kind === "taws"}
      <rect width="120" height="70" fill="#101113" />
      <path d="M0 70 L0 46 Q 20 30 34 40 T 64 26 T 96 34 T 120 22 L120 70 Z" fill="#ecc428" opacity="0.75" />
      <path d="M40 70 L 48 44 Q 58 30 66 36 L 72 50 L 80 70 Z" fill="#e22d28" opacity="0.85" />
      <path d="M58 64 l 6 -10 l 6 10 z" fill="#fff" />
    {:else if kind === "obstacles"}
      <g stroke="#fff" stroke-width="2" fill="none" stroke-linecap="round" stroke-linejoin="round">
        <path d="M30 56 L36 30 L42 56" /><path d="M70 56 L78 18 L86 56" stroke="#ecc428" />
        <path d="M71 14 l-4 -3 M85 14 l4 -3" stroke="#ecc428" stroke-width="1.4" />
        <path d="M100 56 L104 40 L108 56" />
      </g>
      <circle cx="36" cy="53" r="1.6" fill="#fff" /><circle cx="78" cy="53" r="1.8" fill="#ecc428" /><circle cx="104" cy="53" r="1.4" fill="#fff" />
    {:else if kind === "rings"}
      <rect width="120" height="70" fill="#1b2a1f" />
      <g fill="none" stroke="#fff" stroke-width="1.4" opacity="0.9">
        <circle cx="60" cy="40" r="14" /><circle cx="60" cy="40" r="30" />
      </g>
      <circle cx="60" cy="40" r="4" fill="#4aa3ff" stroke="#fff" stroke-width="1.2" />
      <text x="34" y="20" fill="#fff" font-size="8" font-weight="700" transform="rotate(-45 34 20)">20NM</text>
    {:else if kind === "roads"}
      <path d="M-4 54 C 30 44, 60 40, 124 30" fill="none" stroke="#15171a" stroke-width="7" />
      <path d="M-4 54 C 30 44, 60 40, 124 30" fill="none" stroke="#f08c3c" stroke-width="4" />
      <path d="M40 -4 C 44 20, 50 40, 70 70" fill="none" stroke="#e8c55c" stroke-width="2.6" />
      <path d="M0 18 L 120 8" fill="none" stroke="#a4abb3" stroke-width="1.6" />
      <path d="M58 26 q 7 -2 14 0 v 7 q -1 7 -7 10 q -6 -3 -7 -10 z" fill="#1d4fa0" stroke="#fff" stroke-width="1" />
      <path d="M58.4 26.2 q 6.6 -1.8 13.2 0 v 3.6 h -13.2 z" fill="#c8102e" />
      <text x="65" y="38.5" text-anchor="middle" font-size="6.5" font-weight="700" fill="#fff">94</text>
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
    {:else if kind === "airspace"}
      <circle cx="56" cy="34" r="27" fill="#4a8ce8" fill-opacity="0.08" stroke="#4a8ce8" stroke-width="2" />
      <circle cx="56" cy="34" r="13" fill="none" stroke="#4a8ce8" stroke-width="2" />
      <circle cx="102" cy="16" r="10" fill="none" stroke="#c25bb0" stroke-width="1.6" />
      <circle cx="98" cy="50" r="9" fill="none" stroke="#4a8ce8" stroke-width="1.3" stroke-dasharray="3 2" />
      <text x="56" y="11" text-anchor="middle" font-size="7.5" font-weight="700" fill="#4a8ce8">100/30</text>
    {:else if kind === "navaids"}
      <rect width="120" height="64" fill="#15181c" />
      <path d="M14 50 L52 22 L100 40" fill="none" stroke="#5a6068" stroke-width="1" />
      <g transform="translate(52 22)" fill="#14181e" stroke="#5aa9ff" stroke-width="1.6">
        <rect x="-9" y="-7.5" width="18" height="15" /><path d="M6 0 L3 5.2 L-3 5.2 L-6 0 L-3 -5.2 L3 -5.2 Z" />
      </g>
      <circle cx="52" cy="22" r="1.4" fill="#5aa9ff" />
      <text x="52" y="40" text-anchor="middle" font-size="7" font-weight="700" fill="#5aa9ff">BTG</text>
      {#each [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] as i (i)}
        <circle cx={100 + 7 * Math.cos((i * Math.PI) / 5)} cy={40 + 7 * Math.sin((i * Math.PI) / 5)} r="0.9" fill="#c98bff" />
      {/each}
      <circle cx="100" cy="40" r="1.6" fill="#c98bff" />
      <path d="M14 45 L18 52 L10 52 Z" fill="#14181e" stroke="#c9ced6" stroke-width="1.2" />
      <text x="14" y="60" text-anchor="middle" font-size="5.5" fill="#c9ced6">HAMUR</text>
    {:else if kind === "icing"}
      <defs>
        <pattern id="la-ice1" width="7" height="7" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><path d="M0 0V7" stroke="#a9d8ff" stroke-width="1.4" /></pattern>
        <pattern id="la-ice2" width="7" height="7" patternUnits="userSpaceOnUse" patternTransform="rotate(-45)"><path d="M0 0V7" stroke="#8a3cf0" stroke-width="1.6" /></pattern>
      </defs>
      <rect width="120" height="64" fill="#15181c" />
      <ellipse cx="58" cy="34" rx="44" ry="22" fill="url(#la-ice1)" stroke="#a9d8ff" stroke-width="0.8" />
      <ellipse cx="62" cy="34" rx="20" ry="10" fill="url(#la-ice2)" stroke="#8a3cf0" stroke-width="0.8" />
      <g stroke="#fff" stroke-width="1.2" stroke-linecap="round" opacity="0.9" transform="translate(98 14)">
        <path d="M0 -7V7M-6 -3.5L6 3.5M-6 3.5L6 -3.5" />
      </g>
    {:else if kind === "temp"}
      <defs>
        <linearGradient id="la-temp" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0" stop-color="#784aa8" /><stop offset="0.3" stop-color="#3c8ce6" /><stop offset="0.5" stop-color="#5ac878" /><stop offset="0.72" stop-color="#ebdc50" /><stop offset="1" stop-color="#d22828" />
        </linearGradient>
      </defs>
      <rect width="120" height="64" fill="url(#la-temp)" opacity="0.75" />
      <path d="M0 22 C 30 14, 60 30, 120 18 M0 44 C 40 36, 70 52, 120 40" fill="none" stroke="#fff" stroke-width="0.8" opacity="0.4" />
      <text x="96" y="58" text-anchor="middle" font-size="10" font-weight="700" fill="#fff">24°</text>
    {:else if kind === "cloud"}
      <rect width="120" height="64" fill="#2a3a52" />
      <path d="M8 40 C 8 28, 26 24, 34 32 C 40 20, 64 22, 66 36 C 78 32, 90 40, 84 50 L 12 50 C 4 50, 2 44, 8 40 Z" fill="#e8edf3" opacity="0.85" />
      <path d="M70 22 C 74 12, 92 12, 96 20 C 106 18, 116 26, 110 34 L 72 34 C 64 34, 64 26, 70 22 Z" fill="#c9d2dc" opacity="0.6" />
    {:else if kind === "vis"}
      <rect width="120" height="64" fill="#1f2a24" />
      <rect x="0" y="0" width="40" height="64" fill="#cf5bd6" opacity="0.5" />
      <rect x="40" y="0" width="34" height="64" fill="#e2453c" opacity="0.45" />
      <rect x="74" y="0" width="28" height="64" fill="#4f8fe8" opacity="0.4" />
      <text x="20" y="40" text-anchor="middle" font-size="10" font-weight="700" fill="#fff">½ SM</text>
    {:else if kind === "radar"}
      <path d="M10 40 C 20 20, 44 18, 52 30 C 60 42, 40 56, 22 52 C 12 50, 6 46, 10 40 Z" fill="#3aa0ff" opacity="0.55" />
      <path d="M22 40 C 26 32, 38 30, 42 36 C 46 44, 32 48, 26 46 Z" fill="#7ce0ff" opacity="0.8" />
      <path d="M70 18 C 82 6, 108 10, 112 24 C 116 38, 92 44, 80 38 C 70 34, 64 26, 70 18 Z" fill="#3aa0ff" opacity="0.5" />
      <path d="M84 22 C 90 16, 102 18, 102 26 C 102 32, 92 34, 88 31 Z" fill="#fff36a" opacity="0.75" />
    {/if}
  </svg>
{/snippet}

{#snippet card(key: string, label: string, active: boolean, toggle: () => void, extra?: Snippet | string)}
  <button class="card" class:active role="switch" aria-checked={active} onclick={toggle} title={typeof extra === "string" ? `${label}: ${extra}` : label}>
    <span class="thumb">{@render art(key)}</span>
    <span class="text">
      <span class="label">{label}</span>
      {#if typeof extra === "string"}<span class="note">{extra}</span>{:else if extra}{@render extra()}{:else}<span class="note">&nbsp;</span>{/if}
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

{#snippet windNote()}
  <span class="note">{on.has("wind") ? (levelLabel ?? "Auto") : "Moving flow"}</span>
{/snippet}

{#snippet aspNote()}
  <span class="note">
    {#if on.has("airspace") && airspaceState === "loading"}Loading from the FAA…{:else if on.has("airspace") && airspaceNote}{airspaceNote}{:else if on.has("airspace") && airspaceState === "error"}FAA partly not reachable{:else}Worldwide · from zoom 5{/if}
  </span>
{/snippet}

{#snippet navaidsNote()}
  <span class="note">{on.has("navaids") && navNote ? navNote : "VOR, NDB, waypoints"}</span>
{/snippet}

{#snippet icingNote2()}
  <span class="note">{on.has("icing") && icingNote ? icingNote : "By flight level (test)"}</span>
{/snippet}

{#snippet tempNote2()}
  <span class="note">{on.has("temp") && tempNote ? tempNote : "Air temperature"}</span>
{/snippet}

{#snippet cloudNote2()}
  <span class="note">{on.has("cloud") && cloudNote ? cloudNote : "Total cloud cover"}</span>
{/snippet}

{#snippet visNote2()}
  <span class="note">{on.has("vis") && visNote ? visNote : "At the surface"}</span>
{/snippet}

{#snippet tawsNote()}
  <span class="note">{on.has("taws") && !hasAircraft ? "Needs the sim" : "100 mi around you"}</span>
{/snippet}

{#snippet ringsNote()}
  <span class="note">{on.has("rings") && !hasAircraft ? "Needs the sim" : "Around your aircraft"}</span>
{/snippet}

{#snippet obstNote()}
  <span class="note">
    {#if on.has("obstacles") && obstaclesState === "loading"}Loading from the FAA…{:else if on.has("obstacles") && obstaclesState === "error"}FAA not reachable{:else}US only · 200 ft+{/if}
  </span>
{/snippet}

{#snippet radarNote()}
  <span class="note">
    {#if !on.has("radar")}Rain and snow{:else if radarError}Not reachable{:else if radarTime}{zulu(radarTime)}{:else}Loading…{/if}
  </span>
{/snippet}

<div class="rail" class:compact class:stack>
  <div class="seg" role="radiogroup" aria-label="Map view">
    <button role="radio" aria-checked={!three} class:on={!three} onclick={() => onthree(false)}>2D</button>
    <button role="radio" aria-checked={three} class:on={three} onclick={() => onthree(true)}>3D</button>
  </div>
  {#if hasTools}
  <div class="toolwrap">
  <div class="drawer" class:out={toolsOpen} aria-hidden={!toolsOpen} inert={!toolsOpen}>
  {#if onmeasure}
    <button class="layers-btn" class:open={measuring} aria-pressed={measuring} title="Measure distances" onclick={() => ((open = null), onmeasure())}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 16.5L16.5 3 21 7.5 7.5 21z" /><path d="M7 12.5l1.8 1.8M10 9.5l2.6 2.6M13 6.5l1.8 1.8" /></svg>
      <span>Measure</span>
    </button>
  {/if}
  {#if onmarkup}
    <button class="layers-btn" class:open={markupOpen} aria-pressed={markupOpen} title="Mark up the map: draw on it" onclick={() => ((open = null), onmarkup())}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20l1.2-4.2L15.5 5.5a2.1 2.1 0 0 1 3 3L8.2 18.8z" /><path d="M13.5 7.5l3 3" /><path d="M14 20h6" /></svg>
      <span>Markup</span>
    </button>
  {/if}
  {#if onpad}
    <button class="layers-btn" class:open={padOpen} aria-pressed={padOpen} title="Scratch pad: notes, clearances, briefings" onclick={onpad}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="3" width="14" height="18" rx="2" /><path d="M9 3v2h6V3" /><path d="M8.5 11c1.5-2 2.5-2 3 0s1.5 2 3.5-.5M8.5 16h7" /></svg>
      <span>Pad</span>
    </button>
  {/if}
  {#if ontaxi}
    <button class="layers-btn" class:open={taxiOpen} aria-pressed={taxiOpen} title="Taxi assist: follow ATC's taxi instructions" onclick={() => ((open = null), ontaxi())}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 21V9a4 4 0 014-4h10" /><path d="M15 1l4 4-4 4" /><circle cx="5" cy="21" r="0.6" /></svg>
      <span>Taxi</span>
    </button>
  {/if}
  {#if onprofile}
    <button class="layers-btn" class:open={profileOpen} aria-pressed={profileOpen} title="Vertical profile of the route" onclick={() => ((open = null), onprofile())}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 20h18" /><path d="M3 17l4-5 3 2 4-6 3 3 4-2" /><path d="M4 9.5L8 6h9l3 3.5" stroke-dasharray="2 2" /></svg>
      <span>Profile</span>
    </button>
  {/if}
  {#if onpfd}
    <button class="layers-btn" class:open={pfdOpen} aria-pressed={pfdOpen} title="Primary flight display" onclick={onpfd}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="4" width="18" height="16" rx="2" /><path d="M3 13l18-3M9 11.5l3 1.5 3-1.5" /></svg>
      <span>PFD</span>
    </button>
  {/if}
  </div>
  <button class="layers-btn tools" class:open={toolsOpen} aria-expanded={toolsOpen} title={toolsOpen ? "Put the tools away" : "Measure, Markup, Pad, Taxi, Profile, PFD"} onclick={toggleTools}>
    {#if toolsOpen}
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 6l6 6-6 6" /><path d="M9 6l6 6-6 6" opacity="0.45" /></svg>
    {:else}
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M14.5 6.5a4 4 0 0 0-5.3 5.3L3.5 17.5l3 3 5.7-5.7a4 4 0 0 0 5.3-5.3l-2.4 2.4-2.6-.6-.6-2.6z" /></svg>
    {/if}
    <span>Tools</span>
    {#if anyTool && !toolsOpen}<i class="tdot" aria-label="(a tool is on)"></i>{/if}
  </button>
  </div>
  {/if}
  <button class="layers-btn" class:open={!!open} aria-expanded={!!open} onclick={togglePanel}>
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3l9 5-9 5-9-5 9-5zM3 13l9 5 9-5" /></svg>
    <span>Layers</span>
  </button>
</div>

{#if open}
  {@const menu = TABS.find((m) => m.id === open)!}
  <div class="panel" role="dialog" aria-label="Layers: {menu.title}">
    <header>
      <h3>Layers</h3>
      <button class="x" aria-label="Close" onclick={() => (open = null)}>
        <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
      </button>
    </header>
    <div class="tabs" role="group" aria-label="Show layers for">
      {#each TABS as t (t.id)}
        <button aria-pressed={open === t.id} class:sel={open === t.id} onclick={() => showTab(t.id)}>
          <svg viewBox="0 0 24 24" aria-hidden="true" class:fill={t.id === "traffic"}><path d={t.icon} /></svg>
          {t.label}
          {#if tabOn(t.id)}<i class="dot" aria-label="(on)"></i>{/if}
        </button>
      {/each}
    </div>
    {#if open === "map"}
    <div class="grid">
      {@render card("3d", "3D flight", three, () => onthree(!three), "Tilted, with terrain")}
      {@render card("night", "Day / night", on.has("night"), () => ontoggle("night"), "Where it is night")}
      {@render card("satellite", "Satellite", on.has("satellite"), () => ontoggle("satellite"), "Imagery")}
      {@render card("terrain", "Topographic", on.has("terrain"), () => ontoggle("terrain"), "Hills and mountains")}
      {@render card("borders", "Borders", on.has("borders"), () => ontoggle("borders"), "Countries, states")}
      {@render card("roads", "Roads", on.has("roads"), () => ontoggle("roads"), "Roads and highways")}
      {@render card("taws", "Terrain", on.has("taws"), () => ontoggle("taws"), tawsNote)}
      {@render card("obstacles", "Obstacles", on.has("obstacles"), () => ontoggle("obstacles"), obstNote)}
      {@render card("rings", "Range rings", on.has("rings"), () => ontoggle("rings"), ringsNote)}
    </div>
    {#if on.has("rings")}
      <div class="detail rings">
        <span class="lbl">Range rings</span>
        <div class="levels" role="radiogroup" aria-label="Number of rings">
          {#each [2, 3] as n (n)}
            <button role="radio" aria-checked={ringCount === n} class:on={ringCount === n} onclick={() => onrings?.(n, ringSet)}>{n} rings</button>
          {/each}
        </div>
        <span class="lbl">Distances (NM)</span>
        <div class="levels" role="radiogroup" aria-label="Ring distances">
          {#each ["auto", "5", "10", "15"] as v (v)}
            <button role="radio" aria-checked={ringSet === v} class:on={ringSet === v} onclick={() => onrings?.(ringCount, v)}>
              {v === "auto" ? "Auto" : Array.from({ length: ringCount }, (_, i) => (i + 1) * Number(v)).join(", ")}
            </button>
          {/each}
        </div>
      </div>
    {/if}
    {#if on.has("roads")}
      <div class="detail">
        <label for="roads-op">Roads <span>{Math.round(roadsOpacity * 100)}%</span></label>
        <input
          id="roads-op"
          type="range"
          min="0.15"
          max="1"
          step="0.05"
          value={roadsOpacity}
          oninput={(e) => onroadsopacity?.(Number((e.currentTarget as HTMLInputElement).value))}
        />
        <div class="ticks"><span>Faint</span><span>Full</span></div>
      </div>
    {/if}
    {#if on.has("taws") || on.has("obstacles")}
      <p class="legend">
        <span style:--c="#d62622">Above you or within 100 ft below</span><span style:--c="#ecc428">100–1,000 ft below</span><span style:--c="#2ea043">1,000–2,000 ft below</span><span style:--c="#3a3d42">2,000 ft+ below</span>
      </p>
    {/if}
    <details class="credit"><summary>Data sources</summary><p>Satellite Esri · Terrain AWS Terrain Tiles · Map OpenFreeMap, OpenStreetMap · Obstacles FAA Digital Obstacle File (US, every 28 days)</p></details>
    {:else if open === "weather"}
    <div class="grid">
      {@render card("radar", "Rain radar", on.has("radar"), () => ontoggle("radar"), radarNote)}
      {@render card("wx-cat", "Flight category", on.has("wx-cat"), () => ontoggle("wx-cat"), "VFR to LIFR")}
      {@render card("wx-adv", "SIGMET / AIRMET", on.has("wx-adv"), () => ontoggle("wx-adv"), "Hazard areas")}
      {@render card("wind", "Wind", on.has("wind"), () => ontoggle("wind"), windNote)}
      {@render card("wx-wind", "Airport wind", on.has("wx-wind"), () => ontoggle("wx-wind"), "Reported at airports")}
      {@render card("temp", "Temperature", on.has("temp"), () => ontoggle("temp"), tempNote2)}
      {@render card("icing", "Icing Potential", on.has("icing"), () => ontoggle("icing"), icingNote2)}
      {@render card("cloud", "Cloud cover", on.has("cloud"), () => ontoggle("cloud"), cloudNote2)}
      {@render card("vis", "Visibility", on.has("vis"), () => ontoggle("vis"), visNote2)}
    </div>
    {#if on.has("wind") || on.has("temp") || on.has("icing")}
      <div class="lvl">
        <span class="lbl">Level <em>for {[on.has("wind") && "wind", on.has("temp") && "temperature", on.has("icing") && "icing"].filter(Boolean).join(", ")}</em></span>
        <div class="levels" role="radiogroup" aria-label="Level for wind, temperature and icing">
          {#each [{ id: "auto", label: levelNow ? `Auto · ${levelNow}` : "Auto" }, ...WIND_LEVELS.map((l) => ({ id: l.id, label: l.id === "sfc" ? "SFC" : l.label }))] as l (l.id)}
            <button role="radio" aria-checked={level === l.id} class:on={level === l.id} onclick={() => onlevel?.(l.id)}>{l.label}</button>
          {/each}
        </div>
      </div>
    {/if}
    {#if on.has("icing")}
      <div class="iscale" aria-label="Icing Potential colours">
        {#each ICING_BANDS as b, i (b.min)}
          <span class="ib"><img src={icingSwatch(i)} alt="" />{b.label}</span>
        {/each}
      </div>
      <div class="iscale adv" aria-label="Icing SIGMETs and AIRMETs">
        {#each Object.values(ICING_ADV) as a (a.label)}
          <span class="ib"><i class="dash" style:--c={a.color}></i>{a.label}</span>
        {/each}
      </div>
      {#if icingAdvNote}<p class="inote">{icingAdvNote} · faint when not at this level · click one for details</p>{/if}
      <p class="inote">Test version: FlightDeck's own estimate from the forecast, for flight simulation only. Not observed icing, not for real-world flying.</p>
    {/if}
    {#if on.has("wx-cat")}
      <p class="legend">
        <span style:--c="#2fbf71">VFR</span><span style:--c="#4f8fe8">MVFR</span><span style:--c="#e2453c">IFR</span><span style:--c="#cf5bd6">LIFR</span>
      </p>
    {/if}
      <details class="credit"><summary>Data sources</summary><p>Radar RainViewer · METARs, SIGMETs and AIRMETs NOAA Aviation Weather Center (every 10 min while on) · Wind,
        temperature, cloud cover and visibility NOAA GFS (about 60–100 KB each, every 3 h while on) · Icing Potential
        FlightDeck's estimate from NOAA GFS on 20 levels (about 25 MB per forecast step, while on){#if icingInfo} · {icingInfo}{/if}</p></details>
    {:else}
    <div class="grid">
      {@render card("world", "World airports", on.has("world"), () => ontoggle("world"), "Every airport")}
      {@render card("airports", "Your airports", on.has("airports"), () => ontoggle("airports"), "Where you've flown")}
      {@render card("routes", "Past routes", on.has("routes"), () => ontoggle("routes"), "Your flights")}
      {@render card("aeroway", "Runways", !on.has("aeroway-off"), () => ontoggle("aeroway-off"), "And taxiways")}
      {@render card("airspace", "Airspace", on.has("airspace"), () => ontoggle("airspace"), aspNote)}
      {#if NAV_AIDS_ENABLED}{@render card("navaids", "Nav aids", on.has("navaids"), () => ontoggle("navaids"), navaidsNote)}{/if}
      {#if parkedLayer}
        {@render card("parked", "Parked aircraft", on.has("parked"), () => ontoggle("parked"), "Where you left them")}
      {/if}
    </div>
    {#if on.has("airspace")}
      <div class="detail rings">
        <span class="lbl">Airspace shown</span>
        <div class="levels" role="group" aria-label="Airspace filters">
          {#each [["us", "US only"], ["nomr", "Hide MOA & Restricted"], ["noaw", "Hide airways"]] as [k, label] (k)}
            <button aria-pressed={aspFilter.has(k)} class:on={aspFilter.has(k)} onclick={() => onaspfilter?.(k)}>{label}</button>
          {/each}
        </div>
      </div>
    {/if}
    {#if on.has("world")}
      <div class="detail">
        <label for="ap-detail">Airport detail</label>
        <input
          id="ap-detail"
          type="range"
          min="1"
          max="3"
          step="1"
          value={airportDetail}
          oninput={(e) => ondetail?.(Number((e.currentTarget as HTMLInputElement).value))}
        />
        <div class="ticks"><span>Big</span><span>Small</span><span>All</span></div>
      </div>
    {/if}
    <h4>Online networks</h4>
    <div class="grid two">
      {@render netCard("vatsim", "VATSIM")}
      {@render netCard("ivao", "IVAO")}
    </div>
    <details class="credit"><summary>Data sources</summary><p>Airports OurAirports · Runways and taxiways OpenStreetMap · Live traffic VATSIM and IVAO (every 30 s while on) ·
      ATC areas VAT-Spy Data Project (CC BY-SA 4.0) · Airspace FAA (US: classes and special use, every 28 days) and openAIP (outside the US, CC BY-NC 4.0, countries in view from zoom 5)</p></details>
    {/if}
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
  .detail.rings .lbl {
    display: block;
    margin: 8px 2px 0;
    font-size: 13px;
    color: var(--ink);
  }
  .detail.rings .lbl:first-child {
    margin-top: 0;
  }
  .detail.rings .levels {
    margin-top: 5px;
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
  .drawer {
    display: flex;
    gap: 8px;
    max-width: 0;
    overflow: hidden;
    opacity: 0;
    transform: translateX(24px);
    transition:
      max-width 0.28s ease,
      opacity 0.2s ease,
      transform 0.28s ease;
  }
  .drawer.out {
    max-width: 720px;
    opacity: 1;
    transform: none;
  }
  /* One drawer: the tools sit together in a single bar, like 2D/3D. */
  .drawer {
    gap: 2px;
    border-radius: 8px;
    background: rgba(28, 30, 33, 0.92);
    border: 1px solid transparent;
    backdrop-filter: blur(6px);
  }
  .drawer.out {
    padding: 3px;
    border-color: var(--rule);
  }
  .drawer .layers-btn {
    border: none;
    background: transparent;
    backdrop-filter: none;
    border-radius: 6px;
    padding: 0 10px;
    white-space: nowrap;
  }
  .drawer .layers-btn:hover {
    background: rgba(255, 255, 255, 0.05);
  }
  .drawer .layers-btn.open {
    background: rgba(240, 178, 58, 0.16);
    color: var(--amber);
  }
  .layers-btn.tools {
    position: relative;
  }
  .toolwrap {
    position: relative;
    display: flex;
    gap: 8px;
  }
  /* A narrow map (beside the PFD, a tablet): icons only, so they fit. */
  .rail.compact .layers-btn span {
    display: none;
  }
  .rail.compact .layers-btn {
    padding: 0 11px;
  }
  /* Narrower still (phones): the tools drop down under Tools as a list. */
  .rail.stack {
    z-index: 7;
  }
  .rail.stack .drawer {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    flex-direction: column;
    gap: 5px;
    max-width: none;
    max-height: 0;
    transform: translateY(-6px);
    transition:
      max-height 0.25s ease,
      opacity 0.2s ease,
      transform 0.25s ease;
  }
  .rail.stack .drawer.out {
    max-height: 400px;
    transform: none;
  }
  .rail.stack .drawer .layers-btn {
    min-height: 40px;
    min-width: 150px;
    justify-content: flex-start;
    padding: 0 12px;
  }
  .rail.stack .drawer .layers-btn span {
    display: inline;
  }
  .tdot {
    position: absolute;
    top: 5px;
    right: 5px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--amber);
  }
  .iscale {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 10px;
    margin: 10px 2px 0;
    font-size: 11px;
    color: var(--ink-dim);
  }
  .iscale .ib {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .iscale img {
    width: 18px;
    height: 18px;
    border-radius: 3px;
    border: 1px solid var(--rule);
  }
  .iscale .dash {
    width: 18px;
    height: 0;
    border-top: 2px dashed var(--c);
  }
  .inote {
    margin: 8px 2px 0;
    font-size: 11px;
    line-height: 1.35;
    color: var(--ink-faint, #7d828a);
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
    width: 384px;
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
    gap: 6px;
    margin-bottom: 10px;
  }
  h3 {
    margin: 0;
    font-family: var(--cond);
    font-size: 19px;
    font-weight: 600;
  }
  .tabs {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 6px;
    margin-bottom: 12px;
  }
  .tabs button {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 6px;
    border: 1px solid var(--rule);
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.03);
    color: var(--ink-dim);
    font-family: var(--cond);
    font-size: 14.5px;
    font-weight: 600;
    white-space: nowrap;
  }
  .tabs button:hover {
    color: var(--ink);
    border-color: #4b5057;
  }
  .tabs button.sel {
    background: rgba(240, 178, 58, 0.14);
    border-color: var(--amber);
    color: var(--amber);
  }
  .tabs svg {
    width: 15px;
    height: 15px;
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linejoin: round;
  }
  .tabs svg.fill {
    fill: currentColor;
    stroke: none;
  }
  .tabs .dot {
    position: absolute;
    top: 5px;
    right: 6px;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--amber);
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
    grid-template-columns: repeat(3, minmax(0, 1fr));
    grid-auto-rows: 68px;
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
    min-width: 0;
    padding: 5px 7px 6px;
  }
  /* Every tile the same size: one line of name, one line of description. */
  .label,
  .note {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .label {
    font-size: 12px;
    font-weight: 600;
    line-height: 1.25;
  }
  .note {
    font-size: 10.5px;
    line-height: 1.3;
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
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .lvl {
    margin: 10px 2px 0;
  }
  .lvl .lbl {
    font-size: 12px;
    color: var(--ink);
  }
  .lvl .lbl em {
    font-style: normal;
    color: var(--ink-dim);
  }
  .lvl .levels {
    margin-top: 5px;
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
  .credit summary {
    cursor: pointer;
    list-style: none;
    width: max-content;
  }
  .credit summary::-webkit-details-marker {
    display: none;
  }
  .credit summary::before {
    content: "ⓘ ";
  }
  .credit summary:hover {
    color: var(--ink-dim);
  }
  .credit p {
    margin: 4px 0 0;
  }
  /* Tablets and narrow windows: the menu buttons show their icons only, so
     they stay clear of the cards on the left. */
  @media (max-width: 1100px) {
    .layers-btn span {
      display: none;
    }
    .layers-btn {
      padding: 0 11px;
    }
  }
  /* Phones: the menu buttons show their icons only, and a menu opens the
     full width of the screen. */
  @media (max-width: 640px) {
    .rail {
      top: 10px;
      right: 10px;
      left: 10px;
      justify-content: flex-end;
      gap: 6px;
    }
    .layers-btn span {
      display: none;
    }
    .layers-btn {
      padding: 0 11px;
    }
    .panel {
      top: 58px;
      right: 10px;
      left: 10px;
      width: auto;
      max-height: calc(100% - 140px);
    }
    .tabs button {
      padding: 5px 7px;
      font-size: 13.5px;
    }
    .tabs svg {
      display: none;
    }
  }
</style>
