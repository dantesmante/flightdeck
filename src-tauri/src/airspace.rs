//! US airspace on the map: the Class B, C and D areas around airports, from
//! the FAA's NASR 28-day subscription ("class_airspace_shape_files.zip",
//! US government data, public domain, no key). The FAA publishes no free
//! airspace for other countries, so the layer is US only.
//!
//! The file (an ESRI shapefile) is fetched once per 28-day cycle, the B, C
//! and D shapes are kept, simplified to about 20 m, and saved as GeoJSON in
//! FlightDeck's data folder; the map reads that copy until the next cycle.

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

/// One load at a time: a second ask waits for the first download.
static BUSY: Mutex<()> = Mutex::new(());

/// The US Class B, C and D areas as GeoJSON, from this cycle's copy, else
/// fetched now, else the last copy kept. Each feature has: c ("B", "C" or
/// "D"), name, label (as charts print it: ceiling over floor in hundreds
/// of feet, "[25]" for a Class D ceiling).
pub fn us(data_dir: &Path) -> Result<String, String> {
    let _one = BUSY.lock().unwrap_or_else(|e| e.into_inner());
    let dir = folder(data_dir);
    let now = now_ms();
    let cycle = cycle_at(now);
    let file = dir.join(format!("us-{cycle}.json"));
    if let Ok(s) = std::fs::read_to_string(&file) {
        return Ok(s);
    }
    // This cycle, or (just after the change, should the FAA be late) the last.
    let mut last_err = String::new();
    for c in [cycle.clone(), cycle_at(now - CYCLE_MS)] {
        match fetch(&c) {
            Ok(zip) => match from_zip(&zip, &c) {
                Ok(json) => {
                    let _ = std::fs::create_dir_all(&dir);
                    let out = dir.join(format!("us-{c}.json"));
                    if std::fs::write(&out, &json).is_ok() {
                        forget_others(&dir, &out);
                    }
                    log::info!("airspace: US cycle {c}, {} KB", json.len() / 1024);
                    return Ok(json);
                }
                Err(e) => last_err = e,
            },
            Err(e) => last_err = e,
        }
    }
    // Offline: the last copy kept, however old.
    if let Some(old) = newest(&dir) {
        log::warn!("airspace: {last_err}; showing {}", old.display());
        return std::fs::read_to_string(old).map_err(|e| e.to_string());
    }
    Err(last_err)
}

fn fetch(cycle: &str) -> Result<Vec<u8>, String> {
    let mut res = crate::update::agent().get(&url(cycle)).call().map_err(|e| format!("Could not reach the FAA (nfdc.faa.gov): {e}"))?;
    res.body_mut()
        .with_config()
        .limit(200 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| format!("Unexpected answer from the FAA: {e}"))
}

fn newest(dir: &Path) -> Option<PathBuf> {
    let mut all: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("us-") && n.ends_with(".json")))
        .collect();
    all.sort();
    all.pop()
}

fn forget_others(dir: &Path, keep: &Path) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p != keep && p.extension().is_some_and(|x| x == "json") {
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

/// An altitude as charts print it, in hundreds of feet: "SFC", "30", "100".
fn alt(val: Option<String>, uom: Option<String>, code: Option<String>) -> Option<String> {
    let code = code.unwrap_or_default().to_ascii_uppercase();
    let v: f64 = val.as_deref().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    if v <= -9000.0 {
        return None; // not published (up to the airspace above)
    }
    if code == "SFC" && v <= 0.0 {
        return Some("SFC".into());
    }
    let ft = if uom.as_deref() == Some("FL") { v * 100.0 } else { v };
    let h = (ft / 100.0).round() as i64;
    Some(if code == "AGL" || (code == "SFC" && v > 0.0) { format!("{h} AGL") } else { h.to_string() })
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
        let hi = alt(text(&rec, "UPPER_VAL"), text(&rec, "UPPER_UOM"), text(&rec, "UPPER_CODE"));
        let lo = alt(text(&rec, "LOWER_VAL"), text(&rec, "LOWER_UOM"), text(&rec, "LOWER_CODE"));
        let label = match (class, hi, lo) {
            ("D", Some(h), _) => format!("[{h}]"),
            (_, Some(h), Some(l)) => format!("{h}/{l}"),
            (_, Some(h), None) => h,
            _ => String::new(),
        };
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
        out.push(json!({ "type": "Feature", "properties": { "c": class, "name": name, "label": label }, "geometry": geometry }));
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
    fn altitudes_read_as_on_charts() {
        let s = |v: &str| Some(v.to_string());
        assert_eq!(alt(s("10000"), s("FT"), s("MSL")).as_deref(), Some("100"));
        assert_eq!(alt(s("0"), s("FT"), s("SFC")).as_deref(), Some("SFC"));
        assert_eq!(alt(s("-9998"), None, None), None);
        assert_eq!(alt(s("180"), s("FL"), s("STD")).as_deref(), Some("180"));
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
        assert_eq!(fs[0]["properties"]["label"], "100/30");
        assert_eq!(fs[1]["properties"]["label"], "[25]");
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
