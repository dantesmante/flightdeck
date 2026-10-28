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

export interface PlanAirport {
  icao: string;
  name: string;
  lat: number;
  lon: number;
  runway: string;
}

export interface PlanFix {
  ident: string;
  kind: string;
  lat: number;
  lon: number;
  alt_ft: number;
  stage: string;
  procedure: boolean;
}

export interface Notam {
  id: string;
  location: string;
  text: string;
}

export interface OfpLogRow {
  ident: string;
  airway: string;
  stage: string;
  alt_ft: number;
  track_mag: number;
  dist_leg: number;
  time_total: number;
  fuel_onboard: number;
  wind_dir: number;
  wind_kt: number;
  oat: number;
  tas: number;
  gs: number;
  mach: string;
  /** [altitude ft, direction, speed kt, OAT] */
  winds: [number, number, number, number][];
}

/** The briefing: SimBrief's OFP and its sections. */
export interface Ofp {
  html: string;
  units: string;
  fpl: string;
  times: Record<string, string | number>;
  weights: Record<string, string | number>;
  fuel: Record<string, string | number>;
  weather: [string, string][];
  notams: Notam[];
  log: OfpLogRow[];
}

/** A flight plan, e.g. imported from SimBrief. */
export interface FlightPlan {
  source: string;
  id: string;
  generated_ms: number;
  callsign: string;
  airline: string;
  flight_number: string;
  origin: PlanAirport;
  destination: PlanAirport;
  alternate: string;
  route: string;
  cruise_ft: number;
  distance_nm: number;
  ete_min: number;
  aircraft_icao: string;
  aircraft_name: string;
  registration: string;
  block_fuel_kg: number;
  sched_out_ms: number | null;
  sched_in_ms: number | null;
  fixes: PlanFix[];
  /** Only in saved flights; the live plan's briefing comes from currentBriefing(). */
  ofp?: Ofp | null;
}

export interface Status {
  link: "waiting" | "connecting" | "connected";
  /** Connected, waiting for the pilot to power up or press Start a flight. */
  waiting_for_pilot: boolean;
  /** Start a flight was pressed; tracking begins once in the cockpit. */
  armed: boolean;
  plan: FlightPlan | null;
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
  /** [lon, lat, altitude ft, ground speed kt, time ms] */
  track_tail: [number, number, number, number, number][];
}

export interface FullStatus extends Status {
  track: [number, number, number, number, number][];
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
  plan: FlightPlan | null;
}

export interface Totals {
  flights: number;
  minutes: number;
  distance_nm: number;
  landings: number;
  airports: number;
  countries: number;
}

export interface Pilot {
  name: string;
  pilot_id: string;
  home_icao: string;
  nationality: string;
  photo_url: string | null;
  signature_svg: string | null;
}

export interface LogRow {
  flight_id: number;
  date_ms: number;
  sim: string;
  status: string;
  aircraft_type: string;
  ident: string;
  category_class: string;
  engine_count: number;
  from_icao: string;
  to_icao: string;
  out_ms: number | null;
  in_ms: number | null;
  total_min: number;
  night_min: number;
  ifr_min: number;
  xc_min: number;
  pic_min: number;
  dual_min: number;
  sim_inst_min: number;
  act_inst_min: number;
  day_landings: number;
  night_landings: number;
  approaches: number;
  remarks: string;
}

export interface FlightEdit {
  date_ms?: number;
  out_min?: number;
  in_min?: number;
  from_icao?: string;
  to_icao?: string;
  aircraft_type?: string;
  ident?: string;
}

export type LogPatch = Partial<Omit<LogRow, "flight_id" | "date_ms" | "sim" | "status" | "aircraft_type" | "ident" | "category_class" | "engine_count" | "from_icao" | "to_icao" | "out_ms" | "in_ms">>;

export type StampKind = "arrival" | "first_entry";

export interface Route {
  flight_id: number;
  dep_icao: string | null;
  arr_icao: string | null;
  /** [lon, lat] */
  from: [number, number];
  to: [number, number];
  date_ms: number;
  aircraft_type: string;
}

export interface Stamp {
  flight_id: number;
  kind: StampKind;
  country: string;
  region: string;
  icao: string;
  airport_name: string;
  t_ms: number;
  aircraft_type: string;
  seed: number;
  country_name: string;
  region_name: string;
}

export interface CountryVisit {
  country: string;
  name: string;
  first_ms: number;
  arrivals: number;
}

export interface PassportData {
  pilot: Pilot;
  stamps: Stamp[];
  countries: CountryVisit[];
  first_flight_ms: number | null;
  home_name: string | null;
  totals: Totals;
}

export interface HangarItem {
  id: number;
  title: string;
  icao_type: string;
  registration: string;
  engine_type: string;
  engine_count: number;
  category_class: string;
  has_photo: boolean;
  flights: number;
  minutes: number;
  distance_nm: number;
  landings: number;
  first_flight_ms: number;
  last_flight_ms: number;
  best_landing_fpm: number | null;
  last_icao: string | null;
  last_name: string | null;
  last_country: string | null;
  last_country_name: string | null;
  last_region_name: string | null;
  last_lat: number | null;
  last_lon: number | null;
  last_sim: string;
}

export type Settings = Record<string, string>;

export interface UpdateInfo {
  current: string;
  latest: string;
  available: boolean;
  notes: string;
  download_url: string | null;
  size_mb: number;
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
  simbriefLatest: () => call<FlightPlan>("simbrief_latest"),
  startFlight: (plan: FlightPlan | null) => call<void>("start_flight", { plan }),
  setPlan: (plan: FlightPlan | null) => call<void>("set_plan", { plan }),
  currentBriefing: () => call<Ofp | null>("current_briefing"),
  importVolanta: (files: { name: string; text: string }[]) =>
    call<{ imported: string[]; duplicates: string[]; failed: [string, string][] }>("import_volanta", { files }),
  exportFlightVolanta: (id: number) => call<string>("export_flight_volanta", { id }),
  diagnostics: () => call<string>("get_diagnostics"),
  saveDiagnostics: () => call<string>("save_diagnostics"),
  exportAllVolanta: () => call<{ folder: string; count: number }>("export_all_volanta"),
  demoRoutes: () => call<string[]>("demo_routes"),
  settings: () => call<Settings>("get_settings"),
  setSetting: (key: string, value: string) => call<void>("set_setting", { key, value }),
  pilot: () => call<Pilot>("get_pilot"),
  setPilot: (pilot: Pilot) => call<void>("set_pilot", { pilot }),
  logbook: () => call<LogRow[]>("get_logbook"),
  updateLogEntry: (flightId: number, patch: LogPatch) => call<boolean>("update_log_entry", { flightId, patch }),
  updateFlight: (flightId: number, edit: FlightEdit) => call<boolean>("update_flight", { flightId, edit }),
  passport: () => call<PassportData>("get_passport"),
  hangar: () => call<HangarItem[]>("get_hangar"),
  routes: () => call<Route[]>("get_routes"),
  aircraftPhoto: (id: number) => call<string | null>("get_aircraft_photo", { id }),
  setAircraftPhoto: (id: number, dataUrl: string | null) => call<boolean>("set_aircraft_photo", { id, dataUrl }),
  checkUpdate: () => call<UpdateInfo>("check_update"),
  installUpdate: (url: string) => call<void>("install_update", { url }),
  version: () => call<string>("app_version"),
  addSampleFlights: () => call<number>("add_sample_flights"),
  removeSampleFlights: () => call<number>("remove_sample_flights"),
};

export async function on<T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  if (inTauri) return listen<T>(event, (e) => cb(e.payload));
  const { mockListen } = await import("./mock");
  return mockListen(event, cb as (p: unknown) => void);
}
