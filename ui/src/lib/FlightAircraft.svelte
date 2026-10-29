<script lang="ts">
  // The aircraft card in a flight's details: callsign and airline,
  // registration and type, and the aircraft's picture (your own photo, else
  // the Planespotters.net photo of the registration, else a silhouette).
  import { api, type FlightRow, type HangarItem } from "./api";
  import AircraftPicture from "./hangar/AircraftPicture.svelte";
  import { airlineLook, findAirline } from "./hangar/airlines";
  import { modelName } from "./hangar/names";

  let { flight }: { flight: FlightRow } = $props();

  let fleet = $state<HangarItem[]>([]);
  let photo = $state<string | null>(null);

  $effect(() => {
    api.hangar().then((h) => (fleet = h ?? []));
  });

  /** The hangar aircraft this flight was flown in, matched by registration. */
  const item = $derived.by<HangarItem>(() => {
    const reg = flight.registration.trim().toUpperCase();
    const found = reg ? fleet.find((a) => a.registration.toUpperCase() === reg) : undefined;
    return (
      found ?? {
        id: -1, title: flight.aircraft_title, icao_type: flight.icao_type, registration: flight.registration,
        engine_type: "", engine_count: 0, category_class: "", has_photo: false, flights: 0, minutes: 0, distance_nm: 0,
        landings: 0, first_flight_ms: 0, last_flight_ms: 0, best_landing_fpm: null, avg_landing_fpm: null, favorite: false,
        last_callsign: "", last_icao: null, last_name: null, last_country: null, last_country_name: null,
        last_region_name: null, last_lat: null, last_lon: null, last_sim: "", model: null, airline: null,
        sim_registration: flight.registration, manual: false,
      }
    );
  });

  $effect(() => {
    const it = item;
    photo = null;
    if (it.id > 0 && it.has_photo) api.aircraftPhoto(it.id).then((p) => (photo = p ?? null));
  });

  const model = $derived(modelName(item));
  /** A short type for the badge: "Boeing 737-700" reads "B737-700". */
  const short = $derived.by(() => {
    if (/^Boeing \d/.test(model)) return model.replace(/^Boeing /, "B").replace(/ MAX /, " MAX ");
    if (/^Airbus A/.test(model)) return model.replace(/^Airbus /, "");
    return item.icao_type || model;
  });
  /** The airline: from the callsign's airline code, else the aircraft's. */
  const airline = $derived.by(() => {
    const code = flight.callsign.trim().toUpperCase().match(/^([A-Z]{3})\d/)?.[1];
    const fromCallsign = code ? findAirline(code) : null;
    if (fromCallsign) return fromCallsign.name;
    return airlineLook(item.airline)?.name ?? "";
  });
  const headline = $derived(flight.callsign.trim().toUpperCase() || model);
</script>

<div class="card">
  <div class="top">
    <div class="who">
      <span class="cs num" class:plain={!flight.callsign.trim()}>{headline}</span>
      {#if airline}<span class="al">{airline}</span>{:else if flight.callsign.trim()}<span class="al">{model}</span>{/if}
    </div>
    <div class="what">
      {#if item.registration}<span class="reg num">{item.registration}</span>{/if}
      {#if short}<span class="badge num" title={model}>{short}</span>{/if}
    </div>
  </div>
  <div class="pic">
    <AircraftPicture {item} own={photo} variant="banner" />
  </div>
</div>

<style>
  .card {
    margin: 4px 0 14px;
    border: 1px solid var(--rule);
    border-radius: 10px;
    overflow: hidden;
    background: var(--panel-2);
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px 8px;
  }
  .who,
  .what {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .what {
    align-items: flex-end;
    flex: none;
  }
  .cs {
    font-size: 24px;
    font-weight: 700;
    line-height: 1.05;
    color: var(--amber);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cs.plain {
    font-size: 20px;
    color: var(--ink);
  }
  .al {
    font-size: 13px;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .reg {
    font-size: 18px;
    font-weight: 700;
    color: var(--ink);
    line-height: 1.1;
  }
  .badge {
    margin-top: 3px;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 11.5px;
    font-weight: 700;
    color: #fff;
    background: #2f5fa8;
  }
  .pic {
    position: relative;
    height: 150px;
  }
</style>
