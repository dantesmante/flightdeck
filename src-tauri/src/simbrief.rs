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

/// One NOTAM.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Notam {
    pub id: String,
    pub location: String,
    pub text: String,
}

/// One row of the flight log (navlog).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct LogRow {
    pub ident: String,
    pub airway: String,
    pub stage: String,
    pub alt_ft: f64,
    pub track_mag: f64,
    pub dist_leg: f64,
    /// Seconds since departure.
    pub time_total: f64,
    pub fuel_onboard: f64,
    pub wind_dir: f64,
    pub wind_kt: f64,
    pub oat: f64,
    pub tas: f64,
    pub gs: f64,
    pub mach: String,
    /// Winds aloft at this fix: (altitude ft, direction, speed kt, OAT).
    pub winds: Vec<[f64; 4]>,
}

/// The briefing: the OFP itself and the sections pilots jump to.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Ofp {
    /// SimBrief's formatted OFP (HTML around preformatted text).
    pub html: String,
    /// "kgs" or "lbs": the unit of every weight and fuel figure below.
    pub units: String,
    /// The ICAO ATC flight plan, "(FPL-...)".
    pub fpl: String,
    /// Raw sections as SimBrief sends them, shown as labeled tables.
    pub times: serde_json::Map<String, Value>,
    pub weights: serde_json::Map<String, Value>,
    pub fuel: serde_json::Map<String, Value>,
    /// (label, METAR or TAF text)
    pub weather: Vec<(String, String)>,
    pub notams: Vec<Notam>,
    pub log: Vec<LogRow>,
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
    /// The full briefing, when the source has one.
    #[serde(default)]
    pub ofp: Option<Ofp>,
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

/// A list that may come as a single object, an array, or nothing.
fn list(v: Option<&Value>) -> Vec<Value> {
    match v {
        Some(Value::Array(a)) => a.clone(),
        Some(o @ Value::Object(_)) => vec![o.clone()],
        _ => vec![],
    }
}

/// Keep only the plain values of an object (strings and numbers).
fn flat(v: Option<&Value>) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();
    if let Some(Value::Object(m)) = v {
        for (k, x) in m {
            match x {
                Value::String(t) if !t.trim().is_empty() => {
                    out.insert(k.clone(), Value::String(t.trim().to_string()));
                }
                Value::Number(_) => {
                    out.insert(k.clone(), x.clone());
                }
                _ => {}
            }
        }
    }
    out
}

/// Text that may be a string or a list of strings.
fn text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(t)) => t.trim().to_string(),
        Some(Value::Array(a)) => a.iter().filter_map(|x| x.as_str()).map(str::trim).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

fn ofp(v: &Value) -> Ofp {
    let w = v.get("weather");
    let mut weather = Vec::new();
    for (key, label) in [
        ("orig_metar", "Departure METAR"),
        ("orig_taf", "Departure TAF"),
        ("dest_metar", "Destination METAR"),
        ("dest_taf", "Destination TAF"),
        ("altn_metar", "Alternate METAR"),
        ("altn_taf", "Alternate TAF"),
    ] {
        let t = text(w.and_then(|w| w.get(key)));
        if !t.is_empty() {
            weather.push((label.to_string(), t));
        }
    }
    let mut notams = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let sources = [
        v.get("origin").and_then(|o| o.get("notam")),
        v.get("destination").and_then(|o| o.get("notam")),
        v.get("alternate").and_then(|o| o.get("notam")),
        v.get("notams").and_then(|o| o.get("notamdrec")),
    ];
    for src in sources {
        for n in list(src) {
            let body = [s(&n, &["notam_text"]), s(&n, &["notam_raw"]), s(&n, &["notam_html"])]
                .into_iter()
                .find(|t| !t.is_empty())
                .unwrap_or_default();
            if body.is_empty() {
                continue;
            }
            let id = s(&n, &["notam_id"]);
            if !id.is_empty() && !seen.insert(id.clone()) {
                continue;
            }
            let mut location = s(&n, &["location_icao"]);
            if location.is_empty() {
                location = s(&n, &["location_id"]);
            }
            notams.push(Notam { id, location, text: body });
        }
    }
    let log = list(v.get("navlog").and_then(|n| n.get("fix")))
        .iter()
        .map(|f| LogRow {
            ident: s(f, &["ident"]),
            airway: s(f, &["via_airway"]),
            stage: s(f, &["stage"]),
            alt_ft: n(f, &["altitude_feet"]),
            track_mag: n(f, &["track_mag"]),
            dist_leg: n(f, &["distance"]),
            time_total: n(f, &["time_total"]),
            fuel_onboard: n(f, &["fuel_plan_onboard"]),
            wind_dir: n(f, &["wind_dir"]),
            wind_kt: n(f, &["wind_spd"]),
            oat: n(f, &["oat"]),
            tas: n(f, &["true_airspeed"]),
            gs: n(f, &["groundspeed"]),
            mach: s(f, &["mach"]),
            winds: list(f.get("wind_data").and_then(|d| d.get("level")))
                .iter()
                .map(|l| [n(l, &["altitude"]), n(l, &["wind_dir"]), n(l, &["wind_spd"]), n(l, &["oat"])])
                .collect(),
        })
        .collect();
    Ofp {
        html: s(v, &["text", "plan_html"]),
        units: s(v, &["params", "units"]).to_lowercase(),
        fpl: s(v, &["atc", "flightplan_text"]),
        times: flat(v.get("times")),
        weights: flat(v.get("weights")),
        fuel: flat(v.get("fuel")),
        weather,
        notams,
        log,
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
        ofp: Some(ofp(v)),
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
    fn reads_a_plan_typed_in_the_app() {
        // What the Start a flight form sends for a hand-made plan.
        let json = r#"{"source":"manual","id":"manual-1","generated_ms":1793462094467,"callsign":"DAL864",
            "airline":"","flight_number":"DL864",
            "origin":{"icao":"KLAS","name":"Harry Reid","lat":36.084,"lon":-115.1537,"runway":""},
            "destination":{"icao":"KDTW","name":"Detroit","lat":42.2124,"lon":-83.3534,"runway":""},
            "alternate":"KCLE","route":"","cruise_ft":0,"distance_nm":1529,"ete_min":0,"aircraft_icao":"B753",
            "aircraft_name":"Boeing 757-300","registration":"N581NW","block_fuel_kg":0,"sched_out_ms":null,
            "sched_in_ms":null,"fixes":[],"ofp":null}"#;
        let p: FlightPlan = serde_json::from_str(json).unwrap();
        assert_eq!((p.source.as_str(), p.destination.icao.as_str(), p.alternate.as_str()), ("manual", "KDTW", "KCLE"));
    }

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
        let o = p.ofp.unwrap();
        assert_eq!(o.units, "lbs");
        assert_eq!(o.log.len(), 2);
        assert_eq!(o.times.get("est_time_enroute").and_then(|x| x.as_str()), Some("3720"));
    }

    #[test]
    fn briefing_sections() {
        let v: Value = serde_json::json!({
            "origin": { "icao_code": "LFMN", "notam": [
                { "notam_id": "A1", "location_icao": "LFMN", "notam_text": "RWY 04L CLSD" },
                { "notam_id": "A1", "location_icao": "LFMN", "notam_text": "duplicate" }
            ]},
            "destination": { "icao_code": "LIRQ", "notam": { "notam_id": "B2", "location_icao": "LIRQ", "notam_raw": "ILS U/S" } },
            "atc": { "flightplan_text": "(FPL-AUA11-IS -A320/M -LFMN0010 -N0450F260 DCT -LIRQ0037)" },
            "weather": { "orig_metar": "LFMN 280020Z 33010KT CAVOK 15/08 Q1020", "dest_taf": ["TAF LIRQ 1", "BECMG 2"] },
            "text": { "plan_html": "<div><pre><b>[ OFP ]</b></pre></div>" },
            "navlog": { "fix": [ { "ident": "BASIP", "pos_lat": "43.7", "pos_long": "7.6",
                "wind_data": { "level": [ { "altitude": "26000", "wind_dir": "290", "wind_spd": "18", "oat": "-27" } ] } } ] }
        });
        let o = parse_ofp(&v).unwrap().ofp.unwrap();
        assert_eq!(o.notams.len(), 2);
        assert_eq!(o.notams[1].text, "ILS U/S");
        assert!(o.fpl.starts_with("(FPL-AUA11"));
        assert_eq!(o.weather.len(), 2);
        assert_eq!(o.weather[1].1, "TAF LIRQ 1\nBECMG 2");
        assert_eq!(o.log[0].winds, vec![[26000.0, 290.0, 18.0, -27.0]]);
        assert!(o.html.contains("[ OFP ]"));
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
