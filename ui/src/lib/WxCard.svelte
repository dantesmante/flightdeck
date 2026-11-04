<script lang="ts" module>
  import type { WxAdvisoryProps } from "./api";

  /** Colours of the SIGMET and AIRMET families, as drawn on the map. */
  export const WX_GROUP: Record<WxAdvisoryProps["group"], { color: string; name: string }> = {
    TS: { color: "#e2453c", name: "Thunderstorms" },
    TURB: { color: "#f0a03a", name: "Turbulence" },
    ICE: { color: "#5cc8e8", name: "Icing" },
    IFR: { color: "#b07fe0", name: "Low cloud and visibility" },
    VA: { color: "#b58d63", name: "Volcanic ash" },
    TC: { color: "#e05aa8", name: "Tropical cyclone" },
    OTHER: { color: "#d8d070", name: "Other hazard" },
  };
  /** Flight category colours, as drawn on the map. */
  export const CATEGORY = [
    { name: "VFR", color: "#2fbf71" },
    { name: "MVFR", color: "#4f8fe8" },
    { name: "IFR", color: "#e2453c" },
    { name: "LIFR", color: "#cf5bd6" },
  ];
</script>

<script lang="ts">
  // A SIGMET or AIRMET clicked on the map.
  let { item, onclose }: { item: WxAdvisoryProps; onclose: () => void } = $props();

  const HAZARD: Record<string, string> = {
    CONVECTIVE: "Thunderstorms",
    TS: "Thunderstorms",
    TURB: "Turbulence",
    "TURB-HI": "Turbulence (high levels)",
    "TURB-LO": "Turbulence (low levels)",
    ICE: "Icing",
    IFR: "IFR conditions",
    MT_OBSC: "Mountains obscured",
    MTW: "Mountain waves",
    LLWS: "Low-level wind shear",
    SFC_WND: "Strong surface wind",
    VA: "Volcanic ash",
    TC: "Tropical cyclone",
  };
  const QUAL: Record<string, string> = { SEV: "Severe", MOD: "Moderate", EMBD: "Embedded", OBSC: "Obscured", FRQ: "Frequent", ISOL: "Isolated", SQL: "Squall line" };
  const z = (ms: number | null) => {
    if (!ms) return "–";
    const d = new Date(ms);
    return `${String(d.getUTCDate()).padStart(2, "0")} ${String(d.getUTCHours()).padStart(2, "0")}${String(d.getUTCMinutes()).padStart(2, "0")}Z`;
  };
  const level = (v: string) => {
    if (!v) return "";
    const n = Number(v);
    if (!Number.isFinite(n)) return v;
    if (n === 0) return "Surface";
    return n >= 18000 ? `FL${String(Math.round(n / 100)).padStart(3, "0")}` : `${n.toLocaleString("en-US")} ft`;
  };
  const g = $derived(WX_GROUP[item.group] ?? WX_GROUP.OTHER);
  const title = $derived(HAZARD[item.hazard.toUpperCase()] ?? (item.hazard || g.name));
  const qual = $derived(QUAL[item.qualifier.toUpperCase()] ?? "");
  const heading = $derived(qual ? `${qual} ${title.toLowerCase()}` : title);
  const levels = $derived([level(item.base), level(item.top)].filter(Boolean).join(" to "));
</script>

<aside class="wc" aria-label="Weather advisory">
  <button class="x" aria-label="Close" onclick={onclose}>
    <svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
  </button>
  <span class="kind" style:--c={g.color}>{item.kind === "CONVECTIVE" ? "Convective SIGMET" : item.kind}</span>
  <h3 style:color={g.color}>{heading}</h3>
  {#if item.name}<p class="sub">{item.name}</p>{/if}
  <div class="facts">
    <div><span class="lbl">Valid from</span><b class="mono">{z(item.from_ms)}</b></div>
    <div><span class="lbl">Until</span><b class="mono">{z(item.to_ms)}</b></div>
    <div><span class="lbl">Levels</span><b class="mono">{levels || "–"}</b></div>
  </div>
  {#if item.due_to}<p class="sub">Due to {item.due_to}</p>{/if}
  {#if item.raw}<p class="raw">{item.raw}</p>{/if}
  <p class="credit">From the NOAA Aviation Weather Center. For flight simulation only.</p>
</aside>

<style>
  .wc {
    position: absolute;
    top: 16px;
    left: 16px;
    width: 340px;
    max-width: calc(100% - 32px);
    max-height: calc(100% - 32px);
    overflow-y: auto;
    scrollbar-width: thin;
    z-index: 4;
    padding: 12px 14px 10px;
    background: rgba(28, 30, 33, 0.96);
    border: 1px solid var(--rule);
    border-radius: 10px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
    color: var(--ink);
  }
  .x {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 30px;
    height: 30px;
    padding: 6px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    cursor: pointer;
  }
  .x svg {
    width: 18px;
    height: 18px;
    stroke: var(--ink-dim);
    stroke-width: 2;
    fill: none;
  }
  .kind {
    display: inline-block;
    padding: 1px 8px;
    border-radius: 10px;
    border: 1px solid var(--c);
    color: var(--c);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
  }
  h3 {
    margin: 6px 28px 0 0;
    font-family: var(--cond);
    font-size: 21px;
    font-weight: 600;
  }
  .sub {
    margin: 2px 0 0;
    color: var(--ink-dim);
    font-size: 13px;
  }
  .facts {
    display: grid;
    grid-template-columns: 1fr 1fr 1.3fr;
    gap: 6px;
    margin-top: 10px;
  }
  .lbl {
    display: block;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--ink-faint);
  }
  .mono {
    font-family: "IBM Plex Mono", ui-monospace, monospace;
    font-weight: 600;
    font-size: 13px;
  }
  .raw {
    margin: 10px 0 0;
    padding: 6px 8px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.25);
    font: 11.5px/1.4 "IBM Plex Mono", ui-monospace, monospace;
    color: var(--ink-dim);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .credit {
    margin: 10px 0 0;
    color: var(--ink-faint);
    font-size: 11px;
  }
</style>
