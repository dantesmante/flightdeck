//! Obstacles on the map: towers, antennas, wind turbines, stacks and the
//! like, from the FAA's Digital Obstacle File (the daily DOF, US government
//! data, public domain, no key). It covers the US, with some of Canada,
//! Mexico, the Caribbean and the Pacific; there is no free worldwide file.
//!
//! The file (DAILY_DOF_CSV.ZIP, tens of MB) is fetched at most once per
//! 28 days. Obstacles of 200 ft or more above the ground are kept (as on
//! sectional charts) and saved as compact GeoJSON in FlightDeck's data
//! folder, which the map reads until the next fetch.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{json, Value};

const URL: &str = "https://aeronav.faa.gov/Obst_Data/DAILY_DOF_CSV.ZIP";
/// Lower obstacles are not charted and would crowd the map.
const MIN_AGL_FT: f64 = 200.0;

fn folder(data_dir: &Path) -> PathBuf {
    data_dir.join("obstacles")
}

static BUSY: Mutex<()> = Mutex::new(());

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// The obstacles as GeoJSON points: t (kind: "tower", "wind", "stack",
/// "building", "bridge", "other"), agl and amsl (ft), lit (bool), n (how
/// many at the spot).
pub fn us(data_dir: &Path) -> Result<String, String> {
    let _one = BUSY.lock().unwrap_or_else(|e| e.into_inner());
    let dir = folder(data_dir);
    // Same 28-day rhythm as the airspace.
    let cycle = crate::airspace::cycle_at(now_ms());
    let file = dir.join(format!("us-{cycle}.json"));
    if let Ok(s) = std::fs::read_to_string(&file) {
        return Ok(s);
    }
    let fetched = fetch().and_then(|zip| from_zip(&zip));
    match fetched {
        Ok(json) => {
            let _ = std::fs::create_dir_all(&dir);
            if std::fs::write(&file, &json).is_ok() {
                for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                    if e.path() != file {
                        let _ = std::fs::remove_file(e.path());
                    }
                }
            }
            log::info!("obstacles: {} KB", json.len() / 1024);
            Ok(json)
        }
        Err(e) => {
            // Offline: the last copy kept, however old.
            let old = std::fs::read_dir(&dir).ok().and_then(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).max());
            match old {
                Some(p) => {
                    log::warn!("obstacles: {e}; showing {}", p.display());
                    std::fs::read_to_string(p).map_err(|e| e.to_string())
                }
                None => Err(e),
            }
        }
    }
}

fn fetch() -> Result<Vec<u8>, String> {
    let mut res = crate::update::agent().get(URL).call().map_err(|e| format!("Could not reach the FAA (aeronav.faa.gov): {e}"))?;
    res.body_mut()
        .with_config()
        .limit(300 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| format!("Unexpected answer from the FAA: {e}"))
}

pub fn from_zip(bytes: &[u8]) -> Result<String, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("The FAA file is not a zip: {e}"))?;
    let name = zip
        .file_names()
        .filter_map(|n| n.ok().map(|n| n.to_string()))
        .find(|n| n.to_ascii_lowercase().ends_with(".csv"))
        .ok_or("No CSV in the FAA obstacle file")?;
    let mut raw = Vec::new();
    zip.by_name(&name).map_err(|e| e.to_string())?.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&raw);
    let features = parse(&text)?;
    if features.is_empty() {
        return Err("The FAA file had no obstacles".into());
    }
    Ok(json!({ "type": "FeatureCollection", "features": features }).to_string())
}

/// Splits a CSV line, with quoted fields.
fn split(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

fn kind(t: &str) -> &'static str {
    let t = t.to_ascii_uppercase();
    if t.contains("WIND") || t.contains("WINDMILL") {
        "wind"
    } else if t.contains("TOWER") || t.contains("ANTENNA") || t.contains("POLE") || t.contains("MAST") || t.contains("LIGHTHOUSE") || t.contains("RIG") {
        "tower"
    } else if t.contains("STACK") || t.contains("CHIMNEY") {
        "stack"
    } else if t.contains("BLDG") || t.contains("BUILDING") || t.contains("DOME") || t.contains("TANK") {
        "building"
    } else if t.contains("BRIDGE") || t.contains("CATENARY") || t.contains("POWER") || t.contains("CABLE") {
        "bridge"
    } else {
        "other"
    }
}

/// The obstacles from the DOF CSV (columns found by their header names).
pub fn parse(text: &str) -> Result<Vec<Value>, String> {
    let mut lines = text.lines();
    let header: Vec<String> = split(lines.next().ok_or("Empty obstacle file")?).into_iter().map(|h| h.trim().trim_start_matches('\u{feff}').to_ascii_uppercase()).collect();
    let col = |name: &str| header.iter().position(|h| h == name);
    let (Some(lat), Some(lon), Some(agl), Some(amsl)) = (col("LATDEC"), col("LONDEC"), col("AGL"), col("AMSL")) else {
        return Err(format!("The FAA obstacle file has unexpected columns: {}", header.join(", ")));
    };
    let ty = col("TYPE");
    let light = col("LIGHTING");
    let qty = col("QUANTITY");
    let mut out = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let f = split(line);
        let num = |i: usize| f.get(i).and_then(|v| v.trim().parse::<f64>().ok());
        let (Some(la), Some(lo), Some(h), Some(top)) = (num(lat), num(lon), num(agl), num(amsl)) else { continue };
        if h < MIN_AGL_FT || !(-90.0..=90.0).contains(&la) || !(-180.0..=180.0).contains(&lo) {
            continue;
        }
        let t = ty.and_then(|i| f.get(i)).map(|s| kind(s)).unwrap_or("other");
        let lit = light.and_then(|i| f.get(i)).map(|s| !matches!(s.trim(), "N" | "U" | "")).unwrap_or(false);
        let n = qty.and_then(num).unwrap_or(1.0).max(1.0) as i64;
        let r = |v: f64| (v * 1e5).round() / 1e5;
        out.push(json!({
            "type": "Feature",
            "properties": { "t": t, "agl": h.round() as i64, "amsl": top.round() as i64, "lit": lit, "n": n },
            "geometry": { "type": "Point", "coordinates": [r(lo), r(la)] },
        }));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const SAMPLE: &str = "OAS,VERIFIED STATUS,COUNTRY,STATE,CITY,LATDEC,LONDEC,DMSLAT,DMSLON,TYPE,QUANTITY,AGL,AMSL,LIGHTING,ACCURACY,MARKING,FAA STUDY,ACTION,JDATE\n\
27-001879,O,US,MN,AUSTIN ,43.501862,-92.930417,43 30 06.70N,092 55 49.50W,TOWER ,1,315,1486,D, 5D,N,2000AGL06081OE,C,2011327\n\
27-001880,U,US,MN,AUSTIN ,43.5,-92.9,43 30 00.00N,092 54 00.00W,BLDG,1,120,1290,N,9I,N,,A,2011327\n\
19-012345,O,US,IA,\"SOME, PLACE\",42.1,-93.2,x,y,WINDMILL,3,499,1650,M,2C,N,,A,2020001\n";

    #[test]
    fn keeps_charted_obstacles_from_the_dof_csv() {
        let fs = parse(SAMPLE).unwrap();
        assert_eq!(fs.len(), 2, "the 120 ft building is left out");
        assert_eq!(fs[0]["properties"]["t"], "tower");
        assert_eq!(fs[0]["properties"]["amsl"], 1486);
        assert_eq!(fs[0]["properties"]["lit"], true);
        assert_eq!(fs[1]["properties"]["t"], "wind");
        assert_eq!(fs[1]["properties"]["n"], 3);
        assert_eq!(fs[1]["geometry"]["coordinates"][0], -93.2);
    }

    #[test]
    fn reads_the_zip() {
        let mut z = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut z));
            w.start_file("DOF.CSV", zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(SAMPLE.as_bytes()).unwrap();
            w.finish().unwrap();
        }
        let v: Value = serde_json::from_str(&from_zip(&z).unwrap()).unwrap();
        assert_eq!(v["features"].as_array().unwrap().len(), 2);
        assert!(parse("A,B\n1,2\n").is_err(), "other columns are refused");
    }
}
