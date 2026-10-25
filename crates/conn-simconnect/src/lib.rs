//! MSFS 2020 and MSFS 2024 connector.
//!
//! Both sims speak the same SimConnect API, so one connector serves both. The
//! version is told apart by the OPEN message (and the process name the
//! watcher saw). Message parsing is plain byte-offset reading, kept separate
//! from the FFI so it can be unit-tested on any OS.

mod ffi;
pub mod parse;

use std::time::{SystemTime, UNIX_EPOCH};

use telemetry::{ConnectorError, SimConnector, SimKind, SimMessage};

use parse::{Parsed, AIRCRAFT_VARS, TELEMETRY_VARS};

pub use ffi::candidate_paths;

const DEF_TELEMETRY: u32 = 1;
const DEF_AIRCRAFT: u32 = 2;
const REQ_TELEMETRY: u32 = 1;
const REQ_AIRCRAFT: u32 = 2;
pub(crate) const EVENT_PAUSE: u32 = 1;
pub(crate) const EVENT_SIM: u32 = 2;
pub(crate) const EVENT_AIRCRAFT_LOADED: u32 = 3;
pub(crate) const EVENT_CRASHED: u32 = 4;

/// Most messages read per poll, so one poll never stalls the thread.
const MAX_PER_POLL: usize = 500;

pub struct SimConnectConnector {
    api: ffi::Api,
    handle: ffi::Handle,
    sim: SimKind,
    state: parse::State,
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

        let setup = || -> Result<(), String> {
            let check = |hr: i32, what: &str| if hr == ffi::S_OK { Ok(()) } else { Err(format!("{what}: 0x{hr:08X}")) };
            for v in TELEMETRY_VARS {
                check(api.add_to_data_definition(handle, DEF_TELEMETRY, v.name, Some(v.units), ffi::DATATYPE_FLOAT64), v.name)?;
            }
            for v in AIRCRAFT_VARS {
                let (units, ty) = if v.units.is_empty() {
                    (None, ffi::DATATYPE_STRING256)
                } else {
                    (Some(v.units), ffi::DATATYPE_FLOAT64)
                };
                check(api.add_to_data_definition(handle, DEF_AIRCRAFT, v.name, units, ty), v.name)?;
            }
            check(api.subscribe(handle, EVENT_PAUSE, "Pause"), "Pause")?;
            check(api.subscribe(handle, EVENT_SIM, "Sim"), "Sim")?;
            check(api.subscribe(handle, EVENT_AIRCRAFT_LOADED, "AircraftLoaded"), "AircraftLoaded")?;
            check(api.subscribe(handle, EVENT_CRASHED, "Crashed"), "Crashed")?;
            check(api.request_data(handle, REQ_AIRCRAFT, DEF_AIRCRAFT, ffi::PERIOD_ONCE), "aircraft request")?;
            check(api.request_data(handle, REQ_TELEMETRY, DEF_TELEMETRY, ffi::PERIOD_SIM_FRAME), "telemetry request")?;
            Ok(())
        };
        if let Err(e) = setup() {
            api.close(handle);
            return Err(ConnectorError::Failure(e));
        }
        Ok(SimConnectConnector { api, handle, sim: hint.unwrap_or(SimKind::Msfs2020), state: parse::State::default() })
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
                Parsed::Aircraft(a) => out.push(SimMessage::Aircraft(a)),
                Parsed::AircraftReloaded => {
                    let _ = self.api.request_data(self.handle, REQ_AIRCRAFT, DEF_AIRCRAFT, ffi::PERIOD_ONCE);
                }
                Parsed::Quit => {
                    out.push(SimMessage::Disconnected { reason: "MSFS closed".into() });
                    break;
                }
                Parsed::Exception(code) => log::warn!("SimConnect exception {code}"),
                Parsed::Other => {}
            }
        }
        Ok(out)
    }
}

impl Drop for SimConnectConnector {
    fn drop(&mut self) {
        self.api.close(self.handle);
    }
}
