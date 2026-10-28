//! The common telemetry model.
//!
//! Every simulator connector converts its own data into these types, so the
//! tracker, store and UI never need to know which simulator is running.
//! All units are normalized here: feet, knots, feet per minute, degrees, kg.

use serde::{Deserialize, Serialize};

pub mod geo;
pub mod sun;
pub mod units;

/// Which simulator produced the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimKind {
    Msfs2020,
    Msfs2024,
    Xp11,
    Xp12,
    /// Built-in simulated flight, for development and demos without a sim.
    Demo,
    /// Imported from another tracker that did not say which simulator.
    Other,
}

impl SimKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SimKind::Msfs2020 => "msfs2020",
            SimKind::Msfs2024 => "msfs2024",
            SimKind::Xp11 => "xp11",
            SimKind::Xp12 => "xp12",
            SimKind::Demo => "demo",
            SimKind::Other => "other",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SimKind::Msfs2020 => "MSFS 2020",
            SimKind::Msfs2024 => "MSFS 2024",
            SimKind::Xp11 => "X-Plane 11",
            SimKind::Xp12 => "X-Plane 12",
            SimKind::Demo => "Demo",
            SimKind::Other => "Other simulator",
        }
    }

    pub fn parse(s: &str) -> Option<SimKind> {
        Some(match s {
            "msfs2020" => SimKind::Msfs2020,
            "msfs2024" => SimKind::Msfs2024,
            "xp11" => SimKind::Xp11,
            "xp12" => SimKind::Xp12,
            "demo" => SimKind::Demo,
            "other" => SimKind::Other,
            _ => return None,
        })
    }
}

/// One telemetry sample. Connectors emit these at 10 to 60 Hz.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Telemetry {
    /// Wall-clock time of the sample, Unix milliseconds (UTC).
    pub t_ms: i64,
    pub lat: f64,
    pub lon: f64,
    pub alt_msl_ft: f64,
    pub alt_agl_ft: f64,
    pub gs_kt: f64,
    pub ias_kt: f64,
    pub hdg_true_deg: f64,
    pub vs_fpm: f64,
    /// Nose-up positive.
    pub pitch_deg: f64,
    /// Right-wing-down positive.
    pub bank_deg: f64,
    pub g_load: f64,
    pub on_ground: bool,
    pub parking_brake: bool,
    /// Number of engines currently running.
    pub engines_running: u8,
    pub fuel_kg: f64,
    pub paused: bool,
    /// 1.0 = real time.
    pub sim_rate: f64,
    pub wind_dir_deg: f64,
    pub wind_kt: f64,
    /// Touchdown vertical speed reported by the sim itself (MSFS only),
    /// negative = descending. `None` when the sim does not provide it.
    pub touchdown_vs_fpm: Option<f64>,
    /// The aircraft has electrical power (battery switch on or a live main
    /// bus). `None` when the sim does not say. Used to tell "sitting in the
    /// cockpit" from "looking at the menus".
    #[serde(default)]
    pub powered: Option<bool>,
    /// MSFS camera state, for diagnostics (2 = cockpit, 3 = external, ...).
    #[serde(default)]
    pub camera_state: Option<u8>,
}

/// What the connector knows about the loaded aircraft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AircraftInfo {
    /// The sim's full title, e.g. "Cessna Skyhawk G1000 Asobo".
    pub title: String,
    /// ICAO type designator if the sim exposes one, e.g. "C172".
    pub icao_type: String,
    pub registration: String,
    pub engine_count: u8,
    pub engine_type: EngineType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EngineType {
    Piston,
    Jet,
    Turboprop,
    Helo,
    Glider,
    Electric,
    #[default]
    Unknown,
}

impl EngineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EngineType::Piston => "piston",
            EngineType::Jet => "jet",
            EngineType::Turboprop => "turboprop",
            EngineType::Helo => "helo",
            EngineType::Glider => "glider",
            EngineType::Electric => "electric",
            EngineType::Unknown => "unknown",
        }
    }
}

/// Everything a connector can report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SimMessage {
    Connected {
        sim: SimKind,
        app_name: String,
        app_version: String,
    },
    Aircraft(AircraftInfo),
    Telemetry(Telemetry),
    /// The sim closed or the link dropped.
    Disconnected { reason: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("simulator not available: {0}")]
    NotAvailable(String),
    #[error("simulator library not found: {0}")]
    LibraryMissing(String),
    #[error("connector failure: {0}")]
    Failure(String),
}

/// One simulator link. Implementations are polled from a dedicated thread,
/// roughly every 20 ms, and return whatever arrived since the last poll.
pub trait SimConnector: Send {
    fn sim(&self) -> SimKind;
    fn poll(&mut self) -> Result<Vec<SimMessage>, ConnectorError>;
}

/// A position along a flight's track, as stored and drawn on the map.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct TrackPoint {
    pub t_ms: i64,
    pub lat: f64,
    pub lon: f64,
    pub alt_ft: f32,
    pub gs_kt: f32,
    pub vs_fpm: f32,
    pub hdg_deg: f32,
    pub on_ground: bool,
}

impl TrackPoint {
    pub fn from_telemetry(s: &Telemetry) -> Self {
        TrackPoint {
            t_ms: s.t_ms,
            lat: s.lat,
            lon: s.lon,
            alt_ft: s.alt_msl_ft as f32,
            gs_kt: s.gs_kt as f32,
            vs_fpm: s.vs_fpm as f32,
            hdg_deg: s.hdg_true_deg as f32,
            on_ground: s.on_ground,
        }
    }
}
