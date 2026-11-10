// Browser-only stand-in for the Rust backend: one flight in progress and a
// short history, so the UI can be developed with `npm run ui`.

import { mockHoldLines, mockTaxiLayout } from "./taxi/mockLayout";
import { RANKS, type FlightPlan, type FlightRow, type FlightScore, type FullStatus, type PilotProfile, type Totals } from "./api";

const now = Date.now();
const track: [number, number, number, number, number][] = Array.from({ length: 40 }, (_, i) => [
  -122.3093 - i * 0.004,
  47.449 - i * 0.03,
  Math.min(24000, i * 900),
  Math.min(420, 140 + i * 12),
  now - (40 - i) * 30_000,
]);

/** The preview's PFD feed. */
let pfdUntil = 0;
let pfdTimer: ReturnType<typeof setInterval> | undefined;

/** Screenshots deleted in this preview. */
const deletedShots = new Set<number>();

/** A sample SimBrief plan (Nice to Florence), for previews. */
export const samplePlan: FlightPlan = {
  source: "simbrief", id: "1", generated_ms: now - 25 * 60_000, callsign: "AUA11", airline: "AUA", flight_number: "11",
  origin: { icao: "LFMN", name: "Nice Cote d'Azur", lat: 43.6584, lon: 7.2159, runway: "04R" },
  destination: { icao: "LIRQ", name: "Firenze Peretola", lat: 43.81, lon: 11.2051, runway: "05" },
  alternate: "LIRP", route: "NIZ UM728 BASTI UN853 ELB UL865 PEPAT", cruise_ft: 34000, distance_nm: 228, ete_min: 52,
  aircraft_icao: "A320", aircraft_name: "A320-200", registration: "OE-LZE", block_fuel_kg: 5120, sched_out_ms: now, sched_in_ms: null,
  fixes: [
    { ident: "NIZ", kind: "vor", lat: 43.7726, lon: 7.2551, alt_ft: 8000, stage: "CLB", procedure: true },
    { ident: "BASTI", kind: "wpt", lat: 43.31, lon: 8.62, alt_ft: 30000, stage: "CLB", procedure: false },
    { ident: "ELB", kind: "vor", lat: 42.7296, lon: 10.3944, alt_ft: 34000, stage: "CRZ", procedure: false },
    { ident: "PEPAT", kind: "wpt", lat: 43.42, lon: 10.86, alt_ft: 14000, stage: "DSC", procedure: false },
    { ident: "PRT", kind: "vor", lat: 43.82, lon: 11.03, alt_ft: 5000, stage: "DSC", procedure: true },
  ],
};
const ofpText = `<div style="line-height:14px"><pre><b>[ OFP ]</b>
--------------------------------------------------------------------
AUA0011   28OCT2026    LFMN-LIRQ   A320 OELZE   RELEASE 2328 27OCT26
OFP 1      NICE/COTE D'AZUR-PERETOLA
                                       WX PROG 2800 2803   OBS 2718 2718

  ATC C/S   AUA11        LFMN/NCE   LIRQ/FLR      CRZ SYS      CI 24
28OCT2026   OELZE        0000/0020  0057/0105     GND DIST       215
FENIX A320 AUSTRIAN / CFM56-5B4/P   STA  0105     AIR DIST       204

MAXIMUM    TOW  73500  LAW  64500  ZFW  61000      AVG WIND  293/018
ESTIMATED  TOW  65756  LAW  64014  ZFW  60677      AVG W/C      P017
--------------------------------------------------------------------
         PLANNED FUEL
FUEL           ARPT   FUEL   TIME
TRIP            FLR   1742   0037
CONT 15 MIN            697   0015
ALTN            FCO   1581   0037
FINRES                1059   0030
--------------------------------------------------------------------
BLOCK FUEL      NCE   5309
</pre><img src="x" onerror="alert(1)"><script>alert(2)</script></div>`;
const sampleOfp = {
  html: ofpText, units: "kgs",
  fpl: "(FPL-AUA11-IS\n-A320/M-SDE2E3FGHIJ1RWY/LB1\n-LFMN0010\n-N0450F260 BASI8A BASIP DCT ANAKI DCT OTMUV DCT BEROK Y138 GAPMI GAPM1A\n-LIRQ0037 LIRF\n-PBN/A1B1C1D1O1S2 DOF/261028 REG/OELZE)",
  times: { sched_out: Math.round(now / 1000), sched_off: Math.round(now / 1000) + 1200, sched_on: Math.round(now / 1000) + 3420, sched_in: Math.round(now / 1000) + 3900, est_time_enroute: 2220, taxi_out: 1200, taxi_in: 480, est_block: 3900, reserve_time: 1800, endurance: 7140 },
  weights: { oew: 42600, pax_count: 168, cargo: 1200, payload: 18077, est_zfw: 60677, max_zfw: 61000, est_tow: 65756, max_tow: 73500, est_ldw: 64014, max_ldw: 64500, est_ramp: 65986 },
  fuel: { taxi: 230, enroute_burn: 1742, contingency: 697, alternate_burn: 1581, reserve: 1059, extra: 0, min_takeoff: 5079, plan_takeoff: 5079, plan_ramp: 5309, plan_landing: 3337, avg_fuel_flow: 2788 },
  weather: [["Departure METAR", "LFMN 272330Z 33010KT CAVOK 15/08 Q1020 NOSIG"], ["Destination TAF", "TAF LIRQ 272300Z 2800/2809 VRB03KT 9999 FEW030\nBECMG 2806/2808 4000 BR"]] as [string, string][],
  notams: [{ id: "A3317/26", location: "LFMN", text: "RWY 04L/22R CLOSED DUE TO WORKS. 2300-0500." }, { id: "B1102/26", location: "LIRQ", text: "ILS RWY 05 GP UNSERVICEABLE." }],
  log: [
    { ident: "BASIP", airway: "BASI8A", stage: "CLB", alt_ft: 12000, track_mag: 98, dist_leg: 22, time_total: 360, fuel_onboard: 4600, wind_dir: 290, wind_kt: 14, oat: -9, tas: 330, gs: 344, mach: "", winds: [[10000, 285, 12, -5], [18000, 290, 16, -19], [26000, 293, 18, -33], [34000, 295, 24, -49]] },
    { ident: "ANAKI", airway: "DCT", stage: "CRZ", alt_ft: 26000, track_mag: 76, dist_leg: 48, time_total: 900, fuel_onboard: 3900, wind_dir: 293, wind_kt: 18, oat: -33, tas: 448, gs: 466, mach: "0.74", winds: [[10000, 280, 10, -4], [18000, 288, 15, -18], [26000, 292, 19, -33], [34000, 296, 26, -50]] },
    { ident: "OTMUV", airway: "DCT", stage: "CRZ", alt_ft: 26000, track_mag: 92, dist_leg: 55, time_total: 1320, fuel_onboard: 3500, wind_dir: 296, wind_kt: 17, oat: -32, tas: 448, gs: 464, mach: "0.74", winds: [[10000, 279, 9, -3], [18000, 286, 14, -17], [26000, 296, 17, -32], [34000, 298, 25, -49]] },
    { ident: "GAPMI", airway: "Y138", stage: "DSC", alt_ft: 11000, track_mag: 140, dist_leg: 40, time_total: 1980, fuel_onboard: 3380, wind_dir: 300, wind_kt: 8, oat: -6, tas: 300, gs: 306, mach: "", winds: [[10000, 300, 8, -5], [18000, 300, 12, -18], [26000, 300, 16, -32], [34000, 300, 22, -49]] },
  ],
};
const scene = typeof location !== "undefined" ? new URLSearchParams(location.search).get("mock") : null;

/** The boarding pass preview: Seattle to Portland, part way. */
const passPlan: FlightPlan = {
  ...samplePlan,
  callsign: "ASA492", airline: "AS", flight_number: "492", alternate: "KSEA",
  origin: { icao: "KSEA", name: "Seattle-Tacoma International Airport", lat: 47.449, lon: -122.3093, runway: "16L" },
  destination: { icao: "KPDX", name: "Portland International Airport", lat: 45.5887, lon: -122.5975, runway: "10R" },
  fixes: [], ofp: null,
};

const status: FullStatus = {
  waiting_for_pilot: scene === "waiting" || scene === "plan",
  armed: false,
  forced_aircraft: null,
  plan: scene === "plan" ? samplePlan : scene === "pass" ? passPlan : null,
  link: scene === "nosim" ? "waiting" : "connected",
  sim: "msfs2024",
  sim_label: "MSFS 2024",
  sim_version: "12.1",
  notice: null,
  aircraft: { title: "Airbus A320neo", icao_type: "A20N", registration: scene === "pass" ? "D-AINA" : "N320FD", engine_count: 2, engine_type: "jet" },
  phase: "airborne",
  phase_label: "Airborne",
  live: scene && scene !== "pass" ? null : {
    phase: "airborne",
    start_ms: now - 50 * 60_000,
    block_out_ms: now - 44 * 60_000,
    takeoff_ms: now - 36 * 60_000,
    distance_nm: 72.4,
    max_alt_ft: 21000,
    fuel_used_kg: 1180,
    dep_lat: 47.449,
    dep_lon: -122.3093,
    last_landing: null,
    track_points: 40,
  },
  dep: { icao: "KSEA", name: "Seattle–Tacoma International Airport", country: "US", region: "US-WA" },
  telemetry: {
    t_ms: now,
    lat: track[39][1],
    lon: track[39][0],
    alt_msl_ft: 20480,
    alt_agl_ft: 20100,
    gs_kt: 412,
    ias_kt: 280,
    hdg_true_deg: 187,
    vs_fpm: -1200,
    on_ground: false,
    engines_running: 2,
    fuel_kg: 4820,
    paused: false,
    sim_rate: 1,
  },
  last_landing: null,
  demo: false,
  session: 1,
  track_tail: [],
  track: scene === "plan" ? [] : track,
};

function row(id: number, days: number, dep: string, arr: string, type: string, min: number, fpm: number, grade: string): FlightRow {
  const start = now - days * 86_400_000;
  return {
    id, sim: "msfs2024", status: "complete", aircraft_title: type, icao_type: type, registration: "N320FD",
    callsign: id === 3 ? "SWA1016" : "", dep_icao: dep, dep_name: dep, dep_country: "US", arr_icao: arr, arr_name: arr, arr_country: "US",
    start_ms: start, end_ms: start + min * 60_000, block_out: start, takeoff: start + 6e5, landing: start + (min - 8) * 60_000,
    block_in: start + min * 60_000, total_min: min, distance_nm: min * 4.2, max_alt_ft: 31000, fuel_used_kg: min * 40,
    landing_rate_fpm: fpm, landing_g: 1.2, landing_grade: grade, landings: 1,
  };
}

const flights: FlightRow[] = [
  row(3, 1, "KPDX", "KSFO", "A20N", 104, -142, "Smooth"),
  row(2, 3, "EGLL", "LFPG", "A320", 71, -54, "Butter"),
  row(1, 6, "CYVR", "KSEA", "B738", 52, -322, "Hard"),
];

const totals: Totals = { flights: 3, minutes: 227, distance_nm: 953, landings: 3, airports: 6, countries: 3 };

const AIRPORTS = [
  { icao: "KABQ", name: "Albuquerque International Sunport", country: "United States", lat: 35.0402, lon: -106.6092 },
  { icao: "KLAS", name: "Harry Reid International Airport", country: "United States", lat: 36.084, lon: -115.1537 },
  { icao: "KDTW", name: "Detroit Metropolitan Wayne County Airport", country: "United States", lat: 42.2124, lon: -83.3534 },
  { icao: "KSEA", name: "Seattle-Tacoma International Airport", country: "United States", lat: 47.449, lon: -122.3093 },
  { icao: "KPDX", name: "Portland International Airport", country: "United States", lat: 45.5887, lon: -122.5975 },
  { icao: "EGLL", name: "London Heathrow Airport", country: "United Kingdom", lat: 51.4706, lon: -0.4619 },
  { icao: "EGKK", name: "London Gatwick Airport", country: "United Kingdom", lat: 51.1481, lon: -0.1903 },
];

/** A pilot with a few months of history, for the profile page. */
let mockBanner: string | null = null;
let mockPhoto: string | null = null;
function mockProfile(): PilotProfile {
  const routes: [string, string, string, number][] = [
    ["KSEA", "KPDX", "A20N", 48], ["KPDX", "KSFO", "A20N", 95], ["KSFO", "KLAX", "B738", 72], ["KLAX", "KLAS", "B738", 55],
    ["KLAS", "KDEN", "B738", 98], ["KDEN", "KORD", "A20N", 142], ["KPAO", "KSQL", "C172", 22], ["EGLL", "LFPG", "A320", 64],
    ["LFPG", "LIRF", "A320", 118], ["LIRF", "LGAV", "A320", 125], ["EDDF", "LOWW", "A20N", 70], ["LOWW", "LKPR", "A20N", 45],
  ];
  const n = 26;
  let rating = 1000, peak = 1000, rated = 0, xp = 0, sl = 0, ss = 0, sp = 0;
  const history = [{ t_ms: now - 150 * 864e5, rating }];
  const out: FlightScore[] = [];
  const expected = (r: number) => ([[2000, 95], [1750, 90], [1500, 85], [1250, 80], [1000, 75], [750, 70], [500, 60]] as const).find(([m]) => r >= m)?.[1] ?? 50;
  const bonus = (m: number) => ([[480, 20], [360, 18], [240, 15], [120, 12], [60, 8], [30, 5], [15, 2]] as const).find(([x]) => m >= x)?.[1] ?? 0;
  for (let i = 0; i < n; i++) {
    const [dep, arr, type, min] = routes[i % routes.length];
    const start = now - (150 - i * 5.6) * 864e5;
    const unrated = i === 2 ? "Not enough data: too few positions in the air" : i === 17 ? "Ended before landing" : null;
    // The first flights came from Volanta: bank and pitch from its positions, no bounces or G in flight.
    const imported = i < 8;
    const s: FlightScore = {
      flight_id: 100 + i, start_ms: start, dep_icao: dep, arr_icao: arr, icao_type: type, performance: null, landing: null, safety: null,
      unrated, airborne_min: min, xp_base: 0, xp_bonus: 0, xp: 0, rating_before: null, rating_after: null, expected: null, k: null,
    };
    if (!unrated) {
      const wobble = Math.sin(i * 1.7) * 9 + Math.cos(i * 0.6) * 5;
      const vs = -Math.round(110 + Math.abs(Math.sin(i * 2.3)) * 260);
      const bounces = i % 7 === 3 ? 1 : 0;
      const rate = Math.max(0, Math.min(100, 100 - (Math.abs(vs) - 60) / 6)), g = 92 - (i % 4) * 4, bo = bounces ? 60 : 100;
      const landing = imported ? (rate * 0.35 + g * 0.2) / 0.55 : (rate * 0.35 + g * 0.2 + bo * 0.2) / 0.75;
      const safety = Math.min(100, 93 + wobble * 0.6);
      const p = landing / 2 + safety / 2;
      const e = expected(rating), k = rated < 5 ? 48 : rated < 20 ? 40 : rated < 50 ? 32 : 24;
      const after = rating + (k * (p - e)) / 100;
      Object.assign(s, {
        performance: p, rating_before: rating, rating_after: after, expected: e, k, xp_base: Math.round(p),
        landing: { score: Math.min(100, landing * (1 + ((i * 3) % 14) / 100)), rate: { score: rate, weight: imported ? 0.35 / 0.55 : 0.35 / 0.75 }, g: { score: g, weight: imported ? 0.2 / 0.55 : 0.2 / 0.75 }, bounce: imported ? null : { score: bo, weight: 0.2 / 0.75 }, source: imported ? "volanta" : "sim", crosswind_kt: (i * 3) % 14, crosswind_bonus: ((i * 3) % 14) / 100, base_score: landing, vs_fpm: vs, g_peak: 1.08 + (i % 4) * 0.07, bounces, discarded: i === 12 ? 1 : 0 },
        safety: { score: safety, g: imported ? null : { score: Math.min(100, safety + 2), weight: 0.5 }, bank: { score: Math.min(100, safety - 1), weight: imported ? 0.5 : 0.25 }, pitch: { score: 100, weight: imported ? 0.5 : 0.25 }, smoothness: imported ? null : { score: Math.min(100, safety + 4), weight: 0.2 }, approach: { score: Math.round(80 + wobble), weight: 0.2 }, speed: imported ? null : { score: i % 5 === 2 ? 88 : 100, weight: 0.1 }, roll_rate_max: 6 + (i % 4), pitch_rate_max: 2, g_rate_max: 0.3, approach_sink_max: 820 + (i % 5) * 90, approach_bank_max: 4 + (i % 6) * 2, approach_speed_dev: 6, speed_max_low: i % 5 === 2 ? 262 : 248, aerobatic: false, source: imported ? "positions" : "recorded", g_max: 1.35, g_min: 0.72, bank_max: 31 + (i % 5) * 4, pitch_max: 14, pitch_min: -6 },
      });
      rating = after;
      peak = Math.max(peak, rating);
      rated++;
      sl += landing; ss += safety; sp += p;
      history.push({ t_ms: start, rating });
    } else if (unrated !== "Ended before landing") s.xp_base = 50;
    if (s.xp_base) s.xp_bonus = bonus(min);
    s.xp = s.xp_base + s.xp_bonus;
    xp += s.xp;
    out.push(s);
  }
  // A few earlier years of flying, so the rank is not the very first one.
  xp += 1680;
  const ri = RANKS.reduce((at, r, i) => (xp >= r.min_xp ? i : at), 0);
  const months = Array.from({ length: 12 }, (_, i) => {
    const d = new Date(now);
    d.setUTCMonth(d.getUTCMonth() - 11 + i);
    const busy = i >= 7 ? [380, 520, 290, 610, 450][i - 7] : i === 3 ? 90 : 0;
    return { month: `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, "0")}`, minutes: busy, flights: Math.round(busy / 80) };
  });
  return {
    pilot: { name: "Alex Pilot", pilot_id: "FD-0001", home_icao: "KSEA", nationality: "United States", photo_url: mockPhoto, signature_svg: null },
    banner: mockBanner,
    totals: { flights: 64, minutes: 6810, distance_nm: 31840, landings: 71, airports: 38, countries: 9 },
    stats: {
      first_flight_ms: now - 700 * 864e5,
      longest: { id: 105, dep_icao: "KDEN", arr_icao: "KORD", start_ms: now - 120 * 864e5, value: 168 },
      farthest: { id: 109, dep_icao: "LIRF", arr_icao: "LGAV", start_ms: now - 80 * 864e5, value: 571 },
      softest: { id: 118, dep_icao: "LOWW", arr_icao: "LKPR", start_ms: now - 20 * 864e5, value: -38 },
      avg_landing_fpm: -164,
      night_minutes: 820,
      aircraft_types: 7,
      top_types: [{ name: "A20N", count: 24, minutes: 2410 }, { name: "B738", count: 15, minutes: 1620 }, { name: "A320", count: 11, minutes: 1250 }, { name: "C172", count: 9, minutes: 380 }],
      top_airports: [{ name: "KSEA", count: 18, minutes: 0 }, { name: "KPDX", count: 12, minutes: 0 }, { name: "EDDF", count: 9, minutes: 0 }, { name: "KSFO", count: 8, minutes: 0 }],
      months,
    },
    rating: {
      rating, rating_peak: peak, expected: expected(rating), rated_flights: rated, xp, rank: RANKS[ri], next_rank: RANKS[ri + 1] ?? null,
      avg_performance: sp / rated, avg_landing: sl / rated, avg_safety: ss / rated, history, flights: out.reverse(),
    },
  };
}

let mockEfb = false;

export async function mockCall<T>(cmd: string, _args?: Record<string, unknown>): Promise<T> {
  if (cmd === "set_banner") mockBanner = (_args?.dataUrl as string | null) ?? null;
  if (cmd === "set_pilot_photo") mockPhoto = (_args?.dataUrl as string | null) ?? null;
  if (cmd === "get_flight") {
    const p = mockProfile();
    const want = scene === "imported" ? p.rating.flights.find((f) => f.landing?.source === "volanta") : scene === "unrated" ? p.rating.flights.find((f) => f.unrated) : p.rating.flights.find((f) => f.performance != null);
    const r = p.rating;
    return {
      flight: flights[0],
      track: track.map(([lon, lat, alt_ft, gs_kt, t_ms]) => ({ t_ms, lat, lon, alt_ft, gs_kt, vs_fpm: 0, hdg_deg: 180, on_ground: false })),
      landings: [{ t_ms: now, lat: 37.6, lon: -122.37, vs_fpm: -142, vs_source: "sim", g_peak: 1.18, pitch_deg: 3.2, bank_deg: 0.4, gs_kt: 128, ias_kt: 131, hdg_deg: 280, headwind_kt: 8, crosswind_kt: 4, bounces: 0, touch_and_go: false }],
      plan: null,
      score: want ?? null,
      standing: { rating: r.rating, xp: r.xp, rank: r.rank, next_rank: r.next_rank },
    } as T;
  }
  if (cmd === "get_pilot_profile") {
    const p = mockProfile();
    if (scene === "rookie") {
      Object.assign(p.rating, { rating: 1000, rating_peak: 1000, rated_flights: 0, xp: 0, rank: RANKS[0], next_rank: RANKS[1], avg_performance: null, avg_landing: null, avg_safety: null, history: [{ t_ms: now, rating: 1000 }], flights: [] });
      p.totals = { flights: 0, minutes: 0, distance_nm: 0, landings: 0, airports: 0, countries: 0 };
      p.stats = { ...p.stats, longest: null, farthest: null, softest: null, avg_landing_fpm: null, top_types: [], top_airports: [], months: p.stats.months.map((m) => ({ ...m, minutes: 0, flights: 0 })) };
    }
    return p as T;
  }
  if (cmd === "airport_at") {
    const lat = Number(_args?.lat), lon = Number(_args?.lon);
    const near = AIRPORTS.map((a) => ({ a, d: Math.hypot(a.lat - lat, (a.lon - lon) * Math.cos((lat * Math.PI) / 180)) })).sort((x, y) => x.d - y.d)[0];
    return (near && near.d < 0.15 ? near.a : null) as T;
  }
  if (cmd === "efb_info" || cmd === "efb_install" || cmd === "efb_uninstall") {
    if (cmd !== "efb_info") mockEfb = cmd === "efb_install";
    const found = scene !== "nocommunity" || (cmd === "efb_install" && !!_args?.folder);
    return {
      installed: mockEfb && found,
      version: mockEfb && found ? "0.1.36" : null,
      community: found ? (_args?.folder as string) || "C:\\Users\\Pilot\\AppData\\Local\\Packages\\Microsoft.Limitless_8wekyb3d8bbwe\\LocalCache\\Packages\\Community" : null,
      manual: false,
      error: found ? null : "MSFS 2024's Community folder was not found. Choose it below.",
    } as T;
  }
  // Starting, cancelling and planning a flight change the status, as in the app.
  if (cmd === "start_flight" || cmd === "cancel_flight" || cmd === "set_plan") {
    if (cmd === "start_flight") {
      status.armed = true;
      if (_args?.plan) status.plan = _args.plan as FlightPlan;
    } else if (cmd === "cancel_flight") {
      if (!status.live) status.plan = null;
      status.armed = false;
    } else {
      status.plan = (_args?.plan as FlightPlan | null) ?? null;
    }
    setTimeout(pushStatus, 50);
    return null as T;
  }
  if (cmd === "flight_screenshots") {
    const f = Number(_args?.id ?? 0);
    const base = now - 86_400_000 * 2;
    return (f % 2 ? [] : [0, 1, 2].map((i) => ({ id: f * 10 + i, path: `C:\\Users\\you\\Pictures\\Screenshots\\Flight ${f} ${i + 1}.png`, taken_ms: base + (i + 1) * 900_000 }))) as unknown as T;
  }
  if (cmd === "photos") {
    // Three flights' worth around the Pacific Northwest, and one not in a flight.
    const at: [number, number, number, string, string][] = [
      [47.6, -122.9, 8500, "KSEA", "KPDX"], [46.9, -122.7, 11000, "KSEA", "KPDX"], [46.1, -122.8, 9000, "KSEA", "KPDX"],
      [45.9, -121.6, 13000, "KPDX", "KBOI"], [44.6, -118.4, 35000, "KPDX", "KBOI"],
      [48.5, -123.0, 2500, "KBFI", "KFHR"], [48.52, -122.98, 1500, "KBFI", "KFHR"], [48.4, -122.7, 3500, "KBFI", "KFHR"],
    ];
    const list = at.map(([lat, lon, alt, dep, arr], i) => ({
      id: 500 + i,
      path: `C:\\Program Files (x86)\\Steam\\userdata\\1234\\760\\remote\\2537590\\screenshots\\2026110${7 - Math.floor(i / 3)}_1${i}2233_1.jpg`,
      taken_ms: now - (Math.floor(i / 3) * 86_400_000 + (3 - (i % 3)) * 900_000),
      flight_id: 40 - Math.floor(i / 3),
      dep, arr, lat, lon, alt_ft: alt,
    }));
    list.unshift({ id: 499, path: "C:\\Users\\you\\Pictures\\Screenshots\\Menu.png", taken_ms: now - 300_000, flight_id: null as unknown as number, dep: null as unknown as string, arr: null as unknown as string, lat: null as unknown as number, lon: null as unknown as number, alt_ft: null as unknown as number });
    return list.filter((p) => !deletedShots.has(p.id)) as unknown as T;
  }
  if (cmd === "delete_screenshot") {
    deletedShots.add(Number(_args?.id));
    return null as T;
  }
  if (cmd === "screenshots_since") {
    return [{ id: 901, path: "C:\\Users\\you\\Pictures\\Screenshots\\Climb out.png", taken_ms: now - 600_000 }] as T;
  }
  if (cmd === "backup_save") return "C:\\Users\\you\\Documents\\FlightDeck backup 2026-11-06.fdbackup" as T;
  if (cmd === "backup_choose_folder") return "C:\\Users\\you\\OneDrive\\FlightDeck" as T;
  if (cmd === "backup_now") return "C:\\Users\\you\\OneDrive\\FlightDeck\\FlightDeck backup.fdbackup" as T;
  if (cmd === "backup_restore_pick") return { flights: 412, aircraft: 23, last_flight_ms: now - 86_400_000, schema: 12 } as T;
  if (cmd === "backup_forget_folder" || cmd === "backup_restore_cancel" || cmd === "restart_app") return null as T;
  if (cmd === "pick_screenshot_folder") return "C:\\Users\\you\\Pictures\\Screenshots" as T;
  if (cmd === "set_screenshot_folder") return null as T;
  if (cmd === "divert_options") {
    return [
      { ident: "KPAE", name: "Snohomish County (Paine Field)", city: "Everett", country: "US", lat: 47.906, lon: -122.281, distance_nm: 18.4, bearing: 350, longest_ft: 9010, surface: "Asphalt" },
      { ident: "KBFI", name: "Boeing Field", city: "Seattle", country: "US", lat: 47.53, lon: -122.302, distance_nm: 24.9, bearing: 175, longest_ft: 10000, surface: "Asphalt" },
      { ident: "KBLI", name: "Bellingham International", city: "Bellingham", country: "US", lat: 48.793, lon: -122.537, distance_nm: 41.2, bearing: 340, longest_ft: 6700, surface: "Asphalt" },
    ] as T;
  }
  if (cmd === "divert") {
    if (status.plan) {
      status.plan = { ...status.plan, diverted_from: status.plan.diverted_from ?? status.plan.destination.icao, destination: { icao: String(_args?.icao), name: "Diversion airport", lat: 47.9, lon: -122.28, runway: "" } };
      setTimeout(pushStatus, 50);
    }
    return null as T;
  }
  if (cmd === "preview_import") {
    const files = (_args?.files as { name: string }[]) ?? [];
    const routes = [["KSEA", "KPDX"], ["KPDX", "KSFO"], ["KSFO", "KLAX"], ["KSEA", "KPDX"], ["EGLL", "LFPG"]];
    return files.map((f, i) => {
      const [dep, arr] = routes[i % routes.length];
      const bad = f.name.includes("broken");
      return {
        name: f.name, dep: bad ? null : dep, arr: bad ? null : arr, start_ms: now - (i + 1) * 86_400_000, minutes: 45 + i * 7,
        icao_type: "A20N", registration: "N320FD", callsign: i % 2 ? "DAL864" : "",
        duplicate: bad ? null : i === 1 ? "Already imported" : i === 3 ? `Same flight as ${files[0].name} in this import` : i === 4 ? "Same day and airports as a flight in FlightDeck" : null,
        error: bad ? "Not a Volanta flight file" : null,
      };
    }) as T;
  }
  if (cmd === "import_volanta" || cmd === "import_elevatex") {
    const files = (_args?.files as { name: string }[]) ?? [];
    return { imported: files.map((f) => f.name), duplicates: [], failed: [] } as T;
  }
  if (cmd === "export_csv" || cmd === "export_all_volanta") {
    return { folder: cmd === "export_csv" ? "C:\\Users\\you\\Downloads\\FlightDeck export\\FlightDeck flights.csv" : "C:\\Users\\you\\Downloads\\FlightDeck export", count: 3 } as T;
  }
  if (cmd === "pln_text") return ["KSEA-KPDX.pln", "<?xml version=\"1.0\"?>"] as T;
  if (cmd === "save_pln") return "C:\\Users\\you\\Documents\\KSEA-KPDX.pln" as T;
  if (cmd === "pfd_watch") {
    pfdUntil = Date.now() + 6000;
    if (!pfdTimer) {
      const t0 = Date.now();
      pfdTimer = setInterval(() => {
        if (Date.now() > pfdUntil) {
          clearInterval(pfdTimer);
          pfdTimer = undefined;
          return;
        }
        // A steady climbing right turn over the Cascades, west of Mt Rainier.
        const t = (Date.now() - t0) / 1000;
        const hdg = (300 + t * 3) % 360;
        const frame = {
          t_ms: Date.now(), lat: 46.85 + Math.sin(t / 40) * 0.02, lon: -121.95 + Math.cos(t / 40) * 0.02,
          alt_msl_ft: 8200 + t * 8, alt_ft: 8210 + t * 8, agl_ft: 3100, ias_kt: 128 + Math.sin(t / 3) * 2, tas_kt: 146, gs_kt: 139,
          vs_fpm: 480 + Math.sin(t / 2) * 60, pitch_deg: 4.5 + Math.sin(t / 3), bank_deg: 18 + Math.sin(t / 4) * 3,
          hdg_deg: hdg, hdg_true_deg: (hdg + 15) % 360, track_true_deg: (hdg + 18) % 360, ball: Math.sin(t / 5) * 0.15, g_load: 1.05,
          on_ground: false, paused: false, baro_inhg: 29.92, sel_alt_ft: 10000, sel_hdg_deg: 20, sel_spd_kt: 130, sel_vs_fpm: 500, ap_on: true,
        };
        // Tests can pin the aircraft somewhere: window.__pfdOverride.
        Object.assign(frame, (window as unknown as { __pfdOverride?: object }).__pfdOverride ?? {});
        for (const cb of listeners.pfd ?? []) cb(frame);
      }, 66);
    }
    return null as T;
  }
  if (cmd === "us_obstacles" && typeof localStorage !== "undefined" && localStorage.getItem("fd.bigObst")) {
    // Tests: a file the size of the FAA's (about 100,000 obstacles over 200 ft), a few thousand near Seattle–Portland.
    let seed = 7;
    const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
    const features = Array.from({ length: 100000 }, (_, i) => {
      const near = i % 25 === 0;
      const lat = near ? 45.5 + rnd() * 2.1 : 25 + rnd() * 24;
      const lon = near ? -122.9 + rnd() * 0.8 : -124 + rnd() * 57;
      const agl = 200 + Math.floor(rnd() * 1500);
      return { type: "Feature", properties: { t: "TOWER", agl, amsl: agl + Math.floor(rnd() * 3000), lit: true, n: 1 }, geometry: { type: "Point", coordinates: [lon, lat] } };
    });
    return JSON.stringify({ type: "FeatureCollection", features }) as T;
  }
  if (cmd === "us_obstacles") {
    // Towers scattered along Seattle–Portland.
    const features = Array.from({ length: 60 }, (_, i) => {
      const t = (i + 0.5) / 60;
      const lat = 47.449 + (45.5887 - 47.449) * t + Math.sin(i * 7.3) * 0.02;
      const lon = -122.3093 + (-122.5975 + 122.3093) * t + Math.cos(i * 3.1) * 0.02;
      const agl = 200 + ((i * 137) % 1300);
      return { type: "Feature", properties: { t: "TOWER", agl, amsl: agl + 300 + ((i * 53) % 900), lit: i % 2 === 0, n: 1 }, geometry: { type: "Point", coordinates: [lon, lat] } };
    });
    return JSON.stringify({ type: "FeatureCollection", features }) as T;
  }
  if (cmd === "us_airspace") {
    // Rings like a Class B around KSEA and a Class D at KPDX.
    const ring = (lon: number, lat: number, r: number, cw = false) => {
      const pts = Array.from({ length: 73 }, (_, i) => {
        const a = ((cw ? -i : i) * 5 * Math.PI) / 180;
        return [lon + (r * Math.cos(a)) / Math.cos((lat * Math.PI) / 180), lat + r * Math.sin(a)];
      });
      return pts;
    };
    const f = (c: string, lo_t: string, hi_t: string, lo: number, hi: number, rings: number[][][]) => ({ type: "Feature", properties: { c, name: "", label: `${c}: ${lo_t}-${hi_t}`, lo, hi, lo_t, hi_t }, geometry: { type: "Polygon", coordinates: rings } });
    return JSON.stringify({
      type: "FeatureCollection",
      features: [
        f("B", "GND", "10000MSL", 0, 10000, [ring(-122.309, 47.449, 0.1)]),
        f("B", "3000MSL", "10000MSL", 3000, 10000, [ring(-122.309, 47.449, 0.2), ring(-122.309, 47.449, 0.1, true)]),
        f("C", "GND", "4000MSL", 0, 4000, [ring(-122.598, 45.589, 0.08)]),
        f("C", "2000MSL", "4000MSL", 2000, 4000, [ring(-122.598, 45.589, 0.17), ring(-122.598, 45.589, 0.08, true)]),
        f("D", "GND", "2900MSL", 0, 2900, [ring(-122.95, 45.54, 0.07)]),
      ],
    }) as T;
  }
  if (cmd === "wx_categories") {
    return {
      updated_ms: now,
      error: null,
      stations: AIRPORTS.map((a, i) => [a.icao, a.lon, a.lat, i % 4]),
      winds: AIRPORTS.map((a, i) => [a.icao, a.lon, a.lat, i % 7 === 3 ? -1 : (i * 47) % 360, [0, 4, 9, 14, 19, 27, 36][i % 7], i % 3 === 0 ? 25 : 0]),
    } as T;
  }
  if (cmd === "sua_airspace" || cmd === "world_airspace") {
    // A few made-up areas around Seattle (US) and Vancouver (outside).
    const circ = (lon: number, lat: number, r: number) => {
      const ring: [number, number][] = [];
      for (let i = 0; i <= 64; i++) ring.push([lon + (r / Math.cos((lat * Math.PI) / 180)) * Math.cos((i / 64) * Math.PI * 2), lat + r * Math.sin((i / 64) * Math.PI * 2)]);
      return { type: "Polygon", coordinates: [ring] };
    };
    const wobbly = (lon: number, lat: number, r: number) => {
      const ring: [number, number][] = [];
      for (let i = 0; i <= 160; i++) {
        const a = (i / 160) * Math.PI * 2;
        const rr = r * (1 + 0.05 * Math.sin(a * 23) + 0.03 * Math.sin(a * 41));
        ring.push([lon + (rr / Math.cos((lat * Math.PI) / 180)) * Math.cos(a), lat + rr * Math.sin(a)]);
      }
      return { type: "Polygon", coordinates: [ring] };
    };
    const f = (c: string, label: string, lo: number, hi: number, g: object) => {
      const [name, lim] = label.split(": ");
      const [lo_t, hi_t] = (lim ?? "").split("-");
      return { type: "Feature", properties: { c, name, label, lo, hi, lo_t, hi_t }, geometry: g };
    };
    const fc = (features: object[]) => JSON.stringify({ type: "FeatureCollection", features });
    if (cmd === "sua_airspace") return fc([f("R", "R-6703: GND-FL230", 0, 23000, circ(-122.55, 46.9, 0.12)), f("MOA", "OLYMPIC MOA: 6000MSL-FL180", 6000, 18000, wobbly(-122.05, 47.2, 0.13))]) as T;
    return { geojson: fc([f("C", "C: GND-2500AGL", 0, 2500, circ(-123.18, 49.19, 0.12)), f("G", "G: GND-2000AGL", 0, 2000, circ(-122.6, 49.4, 0.2))]), more: false } as T;
  }
  if (cmd === "sky_grid") {
    // Made-up weather systems: bands of cloud, poor visibility under them.
    const kind = (_args as { kind: string }).kind;
    const v = new Int16Array(360 * 181);
    for (let r = 0; r < 181; r++)
      for (let c = 0; c < 360; c++) {
        const lat = 90 - r;
        const w = Math.max(0, Math.sin((c / 360) * Math.PI * 8 + lat / 12) * Math.cos((lat / 90) * Math.PI * 2.5));
        v[r * 360 + c] = kind === "cloud" ? Math.round(Math.min(100, w * 130) * 10) : Math.round((24000 - Math.min(23500, w * 30000)) * 0.1);
      }
    const bytes = new Uint8Array(v.buffer);
    let s = "";
    for (let i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
    const now = Date.now();
    return { run_ms: now - 6 * 3600_000, valid_ms: now, nx: 360, ny: 181, t: btoa(s), error: null } as T;
  }
  if (cmd === "temperature_grid") {
    // Warm tropics, cold poles, colder over a few made-up mountain spots.
    const t = new Int16Array(360 * 181);
    for (let r = 0; r < 181; r++) {
      const lat = 90 - r;
      for (let c = 0; c < 360; c++) {
        const lv = (_args as { level?: string } | undefined)?.level ?? "sfc";
        const drop = lv === "sfc" ? 0 : ({ "850": 10, "700": 18, "500": 32, "400": 42, "300": 55, "250": 62, "200": 65 } as Record<string, number>)[lv] ?? 0;
        const base = 30 - 0.0085 * lat * lat - (lat > 0 ? 6 : 0) - drop;
        const wave = 5 * Math.sin((c / 360) * Math.PI * 6) * Math.cos((lat / 90) * Math.PI);
        t[r * 360 + c] = Math.round((base + wave) * 10);
      }
    }
    const bytes = new Uint8Array(t.buffer);
    let s = "";
    for (let i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
    const now = Date.now();
    return { run_ms: now - 6 * 3_600_000, valid_ms: now, nx: 360, ny: 181, t: btoa(s), error: null } as T;
  }
  if (cmd === "icing_grid") {
    // Made-up blobs of icing that move with the level; unknown over a
    // made-up high plateau at low levels.
    const fl = (_args as { fl: number }).fl;
    const v = new Uint8Array(360 * 181);
    for (let r = 0; r < 181; r++)
      for (let c = 0; c < 360; c++) {
        const lat = 90 - r;
        const lon = c > 180 ? c - 360 : c;
        if (fl <= 100 && lat > 30 && lat < 45 && lon > -115 && lon < -105) {
          v[r * 360 + c] = 255;
          continue;
        }
        const blob = (la: number, lo: number, rad: number, peak: number) => Math.max(0, peak * (1 - Math.hypot(lat - la, (lon - lo) * 0.7) / rad));
        const band = Math.abs(lat) > 30 && Math.abs(lat) < 65 ? 1 : 0.2;
        v[r * 360 + c] = Math.round(Math.min(100, band * (blob(47, -122 + fl / 50, 9, 95) + blob(42, -88, 7, 70) + blob(52, 10 - fl / 40, 10, 60) + blob(-45, 170, 8, 85))));
      }
    let s = "";
    for (let i = 0; i < v.length; i++) s += String.fromCharCode(v[i]);
    const now = Date.now();
    return { run_ms: now - 6 * 3_600_000, valid_ms: now - (now % 10_800_000), fl, product: "0p50", nx: 360, ny: 181, potential: btoa(s), known: 0.97 } as T;
  }
  if (cmd === "navaids") {
    // A few made-up aids around Portland and Seattle, and a waypoint grid.
    const a = (_args ?? {}) as { w: number; s: number; e: number; n: number; waypoints: boolean };
    const all = [
      { kind: "vor", ident: "BTG", region: "K1", lat: 45.7477, lon: -122.5921, freq_hz: 116_600_000, dme: true, dme_only: false },
      { kind: "vor", ident: "UBG", region: "K1", lat: 45.3524, lon: -123.0168, freq_hz: 117_000_000, dme: true, dme_only: false },
      { kind: "vor", ident: "SEA", region: "K1", lat: 47.4353, lon: -122.3098, freq_hz: 116_800_000, dme: true, dme_only: false },
      { kind: "vor", ident: "LTJ", region: "K1", lat: 45.7135, lon: -121.1006, freq_hz: 113_800_000, dme: false, dme_only: false },
      { kind: "ndb", ident: "LA", region: "K1", lat: 45.586, lon: -122.735, freq_hz: 332_000, dme: false, dme_only: false },
      { kind: "ndb", ident: "OS", region: "K1", lat: 45.65, lon: -122.85, freq_hz: 350_000, dme: false, dme_only: false },
      ...Array.from({ length: 60 }, (_, i) => ({
        kind: "waypoint",
        ident: ["HAMUR", "TMBRS", "COUGA", "YKM", "JOTBA", "MIGEL", "ROTRA", "BUFFY", "LAVAA", "SHIPZ"][i % 10] + (i >= 10 ? String.fromCharCode(65 + Math.floor(i / 10)) : ""),
        region: "K1",
        lat: 45.2 + (i % 8) * 0.12,
        lon: -123.2 + Math.floor(i / 8) * 0.17,
        freq_hz: null,
        dme: false,
        dme_only: false,
      })),
    ];
    const items = all.filter((x) => x.lat >= a.s && x.lat <= a.n && x.lon >= a.w && x.lon <= a.e && (a.waypoints || x.kind !== "waypoint"));
    return { items, more: false, known: all.length, waypoints: 60, world_ms: Date.now() - 86_400_000 } as T;
  }
  if (cmd === "navaids_refresh") return undefined as T;
  if (cmd === "wind_grid") {
    // A made-up world: a westerly jet in each hemisphere and an easterly
    // trade belt, stronger with height, with a wave so it curves.
    const lvl = String(_args?.level ?? "sfc");
    const k = { sfc: 0.25, "850": 0.4, "700": 0.55, "500": 0.75, "400": 0.9, "300": 1, "250": 1, "200": 0.9 }[lvl] ?? 0.5;
    const u = new Int16Array(360 * 181);
    const v = new Int16Array(360 * 181);
    for (let r = 0; r < 181; r++) {
      const lat = 90 - r;
      for (let c = 0; c < 360; c++) {
        const wave = Math.sin(((c * 4) / 360) * Math.PI * 2);
        const jet = 140 * Math.exp(-(((Math.abs(lat) - 40 - wave * 6) / 9) ** 2));
        const trade = -18 * Math.exp(-((lat / 14) ** 2));
        u[r * 360 + c] = Math.round(k * (jet + trade + 12));
        v[r * 360 + c] = Math.round(k * (30 * Math.cos(((c * 4) / 360) * Math.PI * 2) * Math.exp(-(((Math.abs(lat) - 40) / 14) ** 2)) * Math.sign(lat || 1)));
      }
    }
    const b64 = (a: Int16Array) => {
      const bytes = new Uint8Array(a.buffer);
      let s = "";
      for (let i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
      return btoa(s);
    };
    return { level: lvl, run_ms: now - 9 * 3600_000, valid_ms: now, nx: 360, ny: 181, u: b64(u), v: b64(v), error: null } as T;
  }
  if (cmd === "wx_advisories") {
    const sq = (x: number, y: number, w: number, h: number) => ({ type: "Polygon", coordinates: [[[x, y], [x + w, y], [x + w, y + h], [x, y + h], [x, y]]] });
    const f = (geometry: unknown, props: Record<string, unknown>) => ({ type: "Feature", geometry, properties: { qualifier: "", base: "", top: "", due_to: "", from_ms: now - 3600_000, to_ms: now + 3 * 3600_000, ...props } });
    return {
      updated_ms: now, error: null,
      features: [
        f(sq(-100, 33, 6, 4), { kind: "CONVECTIVE", group: "TS", hazard: "CONVECTIVE", name: "KKCI 45C", top: "45000", raw: "CONVECTIVE SIGMET 45C VALID UNTIL 0455Z TX OK AREA TS MOV FROM 24025KT. TOPS ABV FL450." }),
        f(sq(-125, 40, 8, 6), { kind: "AIRMET", group: "TURB", hazard: "TURB-HI", name: "TANGO", base: "FL240", top: "FL400", raw: "" }),
        f(sq(-110, 44, 7, 5), { kind: "AIRMET", group: "ICE", hazard: "ICE", name: "ZULU", base: "FL080", top: "FL200", raw: "" }),
        f(sq(-98, 46, 6, 4), { kind: "SIGMET", group: "ICE", hazard: "ICE", qualifier: "SEV", name: "KKCI ZULU 2", base: "12000", top: "22000", raw: "SIGMET ZULU 2 VALID UNTIL 0300Z MN WI SEV ICE BTN 120 AND FL220. CONDS CONTG BYD 0300Z." }),
        f(sq(-40, 50, 15, 8), { kind: "SIGMET", group: "TURB", hazard: "TURB", qualifier: "SEV", name: "SHANWICK OCEANIC A3", base: "FL300", top: "FL380", raw: "EGGX SIGMET A3 VALID 040100/040500 EGRR- SHANWICK OCEANIC SEV TURB FCST FL300/380 MOV E 20KT NC=" }),
      ],
    } as T;
  }
  if (cmd === "online_network" || cmd === "online_pilot") {
    const net = _args?.net === "ivao" ? "ivao" : "vatsim";
    const t = now - 40 * 60_000;
    const c = (callsign: string, kind: string, frequency: string, text: string[] = [], code: string | null = null) => ({
      callsign, id: "1234567", name: net === "vatsim" ? "Justin Controller" : null, frequency, kind, rating: "C1", logon_ms: t, text, atis_code: code,
    });
    const box = (x: number, y: number, w: number, h: number) => [[[[x, y], [x + w, y], [x + w, y + h], [x, y + h], [x, y]]]];
    const pilots = Array.from({ length: 60 }, (_, i) => ({
      callsign: `${net === "vatsim" ? "DAL" : "AFR"}${100 + i}`, id: String(800000 + i), name: net === "vatsim" ? `Pilot ${i}` : null,
      lat: 30 + ((i * 7) % 20) + (net === "ivao" ? 1.5 : 0), lon: -120 + ((i * 13) % 45), alt_ft: 1000 * (i % 38), gs_kt: 120 + (i % 30) * 10, hdg: (i * 37) % 360,
      aircraft: ["B738", "A320", "C172", "B77W"][i % 4], dep: "KJFK", arr: "KLAX", logon_ms: t,
      route: "DEEZZ5 CANDR J60 PSB HAYNZ6", flight_rules: "I", cruise: "35000",
    }));
    if (cmd === "online_pilot") {
      const p = pilots.find((p) => p.callsign === _args?.callsign);
      return (p ? { ...p, dep_pos: [-73.78, 40.64] } : null) as T;
    }
    return {
      net, updated_ms: now - 20_000, pilots,
      areas: net === "vatsim"
        ? [{ id: "KZDC", name: "Washington", oceanic: false, label: [-77.5, 37.6], polys: box(-81, 35.5, 7, 4.5), controllers: [c("DC_32_CTR", "CTR", "133.725", ["ZDC does NOT provide terminal service for PHL", "Feedback? vzdc.org/feedback/new"])] }]
        : [{ id: "KZFW", name: "Fort Worth", oceanic: false, label: [-97.5, 32.5], polys: box(-101, 30.5, 7, 4), controllers: [c("KZFW_CTR", "CTR", "127.000")] }],
      airports: net === "vatsim"
        ? [{ icao: "KLAX", name: "Los Angeles Intl", lat: 33.94, lon: -118.41, controllers: [c("LAX_APP", "APP", "124.500"), c("LAX_TWR", "TWR", "133.900"), c("KLAX_ATIS", "ATIS", "133.800", ["LOS ANGELES INFO K 0253Z 25008KT 10SM FEW020"], "K")] }]
        : [{ icao: "KDFW", name: "Dallas-Fort Worth Intl", lat: 32.9, lon: -97.04, controllers: [c("KDFW_TWR", "TWR", "126.550"), c("KDFW_GND", "GND", "121.650")] }],
      other: [], error: null,
    } as T;
  }
  if (cmd === "map_airports") {
    const d = Number(_args?.detail ?? 1);
    return AIRPORTS.filter((_, i) => d >= 3 || i % (d === 2 ? 2 : 4) === 0).map((a, i) => [a.icao, a.name, Math.min(2, i % 3), a.lon, a.lat]) as T;
  }
  if (cmd === "set_landing_runway") {
    if (status.plan) status.plan = { ...status.plan, destination: { ...status.plan.destination, runway: String(_args?.runway ?? "") } };
    pushStatus();
    return undefined as T;
  }
  if (cmd === "runway_ends") {
    // Portland: 10R/28L and 10L/28R.
    if (_args?.ident !== "KPDX") return null as T;
    return {
      elevation_ft: 31,
      ends: [
        { ident: "10R", lat: 45.5889, lon: -122.6094, heading_deg: 100.5 },
        { ident: "28L", lat: 45.5823, lon: -122.5671, heading_deg: 280.5 },
        { ident: "10L", lat: 45.5942, lon: -122.6012, heading_deg: 100.5 },
        { ident: "28R", lat: 45.5891, lon: -122.5693, heading_deg: 280.5 },
      ],
    } as T;
  }
  if (cmd === "airport_card") {
    const a = AIRPORTS.find((x) => x.icao === _args?.ident) ?? AIRPORTS[0];
    return {
      ident: a.icao, name: a.name, kind: "Major", lat: a.lat, lon: a.lon, elevation_ft: 433, city: "Seattle", iata: "SEA", country: "US",
      country_name: "United States", region_name: "Washington",
      runways: [
        { name: "16L/34R", length_ft: 11901, width_ft: 150, surface: "Concrete", lighted: true },
        { name: "16C/34C", length_ft: 9426, width_ft: 150, surface: "Concrete", lighted: true },
        { name: "16R/34L", length_ft: 8500, width_ft: 150, surface: "Concrete", lighted: true },
      ],
      frequencies: [
        { kind: "ATIS", description: "ATIS", mhz: 118.0 }, { kind: "CLD", description: "Clearance Delivery", mhz: 128.0 },
        { kind: "GND", description: "Ground", mhz: 121.7 }, { kind: "TWR", description: "Tower", mhz: 119.9 },
        { kind: "APP", description: "Seattle Approach", mhz: 119.2 },
      ],
      departures: 3, arrivals: 2, last_visit_ms: now - 864e5 * 3, best_landing_fpm: -88,
      recent: [{ id: 1, dep_icao: a.icao, arr_icao: "KPDX", start_ms: now - 864e5 * 3, icao_type: "A20N", landing_rate_fpm: -142 }],
    } as T;
  }
  if (cmd === "airport_weather") {
    if (scene === "nowx") return null as T;
    return {
      station: String(_args?.ident ?? "KSEA"), station_name: "Seattle-Tacoma Intl, WA, US", distance_nm: 0,
      metar: "KSEA 032153Z 17012G20KT 10SM FEW020 BKN045 09/06 A3001 RMK AO2 SLP165",
      observed_ms: now - 22 * 60_000, category: "VFR", wind_dir: 170, wind_kt: 12, gust_kt: 20, visibility_sm: 10, ceiling_ft: 4500,
      temp_c: 9, dewpoint_c: 6, altimeter_hpa: 1016.3,
      taf: "TAF KSEA 032120Z 0321/0500 17010KT P6SM BKN045\n  FM040300 18012G22KT 5SM -RA OVC025",
    } as T;
  }
  if (cmd === "search_airports") {
    const q = String(_args?.query ?? "").trim().toUpperCase();
    return (q ? AIRPORTS.filter((a) => a.icao.startsWith(q) || (q.length >= 3 && a.name.toUpperCase().includes(q))) : []) as T;
  }
  const answers: Record<string, unknown> = {
    get_status: status,
    list_flights: flights,
    get_totals: totals,
    get_flight: {
      flight: flights[0],
      track: track.map(([lon, lat, alt_ft, gs_kt, t_ms]) => ({ t_ms, lat, lon, alt_ft, gs_kt, vs_fpm: 0, hdg_deg: 180, on_ground: false })),
      landings: [],
      plan: null,
    },
    get_routes: [
      { flight_id: 3, dep_icao: "KPDX", arr_icao: "KSFO", from: [-122.5975, 45.5887], to: [-122.375, 37.6188], date_ms: now, aircraft_type: "A20N" },
      { flight_id: 2, dep_icao: "EGLL", arr_icao: "LFPG", from: [-0.4619, 51.4706], to: [2.5479, 49.0097], date_ms: now, aircraft_type: "A320" },
    ],
    app_version: "0.1.0",
    check_update: { current: "0.1.0", latest: "0.1.0", available: false, notes: "", download_url: null, size_mb: 0 },
    simbrief_latest: { ...samplePlan, ofp: sampleOfp },
    current_briefing: sampleOfp,
    get_passport: { pilot: { name: "Alex Pilot", pilot_id: "FD-0001", home_icao: "KSEA", nationality: "United States", photo_url: null, signature_svg: null }, stamps: [], countries: [], first_flight_ms: now - 90 * 864e5, home_name: "Seattle", totals },
    get_settings: { simbrief_id: "123456" },
    get_logbook: scene === "book" ? Array.from({ length: 40 }, (_, i) => ({ flight_id: 100 + i, date_ms: now - (40 - i) * 864e5, sim: "msfs2024", status: "complete", aircraft_type: "A20N", ident: "D-AINA", category_class: "MEL", engine_count: 2, from_icao: "EDDF", to_icao: "LOWW", out_ms: now, in_ms: now + 4.5e6, total_min: 75, night_min: 0, ifr_min: 75, xc_min: 75, pic_min: 75, dual_min: 0, sim_inst_min: 0, act_inst_min: 0, day_landings: 1, night_landings: 0, approaches: 1, remarks: `Line ${i + 1}` })) : [
      { flight_id: 1, date_ms: now - 864e5, sim: "msfs2024", status: "complete", aircraft_type: "A20N", ident: "D-AINA", category_class: "MEL", engine_count: 2, from_icao: "EDDF", to_icao: "LOWW", out_ms: now - 864e5, in_ms: now - 864e5 + 4.5e6, total_min: 75, night_min: 0, ifr_min: 75, xc_min: 75, pic_min: 75, dual_min: 0, sim_inst_min: 0, act_inst_min: 0, day_landings: 1, night_landings: 0, approaches: 1, remarks: "-140 fpm · MSFS 2024" },
      { flight_id: 2, date_ms: now, sim: "msfs2020", status: "complete", aircraft_type: "C172", ident: "N172SP", category_class: "SEL", engine_count: 1, from_icao: "KPAO", to_icao: "KSQL", out_ms: now, in_ms: now + 1.8e6, total_min: 30, night_min: 0, ifr_min: 0, xc_min: 0, pic_min: 30, dual_min: 0, sim_inst_min: 0, act_inst_min: 0, day_landings: 2, night_landings: 0, approaches: 0, remarks: "Pattern work" },
    ],
    get_hangar: [
      { id: 1, title: "Airbus A320neo", icao_type: "A20N", registration: "D-AINA", engine_type: "jet", engine_count: 2, category_class: "MEL", has_photo: false, flights: 12, minutes: 1490, distance_nm: 8400, landings: 12, first_flight_ms: now - 9e9, last_flight_ms: now, best_landing_fpm: -88, avg_landing_fpm: -160, favorite: true, last_callsign: "DLH4AB", last_icao: "EDDF", last_name: "Frankfurt am Main", last_country: "DE", last_country_name: "Germany", last_region_name: null, last_lat: 50.0333, last_lon: 8.5706, last_sim: "MSFS 2024", model: null, airline: "DLH", sim_registration: "D-AINA", manual: false },
      { id: 2, title: "Cessna 172 Skyhawk", icao_type: "C172", registration: "N172SP", engine_type: "piston", engine_count: 1, category_class: "SEL", has_photo: false, flights: 4, minutes: 300, distance_nm: 400, landings: 6, first_flight_ms: now - 9e9, last_flight_ms: now, best_landing_fpm: -60, avg_landing_fpm: -120, favorite: false, last_callsign: "", last_icao: "KPAO", last_name: "Palo Alto", last_country: "US", last_country_name: "United States", last_region_name: null, last_lat: 37.461, last_lon: -122.115, last_sim: "MSFS 2020", model: null, airline: null, sim_registration: "N172SP", manual: false },
    ],
    // A stand-in picture (the real one comes from Planespotters.net).
    spotter_photo: {
      src: "data:image/svg+xml," + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 260"><defs><linearGradient id="g" x2="0" y2="1"><stop offset="0" stop-color="#8fb4d8"/><stop offset="1" stop-color="#d7e3ec"/></linearGradient></defs><rect width="400" height="260" fill="url(#g)"/><rect y="200" width="400" height="60" fill="#6d7570"/></svg>'),
      width: 400, height: 260, link: "https://www.planespotters.net/", photographer: "Sample Photographer",
    },
    demo_routes: ["KSEA to KPDX", "EGLL to LFPG", "KSFO to KLAX", "EDDF to LOWW", "CYVR to KSEA"],
  };
  return (answers[cmd] ?? null) as T;
}

const listeners: Record<string, ((p: unknown) => void)[]> = {};
export function mockListen(event: string, cb: (p: unknown) => void) {
  (listeners[event] ??= []).push(cb);
  return () => {
    listeners[event] = (listeners[event] ?? []).filter((c) => c !== cb);
  };
}
/** Tell the interface the status changed, as the app does. */
function pushStatus() {
  for (const cb of listeners.status ?? []) cb({ ...status, track_tail: [] });
}

// Tests: a landing (Smart taxi) and the battery switch, and the aircraft
// put somewhere. Mock only.
if (typeof window !== "undefined") {
  const w = window as unknown as Record<string, unknown>;
  w.__fdLand = () => {
    status.last_landing = { t_ms: Date.now(), vs_fpm: -120, vs_source: "sim", g_peak: 1.2, gs_kt: 60, headwind_kt: 4, crosswind_kt: 2, bounces: 0, touch_and_go: false };
    if (status.telemetry) status.telemetry = { ...status.telemetry, on_ground: true, powered: true, gs_kt: 70 };
    pushStatus();
  };
  w.__fdGs = (kt: number) => {
    if (status.telemetry) status.telemetry = { ...status.telemetry, gs_kt: kt };
    pushStatus();
  };
  w.__fdPower = (on: boolean) => {
    if (status.telemetry) status.telemetry = { ...status.telemetry, powered: on };
    pushStatus();
  };
  w.__fdAt = (lat: number, lon: number, alt: number, onGround = false, hdg?: number) => {
    if (status.telemetry) status.telemetry = { ...status.telemetry, lat, lon, alt_msl_ft: alt, on_ground: onGround, ...(hdg != null ? { hdg_true_deg: hdg, hdg_deg: hdg } : {}) };
    pushStatus();
  };
  w.__fdTaxiLayout = mockTaxiLayout;
  w.__fdTaxiHolds = mockHoldLines;
}
