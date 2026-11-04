//! Online networks on the map: who is flying and who is controlling on
//! VATSIM and IVAO.
//!
//! Live data comes from each network's public feed (VATSIM's data feed and
//! IVAO's Whazzup v2, both refreshed every 15 seconds); FlightDeck asks only
//! while a network layer is on, at most every 30 seconds per network. Centre
//! and FIR areas, callsign prefixes and airport positions come from the
//! VAT-Spy Data Project (CC BY-SA 4.0), bundled in data/vatspy.json.zst.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const VATSIM_STATUS: &str = "https://status.vatsim.net/status.json";
const VATSIM_FEED: &str = "https://data.vatsim.net/v3/vatsim-data.json";
const IVAO_FEED: &str = "https://api.ivao.aero/v2/tracker/whazzup";
const REFRESH: Duration = Duration::from_secs(30);

// --- bundled boundaries ---------------------------------------------------

#[derive(Deserialize)]
struct Fir {
    icao: String,
    name: String,
    prefix: String,
    boundary: String,
}

#[derive(Deserialize)]
struct Uir {
    id: String,
    name: String,
    firs: Vec<String>,
}

#[derive(Deserialize)]
struct SpyAirport {
    icao: String,
    name: String,
    lat: f64,
    lon: f64,
    iata: String,
}

#[derive(Deserialize)]
struct Boundary {
    label: [f64; 2],
    oceanic: bool,
    polys: Vec<Vec<Vec<[f64; 2]>>>,
}

#[derive(Deserialize)]
struct Spy {
    firs: Vec<Fir>,
    uirs: Vec<Uir>,
    airports: Vec<SpyAirport>,
    boundaries: HashMap<String, Boundary>,
}

fn spy() -> &'static Spy {
    static SPY: OnceLock<Spy> = OnceLock::new();
    SPY.get_or_init(|| {
        let raw = zstd::decode_all(&include_bytes!("../data/vatspy.json.zst")[..]).unwrap_or_default();
        serde_json::from_slice(&raw).unwrap_or(Spy {
            firs: vec![],
            uirs: vec![],
            airports: vec![],
            boundaries: HashMap::new(),
        })
    })
}

/// The areas (boundary id, name) a centre or FSS callsign covers: the
/// longest callsign prefix that names a FIR or sector (by its callsign
/// prefix or ICAO code), else an upper area (UIR) and all its FIRs.
fn areas_for(callsign: &str) -> Vec<(String, String)> {
    let s = spy();
    let parts: Vec<&str> = callsign.split('_').collect();
    if parts.len() < 2 {
        return vec![];
    }
    let parts = &parts[..parts.len() - 1];
    for n in (1..=parts.len()).rev() {
        let key = parts[..n].join("_");
        let by_prefix: Vec<&Fir> = s.firs.iter().filter(|f| f.prefix == key).collect();
        let hits: Vec<&Fir> =
            if by_prefix.is_empty() { s.firs.iter().filter(|f| f.icao == key && f.prefix.is_empty()).collect() } else { by_prefix };
        let hits: Vec<&Fir> = if hits.is_empty() { s.firs.iter().filter(|f| f.icao == key).take(1).collect() } else { hits };
        if let Some(f) = hits.first() {
            return vec![(f.boundary.clone(), f.name.clone())];
        }
        if let Some(u) = s.uirs.iter().find(|u| u.id == key) {
            return u
                .firs
                .iter()
                .filter_map(|icao| s.firs.iter().find(|f| &f.icao == icao))
                .map(|f| (f.boundary.clone(), u.name.clone()))
                .collect();
        }
    }
    vec![]
}

/// The airport an airport controller's callsign belongs to: its first part
/// as an ICAO code, an IATA/FAA code, or (in the US) with a K in front.
fn airport_for(callsign: &str) -> Option<&'static SpyAirport> {
    let key = callsign.split('_').next()?;
    let s = spy();
    s.airports
        .iter()
        .find(|a| a.icao == key)
        .or_else(|| s.airports.iter().find(|a| a.iata == key))
        .or_else(|| (key.len() == 3).then(|| s.airports.iter().find(|a| a.icao == format!("K{key}"))).flatten())
}

// --- what is sent to the map -----------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct Pilot {
    pub callsign: String,
    /// VATSIM CID or IVAO VID.
    pub id: String,
    /// VATSIM gives a name; IVAO does not.
    pub name: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub alt_ft: i64,
    pub gs_kt: i64,
    pub hdg: i64,
    pub aircraft: String,
    pub dep: String,
    pub arr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flight_rules: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cruise: Option<String>,
    pub logon_ms: i64,
    /// [lon, lat] of the departure airport, when asked for one pilot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dep_pos: Option<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Controller {
    pub callsign: String,
    pub id: String,
    pub name: Option<String>,
    pub frequency: String,
    /// DEL, GND, TWR, APP, DEP, CTR, FSS or ATIS.
    pub kind: String,
    pub rating: Option<String>,
    pub logon_ms: i64,
    pub text: Vec<String>,
    pub atis_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Area {
    /// The boundary's id.
    pub id: String,
    pub name: String,
    pub oceanic: bool,
    /// [lon, lat] for the label.
    pub label: [f64; 2],
    pub polys: Vec<Vec<Vec<[f64; 2]>>>,
    pub controllers: Vec<Controller>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AirportAtc {
    pub icao: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub controllers: Vec<Controller>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct NetworkData {
    /// "vatsim" or "ivao".
    pub net: String,
    pub updated_ms: i64,
    pub pilots: Vec<Pilot>,
    pub areas: Vec<Area>,
    pub airports: Vec<AirportAtc>,
    /// Controllers not placed on the map (no matching area or airport).
    pub other: Vec<Controller>,
    pub error: Option<String>,
}

// --- parsing ----------------------------------------------------------------

/// "2026-11-03T21:15:04.1234567Z" to Unix milliseconds.
fn iso_ms(s: &str) -> i64 {
    let n = |a: usize, b: usize| s.get(a..b).and_then(|t| t.parse::<i64>().ok());
    let (Some(y), Some(mo), Some(d), Some(h), Some(mi), Some(se)) = (n(0, 4), n(5, 7), n(8, 10), n(11, 13), n(14, 16), n(17, 19))
    else {
        return 0;
    };
    // Days from the civil date (Howard Hinnant's algorithm).
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    ((days * 86_400) + h * 3600 + mi * 60 + se) * 1000
}

/// An ISO time in milliseconds, for the other feeds.
pub(crate) fn iso_ms_pub(s: &str) -> Option<i64> {
    let v = iso_ms(s);
    (v != 0).then_some(v)
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn int(v: &Value) -> i64 {
    v.as_f64().map(|f| f.round() as i64).unwrap_or(0)
}

fn kind_of(callsign: &str) -> String {
    let suffix = callsign.rsplit('_').next().unwrap_or("").to_ascii_uppercase();
    match suffix.as_str() {
        "DEL" | "GND" | "TWR" | "APP" | "DEP" | "CTR" | "FSS" | "ATIS" | "RMP" => suffix,
        _ => "OBS".into(),
    }
}

const VATSIM_RATINGS: &[(i64, &str)] =
    &[(1, "OBS"), (2, "S1"), (3, "S2"), (4, "S3"), (5, "C1"), (6, "C2"), (7, "C3"), (8, "I1"), (9, "I2"), (10, "I3"), (11, "SUP"), (12, "ADM")];

fn parse_vatsim(v: &Value) -> (Vec<Pilot>, Vec<Controller>, i64) {
    let updated = iso_ms(&text(&v["general"]["update_timestamp"]));
    let pilots = v["pilots"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|p| {
                    let fp = &p["flight_plan"];
                    Pilot {
                        callsign: text(&p["callsign"]),
                        id: text(&p["cid"]),
                        name: Some(text(&p["name"])).filter(|s| !s.is_empty()),
                        lat: p["latitude"].as_f64().unwrap_or(0.0),
                        lon: p["longitude"].as_f64().unwrap_or(0.0),
                        alt_ft: int(&p["altitude"]),
                        gs_kt: int(&p["groundspeed"]),
                        hdg: int(&p["heading"]),
                        aircraft: text(&fp["aircraft_short"]),
                        dep: text(&fp["departure"]),
                        arr: text(&fp["arrival"]),
                        route: Some(text(&fp["route"])).filter(|s| !s.is_empty()),
                        flight_rules: Some(text(&fp["flight_rules"])).filter(|s| !s.is_empty()),
                        cruise: Some(text(&fp["altitude"])).filter(|s| !s.is_empty()),
                        logon_ms: iso_ms(&text(&p["logon_time"])),
                        dep_pos: None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let mut ctrl = Vec::new();
    for (list, atis) in [(&v["controllers"], false), (&v["atis"], true)] {
        for c in list.as_array().into_iter().flatten() {
            let callsign = text(&c["callsign"]);
            let kind = if atis { "ATIS".to_string() } else { kind_of(&callsign) };
            if kind == "OBS" {
                continue;
            }
            let rating = c["rating"].as_i64().and_then(|r| VATSIM_RATINGS.iter().find(|x| x.0 == r)).map(|x| x.1.to_string());
            ctrl.push(Controller {
                id: text(&c["cid"]),
                name: Some(text(&c["name"])).filter(|s| !s.is_empty()),
                frequency: text(&c["frequency"]),
                kind,
                rating,
                logon_ms: iso_ms(&text(&c["logon_time"])),
                text: c["text_atis"].as_array().map(|l| l.iter().map(text).filter(|s| !s.is_empty()).collect()).unwrap_or_default(),
                atis_code: Some(text(&c["atis_code"])).filter(|s| !s.is_empty()),
                callsign,
            });
        }
    }
    (pilots, ctrl, updated)
}

fn parse_ivao(v: &Value) -> (Vec<Pilot>, Vec<Controller>, i64) {
    let updated = iso_ms(&text(&v["updatedAt"]));
    let clients = &v["clients"];
    let pilots = clients["pilots"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|p| {
                    let t = &p["lastTrack"];
                    let fp = &p["flightPlan"];
                    Pilot {
                        callsign: text(&p["callsign"]),
                        id: text(&p["userId"]),
                        name: None,
                        lat: t["latitude"].as_f64().unwrap_or(0.0),
                        lon: t["longitude"].as_f64().unwrap_or(0.0),
                        alt_ft: int(&t["altitude"]),
                        gs_kt: int(&t["groundSpeed"]),
                        hdg: int(&t["heading"]),
                        aircraft: Some(text(&fp["aircraftId"])).filter(|s| !s.is_empty()).unwrap_or_else(|| text(&fp["aircraft"]["icaoCode"])),
                        dep: text(&fp["departureId"]),
                        arr: text(&fp["arrivalId"]),
                        route: Some(text(&fp["route"])).filter(|s| !s.is_empty()),
                        flight_rules: Some(text(&fp["flightRules"])).filter(|s| !s.is_empty()),
                        cruise: Some(text(&fp["level"])).filter(|s| !s.is_empty()),
                        logon_ms: iso_ms(&text(&p["createdAt"])),
                        dep_pos: None,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let mut ctrl = Vec::new();
    for c in clients["atcs"].as_array().into_iter().flatten() {
        let callsign = text(&c["callsign"]);
        let session = &c["atcSession"];
        let pos = text(&session["position"]).to_ascii_uppercase();
        let kind = if pos.is_empty() { kind_of(&callsign) } else { pos };
        if !["DEL", "GND", "TWR", "APP", "DEP", "CTR", "FSS", "ATIS", "RMP"].contains(&kind.as_str()) {
            continue;
        }
        let freq = session["frequency"].as_f64().map(|f| format!("{f:.3}")).unwrap_or_else(|| text(&session["frequency"]));
        let lines: Vec<String> =
            c["atis"]["lines"].as_array().map(|l| l.iter().map(text).filter(|s| !s.is_empty()).collect()).unwrap_or_default();
        ctrl.push(Controller {
            id: text(&c["userId"]),
            name: None,
            frequency: freq,
            kind,
            rating: c["rating"].as_i64().map(|r| r.to_string()),
            logon_ms: iso_ms(&text(&c["createdAt"])),
            text: lines,
            atis_code: None,
            callsign,
        });
    }
    (pilots, ctrl, updated)
}

/// Put each controller on the map: centres and FSS on their areas, the rest
/// on their airports.
fn place(net: &str, pilots: Vec<Pilot>, ctrl: Vec<Controller>, updated: i64) -> NetworkData {
    let s = spy();
    let mut areas: BTreeMap<String, Area> = BTreeMap::new();
    let mut airports: BTreeMap<String, AirportAtc> = BTreeMap::new();
    let mut other = Vec::new();
    for c in ctrl {
        if c.kind == "CTR" || c.kind == "FSS" {
            let hits = areas_for(&c.callsign);
            let mut placed = false;
            for (id, name) in hits {
                let Some(b) = s.boundaries.get(&id) else { continue };
                placed = true;
                areas
                    .entry(id.clone())
                    .or_insert_with(|| Area {
                        id: id.clone(),
                        name,
                        oceanic: b.oceanic,
                        label: b.label,
                        polys: b.polys.clone(),
                        controllers: vec![],
                    })
                    .controllers
                    .push(c.clone());
            }
            if !placed {
                other.push(c);
            }
        } else if let Some(a) = airport_for(&c.callsign) {
            airports
                .entry(a.icao.clone())
                .or_insert_with(|| AirportAtc { icao: a.icao.clone(), name: a.name.clone(), lat: a.lat, lon: a.lon, controllers: vec![] })
                .controllers
                .push(c);
        } else {
            other.push(c);
        }
    }
    NetworkData {
        net: net.into(),
        updated_ms: updated,
        pilots,
        areas: areas.into_values().collect(),
        airports: airports.into_values().collect(),
        other,
        error: None,
    }
}

// --- fetching -----------------------------------------------------------------

fn get_json(url: &str) -> Result<Value, String> {
    let mut res = crate::update::agent().get(url).call().map_err(|e| format!("Could not reach {url}: {e}"))?;
    res.body_mut()
        .with_config()
        .limit(64 * 1024 * 1024)
        .read_json()
        .map_err(|e| format!("Unexpected answer from {url}: {e}"))
}

fn vatsim_feed_url() -> String {
    // VATSIM asks clients to find the feed's address in status.json.
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| {
        get_json(VATSIM_STATUS)
            .ok()
            .and_then(|v| v["data"]["v3"].as_array().and_then(|a| a.first()).and_then(|u| u.as_str()).map(str::to_string))
            .unwrap_or_else(|| VATSIM_FEED.to_string())
    })
    .clone()
}

type Cache = Mutex<HashMap<String, (Instant, NetworkData)>>;
fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The network's pilots and controllers, at most 30 seconds old.
pub fn network(net: &str) -> NetworkData {
    if let Some((at, d)) = cache().lock().unwrap().get(net) {
        if at.elapsed() < REFRESH {
            return d.clone();
        }
    }
    let fetched = match net {
        "vatsim" => get_json(&vatsim_feed_url()).map(|v| parse_vatsim(&v)),
        "ivao" => get_json(IVAO_FEED).map(|v| parse_ivao(&v)),
        _ => Err(format!("Unknown network {net}")),
    };
    let data = match fetched {
        Ok((p, c, t)) => place(net, p, c, t),
        Err(e) => {
            log::warn!("{net}: {e}");
            // Keep showing the last good data, marked with the error.
            let last = cache().lock().unwrap().get(net).map(|(_, d)| d.clone());
            let mut d = last.unwrap_or_else(|| NetworkData { net: net.into(), ..Default::default() });
            d.error = Some(e);
            return d;
        }
    };
    cache().lock().unwrap().insert(net.to_string(), (Instant::now(), data.clone()));
    data
}

/// What the map needs, light: pilots without their routes, areas with shapes.
pub fn network_for_map(net: &str) -> NetworkData {
    let mut d = network(net);
    for p in &mut d.pilots {
        p.route = None;
    }
    d
}

/// One pilot with the whole flight plan and where they departed from, from
/// the last download.
pub fn pilot(net: &str, callsign: &str) -> Option<Pilot> {
    let mut p = cache().lock().unwrap().get(net).and_then(|(_, d)| d.pilots.iter().find(|p| p.callsign == callsign).cloned())?;
    let dep = p.dep.to_ascii_uppercase();
    p.dep_pos = (!dep.is_empty())
        .then(|| spy().airports.iter().find(|a| a.icao == dep && !a.icao.is_empty()))
        .flatten()
        .map(|a| [a.lon, a.lat]);
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_iso_times() {
        assert_eq!(iso_ms("1970-01-01T00:00:00Z"), 0);
        assert_eq!(iso_ms("2026-11-03T21:15:04.1234567Z"), 1_793_740_504_000);
    }

    #[test]
    fn finds_centre_areas_and_airports() {
        assert_eq!(areas_for("DC_32_CTR").first().map(|a| a.0.as_str()), Some("KZDC"));
        assert_eq!(areas_for("KZNY_CTR").first().map(|a| a.0.as_str()), Some("KZNY"));
        assert!(!areas_for("EGTT_CTR").is_empty());
        assert_eq!(airport_for("KPHL_TWR").map(|a| a.icao.as_str()), Some("KPHL"));
        assert_eq!(airport_for("PHL_APP").map(|a| a.icao.as_str()), Some("KPHL"));
        assert_eq!(airport_for("EDDF_N_GND").map(|a| a.icao.as_str()), Some("EDDF"));
    }

    #[test]
    fn reads_a_vatsim_feed() {
        let v: Value = serde_json::from_str(
            r#"{"general":{"update_timestamp":"2026-11-03T21:15:04Z"},
                "pilots":[{"cid":1,"name":"Jane Pilot","callsign":"DAL123","latitude":40.6,"longitude":-73.8,"altitude":35012,
                  "groundspeed":452,"heading":87,"logon_time":"2026-11-03T20:00:00Z",
                  "flight_plan":{"aircraft_short":"B739","departure":"KJFK","arrival":"KATL","route":"DCT","flight_rules":"I","altitude":"35000"}},
                  {"cid":2,"name":"No Plan","callsign":"N123","latitude":1,"longitude":2,"altitude":0,"groundspeed":0,"heading":0,
                  "logon_time":"2026-11-03T20:00:00Z","flight_plan":null}],
                "controllers":[{"cid":3,"name":"A","callsign":"DC_32_CTR","frequency":"133.725","facility":6,"rating":5,
                  "text_atis":["ZDC does NOT provide terminal service for PHL"],"logon_time":"2026-11-03T20:30:00Z"},
                  {"cid":4,"name":"B","callsign":"KPHL_TWR","frequency":"118.500","facility":4,"rating":3,"text_atis":null,
                  "logon_time":"2026-11-03T20:30:00Z"},
                  {"cid":5,"name":"C","callsign":"KPHL_OBS","frequency":"199.998","facility":0,"rating":1,"logon_time":"2026-11-03T20:30:00Z"}],
                "atis":[{"cid":6,"name":"D","callsign":"KPHL_ATIS","frequency":"133.400","facility":4,"rating":3,"atis_code":"B",
                  "text_atis":["PHILADELPHIA INFO B"],"logon_time":"2026-11-03T20:30:00Z"}]}"#,
        )
        .unwrap();
        let (p, c, t) = parse_vatsim(&v);
        assert_eq!(t, iso_ms("2026-11-03T21:15:04Z"));
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].aircraft.as_str(), p[0].dep.as_str(), p[0].arr.as_str()), ("B739", "KJFK", "KATL"));
        assert_eq!(p[1].dep, "");
        assert_eq!(c.len(), 3, "observers are left out");
        let d = place("vatsim", p, c, t);
        assert_eq!(d.areas.len(), 1);
        assert_eq!(d.areas[0].id, "KZDC");
        assert_eq!(d.airports.len(), 1);
        let kinds: Vec<&str> = d.airports[0].controllers.iter().map(|c| c.kind.as_str()).collect();
        assert_eq!(kinds, ["TWR", "ATIS"]);
        assert_eq!(d.airports[0].controllers[1].atis_code.as_deref(), Some("B"));
    }

    #[test]
    fn reads_an_ivao_feed() {
        let v: Value = serde_json::from_str(
            r#"{"updatedAt":"2026-11-03T21:15:04.000Z","clients":{
                "pilots":[{"userId":111,"callsign":"AFR1234","createdAt":"2026-11-03T20:00:00Z",
                  "lastTrack":{"latitude":49.0,"longitude":2.5,"altitude":11000,"groundSpeed":280,"heading":270,"onGround":false},
                  "flightPlan":{"aircraftId":"A320","departureId":"LFPG","arrivalId":"EGLL","route":"DCT","flightRules":"I","level":"F110"}}],
                "atcs":[{"userId":222,"callsign":"LFFF_CTR","createdAt":"2026-11-03T20:00:00Z","rating":5,
                  "atcSession":{"frequency":128.1,"position":"CTR"},"atis":{"lines":["LFFF_CTR","Paris Control"]}},
                  {"userId":333,"callsign":"LFPG_TWR","createdAt":"2026-11-03T20:00:00Z","atcSession":{"frequency":118.65,"position":"TWR"}}]}}"#,
        )
        .unwrap();
        let (p, c, _) = parse_ivao(&v);
        assert_eq!(p[0].callsign, "AFR1234");
        assert_eq!(p[0].alt_ft, 11000);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].frequency, "128.100");
        let d = place("ivao", p, c, 0);
        assert_eq!(d.areas.len(), 1, "{:?}", d.other);
        assert_eq!(d.airports[0].icao, "LFPG");
    }
}
