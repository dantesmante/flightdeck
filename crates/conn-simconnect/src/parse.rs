//! Decoding SimConnect messages from raw bytes.
//!
//! Layouts follow SimConnect.h. Every struct starts with the 12-byte
//! SIMCONNECT_RECV header {dwSize, dwVersion, dwID}.

use telemetry::{units, AircraftInfo, EngineType, SimKind, Telemetry};

use crate::{EVENT_AIRCRAFT_LOADED, EVENT_CRASHED, EVENT_PAUSE, EVENT_SIM};

pub struct Var {
    pub name: &'static str,
    /// Empty = string (STRING256), otherwise FLOAT64 in these units.
    pub units: &'static str,
}

const fn v(name: &'static str, units: &'static str) -> Var {
    Var { name, units }
}

/// Telemetry definition, all FLOAT64, read back in this order.
pub const TELEMETRY_VARS: &[Var] = &[
    v("PLANE LATITUDE", "degrees"),
    v("PLANE LONGITUDE", "degrees"),
    v("PLANE ALTITUDE", "feet"),
    v("PLANE ALT ABOVE GROUND", "feet"),
    v("GROUND VELOCITY", "knots"),
    v("AIRSPEED INDICATED", "knots"),
    v("PLANE HEADING DEGREES TRUE", "degrees"),
    v("VERTICAL SPEED", "feet per minute"),
    v("PLANE PITCH DEGREES", "degrees"),
    v("PLANE BANK DEGREES", "degrees"),
    v("G FORCE", "gforce"),
    v("SIM ON GROUND", "bool"),
    v("BRAKE PARKING POSITION", "bool"),
    v("GENERAL ENG COMBUSTION:1", "bool"),
    v("GENERAL ENG COMBUSTION:2", "bool"),
    v("GENERAL ENG COMBUSTION:3", "bool"),
    v("GENERAL ENG COMBUSTION:4", "bool"),
    v("FUEL TOTAL QUANTITY WEIGHT", "pounds"),
    v("SIMULATION RATE", "number"),
    v("AMBIENT WIND DIRECTION", "degrees"),
    v("AMBIENT WIND VELOCITY", "knots"),
    v("PLANE TOUCHDOWN NORMAL VELOCITY", "feet per second"),
    v("NUMBER OF ENGINES", "number"),
];

/// Aircraft definition: four STRING256 then two FLOAT64.
pub const AIRCRAFT_VARS: &[Var] = &[
    v("TITLE", ""),
    v("ATC MODEL", ""),
    v("ATC TYPE", ""),
    v("ATC ID", ""),
    v("ENGINE TYPE", "enum"),
    v("NUMBER OF ENGINES", "number"),
];

const HEADER: usize = 12;
/// SIMCONNECT_RECV_SIMOBJECT_DATA: header + 7 DWORDs, then the data.
const SIMOBJECT_DATA_OFFSET: usize = HEADER + 7 * 4;

#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    Open { app_name: String, version_major: u32, version_minor: u32 },
    Telemetry(Telemetry),
    Aircraft(AircraftInfo),
    AircraftReloaded,
    Quit,
    Exception(u32),
    Other,
}

/// What the parser remembers between messages.
#[derive(Debug, Default)]
pub struct State {
    paused: bool,
    last_touchdown_fps: Option<f64>,
}

fn u32_at(b: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(off..off + 4)?.try_into().ok()?))
}

fn f64_at(b: &[u8], off: usize) -> Option<f64> {
    Some(f64::from_le_bytes(b.get(off..off + 8)?.try_into().ok()?))
}

fn str_at(b: &[u8], off: usize, len: usize) -> Option<String> {
    let raw = b.get(off..off + len)?;
    let end = raw.iter().position(|&c| c == 0).unwrap_or(len);
    Some(String::from_utf8_lossy(&raw[..end]).trim().to_string())
}

/// MSFS 2020 reports major version 11, MSFS 2024 reports 12 or later.
/// Unknown versions keep the watcher's hint.
pub fn sim_from_version(major: u32, hint: SimKind) -> SimKind {
    match major {
        11 => SimKind::Msfs2020,
        12.. => SimKind::Msfs2024,
        _ => hint,
    }
}

/// Some aircraft report ATC MODEL as a localisation key such as
/// "TT:ATCCOM.AC_MODEL_C172.0.text"; pull the type designator out of it.
pub fn clean_atc_model(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("TT:ATCCOM.AC_MODEL") {
        let rest = rest.trim_start_matches(['_', ' ']);
        return rest.split('.').next().unwrap_or("").trim().to_uppercase();
    }
    s.trim().to_string()
}

fn engine_type(code: f64) -> EngineType {
    match code as i64 {
        0 => EngineType::Piston,
        1 => EngineType::Jet,
        2 => EngineType::Glider, // "none"
        3 => EngineType::Helo,
        5 => EngineType::Turboprop,
        _ => EngineType::Unknown,
    }
}

pub fn parse(b: &[u8], st: &mut State, now_ms: i64) -> Parsed {
    let Some(id) = u32_at(b, 8) else { return Parsed::Other };
    match id {
        crate::ffi::RECV_ID_OPEN => Parsed::Open {
            app_name: str_at(b, HEADER, 256).unwrap_or_default(),
            version_major: u32_at(b, HEADER + 256).unwrap_or(0),
            version_minor: u32_at(b, HEADER + 260).unwrap_or(0),
        },
        crate::ffi::RECV_ID_QUIT => Parsed::Quit,
        crate::ffi::RECV_ID_EXCEPTION => Parsed::Exception(u32_at(b, HEADER).unwrap_or(0)),
        crate::ffi::RECV_ID_EVENT => {
            let event = u32_at(b, HEADER + 4).unwrap_or(0);
            let data = u32_at(b, HEADER + 8).unwrap_or(0);
            match event {
                EVENT_PAUSE => {
                    st.paused = data != 0;
                    Parsed::Other
                }
                EVENT_SIM => Parsed::Other,
                EVENT_AIRCRAFT_LOADED => Parsed::AircraftReloaded,
                EVENT_CRASHED => Parsed::Other,
                _ => Parsed::Other,
            }
        }
        crate::ffi::RECV_ID_SIMOBJECT_DATA => {
            let req = u32_at(b, HEADER).unwrap_or(0);
            match req {
                1 => parse_telemetry(b, st, now_ms).map_or(Parsed::Other, Parsed::Telemetry),
                2 => parse_aircraft(b).map_or(Parsed::Other, Parsed::Aircraft),
                _ => Parsed::Other,
            }
        }
        _ => Parsed::Other,
    }
}

fn parse_telemetry(b: &[u8], st: &mut State, now_ms: i64) -> Option<Telemetry> {
    let mut vals = [0.0f64; 23];
    for (i, v) in vals.iter_mut().enumerate() {
        *v = f64_at(b, SIMOBJECT_DATA_OFFSET + i * 8)?;
    }
    let engine_count = (vals[22] as usize).clamp(0, 4);
    let running = vals[13..13 + engine_count].iter().filter(|&&x| x != 0.0).count() as u8;

    // PLANE TOUCHDOWN NORMAL VELOCITY keeps the last touchdown's value; only
    // report it when it changes, so a stale value is never taken as new.
    let td = vals[21];
    let touchdown_vs_fpm = match st.last_touchdown_fps {
        Some(prev) if (prev - td).abs() < 1e-9 => None,
        _ if td == 0.0 => None,
        _ => Some(-td.abs() * units::FTPS_TO_FPM),
    };
    st.last_touchdown_fps = Some(td);

    Some(Telemetry {
        t_ms: now_ms,
        lat: vals[0],
        lon: vals[1],
        alt_msl_ft: vals[2],
        alt_agl_ft: vals[3],
        gs_kt: vals[4],
        ias_kt: vals[5],
        hdg_true_deg: vals[6],
        vs_fpm: vals[7],
        // MSFS reports pitch nose-down positive and bank left-wing-down
        // positive; flip both to FlightDeck's convention.
        // TODO(milestone 1 test flight): confirm both signs in MSFS 2024.
        pitch_deg: -vals[8],
        bank_deg: -vals[9],
        g_load: vals[10],
        on_ground: vals[11] != 0.0,
        parking_brake: vals[12] > 0.5,
        engines_running: running,
        fuel_kg: vals[17] * units::LB_TO_KG,
        paused: st.paused,
        sim_rate: vals[18],
        wind_dir_deg: vals[19],
        wind_kt: vals[20],
        touchdown_vs_fpm,
    })
}

fn parse_aircraft(b: &[u8]) -> Option<AircraftInfo> {
    let o = SIMOBJECT_DATA_OFFSET;
    let title = str_at(b, o, 256)?;
    let model = clean_atc_model(&str_at(b, o + 256, 256)?);
    let atc_type = str_at(b, o + 512, 256)?;
    let atc_id = str_at(b, o + 768, 256)?;
    let engine = f64_at(b, o + 1024)?;
    let engines = f64_at(b, o + 1032)?;
    Some(AircraftInfo {
        title: if title.is_empty() { atc_type } else { title },
        icao_type: model,
        registration: atc_id,
        engine_count: engines.clamp(0.0, 8.0) as u8,
        engine_type: engine_type(engine),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(id: u32, size: usize) -> Vec<u8> {
        let mut b = vec![0u8; size];
        b[0..4].copy_from_slice(&(size as u32).to_le_bytes());
        b[8..12].copy_from_slice(&id.to_le_bytes());
        b
    }

    fn data_msg(req: u32, payload: &[u8]) -> Vec<u8> {
        let mut b = header(crate::ffi::RECV_ID_SIMOBJECT_DATA, SIMOBJECT_DATA_OFFSET + payload.len());
        b[HEADER..HEADER + 4].copy_from_slice(&req.to_le_bytes());
        b[SIMOBJECT_DATA_OFFSET..].copy_from_slice(payload);
        b
    }

    fn telemetry_payload(vals: [f64; 23]) -> Vec<u8> {
        vals.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    fn sample() -> [f64; 23] {
        let mut v = [0.0; 23];
        v[0] = 47.449;
        v[1] = -122.309;
        v[2] = 433.0;
        v[4] = 12.0;
        v[8] = -3.0; // nose up 3 degrees in MSFS terms
        v[11] = 1.0;
        v[13] = 1.0;
        v[14] = 1.0;
        v[17] = 10_000.0;
        v[18] = 1.0;
        v[22] = 2.0;
        v
    }

    #[test]
    fn telemetry_fields_and_units() {
        let mut st = State::default();
        let Parsed::Telemetry(t) = parse(&data_msg(1, &telemetry_payload(sample())), &mut st, 42) else {
            panic!("expected telemetry")
        };
        assert_eq!(t.t_ms, 42);
        assert_eq!((t.lat, t.lon), (47.449, -122.309));
        assert!(t.on_ground);
        assert_eq!(t.engines_running, 2);
        assert_eq!(t.pitch_deg, 3.0);
        assert!((t.fuel_kg - 4535.9237).abs() < 1e-6);
        assert_eq!(t.touchdown_vs_fpm, None);
    }

    #[test]
    fn engines_beyond_the_count_are_ignored() {
        let mut v = sample();
        v[22] = 1.0;
        v[15] = 1.0; // junk on engine 3
        let Parsed::Telemetry(t) = parse(&data_msg(1, &telemetry_payload(v)), &mut State::default(), 0) else {
            panic!()
        };
        assert_eq!(t.engines_running, 1);
    }

    #[test]
    fn touchdown_rate_reported_only_when_it_changes() {
        let mut st = State::default();
        let mut v = sample();
        v[21] = 2.5; // stale value from an earlier landing
        assert!(matches!(parse(&data_msg(1, &telemetry_payload(v)), &mut st, 0), Parsed::Telemetry(t) if t.touchdown_vs_fpm == Some(-150.0)));
        assert!(matches!(parse(&data_msg(1, &telemetry_payload(v)), &mut st, 0), Parsed::Telemetry(t) if t.touchdown_vs_fpm.is_none()));
        v[21] = 3.0;
        assert!(matches!(parse(&data_msg(1, &telemetry_payload(v)), &mut st, 0), Parsed::Telemetry(t) if t.touchdown_vs_fpm == Some(-180.0)));
    }

    #[test]
    fn pause_event_sets_the_flag() {
        let mut st = State::default();
        let mut ev = header(crate::ffi::RECV_ID_EVENT, HEADER + 12);
        ev[HEADER + 4..HEADER + 8].copy_from_slice(&EVENT_PAUSE.to_le_bytes());
        ev[HEADER + 8..HEADER + 12].copy_from_slice(&1u32.to_le_bytes());
        parse(&ev, &mut st, 0);
        let Parsed::Telemetry(t) = parse(&data_msg(1, &telemetry_payload(sample())), &mut st, 0) else { panic!() };
        assert!(t.paused);
    }

    #[test]
    fn aircraft_strings() {
        let mut p = vec![0u8; 1040];
        p[..20].copy_from_slice(b"Cessna Skyhawk G1000");
        let model = b"TT:ATCCOM.AC_MODEL_C172.0.text";
        p[256..256 + model.len()].copy_from_slice(model);
        p[768..774].copy_from_slice(b"N172SP");
        p[1024..1032].copy_from_slice(&0.0f64.to_le_bytes());
        p[1032..1040].copy_from_slice(&1.0f64.to_le_bytes());
        let Parsed::Aircraft(a) = parse(&data_msg(2, &p), &mut State::default(), 0) else { panic!() };
        assert_eq!(a.title, "Cessna Skyhawk G1000");
        assert_eq!(a.icao_type, "C172");
        assert_eq!(a.registration, "N172SP");
        assert_eq!((a.engine_count, a.engine_type), (1, EngineType::Piston));
    }

    #[test]
    fn open_message_and_version() {
        let mut b = header(crate::ffi::RECV_ID_OPEN, HEADER + 256 + 8 * 4);
        b[HEADER..HEADER + 9].copy_from_slice(b"KittyHawk");
        b[HEADER + 256..HEADER + 260].copy_from_slice(&11u32.to_le_bytes());
        let p = parse(&b, &mut State::default(), 0);
        assert_eq!(p, Parsed::Open { app_name: "KittyHawk".into(), version_major: 11, version_minor: 0 });
        assert_eq!(sim_from_version(11, SimKind::Msfs2024), SimKind::Msfs2020);
        assert_eq!(sim_from_version(12, SimKind::Msfs2020), SimKind::Msfs2024);
        assert_eq!(sim_from_version(0, SimKind::Msfs2024), SimKind::Msfs2024);
    }

    #[test]
    fn short_or_unknown_messages_are_ignored() {
        assert_eq!(parse(&[1, 2, 3], &mut State::default(), 0), Parsed::Other);
        assert_eq!(parse(&data_msg(1, &[0u8; 16]), &mut State::default(), 0), Parsed::Other);
    }
}
