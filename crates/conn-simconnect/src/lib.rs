//! MSFS 2020 and MSFS 2024 connector.
//!
//! Both sims speak the same SimConnect API, so one connector serves both. The
//! version is told apart by the OPEN message (and the process name the
//! watcher saw). Message parsing is plain byte-offset reading, kept separate
//! from the FFI so it can be unit-tested on any OS.

mod ffi;
pub mod parse;

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use telemetry::{ConnectorError, SimConnector, SimKind, SimMessage};

use parse::{Parsed, AIRCRAFT_BASE, AIRCRAFT_VARS, POWER_BASE, POWER_VARS, TELEMETRY_VARS};

pub use ffi::candidate_paths;

const DEF_TELEMETRY: u32 = 1;
const REQ_TELEMETRY: u32 = 1;
pub(crate) const REQ_SIM_STATE: u32 = 3;
pub(crate) const EVENT_PAUSE: u32 = 1;
pub(crate) const EVENT_SIM: u32 = 2;
pub(crate) const EVENT_AIRCRAFT_LOADED: u32 = 3;
pub(crate) const EVENT_CRASHED: u32 = 4;
pub(crate) const EVENT_SIM_START: u32 = 5;

/// Most messages read per poll, so one poll never stalls the thread.
const MAX_PER_POLL: usize = 500;
/// Ask again for aircraft details until they arrive.
const AIRCRAFT_RETRY: Duration = Duration::from_secs(15);

pub struct SimConnectConnector {
    api: ffi::Api,
    handle: ffi::Handle,
    sim: SimKind,
    state: parse::State,
    /// Which call each SimConnect send ID belongs to, to explain exceptions.
    sent: HashMap<u32, String>,
    have_aircraft: bool,
    last_aircraft_request: Instant,
}

// The raw handle is only used from the thread that polls the connector.
unsafe impl Send for SimConnectConnector {}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

impl SimConnectConnector {
    /// Load SimConnect.dll and connect to a running MSFS.
    /// `hint` is the version the process watcher saw, if any.
    pub fn connect(hint: Option<SimKind>) -> Result<Self, ConnectorError> {
        let api = ffi::Api::load().map_err(ConnectorError::LibraryMissing)?;
        let handle = api
            .open("FlightDeck")
            .map_err(|hr| ConnectorError::NotAvailable(format!("SimConnect_Open failed (0x{hr:08X}); is MSFS running?")))?;
        let mut c = SimConnectConnector {
            api,
            handle,
            sim: hint.unwrap_or(SimKind::Msfs2020),
            state: parse::State::default(),
            sent: HashMap::new(),
            have_aircraft: false,
            last_aircraft_request: Instant::now(),
        };
        if let Err(e) = c.setup() {
            return Err(ConnectorError::Failure(e));
        }
        Ok(c)
    }

    /// Remember what a call was, so a later exception can name it.
    fn note(&mut self, what: &str) {
        if let Some(id) = self.api.last_send_id(self.handle) {
            self.sent.insert(id, what.to_string());
        }
    }

    fn check(&mut self, hr: i32, what: &str) -> Result<(), String> {
        self.note(what);
        if hr == ffi::S_OK {
            Ok(())
        } else {
            Err(format!("{what}: 0x{hr:08X}"))
        }
    }

    fn setup(&mut self) -> Result<(), String> {
        let h = self.handle;
        for v in TELEMETRY_VARS {
            let hr = self.api.add_to_data_definition(h, DEF_TELEMETRY, v.name, Some(v.units), ffi::DATATYPE_FLOAT64);
            self.check(hr, &format!("telemetry variable '{}'", v.name))?;
        }
        for (i, v) in AIRCRAFT_VARS.iter().enumerate() {
            let def = AIRCRAFT_BASE + i as u32;
            let (units, ty) = if v.units.is_empty() {
                (None, ffi::DATATYPE_STRING256)
            } else {
                (Some(v.units), ffi::DATATYPE_FLOAT64)
            };
            let hr = self.api.add_to_data_definition(h, def, v.name, units, ty);
            self.check(hr, &format!("aircraft variable '{}'", v.name))?;
        }
        for (i, v) in POWER_VARS.iter().enumerate() {
            let def = POWER_BASE + i as u32;
            let hr = self.api.add_to_data_definition(h, def, v.name, Some(v.units), ffi::DATATYPE_FLOAT64);
            self.check(hr, &format!("power variable '{}'", v.name))?;
        }
        for (id, name) in [
            (EVENT_PAUSE, "Pause"),
            (EVENT_SIM, "Sim"),
            (EVENT_AIRCRAFT_LOADED, "AircraftLoaded"),
            (EVENT_CRASHED, "Crashed"),
            (EVENT_SIM_START, "SimStart"),
        ] {
            let hr = self.api.subscribe(h, id, name);
            self.check(hr, &format!("event '{name}'"))?;
        }
        // Are we in a flight right now, or in the menus?
        let hr = self.api.request_system_state(h, REQ_SIM_STATE, "Sim");
        self.check(hr, "system state 'Sim'")?;
        self.request_aircraft();
        let hr = self.api.request_data(h, REQ_TELEMETRY, DEF_TELEMETRY, ffi::PERIOD_SIM_FRAME);
        self.check(hr, "telemetry request")?;
        for (i, v) in POWER_VARS.iter().enumerate() {
            let id = POWER_BASE + i as u32;
            let _ = self.api.request_data(h, id, id, ffi::PERIOD_SECOND);
            self.note(&format!("power request '{}'", v.name));
        }
        Ok(())
    }

    fn request_aircraft(&mut self) {
        self.last_aircraft_request = Instant::now();
        for (i, v) in AIRCRAFT_VARS.iter().enumerate() {
            let id = AIRCRAFT_BASE + i as u32;
            let _ = self.api.request_data(self.handle, id, id, ffi::PERIOD_ONCE);
            self.note(&format!("aircraft request '{}'", v.name));
        }
    }
}

impl SimConnector for SimConnectConnector {
    fn sim(&self) -> SimKind {
        self.sim
    }

    fn poll(&mut self) -> Result<Vec<SimMessage>, ConnectorError> {
        let mut out = Vec::new();
        for _ in 0..MAX_PER_POLL {
            let Some(buf) = self.api.next_dispatch(self.handle) else { break };
            match parse::parse(&buf, &mut self.state, now_ms()) {
                Parsed::Open { app_name, version_major, version_minor } => {
                    self.sim = parse::sim_from_version(version_major, self.sim);
                    log::info!("SimConnect open: {app_name} {version_major}.{version_minor} -> {:?}", self.sim);
                    out.push(SimMessage::Connected {
                        sim: self.sim,
                        app_name,
                        app_version: format!("{version_major}.{version_minor}"),
                    });
                }
                Parsed::Telemetry(t) => out.push(SimMessage::Telemetry(t)),
                Parsed::Aircraft(a) => {
                    if !self.have_aircraft {
                        log::info!("aircraft: {a:?}");
                    }
                    self.have_aircraft = true;
                    out.push(SimMessage::Aircraft(a));
                }
                Parsed::AircraftReloaded => {
                    log::info!("aircraft or flight loaded; asking for aircraft details");
                    self.request_aircraft();
                }
                Parsed::SimRunning(running) => {
                    log::info!("MSFS {}", if running { "is in a flight" } else { "is in the menus or loading" });
                    if running {
                        self.request_aircraft();
                    }
                }
                Parsed::Quit => {
                    out.push(SimMessage::Disconnected { reason: "MSFS closed".into() });
                    break;
                }
                Parsed::Exception { code, send_id, index } => {
                    let what = self.sent.get(&send_id).map(String::as_str).unwrap_or("an unknown request");
                    log::warn!("MSFS rejected {what}: SimConnect exception {code} (parameter {index})");
                }
                Parsed::Other => {}
            }
        }
        if !self.have_aircraft
            && self.state.in_flight != Some(false)
            && self.last_aircraft_request.elapsed() >= AIRCRAFT_RETRY
        {
            log::info!("no aircraft details yet; asking again");
            self.request_aircraft();
        }
        Ok(out)
    }
}

impl Drop for SimConnectConnector {
    fn drop(&mut self) {
        self.api.close(self.handle);
    }
}
