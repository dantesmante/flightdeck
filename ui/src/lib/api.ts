// Typed bridge to the Rust side. In a plain browser (npm run ui) it falls
// back to a small mock so the interface can be worked on without Tauri.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { inRemote, remoteCall, remoteListen } from "./remote.svelte";
export { inRemote };

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

export interface RunwayEnd {
  ident: string;
  lat: number;
  lon: number;
  /** True heading of a landing on this end. */
  heading_deg: number;
}
export interface RunwayEnds {
  elevation_ft: number;
  ends: RunwayEnd[];
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
  /** Electrical power (battery or a live bus); null when the sim doesn't say. */
  powered?: boolean | null;
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

export interface AirportHit {
  icao: string;
  name: string;
  country: string;
  lat: number;
  lon: number;
}

/** A route drawn on the map, for an MSFS .PLN file. */
export interface PlnRoute {
  rules: "VFR" | "IFR";
  cruise_ft: number;
  origin: PlnPoint;
  destination: PlnPoint;
  waypoints: PlnPoint[];
}
export interface PlnPoint {
  ident: string;
  name: string;
  lat: number;
  lon: number;
  airport: boolean;
  /** Altitude to cross it at, ft (else the cruise). */
  alt_ft?: number | null;
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
/** What a backup file holds. */
export interface BackupInfo {
  flights: number;
  aircraft: number;
  last_flight_ms: number | null;
  schema: number;
}

/** A screenshot: where the file is on the PC (never copied) and when it
 * was taken. */
export interface Screenshot {
  id: number;
  path: string;
  taken_ms: number;
}

/** What a PFD draws, about 15 times a second while one is open ("pfd" event). */
export interface Pfd {
  t_ms: number;
  lat: number;
  lon: number;
  alt_msl_ft: number;
  /** On the altimeter (its pressure setting applied where the sim says). */
  alt_ft: number;
  agl_ft: number;
  ias_kt: number;
  tas_kt: number | null;
  gs_kt: number;
  vs_fpm: number;
  pitch_deg: number;
  bank_deg: number;
  /** Magnetic where the sim says, else true. */
  hdg_deg: number;
  hdg_true_deg: number;
  track_true_deg: number | null;
  ball: number | null;
  g_load: number;
  on_ground: boolean;
  paused: boolean;
  baro_inhg: number | null;
  sel_alt_ft: number | null;
  sel_hdg_deg: number | null;
  sel_spd_kt: number | null;
  sel_vs_fpm: number | null;
  ap_on: boolean | null;
}

/** A screenshot in the Photos tab: its flight, and where it was taken. */
export interface Photo extends Screenshot {
  flight_id: number | null;
  dep: string | null;
  arr: string | null;
  lat: number | null;
  lon: number | null;
  alt_ft: number | null;
}

/** An airport to divert to. */
export interface DivertOption {
  ident: string;
  name: string;
  city: string;
  country: string;
  lat: number;
  lon: number;
  distance_nm: number;
  bearing: number;
  longest_ft: number;
  surface: string;
}

export interface FlightPlan {
  /** The destination first planned, when the pilot diverted. */
  diverted_from?: string | null;
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
  /** Hangar aircraft chosen with "Fly this aircraft". */
  forced_aircraft: number | null;
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

/** One of a set of flights that look like the same flight logged twice. */
/** One flight file as it would be imported (nothing saved yet). */
export interface ImportPreview {
  name: string;
  dep: string | null;
  arr: string | null;
  start_ms: number;
  minutes: number;
  icao_type: string;
  registration: string;
  callsign: string;
  /** Why it looks like a flight FlightDeck already has. */
  duplicate: string | null;
  /** Why it cannot be imported. */
  error: string | null;
}

export interface DuplicateFlight extends FlightRow {
  has_track: boolean;
  /** "" (recorded by FlightDeck), "manual", "volanta" or "elevatex". */
  source: string;
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
  score?: FlightScore | null;
  standing?: Standing | null;
}

export interface Standing {
  rating: number;
  xp: number;
  rank: Rank;
  next_rank: Rank | null;
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

export interface ScorePart {
  score: number;
  weight: number;
}

export interface LandingScore {
  score: number;
  rate: ScorePart;
  g: ScorePart | null;
  bounce: ScorePart | null;
  source: string;
  vs_fpm: number;
  g_peak: number;
  bounces: number;
  crosswind_kt: number;
  crosswind_bonus: number;
  base_score: number;
  discarded: number;
}

export interface SafetyScore {
  score: number;
  g: ScorePart | null;
  bank: ScorePart | null;
  pitch: ScorePart | null;
  smoothness: ScorePart | null;
  approach: ScorePart | null;
  speed: ScorePart | null;
  roll_rate_max: number;
  pitch_rate_max: number;
  g_rate_max: number;
  approach_sink_max: number;
  approach_bank_max: number;
  approach_speed_dev: number;
  speed_max_low: number;
  aerobatic: boolean;
  source: string;
  g_max: number;
  g_min: number;
  bank_max: number;
  pitch_max: number;
  pitch_min: number;
}

export interface FlightScore {
  flight_id: number;
  start_ms: number;
  dep_icao: string;
  arr_icao: string;
  icao_type: string;
  performance: number | null;
  landing: LandingScore | null;
  safety: SafetyScore | null;
  unrated: string | null;
  airborne_min: number;
  xp_base: number;
  xp_bonus: number;
  xp: number;
  rating_before: number | null;
  rating_after: number | null;
  expected: number | null;
  k: number | null;
}

export interface Insignia {
  stripes: number;
  thin: boolean;
  wings: "bronze" | "silver" | "gold" | null;
  oak_leaf: boolean;
}

export interface Rank {
  level: number;
  title: string;
  min_xp: number;
  insignia: Insignia;
}

export interface RatingProfile {
  rating: number;
  rating_peak: number;
  expected: number;
  rated_flights: number;
  xp: number;
  rank: Rank;
  next_rank: Rank | null;
  avg_performance: number | null;
  avg_landing: number | null;
  avg_safety: number | null;
  history: { t_ms: number; rating: number }[];
  flights: FlightScore[];
}

export interface StatFlight {
  id: number;
  dep_icao: string | null;
  arr_icao: string | null;
  start_ms: number;
  value: number;
}

export interface PilotStats {
  first_flight_ms: number | null;
  longest: StatFlight | null;
  farthest: StatFlight | null;
  softest: StatFlight | null;
  avg_landing_fpm: number | null;
  night_minutes: number;
  aircraft_types: number;
  top_types: { name: string; count: number; minutes: number }[];
  top_airports: { name: string; count: number; minutes: number }[];
  months: { month: string; minutes: number; flights: number }[];
  /** Every month with flights, oldest first. */
  history?: { month: string; minutes: number; flights: number }[];
}

export interface PilotProfile {
  pilot: Pilot;
  banner: string | null;
  totals: Totals;
  stats: PilotStats;
  rating: RatingProfile;
}

/** Every rank, lowest first (the same table as the backend's). */
export const RANKS: Rank[] = [
  { level: 1, title: "Student Pilot", min_xp: 0, insignia: { stripes: 0, thin: false, wings: null, oak_leaf: false } },
  { level: 2, title: "Solo Pilot", min_xp: 500, insignia: { stripes: 1, thin: true, wings: null, oak_leaf: false } },
  { level: 3, title: "Private Pilot", min_xp: 1500, insignia: { stripes: 1, thin: false, wings: "bronze", oak_leaf: false } },
  { level: 4, title: "Line Pilot", min_xp: 3500, insignia: { stripes: 2, thin: true, wings: null, oak_leaf: false } },
  { level: 5, title: "Cadet Pilot", min_xp: 5000, insignia: { stripes: 2, thin: true, wings: "silver", oak_leaf: false } },
  { level: 6, title: "Second Officer", min_xp: 7500, insignia: { stripes: 2, thin: false, wings: null, oak_leaf: false } },
  { level: 7, title: "First Officer", min_xp: 10000, insignia: { stripes: 3, thin: false, wings: null, oak_leaf: false } },
  { level: 8, title: "Senior First Officer", min_xp: 12000, insignia: { stripes: 3, thin: false, wings: "gold", oak_leaf: false } },
  { level: 9, title: "Captain", min_xp: 15000, insignia: { stripes: 4, thin: false, wings: null, oak_leaf: false } },
  { level: 10, title: "Fleet Captain", min_xp: 30000, insignia: { stripes: 4, thin: false, wings: null, oak_leaf: true } },
];

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

export type LogPatch = Partial<Omit<LogRow, "flight_id" | "date_ms" | "sim" | "status" | "aircraft_type" | "ident" | "category_class" | "engine_count" | "from_icao" | "to_icao" | "out_ms" | "in_ms">> & {
  /** SEL, MEL, SES or MES for this line; "" = the aircraft's own class. */
  class_override?: string;
};

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
  avg_landing_fpm: number | null;
  favorite: boolean;
  last_callsign: string;
  last_icao: string | null;
  last_name: string | null;
  last_country: string | null;
  last_country_name: string | null;
  last_region_name: string | null;
  last_lat: number | null;
  last_lon: number | null;
  last_sim: string;
  /** The pilot's own model name, if set. */
  model: string | null;
  /** Airline code or name, if set. */
  airline: string | null;
  /** The registration the sim reports (the matching key). */
  sim_registration: string;
  /** Added by hand in the hangar. */
  manual: boolean;
}

export interface AircraftEdit {
  registration?: string;
  model?: string;
  airline?: string;
}

export interface NewAircraft {
  registration: string;
  model: string;
  icao_type: string;
  engine_type: string;
  engine_count: number;
  airline: string | null;
}

export interface ManualEntry {
  date_ms: number;
  out_min: number | null;
  in_min: number | null;
  from_icao: string;
  to_icao: string;
  aircraft_id: number | null;
  aircraft_type: string;
  ident: string;
  callsign: string;
  total_min: number;
  night_min: number;
  ifr_min: number;
  day_landings: number;
  night_landings: number;
  remarks: string;
}

export type Settings = Record<string, string>;

/** A Planespotters.net photo: show it from `src`, credit the photographer
 * and link to `link`. */
export interface SpotterPhoto {
  src: string;
  width: number;
  height: number;
  link: string;
  photographer: string;
}

/** Access from other devices on the home network (Settings on the PC). */
export interface RemoteInfo {
  enabled: boolean;
  running: boolean;
  can_edit: boolean;
  port: number;
  urls: string[];
  qr_svg: string | null;
  code: string;
  devices: { name: string; paired_ms: number }[];
  error: string | null;
}

/** One airport dot: [ident, name, rank (0 major, 1 regional, 2 small), lon, lat]. */
/** [ident, name, rank (0 major, 1 regional, 2 small), lon, lat, chart symbol, towered, runway pattern]. */
export type MapAirport = [string, string, number, number, number, number?, boolean?, string?];

export interface RunwayInfo {
  name: string;
  length_ft: number;
  width_ft: number;
  surface: string;
  lighted: boolean;
}

export interface AirportCardData {
  ident: string;
  name: string;
  kind: string;
  lat: number;
  lon: number;
  elevation_ft: number;
  city: string;
  iata: string;
  country: string;
  country_name: string;
  region_name: string;
  runways: RunwayInfo[];
  frequencies: { kind: string; description: string; mhz: number }[];
  departures: number;
  arrivals: number;
  last_visit_ms: number | null;
  best_landing_fpm: number | null;
  recent: { id: number; dep_icao: string | null; arr_icao: string | null; start_ms: number; icao_type: string; landing_rate_fpm: number | null }[];
}

export interface AirportWeather {
  station: string;
  station_name: string;
  distance_nm: number;
  metar: string;
  observed_ms: number | null;
  category: "VFR" | "MVFR" | "IFR" | "LIFR" | null;
  wind_dir: number | null;
  wind_kt: number | null;
  gust_kt: number | null;
  visibility_sm: number | null;
  ceiling_ft: number | null;
  temp_c: number | null;
  dewpoint_c: number | null;
  altimeter_hpa: number | null;
  taf: string | null;
}

/** Online networks (VATSIM, IVAO): who is flying and controlling. */
export type Net = "vatsim" | "ivao";
export interface NetPilot {
  callsign: string;
  id: string;
  name: string | null;
  lat: number;
  lon: number;
  alt_ft: number;
  gs_kt: number;
  hdg: number;
  aircraft: string;
  dep: string;
  arr: string;
  route?: string;
  flight_rules?: string;
  cruise?: string;
  logon_ms: number;
  /** [lon, lat] of the departure airport (only for one pilot). */
  dep_pos?: [number, number];
}
export interface NetController {
  callsign: string;
  id: string;
  name: string | null;
  frequency: string;
  kind: "DEL" | "GND" | "TWR" | "APP" | "DEP" | "CTR" | "FSS" | "ATIS" | "RMP";
  rating: string | null;
  logon_ms: number;
  text: string[];
  atis_code: string | null;
}
export interface NetArea {
  id: string;
  name: string;
  oceanic: boolean;
  label: [number, number];
  polys: [number, number][][][];
  controllers: NetController[];
}
export interface NetAirport {
  icao: string;
  name: string;
  lat: number;
  lon: number;
  controllers: NetController[];
}
export interface NetworkData {
  net: Net;
  updated_ms: number;
  pilots: NetPilot[];
  areas: NetArea[];
  airports: NetAirport[];
  other: NetController[];
  error: string | null;
}

/** Flight categories of METAR stations: [code, lon, lat, 0 VFR | 1 MVFR | 2 IFR | 3 LIFR]. */
export interface WxCategories {
  updated_ms: number;
  stations: [string, number, number, number][];
  /** Surface wind: [code, lon, lat, direction from (-1 variable), knots, gust knots (0 none)]. */
  winds: [string, number, number, number, number, number][];
  error: string | null;
}
/** Forecast wind at one level on a one-degree grid (GFS). */
export interface WindGrid {
  level: string;
  run_ms: number;
  valid_ms: number;
  /** 360 columns from 0°E, 181 rows from 90°N. */
  nx: number;
  ny: number;
  /** East and north components in knots, little-endian Int16, base64. */
  u: string;
  v: string;
  error: string | null;
}
export interface TempGrid {
  run_ms: number;
  valid_ms: number;
  nx: number;
  ny: number;
  /** Tenths of °C, little-endian Int16, base64; rows from 90°N, columns from 0°E. */
  t: string;
  error: string | null;
}
export interface IcingGrid {
  run_ms: number;
  valid_ms: number;
  fl: number;
  /** GFS file used: "1p00" or "0p50". */
  product: string;
  nx: number;
  ny: number;
  /** One byte per degree from 90°N / 0°E: 0–100, 255 unknown. Base64. */
  potential: string;
  /** Share of the world with a score. */
  known: number;
}
/** A VOR, NDB or waypoint from the sim's navigation database. */
export interface NavAid {
  kind: "vor" | "ndb" | "waypoint";
  ident: string;
  region: string;
  lat: number;
  lon: number;
  freq_hz: number | null;
  dme: boolean;
  dme_only: boolean;
}
export interface NavView {
  items: NavAid[];
  more: boolean;
  /** Known in all, and when the whole world was last read from the sim (0 never). */
  known: number;
  /** Of those, waypoints. */
  waypoints?: number;
  world_ms: number;
}
export interface WxAdvisoryProps {
  kind: "SIGMET" | "AIRMET" | "CONVECTIVE";
  group: "TS" | "TURB" | "ICE" | "IFR" | "VA" | "TC" | "OTHER";
  hazard: string;
  qualifier: string;
  name: string;
  from_ms: number | null;
  to_ms: number | null;
  base: string;
  top: string;
  raw: string;
  due_to: string;
}
export interface WxAdvisories {
  updated_ms: number;
  features: GeoJSON.Feature<GeoJSON.Polygon | GeoJSON.MultiPolygon, WxAdvisoryProps>[];
  error: string | null;
}

/** FlightDeck's app in the MSFS 2024 EFB (Settings on the PC). */
export interface EfbInfo {
  installed: boolean;
  version: string | null;
  community: string | null;
  manual: boolean;
  error: string | null;
}

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
  if (inRemote) return remoteCall<T>(cmd, args);
  const { mockCall } = await import("./mock");
  return mockCall<T>(cmd, args);
}

export const api = {
  status: () => call<FullStatus>("get_status"),
  flights: (limit = 200) => call<FlightRow[]>("list_flights", { limit }),
  totals: () => call<Totals>("get_totals"),
  flight: (id: number) => call<FlightDetail | null>("get_flight", { id }),
  deleteFlight: (id: number) => call<boolean>("delete_flight", { id }),
  findDuplicates: () => call<DuplicateFlight[][]>("find_duplicates"),
  deleteFlights: (ids: number[]) => call<number>("delete_flights", { ids }),
  startDemo: (route: number | null, speedup = 30) => call<void>("start_demo", { route, speedup }),
  stopDemo: () => call<void>("stop_demo"),
  endFlight: () => call<void>("end_flight"),
  simbriefLatest: () => call<FlightPlan>("simbrief_latest"),
  startFlight: (plan: FlightPlan | null, aircraftId: number | null = null) =>
    call<void>("start_flight", { plan, aircraftId }),
  setPlan: (plan: FlightPlan | null) => call<void>("set_plan", { plan }),
  divertOptions: (lat: number, lon: number, skip: string) => call<DivertOption[]>("divert_options", { lat, lon, skip }),
  divert: (icao: string) => call<void>("divert", { icao }),
  backupSave: () => call<string | null>("backup_save"),
  backupChooseFolder: () => call<string | null>("backup_choose_folder"),
  backupForgetFolder: () => call<void>("backup_forget_folder"),
  backupNow: () => call<string>("backup_now"),
  backupRestorePick: () => call<BackupInfo | null>("backup_restore_pick"),
  backupRestoreCancel: () => call<void>("backup_restore_cancel"),
  restartApp: () => call<void>("restart_app"),
  flightScreenshots: (id: number) => call<Screenshot[]>("flight_screenshots", { id }),
  screenshotsSince: (sinceMs: number) => call<Screenshot[]>("screenshots_since", { sinceMs }),
  /** Every screenshot, newest first, placed on its flight. */
  photos: () => call<Photo[]>("photos"),
  /** Delete a screenshot: to the Recycle Bin (fileToo), or only forgotten by FlightDeck. */
  deleteScreenshot: (id: number, fileToo: boolean) => call<void>("delete_screenshot", { id, fileToo }),
  pickScreenshotFolder: () => call<string | null>("pick_screenshot_folder"),
  setScreenshotFolder: (path: string | null) => call<void>("set_screenshot_folder", { path }),
  currentBriefing: () => call<Ofp | null>("current_briefing"),
  importVolanta: (files: { name: string; text: string }[], force: string[] = []) =>
    call<{ imported: string[]; duplicates: string[]; failed: [string, string][] }>("import_volanta", { files, force }),
  importElevatex: (files: { name: string; text: string }[], force: string[] = []) =>
    call<{ imported: string[]; duplicates: string[]; failed: [string, string][] }>("import_elevatex", { files, force }),
  previewImport: (kind: "volanta" | "elevatex", files: { name: string; text: string }[]) =>
    call<ImportPreview[]>("preview_import", { kind, files }),
  exportCsv: (ids: number[] | null) => call<{ folder: string; count: number }>("export_csv", { ids }),

  diagnostics: () => call<string>("get_diagnostics"),
  saveDiagnostics: () => call<string>("save_diagnostics"),
  exportAllVolanta: (ids: number[] | null) => call<{ folder: string; count: number }>("export_all_volanta", { ids }),
  spotterPhoto: (registration: string) => call<SpotterPhoto | null>("spotter_photo", { registration }),
  cancelFlight: () => call<void>("cancel_flight"),
  demoRoutes: () => call<string[]>("demo_routes"),
  settings: () => call<Settings>("get_settings"),
  setSetting: (key: string, value: string) => call<void>("set_setting", { key, value }),
  pilot: () => call<Pilot>("get_pilot"),
  setPilot: (pilot: Pilot) => call<void>("set_pilot", { pilot }),
  pilotProfile: () => call<PilotProfile>("get_pilot_profile"),
  setBanner: (dataUrl: string | null) => call<void>("set_banner", { dataUrl }),
  setPilotPhoto: (dataUrl: string | null) => call<void>("set_pilot_photo", { dataUrl }),
  logbook: () => call<LogRow[]>("get_logbook"),
  updateLogEntry: (flightId: number, patch: LogPatch) => call<boolean>("update_log_entry", { flightId, patch }),
  updateFlight: (flightId: number, edit: FlightEdit) => call<boolean>("update_flight", { flightId, edit }),
  passport: () => call<PassportData>("get_passport"),
  hangar: () => call<HangarItem[]>("get_hangar"),
  searchAirports: (query: string) => call<AirportHit[]>("search_airports", { query }),
  airportAt: (lat: number, lon: number) => call<AirportHit | null>("airport_at", { lat, lon }),
  routes: () => call<Route[]>("get_routes"),
  aircraftPhoto: (id: number) => call<string | null>("get_aircraft_photo", { id }),
  setAircraftPhoto: (id: number, dataUrl: string | null) => call<boolean>("set_aircraft_photo", { id, dataUrl }),
  setAircraftFavorite: (id: number, favorite: boolean) => call<boolean>("set_aircraft_favorite", { id, favorite }),
  updateAircraft: (id: number, edit: AircraftEdit) => call<boolean>("update_aircraft", { id, edit }),
  addAircraft: (aircraft: NewAircraft) => call<number>("add_aircraft", { aircraft }),
  deleteAircraft: (id: number, moveTo: number | null) => call<number>("delete_aircraft", { id, moveTo }),
  aircraftWithRegistration: (registration: string) => call<number | null>("aircraft_with_registration", { registration }),
  addManualEntry: (entry: ManualEntry) => call<number>("add_manual_entry", { entry }),
  aircraftFlights: (id: number) => call<FlightRow[]>("aircraft_flights", { id }),
  remoteInfo: () => call<RemoteInfo>("remote_info"),
  remoteSet: (enabled: boolean | null, canEdit: boolean | null) => call<RemoteInfo>("remote_set", { enabled, canEdit }),
  remoteNewCode: () => call<RemoteInfo>("remote_new_code"),
  remoteForget: () => call<RemoteInfo>("remote_forget"),
  wxCategories: () => call<WxCategories>("wx_categories"),
  windGrid: (level: string) => call<WindGrid>("wind_grid", { level }),
  temperatureGrid: (level: string) => call<TempGrid>("temperature_grid", { level }),
  /** Cloud cover (tenths of a percent) or surface visibility (tens of metres), same layout as the temperature. */
  skyGrid: (kind: "cloud" | "vis") => call<TempGrid>("sky_grid", { kind }),
  icingGrid: (fl: number) => call<IcingGrid>("icing_grid", { fl }),
  navaids: (w: number, s: number, e: number, n: number, waypoints: boolean) => call<NavView>("navaids", { w, s, e, n, waypoints }),
  navaidsRefresh: () => call<void>("navaids_refresh"),
  /** US Class B, C and D airspace, GeoJSON text (FAA, 28-day cycle). */
  usAirspace: () => call<string>("us_airspace"),
  suaAirspace: () => call<string>("sua_airspace"),
  worldAirspace: (w: number, s: number, e: number, n: number) => call<{ geojson: string; more: boolean }>("world_airspace", { w, s, e, n }),
  /** US obstacles (towers, antennas, turbines), GeoJSON text (FAA DOF). */
  usObstacles: () => call<string>("us_obstacles"),
  /** A PFD is open: keep its feed coming for a few more seconds. */
  pfdWatch: () => call<void>("pfd_watch"),
  /** A drawn route as an MSFS .PLN: [file name, text]. */
  plnText: (route: PlnRoute) => call<[string, string]>("pln_text", { route }),
  /** Save a drawn route as an MSFS .PLN (PC only): the path, or null if cancelled. */
  savePln: (route: PlnRoute) => call<string | null>("save_pln", { route }),
  wxAdvisories: () => call<WxAdvisories>("wx_advisories"),
  onlineNetwork: (net: Net) => call<NetworkData>("online_network", { net }),
  onlinePilot: (net: Net, callsign: string) => call<NetPilot | null>("online_pilot", { net, callsign }),
  /** openAIP's airports in a box: [lat, lon, ICAO, private, fuel]. */
  airportExtras: (w: number, s: number, e: number, n: number) => call<{ list: [number, number, string, boolean, boolean][]; more: boolean }>("airport_extras", { w, s, e, n }),
  mapAirports: (detail: number) => call<MapAirport[]>("map_airports", { detail }),
  airportCard: (ident: string) => call<AirportCardData | null>("airport_card", { ident }),
  /** An airport's elevation and runway thresholds (glidepath view). */
  setLandingRunway: (runway: string) => call<void>("set_landing_runway", { runway }),
  runwayEnds: (ident: string) => call<RunwayEnds | null>("runway_ends", { ident }),
  airportWeather: (ident: string) => call<AirportWeather | null>("airport_weather", { ident }),
  efbInfo: () => call<EfbInfo>("efb_info"),
  efbInstall: (folder: string | null = null) => call<EfbInfo>("efb_install", { folder }),
  efbUninstall: () => call<EfbInfo>("efb_uninstall"),
  checkUpdate: () => call<UpdateInfo>("check_update"),
  installUpdate: (url: string) => call<void>("install_update", { url }),
  version: () => call<string>("app_version"),
  addSampleFlights: () => call<number>("add_sample_flights"),
  removeSampleFlights: () => call<number>("remove_sample_flights"),
};

export async function on<T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  if (inTauri) return listen<T>(event, (e) => cb(e.payload));
  if (inRemote) return remoteListen(event, cb as (p: unknown) => void);
  const { mockListen } = await import("./mock");
  return mockListen(event, cb as (p: unknown) => void);
}
