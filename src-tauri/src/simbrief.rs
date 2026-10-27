//! SimBrief: fetch the pilot's latest dispatched flight plan (OFP).
//!
//! https://developers.navigraph.com/docs/simbrief/fetching-ofp-data
//! `xml.fetcher.php?userid=…&json=1` (or `username=…`) returns the newest
//! OFP as JSON, or HTTP 400 when the user is unknown or has no flights.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PlanAirport {
    pub icao: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub runway: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PlanFix {
    pub ident: String,
    /// "wpt", "vor", "ndb", "apt", "ltlg", …
    pub kind: String,
    pub lat: f64,
    pub lon: f64,
    pub alt_ft: f64,
    /// "CLB", "CRZ" or "DSC".
    pub stage: String,
    /// Part of the SID or STAR.
    pub procedure: bool,
}

/// A flight plan, from SimBrief or (later) other sources.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct FlightPlan {
    pub source: String,
    /// SimBrief's id for this OFP, so the same plan is not offered twice.
    pub id: String,
    pub generated_ms: i64,
    /// e.g. "AUA11"
    pub callsign: String,
    pub airline: String,
    pub flight_number: String,
    pub origin: PlanAirport,
    pub destination: PlanAirport,
    pub alternate: String,
    pub route: String,
    pub cruise_ft: f64,
    pub distance_nm: f64,
    pub ete_min: f64,
    pub aircraft_icao: String,
    pub aircraft_name: String,
    pub registration: String,
    /// Block fuel in kilograms.
    pub block_fuel_kg: f64,
    pub sched_out_ms: Option<i64>,
    pub sched_in_ms: Option<i64>,
    pub fixes: Vec<PlanFix>,
}

/// A text field. SimBrief writes empty fields as `{}`, so anything that is
/// not a string or number reads as "".
fn s(v: &Value, path: &[&str]) -> String {
    let mut cur = v;
    for p in path {
        match cur.get(p) {
            Some(n) => cur = n,
            None => return String::new(),
        }
    }
    match cur {
        Value::String(t) => t.trim().to_string(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn n(v: &Value, path: &[&str]) -> f64 {
    s(v, path).parse().unwrap_or(0.0)
}

fn t_ms(v: &Value, path: &[&str]) -> Option<i64> {
    s(v, path).parse::<i64>().ok().filter(|t| *t > 0).map(|t| t * 1000)
}

fn airport(v: &Value, key: &str) -> PlanAirport {
    PlanAirport {
        icao: s(v, &[key, "icao_code"]),
        name: s(v, &[key, "name"]),
        lat: n(v, &[key, "pos_lat"]),
        lon: n(v, &[key, "pos_long"]),
        runway: s(v, &[key, "plan_rwy"]),
    }
}

/// Turn SimBrief's OFP JSON into a FlightPlan.
pub fn parse_ofp(v: &Value) -> Result<FlightPlan, String> {
    let origin = airport(v, "origin");
    let destination = airport(v, "destination");
    if origin.icao.is_empty() || destination.icao.is_empty() {
        return Err("SimBrief sent a flight plan without airports.".into());
    }
    // A single fix comes as an object rather than a list.
    let fixes = match v.get("navlog").and_then(|n| n.get("fix")) {
        Some(Value::Array(a)) => a.clone(),
        Some(o @ Value::Object(_)) => vec![o.clone()],
        _ => vec![],
    };
    let fixes = fixes
        .iter()
        .map(|f| PlanFix {
            ident: s(f, &["ident"]),
            kind: s(f, &["type"]),
            lat: n(f, &["pos_lat"]),
            lon: n(f, &["pos_long"]),
            alt_ft: n(f, &["altitude_feet"]),
            stage: s(f, &["stage"]),
            procedure: s(f, &["is_sid_star"]) == "1",
        })
        .filter(|f| f.lat != 0.0 || f.lon != 0.0)
        .collect();
    let kg = if s(v, &["params", "units"]).to_lowercase().starts_with("lb") { 0.453_592_37 } else { 1.0 };
    let airline = s(v, &["general", "icao_airline"]);
    let flight_number = s(v, &["general", "flight_number"]);
    let mut callsign = s(v, &["atc", "callsign"]);
    if callsign.is_empty() {
        callsign = format!("{airline}{flight_number}");
    }
    let mut distance_nm = n(v, &["general", "route_distance"]);
    if distance_nm == 0.0 {
        distance_nm = n(v, &["general", "air_distance"]);
    }
    Ok(FlightPlan {
        source: "simbrief".into(),
        id: s(v, &["params", "request_id"]),
        generated_ms: t_ms(v, &["params", "time_generated"]).unwrap_or(0),
        callsign,
        airline,
        flight_number,
        origin,
        destination,
        alternate: s(v, &["alternate", "icao_code"]),
        route: s(v, &["general", "route"]),
        cruise_ft: n(v, &["general", "initial_altitude"]),
        distance_nm,
        ete_min: (n(v, &["times", "est_time_enroute"]) / 60.0).round(),
        aircraft_icao: s(v, &["aircraft", "icaocode"]),
        aircraft_name: s(v, &["aircraft", "name"]),
        registration: s(v, &["aircraft", "reg"]),
        block_fuel_kg: (n(v, &["fuel", "plan_ramp"]) * kg).round(),
        sched_out_ms: t_ms(v, &["times", "sched_out"]),
        sched_in_ms: t_ms(v, &["times", "sched_in"]),
        fixes,
    })
}

/// Fetch the latest OFP for a SimBrief Pilot ID (digits) or username.
pub fn latest(who: &str) -> Result<FlightPlan, String> {
    let who = who.trim();
    if who.is_empty() {
        return Err("Add your SimBrief Pilot ID in Settings first.".into());
    }
    let key = if who.chars().all(|c| c.is_ascii_digit()) { "userid" } else { "username" };
    let res = crate::update::agent()
        .get("https://www.simbrief.com/api/xml.fetcher.php")
        .query(key, who)
        .query("json", "1")
        .call();
    let mut res = match res {
        Ok(r) => r,
        Err(ureq::Error::StatusCode(400)) => {
            return Err(format!(
                "SimBrief has no flight plan for {who}. Check the Pilot ID in Settings, and dispatch a flight on simbrief.com."
            ))
        }
        Err(e) => return Err(format!("Could not reach SimBrief: {e}")),
    };
    let v: Value = res
        .body_mut()
        .with_config()
        .limit(20 * 1024 * 1024)
        .read_json()
        .map_err(|e| format!("Unexpected answer from SimBrief: {e}"))?;
    parse_ofp(&v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_ofp() {
        let v: Value = serde_json::json!({
            "params": { "request_id": "123", "time_generated": "1793140000", "units": "lbs" },
            "general": { "icao_airline": "AUA", "flight_number": "11", "route": "DCT NIZ UM2 ELB DCT",
                          "initial_altitude": "36000", "route_distance": "390" },
            "atc": { "callsign": "AUA11" },
            "origin": { "icao_code": "LFMN", "name": "Nice Cote D'Azur", "pos_lat": "43.658", "pos_long": "7.215", "plan_rwy": "04R" },
            "destination": { "icao_code": "LIRQ", "name": "Firenze", "pos_lat": "43.810", "pos_long": "11.205", "plan_rwy": "05" },
            "alternate": {},
            "aircraft": { "icaocode": "A320", "name": "A320-200", "reg": "OE-LZE" },
            "times": { "est_time_enroute": "3720", "sched_out": "1793141000" },
            "fuel": { "plan_ramp": "11000" },
            "navlog": { "fix": [
                { "ident": "NIZ", "type": "vor", "pos_lat": "43.77", "pos_long": "7.25", "altitude_feet": "12000", "stage": "CLB", "is_sid_star": "1" },
                { "ident": "ELB", "type": "vor", "pos_lat": "42.73", "pos_long": "10.39", "altitude_feet": "36000", "stage": "CRZ", "is_sid_star": "0" }
            ]}
        });
        let p = parse_ofp(&v).unwrap();
        assert_eq!(p.callsign, "AUA11");
        assert_eq!((p.origin.icao.as_str(), p.destination.icao.as_str()), ("LFMN", "LIRQ"));
        assert_eq!(p.alternate, "");
        assert_eq!(p.fixes.len(), 2);
        assert!(p.fixes[0].procedure);
        assert_eq!(p.ete_min, 62.0);
        assert_eq!(p.block_fuel_kg, 4990.0);
        assert_eq!(p.sched_out_ms, Some(1_793_141_000_000));
    }

    #[test]
    fn single_fix_and_missing_callsign() {
        let v: Value = serde_json::json!({
            "general": { "icao_airline": "FDX", "flight_number": "100" },
            "atc": { "callsign": {} },
            "origin": { "icao_code": "KSEA" }, "destination": { "icao_code": "KPDX" },
            "navlog": { "fix": { "ident": "BTG", "pos_lat": "45.7", "pos_long": "-122.6" } }
        });
        let p = parse_ofp(&v).unwrap();
        assert_eq!(p.callsign, "FDX100");
        assert_eq!(p.fixes.len(), 1);
    }
}
