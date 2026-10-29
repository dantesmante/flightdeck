//! Elevatex flight files (one JSON per flight): import them as FlightDeck
//! flights. The original file is kept with the flight, as for Volanta.

use serde_json::Value;
use store::{Imported, Place, SaveContext};
use telemetry::{AircraftInfo, EngineType, SimKind, TrackPoint};
use tracker::{FlightStatus, FlightSummary, LandingReport};

use crate::volanta::{parse_time, ImportedFlight};

pub const SOURCE: &str = "elevatex";

fn f(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64)
}
fn st(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}
fn t(v: &Value, k: &str) -> Option<i64> {
    v.get(k).and_then(Value::as_str).and_then(parse_time)
}

/// Does this JSON look like an Elevatex flight?
pub fn looks_like(v: &Value) -> bool {
    v.get("Id").is_some() && (v.get("Positions").is_some() || v.get("Origin").is_some())
}

fn place(refdata: &refdata::RefData, apt: Option<&Value>) -> Option<Place> {
    let a = apt.filter(|a| a.is_object())?;
    let icao = st(a, "Icao").or_else(|| st(a, "Gps"))?;
    match refdata.airport(&icao) {
        Some(r) => Some(Place { icao: r.ident.clone(), name: r.name.clone(), country: r.country.clone(), region: r.region.clone() }),
        // Not in FlightDeck's airport list: keep Elevatex's name.
        None => Some(Place { icao, name: st(a, "Name").unwrap_or_default(), country: String::new(), region: String::new() }),
    }
}

fn sim(v: &Value) -> SimKind {
    let s = st(v, "Simulator").unwrap_or_default().to_lowercase();
    if s.contains("2024") {
        SimKind::Msfs2024
    } else if s.contains("2020") || s.contains("msfs") || s.contains("microsoft") {
        SimKind::Msfs2020
    } else if s.contains("x-plane") || s.contains("xplane") {
        if s.contains("11") { SimKind::Xp11 } else { SimKind::Xp12 }
    } else {
        SimKind::Other
    }
}

/// Head and cross wind from the wind direction and speed, for a heading.
fn wind_parts(hdg: f64, wind_from: f64, kt: f64) -> (f64, f64) {
    let a = (wind_from - hdg).to_radians();
    (kt * a.cos(), kt * a.sin())
}

/// Landing reports from Elevatex's events (a JSON list inside a string,
/// each event's details again a JSON string).
fn landings(v: &Value) -> Vec<LandingReport> {
    let events: Vec<Value> = match v.get("Events") {
        Some(Value::String(s)) => serde_json::from_str(s).unwrap_or_default(),
        Some(Value::Array(a)) => a.clone(),
        _ => vec![],
    };
    let mut out: Vec<LandingReport> = events
        .iter()
        .filter(|e| st(e, "eventType").or_else(|| st(e, "EventType")).is_some_and(|k| k.eq_ignore_ascii_case("landing")))
        .filter_map(|e| {
            let args: Value = match e.get("args").or_else(|| e.get("Args")) {
                Some(Value::String(s)) => serde_json::from_str(s).ok()?,
                Some(a @ Value::Object(_)) => a.clone(),
                _ => return None,
            };
            let hdg = f(&args, "heading").unwrap_or(0.0);
            let (hw, xw) = wind_parts(hdg, f(&args, "windDirection").unwrap_or(0.0), f(&args, "windSpeed").unwrap_or(0.0));
            Some(LandingReport {
                t_ms: t(e, "DateTime").or_else(|| t(e, "dateTime"))?,
                lat: f(&args, "latitude").unwrap_or(0.0),
                lon: f(&args, "longitude").unwrap_or(0.0),
                vs_fpm: f(&args, "verticleSpeed").or_else(|| f(&args, "verticalSpeed"))?,
                vs_source: "elevatex".into(),
                g_peak: f(&args, "gforce").unwrap_or(0.0),
                pitch_deg: f(&args, "pitch").unwrap_or(0.0),
                bank_deg: f(&args, "bank").unwrap_or(0.0),
                gs_kt: f(&args, "groundSpeed").unwrap_or(0.0),
                ias_kt: 0.0,
                hdg_deg: hdg,
                headwind_kt: hw,
                crosswind_kt: xw,
                bounces: f(&args, "bounces").unwrap_or(0.0).max(0.0) as u32,
                touch_and_go: false,
            })
        })
        .collect();
    out.sort_by_key(|l| l.t_ms);
    let n = out.len();
    for l in out.iter_mut().take(n.saturating_sub(1)) {
        l.touch_and_go = true;
    }
    out
}

pub fn parse(text: &str, refdata: &refdata::RefData) -> Result<ImportedFlight, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("not a JSON file ({e})"))?;
    if !looks_like(&v) {
        return Err("not an Elevatex flight".into());
    }
    let id = st(&v, "Id").ok_or("not an Elevatex flight (no Id)")?;
    let positions = v.get("Positions").and_then(Value::as_array).cloned().unwrap_or_default();

    let mut track: Vec<TrackPoint> = positions
        .iter()
        .filter_map(|p| {
            Some(TrackPoint {
                t_ms: t(p, "CreatedAt")?,
                lat: f(p, "Latitude")?,
                lon: f(p, "Longitude")?,
                alt_ft: f(p, "Altitude").unwrap_or(0.0) as f32,
                gs_kt: f(p, "GroundSpeed").unwrap_or(0.0) as f32,
                vs_fpm: f(p, "VerticalSpeed").unwrap_or(0.0) as f32,
                hdg_deg: f(p, "Heading").unwrap_or(0.0) as f32,
                on_ground: p.get("OnGround").and_then(Value::as_bool).unwrap_or(false),
            })
        })
        .collect();
    track.sort_by_key(|p| p.t_ms);
    if track.is_empty() && v.get("Origin").is_none() {
        return Err("not an Elevatex flight (no positions or origin)".into());
    }

    let block_out = t(&v, "OffBlocks");
    let block_in = t(&v, "OnBlocks");
    let start_ms = [block_out, t(&v, "DepartedAt"), track.first().map(|p| p.t_ms)].into_iter().flatten().min().unwrap_or(0);
    let end_ms = [block_in, t(&v, "LandedAt"), track.last().map(|p| p.t_ms)].into_iter().flatten().max().unwrap_or(start_ms);

    let takeoff_ms = t(&v, "DepartedAt").or_else(|| track.windows(2).find(|w| w[0].on_ground && !w[1].on_ground).map(|w| w[1].t_ms));
    let landings = landings(&v);
    let landing_ms = t(&v, "LandedAt")
        .or_else(|| track.windows(2).rev().find(|w| !w[0].on_ground && w[1].on_ground).map(|w| w[1].t_ms))
        .or(landings.last().map(|l| l.t_ms));

    let first = track.first();
    let last = track.last();
    let origin = v.get("Origin");
    let dest = v.get("Destination");
    let complete = v.get("Completed").and_then(Value::as_bool).unwrap_or(true)
        && !v.get("Cancelled").and_then(Value::as_bool).unwrap_or(false);
    let summary = FlightSummary {
        sim: sim(&v),
        aircraft: AircraftInfo {
            title: st(&v, "AircraftName").unwrap_or_default(),
            icao_type: st(&v, "AircraftIcao").unwrap_or_default(),
            registration: st(&v, "AircraftRegistration").unwrap_or_default(),
            engine_count: 0,
            engine_type: EngineType::Unknown,
        },
        status: if complete { FlightStatus::Complete } else { FlightStatus::Partial },
        end_reason: "imported from Elevatex".into(),
        started_airborne: first.is_some_and(|p| !p.on_ground),
        start_ms,
        end_ms,
        block_out_ms: block_out,
        takeoff_ms,
        landing_ms,
        block_in_ms: block_in,
        dep_lat: first.map(|p| p.lat).or_else(|| origin.and_then(|o| f(o, "Latitude"))).unwrap_or(0.0),
        dep_lon: first.map(|p| p.lon).or_else(|| origin.and_then(|o| f(o, "Longitude"))).unwrap_or(0.0),
        arr_lat: last.map(|p| p.lat).or_else(|| dest.and_then(|o| f(o, "Latitude"))).unwrap_or(0.0),
        arr_lon: last.map(|p| p.lon).or_else(|| dest.and_then(|o| f(o, "Longitude"))).unwrap_or(0.0),
        distance_nm: f(&v, "FlownDistance").filter(|d| *d > 0.0).or_else(|| f(&v, "GcDistance")).unwrap_or(0.0),
        max_alt_ft: track.iter().map(|p| f64::from(p.alt_ft)).fold(0.0, f64::max),
        fuel_used_kg: f(&v, "FuelBurnedKg").unwrap_or(0.0),
        paused_ms: (f(&v, "TimePaused").unwrap_or(0.0) * 1000.0).round() as i64,
        max_sim_rate: 1.0,
        landings,
        track,
        events: vec![],
    };
    let ctx = SaveContext {
        dep: place(refdata, origin),
        arr: place(refdata, dest),
        planned_arr_icao: None,
        callsign: st(&v, "Callsign").unwrap_or_default(),
        flight_no: st(&v, "FlightNumber").unwrap_or_default(),
        plan: None,
        source: Some(Imported { source: SOURCE.into(), id: id.clone(), file: text.to_string() }),
        aircraft_id: None,
    };
    Ok(ImportedFlight { id, summary, ctx, note: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_elevatex_flight() {
        let text = r#"{"Id":"0f3c1b26","Callsign":"DLH245","FlightNumber":"LH245","Simulator":null,
            "Events":"[{\"eventType\":\"Landing\",\"args\":\"{\\\"bounces\\\":0,\\\"gforce\\\":1.06,\\\"heading\\\":224.0,\\\"verticleSpeed\\\":-121.9,\\\"windDirection\\\":250.0,\\\"windSpeed\\\":4.3,\\\"latitude\\\":51.89,\\\"longitude\\\":0.247}\",\"DateTime\":\"2024-01-13T16:23:20.33246\"}]",
            "Completed":true,"Cancelled":false,
            "Origin":{"Icao":"EDDH","Name":"Hamburg","Latitude":53.63,"Longitude":9.99},
            "Destination":{"Icao":"EGSS","Name":"Stansted","Latitude":51.885,"Longitude":0.235},
            "AircraftName":null,"AircraftRegistration":"EI-DTE","AircraftIcao":"A320",
            "OffBlocks":null,"OnBlocks":null,"DepartedAt":null,"LandedAt":null,
            "FlownDistance":133.4,"FlightTime":1137,"BlockTime":1437,"TimePaused":0,"FuelBurnedKg":677.7,
            "Positions":[
              {"Longitude":0.25,"Latitude":51.89,"Altitude":106,"GroundSpeed":0,"Heading":224,"VerticalSpeed":0,"OnGround":true,"CreatedAt":"2024-01-13T16:28:39.349Z"},
              {"Longitude":1.49,"Latitude":52.03,"Altitude":3614.6,"GroundSpeed":142,"Heading":298,"VerticalSpeed":-126,"OnGround":false,"CreatedAt":"2024-01-13T16:04:32.348Z"}
            ]}"#;
        let refdata = refdata::RefData::load();
        let p = parse(text, &refdata).unwrap();
        assert_eq!(p.id, "0f3c1b26");
        let s = &p.summary;
        assert_eq!(s.aircraft.registration, "EI-DTE");
        assert_eq!(s.track.len(), 2);
        assert!(s.track[0].t_ms < s.track[1].t_ms, "positions sorted by time");
        assert!(s.started_airborne);
        assert_eq!(s.landings.len(), 1);
        assert!((s.landings[0].vs_fpm + 121.9).abs() < 0.01);
        assert_eq!(s.end_ms - s.start_ms, 24 * 60_000 + 7_001);
        assert_eq!(p.ctx.dep.as_ref().unwrap().icao, "EDDH");
        assert_eq!(p.ctx.arr.as_ref().unwrap().icao, "EGSS");
        assert_eq!(p.ctx.callsign, "DLH245");
        assert!(matches!(s.status, FlightStatus::Complete));
    }

    /// Check a real export: FD_ELEVATEX_FILE=path cargo test -- --ignored elevatex_file
    #[test]
    #[ignore]
    fn elevatex_file() {
        let path = std::env::var("FD_ELEVATEX_FILE").expect("set FD_ELEVATEX_FILE");
        let text = std::fs::read_to_string(path).unwrap();
        let p = parse(&text, &refdata::RefData::load()).unwrap();
        let s = &p.summary;
        println!(
            "{} {:?}->{:?} {} {} pts={} landings={:?} dist={:.0} fuel={:.0} minutes={:.1}",
            p.id,
            p.ctx.dep.as_ref().map(|d| &d.icao),
            p.ctx.arr.as_ref().map(|d| &d.icao),
            s.aircraft.icao_type,
            s.aircraft.registration,
            s.track.len(),
            s.landings.iter().map(|l| l.vs_fpm.round()).collect::<Vec<_>>(),
            s.distance_nm,
            s.fuel_used_kg,
            s.total_minutes()
        );
    }
}
