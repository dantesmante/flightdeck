<script lang="ts">
  // The active flight drawn as an ATC flight progress strip: a cream paper
  // strip with ruled boxes, sitting in a holder whose edge lights up while
  // the flight is live. Boxes with nothing in them stay empty, like paper.
  import type { FlightPlan, AircraftInfo, Place, Telemetry } from "./api";

  interface Props {
    aircraft: AircraftInfo | null;
    dep: Place | null;
    telemetry: Telemetry | null;
    simLabel: string | null;
    active: boolean;
    /** The flight plan, if loaded: callsign, type and destination. */
    plan?: FlightPlan | null;
  }
  let { aircraft, dep, telemetry, simLabel, active, plan = null }: Props = $props();
  const type = $derived(aircraft?.icao_type || plan?.aircraft_icao || "");

  const level = $derived.by(() => {
    if (!telemetry) return "";
    const ft = telemetry.alt_msl_ft;
    return ft >= 18000 ? `FL${String(Math.round(ft / 100)).padStart(3, "0")}` : String(Math.round(ft / 100) * 100);
  });
  const trend = $derived(
    !telemetry || Math.abs(telemetry.vs_fpm) < 200 ? "" : telemetry.vs_fpm > 0 ? "climbing" : "descending",
  );
  const engines = $derived(
    aircraft && aircraft.engine_count > 0
      ? `${aircraft.engine_count} ${aircraft.engine_type === "unknown" ? "engine" : aircraft.engine_type}${aircraft.engine_count > 1 ? "s" : ""}`
      : "",
  );
</script>

<div class="holder" class:active>
  <div class="strip" aria-label="Flight strip">
    <div class="box ident">
      <span class="big">{plan?.callsign || aircraft?.registration || type || "—"}</span>
      <span class="small" title={aircraft?.title}>
        {plan?.callsign && aircraft?.registration ? `${aircraft.registration}, ` : ""}{aircraft?.title || "Aircraft not reported yet"}
      </span>
    </div>
    <div class="box type">
      <span class="big">{type}</span>
      <span class="small">{engines}</span>
    </div>
    <div class="box route">
      <span class="big">{dep?.icao ?? plan?.origin.icao ?? ""}</span>
      <span class="small" title={dep?.name ?? plan?.origin.name}>{dep?.name ?? plan?.origin.name ?? "Departure set at block out"}</span>
    </div>
    <div class="box dest">
      <span class="big hand">{plan?.destination.icao ?? ""}</span>
      <span class="small" title={plan?.destination.name}>{plan?.destination.name || "Destination"}</span>
    </div>
    <div class="box level">
      <span class="big">{level}</span>
      <span class="small">{trend || simLabel || ""}</span>
    </div>
  </div>
</div>

<style>
  .holder {
    border-radius: 4px;
    padding: 0 0 0 8px;
    background: var(--panel-3);
    transition: background 0.4s ease;
  }
  .holder.active {
    background: var(--amber);
  }
  .strip {
    display: grid;
    grid-template-columns: 1.35fr 0.8fr 0.85fr;
    grid-template-areas:
      "ident type level"
      "route route dest";
    background: var(--strip);
    color: var(--strip-ink);
    border-radius: 0 3px 3px 0;
    /* faint paper grain */
    background-image: repeating-linear-gradient(0deg, rgba(0, 0, 0, 0.018) 0 1px, transparent 1px 3px);
    min-height: 116px;
  }
  .box {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 2px;
    padding: 6px 7px 5px;
    border-left: 1px solid var(--strip-rule);
    min-width: 0;
  }
  .ident {
    grid-area: ident;
    border-left: none;
  }
  .type {
    grid-area: type;
  }
  .level {
    grid-area: level;
  }
  .route {
    grid-area: route;
    border-left: none;
    border-top: 1px solid var(--strip-rule);
  }
  .dest {
    grid-area: dest;
    border-top: 1px solid var(--strip-rule);
  }
  .big {
    font-family: var(--cond);
    font-weight: 600;
    font-size: 21px;
    line-height: 1.1;
    letter-spacing: 0.01em;
    min-height: 23px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .small {
    font-size: 11px;
    line-height: 1.25;
    color: #6d6650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dest .big {
    border-bottom: 1px dashed var(--strip-rule);
  }
</style>
