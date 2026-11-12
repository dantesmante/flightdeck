//! Airspace on the map.
//!
//! - US: the Class B, C and D areas around airports, from the FAA's NASR
//!   28-day subscription ("class_airspace_shape_files.zip"), and special use
//!   airspace (prohibited, restricted, MOAs, warning and alert areas) from
//!   the FAA's ADDS open data. US government data, public domain, no key.
//! - Elsewhere: openAIP's per-country exports (CC BY-NC 4.0, attribution
//!   "openAIP"; FlightDeck is free, which the licence requires), read only
//!   for the countries in view.
//!
//! Every file is fetched once per 28-day cycle, simplified, and saved as
//! GeoJSON in FlightDeck's data folder; the map reads that copy until the
//! next cycle. Every feature has the same properties: c (kind: "B", "C",
//! "D", "E", "G"…, or "P", "R", "DNG", "W", "AL", "MOA", "TRA"…), name,
//! label (as the user asked for: "G: GND-2000AGL"), lo and hi (feet, for
//! the 3D view; hi 60000 when unlimited).

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{json, Value};
use shapefile::dbase;

/// A NASR cycle's effective date (Unix ms, 00:00 UTC). Cycles follow every
/// 28 days.
const ANCHOR_MS: i64 = 1_793_232_000_000; // 2026-10-29
const CYCLE_MS: i64 = 28 * 86_400_000;
/// Douglas-Peucker tolerance in degrees (about 20 m), well inside how
/// exactly the boundaries are published.
const SIMPLIFY_DEG: f64 = 0.0002;

fn url(cycle: &str) -> String {
    format!("https://nfdc.faa.gov/webContent/28DaySub/{cycle}/class_airspace_shape_files.zip")
}

/// The effective date of the cycle in force at a time, "YYYY-MM-DD".
pub fn cycle_at(ms: i64) -> String {
    let start = ANCHOR_MS + (ms - ANCHOR_MS).div_euclid(CYCLE_MS) * CYCLE_MS;
    crate::volanta::format_time(start, false)[0..10].to_string()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn folder(data_dir: &Path) -> PathBuf {
    data_dir.join("airspace")
}

/// A cycle's copy of one file (`{prefix}-{cycle}.json`), else made now with
/// `make`, else the last copy kept however old (offline). Older copies with
/// the same prefix are removed.
fn cached(data_dir: &Path, prefix: &str, make: impl Fn(&str) -> Result<String, String>) -> Result<String, String> {
    let dir = folder(data_dir);
    let now = now_ms();
    let cycle = cycle_at(now);
    if let Ok(s) = std::fs::read_to_string(dir.join(format!("{prefix}-{cycle}.json"))) {
        return Ok(s);
    }
    let mut last_err = String::new();
    // This cycle, or (just after the change, should the source be late) the last.
    for c in [cycle.clone(), cycle_at(now - CYCLE_MS)] {
        match make(&c) {
            Ok(json) => {
                let _ = std::fs::create_dir_all(&dir);
                let out = dir.join(format!("{prefix}-{c}.json"));
                if std::fs::write(&out, &json).is_ok() {
                    forget_others(&dir, prefix, &out);
                }
                log::info!("airspace: {prefix} cycle {c}, {} KB", json.len() / 1024);
                return Ok(json);
            }
            Err(e) => last_err = e,
        }
    }
    if let Some(old) = newest(&dir, prefix) {
        log::warn!("airspace: {last_err}; showing {}", old.display());
        return std::fs::read_to_string(old).map_err(|e| e.to_string());
    }
    Err(last_err)
}

/// One load at a time: a second ask waits for the first download.
static BUSY: Mutex<()> = Mutex::new(());

/// The US Class B, C and D areas as GeoJSON.
pub fn us(data_dir: &Path) -> Result<String, String> {
    let _one = BUSY.lock().unwrap_or_else(|e| e.into_inner());
    cached(data_dir, "class2", |c| from_zip(&fetch(c)?, c))
}

/// US special use airspace as GeoJSON: prohibited, restricted, MOAs,
/// warning, alert and national security areas.
pub fn sua(data_dir: &Path) -> Result<String, String> {
    static ONE: Mutex<()> = Mutex::new(());
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    cached(data_dir, "sua2", |c| {
        let mut features = Vec::new();
        let mut offset = 0;
        // The service hands out a page at a time.
        loop {
            let url = format!("{SUA_URL}&resultOffset={offset}&resultRecordCount=1000");
            let v: Value = serde_json::from_str(&get_text(&url, "the FAA (ADDS)")?).map_err(|e| format!("Unexpected answer from the FAA: {e}"))?;
            let page = v["features"].as_array().cloned().unwrap_or_default();
            let n = page.len();
            features.extend(page.iter().filter_map(sua_feature));
            let more = v["exceededTransferLimit"].as_bool().unwrap_or(false) || v["properties"]["exceededTransferLimit"].as_bool().unwrap_or(false);
            if n == 0 || !more || offset > 20_000 {
                break;
            }
            offset += n;
        }
        if features.is_empty() {
            return Err("The FAA's special use airspace came back empty".into());
        }
        Ok(json!({ "type": "FeatureCollection", "cycle": c, "features": features }).to_string())
    })
}

/// Which US airports are private use, as the FAA lists them (its airport
/// list marks each one PRIVATEUSE 0 or 1): the codes of every listed
/// airport ("SEA", "KSEA" and the like) in `public` or `private`. Kept a
/// cycle like the airspace.
pub fn us_airport_use(data_dir: &Path) -> Result<String, String> {
    static ONE: Mutex<()> = Mutex::new(());
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    cached(data_dir, "usapt1", |c| {
        let mut public: Vec<String> = Vec::new();
        let mut private: Vec<String> = Vec::new();
        let mut offset = 0;
        loop {
            let url = format!("{US_AIRPORT_URL}&resultOffset={offset}&resultRecordCount=2000");
            let v: Value = serde_json::from_str(&get_text(&url, "the FAA (ADDS)")?).map_err(|e| format!("Unexpected answer from the FAA: {e}"))?;
            let page = v["features"].as_array().cloned().unwrap_or_default();
            let n = page.len();
            for f in &page {
                let a = if f["attributes"].is_object() { &f["attributes"] } else { &f["properties"] };
                let codes = airport_codes(a["IDENT"].as_str().unwrap_or(""), a["ICAO_ID"].as_str().unwrap_or(""));
                if a["PRIVATEUSE"].as_i64() == Some(1) { private.extend(codes) } else { public.extend(codes) }
            }
            let more = v["exceededTransferLimit"].as_bool().unwrap_or(false) || v["properties"]["exceededTransferLimit"].as_bool().unwrap_or(false);
            if n == 0 || !more || offset > 60_000 {
                break;
            }
            offset += n;
        }
        if public.is_empty() {
            return Err("The FAA's airport list came back empty".into());
        }
        log::info!("airports: FAA list: {} public-use codes, {} private-use codes", public.len(), private.len());
        Ok(json!({ "cycle": c, "public": public, "private": private }).to_string())
    })
}

/// The codes an airport may go by: the FAA ident ("SEA", "63CN"), its ICAO
/// code, and K + a three-letter ident (as most US airports are known).
fn airport_codes(ident: &str, icao: &str) -> Vec<String> {
    let mut v = Vec::new();
    for s in [ident.trim(), icao.trim()] {
        if !s.is_empty() {
            v.push(s.to_ascii_uppercase());
        }
    }
    let id = ident.trim();
    if id.len() == 3 && id.chars().all(|c| c.is_ascii_alphabetic()) {
        v.push(format!("K{}", id.to_ascii_uppercase()));
    }
    v.sort();
    v.dedup();
    v
}

const US_AIRPORT_URL: &str = "https://services6.arcgis.com/ssFJjBXIUyZDrSYZ/arcgis/rest/services/US_Airport/FeatureServer/0/query?where=1%3D1&outFields=IDENT,ICAO_ID,PRIVATEUSE&returnGeometry=false&f=json";

/// The FAA's special use airspace, as GeoJSON, simplified by the service
/// (about 50 m) and rounded.
const SUA_URL: &str = "https://services6.arcgis.com/ssFJjBXIUyZDrSYZ/arcgis/rest/services/Special_Use_Airspace/FeatureServer/0/query?where=1%3D1&outFields=NAME,TYPE_CODE,UPPER_VAL,UPPER_UOM,UPPER_CODE,LOWER_VAL,LOWER_UOM,LOWER_CODE&returnGeometry=true&maxAllowableOffset=0.0005&geometryPrecision=5&outSR=4326&f=geojson";

/// For big files: no limit on the whole download (a country's airspace can
/// be tens of MB on a slow line), only on connecting and on silence.
fn big_agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .proxy(ureq::Proxy::try_from_env())
        .timeout_connect(Some(std::time::Duration::from_secs(20)))
        .timeout_recv_response(Some(std::time::Duration::from_secs(60)))
        .timeout_recv_body(Some(std::time::Duration::from_secs(600)))
        .user_agent(concat!("FlightDeck/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// A web page's text (for other modules: the FAA's fix lookups).
pub fn fetch_text(url: &str, who: &str) -> Result<String, String> {
    get_text(url, who)
}

fn get_text(url: &str, who: &str) -> Result<String, String> {
    let t = std::time::Instant::now();
    let mut res = big_agent().get(url).call().map_err(|e| format!("Could not reach {who}: {e}"))?;
    log::info!("airspace: {url} answered in {:.1} s", t.elapsed().as_secs_f32());
    res.body_mut().with_config().limit(100 * 1024 * 1024).read_to_string().map_err(|e| format!("Unexpected answer from {who}: {e}"))
}

fn val_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.trim().to_string()).filter(|s| !s.is_empty()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn sua_feature(f: &Value) -> Option<Value> {
    let p = &f["properties"];
    let kind = match val_str(&p["TYPE_CODE"])?.to_ascii_uppercase().as_str() {
        "P" => "P",
        "R" => "R",
        "MOA" => "MOA",
        "W" => "W",
        "A" => "AL",
        "NSA" => "NSA",
        _ => return None,
    };
    let name = val_str(&p["NAME"]).unwrap_or_default();
    let lo = faa_limit(val_str(&p["LOWER_VAL"]), val_str(&p["LOWER_UOM"]), val_str(&p["LOWER_CODE"]));
    let hi = faa_limit(val_str(&p["UPPER_VAL"]), val_str(&p["UPPER_UOM"]), val_str(&p["UPPER_CODE"]));
    let geometry = clean_geometry(&f["geometry"], SIMPLIFY_SUA)?;
    Some(feature(kind, &name, &sua_tag(kind, &name), lo, hi, geometry))
}

/// What a label starts with: the class letter, or the area's own short name
/// ("R-2508", "W-122"), else its kind.
fn sua_tag(kind: &str, name: &str) -> String {
    let short = name.split_whitespace().next().unwrap_or("");
    if !short.is_empty() && short.len() <= 10 && short.chars().any(|c| c.is_ascii_digit()) {
        short.to_string()
    } else if !name.is_empty() && name.len() <= 18 {
        name.to_string()
    } else {
        kind.to_string()
    }
}

/// A limit: its text as the label prints it, and feet for the 3D view.
#[derive(Clone, Debug, PartialEq)]
struct Limit {
    text: String,
    ft: f64,
}

/// "GND", "2000AGL", "10000MSL", "FL180", "UNL".
fn faa_limit(val: Option<String>, uom: Option<String>, code: Option<String>) -> Option<Limit> {
    let code = code.unwrap_or_default().to_ascii_uppercase();
    let raw = val.unwrap_or_default();
    if raw.eq_ignore_ascii_case("UNLTD") || raw.eq_ignore_ascii_case("UNL") || code == "UNLTD" {
        return Some(Limit { text: "UNL".into(), ft: 60_000.0 });
    }
    let v: f64 = raw.parse().ok()?;
    if v <= -9000.0 {
        return None; // not published (up to the airspace above)
    }
    if uom.as_deref().is_some_and(|u| u.eq_ignore_ascii_case("FL")) || code == "STD" {
        let fl = if v >= 1000.0 { v / 100.0 } else { v };
        return Some(Limit { text: format!("FL{:03}", fl.round() as i64), ft: fl * 100.0 });
    }
    if (code == "SFC" || code == "GND") && v <= 0.0 {
        return Some(Limit { text: "GND".into(), ft: 0.0 });
    }
    let ft = v.round();
    let datum = if code == "AGL" || code == "SFC" || code == "GND" { "AGL" } else { "MSL" };
    Some(Limit { text: format!("{}{datum}", ft as i64), ft })
}

fn label(tag: &str, lo: &Option<Limit>, hi: &Option<Limit>) -> String {
    match (lo, hi) {
        (Some(l), Some(h)) => format!("{tag}: {}-{}", l.text, h.text),
        (None, Some(h)) => format!("{tag}: -{}", h.text),
        (Some(l), None) => format!("{tag}: {}-", l.text),
        (None, None) => tag.to_string(),
    }
}

fn feature(kind: &str, name: &str, tag: &str, lo: Option<Limit>, hi: Option<Limit>, geometry: Value) -> Value {
    let label = label(tag, &lo, &hi);
    let lo_ft = lo.as_ref().map(|l| l.ft).unwrap_or(0.0);
    let hi_ft = hi.as_ref().map(|l| l.ft).unwrap_or(lo_ft + 1000.0).max(lo_ft + 300.0).min(60_000.0);
    let lo_t = lo.as_ref().map(|l| l.text.clone()).unwrap_or_default();
    let hi_t = hi.as_ref().map(|l| l.text.clone()).unwrap_or_default();
    json!({ "type": "Feature", "properties": { "c": kind, "name": name, "label": label, "lo": lo_ft.round(), "hi": hi_ft.round(), "lo_t": lo_t, "hi_t": hi_t }, "geometry": geometry })
}

/// A Polygon or MultiPolygon, simplified and rounded; None if nothing is left.
fn clean_geometry(g: &Value, tol: f64) -> Option<Value> {
    let ring = |r: &Value| -> Option<Vec<[f64; 2]>> {
        let pts: Vec<(f64, f64)> = r.as_array()?.iter().filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?))).collect();
        let out: Vec<[f64; 2]> = simplify(&pts, tol).into_iter().map(|(x, y)| [round5(x), round5(y)]).collect();
        (out.len() >= 4).then_some(out)
    };
    let poly = |p: &Value| -> Option<Vec<Vec<[f64; 2]>>> {
        let rings: Vec<Vec<[f64; 2]>> = p.as_array()?.iter().filter_map(ring).collect();
        (!rings.is_empty()).then_some(rings)
    };
    match g["type"].as_str()? {
        "Polygon" => Some(json!({ "type": "Polygon", "coordinates": poly(&g["coordinates"])? })),
        "MultiPolygon" => {
            let ps: Vec<_> = g["coordinates"].as_array()?.iter().filter_map(poly).collect();
            (!ps.is_empty()).then(|| json!({ "type": "MultiPolygon", "coordinates": ps }))
        }
        _ => None,
    }
}

const SIMPLIFY_SUA: f64 = 0.0004;

// ---- openAIP, outside the US -------------------------------------------------

/// openAIP's nightly keyless exports (moved here from a Google bucket that
/// became "requester pays" in July 2026 and answers anonymous reads with 400).
const OPENAIP: &str = "https://s3.openaip.net/openaip-system-exports";

/// One country's airspace, ready to answer for a box.
struct Country {
    /// Each area's box and its GeoJSON, kept as compact text (a parsed
    /// tree takes several times the memory).
    features: Vec<([f64; 4], Box<str>)>,
    /// Could not be read (offline, openAIP down): try again after a while.
    failed: Option<std::time::Instant>,
}

/// Each country read, and when it was last in view.
type Loaded = std::collections::HashMap<String, (std::sync::Arc<Country>, std::time::Instant)>;
static WORLD: Mutex<Option<Loaded>> = Mutex::new(None);

/// Countries out of view this long are let go from memory (read again from
/// the copy on disk, quickly, if they come back into view).
const FORGET_AFTER: std::time::Duration = std::time::Duration::from_secs(180);

/// Lets go of what has been out of view for a while: the ones in `keep`
/// are marked as just used.
fn forget_unused<V>(map: &mut std::collections::HashMap<String, (V, std::time::Instant)>, keep: &[String]) {
    let now = std::time::Instant::now();
    for cc in keep {
        if let Some(e) = map.get_mut(cc) {
            e.1 = now;
        }
    }
    map.retain(|_, (_, used)| now.duration_since(*used) < FORGET_AFTER);
}

/// The airspace outside the US in a box (countries are read as they come
/// into view, at most `max_new` new ones per call so a call stays quick).
/// Returns the GeoJSON and whether more countries in the box are still to
/// be read.
pub fn world(data_dir: &Path, w: f64, s: f64, e: f64, n: f64, max_new: usize) -> (String, bool) {
    let codes = countries_in(w, s, e, n);
    forget_unused(WORLD.lock().unwrap().get_or_insert_with(Default::default), &codes);
    let mut fresh = 0;
    let mut more = false;
    let mut list: Vec<std::sync::Arc<Country>> = Vec::new();
    for cc in codes {
        let have = WORLD
            .lock()
            .unwrap()
            .get_or_insert_with(Default::default)
            .get(&cc)
            .map(|e| e.0.clone())
            .filter(|c| c.failed.is_none_or(|t| t.elapsed() < std::time::Duration::from_secs(120)));
        let c = match have {
            Some(c) => c,
            None if fresh < max_new => {
                fresh += 1;
                let c = std::sync::Arc::new(load_country(data_dir, &cc));
                WORLD.lock().unwrap().get_or_insert_with(Default::default).insert(cc.clone(), (c.clone(), std::time::Instant::now()));
                c
            }
            None => {
                more = true;
                continue;
            }
        };
        list.push(c);
    }
    let mut out = String::from(r#"{"type":"FeatureCollection","features":["#);
    let mut count = 0;
    'all: for c in &list {
        for (b, f) in &c.features {
            if b[2] >= w && b[0] <= e && b[3] >= s && b[1] <= n {
                if count > 0 {
                    out.push(',');
                }
                out.push_str(f);
                count += 1;
                if count >= 6000 {
                    break 'all;
                }
            }
        }
    }
    out.push_str("]}");
    (out, more)
}

/// Country codes whose box meets the view (the view may cross the date
/// line), leaving out the US (its airspace comes from the FAA).
fn countries_in(w: f64, s: f64, e: f64, n: f64) -> Vec<String> {
    countries_in_all(w, s, e, n).into_iter().filter(|c| !crate::countries::FAA_COVERED.contains(&c.as_str())).collect()
}

/// Every country code whose box meets the view.
fn countries_in_all(w: f64, s: f64, e: f64, n: f64) -> Vec<String> {
    let spans: Vec<(f64, f64)> = if w <= e { vec![(w, e)] } else { vec![(w, 180.0), (-180.0, e)] };
    let mut out: Vec<String> = Vec::new();
    for (cc, b) in crate::countries::COUNTRY_BOXES {
        let (bw, bs, be, bn) = (b[0] as f64, b[1] as f64, b[2] as f64, b[3] as f64);
        if bn < s || bs > n {
            continue;
        }
        if spans.iter().any(|(a, z)| be >= *a && bw <= *z) && !out.iter().any(|x| x == cc) {
            out.push((*cc).to_string());
        }
    }
    out
}

fn load_country(data_dir: &Path, cc: &str) -> Country {
    // "oaip2": copies kept before the reader understood every export are not trusted.
    let json = cached(data_dir, &format!("oaip4-{cc}"), |_| {
        let url = format!("{OPENAIP}/{cc}_asp.geojson");
        let text = match get_text(&url, "openAIP") {
            Ok(t) => t,
            // No file for this country: remember that (an empty list) for the cycle.
            Err(e) if e.contains("404") => return Ok(r#"{"type":"FeatureCollection","features":[]}"#.into()),
            Err(e) => return Err(e),
        };
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("Unexpected answer from openAIP: {e}"))?;
        let all = v["features"].as_array().cloned().unwrap_or_default();
        let features: Vec<Value> = all.iter().filter_map(openaip_feature).collect();
        log::info!("airspace: openAIP {cc}: {} areas, {} kept", all.len(), features.len());
        if features.is_empty() && !all.is_empty() {
            // Nothing understood: don't keep an empty copy for the cycle, and
            // show what the file looks like so it can be fixed.
            let p = &all[0]["properties"];
            return Err(format!("openAIP {cc}: none of {} areas understood (first: {})", all.len(), p.to_string().chars().take(400).collect::<String>()));
        }
        Ok(json!({ "type": "FeatureCollection", "features": features }).to_string())
    });
    let mut failed = None;
    let features = match json {
        Ok(j) => serde_json::from_str::<Value>(&j)
            .ok()
            .and_then(|v| v["features"].as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|f| Some((bbox(&f["geometry"])?, f.to_string().into_boxed_str())))
            .collect(),
        Err(e) => {
            log::warn!("airspace: openAIP {cc}: {e}");
            failed = Some(std::time::Instant::now());
            Vec::new()
        }
    };
    Country { features, failed }
}

fn bbox(g: &Value) -> Option<[f64; 4]> {
    let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    fn walk(v: &Value, b: &mut [f64; 4]) {
        if let Some(a) = v.as_array() {
            if let (Some(x), Some(y)) = (a.first().and_then(Value::as_f64), a.get(1).and_then(Value::as_f64)) {
                b[0] = b[0].min(x);
                b[1] = b[1].min(y);
                b[2] = b[2].max(x);
                b[3] = b[3].max(y);
            } else {
                a.iter().for_each(|x| walk(x, b));
            }
        }
    }
    walk(&g["coordinates"], &mut b);
    (b[0] <= b[2]).then_some(b)
}

// ---- openAIP airports: private, fuel ---------------------------------------

/// What the chart symbols need that the world airport list lacks: whether
/// an airport is private and whether it sells fuel. One row per airport:
/// [lat, lon, ICAO code, private, fuel].
pub type AirportExtra = (f32, f32, String, bool, bool);

type AptEntry = (std::sync::Arc<Vec<AirportExtra>>, Option<std::time::Instant>);
type LoadedApt = std::collections::HashMap<String, (AptEntry, std::time::Instant)>;
static APTS: Mutex<Option<LoadedApt>> = Mutex::new(None);

/// openAIP's airports in a box (countries read as they come into view, at
/// most `max_new` new ones per call), and whether more are still to come.
pub fn airport_extras(data_dir: &Path, w: f64, s: f64, e: f64, n: f64, max_new: usize) -> (Vec<AirportExtra>, bool) {
    let mut out = Vec::new();
    let mut fresh = 0;
    let mut more = false;
    let codes = countries_in_all(w, s, e, n);
    forget_unused(APTS.lock().unwrap().get_or_insert_with(Default::default), &codes);
    for cc in codes {
        let have = APTS
            .lock()
            .unwrap()
            .get_or_insert_with(Default::default)
            .get(&cc)
            .map(|e| e.0.clone())
            .filter(|(_, failed)| failed.is_none_or(|t| t.elapsed() < std::time::Duration::from_secs(120)));
        let list = match have {
            Some((l, _)) => l,
            None if fresh < max_new => {
                fresh += 1;
                let (l, failed) = load_airports(data_dir, &cc);
                let l = std::sync::Arc::new(l);
                APTS.lock().unwrap().get_or_insert_with(Default::default).insert(cc.clone(), ((l.clone(), failed), std::time::Instant::now()));
                l
            }
            None => {
                more = true;
                continue;
            }
        };
        out.extend(list.iter().filter(|a| (a.0 as f64) >= s && (a.0 as f64) <= n && (a.1 as f64) >= w && (a.1 as f64) <= e).cloned());
    }
    (out, more)
}

fn truthy(v: &Value) -> bool {
    v.as_bool().unwrap_or(false) || v.as_i64().is_some_and(|x| x != 0) || v.as_str().is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "true" | "yes" | "1"))
}

fn openaip_airport(f: &Value) -> Option<AirportExtra> {
    let p = &f["properties"];
    let c = f["geometry"]["coordinates"].as_array()?;
    let (lon, lat) = (c.first()?.as_f64()?, c.get(1)?.as_f64()?);
    let icao = p["icaoCode"].as_str().or(p["icao"].as_str()).unwrap_or("").trim().to_ascii_uppercase();
    let private = truthy(&p["private"]);
    let fuel = p["services"]["fuelTypes"].as_array().is_some_and(|a| !a.is_empty()) || p["fuelTypes"].as_array().is_some_and(|a| !a.is_empty());
    Some((lat as f32, lon as f32, icao, private, fuel))
}

fn load_airports(data_dir: &Path, cc: &str) -> (Vec<AirportExtra>, Option<std::time::Instant>) {
    let json = cached(data_dir, &format!("oapt1-{cc}"), |_| {
        let url = format!("{OPENAIP}/{cc}_apt.geojson");
        let text = match get_text(&url, "openAIP") {
            Ok(t) => t,
            Err(e) if e.contains("404") => return Ok("[]".into()),
            Err(e) => return Err(e),
        };
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("Unexpected answer from openAIP: {e}"))?;
        let all = v["features"].as_array().cloned().unwrap_or_default();
        let rows: Vec<AirportExtra> = all.iter().filter_map(openaip_airport).collect();
        let (pv, fu) = (rows.iter().filter(|r| r.3).count(), rows.iter().filter(|r| r.4).count());
        log::info!("airports: openAIP {cc}: {} airports, {pv} private, {fu} with fuel", rows.len());
        if pv == 0 && fu == 0 && !all.is_empty() {
            log::info!("airports: openAIP {cc}: first airport's properties: {}", all[0]["properties"].to_string().chars().take(500).collect::<String>());
        }
        serde_json::to_string(&rows).map_err(|e| e.to_string())
    });
    match json.and_then(|j| serde_json::from_str::<Vec<AirportExtra>>(&j).map_err(|e| e.to_string())) {
        Ok(l) => (l, None),
        Err(e) => {
            log::warn!("airports: openAIP {cc}: {e}");
            (Vec::new(), Some(std::time::Instant::now()))
        }
    }
}

/// A number, or a number written as text.
fn num(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

/// openAIP's limit: value, unit (0 m, 1 ft, 6 FL) and reference datum
/// (0 ground, 1 MSL, 2 standard). Codes may also come as words ("FT",
/// "FL", "GND", "MSL", "STD") or the limit as text ("FL95", "2000ft AGL").
fn openaip_limit(l: &Value) -> Option<Limit> {
    if let Some(t) = l.as_str() {
        return text_limit(t);
    }
    let v = num(&l["value"])?;
    let code = |k: &str, words: &[(&str, i64)]| -> Option<i64> {
        num(&l[k]).map(|x| x as i64).or_else(|| {
            let w = l[k].as_str()?.trim().to_ascii_uppercase();
            words.iter().find(|(n, _)| *n == w).map(|(_, c)| *c)
        })
    };
    let unit = code("unit", &[("M", 0), ("METER", 0), ("METERS", 0), ("FT", 1), ("FEET", 1), ("FL", 6)]).unwrap_or(1);
    let datum = code("referenceDatum", &[("GND", 0), ("SFC", 0), ("AGL", 0), ("MSL", 1), ("STD", 2)]).unwrap_or(1);
    let ft = match unit {
        0 => v * 3.28084,
        6 => v * 100.0,
        _ => v,
    };
    if unit == 6 || datum == 2 {
        let fl = (ft / 100.0).round() as i64;
        return Some(Limit { text: format!("FL{fl:03}"), ft });
    }
    if datum == 0 {
        if ft <= 0.0 {
            return Some(Limit { text: "GND".into(), ft: 0.0 });
        }
        return Some(Limit { text: format!("{}AGL", ft.round() as i64), ft });
    }
    if ft >= 99_000.0 {
        return Some(Limit { text: "UNL".into(), ft: 60_000.0 });
    }
    Some(Limit { text: format!("{}MSL", ft.round() as i64), ft })
}

/// "GND", "SFC", "UNL", "FL95", "2000ft AGL", "4500 MSL".
fn text_limit(t: &str) -> Option<Limit> {
    let u = t.trim().to_ascii_uppercase();
    if u.is_empty() {
        return None;
    }
    if u == "GND" || u == "SFC" || u == "0" {
        return Some(Limit { text: "GND".into(), ft: 0.0 });
    }
    if u.starts_with("UNL") {
        return Some(Limit { text: "UNL".into(), ft: 60_000.0 });
    }
    let digits: String = u.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
    let v: f64 = digits.parse().ok()?;
    if u.starts_with("FL") {
        return Some(Limit { text: format!("FL{:03}", v.round() as i64), ft: v * 100.0 });
    }
    let ft = if u.contains('M') && !u.contains("MSL") || u.ends_with(" M") { v * 3.28084 } else { v };
    let datum = if u.contains("AGL") || u.contains("GND") || u.contains("SFC") { "AGL" } else { "MSL" };
    Some(Limit { text: format!("{}{datum}", ft.round() as i64), ft })
}

/// The airspace worth showing a pilot (not flight information regions,
/// sectors, airways or gliding areas).
fn openaip_feature(f: &Value) -> Option<Value> {
    let p = &f["properties"];
    // Codes as numbers, or as words in some exports.
    let t = num(&p["type"]).map(|x| x as i64).unwrap_or_else(|| {
        match p["type"].as_str().unwrap_or("").trim().to_ascii_uppercase().as_str() {
            "RESTRICTED" | "R" => 1,
            "DANGER" | "Q" | "D" => 2,
            "PROHIBITED" | "P" => 3,
            "CTR" => 4,
            "TMA" => 7,
            "TRA" => 8,
            "TSA" => 9,
            "ATZ" => 13,
            "MATZ" => 14,
            "ALERT" => 17,
            "WARNING" => 18,
            "MTA" => 25,
            "CTA" => 26,
            "MCTR" => 36,
            "OTHER" | "" => 0,
            _ => -1,
        }
    });
    let class = match num(&p["icaoClass"]).map(|x| x as i64) {
        Some(0) => Some("A"),
        Some(1) => Some("B"),
        Some(2) => Some("C"),
        Some(3) => Some("D"),
        Some(4) => Some("E"),
        Some(5) => Some("F"),
        Some(6) => Some("G"),
        Some(_) => None,
        None => match p["icaoClass"].as_str().or(p["class"].as_str()).unwrap_or("").trim().to_ascii_uppercase().as_str() {
            c @ ("A" | "B" | "C" | "D" | "E" | "F" | "G") => Some(match c {
                "A" => "A",
                "B" => "B",
                "C" => "C",
                "D" => "D",
                "E" => "E",
                "F" => "F",
                _ => "G",
            }),
            _ => None,
        },
    };
    // Airways (and Canada's low-level airways, given as class E corridors):
    // kept, marked, so the map can hide them.
    let name_u = p["name"].as_str().unwrap_or("").to_ascii_uppercase();
    let airway = t == 15 || name_u.contains("AIRWAY") || name_u.contains("LOW LEVEL") || name_u.contains("CORRIDOR");
    let kind = match t {
        15 => class.unwrap_or("E"),
        3 => "P",
        1 => "R",
        2 => "DNG",
        18 => "W",
        17 => "AL",
        8 => "TRA",
        9 => "TSA",
        25 | 14 => "MIL",
        // Controlled areas: by their class, else as a control zone.
        4 | 7 | 13 | 26 | 36 | 0 | 6 | 5 => match (class, t) {
            (Some(c), _) => c,
            (None, 4 | 13 | 36) => "CTR",
            _ => return None,
        },
        _ => return None,
    };
    let name = p["name"].as_str().unwrap_or("").trim().to_string();
    let tag = if "ABCDEFG".contains(kind) && kind.len() == 1 { kind.to_string() } else { sua_tag(kind, &name) };
    let lo = openaip_limit(&p["lowerLimit"]).or_else(|| openaip_limit(&p["lower_limit"])).or_else(|| openaip_limit(&p["bottom"]));
    let hi = openaip_limit(&p["upperLimit"]).or_else(|| openaip_limit(&p["upper_limit"])).or_else(|| openaip_limit(&p["top"]));
    let geometry = clean_geometry(&f["geometry"], SIMPLIFY_SUA)?;
    let mut out = feature(kind, &name, &tag, lo, hi, geometry);
    if airway {
        out["properties"]["aw"] = json!(true);
    }
    Some(out)
}

fn fetch(cycle: &str) -> Result<Vec<u8>, String> {
    let mut res = crate::update::agent().get(&url(cycle)).call().map_err(|e| format!("Could not reach the FAA (nfdc.faa.gov): {e}"))?;
    res.body_mut()
        .with_config()
        .limit(200 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| format!("Unexpected answer from the FAA: {e}"))
}

/// `{prefix}-YYYY-MM-DD.json`
fn is_copy(p: &Path, prefix: &str) -> bool {
    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
        n.strip_prefix(prefix).and_then(|r| r.strip_prefix('-')).and_then(|r| r.strip_suffix(".json")).is_some_and(|d| d.len() == 10 && d.as_bytes()[4] == b'-')
    })
}

fn newest(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let mut all: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).filter(|p| is_copy(p, prefix)).collect();
    all.sort();
    all.pop()
}

fn forget_others(dir: &Path, prefix: &str, keep: &Path) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        // Also the old US copies ("us-…", before the labels changed).
        // Also the copies of older versions ("us-…", "class-…", "oaip-xx-…").
        let base = prefix.replace("oaip4-", "oaip-").replace("class2", "class").replace("sua2", "sua");
        let older = is_copy(&p, "us") || ["oaip2-", "oaip3-"].iter().any(|o| is_copy(&p, &prefix.replace("oaip4-", o))) || is_copy(&p, &base);
        if p != keep && (is_copy(&p, prefix) || older) {
            let _ = std::fs::remove_file(p);
        }
    }
}

/// Reads the shapefile out of the FAA's zip and keeps Class B, C and D.
pub fn from_zip(bytes: &[u8], cycle: &str) -> Result<String, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("The FAA file is not a zip: {e}"))?;
    let names: Vec<String> = zip.file_names().filter_map(|n| n.ok().map(|n| n.to_string())).collect();
    let pick = |ext: &str| -> Option<String> {
        let ext = ext.to_ascii_lowercase();
        names
            .iter()
            .filter(|n| n.to_ascii_lowercase().ends_with(&ext))
            .min_by_key(|n| (!n.to_ascii_lowercase().contains("class_airspace"), n.len()))
            .cloned()
    };
    let shp = pick(".shp").ok_or("No shapefile in the FAA file")?;
    let stem = &shp[..shp.len() - 4];
    let dbf = names.iter().find(|n| n.len() == shp.len() && n.to_ascii_lowercase() == format!("{}.dbf", stem.to_ascii_lowercase())).cloned().ok_or("No attribute table in the FAA file")?;
    let read = |zip: &mut zip::ZipArchive<Cursor<&[u8]>>, n: &str| -> Result<Vec<u8>, String> {
        let mut f = zip.by_name(n).map_err(|e| e.to_string())?;
        let mut v = Vec::new();
        f.read_to_end(&mut v).map_err(|e| e.to_string())?;
        Ok(v)
    };
    let shp_bytes = read(&mut zip, &shp)?;
    let dbf_bytes = read(&mut zip, &dbf)?;
    let features = parse(shp_bytes, dbf_bytes)?;
    if features.is_empty() {
        return Err("The FAA file had no Class B, C or D airspace".into());
    }
    let fc = json!({ "type": "FeatureCollection", "cycle": cycle, "features": features });
    Ok(fc.to_string())
}

fn text(r: &dbase::Record, field: &str) -> Option<String> {
    use dbase::FieldValue as F;
    let s = match r.get(field)? {
        F::Character(Some(s)) | F::Memo(s) => s.trim().to_string(),
        F::Numeric(Some(n)) | F::Double(n) | F::Currency(n) => format!("{n}"),
        F::Float(Some(n)) => format!("{n}"),
        F::Integer(n) => n.to_string(),
        _ => return None,
    };
    (!s.is_empty() && s != "<Null>").then_some(s)
}

fn parse(shp: Vec<u8>, dbf: Vec<u8>) -> Result<Vec<Value>, String> {
    let shapes = shapefile::ShapeReader::new(Cursor::new(shp)).map_err(|e| format!("FAA shapefile: {e}"))?;
    let table = dbase::Reader::new(Cursor::new(dbf)).map_err(|e| format!("FAA attribute table: {e}"))?;
    let mut reader = shapefile::Reader::new(shapes, table);
    let mut out = Vec::new();
    for item in reader.iter_shapes_and_records() {
        let (shape, rec) = item.map_err(|e| format!("FAA shapefile: {e}"))?;
        let class = match text(&rec, "LOCAL_TYPE").as_deref() {
            Some("CLASS_B") => "B",
            Some("CLASS_C") => "C",
            Some("CLASS_D") => "D",
            _ => continue,
        };
        let rings: Vec<(bool, Vec<(f64, f64)>)> = match shape {
            shapefile::Shape::Polygon(p) => p.rings().iter().map(|r| (is_outer(r), r.points().iter().map(|q| (q.x, q.y)).collect())).collect(),
            shapefile::Shape::PolygonM(p) => p.rings().iter().map(|r| (is_outer(r), r.points().iter().map(|q| (q.x, q.y)).collect())).collect(),
            shapefile::Shape::PolygonZ(p) => p.rings().iter().map(|r| (is_outer(r), r.points().iter().map(|q| (q.x, q.y)).collect())).collect(),
            _ => continue,
        };
        let hi = faa_limit(text(&rec, "UPPER_VAL"), text(&rec, "UPPER_UOM"), text(&rec, "UPPER_CODE"));
        let lo = faa_limit(text(&rec, "LOWER_VAL"), text(&rec, "LOWER_UOM"), text(&rec, "LOWER_CODE"));
        let name = text(&rec, "NAME").unwrap_or_default();
        // Each outer ring with the holes after it, as GeoJSON wants them:
        // outer rings counterclockwise, holes clockwise (shapefiles have
        // them the other way round).
        let mut polys: Vec<Vec<Vec<[f64; 2]>>> = Vec::new();
        for (outer, pts) in rings {
            let mut ring: Vec<[f64; 2]> = simplify(&pts, SIMPLIFY_DEG).into_iter().map(|(x, y)| [round5(x), round5(y)]).collect();
            if ring.len() < 4 {
                continue;
            }
            if (signed_area(&ring) < 0.0) == outer {
                ring.reverse();
            }
            if outer || polys.is_empty() {
                polys.push(vec![ring]);
            } else if let Some(last) = polys.last_mut() {
                last.push(ring);
            }
        }
        if polys.is_empty() {
            continue;
        }
        let geometry = if polys.len() == 1 {
            json!({ "type": "Polygon", "coordinates": polys.remove(0) })
        } else {
            json!({ "type": "MultiPolygon", "coordinates": polys })
        };
        out.push(feature(class, &name, class, lo, hi, geometry));
    }
    Ok(out)
}

fn is_outer<P>(r: &shapefile::PolygonRing<P>) -> bool {
    matches!(r, shapefile::PolygonRing::Outer(_))
}

fn round5(v: f64) -> f64 {
    (v * 1e5).round() / 1e5
}

/// Twice the signed area: positive counterclockwise.
fn signed_area(r: &[[f64; 2]]) -> f64 {
    r.windows(2).map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1]).sum()
}

/// Douglas-Peucker on a closed ring (first and last points kept).
fn simplify(pts: &[(f64, f64)], tol: f64) -> Vec<(f64, f64)> {
    if pts.len() < 5 {
        return pts.to_vec();
    }
    let mut keep = vec![false; pts.len()];
    keep[0] = true;
    keep[pts.len() - 1] = true;
    // A closed ring: split at the point farthest from the start, so the two
    // halves are open lines.
    let far = (1..pts.len() - 1)
        .max_by(|&a, &b| dist2(pts[a], pts[0]).total_cmp(&dist2(pts[b], pts[0])))
        .unwrap_or(1);
    keep[far] = true;
    let mut stack = vec![(0, far), (far, pts.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (mut best, mut at) = (0.0, a);
        for i in a + 1..b {
            let d = seg_dist(pts[i], pts[a], pts[b]);
            if d > best {
                best = d;
                at = i;
            }
        }
        if best > tol {
            keep[at] = true;
            stack.push((a, at));
            stack.push((at, b));
        }
    }
    pts.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

fn dist2(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)
}

fn seg_dist(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let l = dist2(a, b);
    if l == 0.0 {
        return dist2(p, a).sqrt();
    }
    let t = (((p.0 - a.0) * (b.0 - a.0) + (p.1 - a.1) * (b.1 - a.1)) / l).clamp(0.0, 1.0);
    dist2(p, (a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1))).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn cycles_follow_every_28_days() {
        assert_eq!(cycle_at(ANCHOR_MS), "2026-10-29");
        assert_eq!(cycle_at(ANCHOR_MS + 27 * 86_400_000), "2026-10-29");
        assert_eq!(cycle_at(ANCHOR_MS + CYCLE_MS), "2026-11-26");
        assert_eq!(cycle_at(ANCHOR_MS - 1), "2026-10-01");
    }

    #[test]
    fn limits_read_as_the_user_likes_them() {
        let s = |v: &str| Some(v.to_string());
        let t = |l: Option<Limit>| l.map(|l| l.text);
        assert_eq!(t(faa_limit(s("10000"), s("FT"), s("MSL"))).as_deref(), Some("10000MSL"));
        assert_eq!(t(faa_limit(s("0"), s("FT"), s("SFC"))).as_deref(), Some("GND"));
        assert_eq!(t(faa_limit(s("2000"), s("FT"), s("AGL"))).as_deref(), Some("2000AGL"));
        assert_eq!(faa_limit(s("-9998"), None, None), None);
        assert_eq!(t(faa_limit(s("180"), s("FL"), s("STD"))).as_deref(), Some("FL180"));
        assert_eq!(t(faa_limit(s("UNLTD"), None, None)).as_deref(), Some("UNL"));
        let o = |v: f64, u: i64, d: i64| t(openaip_limit(&json!({ "value": v, "unit": u, "referenceDatum": d })));
        assert_eq!(o(0.0, 1, 0).as_deref(), Some("GND"));
        assert_eq!(o(2000.0, 1, 0).as_deref(), Some("2000AGL"));
        assert_eq!(o(95.0, 6, 2).as_deref(), Some("FL095"));
        assert_eq!(o(4500.0, 1, 1).as_deref(), Some("4500MSL"));
        assert_eq!(t(openaip_limit(&json!({ "value": "95", "unit": "FL", "referenceDatum": "STD" }))).as_deref(), Some("FL095"));
        assert_eq!(t(openaip_limit(&json!("2000ft AGL"))).as_deref(), Some("2000AGL"));
        assert_eq!(t(openaip_limit(&json!("FL95"))).as_deref(), Some("FL095"));
        assert_eq!(label("G", &openaip_limit(&json!({ "value": 0, "unit": 1, "referenceDatum": 0 })), &openaip_limit(&json!({ "value": 2000, "unit": 1, "referenceDatum": 0 }))), "G: GND-2000AGL");
    }

    #[test]
    fn faa_special_use_and_openaip_features() {
        let sq = json!({ "type": "Polygon", "coordinates": [[[-117.0, 35.0], [-116.0, 35.0], [-116.0, 36.0], [-117.0, 36.0], [-117.0, 35.0]]] });
        let r = sua_feature(&json!({ "properties": { "NAME": "R-2508 COMPLEX", "TYPE_CODE": "R", "LOWER_VAL": "0", "LOWER_CODE": "SFC", "UPPER_VAL": "UNLTD" }, "geometry": sq })).unwrap();
        assert_eq!(r["properties"]["c"], "R");
        assert_eq!(r["properties"]["label"], "R-2508: GND-UNL");
        assert_eq!(r["properties"]["hi"], 60000.0);
        let m = sua_feature(&json!({ "properties": { "NAME": "SHOSHONE MOA", "TYPE_CODE": "MOA", "LOWER_VAL": "100", "LOWER_UOM": "FT", "LOWER_CODE": "AGL", "UPPER_VAL": "180", "UPPER_UOM": "FL" }, "geometry": sq })).unwrap();
        assert_eq!(m["properties"]["label"], "SHOSHONE MOA: 100AGL-FL180");
        assert!(sua_feature(&json!({ "properties": { "TYPE_CODE": "ZZZ" }, "geometry": sq })).is_none());
        let g = openaip_feature(&json!({ "properties": { "name": "LONDON TMA", "type": 7, "icaoClass": 0, "lowerLimit": { "value": 25, "unit": 6, "referenceDatum": 2 }, "upperLimit": { "value": 245, "unit": 6, "referenceDatum": 2 } }, "geometry": sq })).unwrap();
        assert_eq!(g["properties"]["label"], "A: FL025-FL245");
        let fir = openaip_feature(&json!({ "properties": { "name": "X FIR", "type": 10, "icaoClass": 8 }, "geometry": sq }));
        assert!(fir.is_none(), "flight information regions left out");
        let w = openaip_feature(&json!({ "properties": { "name": "LONDON CTR", "type": "CTR", "icaoClass": "D", "lowerLimit": { "value": 0, "unit": "FT", "referenceDatum": "GND" }, "upperLimit": { "value": 2500, "unit": "FT", "referenceDatum": "MSL" } }, "geometry": sq })).unwrap();
        assert_eq!(w["properties"]["label"], "D: GND-2500MSL");
        let d = openaip_feature(&json!({ "properties": { "name": "ED-R 116", "type": 1, "icaoClass": 8, "lowerLimit": { "value": 0, "unit": 1, "referenceDatum": 0 }, "upperLimit": { "value": 100, "unit": 6, "referenceDatum": 2 } }, "geometry": sq })).unwrap();
        assert_eq!(d["properties"]["label"], "ED-R 116: GND-FL100");
    }

    #[test]
    fn faa_airport_codes() {
        assert_eq!(airport_codes("SEA", "KSEA"), vec!["KSEA", "SEA"]);
        assert_eq!(airport_codes("63CN", ""), vec!["63CN"]);
        assert_eq!(airport_codes("S43", ""), vec!["S43"]);
    }

    #[test]
    fn openaip_airports_private_and_fuel() {
        let f = json!({ "type": "Feature", "properties": { "name": "X", "icaoCode": "kful", "private": true, "services": { "fuelTypes": [1, 2] } }, "geometry": { "type": "Point", "coordinates": [-117.98, 33.87] } });
        let a = openaip_airport(&f).unwrap();
        assert_eq!((a.2.as_str(), a.3, a.4), ("KFUL", true, true));
        let g = json!({ "properties": { "name": "Y" }, "geometry": { "type": "Point", "coordinates": [1.0, 2.0] } });
        assert_eq!(openaip_airport(&g).map(|a| (a.3, a.4)), Some((false, false)));
        assert!(countries_in_all(-125.0, 30.0, -70.0, 49.0).iter().any(|c| c == "us"));
        let aw = openaip_feature(&json!({ "properties": { "name": "V300 LOW LEVEL AIRWAY", "type": 0, "icaoClass": 4 }, "geometry": { "type": "Polygon", "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]] } })).unwrap();
        assert_eq!(aw["properties"]["aw"], true);
    }

    #[test]
    fn countries_in_view_leave_the_us_out() {
        let c = countries_in(-125.0, 30.0, -70.0, 49.0);
        assert!(c.iter().any(|x| x == "ca") && c.iter().any(|x| x == "mx"), "{c:?}");
        assert!(!c.iter().any(|x| x == "us"));
        let eu = countries_in(5.0, 47.0, 15.0, 55.0);
        assert!(eu.iter().any(|x| x == "de") && eu.iter().any(|x| x == "fr"));
        // Across the date line.
        assert!(countries_in(175.0, -45.0, -175.0, -35.0).iter().any(|x| x == "nz"));
    }

    /// A shapefile like the FAA's, written here: a Class B with a hole, a
    /// Class D, and a Class E that is left out.
    #[test]
    fn reads_class_b_c_d_from_the_faa_zip() {
        let dir = std::env::temp_dir().join(format!("fd-airspace-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let shp = dir.join("Class_Airspace.shp");
        let circle = |cx: f64, cy: f64, r: f64, cw: bool| -> Vec<shapefile::Point> {
            // Densely drawn, as the FAA's arcs are.
            let mut v: Vec<shapefile::Point> = (0..=720)
                .map(|i| {
                    let a = (i as f64) * 0.5f64.to_radians();
                    shapefile::Point::new(cx + r * a.cos(), cy + r * a.sin())
                })
                .collect();
            if cw {
                v.reverse();
            }
            v
        };
        let table = dbase::TableWriterBuilder::new()
            .add_character_field("IDENT".try_into().unwrap(), 8)
            .add_character_field("NAME".try_into().unwrap(), 60)
            .add_character_field("LOCAL_TYPE".try_into().unwrap(), 12)
            .add_character_field("UPPER_VAL".try_into().unwrap(), 8)
            .add_character_field("UPPER_UOM".try_into().unwrap(), 4)
            .add_character_field("UPPER_CODE".try_into().unwrap(), 6)
            .add_character_field("LOWER_VAL".try_into().unwrap(), 8)
            .add_character_field("LOWER_UOM".try_into().unwrap(), 4)
            .add_character_field("LOWER_CODE".try_into().unwrap(), 6);
        {
            let mut w = shapefile::Writer::from_path(&shp, table).unwrap();
            let rec = |ident: &str, name: &str, t: &str, hi: &str, lo: &str, locode: &str| {
                let mut r = dbase::Record::default();
                for (k, v) in [("IDENT", ident), ("NAME", name), ("LOCAL_TYPE", t), ("UPPER_VAL", hi), ("UPPER_UOM", "FT"), ("UPPER_CODE", "MSL"), ("LOWER_VAL", lo), ("LOWER_UOM", "FT"), ("LOWER_CODE", locode)] {
                    r.insert(k.to_string(), dbase::FieldValue::Character(Some(v.to_string())));
                }
                r
            };
            let shelf = shapefile::Polygon::with_rings(vec![
                shapefile::PolygonRing::Outer(circle(-122.3, 47.45, 0.3, true)),
                shapefile::PolygonRing::Inner(circle(-122.3, 47.45, 0.15, false)),
            ]);
            w.write_shape_and_record(&shelf, &rec("SEA", "SEATTLE CLASS B", "CLASS_B", "10000", "3000", "MSL")).unwrap();
            let d = shapefile::Polygon::new(shapefile::PolygonRing::Outer(circle(-122.2, 47.5, 0.07, true)));
            w.write_shape_and_record(&d, &rec("BFI", "BOEING FIELD CLASS D", "CLASS_D", "2500", "0", "SFC")).unwrap();
            let e = shapefile::Polygon::new(shapefile::PolygonRing::Outer(circle(-121.0, 47.0, 0.2, true)));
            w.write_shape_and_record(&e, &rec("XXX", "SOME E", "CLASS_E5", "-9998", "700", "AGL")).unwrap();
        }
        let mut zbytes = Vec::new();
        {
            let mut z = zip::ZipWriter::new(Cursor::new(&mut zbytes));
            let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            for ext in ["shp", "shx", "dbf"] {
                z.start_file(format!("Shape_Files/Class_Airspace.{ext}"), opt).unwrap();
                z.write_all(&std::fs::read(dir.join(format!("Class_Airspace.{ext}"))).unwrap()).unwrap();
            }
            z.finish().unwrap();
        }
        let json: Value = serde_json::from_str(&from_zip(&zbytes, "2026-10-29").unwrap()).unwrap();
        let fs = json["features"].as_array().unwrap();
        assert_eq!(fs.len(), 2, "Class E left out");
        assert_eq!(fs[0]["properties"]["c"], "B");
        assert_eq!(fs[0]["properties"]["label"], "B: 3000MSL-10000MSL");
        assert_eq!(fs[1]["properties"]["label"], "D: GND-2500MSL");
        assert_eq!(fs[0]["properties"]["lo"], 3000.0);
        // The shelf keeps its hole, the outer ring counterclockwise and the
        // hole clockwise, and fewer points than the source.
        let rings = fs[0]["geometry"]["coordinates"].as_array().unwrap();
        assert_eq!(rings.len(), 2);
        let ring = |i: usize| -> Vec<[f64; 2]> { serde_json::from_value(rings[i].clone()).unwrap() };
        assert!(signed_area(&ring(0)) > 0.0 && signed_area(&ring(1)) < 0.0);
        assert!(ring(0).len() < 300, "{} points", ring(0).len());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
