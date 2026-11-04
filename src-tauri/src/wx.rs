//! Aviation weather on the map, from NOAA's Aviation Weather Center
//! (aviationweather.gov, US government data, public domain, no key):
//!
//! - flight category dots: every current METAR worldwide (the centre's
//!   "cache" file, refreshed every minute; FlightDeck takes it at most every
//!   10 minutes while the layer is on);
//! - SIGMETs (international and US, including convective) and US G-AIRMETs,
//!   as shapes.
//!
//! The centre limits clients to 100 requests a minute; FlightDeck makes a
//! handful an hour.

use std::io::Read;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Map, Value};

const METARS: &str = "https://aviationweather.gov/data/cache/metars.cache.csv.gz";
const ISIGMET: &str = "https://aviationweather.gov/api/data/isigmet?format=geojson";
const AIRSIGMET: &str = "https://aviationweather.gov/api/data/airsigmet?format=geojson";
const GAIRMET: &str = "https://aviationweather.gov/api/data/gairmet?format=geojson";
const REFRESH: Duration = Duration::from_secs(600);

/// One station's flight category: [code, lon, lat, category] with category
/// 0 VFR, 1 MVFR, 2 IFR, 3 LIFR.
pub type CategoryDot = (String, f32, f32, u8);

#[derive(Debug, Clone, Serialize, Default)]
pub struct Categories {
    pub updated_ms: i64,
    pub stations: Vec<CategoryDot>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Advisories {
    pub updated_ms: i64,
    /// GeoJSON features with FlightDeck's own properties: kind (SIGMET,
    /// AIRMET or CONVECTIVE), group (TS, TURB, ICE, IFR, VA, TC, OTHER),
    /// hazard, name, from_ms, to_ms, base, top, raw.
    pub features: Vec<Value>,
    pub error: Option<String>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn get(url: &str) -> Result<Vec<u8>, String> {
    let mut res = crate::update::agent().get(url).call().map_err(|e| format!("Could not reach aviationweather.gov: {e}"))?;
    res.body_mut()
        .with_config()
        .limit(64 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| format!("Unexpected answer from aviationweather.gov: {e}"))
}

// --- flight categories -----------------------------------------------------

/// Reads the METAR cache CSV (a few lines of notes, then a header row).
fn parse_metar_csv(text: &str) -> Vec<CategoryDot> {
    let mut lines = text.lines().skip_while(|l| !l.starts_with("raw_text,"));
    let Some(header) = lines.next() else { return vec![] };
    let cols: Vec<&str> = header.split(',').collect();
    let at = |name: &str| cols.iter().position(|c| *c == name);
    let (Some(id), Some(la), Some(lo), Some(cat)) = (at("station_id"), at("latitude"), at("longitude"), at("flight_category")) else {
        return vec![];
    };
    lines
        .filter_map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            let c = match *f.get(cat)? {
                "VFR" => 0,
                "MVFR" => 1,
                "IFR" => 2,
                "LIFR" => 3,
                _ => return None,
            };
            Some((f.get(id)?.to_string(), f.get(lo)?.parse().ok()?, f.get(la)?.parse().ok()?, c))
        })
        .collect()
}

fn fetch_categories() -> Result<Vec<CategoryDot>, String> {
    let gz = get(METARS)?;
    let mut text = String::new();
    flate2::read::GzDecoder::new(&gz[..])
        .read_to_string(&mut text)
        .or_else(|_| String::from_utf8(gz.clone()).map(|t| text = t).map(|_| 0))
        .map_err(|e| format!("Unexpected METAR file: {e}"))?;
    Ok(parse_metar_csv(&text))
}

// --- SIGMETs and AIRMETs -----------------------------------------------------

/// The first of these properties that has a value, as text.
fn prop(p: &Map<String, Value>, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|k| match p.get(*k) {
            Some(Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
            Some(Value::Number(n)) => Some(n.to_string()),
            _ => None,
        })
        .unwrap_or_default()
}

/// A time given as Unix seconds or an ISO date, in milliseconds.
fn time_ms(p: &Map<String, Value>, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|k| match p.get(*k)? {
        Value::Number(n) => n.as_i64().map(|s| if s > 10_000_000_000 { s } else { s * 1000 }),
        Value::String(s) => crate::networks::iso_ms_pub(s).filter(|v| *v > 0),
        _ => None,
    })
}

/// Which colour family a hazard belongs to.
fn group(hazard: &str) -> &'static str {
    let h = hazard.to_ascii_uppercase();
    if h.contains("TS") || h.contains("CONV") || h.contains("THUNDER") || h.contains("CB") {
        "TS"
    } else if h.contains("TURB") || h.contains("LLWS") || h.contains("WND") || h.contains("MTW") {
        "TURB"
    } else if h.contains("ICE") || h.contains("ICING") || h.contains("FZLVL") {
        "ICE"
    } else if h.contains("IFR") || h.contains("MT_OBSC") || h.contains("OBSC") || h.contains("MTN") {
        "IFR"
    } else if h.contains("VA") || h.contains("ASH") {
        "VA"
    } else if h.contains("TC") || h.contains("TROP") || h.contains("CYCLONE") {
        "TC"
    } else {
        "OTHER"
    }
}

/// Our own properties for a feature from one of the centre's feeds.
fn normalise(f: &Value, source: &str) -> Option<Value> {
    let geom = f.get("geometry")?;
    if !matches!(geom["type"].as_str(), Some("Polygon" | "MultiPolygon")) {
        return None;
    }
    let p = f.get("properties").and_then(Value::as_object).cloned().unwrap_or_default();
    // G-AIRMETs come as a series of forecasts: keep the current one.
    let hour = prop(&p, &["forecastHour", "forecast", "fcstHour", "forecast_hour"]);
    if source == "gairmet" && hour.parse::<f64>().is_ok_and(|h| h != 0.0) {
        return None;
    }
    let hazard = prop(&p, &["hazard", "type", "tag"]);
    let kind = match source {
        "gairmet" => "AIRMET",
        "airsigmet" => {
            let t = prop(&p, &["airSigmetType", "type"]).to_ascii_uppercase();
            if t.contains("AIRMET") {
                "AIRMET"
            } else if t.contains("OUTLOOK") {
                return None;
            } else if group(&hazard) == "TS" {
                "CONVECTIVE"
            } else {
                "SIGMET"
            }
        }
        _ => "SIGMET",
    };
    let qualifier = prop(&p, &["qualifier", "severity"]);
    let name = {
        let fir = prop(&p, &["firName", "firId", "icaoId"]);
        let series = prop(&p, &["seriesId", "alphaChar", "tag"]);
        [fir, series].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
    };
    let props = json!({
        "kind": kind,
        "group": group(&hazard),
        "hazard": hazard,
        "qualifier": qualifier,
        "name": name,
        "from_ms": time_ms(&p, &["validTimeFrom", "validTime", "issueTime", "receiptTime"]),
        "to_ms": time_ms(&p, &["validTimeTo", "expireTime"]),
        "base": prop(&p, &["base", "bottom", "altitudeLow1", "altLo"]),
        "top": prop(&p, &["top", "altitudeHi1", "altHi"]),
        "raw": prop(&p, &["rawSigmet", "rawAirSigmet", "rawText", "raw"]),
        "due_to": prop(&p, &["dueTo", "due_to"]),
    });
    Some(json!({ "type": "Feature", "properties": props, "geometry": geom }))
}

fn fetch_advisories() -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    for (url, source) in [(ISIGMET, "isigmet"), (AIRSIGMET, "airsigmet"), (GAIRMET, "gairmet")] {
        match get(url).and_then(|b| serde_json::from_slice::<Value>(&b).map_err(|e| e.to_string())) {
            Ok(v) => out.extend(v["features"].as_array().into_iter().flatten().filter_map(|f| normalise(f, source))),
            Err(e) => errors.push(e),
        }
    }
    if out.is_empty() && !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok(out)
}

// --- cached for the map -----------------------------------------------------------

struct Cached<T> {
    at: Option<Instant>,
    data: T,
}

fn categories_cache() -> &'static Mutex<Cached<Categories>> {
    static C: OnceLock<Mutex<Cached<Categories>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(Cached { at: None, data: Categories::default() }))
}

fn advisories_cache() -> &'static Mutex<Cached<Advisories>> {
    static C: OnceLock<Mutex<Cached<Advisories>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(Cached { at: None, data: Advisories::default() }))
}

/// Flight categories of every reporting station, at most 10 minutes old.
pub fn categories() -> Categories {
    {
        let c = categories_cache().lock().unwrap();
        if c.at.is_some_and(|t| t.elapsed() < REFRESH) {
            return c.data.clone();
        }
    }
    let mut c = categories_cache().lock().unwrap();
    match fetch_categories() {
        Ok(stations) => {
            c.data = Categories { updated_ms: now_ms(), stations, error: None };
            c.at = Some(Instant::now());
        }
        Err(e) => {
            log::warn!("flight categories: {e}");
            c.data.error = Some(e);
        }
    }
    c.data.clone()
}

/// Current SIGMETs and AIRMETs, at most 10 minutes old.
pub fn advisories() -> Advisories {
    {
        let c = advisories_cache().lock().unwrap();
        if c.at.is_some_and(|t| t.elapsed() < REFRESH) {
            return c.data.clone();
        }
    }
    let mut c = advisories_cache().lock().unwrap();
    match fetch_advisories() {
        Ok(features) => {
            c.data = Advisories { updated_ms: now_ms(), features, error: None };
            c.at = Some(Instant::now());
        }
        Err(e) => {
            log::warn!("SIGMETs and AIRMETs: {e}");
            c.data.error = Some(e);
        }
    }
    c.data.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_metar_cache() {
        let csv = "No errors\nNo warnings\n12 ms\ndata source=metars\n3 results\n\
            raw_text,station_id,observation_time,latitude,longitude,temp_c,flight_category,metar_type\n\
            KSEA 040053Z 03004KT 10SM,KSEA,2026-11-04T00:53:00Z,47.44,-122.31,8,VFR,METAR\n\
            KPAE 040053Z 00000KT 2SM BR,KPAE,2026-11-04T00:53:00Z,47.9,-122.28,7,IFR,METAR\n\
            XXXX 040053Z,XXXX,2026-11-04T00:53:00Z,1,2,3,,METAR\n";
        let dots = parse_metar_csv(csv);
        assert_eq!(dots.len(), 2);
        assert_eq!(dots[0], ("KSEA".to_string(), -122.31, 47.44, 0));
        assert_eq!(dots[1].3, 2);
    }

    #[test]
    fn keeps_shapes_with_our_properties() {
        let square = json!({"type": "Polygon", "coordinates": [[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]]});
        let isig = json!({"type": "Feature", "geometry": square, "properties": {"firName": "SHANWICK OCEANIC", "seriesId": "A3",
            "hazard": "TURB", "qualifier": "SEV", "validTimeFrom": 1793700000, "validTimeTo": 1793714400, "base": 30000, "top": 38000,
            "rawSigmet": "EGGX SIGMET A3 VALID"}});
        let f = normalise(&isig, "isigmet").unwrap();
        assert_eq!(f["properties"]["group"], "TURB");
        assert_eq!(f["properties"]["kind"], "SIGMET");
        assert_eq!(f["properties"]["to_ms"], 1_793_714_400_000i64);
        assert_eq!(f["properties"]["top"], "38000");
        let conv = json!({"type": "Feature", "geometry": square, "properties": {"airSigmetType": "SIGMET", "hazard": "CONVECTIVE"}});
        assert_eq!(normalise(&conv, "airsigmet").unwrap()["properties"]["kind"], "CONVECTIVE");
        let later = json!({"type": "Feature", "geometry": square, "properties": {"hazard": "ICE", "forecastHour": 6}});
        assert!(normalise(&later, "gairmet").is_none());
        let now = json!({"type": "Feature", "geometry": square, "properties": {"hazard": "ICE", "forecastHour": 0}});
        assert_eq!(normalise(&now, "gairmet").unwrap()["properties"]["group"], "ICE");
        let point = json!({"type": "Feature", "geometry": {"type": "Point", "coordinates": [0, 0]}, "properties": {}});
        assert!(normalise(&point, "isigmet").is_none());
    }
}
