//! Airports on the map: the dots for the detail level chosen, the card an
//! airport opens (runways, frequencies, your visits) and its weather (the
//! latest METAR and TAF from aviationweather.gov, fetched only when a card
//! is opened).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use refdata::{AirportKind, Frequency, RefData, RunwayInfo};
use serde::Serialize;
use serde_json::Value;
use store::FlightRow;

/// Kinds shown at each detail level: 1 major, 2 adds regional, 3 adds small
/// airports and strips.
fn kinds(detail: u8) -> &'static [AirportKind] {
    match detail {
        0 | 1 => &[AirportKind::Large],
        2 => &[AirportKind::Large, AirportKind::Medium],
        _ => &[AirportKind::Large, AirportKind::Medium, AirportKind::Small],
    }
}

fn kind_rank(k: AirportKind) -> u8 {
    match k {
        AirportKind::Large => 0,
        AirportKind::Medium => 1,
        _ => 2,
    }
}

/// [ident, name, rank (0 major, 1 regional, 2 small), lon, lat] for each airport.
pub fn map_airports(rd: &RefData, detail: u8) -> Vec<(String, String, u8, f32, f32)> {
    rd.airports_of(kinds(detail))
        .map(|a| (a.ident.clone(), a.name.clone(), kind_rank(a.kind), a.lon as f32, a.lat as f32))
        .collect()
}

#[derive(Debug, Serialize)]
pub struct Visit {
    pub id: i64,
    pub dep_icao: Option<String>,
    pub arr_icao: Option<String>,
    pub start_ms: i64,
    pub icao_type: String,
    pub landing_rate_fpm: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct AirportCard {
    pub ident: String,
    pub name: String,
    /// "Major", "Regional", "Small", "Seaplane base", "Heliport".
    pub kind: &'static str,
    pub lat: f64,
    pub lon: f64,
    pub elevation_ft: i32,
    pub city: String,
    pub iata: String,
    pub country: String,
    pub country_name: String,
    pub region_name: String,
    pub runways: Vec<RunwayInfo>,
    pub frequencies: Vec<Frequency>,
    pub departures: usize,
    pub arrivals: usize,
    pub last_visit_ms: Option<i64>,
    /// The softest landing here (closest to 0 fpm).
    pub best_landing_fpm: Option<f64>,
    /// The latest few flights from or to here.
    pub recent: Vec<Visit>,
}

pub fn card(rd: &RefData, flights: &[FlightRow], ident: &str) -> Option<AirportCard> {
    let a = rd.airport(ident)?;
    let id = a.ident.as_str();
    let is = |v: &Option<String>| v.as_deref() == Some(id);
    let arrivals: Vec<&FlightRow> = flights.iter().filter(|f| is(&f.arr_icao)).collect();
    Some(AirportCard {
        ident: a.ident.clone(),
        name: a.name.clone(),
        kind: match a.kind {
            AirportKind::Large => "Major",
            AirportKind::Medium => "Regional",
            AirportKind::Small => "Small",
            AirportKind::Seaplane => "Seaplane base",
            AirportKind::Heliport => "Heliport",
        },
        lat: a.lat,
        lon: a.lon,
        elevation_ft: a.elevation_ft,
        city: a.city.clone(),
        iata: a.iata.clone(),
        country: a.country.clone(),
        country_name: rd.country_name(&a.country).unwrap_or(&a.country).to_string(),
        region_name: rd.region_name(&a.region).unwrap_or("").to_string(),
        runways: rd.runways_of(id),
        frequencies: rd.frequencies_of(id),
        departures: flights.iter().filter(|f| is(&f.dep_icao)).count(),
        arrivals: arrivals.len(),
        last_visit_ms: flights.iter().map(|f| f.start_ms).max(),
        best_landing_fpm: arrivals
            .iter()
            .filter_map(|f| f.landing_rate_fpm)
            .filter(|v| *v < 0.0)
            .max_by(|a, b| a.total_cmp(b)),
        recent: flights
            .iter()
            .take(5)
            .map(|f| Visit {
                id: f.id,
                dep_icao: f.dep_icao.clone(),
                arr_icao: f.arr_icao.clone(),
                start_ms: f.start_ms,
                icao_type: f.icao_type.clone(),
                landing_rate_fpm: f.landing_rate_fpm,
            })
            .collect(),
    })
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Weather {
    /// The reporting station: the airport itself or the nearest one that reports.
    pub station: String,
    pub station_name: String,
    /// 0 when the airport reports itself.
    pub distance_nm: f64,
    pub metar: String,
    pub observed_ms: Option<i64>,
    /// "VFR", "MVFR", "IFR" or "LIFR".
    pub category: Option<String>,
    /// Degrees true; None when variable or calm.
    pub wind_dir: Option<f64>,
    pub wind_kt: Option<f64>,
    pub gust_kt: Option<f64>,
    /// Statute miles; "10+" style values come as 10.
    pub visibility_sm: Option<f64>,
    /// Lowest broken or overcast layer, feet above the ground.
    pub ceiling_ft: Option<f64>,
    pub temp_c: Option<f64>,
    pub dewpoint_c: Option<f64>,
    pub altimeter_hpa: Option<f64>,
    pub taf: Option<String>,
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim_end_matches('+').parse().ok(),
        _ => None,
    }
}

/// Flight category from visibility (statute miles) and ceiling (feet).
fn category(vis: Option<f64>, ceiling: Option<f64>) -> Option<String> {
    if vis.is_none() && ceiling.is_none() {
        return None;
    }
    let v = vis.unwrap_or(10.0);
    let c = ceiling.unwrap_or(f64::INFINITY);
    Some(
        if v < 1.0 || c < 500.0 {
            "LIFR"
        } else if v < 3.0 || c < 1000.0 {
            "IFR"
        } else if v <= 5.0 || c <= 3000.0 {
            "MVFR"
        } else {
            "VFR"
        }
        .into(),
    )
}

fn parse_metar(m: &Value) -> Weather {
    let ceiling = m["clouds"].as_array().and_then(|layers| {
        layers
            .iter()
            .filter(|l| matches!(l["cover"].as_str(), Some("BKN" | "OVC" | "OVX" | "VV")))
            .filter_map(|l| num(&l["base"]))
            .reduce(f64::min)
    });
    // An obscured sky reports a vertical visibility instead.
    let ceiling = match (ceiling, num(&m["vertVis"])) {
        (Some(c), Some(v)) => Some(c.min(v)),
        (c, v) => c.or(v),
    };
    let vis = num(&m["visib"]);
    let category = m["fltCat"].as_str().filter(|s| !s.is_empty()).map(str::to_string).or_else(|| category(vis, ceiling));
    Weather {
        station: m["icaoId"].as_str().unwrap_or("").to_string(),
        station_name: m["name"].as_str().unwrap_or("").to_string(),
        metar: m["rawOb"].as_str().unwrap_or("").to_string(),
        observed_ms: m["obsTime"].as_i64().map(|s| s * 1000),
        category,
        wind_dir: num(&m["wdir"]),
        wind_kt: num(&m["wspd"]),
        gust_kt: num(&m["wgst"]),
        visibility_sm: vis,
        ceiling_ft: ceiling,
        temp_c: num(&m["temp"]),
        dewpoint_c: num(&m["dewp"]),
        altimeter_hpa: num(&m["altim"]),
        ..Default::default()
    }
}

fn fetch(kind: &str, ids: &[String]) -> Result<Vec<Value>, String> {
    let mut res = crate::update::agent()
        .get(format!("https://aviationweather.gov/api/data/{kind}"))
        .query("ids", ids.join(","))
        .query("format", "json")
        .call()
        .map_err(|e| format!("Could not reach aviationweather.gov: {e}"))?;
    if res.status() == 204 {
        return Ok(Vec::new());
    }
    let text = res.body_mut().read_to_string().map_err(|e| format!("Unexpected answer from aviationweather.gov: {e}"))?;
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("Unexpected answer from aviationweather.gov: {e}"))?;
    Ok(v.as_array().cloned().unwrap_or_default())
}

static CACHE: Mutex<Option<HashMap<String, (Instant, Weather)>>> = Mutex::new(None);

/// The latest METAR (and TAF) for an airport, or for the nearest airport
/// within 30 nm that reports when it does not. Kept for 5 minutes.
pub fn weather(rd: &RefData, ident: &str) -> Result<Option<Weather>, String> {
    let Some(a) = rd.airport(ident) else { return Ok(None) };
    if let Some((at, w)) = CACHE.lock().unwrap().get_or_insert_with(HashMap::new).get(&a.ident) {
        if at.elapsed() < Duration::from_secs(300) {
            return Ok(Some(w.clone()));
        }
    }
    let code = |s: &str| s.len() == 4 && s.chars().all(|c| c.is_ascii_alphanumeric());
    // The airport first, then the nearest airports with ICAO-style codes.
    let near = rd.near(a.lat, a.lon, 30.0);
    let mut ids: Vec<String> = Vec::new();
    if code(&a.ident) {
        ids.push(a.ident.clone());
    }
    for (n, _) in &near {
        if ids.len() >= 15 {
            break;
        }
        if code(&n.ident) && !ids.contains(&n.ident) && n.kind != AirportKind::Heliport {
            ids.push(n.ident.clone());
        }
    }
    if ids.is_empty() {
        return Ok(None);
    }
    let reports = fetch("metar", &ids)?;
    // The airport itself if it reports, else the nearest that does.
    let found = near
        .iter()
        .find_map(|(n, d)| reports.iter().find(|m| m["icaoId"].as_str() == Some(n.ident.as_str())).map(|m| (m, *d)));
    let Some((m, dist)) = found else { return Ok(None) };
    let mut w = parse_metar(m);
    w.distance_nm = if w.station == a.ident { 0.0 } else { (dist * 10.0).round() / 10.0 };
    if w.station_name.is_empty() {
        w.station_name = rd.airport(&w.station).map(|s| s.name.clone()).unwrap_or_default();
    }
    w.taf = fetch("taf", std::slice::from_ref(&w.station))
        .ok()
        .and_then(|t| t.first().and_then(|t| t["rawTAF"].as_str()).map(str::to_string));
    CACHE.lock().unwrap().get_or_insert_with(HashMap::new).insert(a.ident.clone(), (Instant::now(), w.clone()));
    Ok(Some(w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_aviationweather_metar() {
        let m: Value = serde_json::from_str(
            r#"{"icaoId":"KSEA","obsTime":1793740380,"temp":9.4,"dewp":6.1,"wdir":"VRB","wspd":4,"visib":"10+",
                "altim":1016.2,"rawOb":"KSEA 032153Z VRB04KT 10SM FEW020 BKN045 09/06 A3001","name":"Seattle-Tacoma Intl, WA, US",
                "clouds":[{"cover":"FEW","base":2000},{"cover":"BKN","base":4500}]}"#,
        )
        .unwrap();
        let w = parse_metar(&m);
        assert_eq!(w.station, "KSEA");
        assert_eq!(w.wind_dir, None);
        assert_eq!(w.wind_kt, Some(4.0));
        assert_eq!(w.visibility_sm, Some(10.0));
        assert_eq!(w.ceiling_ft, Some(4500.0));
        assert_eq!(w.category.as_deref(), Some("VFR"));
        assert_eq!(w.observed_ms, Some(1_793_740_380_000));
    }

    #[test]
    fn flight_categories() {
        assert_eq!(category(Some(10.0), Some(2500.0)).as_deref(), Some("MVFR"));
        assert_eq!(category(Some(2.0), None).as_deref(), Some("IFR"));
        assert_eq!(category(Some(0.5), Some(5000.0)).as_deref(), Some("LIFR"));
        assert_eq!(category(None, None), None);
    }

    #[test]
    fn detail_levels_add_smaller_airports() {
        let rd = RefData::load();
        let (one, two, three) = (map_airports(&rd, 1).len(), map_airports(&rd, 2).len(), map_airports(&rd, 3).len());
        assert!(one > 500 && one < two && two < three, "{one} {two} {three}");
        let c = card(&rd, &[], "KSEA").unwrap();
        assert_eq!(c.kind, "Major");
        assert!(c.runways.len() >= 3 && c.runways[0].length_ft > 11000, "{:?}", c.runways);
        assert!(c.frequencies.iter().any(|f| f.kind == "TWR"));
    }
}
