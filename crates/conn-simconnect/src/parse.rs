//! Decoding SimConnect messages from raw bytes.
//!
//! Layouts follow SimConnect.h. Every struct starts with the 12-byte
//! SIMCONNECT_RECV header {dwSize, dwVersion, dwID}.

use telemetry::{units, AircraftInfo, EngineType, SimKind, Telemetry};

use crate::{EVENT_AIRCRAFT_LOADED, EVENT_CRASHED, EVENT_PAUSE, EVENT_SIM, EVENT_SIM_START, REQ_SIM_STATE};

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

/// Electrical power and camera, each its own definition and request (id
/// `POWER_BASE + index`, once a second), so a variable a sim or aircraft
/// rejects never stops position tracking.
pub const POWER_BASE: u32 = 30;
pub const POWER_VARS: &[Var] = &[
    v("ELECTRICAL MASTER BATTERY", "bool"),
    v("ELECTRICAL MAIN BUS VOLTAGE", "volts"),
    v("CAMERA STATE", "number"),
];

/// Aircraft details. Each one is its own definition and request (id
/// `AIRCRAFT_BASE + index`), so a variable a sim version rejects only loses
/// that one field. Empty units = STRING256.
pub const AIRCRAFT_BASE: u32 = 10;
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
    /// The sim entered (true) or left (false) a flight, e.g. back to the menus.
    SimRunning(bool),
    Quit,
    Exception { code: u32, send_id: u32, index: u32 },
    Other,
}

/// What the parser remembers between messages.
#[derive(Debug, Default)]
pub struct State {
    paused: bool,
    /// From the "Sim" state: `Some(false)` while in menus or loading.
    pub in_flight: Option<bool>,
    last_touchdown_fps: Option<f64>,
    aircraft: AircraftInfo,
    atc_type: String,
    battery: Option<bool>,
    bus_volts: Option<f64>,
    camera: Option<u8>,
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

/// Some aircraft report ATC MODEL as a localization key such as
/// "TT:ATCCOM.AC_MODEL_C172.0.text", "ATCCOM.AC_MODEL A320.0.text" or
/// "$$:ATCCOM.AC_MODEL_B738.0.text"; pull the type designator out of it.
/// Anything else that is not a plausible type comes back empty.
pub fn clean_atc_model(s: &str) -> String {
    let s = s.trim();
    if let Some(i) = s.find("AC_MODEL") {
        let rest = s[i + "AC_MODEL".len()..].trim_start_matches(['_', ' ']);
        let code = rest.split('.').next().unwrap_or("").trim().to_uppercase();
        return if is_type_code(&code) { code } else { String::new() };
    }
    if s.contains("ATCCOM") || s.contains(':') || s.contains('.') {
        return String::new();
    }
    s.to_string()
}

/// ICAO type designators are 2 to 4 letters and digits, starting with a letter.
fn is_type_code(s: &str) -> bool {
    (2..=4).contains(&s.len())
        && s.chars().all(|c| c.is_ascii_alphanumeric())
        && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
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
        crate::ffi::RECV_ID_EXCEPTION => Parsed::Exception {
            code: u32_at(b, HEADER).unwrap_or(0),
            send_id: u32_at(b, HEADER + 4).unwrap_or(0),
            index: u32_at(b, HEADER + 8).unwrap_or(0),
        },
        crate::ffi::RECV_ID_SYSTEM_STATE => {
            if u32_at(b, HEADER) == Some(REQ_SIM_STATE) {
                let running = u32_at(b, HEADER + 4).unwrap_or(1) != 0;
                st.in_flight = Some(running);
                Parsed::SimRunning(running)
            } else {
                Parsed::Other
            }
        }
        crate::ffi::RECV_ID_EVENT => {
            let event = u32_at(b, HEADER + 4).unwrap_or(0);
            let data = u32_at(b, HEADER + 8).unwrap_or(0);
            match event {
                EVENT_PAUSE => {
                    st.paused = data != 0;
                    Parsed::Other
                }
                EVENT_SIM => {
                    st.in_flight = Some(data != 0);
                    Parsed::SimRunning(data != 0)
                }
                EVENT_AIRCRAFT_LOADED | EVENT_SIM_START => Parsed::AircraftReloaded,
                EVENT_CRASHED => Parsed::Other,
                _ => Parsed::Other,
            }
        }
        crate::ffi::RECV_ID_SIMOBJECT_DATA => {
            let req = u32_at(b, HEADER).unwrap_or(0);
            match req {
                1 => parse_telemetry(b, st, now_ms).map_or(Parsed::Other, Parsed::Telemetry),
                r if (POWER_BASE..POWER_BASE + POWER_VARS.len() as u32).contains(&r) => {
                    if let Some(x) = f64_at(b, SIMOBJECT_DATA_OFFSET) {
                        match r - POWER_BASE {
                            0 => st.battery = Some(x != 0.0),
                            1 => st.bus_volts = Some(x),
                            _ => st.camera = Some(x.clamp(0.0, 255.0) as u8),
                        }
                    }
                    Parsed::Other
                }
                r if (AIRCRAFT_BASE..AIRCRAFT_BASE + AIRCRAFT_VARS.len() as u32).contains(&r) => {
                    parse_aircraft_field(b, st, (r - AIRCRAFT_BASE) as usize).map_or(Parsed::Other, Parsed::Aircraft)
                }
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
        // Outside a flight (menus, loading) positions are meaningless: report
        // them as paused so the tracker ignores them. Also guard 0,0.
        paused: st.paused || st.in_flight == Some(false) || (vals[0].abs() < 0.01 && vals[1].abs() < 0.01),
        sim_rate: vals[18],
        wind_dir_deg: vals[19],
        wind_kt: vals[20],
        touchdown_vs_fpm,
        powered: match (st.battery, st.bus_volts) {
            (None, None) => None,
            (b, v) => Some(b == Some(true) || v.is_some_and(|v| v > 8.0)),
        },
        camera_state: st.camera,
    })
}

/// One aircraft field arrived; merge it and return the updated aircraft.
fn parse_aircraft_field(b: &[u8], st: &mut State, index: usize) -> Option<AircraftInfo> {
    let o = SIMOBJECT_DATA_OFFSET;
    let a = &mut st.aircraft;
    match AIRCRAFT_VARS.get(index)?.name {
        "TITLE" => a.title = str_at(b, o, 256)?,
        "ATC MODEL" => a.icao_type = clean_atc_model(&str_at(b, o, 256)?),
        "ATC TYPE" => st.atc_type = str_at(b, o, 256)?,
        "ATC ID" => a.registration = str_at(b, o, 256)?,
        "ENGINE TYPE" => a.engine_type = engine_type(f64_at(b, o)?),
        "NUMBER OF ENGINES" => a.engine_count = f64_at(b, o)?.clamp(0.0, 8.0) as u8,
        _ => return None,
    }
    let mut out = st.aircraft.clone();
    if out.title.is_empty() {
        out.title = st.atc_type.clone();
    }
    Some(out)
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

    fn string_field(s: &[u8]) -> Vec<u8> {
        let mut p = vec![0u8; 256];
        p[..s.len()].copy_from_slice(s);
        p
    }

    fn field_req(name: &str) -> u32 {
        AIRCRAFT_BASE + AIRCRAFT_VARS.iter().position(|v| v.name == name).unwrap() as u32
    }

    #[test]
    fn aircraft_fields_merge() {
        let mut st = State::default();
        parse(&data_msg(field_req("TITLE"), &string_field(b"Cessna Skyhawk G1000")), &mut st, 0);
        parse(&data_msg(field_req("ATC MODEL"), &string_field(b"TT:ATCCOM.AC_MODEL_C172.0.text")), &mut st, 0);
        parse(&data_msg(field_req("ENGINE TYPE"), &0.0f64.to_le_bytes()), &mut st, 0);
        parse(&data_msg(field_req("NUMBER OF ENGINES"), &1.0f64.to_le_bytes()), &mut st, 0);
        let Parsed::Aircraft(a) = parse(&data_msg(field_req("ATC ID"), &string_field(b"N172SP")), &mut st, 0) else {
            panic!()
        };
        assert_eq!(a.title, "Cessna Skyhawk G1000");
        assert_eq!(a.icao_type, "C172");
        assert_eq!(a.registration, "N172SP");
        assert_eq!((a.engine_count, a.engine_type), (1, EngineType::Piston));
    }

    #[test]
    fn atc_model_keys_become_type_codes() {
        assert_eq!(clean_atc_model("TT:ATCCOM.AC_MODEL_C172.0.text"), "C172");
        assert_eq!(clean_atc_model("ATCCOM.AC_MODEL_A320.0.text"), "A320");
        assert_eq!(clean_atc_model("$$:ATCCOM.AC_MODEL B738.0.text"), "B738");
        assert_eq!(clean_atc_model("ATCCOM.AC_MODEL_FENIX_A320.0.text"), "");
        assert_eq!(clean_atc_model("A20N"), "A20N");
        assert_eq!(clean_atc_model("ATCCOM.SOMETHING"), "");
    }

    #[test]
    fn power_from_battery_or_bus() {
        let mut st = State::default();
        let t = |st: &mut State| match parse(&data_msg(1, &telemetry_payload(sample())), st, 0) {
            Parsed::Telemetry(t) => t,
            _ => panic!(),
        };
        assert_eq!(t(&mut st).powered, None, "unknown until the sim says");
        parse(&data_msg(POWER_BASE, &0.0f64.to_le_bytes()), &mut st, 0);
        parse(&data_msg(POWER_BASE + 1, &0.0f64.to_le_bytes()), &mut st, 0);
        assert_eq!(t(&mut st).powered, Some(false));
        // Battery switch reads off (custom systems) but the bus is live.
        parse(&data_msg(POWER_BASE + 1, &28.0f64.to_le_bytes()), &mut st, 0);
        assert_eq!(t(&mut st).powered, Some(true));
        parse(&data_msg(POWER_BASE + 2, &2.0f64.to_le_bytes()), &mut st, 0);
        assert_eq!(t(&mut st).camera_state, Some(2));
    }

    #[test]
    fn missing_title_falls_back_to_atc_type() {
        let mut st = State::default();
        let Parsed::Aircraft(a) = parse(&data_msg(field_req("ATC TYPE"), &string_field(b"Cessna")), &mut st, 0) else {
            panic!()
        };
        assert_eq!(a.title, "Cessna");
    }

    fn sim_event(running: u32) -> Vec<u8> {
        let mut ev = header(crate::ffi::RECV_ID_EVENT, HEADER + 12);
        ev[HEADER + 4..HEADER + 8].copy_from_slice(&EVENT_SIM.to_le_bytes());
        ev[HEADER + 8..HEADER + 12].copy_from_slice(&running.to_le_bytes());
        ev
    }

    #[test]
    fn menus_count_as_paused() {
        let mut st = State::default();
        assert_eq!(parse(&sim_event(0), &mut st, 0), Parsed::SimRunning(false));
        let Parsed::Telemetry(t) = parse(&data_msg(1, &telemetry_payload(sample())), &mut st, 0) else { panic!() };
        assert!(t.paused);
        parse(&sim_event(1), &mut st, 0);
        let Parsed::Telemetry(t) = parse(&data_msg(1, &telemetry_payload(sample())), &mut st, 0) else { panic!() };
        assert!(!t.paused);
    }

    #[test]
    fn system_state_reply_sets_flight_state() {
        let mut st = State::default();
        let mut b = header(crate::ffi::RECV_ID_SYSTEM_STATE, HEADER + 12 + 260);
        b[HEADER..HEADER + 4].copy_from_slice(&REQ_SIM_STATE.to_le_bytes());
        b[HEADER + 4..HEADER + 8].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(parse(&b, &mut st, 0), Parsed::SimRunning(false));
        assert_eq!(st.in_flight, Some(false));
    }

    #[test]
    fn null_island_is_ignored() {
        let mut v = sample();
        v[0] = 0.0;
        v[1] = 0.0;
        let Parsed::Telemetry(t) = parse(&data_msg(1, &telemetry_payload(v)), &mut State::default(), 0) else {
            panic!()
        };
        assert!(t.paused);
    }

    #[test]
    fn exception_details() {
        let mut b = header(crate::ffi::RECV_ID_EXCEPTION, HEADER + 12);
        b[HEADER..HEADER + 4].copy_from_slice(&7u32.to_le_bytes());
        b[HEADER + 4..HEADER + 8].copy_from_slice(&42u32.to_le_bytes());
        b[HEADER + 8..HEADER + 12].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(parse(&b, &mut State::default(), 0), Parsed::Exception { code: 7, send_id: 42, index: 3 });
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
