// Browser-only stand-in for the Rust backend: one flight in progress and a
// short history, so the UI can be developed with `npm run ui`.

import type { FlightRow, FullStatus, Totals } from "./api";

const now = Date.now();
const track: [number, number][] = Array.from({ length: 40 }, (_, i) => [
  -122.3093 - i * 0.004,
  47.449 - i * 0.03,
]);

const status: FullStatus = {
  link: "connected",
  sim: "msfs2024",
  sim_label: "MSFS 2024",
  sim_version: "12.1",
  notice: null,
  aircraft: { title: "Airbus A320neo", icao_type: "A20N", registration: "N320FD", engine_count: 2, engine_type: "jet" },
  phase: "airborne",
  phase_label: "Airborne",
  live: {
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
  track,
};

function row(id: number, days: number, dep: string, arr: string, type: string, min: number, fpm: number, grade: string): FlightRow {
  const start = now - days * 86_400_000;
  return {
    id, sim: "msfs2024", status: "complete", aircraft_title: type, icao_type: type, registration: "N320FD",
    callsign: "", dep_icao: dep, dep_name: dep, dep_country: "US", arr_icao: arr, arr_name: arr, arr_country: "US",
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

export async function mockCall<T>(cmd: string, _args?: Record<string, unknown>): Promise<T> {
  const answers: Record<string, unknown> = {
    get_status: status,
    list_flights: flights,
    get_totals: totals,
    get_flight: {
      flight: flights[0],
      track: track.map(([lon, lat], i) => ({ t_ms: i, lat, lon, alt_ft: 1000, gs_kt: 300, vs_fpm: 0, hdg_deg: 180, on_ground: false })),
      landings: [],
    },
    get_routes: [
      { flight_id: 3, dep_icao: "KPDX", arr_icao: "KSFO", from: [-122.5975, 45.5887], to: [-122.375, 37.6188], date_ms: now, aircraft_type: "A20N" },
      { flight_id: 2, dep_icao: "EGLL", arr_icao: "LFPG", from: [-0.4619, 51.4706], to: [2.5479, 49.0097], date_ms: now, aircraft_type: "A320" },
    ],
    demo_routes: ["KSEA to KPDX", "EGLL to LFPG", "KSFO to KLAX", "EDDF to LOWW", "CYVR to KSEA"],
  };
  return (answers[cmd] ?? null) as T;
}

export function mockListen(_event: string, _cb: (p: unknown) => void) {
  return () => {};
}
