//! The tracking engine: one background thread that watches for sims, polls
//! the active connector, runs the tracker, journals the flight and saves it.
//!
//! The UI never sees raw telemetry. It gets a throttled status snapshot
//! (twice a second) holding only the track points added since the last one.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use store::journal::{self, Journal, JournalHeader};
use store::{Place, SaveContext, Store};
use tauri::{AppHandle, Emitter};
use telemetry::{AircraftInfo, EngineType, SimConnector, SimKind, SimMessage, Telemetry};
use crate::simbrief::FlightPlan;
use tracker::{Config, FlightSummary, LandingReport, LiveFlight, Phase, Tracker, TrackerOutput};

const POLL: Duration = Duration::from_millis(20);
const STATUS_EVERY: Duration = Duration::from_millis(500);
const SCAN_EVERY: Duration = Duration::from_secs(3);
const RETRY_CONNECT: Duration = Duration::from_secs(5);

pub enum Command {
    StartDemo { route: Option<usize>, speedup: f64 },
    StopDemo,
    EndFlight,
    /// The pilot pressed "Start a flight": track from now, with this plan.
    StartFlight { plan: Option<FlightPlan> },
    /// Throw away the flight in progress without saving it.
    CancelFlight,
    /// Attach (or clear) a flight plan without changing anything else.
    SetPlan(Option<FlightPlan>),
}

/// What the UI shows. Sent as the `status` event and from `get_status`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct Status {
    /// "waiting" | "connecting" | "connected"
    pub link: String,
    pub sim: Option<SimKind>,
    pub sim_label: Option<String>,
    pub sim_version: Option<String>,
    /// A sim we can see but cannot connect to yet, e.g. X-Plane before Milestone 2.
    pub notice: Option<String>,
    pub aircraft: Option<AircraftInfo>,
    /// Connected, but no flight yet: waiting for the pilot to get in the
    /// aircraft (power on or an engine running) or press "Start a flight".
    pub waiting_for_pilot: bool,
    /// "Start a flight" was pressed and the flight has not begun yet.
    pub armed: bool,
    /// The flight plan for this (or the next) flight.
    pub plan: Option<FlightPlan>,
    pub phase: Option<Phase>,
    pub phase_label: Option<String>,
    pub live: Option<LiveFlight>,
    pub dep: Option<Place>,
    pub telemetry: Option<Telemetry>,
    pub last_landing: Option<LandingReport>,
    pub demo: bool,
    /// Changes whenever a new flight begins, so the UI starts a fresh track.
    pub session: u64,
    /// Points added since the previous status event.
    pub track_tail: Vec<Pt>,
}

/// Status plus the whole current track, for a UI that has just opened.
#[derive(Debug, Clone, Serialize)]
pub struct FullStatus {
    #[serde(flatten)]
    pub status: Status,
    pub track: Vec<Pt>,
}

#[derive(Default)]
pub struct Shared {
    pub status: Status,
    pub track: Vec<Pt>,
    /// The current plan's full briefing, kept out of the twice-a-second
    /// status and fetched when the pilot opens it.
    pub briefing: Option<crate::simbrief::Ofp>,
}

/// One live track point: [lon, lat, altitude ft, ground speed kt, time ms].
pub type Pt = [f64; 5];

pub type SharedState = Arc<Mutex<Shared>>;

#[derive(Serialize, Clone)]
struct Saved {
    id: i64,
    dep: Option<String>,
    arr: Option<String>,
}

pub fn place_at(refdata: &refdata::RefData, lat: f64, lon: f64, heli: bool) -> Option<Place> {
    refdata.airport_at(lat, lon, heli).map(|a| Place {
        icao: a.ident.clone(),
        name: a.name.clone(),
        country: a.country.clone(),
        region: a.region.clone(),
    })
}

pub fn save(
    db: &Mutex<Store>,
    refdata: &refdata::RefData,
    f: &FlightSummary,
    plan: Option<&FlightPlan>,
) -> Option<(i64, SaveContext)> {
    let heli = f.aircraft.engine_type == EngineType::Helo;
    let ctx = SaveContext {
        dep: place_at(refdata, f.dep_lat, f.dep_lon, heli),
        arr: place_at(refdata, f.arr_lat, f.arr_lon, heli),
        planned_arr_icao: plan.map(|p| p.destination.icao.clone()).filter(|s| !s.is_empty()),
        callsign: plan.map(|p| p.callsign.clone()).unwrap_or_default(),
        flight_no: plan.map(|p| format!("{}{}", p.airline, p.flight_number)).unwrap_or_default(),
        plan: plan.and_then(|p| serde_json::to_string(p).ok()),
        source: None,
    };
    match db.lock().unwrap().save_flight(f, &ctx) {
        Ok(id) => Some((id, ctx)),
        Err(e) => {
            log::error!("could not save flight: {e}");
            None
        }
    }
}

/// Save a flight left behind by a crash, if any.
pub fn recover_journal(dir: &std::path::Path, db: &Mutex<Store>, refdata: &refdata::RefData) {
    let Some(r) = journal::recover(dir) else { return };
    let mut tr = Tracker::new(r.header.sim, r.header.aircraft, Config::default());
    for s in &r.samples {
        tr.update(s);
    }
    if let Some(f) = tr.end("recovered after a crash") {
        if let Some((id, _)) = save(db, refdata, &f, None) {
            log::info!("recovered crashed flight as #{id}");
        }
    }
}

struct Engine {
    app: AppHandle,
    shared: SharedState,
    db: Arc<Mutex<Store>>,
    refdata: Arc<refdata::RefData>,
    journal_dir: PathBuf,
    watcher: watcher::Watcher,
    conn: Option<Box<dyn SimConnector>>,
    demo: bool,
    sim: Option<SimKind>,
    sim_version: Option<String>,
    aircraft: AircraftInfo,
    tracker: Option<Tracker>,
    journal: Option<Journal>,
    dep: Option<Place>,
    last_sample: Option<Telemetry>,
    notice: Option<String>,
    tail: Vec<Pt>,
    last_scan: Option<Instant>,
    last_connect: Option<Instant>,
    last_status: Instant,
    session: u64,
    last_diag: Instant,
    /// "Start a flight" was pressed: begin tracking even without power.
    armed: bool,
    /// After a flight ends, wait for power or engines to go off (or the
    /// button) before starting another, so parking does not begin a new one.
    need_reset: bool,
    plan: Option<FlightPlan>,
    /// Last power and camera state seen, to log only when they change.
    last_power: Option<Option<bool>>,
    last_camera: Option<Option<u8>>,
    last_paused: Option<bool>,
}

pub fn spawn(
    app: AppHandle,
    shared: SharedState,
    db: Arc<Mutex<Store>>,
    refdata: Arc<refdata::RefData>,
    journal_dir: PathBuf,
    rx: Receiver<Command>,
) {
    std::thread::Builder::new()
        .name("flightdeck-engine".into())
        .spawn(move || {
            let mut e = Engine {
                app,
                shared,
                db,
                refdata,
                journal_dir,
                watcher: watcher::Watcher::new(),
                conn: None,
                demo: false,
                sim: None,
                sim_version: None,
                aircraft: AircraftInfo::default(),
                tracker: None,
                journal: None,
                dep: None,
                last_sample: None,
                notice: None,
                tail: Vec::new(),
                last_scan: None,
                last_connect: None,
                last_status: Instant::now(),
                session: 0,
                last_diag: Instant::now(),
                armed: false,
                need_reset: false,
                plan: None,
                last_power: None,
                last_camera: None,
                last_paused: None,
            };
            loop {
                match rx.try_recv() {
                    Ok(cmd) => e.command(cmd),
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => break,
                }
                e.tick();
                std::thread::sleep(POLL);
            }
        })
        .expect("spawn engine thread");
}

impl Engine {
    fn command(&mut self, cmd: Command) {
        match cmd {
            Command::StartDemo { route, speedup } => {
                if self.conn.is_some() && !self.demo {
                    return; // a real sim is connected; it wins
                }
                self.end_flight("demo restarted");
                let routes = conn_demo::sample_routes();
                let ix = route.unwrap_or_else(|| (now_ms() / 1000) as usize % routes.len()) % routes.len();
                let opts = conn_demo::DemoOptions { speedup: speedup.clamp(1.0, 200.0), ..Default::default() };
                let flight = conn_demo::DemoFlight::new(routes[ix].clone(), opts, now_ms());
                self.conn = Some(Box::new(conn_demo::DemoConnector::new(flight)));
                self.demo = true;
                self.armed = true;
            }
            Command::StopDemo => {
                if self.demo {
                    self.end_flight("demo stopped");
                    self.disconnect();
                }
            }
            Command::EndFlight => self.end_flight("ended by pilot"),
            Command::StartFlight { plan } => {
                self.armed = true;
                if plan.is_some() {
                    self.plan = plan;
                }
            }
            Command::SetPlan(plan) => self.plan = plan,
            Command::CancelFlight => self.cancel_flight(),
        }
        self.shared.lock().unwrap().briefing = self.plan.as_ref().and_then(|p| p.ofp.clone());
        self.publish(true);
    }

    fn tick(&mut self) {
        if self.conn.is_none() {
            self.look_for_sim();
        }
        if let Some(conn) = self.conn.as_mut() {
            match conn.poll() {
                Ok(msgs) => {
                    for m in msgs {
                        self.handle(m);
                    }
                }
                Err(e) => {
                    log::warn!("connector error: {e}");
                    self.end_flight("connection lost");
                    self.disconnect();
                }
            }
        }
        if self.last_status.elapsed() >= STATUS_EVERY {
            self.publish(false);
        }
    }

    fn look_for_sim(&mut self) {
        if self.last_scan.is_some_and(|t| t.elapsed() < SCAN_EVERY) {
            return;
        }
        self.last_scan = Some(Instant::now());
        let found = self.watcher.scan();
        self.notice = None;
        for d in found {
            match d {
                watcher::Detected::Msfs(hint) => {
                    if self.last_connect.is_some_and(|t| t.elapsed() < RETRY_CONNECT) {
                        continue;
                    }
                    self.last_connect = Some(Instant::now());
                    match conn_simconnect::SimConnectConnector::connect(Some(hint)) {
                        Ok(c) => {
                            log::info!("connected to {hint:?}");
                            self.sim = Some(c.sim());
                            self.conn = Some(Box::new(c));
                            return;
                        }
                        Err(e) => {
                            // MSFS takes a while to start accepting connections.
                            log::info!("{hint:?} found, not ready yet: {e}");
                            self.notice = Some(match e {
                                telemetry::ConnectorError::LibraryMissing(_) => {
                                    "MSFS is running, but SimConnect.dll was not found. See the README.".into()
                                }
                                _ => format!("{} is starting…", hint.label()),
                            });
                        }
                    }
                }
                watcher::Detected::XPlane => {
                    self.notice = Some("X-Plane is running. X-Plane tracking arrives in the next milestone.".into());
                }
            }
        }
    }

    fn handle(&mut self, m: SimMessage) {
        match m {
            SimMessage::Connected { sim, app_version, .. } => {
                self.sim = Some(sim);
                self.sim_version = Some(app_version);
                self.publish(true);
            }
            SimMessage::Aircraft(a) => {
                self.aircraft = a.clone();
                if let Some(t) = self.tracker.as_mut() {
                    t.set_aircraft(a.clone());
                }
                if let (Some(j), Some(sim)) = (self.journal.as_mut(), self.sim) {
                    let _ = j.set_aircraft(JournalHeader { sim, aircraft: a });
                }
            }
            SimMessage::Telemetry(s) => self.sample(s),
            SimMessage::Disconnected { reason } => {
                self.end_flight(&reason);
                self.disconnect();
            }
        }
    }

    fn sample(&mut self, s: Telemetry) {
        let sim = self.sim.unwrap_or(SimKind::Demo);
        // Note changes that decide when flights start and stop.
        if self.last_power != Some(s.powered) || self.last_camera != Some(s.camera_state) || self.last_paused != Some(s.paused) {
            log::info!(
                "sim state: powered {:?}, camera {:?}, paused {}, on ground {}, engines {}, alt {:.0} ft (agl {:.0}), gs {:.1} kt",
                s.powered, s.camera_state, s.paused, s.on_ground, s.engines_running, s.alt_msl_ft, s.alt_agl_ft, s.gs_kt
            );
            self.last_power = Some(s.powered);
            self.last_camera = Some(s.camera_state);
            self.last_paused = Some(s.paused);
        }
        if self.tracker.as_ref().is_none_or(Tracker::is_finished) {
            // No flight yet. In the MSFS menus the sim still sends a position
            // (often in the air), so only start once the pilot is really in
            // the aircraft: it has power or an engine running, or the pilot
            // pressed "Start a flight". Sims that do not report power (demo,
            // X-Plane) start straight away.
            let msfs = matches!(sim, SimKind::Msfs2020 | SimKind::Msfs2024);
            let signal = !s.paused && (s.engines_running > 0 || s.powered.unwrap_or(!msfs));
            if !signal {
                self.need_reset = false;
            }
            // "Start a flight" skips the power check but still waits for a
            // real cockpit: not paused, and on the ground or powered (the
            // menus can report an unpowered aircraft hanging in the air).
            let armed_ready = self.armed && !s.paused && (signal || s.on_ground);
            if !(armed_ready || (signal && !self.need_reset)) {
                self.last_sample = Some(s);
                return;
            }
            log::info!(
                "flight started: by {} | powered {:?}, engines {}, on ground {}, gs {:.1} kt, alt {:.0} ft, camera {:?}",
                if armed_ready && !signal { "Start a flight" } else if s.engines_running > 0 { "engine running" } else { "power on" },
                s.powered, s.engines_running, s.on_ground, s.gs_kt, s.alt_msl_ft, s.camera_state
            );
            self.tracker = Some(Tracker::new(sim, self.aircraft.clone(), Config::default()));
            self.journal = Journal::create(&self.journal_dir, JournalHeader { sim, aircraft: self.aircraft.clone() })
                .map_err(|e| log::warn!("journal unavailable: {e}"))
                .ok();
            self.dep = None;
            self.session += 1;
            self.shared.lock().unwrap().track.clear();
            self.tail.clear();
        }
        self.last_sample = Some(s);
        self.diagnose(&s);
        let outputs = self.tracker.as_mut().unwrap().update(&s);
        for o in outputs {
            match o {
                TrackerOutput::TrackPoint(p) => {
                    if let Some(j) = self.journal.as_mut() {
                        let _ = j.append(&s);
                    }
                    let pt = [p.lon, p.lat, f64::from(p.alt_ft), f64::from(p.gs_kt), p.t_ms as f64];
                    self.tail.push(pt);
                    self.shared.lock().unwrap().track.push(pt);
                }
                TrackerOutput::PhaseChanged { from, to, .. } => {
                    log::info!(
                        "phase {from:?} -> {to:?}: gs {:.1} kt, on ground {}, agl {:.0} ft, vs {:.0} fpm, engines {}, brake {}, powered {:?}",
                        s.gs_kt, s.on_ground, s.alt_agl_ft, s.vs_fpm, s.engines_running, s.parking_brake, s.powered
                    );
                    if to == Phase::TaxiOut || (to == Phase::Airborne && self.dep.is_none()) || self.dep.is_none() {
                        let heli = self.aircraft.engine_type == EngineType::Helo;
                        self.dep = place_at(&self.refdata, s.lat, s.lon, heli);
                    }
                    self.publish(true);
                }
                TrackerOutput::Touchdown(l) => {
                    let _ = self.app.emit("touchdown", &l);
                }
                TrackerOutput::Completed(f) => {
                    log::info!("flight complete: {} ({:?})", f.end_reason, f.status);
                    self.store(&f)
                }
            }
        }
    }

    /// While taxiing in, log the values the "parked" check uses, so a sim
    /// that reports them differently is easy to spot in the terminal.
    fn diagnose(&mut self, s: &Telemetry) {
        let taxiing_in = self.tracker.as_ref().and_then(Tracker::phase) == Some(Phase::TaxiIn);
        if taxiing_in && self.last_diag.elapsed() >= Duration::from_secs(5) {
            self.last_diag = Instant::now();
            log::info!(
                "taxi in: ground speed {:.2} kt, on ground {}, engines running {}, parking brake {}",
                s.gs_kt,
                s.on_ground,
                s.engines_running,
                s.parking_brake
            );
        }
    }

    fn store(&mut self, f: &FlightSummary) {
        let plan = self.plan.take();
        self.shared.lock().unwrap().briefing = None;
        self.armed = false;
        self.need_reset = true;
        // The sim gave no usable type code: take it from the flight plan.
        let mut filled;
        let mut f = f;
        if let Some(p) = plan.as_ref().filter(|p| f.aircraft.icao_type.is_empty() && !p.aircraft_icao.is_empty()) {
            filled = f.clone();
            filled.aircraft.icao_type = p.aircraft_icao.clone();
            f = &filled;
        }
        if let Some((id, ctx)) = save(&self.db, &self.refdata, f, plan.as_ref()) {
            let _ = self.app.emit(
                "flight-saved",
                Saved { id, dep: ctx.dep.map(|p| p.icao), arr: ctx.arr.map(|p| p.icao) },
            );
        }
        if let Some(j) = self.journal.take() {
            let _ = j.finish();
        }
    }

    /// Discard the flight in progress: nothing is saved, the crash journal
    /// is removed, and the next flight waits for power off and on (or the
    /// Start a flight button) so it does not begin again straight away.
    fn cancel_flight(&mut self) {
        if self.tracker.take().is_some() {
            log::info!("flight cancelled by pilot; nothing saved");
        }
        if let Some(j) = self.journal.take() {
            let _ = j.finish();
        }
        self.shared.lock().unwrap().track.clear();
        self.tail.clear();
        self.dep = None;
        self.armed = false;
        self.need_reset = true;
        self.session += 1;
    }

    fn end_flight(&mut self, reason: &str) {
        if self.tracker.as_ref().is_some_and(|t| !t.is_finished()) {
            log::info!("flight ended: {reason}");
        }
        if let Some(mut t) = self.tracker.take() {
            match t.end(reason) {
                Some(f) => self.store(&f),
                None => {
                    if let Some(j) = self.journal.take() {
                        let _ = j.finish();
                    }
                }
            }
        }
        self.shared.lock().unwrap().track.clear();
        self.tail.clear();
        self.dep = None;
        self.armed = false;
        self.need_reset = true;
    }

    fn disconnect(&mut self) {
        self.conn = None;
        self.demo = false;
        self.sim = None;
        self.sim_version = None;
        self.last_sample = None;
        self.aircraft = AircraftInfo::default();
        self.last_connect = Some(Instant::now());
        self.need_reset = false;
        self.publish(true);
    }

    fn publish(&mut self, force: bool) {
        if !force && self.conn.is_none() && self.tail.is_empty() && self.last_status.elapsed() < Duration::from_secs(3) {
            return;
        }
        self.last_status = Instant::now();
        let live = self.tracker.as_ref().and_then(Tracker::live);
        let phase = live.as_ref().map(|l| l.phase);
        let status = Status {
            link: if self.conn.is_some() { "connected" } else if self.notice.is_some() { "connecting" } else { "waiting" }.into(),
            sim: self.sim,
            sim_label: self.sim.map(|s| s.label().to_string()),
            sim_version: self.sim_version.clone(),
            notice: self.notice.clone(),
            aircraft: (!self.aircraft.title.is_empty()
                || !self.aircraft.icao_type.is_empty()
                || !self.aircraft.registration.is_empty())
            .then(|| self.aircraft.clone()),
            waiting_for_pilot: self.conn.is_some() && self.tracker.as_ref().is_none_or(Tracker::is_finished),
            plan: self.plan.as_ref().map(|p| FlightPlan { ofp: None, ..p.clone() }),
            armed: self.armed,
            phase,
            phase_label: phase.map(|p| p.label().to_string()),
            last_landing: live.as_ref().and_then(|l| l.last_landing.clone()),
            live,
            dep: self.dep.clone(),
            telemetry: self.last_sample,
            demo: self.demo,
            session: self.session,
            track_tail: std::mem::take(&mut self.tail),
        };
        self.shared.lock().unwrap().status = Status { track_tail: vec![], ..status.clone() };
        let _ = self.app.emit("status", &status);
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}
