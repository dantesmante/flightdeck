//! Icing Potential (phase 1): an estimate, from NOAA's GFS forecast, of
//! how likely an aircraft is to pick up ice at a place and flight level.
//! For flight simulation only: it is not a validated icing forecast and is
//! never to be used for real flying.
//!
//! The pipeline, kept in separate steps so the scoring can change without
//! touching the download:
//!
//! 1. **Download** ([`load`]): from the same GFS run and step as the wind
//!    layer, the fields below on 20 pressure levels (1000–150 hPa), read
//!    by byte range from the one-degree file (the half-degree file, thinned
//!    to one degree, when the one-degree file lacks cloud water). About 120
//!    small fields, kept in memory per forecast step.
//!    TMP temperature, RH relative humidity, CLWMR cloud liquid water,
//!    ICMR cloud ice, VVEL vertical velocity (Pa/s), HGT geopotential
//!    height; and HGT at the surface (terrain).
//! 2. **Vertical interpolation** ([`Column::at_pressure`]): a flight level
//!    is a pressure altitude, a surface of constant pressure in the
//!    standard atmosphere (FL180 is 506 hPa wherever you are), so a flight
//!    level is turned into its standard pressure and the fields are
//!    interpolated linearly in the logarithm of pressure between the two
//!    levels around it. Where that surface lies under the ground (its
//!    height, from HGT, is below the terrain) or outside the levels read,
//!    the answer is "unknown", never a guess. Altitudes above sea level
//!    (below the transition level) go through HGT the same way.
//! 3. **Scoring** ([`score`]): see its notes. 0–100, `None` when unknown.
//! 4. **Grid** ([`grid`]): the score at one flight level for every degree
//!    of the world (360 × 181), 255 where unknown.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;

use crate::winds;

/// Pressure levels read (hPa), surface upwards. Cloud water and ice exist
/// in GFS on these (not on the levels above 150 hPa we would never need).
pub const LEVELS_HPA: [f32; 20] = [
    1000.0, 975.0, 950.0, 925.0, 900.0, 850.0, 800.0, 750.0, 700.0, 650.0, 600.0, 550.0, 500.0, 450.0, 400.0, 350.0, 300.0, 250.0, 200.0, 150.0,
];
const VARS: [&str; 6] = ["TMP", "RH", "CLWMR", "ICMR", "VVEL", "HGT"];
pub const NX: usize = 360;
pub const NY: usize = 181;

/// All fields of one forecast step, one degree, rows from 90N, columns from 0E.
pub struct Cube {
    pub run_ms: i64,
    pub valid_ms: i64,
    /// "1p00" or "0p50" (thinned).
    pub product: &'static str,
    /// [var][level] → values (VARS order), in the file's units.
    fields: Vec<Vec<Vec<f32>>>,
    /// Terrain height (m).
    surface: Vec<f32>,
}

/// One place's column of values on the pressure levels.
pub struct Column {
    /// Per level: T (°C), RH (%), cloud liquid (g/kg), cloud ice (g/kg), ω (Pa/s), height (m).
    pub levels: Vec<[f32; 6]>,
    pub surface_m: f32,
}

/// The conditions at one point and level, and the score.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Conditions {
    pub pressure_hpa: f32,
    pub height_m: f32,
    pub temp_c: f32,
    pub rh_pct: f32,
    pub cloud_liquid_gkg: f32,
    pub cloud_ice_gkg: f32,
    /// Vertical velocity, Pa/s: negative is rising air.
    pub omega_pas: f32,
    /// 0–100; None when the data does not allow a score.
    pub potential: Option<u8>,
}

/// Standard-atmosphere pressure (hPa) of a pressure altitude (ft).
pub fn isa_pressure(ft: f64) -> f64 {
    if ft <= 36_089.0 {
        1013.25 * (1.0 - 6.875_6e-6 * ft).powf(5.255_9)
    } else {
        226.32 * (-(ft - 36_089.0) / 20_806.0).exp()
    }
}

impl Cube {
    fn idx(lat: f64, lon: f64) -> (usize, usize) {
        let row = (90.0 - lat).round().clamp(0.0, (NY - 1) as f64) as usize;
        let col = (lon.rem_euclid(360.0).round() as usize) % NX;
        (row, col)
    }

    /// The column at a grid point. Units turned into °C, g/kg.
    pub fn column(&self, row: usize, col: usize) -> Column {
        let i = row * NX + col;
        let levels = (0..LEVELS_HPA.len())
            .map(|k| {
                let v = |n: usize| self.fields[n][k][i];
                [v(0) - 273.15, v(1), v(2) * 1000.0, v(3) * 1000.0, v(4), v(5)]
            })
            .collect();
        Column { levels, surface_m: self.surface[i] }
    }

    pub fn column_at(&self, lat: f64, lon: f64) -> Column {
        let (r, c) = Self::idx(lat, lon);
        self.column(r, c)
    }
}

impl Column {
    /// Values at a pressure (hPa), interpolated in log pressure; None
    /// outside the levels, under the ground, or where a value is missing.
    pub fn at_pressure(&self, p: f32) -> Option<Conditions> {
        let k = LEVELS_HPA.windows(2).position(|w| p <= w[0] && p >= w[1])?;
        let (p0, p1) = (LEVELS_HPA[k], LEVELS_HPA[k + 1]);
        let t = if p0 == p1 { 0.0 } else { (p0.ln() - p.ln()) / (p0.ln() - p1.ln()) };
        let a = &self.levels[k];
        let b = &self.levels[k + 1];
        let mut v = [0f32; 6];
        for n in 0..6 {
            if !a[n].is_finite() || !b[n].is_finite() {
                return None;
            }
            v[n] = a[n] + (b[n] - a[n]) * t;
        }
        // The surface itself is under the ground here (GFS fills those
        // levels in from below the terrain).
        if !self.surface_m.is_finite() || v[5] < self.surface_m {
            return None;
        }
        let clw = v[2].max(0.0);
        let ice = v[3].max(0.0);
        Some(Conditions {
            pressure_hpa: p,
            height_m: v[5],
            temp_c: v[0],
            rh_pct: v[1].clamp(0.0, 100.0),
            cloud_liquid_gkg: clw,
            cloud_ice_gkg: ice,
            omega_pas: v[4],
            potential: Some(score(v[0], v[1], clw, ice, v[4])),
        })
    }

    /// At a flight level (hundreds of feet, pressure altitude).
    pub fn at_flight_level(&self, fl: f64) -> Option<Conditions> {
        self.at_pressure(isa_pressure(fl * 100.0) as f32)
    }

    /// At an altitude above sea level (ft), through the heights of the levels.
    #[allow(dead_code)] // For "follow the aircraft" below the transition level (phase 3).
    pub fn at_altitude(&self, ft: f64) -> Option<Conditions> {
        let m = (ft * 0.3048) as f32;
        let k = self.levels.windows(2).position(|w| w[0][5].is_finite() && w[1][5].is_finite() && m >= w[0][5] && m <= w[1][5])?;
        let (z0, z1) = (self.levels[k][5], self.levels[k + 1][5]);
        let f = if z1 == z0 { 0.0 } else { (m - z0) / (z1 - z0) };
        let lp = LEVELS_HPA[k].ln() + (LEVELS_HPA[k + 1].ln() - LEVELS_HPA[k].ln()) * f;
        self.at_pressure(lp.exp())
    }
}

/// The Icing Potential score, 0–100, for temperature (°C), relative
/// humidity (%), cloud liquid and ice (g/kg) and vertical velocity (Pa/s,
/// negative rising). A transparent rule of thumb, to be tuned later
/// against established products (such as CIP/FIP); not a severity.
///
/// - **Cloud is required.** Humid air (relative humidity from 70 %, all
///   of it from 90 %) or any cloud liquid (from 0.001 g/kg) counts as
///   cloud; dry air scores 0 however cold. GFS keeps cloud water very low
///   in layered cloud, and two comparisons with AWC (CIP on 9 Nov, WAFS
///   icing on 9 Nov 2026) showed that requiring liquid missed most icing,
///   so humidity finds the cloud as the established methods (GFIP) do.
/// - **Liquid raises it:** a saturated cloud with no liquid in the
///   forecast counts 45 %, rising on a logarithmic scale of cloud liquid
///   to 100 % at 0.3 g/kg (0.01 g/kg gives 67 %, 0.1 g/kg 89 %).
/// - **Temperature** sets how much of that counts: nothing above 0 °C or
///   below −40 °C (no supercooled water); most between −2 and −12 °C;
///   tapering to −20 °C and on to −40 °C.
/// - **Phase:** the share of the cloud's water that is liquid; a mostly
///   glaciated cloud scores lower than an all-liquid one (×0.6 to ×1).
/// - **Rising air** supports a little (×0.85 to ×1.15) and can never make
///   a score on its own.
pub fn score(t_c: f32, rh: f32, clw_gkg: f32, ice_gkg: f32, omega: f32) -> u8 {
    if !(t_c < 0.0 && t_c > -40.0) {
        return 0;
    }
    let liquid = clw_gkg >= 0.001;
    let humid = ((rh - 70.0) / 20.0).clamp(0.0, 1.0);
    let cloud = if liquid { humid.max(0.8) } else { humid };
    if cloud <= 0.0 {
        return 0;
    }
    let f_l = if liquid { ((clw_gkg / 0.001).log10() / (0.3f32 / 0.001).log10()).clamp(0.0, 1.0) } else { 0.0 };
    let f_liq = 0.45 + 0.55 * f_l;
    let f_t = match t_c {
        t if t > -2.0 => 0.5 + 0.5 * (-t / 2.0),
        t if t >= -12.0 => 1.0,
        t if t >= -20.0 => 1.0 - 0.4 * ((-12.0 - t) / 8.0),
        t if t >= -30.0 => 0.6 - 0.4 * ((-20.0 - t) / 10.0),
        t => 0.2 * ((40.0 + t) / 10.0),
    };
    let (w, i) = (clw_gkg.max(0.0), ice_gkg.max(0.0));
    let lf = if w + i > 0.0 { w / (w + i) } else { 1.0 };
    let f_p = 0.6 + 0.4 * lf;
    let f_w = 1.0 + (-omega / 0.5).clamp(-1.0, 1.0) * 0.15;
    (100.0 * cloud * f_liq * f_t * f_p * f_w).round().clamp(0.0, 100.0) as u8
}

// ---- Download -----------------------------------------------------------------

fn held() -> &'static Mutex<Option<Arc<Cube>>> {
    static C: OnceLock<Mutex<Option<Arc<Cube>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

/// The fields of the forecast in use, downloaded once per forecast step
/// (the last good one is kept if a new download fails).
pub fn load() -> Result<Arc<Cube>, String> {
    let (run, step, idx1) = winds::forecast()?;
    if let Some(c) = held().lock().unwrap().as_ref() {
        if c.run_ms == run && c.valid_ms == run + step * 3_600_000 {
            return Ok(c.clone());
        }
    }
    let fresh = download(run, step, &idx1);
    let mut h = held().lock().unwrap();
    match fresh {
        Ok(c) => {
            let c = Arc::new(c);
            *h = Some(c.clone());
            Ok(c)
        }
        Err(e) => {
            log::warn!("icing: {e}");
            h.clone().ok_or(e)
        }
    }
}

fn download(run: i64, step: i64, idx1: &str) -> Result<Cube, String> {
    // Cloud water ("CLWMR", or "CLMR" in some GFS versions) is not in the
    // one-degree file (checked 9 Nov 2026: ICMR is, CLWMR is not), so the
    // half-degree file (about four times the bytes, thinned to one degree)
    // is used when needed.
    let cloud = |idx: &str| ["CLWMR", "CLMR"].into_iter().find(|v| winds::find_range(idx, v, "500 mb").is_some());
    let (product, idx, cloud_var) = if let Some(v) = cloud(idx1) {
        ("1p00", idx1.to_string(), v)
    } else {
        let url = winds::product_url(run, step, "0p50");
        let idx = String::from_utf8_lossy(&winds::get_range(&format!("{url}.idx"), None)?).into_owned();
        let v = cloud(&idx).ok_or("Cloud water is missing from this forecast")?;
        ("0p50", idx, v)
    };
    let url = winds::product_url(run, step, product);
    let mut jobs: Vec<(usize, usize, (u64, Option<u64>))> = Vec::new();
    for (n, var) in VARS.iter().enumerate() {
        let var = if *var == "CLWMR" { cloud_var } else { var };
        for (k, p) in LEVELS_HPA.iter().enumerate() {
            let level = format!("{} mb", *p as i32);
            let r = winds::find_range(&idx, var, &level).ok_or_else(|| format!("{var} at {level} is missing from the forecast"))?;
            jobs.push((n, k, r));
        }
    }
    let surf = winds::find_range(&idx, "HGT", "surface").ok_or("Terrain height is missing from the forecast")?;
    let t0 = std::time::Instant::now();
    // A few at a time: about 120 small requests.
    let results: Mutex<HashMap<(usize, usize), Vec<f32>>> = Mutex::default();
    let queue = Mutex::new(jobs.clone());
    let failed: Mutex<Option<String>> = Mutex::default();
    std::thread::scope(|s| {
        for _ in 0..3 {
            s.spawn(|| loop {
                let Some((n, k, r)) = queue.lock().unwrap().pop() else { break };
                if failed.lock().unwrap().is_some() {
                    break;
                }
                match winds::get_range(&url, Some(r)).and_then(|b| winds::decode_field(&b)) {
                    Ok(v) => {
                        results.lock().unwrap().insert((n, k), v);
                    }
                    Err(e) => *failed.lock().unwrap() = Some(e),
                }
            });
        }
    });
    if let Some(e) = failed.into_inner().unwrap() {
        return Err(e);
    }
    let mut res = results.into_inner().unwrap();
    let surface = winds::decode_field(&winds::get_range(&url, Some(surf))?)?;
    let fields = (0..VARS.len()).map(|n| (0..LEVELS_HPA.len()).map(|k| res.remove(&(n, k)).unwrap_or_default()).collect()).collect();
    log::info!("icing: {} fields from GFS {product} in {:.1} s", jobs.len() + 1, t0.elapsed().as_secs_f32());
    Ok(Cube { run_ms: run, valid_ms: run + step * 3_600_000, product, fields, surface })
}

// ---- What the app asks for ------------------------------------------------------

#[derive(Serialize)]
pub struct IcingGrid {
    pub run_ms: i64,
    pub valid_ms: i64,
    pub fl: u32,
    /// The GFS file used: "1p00" or "0p50".
    pub product: &'static str,
    pub nx: usize,
    pub ny: usize,
    /// One byte per degree, rows from 90N, columns from 0E: 0–100, 255
    /// unknown. Base64.
    pub potential: String,
    /// Share of the world with a score (the rest unknown: terrain, gaps).
    pub known: f32,
}

/// Icing Potential at a flight level for the whole world.
pub fn grid(fl: u32) -> Result<IcingGrid, String> {
    let c = load()?;
    let p = isa_pressure(f64::from(fl) * 100.0) as f32;
    let mut out = vec![255u8; NX * NY];
    let mut known = 0usize;
    for row in 0..NY {
        for col in 0..NX {
            if let Some(v) = c.column(row, col).at_pressure(p).and_then(|x| x.potential) {
                out[row * NX + col] = v;
                known += 1;
            }
        }
    }
    Ok(IcingGrid { run_ms: c.run_ms, valid_ms: c.valid_ms, fl, product: c.product, nx: NX, ny: NY, potential: winds::base64_pub(&out), known: known as f32 / (NX * NY) as f32 })
}

#[derive(Serialize)]
pub struct IcingPoint {
    pub run_ms: i64,
    pub valid_ms: i64,
    pub lat: f64,
    pub lon: f64,
    pub fl: u32,
    pub product: &'static str,
    /// None where the level is under the ground or data is missing.
    pub at: Option<Conditions>,
}

/// Everything at one place and flight level (for checking, and later the
/// map's popup).
pub fn point(lat: f64, lon: f64, fl: u32) -> Result<IcingPoint, String> {
    let c = load()?;
    let at = c.column_at(lat, lon).at_flight_level(f64::from(fl));
    Ok(IcingPoint { run_ms: c.run_ms, valid_ms: c.valid_ms, lat, lon, fl, product: c.product, at })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_pressures_of_flight_levels() {
        assert!((isa_pressure(0.0) - 1013.25).abs() < 0.01);
        assert!((isa_pressure(18_000.0) - 505.98).abs() < 0.5);
        assert!((isa_pressure(35_000.0) - 238.4).abs() < 0.5);
        assert!((isa_pressure(40_000.0) - 187.5).abs() < 0.5);
    }

    #[test]
    fn dry_air_no_icing() {
        // Ideal temperature, rising, but dry: nothing.
        assert_eq!(score(-8.0, 65.0, 0.0, 0.5, -1.0), 0);
        assert_eq!(score(-8.0, 70.0, 0.0009, 0.0, -1.0), 0);
        // Cloud, but above freezing or too cold for supercooled water.
        assert_eq!(score(2.0, 100.0, 0.3, 0.0, 0.0), 0);
        assert_eq!(score(-42.0, 100.0, 0.3, 0.0, 0.0), 0);
    }

    #[test]
    fn humid_cloud_without_forecast_liquid_is_light_to_moderate() {
        let sat = score(-8.0, 100.0, 0.0, 0.0, 0.0);
        assert!((41..=50).contains(&sat), "{sat}");
        let damp = score(-8.0, 80.0, 0.0, 0.0, 0.0);
        assert!((6..sat).contains(&damp), "{damp}");
        assert!(score(-8.0, 71.0, 0.0, 0.0, 0.0) < 6, "barely humid stays below the lowest band");
        // All ice: lower again.
        assert!(score(-8.0, 100.0, 0.0, 0.3, 0.0) < sat);
    }

    #[test]
    fn scores_rise_with_liquid_and_peak_in_the_icing_band() {
        let at = |t, clw| score(t, 95.0, clw, 0.0, 0.0);
        assert!(at(-8.0, 0.05) < at(-8.0, 0.2));
        assert!(at(-8.0, 0.2) < at(-8.0, 0.5) || at(-8.0, 0.5) >= 90);
        assert!(at(-8.0, 0.3) > at(-25.0, 0.3));
        assert!(at(-25.0, 0.3) > at(-35.0, 0.3));
        assert!(at(-8.0, 0.3) >= 80, "rich supercooled cloud at -8 °C scores high: {}", at(-8.0, 0.3));
        // Mostly ice: lower than all liquid.
        assert!(score(-10.0, 95.0, 0.1, 0.4, 0.0) < score(-10.0, 95.0, 0.1, 0.0, 0.0));
        // Thin layered cloud with a little liquid still shows as light icing.
        assert!(score(-8.0, 95.0, 0.02, 0.0, 0.0) >= 41);
        // Humidity and lift only nudge.
        let base = score(-10.0, 95.0, 0.1, 0.0, 0.0);
        assert!(score(-10.0, 50.0, 0.1, 0.0, 0.0) < base);
        assert!(score(-10.0, 95.0, 0.1, 0.0, -1.0) > base);
        assert!(score(-10.0, 95.0, 0.1, 0.0, -1.0) <= (f32::from(base) * 1.16).ceil() as u8);
    }

    /// A made-up column: standard-ish temperatures, a supercooled cloud
    /// layer between 700 and 600 hPa.
    fn column(surface_m: f32) -> Column {
        let levels = LEVELS_HPA
            .iter()
            .map(|&p| {
                let ft = {
                    // Invert the standard atmosphere (troposphere).
                    (1.0 - (f64::from(p) / 1013.25).powf(1.0 / 5.255_9)) / 6.875_6e-6
                };
                let h = (ft * 0.3048) as f32;
                let t = 15.0 - 0.0065 * h;
                let cloud = (600.0..=700.0).contains(&p);
                [t, if cloud { 98.0 } else { 50.0 }, if cloud { 0.25 } else { 0.0 }, 0.0, if cloud { -0.4 } else { 0.0 }, h]
            })
            .collect();
        Column { levels, surface_m }
    }

    #[test]
    fn interpolates_by_flight_level_and_knows_the_ground() {
        let c = column(0.0);
        // FL120 ≈ 644 hPa: inside the cloud, about -9 °C.
        let x = c.at_flight_level(120.0).unwrap();
        assert!((x.pressure_hpa - 644.0).abs() < 2.0, "{}", x.pressure_hpa);
        assert!((x.temp_c + 8.8).abs() < 1.0, "{}", x.temp_c);
        assert!(x.potential.unwrap() >= 70);
        // FL250: dry air, no icing (known, zero).
        assert_eq!(c.at_flight_level(250.0).unwrap().potential, Some(0));
        // Above the highest level read: unknown.
        assert!(c.at_flight_level(450.0).is_none());
        // On a 3,000 m plateau FL050 is under the ground: unknown, not zero.
        assert!(column(3000.0).at_flight_level(50.0).is_none());
        // An altitude in feet above sea level works through the heights.
        let y = c.at_altitude(12_000.0).unwrap();
        assert!((y.pressure_hpa - 644.0).abs() < 3.0);
        // A missing value makes the level unknown.
        let mut gap = column(0.0);
        for l in gap.levels.iter_mut() {
            l[2] = f32::NAN;
        }
        assert!(gap.at_flight_level(120.0).is_none());
    }
}
