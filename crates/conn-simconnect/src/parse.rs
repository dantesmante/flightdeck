//! Decoding SimConnect messages from raw bytes.
//!
//! Layouts follow SimConnect.h. Every struct starts with the 12-byte
//! SIMCONNECT_RECV header {dwSize, dwVersion, dwID}.

use telemetry::{units, AircraftInfo, EngineType, NavAid, NavBatch, NavKind, SimKind, Telemetry};

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
    // The sim's own clock (UTC), for logbook day and night by sim time.
    v("ZULU TIME", "seconds"),
    v("ZULU DAY OF YEAR", "number"),
    v("ZULU YEAR", "number"),
];

/// Instruments for the PFD. Each its own definition and request (id
/// `INSTR_BASE + index`), every frame but only when the value changes, so a
/// variable an aircraft lacks loses only that one reading.
pub const INSTR_BASE: u32 = 40;
pub const INSTR_VARS: &[Var] = &[
    v("INDICATED ALTITUDE", "feet"),
    v("PLANE HEADING DEGREES MAGNETIC", "degrees"),
    v("AIRSPEED TRUE", "knots"),
    v("GPS GROUND TRUE TRACK", "degrees"),
    v("TURN COORDINATOR BALL", "position 128"),
    v("KOHLSMAN SETTING HG", "inHg"),
    v("AUTOPILOT ALTITUDE LOCK VAR", "feet"),
    v("AUTOPILOT HEADING LOCK DIR", "degrees"),
    v("AUTOPILOT AIRSPEED HOLD VAR", "knots"),
    v("AUTOPILOT VERTICAL HOLD VAR", "feet per minute"),
    v("AUTOPILOT MASTER", "bool"),
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
    NavAids(NavBatch),
    Other,
}

/// Request ids for facility lists: `NAV_WORLD` + kind for the whole world,
/// `NAV_NEAR` + kind for those around the aircraft (kind 0 VOR, 1 NDB, 2
/// waypoint).
pub const NAV_WORLD: u32 = 60;
pub const NAV_NEAR: u32 = 70;
pub const NAV_KINDS: [NavKind; 3] = [NavKind::Vor, NavKind::Ndb, NavKind::Waypoint];

/// SIMCONNECT_RECV_FACILITIES_LIST: header + request id, array size, entry
/// number, out of; then the entries.
const LIST_OFFSET: usize = HEADER + 4 * 4;

/// Bytes after the ident in each entry (pack 1): region[3], lat, lon, alt;
/// waypoints add magvar (f32); NDBs a frequency (u32); VORs flags (u32),
/// localizer course (f32), glide slope lat/lon/alt and angle (f32).
fn tail_len(kind: NavKind) -> usize {
    match kind {
        NavKind::Waypoint => 3 + 24 + 4,
        NavKind::Ndb => 3 + 24 + 4 + 4,
        NavKind::Vor => 3 + 24 + 4 + 4 + 4 + 4 + 24 + 4,
    }
}

/// The ident's length in this list's entries: 6 (MSFS 2020's layout) or 9
/// (MSFS 2024's). Taken from the entry size when it matches the known
/// layout, otherwise from whichever length reads as real places (the
/// layout of a list can differ between sim versions; waypoints did).
fn ident_len(b: &[u8], n: usize, stride: usize, kind: NavKind) -> Option<usize> {
    let known = stride.checked_sub(tail_len(kind)).filter(|il| [6, 9].contains(il));
    let reads = |il: usize| {
        if stride < il + 3 + 24 {
            return false;
        }
        (0..n.min(5)).all(|i| {
            let o = LIST_OFFSET + i * stride;
            let ok_str = |at: usize, len: usize| b.get(at..at + len).is_some_and(|s| s.iter().all(|&c| c == 0 || c.is_ascii_alphanumeric() || c == b' '));
            let (Some(lat), Some(lon), Some(alt)) = (f64_at(b, o + il + 3), f64_at(b, o + il + 11), f64_at(b, o + il + 19)) else { return false };
            ok_str(o, il) && ok_str(o + il, 3) && lat.abs() <= 90.0 && lon.abs() <= 180.0 && alt.is_finite() && alt.abs() < 20_000.0 && (lat != 0.0 || lon != 0.0)
        })
    };
    known.filter(|&il| reads(il)).or_else(|| [9, 6].into_iter().find(|&il| reads(il)))
}

/// A VOR, NDB or waypoint list. The ident is 6 bytes long in MSFS 2020's
/// layout and 9 in MSFS 2024's, so the entry size tells which.
fn parse_facilities(b: &[u8], kind: NavKind, world: bool) -> Option<NavBatch> {
    let n = u32_at(b, HEADER + 4)? as usize;
    let entry = u32_at(b, HEADER + 8)?;
    let out_of = u32_at(b, HEADER + 12)?;
    let last = entry + 1 >= out_of;
    if n == 0 {
        return Some(NavBatch { kind, items: Vec::new(), last, world });
    }
    let stride = (b.len().checked_sub(LIST_OFFSET)?) / n;
    let Some(il) = ident_len(b, n, stride, kind) else {
        log::warn!("MSFS {kind:?} list not understood: {n} entries in {} bytes ({stride} each)", b.len());
        return None;
    };
    let mut items = Vec::with_capacity(n);
    for i in 0..n {
        let o = LIST_OFFSET + i * stride;
        let ident = str_at(b, o, il)?;
        let region = str_at(b, o + il, 3)?;
        let lat = f64_at(b, o + il + 3)?;
        let lon = f64_at(b, o + il + 11)?;
        if ident.is_empty() || !lat.is_finite() || !lon.is_finite() || lat.abs() > 90.0 || lon.abs() > 180.0 {
            continue;
        }
        let (mut freq_hz, mut dme, mut dme_only) = (None, false, false);
        if kind != NavKind::Waypoint && stride >= il + 35 {
            freq_hz = u32_at(b, o + il + 31).filter(|&f| f > 0);
        }
        if kind == NavKind::Vor && stride >= il + 39 {
            let flags = u32_at(b, o + il + 35).unwrap_or(0);
            // ILS localizers come in the VOR list too: not VORs.
            if flags & 0x2 != 0 {
                continue;
            }
            dme = flags & 0x8 != 0;
            dme_only = dme && flags & 0x1 == 0;
        }
        items.push(NavAid { kind, ident, region, lat, lon, freq_hz, dme, dme_only });
    }
    Some(NavBatch { kind, items, last, world })
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
    zulu_s: Option<f64>,
    zulu_doy: Option<f64>,
    zulu_year: Option<f64>,
    instr: telemetry::Instruments,
    have_instr: bool,
}

impl State {
    /// The instruments read so far, once any have come.
    pub fn instruments(&self) -> Option<telemetry::Instruments> {
        self.have_instr.then_some(self.instr)
    }
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
        crate::ffi::RECV_ID_VOR_LIST | crate::ffi::RECV_ID_NDB_LIST | crate::ffi::RECV_ID_WAYPOINT_LIST => {
            let req = u32_at(b, HEADER).unwrap_or(0);
            let (world, k) = if (NAV_WORLD..NAV_WORLD + 3).contains(&req) {
                (true, req - NAV_WORLD)
            } else if (NAV_NEAR..NAV_NEAR + 3).contains(&req) {
                (false, req - NAV_NEAR)
            } else {
                return Parsed::Other;
            };
            parse_facilities(b, NAV_KINDS[k as usize], world).map_or(Parsed::Other, Parsed::NavAids)
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
                            2 => st.camera = Some(x.clamp(0.0, 255.0) as u8),
                            3 => st.zulu_s = Some(x),
                            4 => st.zulu_doy = Some(x),
                            _ => st.zulu_year = Some(x),
                        }
                    }
                    Parsed::Other
                }
                r if (INSTR_BASE..INSTR_BASE + INSTR_VARS.len() as u32).contains(&r) => {
                    if let Some(x) = f64_at(b, SIMOBJECT_DATA_OFFSET).filter(|x| x.is_finite()) {
                        let i = &mut st.instr;
                        match r - INSTR_BASE {
                            0 => i.ind_alt_ft = Some(x),
                            1 => i.hdg_mag_deg = Some(x.rem_euclid(360.0)),
                            2 => i.tas_kt = Some(x),
                            3 => i.track_true_deg = Some(x.rem_euclid(360.0)),
                            4 => i.ball = Some((x / 127.0).clamp(-1.0, 1.0)),
                            5 => i.baro_inhg = Some(x),
                            6 => i.sel_alt_ft = Some(x),
                            7 => i.sel_hdg_deg = Some(x.rem_euclid(360.0)),
                            8 => i.sel_spd_kt = Some(x),
                            9 => i.sel_vs_fpm = Some(x),
                            _ => i.ap_on = Some(x != 0.0),
                        }
                        st.have_instr = true;
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
        sim_clock_offset_ms: sim_clock_ms(st).map(|sim| sim - now_ms),
    })
}

/// The sim's clock as Unix milliseconds (UTC), once it has sent it.
fn sim_clock_ms(st: &State) -> Option<i64> {
    let (s, doy, y) = (st.zulu_s?, st.zulu_doy?, st.zulu_year?);
    if !(1971.0..=2200.0).contains(&y) || !(1.0..=366.0).contains(&doy) || !(0.0..86_401.0).contains(&s) {
        return None;
    }
    // Days from 1970-01-01 to 1 January of year y.
    let y = y as i64;
    let days = (1970..y).map(|yr| if (yr % 4 == 0 && yr % 100 != 0) || yr % 400 == 0 { 366 } else { 365 }).sum::<i64>();
    Some((days + doy as i64 - 1) * 86_400_000 + (s * 1000.0) as i64)
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

    /// A facilities list message with entries laid out with an ident of `il` bytes.
    fn list_msg(id: u32, req: u32, il: usize, kind: NavKind, entries: &[(&str, &str, f64, f64, u32, u32)], entry: u32, out_of: u32) -> Vec<u8> {
        list_msg_extra(id, req, il, kind, entries, entry, out_of, 0)
    }

    #[allow(clippy::too_many_arguments)]
    fn list_msg_extra(id: u32, req: u32, il: usize, kind: NavKind, entries: &[(&str, &str, f64, f64, u32, u32)], entry: u32, out_of: u32, extra: usize) -> Vec<u8> {
        let stride = il + tail_len(kind) + extra;
        let mut b = header(id, LIST_OFFSET + stride * entries.len());
        b[HEADER..HEADER + 4].copy_from_slice(&req.to_le_bytes());
        b[HEADER + 4..HEADER + 8].copy_from_slice(&(entries.len() as u32).to_le_bytes());
        b[HEADER + 8..HEADER + 12].copy_from_slice(&entry.to_le_bytes());
        b[HEADER + 12..HEADER + 16].copy_from_slice(&out_of.to_le_bytes());
        for (i, (ident, region, lat, lon, freq, flags)) in entries.iter().enumerate() {
            let o = LIST_OFFSET + i * stride;
            b[o..o + ident.len()].copy_from_slice(ident.as_bytes());
            b[o + il..o + il + region.len()].copy_from_slice(region.as_bytes());
            b[o + il + 3..o + il + 11].copy_from_slice(&lat.to_le_bytes());
            b[o + il + 11..o + il + 19].copy_from_slice(&lon.to_le_bytes());
            if kind != NavKind::Waypoint {
                b[o + il + 31..o + il + 35].copy_from_slice(&freq.to_le_bytes());
            }
            if kind == NavKind::Vor {
                b[o + il + 35..o + il + 39].copy_from_slice(&flags.to_le_bytes());
            }
        }
        b
    }

    #[test]
    fn vor_lists_in_both_layouts() {
        let mut st = State::default();
        for il in [6, 9] {
            let m = list_msg(
                crate::ffi::RECV_ID_VOR_LIST,
                NAV_WORLD,
                il,
                NavKind::Vor,
                &[("SEA", "K1", 47.435, -122.31, 116_800_000, 0x9), ("ISEA", "K1", 47.43, -122.3, 110_300_000, 0x7)],
                0,
                2,
            );
            let Parsed::NavAids(batch) = parse(&m, &mut st, 0) else { panic!("not a nav list") };
            assert!(batch.world && !batch.last);
            assert_eq!(batch.items.len(), 1, "the localizer is left out");
            let v = &batch.items[0];
            assert_eq!((v.ident.as_str(), v.region.as_str(), v.freq_hz, v.dme), ("SEA", "K1", Some(116_800_000), true));
            assert!((v.lat - 47.435).abs() < 1e-9 && (v.lon + 122.31).abs() < 1e-9);
        }
    }

    #[test]
    fn ndb_and_waypoint_lists() {
        let mut st = State::default();
        let m = list_msg(crate::ffi::RECV_ID_NDB_LIST, NAV_NEAR + 1, 9, NavKind::Ndb, &[("OS", "K1", 47.0, -122.0, 350_000, 0)], 1, 2);
        let Parsed::NavAids(b) = parse(&m, &mut st, 0) else { panic!() };
        assert!(!b.world && b.last && b.kind == NavKind::Ndb && b.items[0].freq_hz == Some(350_000));
        let m = list_msg(crate::ffi::RECV_ID_WAYPOINT_LIST, NAV_WORLD + 2, 6, NavKind::Waypoint, &[("HAMUR", "K1", 46.0, -121.0, 0, 0), ("", "K1", 1.0, 1.0, 0, 0)], 0, 1);
        let Parsed::NavAids(b) = parse(&m, &mut st, 0) else { panic!() };
        assert_eq!(b.items.len(), 1);
        assert_eq!(b.items[0].ident, "HAMUR");
        assert_eq!(b.items[0].freq_hz, None);
    }

    #[test]
    fn waypoints_in_an_unexpected_layout_still_read() {
        // Entries longer than the documented layout (another sim version):
        // the ident length is found from where the places make sense.
        let mut st = State::default();
        let pts = [("HITBA", "K2", 39.02, -122.95, 0, 0), ("LAPED", "K2", 39.08, -122.79, 0, 0), ("ZOGNA", "K2", 39.05, -122.85, 0, 0)];
        for (il, extra) in [(9, 8), (6, 12), (9, 0)] {
            let m = list_msg_extra(crate::ffi::RECV_ID_WAYPOINT_LIST, NAV_WORLD + 2, il, NavKind::Waypoint, &pts, 0, 1, extra);
            let Parsed::NavAids(b) = parse(&m, &mut st, 0) else { panic!("{il} {extra}") };
            assert_eq!(b.items.len(), 3, "{il} {extra}");
            assert_eq!(b.items[1].ident, "LAPED");
            assert!((b.items[1].lon + 122.79).abs() < 1e-9);
        }
    }

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
        // The sim's clock: 2026, day 315 (11 Nov), 23:00 UTC.
        parse(&data_msg(POWER_BASE + 3, &82_800.0f64.to_le_bytes()), &mut st, 0);
        parse(&data_msg(POWER_BASE + 4, &315.0f64.to_le_bytes()), &mut st, 0);
        assert_eq!(t(&mut st).sim_clock_offset_ms, None);
        parse(&data_msg(POWER_BASE + 5, &2026.0f64.to_le_bytes()), &mut st, 0);
        assert_eq!(t(&mut st).sim_clock_offset_ms, Some(1_794_438_000_000));
    }

    #[test]
    fn instruments_arrive_one_by_one() {
        let mut st = State::default();
        assert_eq!(st.instruments(), None);
        parse(&data_msg(INSTR_BASE + 1, &(-10.0f64).to_le_bytes()), &mut st, 0);
        parse(&data_msg(INSTR_BASE + 4, &(63.5f64).to_le_bytes()), &mut st, 0);
        parse(&data_msg(INSTR_BASE + 10, &1.0f64.to_le_bytes()), &mut st, 0);
        let i = st.instruments().unwrap();
        assert_eq!(i.hdg_mag_deg, Some(350.0));
        assert_eq!(i.ball, Some(0.5));
        assert_eq!(i.ap_on, Some(true));
        assert_eq!(i.sel_alt_ft, None);
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
