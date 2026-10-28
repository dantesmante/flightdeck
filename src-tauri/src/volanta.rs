//! Volanta flight files (one JSON per flight): import them as FlightDeck
//! flights, and export FlightDeck flights in the same format.
//!
//! An imported flight keeps its original file (compressed, in the store),
//! so exporting it again gives back exactly the file that came in. Flights
//! FlightDeck recorded itself are written in Volanta's structure from what
//! FlightDeck knows; values it does not record are null.

use serde_json::{json, Map, Value};
use store::{FlightRow, Imported, Place, SaveContext};
use telemetry::{AircraftInfo, EngineType, SimKind, TrackPoint};
use tracker::{FlightStatus, FlightSummary, LandingReport};

pub const SOURCE: &str = "volanta";

// --- time ---------------------------------------------------------------

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// "2023-11-24T16:42:09.958852", "…578Z" or "…+01:00" to Unix ms. Times
/// without a zone are UTC (Volanta writes them that way).
pub fn parse_time(s: &str) -> Option<i64> {
    let s = s.trim();
    let num = |a: usize, b: usize| s.get(a..b)?.parse::<i64>().ok();
    let (y, mo, d, h, mi, se) = (num(0, 4)?, num(5, 7)?, num(8, 10)?, num(11, 13)?, num(14, 16)?, num(17, 19)?);
    let mut rest = &s[19..];
    let mut ms = 0i64;
    if let Some(frac) = rest.strip_prefix('.') {
        let digits: String = frac.chars().take_while(|c| c.is_ascii_digit()).collect();
        rest = &frac[digits.len()..];
        let mut f = digits.chars().take(3).collect::<String>();
        while f.len() < 3 {
            f.push('0');
        }
        ms = f.parse().unwrap_or(0);
    }
    let offset_min = match rest.chars().next() {
        Some('+') | Some('-') if rest.len() >= 6 => {
            let sign = if rest.starts_with('-') { -1 } else { 1 };
            sign * (rest[1..3].parse::<i64>().ok()? * 60 + rest[4..6].parse::<i64>().ok()?)
        }
        _ => 0,
    };
    let secs = days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + se - offset_min * 60;
    Some(secs * 1000 + ms)
}

/// Unix ms to "2023-11-24T16:41:44.578" (with "Z" when `zulu`).
pub fn format_time(ms: i64, zulu: bool) -> String {
    let secs = ms.div_euclid(1000);
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    let t = secs.rem_euclid(86_400);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}{}",
        t / 3600,
        t % 3600 / 60,
        t % 60,
        ms.rem_euclid(1000),
        if zulu { "Z" } else { "" }
    )
}

// --- import ---------------------------------------------------------------

fn f(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64)
}
fn st(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}
fn t(v: &Value, k: &str) -> Option<i64> {
    v.get(k).and_then(Value::as_str).and_then(parse_time)
}

/// A Volanta flight, ready to save.
pub struct ImportedFlight {
    pub id: String,
    pub summary: FlightSummary,
    pub ctx: SaveContext,
    pub note: Option<String>,
}

/// Which simulator: Volanta files may name it; otherwise an MSFS package
/// folder before MSFS 2024 existed (19 Nov 2024) means MSFS 2020.
fn guess_sim(v: &Value, start_ms: i64) -> SimKind {
    for key in ["simulator", "simulatorName", "sim", "simulatorType"] {
        if let Some(s) = st(v, key).map(|s| s.to_lowercase()) {
            if s.contains("2024") {
                return SimKind::Msfs2024;
            }
            if s.contains("2020") || s.contains("microsoft") || s.contains("msfs") {
                return SimKind::Msfs2020;
            }
            if s.contains("x-plane") || s.contains("xplane") {
                return if s.contains("11") { SimKind::Xp11 } else { SimKind::Xp12 };
            }
        }
    }
    let msfs_folder = st(v, "aircraftDirectory").is_some();
    if msfs_folder && start_ms < parse_time("2024-11-19T00:00:00").unwrap_or(0) {
        return SimKind::Msfs2020;
    }
    SimKind::Other
}

fn place(refdata: &refdata::RefData, apt: Option<&Value>) -> Option<Place> {
    let a = apt.filter(|a| a.is_object())?;
    let icao = st(a, "icaoCode")?;
    match refdata.airport(&icao) {
        Some(r) => Some(Place { icao: r.ident.clone(), name: r.name.clone(), country: r.country.clone(), region: r.region.clone() }),
        // Not in FlightDeck's airport list: keep Volanta's name.
        None => Some(Place { icao, name: st(a, "name").unwrap_or_default(), country: String::new(), region: String::new() }),
    }
}

/// Head and cross wind from the wind direction and speed, for a heading.
fn wind_parts(hdg: f64, wind_from: f64, kt: f64) -> (f64, f64) {
    let a = (wind_from - hdg).to_radians();
    (kt * a.cos(), kt * a.sin())
}

pub fn parse(text: &str, refdata: &refdata::RefData) -> Result<ImportedFlight, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("not a JSON file ({e})"))?;
    let id = st(&v, "id").ok_or("not a Volanta flight (no id)")?;
    let positions = v.get("positions").and_then(Value::as_array).cloned().unwrap_or_default();
    if positions.is_empty() && v.get("origin").is_none() {
        return Err("not a Volanta flight (no positions or origin)".into());
    }

    let mut track: Vec<TrackPoint> = positions
        .iter()
        .filter_map(|p| {
            Some(TrackPoint {
                t_ms: t(p, "time")?,
                lat: f(p, "latitude")?,
                lon: f(p, "longitude")?,
                alt_ft: f(p, "altitude").unwrap_or(0.0) as f32,
                gs_kt: f(p, "groundSpeed").unwrap_or(0.0) as f32,
                vs_fpm: f(p, "verticalSpeed").unwrap_or(0.0) as f32,
                hdg_deg: f(p, "headingTrue").unwrap_or(0.0) as f32,
                on_ground: p.get("onGround").and_then(Value::as_bool).unwrap_or(false),
            })
        })
        .collect();
    track.sort_by_key(|p| p.t_ms);

    let block_out = t(&v, "offBlocksTime");
    let block_in = t(&v, "onBlocksTime");
    let created = t(&v, "createdAt");
    let start_ms = [block_out, track.first().map(|p| p.t_ms), created].into_iter().flatten().min().unwrap_or(0);
    let end_ms = [block_in, track.last().map(|p| p.t_ms)].into_iter().flatten().max().unwrap_or(start_ms);

    // Volanta's block time leaves out time paused: keep its exact figure.
    let paused_ms = match (block_out, block_in, f(&v, "realBlockTime")) {
        (Some(a), Some(b), Some(real_s)) => ((b - a) - (real_s * 1000.0).round() as i64).max(0),
        _ => 0,
    };

    // Takeoff: the first airborne position after being on the ground;
    // landing: back on the ground after the last airborne position.
    let takeoff_ms = track.windows(2).find(|w| w[0].on_ground && !w[1].on_ground).map(|w| w[1].t_ms);
    let landing_ms = track.windows(2).rev().find(|w| !w[0].on_ground && w[1].on_ground).map(|w| w[1].t_ms);

    let events = v.get("events").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut landings: Vec<LandingReport> = events
        .iter()
        .filter(|e| e.get("landingRate").is_some())
        .filter_map(|e| {
            let hdg = f(e, "heading").unwrap_or(0.0);
            let (hw, xw) = wind_parts(hdg, f(e, "windHeading").unwrap_or(0.0), f(e, "windSpeed").unwrap_or(0.0));
            Some(LandingReport {
                t_ms: t(e, "time")?,
                lat: f(e, "latitude").unwrap_or(0.0),
                lon: f(e, "longitude").unwrap_or(0.0),
                vs_fpm: f(e, "landingRate")?,
                vs_source: "volanta".into(),
                g_peak: f(e, "gForce").unwrap_or(0.0),
                pitch_deg: f(e, "pitch").unwrap_or(0.0),
                bank_deg: f(e, "roll").unwrap_or(0.0),
                gs_kt: f(e, "groundSpeed").unwrap_or(0.0),
                ias_kt: 0.0,
                hdg_deg: hdg,
                headwind_kt: hw,
                crosswind_kt: xw,
                bounces: 0,
                touch_and_go: false,
            })
        })
        .collect();
    landings.sort_by_key(|l| l.t_ms);
    // Every landing but the last was a touch-and-go.
    let n = landings.len();
    for l in landings.iter_mut().take(n.saturating_sub(1)) {
        l.touch_and_go = true;
    }

    let max_sim_rate = events.iter().filter_map(|e| f(e, "rate")).fold(1.0, f64::max);
    let first = track.first();
    let last = track.last();
    let origin = v.get("origin");
    let dest = v.get("destination");
    let summary = FlightSummary {
        sim: guess_sim(&v, start_ms),
        aircraft: AircraftInfo {
            title: st(&v, "aircraftTitle").unwrap_or_default(),
            icao_type: st(&v, "aircraftIcao").unwrap_or_default(),
            registration: st(&v, "aircraftRegistration").unwrap_or_default(),
            engine_count: 0,
            engine_type: EngineType::Unknown,
        },
        status: if st(&v, "state").as_deref() == Some("Completed") { FlightStatus::Complete } else { FlightStatus::Partial },
        end_reason: "imported from Volanta".into(),
        started_airborne: first.is_some_and(|p| !p.on_ground),
        start_ms,
        end_ms,
        block_out_ms: block_out,
        takeoff_ms,
        landing_ms: landing_ms.or(landings.last().map(|l| l.t_ms)),
        block_in_ms: block_in,
        dep_lat: first.map(|p| p.lat).or_else(|| origin.and_then(|o| f(o, "latitude"))).unwrap_or(0.0),
        dep_lon: first.map(|p| p.lon).or_else(|| origin.and_then(|o| f(o, "longitude"))).unwrap_or(0.0),
        arr_lat: last.map(|p| p.lat).or_else(|| dest.and_then(|o| f(o, "latitude"))).unwrap_or(0.0),
        arr_lon: last.map(|p| p.lon).or_else(|| dest.and_then(|o| f(o, "longitude"))).unwrap_or(0.0),
        distance_nm: f(&v, "distanceFlownInNauticalMiles").unwrap_or(0.0),
        max_alt_ft: track.iter().map(|p| f64::from(p.alt_ft)).fold(0.0, f64::max),
        fuel_used_kg: f(&v, "fuelBurn").unwrap_or(0.0),
        paused_ms,
        max_sim_rate,
        landings,
        track,
        events: vec![],
    };
    let ctx = SaveContext {
        dep: place(refdata, origin),
        arr: place(refdata, dest),
        planned_arr_icao: None,
        callsign: st(&v, "callsign").unwrap_or_default(),
        flight_no: st(&v, "flightNumber").unwrap_or_default(),
        plan: None,
        source: Some(Imported { source: SOURCE.into(), id: id.clone(), file: text.to_string() }),
        aircraft_id: None,
    };
    Ok(ImportedFlight { id, summary, ctx, note: st(&v, "note") })
}

// --- export ---------------------------------------------------------------

/// A stable, UUID-shaped id from a seed, so exporting the same flight twice
/// gives the same ids.
fn uuid_from(seed: &str) -> String {
    let h = |salt: u64| {
        let mut x: u64 = 0xcbf2_9ce4_8422_2325 ^ salt;
        for b in seed.bytes() {
            x ^= u64::from(b);
            x = x.wrapping_mul(0x0000_0100_0000_01b3);
        }
        x
    };
    let (a, b) = (h(1), h(2));
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&a.to_be_bytes());
    bytes[8..].copy_from_slice(&b.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4 layout
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // RFC 4122 variant
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

fn airport_json(refdata: &refdata::RefData, icao: Option<&str>, name: Option<&str>) -> Value {
    let Some(icao) = icao.filter(|s| !s.is_empty()) else { return Value::Null };
    let r = refdata.airport(icao);
    json!({
        "name": name.map(str::to_string).or(r.map(|a| a.name.clone())),
        "icaoCode": icao,
        "iataCode": null,
        "latitude": r.map(|a| a.lat),
        "longitude": r.map(|a| a.lon),
        "countryId": null,
        "country": null,
        "timezone": null,
        "isClosed": false,
        "metar": null,
        "isPending": null,
        "id": null
    })
}

/// Everything about a FlightDeck flight that the Volanta format holds.
pub struct ExportInput<'a> {
    pub row: &'a FlightRow,
    pub track: &'a [TrackPoint],
    pub landings: &'a [LandingReport],
    /// (time ms, kind, data JSON) from the flight's event log.
    pub events: &'a [(i64, String, String)],
    pub plan: Option<&'a crate::simbrief::FlightPlan>,
    pub note: Option<String>,
}

/// A FlightDeck flight as a Volanta flight file.
pub fn export(refdata: &refdata::RefData, x: &ExportInput) -> Value {
    let r = x.row;
    let fid = uuid_from(&format!("flightdeck-flight-{}-{}", r.id, r.start_ms));
    let mut events: Vec<Value> = Vec::new();
    for (i, l) in x.landings.iter().enumerate() {
        let kt = l.headwind_kt.hypot(l.crosswind_kt);
        let from = (l.hdg_deg + l.crosswind_kt.atan2(l.headwind_kt).to_degrees()).rem_euclid(360.0);
        events.push(json!({
            "landingRate": l.vs_fpm,
            "gForce": l.g_peak,
            "pitch": l.pitch_deg,
            "roll": l.bank_deg,
            "groundSpeed": l.gs_kt,
            "latitude": l.lat,
            "longitude": l.lon,
            "heading": l.hdg_deg,
            "windHeading": from,
            "windSpeed": kt,
            "isHidden": false,
            "flightId": fid,
            "time": format_time(l.t_ms, false),
            "id": uuid_from(&format!("{fid}-landing-{i}"))
        }));
    }
    // Pauses and sim-rate changes become Volanta's "rate" events.
    let mut paused_ms = 0i64;
    let mut paused_at: Option<i64> = None;
    for (i, (t_ms, kind, data)) in x.events.iter().enumerate() {
        let rate = match kind.as_str() {
            "paused" => {
                paused_at = Some(*t_ms);
                0.0
            }
            "resumed" => {
                if let Some(p) = paused_at.take() {
                    paused_ms += t_ms - p;
                }
                1.0
            }
            "sim_rate" => serde_json::from_str::<Value>(data).ok().and_then(|d| f(&d, "rate")).unwrap_or(1.0),
            _ => continue,
        };
        events.push(json!({
            "rate": rate,
            "flightId": fid,
            "time": format_time(*t_ms, false),
            "id": uuid_from(&format!("{fid}-rate-{i}"))
        }));
    }
    let positions: Vec<Value> = x
        .track
        .iter()
        .map(|p| {
            json!({
                "latitude": p.lat,
                "longitude": p.lon,
                "altitude": f64::from(p.alt_ft),
                "altitudeAgl": null,
                "pitch": null,
                "bank": null,
                "headingTrue": f64::from(p.hdg_deg),
                "groundSpeed": f64::from(p.gs_kt),
                "onGround": p.on_ground,
                "time": format_time(p.t_ms, true),
                "network": "Volanta",
                "frameRate": 0.0,
                "memoryUsedMB": 0,
                "fuelKg": null,
                "transponder": null,
                "verticalSpeed": f64::from(p.vs_fpm)
            })
        })
        .collect();
    let block_s = match (r.block_out, r.block_in) {
        (Some(a), Some(b)) => Some(((b - a - paused_ms).max(0)) as f64 / 1000.0),
        _ => None,
    };
    let air_s = match (r.takeoff, r.landing) {
        (Some(a), Some(b)) => Some(((b - a).max(0)) as f64 / 1000.0),
        _ => None,
    };
    let plan = x.plan;
    let alt = plan.map(|p| p.alternate.clone()).filter(|s| !s.is_empty());
    let mut m = Map::new();
    let mut put = |k: &str, v: Value| {
        m.insert(k.to_string(), v);
    };
    put("id", json!(fid));
    put("plan", Value::Null);
    put("callsign", if r.callsign.is_empty() { Value::Null } else { json!(r.callsign) });
    put("flightNumber", plan.map(|p| json!(p.flight_number)).unwrap_or(Value::Null));
    put("origin", airport_json(refdata, r.dep_icao.as_deref(), r.dep_name.as_deref()));
    put("originId", Value::Null);
    put("originIcao", json!(r.dep_icao));
    put("destination", airport_json(refdata, r.arr_icao.as_deref(), r.arr_name.as_deref()));
    put("destinationId", Value::Null);
    put("destinationIcao", json!(r.arr_icao));
    put("alternate", airport_json(refdata, alt.as_deref(), None));
    put("alternateId", Value::Null);
    put("alternateIcao", json!(alt));
    put("hasDiverted", json!(plan.is_some_and(|p| Some(&p.destination.icao) != r.arr_icao.as_ref())));
    put("atcRoute", plan.map(|p| json!(p.route)).unwrap_or(Value::Null));
    put("sessionId", json!(uuid_from(&format!("{fid}-session"))));
    put("aircraftTitle", json!(r.aircraft_title));
    put("aircraftDirectory", Value::Null);
    put("aircraftRegistration", if r.registration.is_empty() { Value::Null } else { json!(r.registration) });
    put("aircraftIcao", if r.icao_type.is_empty() { Value::Null } else { json!(r.icao_type) });
    put("events", Value::Array(events));
    put("positions", Value::Array(positions));
    put("network", json!("Volanta"));
    put("networkUserId", Value::Null);
    put("state", json!(if r.status == "complete" { "Completed" } else { "Aborted" }));
    put("offBlocksTime", r.block_out.map(|t| json!(format_time(t, false))).unwrap_or(Value::Null));
    put("onBlocksTime", r.block_in.map(|t| json!(format_time(t, false))).unwrap_or(Value::Null));
    put("realBlockTime", json!(block_s));
    put("effectiveBlockTime", json!(block_s));
    put("realFlightTime", json!(air_s));
    put("effectiveFlightTime", json!(air_s));
    put("fuelBurn", json!(r.fuel_used_kg));
    put("plannedFuelBurn", Value::Null);
    put("plannedBlockTime", plan.map(|p| json!(p.ete_min * 60.0)).unwrap_or(Value::Null));
    put("plannedFlightTime", Value::Null);
    put("plannedPax", Value::Null);
    put("plannedCargo", Value::Null);
    put("plannedAltitude", plan.map(|p| json!(p.cruise_ft)).unwrap_or(Value::Null));
    put("createdAt", json!(format_time(r.start_ms, false)));
    put("importedFlightId", Value::Null);
    put("originCountryId", Value::Null);
    put("destinationCountryId", Value::Null);
    put("note", json!(x.note));
    put("networkUserName", Value::Null);
    put("distanceFlownInNauticalMiles", json!(r.distance_nm));
    put("screenshots", json!([]));
    put("savedFlightFiles", json!([]));
    put("tags", json!([]));
    Value::Object(m)
}

/// The file name Volanta uses: the flight id.
pub fn file_name(json: &Value) -> String {
    format!("{}.json", json.get("id").and_then(Value::as_str).unwrap_or("flight"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times() {
        assert_eq!(parse_time("1970-01-01T00:00:00Z"), Some(0));
        let t = parse_time("2023-11-24T16:42:09.958852").unwrap();
        assert_eq!(format_time(t, false), "2023-11-24T16:42:09.958");
        assert_eq!(parse_time("2023-11-24T16:41:44.578Z").map(|t| format_time(t, true)).as_deref(), Some("2023-11-24T16:41:44.578Z"));
        assert_eq!(parse_time("2023-11-24T17:41:44+01:00"), parse_time("2023-11-24T16:41:44Z"));
        assert_eq!(parse_time("2024-02-29T12:00:00Z").map(|t| format_time(t, true)).as_deref(), Some("2024-02-29T12:00:00.000Z"));
    }

    #[test]
    fn uuids_are_stable_and_well_formed() {
        let a = uuid_from("x");
        assert_eq!(a, uuid_from("x"));
        assert_ne!(a, uuid_from("y"));
        assert_eq!(a.len(), 36);
        assert_eq!(&a[14..15], "4");
    }

    /// Round trip a real Volanta file: set FLIGHTDECK_VOLANTA_SAMPLE to its
    /// path and run with `--ignored`. (Pilots' files stay out of the repo.)
    #[test]
    #[ignore]
    fn real_volanta_file_round_trip() {
        let path = std::env::var("FLIGHTDECK_VOLANTA_SAMPLE").expect("set FLIGHTDECK_VOLANTA_SAMPLE");
        let text = std::fs::read_to_string(path).unwrap();
        let refdata = refdata::RefData::load();
        let mut db = store::Store::open_in_memory().unwrap();
        let p = parse(&text, &refdata).unwrap();
        let id = db.save_flight(&p.summary, &p.ctx).unwrap();
        // Exactly the same file back.
        assert_eq!(db.get_imported(id).unwrap().unwrap().file, text);
        assert_eq!(db.find_imported(SOURCE, &p.id).unwrap(), Some(id));

        let orig: Value = serde_json::from_str(&text).unwrap();
        let row = db.get_flight(id).unwrap().unwrap();
        println!("{row:#?}");
        let track = db.get_track(id).unwrap();
        let landings = db.get_landings(id).unwrap();
        let events = db.get_events(id).unwrap();
        let x = export(&refdata, &ExportInput { row: &row, track: &track, landings: &landings, events: &events, plan: None, note: None });
        // Same fields, in the same order.
        let keys = |v: &Value| v.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
        assert_eq!(keys(&x), keys(&orig));
        for k in ["offBlocksTime", "onBlocksTime", "originIcao", "destinationIcao", "aircraftTitle", "state"] {
            println!("{k}: {} | {}", orig[k], x[k]);
        }
        assert_eq!(x["offBlocksTime"], json!(format_time(parse_time(orig["offBlocksTime"].as_str().unwrap()).unwrap(), false)));
        assert_eq!(x["originIcao"], orig["originIcao"]);
        assert_eq!(x["aircraftTitle"], orig["aircraftTitle"]);
        assert!((x["fuelBurn"].as_f64().unwrap() - orig["fuelBurn"].as_f64().unwrap()).abs() < 1e-6);
        assert!((x["distanceFlownInNauticalMiles"].as_f64().unwrap() - orig["distanceFlownInNauticalMiles"].as_f64().unwrap()).abs() < 1e-6);
        let lr = |v: &Value| v["events"].as_array().unwrap().iter().find_map(|e| e["landingRate"].as_f64());
        assert!((lr(&x).unwrap() - lr(&orig).unwrap()).abs() < 1e-6);
        println!("block time: original {} s, FlightDeck {} min", orig["realBlockTime"], row.total_min);
        println!("positions: original {}, stored {}", orig["positions"].as_array().unwrap().len(), track.len());
    }

    #[test]
    fn wind_split() {
        let (hw, xw) = wind_parts(90.0, 90.0, 10.0);
        assert!((hw - 10.0).abs() < 1e-9 && xw.abs() < 1e-9);
        let (hw, xw) = wind_parts(90.0, 180.0, 10.0);
        assert!(hw.abs() < 1e-9 && (xw - 10.0).abs() < 1e-9, "wind from the right is positive");
    }
}
