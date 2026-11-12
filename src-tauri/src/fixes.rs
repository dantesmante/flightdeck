//! Waypoints named in a flight plan without their positions (SimBrief's
//! and MSFS 2024's .PLN files list only idents): looked up in the sim's own
//! nav data kept on this PC, then in the FAA's published designated points
//! and nav aids (US, public data). Every candidate is returned; the planner
//! picks the one nearest the route.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use serde_json::Value;

use crate::navaids;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FixPos {
    pub ident: String,
    pub region: String,
    /// "vor", "ndb" or "wpt".
    pub kind: String,
    pub lat: f64,
    pub lon: f64,
}

/// The FAA's answers so far, by ident (empty: asked, not found).
static FAA: Mutex<Option<HashMap<String, Vec<FixPos>>>> = Mutex::new(None);

const FAA_POINTS: &str = "https://services6.arcgis.com/ssFJjBXIUyZDrSYZ/arcgis/rest/services/DesignatedPoints/FeatureServer/0/query";
const FAA_NAVAIDS: &str = "https://services6.arcgis.com/ssFJjBXIUyZDrSYZ/arcgis/rest/services/NAVAIDSystem/FeatureServer/0/query";

/// Only letters and digits, up to 6: an ident as charts write it.
fn clean(ident: &str) -> Option<String> {
    let s: String = ident.trim().to_ascii_uppercase();
    (!s.is_empty() && s.len() <= 6 && s.chars().all(|c| c.is_ascii_alphanumeric())).then_some(s)
}

/// The query URL for idents: where IDENT IN ('A','B'), as GeoJSON.
fn faa_url(base: &str, idents: &[String], fields: &str) -> String {
    let list = idents.iter().map(|i| format!("%27{i}%27")).collect::<Vec<_>>().join("%2C");
    format!("{base}?where=IDENT%20IN%20%28{list}%29&outFields={fields}&returnGeometry=true&outSR=4326&f=geojson")
}

/// Positions from an FAA answer.
fn faa_points(text: &str, navaid: bool) -> Vec<FixPos> {
    let Ok(v) = serde_json::from_str::<Value>(text) else { return Vec::new() };
    v["features"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|f| {
                    let p = &f["properties"];
                    let c = f["geometry"]["coordinates"].as_array()?;
                    let class = p["CLASS_TXT"].as_str().unwrap_or("").to_ascii_uppercase();
                    Some(FixPos {
                        ident: p["IDENT"].as_str()?.trim().to_ascii_uppercase(),
                        region: String::new(),
                        kind: if !navaid { "wpt" } else if class.contains("NDB") { "ndb" } else { "vor" }.into(),
                        lon: c.first()?.as_f64()?,
                        lat: c.get(1)?.as_f64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn ask_faa(idents: &[String]) {
    for chunk in idents.chunks(40) {
        let mut found: HashMap<String, Vec<FixPos>> = chunk.iter().map(|i| (i.clone(), Vec::new())).collect();
        let mut ok = false;
        for (base, fields, navaid) in [(FAA_POINTS, "IDENT", false), (FAA_NAVAIDS, "IDENT,CLASS_TXT", true)] {
            match crate::airspace::fetch_text(&faa_url(base, chunk, fields), "the FAA") {
                Ok(t) => {
                    ok = true;
                    for p in faa_points(&t, navaid) {
                        found.entry(p.ident.clone()).or_default().push(p);
                    }
                }
                Err(e) => log::warn!("fixes: {e}"),
            }
        }
        // Not reachable: ask again another time.
        if ok {
            FAA.lock().unwrap().get_or_insert_with(Default::default).extend(found);
        }
    }
}

/// Every known position for each ident: the sim's nav data first, then
/// the FAA for idents the sim's data on this PC does not have.
pub fn resolve(nav: &navaids::Shared, dir: &Path, idents: &[String]) -> Vec<FixPos> {
    let idents: Vec<String> = idents.iter().filter_map(|i| clean(i)).collect();
    let mut out = Vec::new();
    let mut missing = Vec::new();
    {
        let mut db = nav.lock().unwrap();
        if !db.loaded {
            db.load(dir);
        }
        for i in &idents {
            let hits = db.find(i);
            if hits.is_empty() {
                missing.push(i.clone());
            }
            out.extend(hits.into_iter().map(|a| FixPos {
                ident: a.ident.clone(),
                region: a.region.clone(),
                kind: match a.kind {
                    telemetry::NavKind::Vor => "vor",
                    telemetry::NavKind::Ndb => "ndb",
                    telemetry::NavKind::Waypoint => "wpt",
                }
                .into(),
                lat: a.lat,
                lon: a.lon,
            }));
        }
    }
    missing.sort();
    missing.dedup();
    let ask: Vec<String> = {
        let faa = FAA.lock().unwrap();
        missing.iter().filter(|i| faa.as_ref().is_none_or(|m| !m.contains_key(*i))).cloned().collect()
    };
    if !ask.is_empty() {
        ask_faa(&ask);
    }
    if let Some(m) = FAA.lock().unwrap().as_ref() {
        for i in &missing {
            out.extend(m.get(i).cloned().unwrap_or_default());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idents_and_faa_answers() {
        assert_eq!(clean(" sebby "), Some("SEBBY".into()));
        assert_eq!(clean("A B"), None);
        assert_eq!(clean("TOOLONG1"), None);
        let u = faa_url(FAA_POINTS, &["SEBBY".into(), "MISEN".into()], "IDENT");
        assert!(u.contains("where=IDENT%20IN%20%28%27SEBBY%27%2C%27MISEN%27%29"));
        let t = r#"{"type":"FeatureCollection","features":[{"type":"Feature","geometry":{"type":"Point","coordinates":[-116.9,33.1]},"properties":{"IDENT":"SEBBY"}}]}"#;
        let p = faa_points(t, false);
        assert_eq!(p.len(), 1);
        assert_eq!((p[0].ident.as_str(), p[0].kind.as_str(), p[0].lat), ("SEBBY", "wpt", 33.1));
        let n = r#"{"features":[{"geometry":{"coordinates":[-117.0,32.8]},"properties":{"IDENT":"SLI","CLASS_TXT":"H-VORTACW"}}]}"#;
        assert_eq!(faa_points(n, true)[0].kind, "vor");
    }
}
