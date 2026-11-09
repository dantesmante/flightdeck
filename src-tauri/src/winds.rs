//! Winds aloft for the map's wind layer, from NOAA's Global Forecast System
//! (GFS): US government data, public domain, served free and keyless from
//! the NOAA Open Data Dissemination bucket on AWS.
//!
//! FlightDeck takes the one-degree GFS file nearest to now and, for the one
//! level shown, only the two wind fields (east and north components), by
//! byte range: about 100 KB per level, at most once per forecast step
//! (three hours). Everything else happens on this PC's processor, and only
//! while the layer is on.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use grib::LatLons;
use serde::Serialize;

const BUCKET: &str = "https://noaa-gfs-bdp-pds.s3.amazonaws.com";
/// How often to look for a newer forecast.
const RECHECK: Duration = Duration::from_secs(30 * 60);
const MS_PER_KT: f32 = 1.943_844;

/// The levels offered, as FlightDeck names them and as the GFS index does.
pub const LEVELS: &[(&str, &str)] = &[
    ("sfc", "10 m above ground"),
    ("850", "850 mb"),
    ("700", "700 mb"),
    ("500", "500 mb"),
    ("400", "400 mb"),
    ("300", "300 mb"),
    ("250", "250 mb"),
    ("200", "200 mb"),
];

/// One level of wind on a one-degree grid.
#[derive(Debug, Clone, Serialize, Default)]
pub struct WindGrid {
    pub level: String,
    /// When the forecast run started, and the time it is for.
    pub run_ms: i64,
    pub valid_ms: i64,
    /// Columns and rows: longitudes 0 to 359 east, latitudes 90 north to 90 south.
    pub nx: usize,
    pub ny: usize,
    /// East and north components in knots, row by row from the north, as
    /// little-endian 16-bit integers, base64.
    pub u: String,
    pub v: String,
    pub error: Option<String>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn get(url: &str, range: Option<(u64, Option<u64>)>) -> Result<Vec<u8>, String> {
    let mut req = crate::update::agent().get(url);
    if let Some((a, b)) = range {
        req = req.header("Range", &match b {
            Some(b) => format!("bytes={a}-{b}"),
            None => format!("bytes={a}-"),
        });
    }
    let mut res = req.call().map_err(|e| format!("Could not reach NOAA's forecast server: {e}"))?;
    res.body_mut()
        .with_config()
        .limit(16 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| format!("Unexpected answer from NOAA's forecast server: {e}"))
}

/// A forecast run (start, Unix ms) and step (hours) for a time: the newest
/// run likely to be published (they appear about four hours after the run
/// starts), `back` runs earlier, and the 3-hour step nearest that time.
pub fn run_for(now_ms: i64, back: i64) -> (i64, i64) {
    const H: i64 = 3_600_000;
    let run = ((now_ms - 4 * H).div_euclid(6 * H) - back) * 6 * H;
    let step = (((now_ms - run) as f64 / (3 * H) as f64).round() as i64 * 3).clamp(0, 384);
    (run, step)
}

fn file_url(run_ms: i64, step: i64) -> String {
    let t = chrono_like(run_ms);
    format!("{BUCKET}/gfs.{}/{:02}/atmos/gfs.t{:02}z.pgrb2.1p00.f{:03}", t.0, t.1, t.1, step)
}

/// (yyyymmdd, hour) of a Unix time in UTC.
fn chrono_like(ms: i64) -> (String, i64) {
    let days = ms.div_euclid(86_400_000);
    let hour = ms.rem_euclid(86_400_000) / 3_600_000;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (format!("{y:04}{m:02}{d:02}"), hour)
}

/// Byte range of a field in a GFS index (".idx") file: lines like
/// `12:345678:d=2026110406:UGRD:850 mb:3 hour fcst:`.
pub fn find_range(idx: &str, var: &str, level: &str) -> Option<(u64, Option<u64>)> {
    let lines: Vec<&str> = idx.lines().filter(|l| !l.trim().is_empty()).collect();
    for (i, l) in lines.iter().enumerate() {
        let f: Vec<&str> = l.split(':').collect();
        if f.len() >= 5 && f[3] == var && f[4] == level {
            let start: u64 = f[1].parse().ok()?;
            let end = lines.get(i + 1).and_then(|n| n.split(':').nth(1)?.parse::<u64>().ok()).map(|n| n - 1);
            return Some((start, end));
        }
    }
    None
}

/// The values of a one-message GRIB2 field on the one-degree grid (360 x
/// 181, from 90N and 0E), in the field's own unit.
pub fn decode_field(bytes: &[u8]) -> Result<Vec<f32>, String> {
    let g = grib::from_bytes(bytes).map_err(|e| format!("Unreadable forecast field: {e}"))?;
    let (_, sub) = g.iter().next().ok_or("Empty forecast field")?;
    let points: Vec<(f64, f64)> = sub.latlons().map_err(|e| format!("Unexpected forecast grid: {e}"))?.collect();
    let dec = grib::Grib2SubmessageDecoder::from(sub).map_err(|e| format!("Unreadable forecast field: {e}"))?;
    let values = dec.dispatch().map_err(|e| format!("Unreadable forecast field: {e}"))?;
    let mut out = vec![f32::NAN; 360 * 181];
    for ((lat, lon), v) in points.into_iter().zip(values) {
        let row = (90.0 - lat).round() as i64;
        let lon = lon.rem_euclid(360.0);
        let col = lon.round() as i64 % 360;
        // Only the whole-degree points (a finer grid is thinned to one degree).
        if (0..181).contains(&row) && (lat - (90.0 - row as f64)).abs() < 0.01 && (lon - lon.round()).abs() < 0.01 {
            out[row as usize * 360 + col as usize] = v;
        }
    }
    Ok(out)
}

fn pack(knots: &[f32]) -> String {
    let mut b = Vec::with_capacity(knots.len() * 2);
    for k in knots {
        let v = if k.is_finite() { k.round().clamp(-30_000.0, 30_000.0) as i16 } else { 0 };
        b.extend_from_slice(&v.to_le_bytes());
    }
    base64(&b)
}

fn base64(b: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n = (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        s.push(A[(n >> 18) as usize & 63] as char);
        s.push(A[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 { A[(n >> 6) as usize & 63] as char } else { '=' });
        s.push(if c.len() > 2 { A[n as usize & 63] as char } else { '=' });
    }
    s
}

struct Cache {
    /// The run and step in use, and when they were last checked.
    current: Option<(i64, i64, String)>,
    checked: Option<Instant>,
    grids: HashMap<(i64, i64, String), WindGrid>,
}

fn cache() -> &'static Mutex<Cache> {
    static C: OnceLock<Mutex<Cache>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(Cache { current: None, checked: None, grids: HashMap::new() }))
}

/// The newest published forecast for about now: (run, step, index text).
fn find_forecast() -> Result<(i64, i64, String), String> {
    let now = now_ms();
    let mut last = String::new();
    for back in 0..3 {
        let (run, step) = run_for(now, back);
        match get(&format!("{}.idx", file_url(run, step)), None) {
            Ok(b) => return Ok((run, step, String::from_utf8_lossy(&b).into_owned())),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// The forecast in use (run, step, index), looking for a newer one now and
/// then; clears the stored grids when it changes.
fn current(c: &mut Cache) -> Result<(i64, i64, String), String> {
    if c.current.is_none() || c.checked.is_none_or(|t| t.elapsed() > RECHECK) {
        match find_forecast() {
            Ok(f) => {
                if c.current.as_ref().is_none_or(|(r, s, _)| (*r, *s) != (f.0, f.1)) {
                    c.grids.clear();
                }
                c.current = Some(f);
                c.checked = Some(Instant::now());
            }
            Err(e) => {
                log::warn!("winds: {e}");
                // Keep what we have; try again in a few minutes.
                c.checked = Some(Instant::now() - RECHECK + Duration::from_secs(300));
                if c.current.is_none() {
                    return Err(e);
                }
            }
        }
    }
    Ok(c.current.clone().unwrap())
}

/// Wind at one level ("sfc", "850", ... "200"), at most one forecast step old.
pub fn wind(level: &str) -> WindGrid {
    let Some(&(_, idx_level)) = LEVELS.iter().find(|(k, _)| *k == level) else {
        return WindGrid { level: level.into(), error: Some("Unknown wind level".into()), ..Default::default() };
    };
    let mut c = cache().lock().unwrap();
    let (run, step, idx) = match current(&mut c) {
        Ok(f) => f,
        Err(e) => return WindGrid { level: level.into(), error: Some(e), ..Default::default() },
    };
    let key = (run, step, level.to_string());
    if let Some(g) = c.grids.get(&key) {
        return g.clone();
    }
    let url = file_url(run, step);
    let field = |var: &str| -> Result<Vec<f32>, String> {
        let range = find_range(&idx, var, idx_level).ok_or("This level is missing from the forecast")?;
        decode_field(&get(&url, Some(range))?)
    };
    match field("UGRD").and_then(|u| Ok((u, field("VGRD")?))) {
        Ok((u, v)) => {
            let kt = |f: Vec<f32>| f.into_iter().map(|x| x * MS_PER_KT).collect::<Vec<_>>();
            let g = WindGrid {
                level: level.into(),
                run_ms: run,
                valid_ms: run + step * 3_600_000,
                nx: 360,
                ny: 181,
                u: pack(&kt(u)),
                v: pack(&kt(v)),
                error: None,
            };
            c.grids.insert(key, g.clone());
            g
        }
        Err(e) => {
            log::warn!("winds {level}: {e}");
            WindGrid { level: level.into(), error: Some(e), ..Default::default() }
        }
    }
}

/// Air temperature 2 m above the ground on the one-degree grid, for the
/// map's temperature layer. Same forecast file as the wind (one field,
/// about 60 KB), at most once per forecast step.
#[derive(Debug, Clone, Serialize, Default)]
pub struct TempGrid {
    pub run_ms: i64,
    pub valid_ms: i64,
    pub nx: usize,
    pub ny: usize,
    /// Tenths of a degree Celsius, row by row from 90N, columns from 0E, as
    /// little-endian 16-bit integers, base64.
    pub t: String,
    pub error: Option<String>,
}

pub fn temperature() -> TempGrid {
    static GRID: OnceLock<Mutex<Option<(i64, i64, TempGrid)>>> = OnceLock::new();
    let slot = GRID.get_or_init(|| Mutex::new(None));
    let (run, step, idx) = {
        let mut c = cache().lock().unwrap();
        match current(&mut c) {
            Ok(f) => f,
            Err(e) => return TempGrid { error: Some(e), ..Default::default() },
        }
    };
    let mut held = slot.lock().unwrap();
    if let Some((r, s, g)) = held.as_ref() {
        if (*r, *s) == (run, step) {
            return g.clone();
        }
    }
    let got = find_range(&idx, "TMP", "2 m above ground")
        .ok_or_else(|| "Temperature is missing from the forecast".to_string())
        .and_then(|range| decode_field(&get(&file_url(run, step), Some(range))?));
    match got {
        Ok(k) => {
            let tenths: Vec<f32> = k.into_iter().map(|x| if x.is_finite() { (x - 273.15) * 10.0 } else { f32::NAN }).collect();
            let g = TempGrid { run_ms: run, valid_ms: run + step * 3_600_000, nx: 360, ny: 181, t: pack(&tenths), error: None };
            *held = Some((run, step, g.clone()));
            g
        }
        Err(e) => {
            log::warn!("temperature: {e}");
            TempGrid { error: Some(e), ..Default::default() }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_and_steps() {
        // 2026-11-04 15:10Z: the 06Z run is out (after 10Z), step +9 h.
        let now = 1_793_805_000_000; // 2026-11-04T15:10:00Z
        let (run, step) = run_for(now, 0);
        assert_eq!(chrono_like(run), ("20261104".into(), 6));
        assert_eq!(step, 9);
        let (run, step) = run_for(now, 1);
        assert_eq!(chrono_like(run), ("20261104".into(), 0));
        assert_eq!(step, 15);
        assert!(file_url(run, step).ends_with("/gfs.20261104/00/atmos/gfs.t00z.pgrb2.1p00.f015"));
    }

    #[test]
    fn index_ranges() {
        let idx = "1:0:d=2026110406:PRMSL:mean sea level:9 hour fcst:\n\
                   2:1000:d=2026110406:UGRD:850 mb:9 hour fcst:\n\
                   3:2500:d=2026110406:VGRD:850 mb:9 hour fcst:\n\
                   4:4000:d=2026110406:UGRD:10 m above ground:9 hour fcst:\n";
        assert_eq!(find_range(idx, "UGRD", "850 mb"), Some((1000, Some(2499))));
        assert_eq!(find_range(idx, "UGRD", "10 m above ground"), Some((4000, None)));
        assert_eq!(find_range(idx, "VGRD", "500 mb"), None);
    }

    #[test]
    fn base64_matches() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(pack(&[1.0, -2.0]), base64(&[1, 0, 254, 255]));
    }
}

#[cfg(test)]
mod sample {
    /// Decodes a real GFS field, given in FD_GRIB_SAMPLE (not in the repo).
    #[test]
    #[ignore]
    fn decodes_gfs() {
        let path = std::env::var("FD_GRIB_SAMPLE").unwrap();
        let v = super::decode_field(&std::fs::read(path).unwrap()).unwrap();
        let n = v.iter().filter(|x| x.is_finite()).count();
        let max = v.iter().filter(|x| x.is_finite()).fold(0f32, |a, b| a.max(b.abs()));
        println!("{n} values, max {max}");
        assert!(n > 60_000);
    }
}
