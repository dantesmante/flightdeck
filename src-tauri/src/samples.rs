//! Sample flights: fly the built-in demo routes instantly and save them, so
//! the passport, logbook and hangar can be tried without hours of flying.
//! They are saved as sim "demo" and can be removed in one go.

use std::sync::Mutex;

use conn_demo::{sample_routes, DemoFlight, DemoOptions};
use store::Store;
use telemetry::{AircraftInfo, EngineType, SimKind};
use tracker::{Config, Tracker, TrackerOutput};

struct Sample {
    route: usize,
    days_ago: i64,
    /// Departure time of day, UTC hours.
    utc_hour: f64,
    touchdown_fpm: f64,
    aircraft: (&'static str, &'static str, &'static str, u8, EngineType),
}

// Oldest first, so the passport reads as a trip: US regions, then Canada to
// the US, then Europe. The London flight leaves after dark.
const SAMPLES: &[Sample] = &[
    Sample { route: 0, days_ago: 12, utc_hour: 17.0, touchdown_fpm: -95.0, aircraft: ("Cessna 208 Caravan", "C208", "N208FD", 1, EngineType::Turboprop) },
    Sample { route: 2, days_ago: 9, utc_hour: 20.0, touchdown_fpm: -180.0, aircraft: ("Boeing 737-800", "B738", "N738FD", 2, EngineType::Jet) },
    Sample { route: 4, days_ago: 6, utc_hour: 22.0, touchdown_fpm: -55.0, aircraft: ("Cessna 172 Skyhawk", "C172", "C-GFDK", 1, EngineType::Piston) },
    Sample { route: 1, days_ago: 4, utc_hour: 20.5, touchdown_fpm: -140.0, aircraft: ("Airbus A320neo", "A20N", "G-FDKA", 2, EngineType::Jet) },
    Sample { route: 3, days_ago: 2, utc_hour: 8.0, touchdown_fpm: -260.0, aircraft: ("De Havilland Dash 8-400", "DH8D", "OE-FDQ", 2, EngineType::Turboprop) },
];

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Fly and save every sample flight. Returns how many were saved.
pub fn add(db: &Mutex<Store>, refdata: &refdata::RefData) -> usize {
    let routes = sample_routes();
    let today = now_ms() / 86_400_000 * 86_400_000;
    let mut saved = 0;
    for s in SAMPLES {
        let start = today - s.days_ago * 86_400_000 + (s.utc_hour * 3_600_000.0) as i64;
        let opts = DemoOptions { touchdown_fpm: s.touchdown_fpm, ..Default::default() };
        let (title, icao, reg, engines, kind) = s.aircraft;
        let aircraft = AircraftInfo {
            title: title.into(),
            icao_type: icao.into(),
            registration: reg.into(),
            engine_count: engines,
            engine_type: kind,
        };
        let mut tr = Tracker::new(SimKind::Demo, aircraft, Config::default());
        let summary = DemoFlight::new(routes[s.route].clone(), opts, start)
            .run_to_end()
            .iter()
            .flat_map(|t| tr.update(t))
            .find_map(|o| match o {
                TrackerOutput::Completed(f) => Some(*f),
                _ => None,
            });
        if let Some(f) = summary {
            if crate::engine::save(db, refdata, &f, None, None).is_some() {
                saved += 1;
            }
        }
    }
    saved
}

pub fn remove(db: &Mutex<Store>) -> usize {
    db.lock().unwrap().delete_flights_from_sim(SimKind::Demo.as_str()).unwrap_or(0)
}
