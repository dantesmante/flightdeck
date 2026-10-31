<script lang="ts">
  // The active flight as a boarding pass: callsign and aircraft at the top,
  // the aircraft's picture when it is in the hangar, the route, then a
  // tear-off stub with the times and the progress to the destination.
  import { api, type HangarItem, type Status } from "./api";
  import { hhmm, zulu } from "./format";
  import AircraftPicture from "./hangar/AircraftPicture.svelte";
  import { airlineLook, findAirline } from "./hangar/airlines";
  import { modelName } from "./hangar/names";
  import { distance } from "./prefs.svelte";

  let { status }: { status: Status } = $props();

  let fleet = $state<HangarItem[]>([]);
  let photo = $state<string | null>(null);
  let loadedFor = "";

  const plan = $derived(status.plan);
  const ac = $derived(status.aircraft);
  const reg = $derived((ac?.registration || plan?.registration || "").trim().toUpperCase());

  // The hangar, fetched again when the registration changes (a first flight
  // in an aircraft is not in the hangar yet).
  $effect(() => {
    const key = `${reg}|${status.forced_aircraft ?? ""}`;
    if (key === loadedFor) return;
    loadedFor = key;
    api.hangar().then((h) => (fleet = h ?? []));
  });

  /** The hangar aircraft: the one chosen with Fly this aircraft, else by registration. */
  const item = $derived(
    (status.forced_aircraft != null ? fleet.find((a) => a.id === status.forced_aircraft) : undefined) ??
      (reg ? fleet.find((a) => a.registration.toUpperCase() === reg) : undefined) ??
      null,
  );

  $effect(() => {
    const it = item;
    photo = null;
    if (it?.has_photo) api.aircraftPhoto(it.id).then((p) => (photo = p ?? null));
  });

  const type = $derived(item?.icao_type || ac?.icao_type || plan?.aircraft_icao || "");
  const model = $derived(item ? modelName(item) : ac ? modelName({ icao_type: ac.icao_type, title: ac.title }) : plan?.aircraft_name || "");
  const callsign = $derived((plan?.callsign || "").trim().toUpperCase());
  const flightNo = $derived(plan ? `${plan.airline}${plan.flight_number}`.trim().toUpperCase() : "");
  const airline = $derived.by(() => {
    const code = callsign.match(/^([A-Z]{3})\d/)?.[1];
    return (code && findAirline(code)?.name) || airlineLook(item?.airline)?.name || "";
  });
  const headline = $derived(callsign || item?.registration || reg || type || "Your flight");
  const shownReg = $derived(item?.registration || reg);

  const from = $derived(status.dep ? { icao: status.dep.icao, name: status.dep.name } : plan ? { icao: plan.origin.icao, name: plan.origin.name } : null);
  const to = $derived(plan ? { icao: plan.destination.icao, name: plan.destination.name } : null);

  const t = $derived(status.telemetry);
  const live = $derived(status.live);
  const now = $derived(t?.t_ms || Date.now());

  /** Great-circle distance left to the destination, in nautical miles. */
  const left = $derived.by(() => {
    if (!plan || !t || (!plan.destination.lat && !plan.destination.lon)) return null;
    const r = Math.PI / 180;
    const d = plan.destination;
    const h =
      Math.sin(((d.lat - t.lat) * r) / 2) ** 2 +
      Math.cos(t.lat * r) * Math.cos(d.lat * r) * Math.sin(((d.lon - t.lon) * r) / 2) ** 2;
    return 2 * 3440.065 * Math.asin(Math.sqrt(h));
  });
  const flown = $derived(live?.distance_nm ?? 0);
  const progress = $derived(left == null ? null : flown + left > 0 ? Math.min(1, flown / (flown + left)) : 0);
  /** Minutes to go at the present ground speed, once moving fast enough. */
  const toGo = $derived(left != null && t && !t.on_ground && t.gs_kt > 40 ? (left / t.gs_kt) * 60 : null);
  const since = $derived(live?.takeoff_ms ?? live?.block_out_ms ?? null);
  const arrived = $derived(status.phase === "arrived" || status.phase === "taxi_in" || status.phase === "landing_rollout");

  const nm = (v: number) => `${distance(v).value} ${distance(v).unit}`;
</script>

<article class="pass" class:active={status.phase !== "preflight" && status.phase !== "arrived"} aria-label="Boarding pass">
  <header>
    <div class="who">
      <span class="label">Flight</span>
      <span class="cs num" class:plain={!callsign}>{headline}</span>
      <span class="sub">{[airline, flightNo !== callsign ? flightNo : ""].filter(Boolean).join(" · ") || model || ac?.title || "Aircraft not reported yet"}</span>
    </div>
    <div class="what">
      {#if shownReg}<span class="reg num">{shownReg}</span>{/if}
      {#if type}<span class="badge num" title={model}>{type}</span>{/if}
    </div>
  </header>

  {#if item}
    <div class="pic">
      <AircraftPicture {item} own={photo} variant="banner" />
    </div>
  {/if}

  <div class="route">
    <div class="end">
      <span class="label">From</span>
      <span class="icao num">{from?.icao || "––––"}</span>
      <span class="name" title={from?.name}>{from?.name || "Set at block out"}</span>
    </div>
    <div class="mid" aria-hidden="true">
      <span class="line"></span>
      <span class="plane">
        <svg viewBox="0 0 24 24"><path d="M2.5 13.5l7-1.5 5-8h2.5l-2.5 8 5.5-.5 1.8-2.5H23l-1.2 4 1.2 4h-1.2L20 14.5l-5.5-.5 2.5 8h-2.5l-5-8-7-1.5z" /></svg>
      </span>
      <span class="line"></span>
    </div>
    <div class="end to">
      <span class="label">To</span>
      <span class="icao num" class:none={!to}>{to?.icao || "––––"}</span>
      <span class="name" title={to?.name}>{to?.name || "No destination"}</span>
    </div>
  </div>

  <div class="tear" aria-hidden="true"></div>

  <div class="stub">
    <dl>
      <div>
        <dt>Out</dt>
        <dd class="num">{live?.block_out_ms ? zulu(live.block_out_ms) : "At gate"}</dd>
      </div>
      <div>
        <dt>Airborne</dt>
        <dd class="num">{live?.takeoff_ms ? zulu(live.takeoff_ms) : "–"}</dd>
      </div>
      <div>
        <dt>{arrived ? "Altn" : "ETA"}</dt>
        <dd class="num">{arrived ? plan?.alternate || "–" : toGo != null ? zulu(now + toGo * 60000) : "–"}</dd>
      </div>
    </dl>
    <span class="code" aria-hidden="true"></span>
  </div>

  {#if progress != null}
    <div class="progress">
      <div class="bar">
        <span class="done" style:width="{progress * 100}%"></span>
        <span class="at" style:left="{progress * 100}%">
          <svg viewBox="0 0 24 24"><path d="M2.5 13.5l7-1.5 5-8h2.5l-2.5 8 5.5-.5 1.8-2.5H23l-1.2 4 1.2 4h-1.2L20 14.5l-5.5-.5 2.5 8h-2.5l-5-8-7-1.5z" /></svg>
        </span>
      </div>
      <div class="legend num">
        <span>{nm(flown)}{since ? `, ${hhmm((now - since) / 60000)} ago` : ""}</span>
        <span>{nm(left ?? 0)}{toGo != null ? `, in ${hhmm(toGo)}` : ""}</span>
      </div>
    </div>
  {:else if flown > 0}
    <div class="progress">
      <div class="legend num"><span>{nm(flown)} flown{since ? `, ${hhmm((now - since) / 60000)} ago` : ""}</span></div>
    </div>
  {/if}
</article>

<style>
  .pass {
    --edge: var(--panel);
    position: relative;
    border-radius: 12px;
    background: var(--panel-2);
    border: 1px solid var(--rule);
    overflow: hidden;
  }
  .pass::before {
    content: "";
    position: absolute;
    inset: 0 0 auto;
    height: 3px;
    background: var(--panel-3);
    transition: background 0.4s;
    z-index: 1;
  }
  .pass.active::before {
    background: var(--amber);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 14px 10px;
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
    padding-top: 13px;
  }
  .label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--ink-faint);
  }
  .cs {
    font-family: var(--cond);
    font-size: 27px;
    font-weight: 700;
    line-height: 1.05;
    color: var(--amber);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cs.plain {
    color: var(--ink);
  }
  .sub {
    font-size: 13px;
    color: var(--ink-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .reg {
    font-size: 18px;
    font-weight: 700;
    line-height: 1.1;
  }
  .badge {
    margin-top: 4px;
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 11.5px;
    font-weight: 700;
    color: #fff;
    background: #2f5fa8;
  }
  .pic {
    position: relative;
    height: 140px;
  }
  .route {
    display: grid;
    grid-template-columns: 1fr 64px 1fr;
    align-items: center;
    gap: 6px;
    padding: 12px 14px 14px;
  }
  .end {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .end.to {
    align-items: flex-end;
    text-align: right;
  }
  .icao {
    font-family: var(--cond);
    font-size: 34px;
    font-weight: 700;
    line-height: 1;
    letter-spacing: 0.02em;
    margin-top: 2px;
  }
  .icao.none {
    color: var(--ink-faint);
  }
  .name {
    margin-top: 3px;
    font-size: 12px;
    color: var(--ink-dim);
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .mid {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .mid .line {
    flex: 1;
    border-top: 1px dashed #5a5f66;
  }
  .plane {
    width: 30px;
    height: 30px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--amber);
  }
  .plane svg,
  .at svg {
    width: 18px;
    height: 18px;
    fill: #1b1406;
    /* The icon points left: flip it to fly from origin to destination. */
    transform: scaleX(-1);
  }
  /* The perforation, with a notch cut out of each side. */
  .tear {
    position: relative;
    height: 0;
    margin: 0 12px;
    border-top: 2px dashed #3d4147;
  }
  .tear::before,
  .tear::after {
    content: "";
    position: absolute;
    top: -10px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--edge);
    border: 1px solid var(--rule);
  }
  .tear::before {
    left: -22px;
  }
  .tear::after {
    right: -22px;
  }
  .stub {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px 10px;
  }
  dl {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    margin: 0;
  }
  dt {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--ink-faint);
  }
  dd {
    margin: 2px 0 0;
    font-family: var(--cond);
    font-size: 19px;
    font-weight: 600;
  }
  /* A boarding-pass barcode, purely decorative. */
  .code {
    flex: none;
    width: 46px;
    height: 34px;
    opacity: 0.55;
    background: repeating-linear-gradient(
      90deg,
      var(--ink-dim) 0 2px,
      transparent 2px 3px,
      var(--ink-dim) 3px 4px,
      transparent 4px 6px,
      var(--ink-dim) 6px 9px,
      transparent 9px 10px
    );
  }
  .progress {
    padding: 2px 14px 14px;
  }
  .bar {
    position: relative;
    height: 4px;
    margin: 10px 9px 8px 0;
    border-radius: 2px;
    background: var(--panel-3);
  }
  .done {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 2px;
    background: var(--amber);
    transition: width 1s linear;
  }
  .at {
    position: absolute;
    top: 50%;
    width: 22px;
    height: 22px;
    transform: translate(-50%, -50%);
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--amber);
    box-shadow: 0 0 0 3px var(--panel-2);
    transition: left 1s linear;
  }
  .at svg {
    width: 14px;
    height: 14px;
  }
  .legend {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-size: 12px;
    color: var(--ink-dim);
  }
</style>
