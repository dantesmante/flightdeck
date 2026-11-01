// Browser-only stand-in for the Rust backend: one flight in progress and a
// short history, so the UI can be developed with `npm run ui`.

import { RANKS, type FlightPlan, type FlightRow, type FlightScore, type FullStatus, type PilotProfile, type Totals } from "./api";

const now = Date.now();
const track: [number, number, number, number, number][] = Array.from({ length: 40 }, (_, i) => [
  -122.3093 - i * 0.004,
  47.449 - i * 0.03,
  Math.min(24000, i * 900),
  Math.min(420, 140 + i * 12),
  now - (40 - i) * 30_000,
]);

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
  link: "connected",
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
      unrated, confidence: 0, estimated: [], airborne_min: min, xp_base: 0, xp_bonus: 0, xp: 0, rating_before: null, rating_after: null, expected: null, k: null,
    };
    if (!unrated) {
      const wobble = Math.sin(i * 1.7) * 9 + Math.cos(i * 0.6) * 5;
      const vs = -Math.round(110 + Math.abs(Math.sin(i * 2.3)) * 260);
      const off = Math.abs(Math.sin(i * 3.1)) * 22;
      const bounces = i % 7 === 3 ? 1 : 0;
      const rate = Math.max(0, Math.min(100, 100 - (Math.abs(vs) - 60) / 4)), g = 92 - (i % 4) * 4, cl = Math.max(0, 100 - off * 2.4), bo = bounces ? 60 : 100;
      const landing = imported ? (rate * 0.35 + g * 0.2 + cl * 0.25) / 0.8 : rate * 0.35 + g * 0.2 + cl * 0.25 + bo * 0.2;
      const safety = Math.min(100, 93 + wobble * 0.6);
      const p = landing / 2 + safety / 2;
      const confidence = imported ? 0.75 : 1;
      const e = expected(rating), k = (rated < 5 ? 48 : rated < 20 ? 40 : rated < 50 ? 32 : 24) * confidence;
      const after = rating + (k * (p - e)) / 100;
      Object.assign(s, {
        confidence,
        estimated: imported ? ["Bank and pitch from the positions the flight was logged with", "Not recorded, so left out: bounces, G-force in flight"] : [],
        performance: p, rating_before: rating, rating_after: after, expected: e, k, xp_base: Math.round(p),
        landing: { score: landing, rate: { score: rate, weight: 0.35 }, g: { score: g, weight: 0.2 }, centerline: { score: cl, weight: 0.25 }, bounce: imported ? null : { score: bo, weight: 0.2 }, source: imported ? "volanta" : "sim", vs_fpm: vs, g_peak: 1.08 + (i % 4) * 0.07, bounces, runway: { airport: arr, runway: ["16L", "28R", "25L", "09", "34R"][i % 5], heading_deg: 160, width_ft: 150, offset_ft: (i % 2 ? 1 : -1) * off, along_ft: 1200 }, crosswind_kt: (i * 3) % 14, discarded: i === 12 ? 1 : 0 },
        safety: { score: safety, g: imported ? null : { score: Math.min(100, safety + 2), weight: 0.5 }, bank: { score: Math.min(100, safety - 1), weight: imported ? 0.5 : 0.25 }, pitch: { score: 100, weight: imported ? 0.5 : 0.25 }, aerobatic: false, source: imported ? "positions" : "recorded", g_max: 1.35, g_min: 0.72, bank_max: 31 + (i % 5) * 4, pitch_max: 14, pitch_min: -6 },
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

export async function mockCall<T>(cmd: string, _args?: Record<string, unknown>): Promise<T> {
  if (cmd === "set_banner") mockBanner = (_args?.dataUrl as string | null) ?? null;
  if (cmd === "set_pilot_photo") mockPhoto = (_args?.dataUrl as string | null) ?? null;
  if (cmd === "get_flight") {
    const p = mockProfile();
    const want = scene === "imported" ? p.rating.flights.find((f) => f.confidence > 0 && f.confidence < 1) : scene === "unrated" ? p.rating.flights.find((f) => f.unrated) : p.rating.flights.find((f) => f.performance != null);
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

export function mockListen(_event: string, _cb: (p: unknown) => void) {
  return () => {};
}
