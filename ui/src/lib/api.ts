// Typed bridge to the Rust side. In a plain browser (npm run ui) it falls
// back to a small mock so the interface can be worked on without Tauri.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Phase =
  | "preflight"
  | "taxi_out"
  | "takeoff_roll"
  | "airborne"
  | "landing_rollout"
  | "taxi_in"
  | "arrived";

export interface AircraftInfo {
  title: string;
  icao_type: string;
  registration: string;
  engine_count: number;
  engine_type: string;
}

export interface Place {
  icao: string;
  name: string;
  country: string;
  region: string;
}

export interface Telemetry {
  t_ms: number;
  lat: number;
  lon: number;
  alt_msl_ft: number;
  alt_agl_ft: number;
  gs_kt: number;
  ias_kt: number;
  hdg_true_deg: number;
  vs_fpm: number;
  on_ground: boolean;
  engines_running: number;
  fuel_kg: number;
  paused: boolean;
  sim_rate: number;
}

export interface LandingReport {
  t_ms: number;
  vs_fpm: number;
  vs_source: string;
  g_peak: number;
  gs_kt: number;
  headwind_kt: number;
  crosswind_kt: number;
  bounces: number;
  touch_and_go: boolean;
}

export interface LiveFlight {
  phase: Phase;
  start_ms: number;
  block_out_ms: number | null;
  takeoff_ms: number | null;
  distance_nm: number;
  max_alt_ft: number;
  fuel_used_kg: number;
  dep_lat: number;
  dep_lon: number;
  last_landing: LandingReport | null;
  track_points: number;
}

export interface Status {
  link: "waiting" | "connecting" | "connected";
  sim: string | null;
  sim_label: string | null;
  sim_version: string | null;
  notice: string | null;
  aircraft: AircraftInfo | null;
  phase: Phase | null;
  phase_label: string | null;
  live: LiveFlight | null;
  dep: Place | null;
  telemetry: Telemetry | null;
  last_landing: LandingReport | null;
  demo: boolean;
  session: number;
  track_tail: [number, number][];
}

export interface FullStatus extends Status {
  track: [number, number][];
}

export interface FlightRow {
  id: number;
  sim: string;
  status: string;
  aircraft_title: string;
  icao_type: string;
  registration: string;
  callsign: string;
  dep_icao: string | null;
  dep_name: string | null;
  dep_country: string | null;
  arr_icao: string | null;
  arr_name: string | null;
  arr_country: string | null;
  start_ms: number;
  end_ms: number;
  block_out: number | null;
  takeoff: number | null;
  landing: number | null;
  block_in: number | null;
  total_min: number;
  distance_nm: number;
  max_alt_ft: number;
  fuel_used_kg: number;
  landing_rate_fpm: number | null;
  landing_g: number | null;
  landing_grade: string | null;
  landings: number;
}

export interface TrackPoint {
  t_ms: number;
  lat: number;
  lon: number;
  alt_ft: number;
  gs_kt: number;
  vs_fpm: number;
  hdg_deg: number;
  on_ground: boolean;
}

export interface FlightDetail {
  flight: FlightRow;
  track: TrackPoint[];
  landings: LandingReport[];
}

export interface Totals {
  flights: number;
  minutes: number;
  distance_nm: number;
  landings: number;
  airports: number;
  countries: number;
}

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return invoke<T>(cmd, args);
  const { mockCall } = await import("./mock");
  return mockCall<T>(cmd, args);
}

export const api = {
  status: () => call<FullStatus>("get_status"),
  flights: (limit = 200) => call<FlightRow[]>("list_flights", { limit }),
  totals: () => call<Totals>("get_totals"),
  flight: (id: number) => call<FlightDetail | null>("get_flight", { id }),
  deleteFlight: (id: number) => call<boolean>("delete_flight", { id }),
  startDemo: (route: number | null, speedup = 30) => call<void>("start_demo", { route, speedup }),
  stopDemo: () => call<void>("stop_demo"),
  endFlight: () => call<void>("end_flight"),
  demoRoutes: () => call<string[]>("demo_routes"),
};

export async function on<T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  if (inTauri) return listen<T>(event, (e) => cb(e.payload));
  const { mockListen } = await import("./mock");
  return mockListen(event, cb as (p: unknown) => void);
}
