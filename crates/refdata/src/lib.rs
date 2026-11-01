//! Airport lookup by position, plus country and region names.
//!
//! Data comes from OurAirports (public domain) and is embedded in the binary
//! zstd-compressed (about 1.5 MB). Rebuild it with the `build-refdata` tool.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use telemetry::geo;

static AIRPORTS: &[u8] = include_bytes!("../data/airports.tsv.zst");
static COUNTRIES: &[u8] = include_bytes!("../data/countries.tsv.zst");
static REGIONS: &[u8] = include_bytes!("../data/regions.tsv.zst");
static RUNWAYS: &[u8] = include_bytes!("../data/runways.tsv.zst");

/// One runway, both ends.
#[derive(Debug, Clone)]
struct Runway {
    airport: String,
    width_ft: f64,
    ends: [(String, f64, f64); 2],
}

/// Where a touchdown was on a runway.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunwayHit {
    pub airport: String,
    /// The runway landed on, e.g. "34R".
    pub runway: String,
    /// True heading of the runway in the direction of landing.
    pub heading_deg: f64,
    /// 0 when the runway's width is not known.
    pub width_ft: f64,
    /// Distance from the centerline, positive = right of it.
    pub offset_ft: f64,
    /// Distance past the runway's threshold.
    pub along_ft: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AirportKind {
    Large,
    Medium,
    Small,
    Seaplane,
    Heliport,
}

impl AirportKind {
    fn from_code(c: &str) -> Option<Self> {
        Some(match c {
            "L" => AirportKind::Large,
            "M" => AirportKind::Medium,
            "S" => AirportKind::Small,
            "W" => AirportKind::Seaplane,
            "H" => AirportKind::Heliport,
            _ => return None,
        })
    }

    /// Bigger airports win ties: an aircraft on the ramp at a major airport
    /// may be closer to a nearby strip's reference point than to its own.
    fn weight(self) -> f64 {
        match self {
            AirportKind::Large => 3.0,
            AirportKind::Medium => 2.0,
            AirportKind::Small => 1.0,
            AirportKind::Seaplane => 0.8,
            AirportKind::Heliport => 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Airport {
    pub ident: String,
    pub kind: AirportKind,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub elevation_ft: i32,
    pub country: String,
    pub region: String,
}

pub struct RefData {
    airports: Vec<Airport>,
    grid: HashMap<(i16, i16), Vec<u32>>,
    by_ident: HashMap<String, u32>,
    countries: HashMap<String, String>,
    regions: HashMap<String, String>,
    runways: Vec<Runway>,
    runway_grid: HashMap<(i16, i16), Vec<u32>>,
}

fn cell(lat: f64, lon: f64) -> (i16, i16) {
    (lat.floor() as i16, lon.floor() as i16)
}

fn unpack(bytes: &[u8]) -> String {
    String::from_utf8(zstd::decode_all(bytes).expect("embedded data is valid zstd")).expect("utf-8")
}

fn name_table(bytes: &[u8]) -> HashMap<String, String> {
    unpack(bytes)
        .lines()
        .skip(1)
        .filter_map(|l| l.split_once('\t'))
        .map(|(c, n)| (c.to_string(), n.to_string()))
        .collect()
}

impl RefData {
    /// Load the embedded data (about 60 ms on a mid-range PC).
    pub fn load() -> Self {
        let text = unpack(AIRPORTS);
        let mut airports = Vec::with_capacity(70_000);
        for line in text.lines().skip(1) {
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 8 {
                continue;
            }
            let (Some(kind), Ok(lat), Ok(lon)) = (AirportKind::from_code(f[1]), f[3].parse(), f[4].parse()) else {
                continue;
            };
            airports.push(Airport {
                ident: f[0].to_string(),
                kind,
                name: f[2].to_string(),
                lat,
                lon,
                elevation_ft: f[5].parse().unwrap_or(0),
                country: f[6].to_string(),
                region: f[7].to_string(),
            });
        }
        let mut grid: HashMap<(i16, i16), Vec<u32>> = HashMap::new();
        let mut by_ident = HashMap::with_capacity(airports.len());
        for (i, a) in airports.iter().enumerate() {
            grid.entry(cell(a.lat, a.lon)).or_default().push(i as u32);
            by_ident.entry(a.ident.to_ascii_uppercase()).or_insert(i as u32);
        }
        let mut runways = Vec::with_capacity(16_000);
        let mut runway_grid: HashMap<(i16, i16), Vec<u32>> = HashMap::new();
        for line in unpack(RUNWAYS).lines().skip(1) {
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 8 {
                continue;
            }
            let (Ok(a), Ok(b), Ok(c), Ok(d)) = (f[3].parse(), f[4].parse(), f[6].parse(), f[7].parse()) else {
                continue;
            };
            runway_grid.entry(cell(a, b)).or_default().push(runways.len() as u32);
            runways.push(Runway {
                airport: f[0].to_string(),
                width_ft: f[1].parse().unwrap_or(0.0),
                ends: [(f[2].to_string(), a, b), (f[5].to_string(), c, d)],
            });
        }
        RefData {
            airports,
            grid,
            by_ident,
            countries: name_table(COUNTRIES),
            regions: name_table(REGIONS),
            runways,
            runway_grid,
        }
    }

    pub fn len(&self) -> usize {
        self.airports.len()
    }

    pub fn is_empty(&self) -> bool {
        self.airports.is_empty()
    }

    pub fn airport(&self, ident: &str) -> Option<&Airport> {
        self.by_ident.get(&ident.to_ascii_uppercase()).map(|&i| &self.airports[i as usize])
    }

    /// Airports matching what the pilot typed: the exact code first, then
    /// codes starting with it, then names containing it (from 3 letters),
    /// bigger airports first within each group.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Airport> {
        let q = query.trim().to_ascii_uppercase();
        if q.is_empty() {
            return Vec::new();
        }
        let words: Vec<String> = q.split_whitespace().map(str::to_string).collect();
        let mut hits: Vec<(u8, f64, &Airport)> = self
            .airports
            .iter()
            .filter_map(|a| {
                let rank = if a.ident == q {
                    0
                } else if a.ident.starts_with(&q) {
                    1
                } else if q.len() >= 3 && {
                    let name = a.name.to_ascii_uppercase();
                    words.iter().all(|w| name.contains(w.as_str()))
                } {
                    2
                } else {
                    return None;
                };
                Some((rank, a.kind.weight(), a))
            })
            .collect();
        hits.sort_by(|x, y| x.0.cmp(&y.0).then(y.1.total_cmp(&x.1)).then(x.2.ident.cmp(&y.2.ident)));
        hits.into_iter().take(limit).map(|h| h.2).collect()
    }

    pub fn country_name(&self, code: &str) -> Option<&str> {
        self.countries.get(code).map(String::as_str)
    }

    pub fn region_name(&self, code: &str) -> Option<&str> {
        self.regions.get(code).map(String::as_str)
    }

    fn candidates(&self, lat: f64, lon: f64) -> impl Iterator<Item = &Airport> {
        let (cy, cx) = cell(lat, lon);
        // Longitude cells shrink towards the poles; widen the search there.
        let dx = if lat.abs() > 60.0 { 3 } else { 1 };
        (cy - 1..=cy + 1)
            .flat_map(move |y| (cx - dx..=cx + dx).map(move |x| (y, ((x + 180).rem_euclid(360) - 180))))
            .filter_map(|k| self.grid.get(&k))
            .flatten()
            .map(|&i| &self.airports[i as usize])
    }

    /// The runway a touchdown at this position and true heading was made
    /// on, and how far from its centerline. None when no known runway lines
    /// up with the heading within a few hundred feet (grass strips, water,
    /// runways without surveyed ends).
    pub fn runway_at(&self, lat: f64, lon: f64, hdg_deg: f64) -> Option<RunwayHit> {
        const FT_PER_DEG: f64 = 60.0 * 6076.12;
        let k = lat.to_radians().cos();
        // Flat-earth feet around the touchdown: plenty over a runway's length.
        let xy = |la: f64, lo: f64| {
            let dlon = ((lo - lon + 540.0) % 360.0) - 180.0;
            (dlon * k * FT_PER_DEG, (la - lat) * FT_PER_DEG)
        };
        let (cy, cx) = cell(lat, lon);
        let mut best: Option<RunwayHit> = None;
        for y in cy - 1..=cy + 1 {
            for x in cx - 1..=cx + 1 {
                let key = (y, ((x as i32 + 180).rem_euclid(360) - 180) as i16);
                for &i in self.runway_grid.get(&key).into_iter().flatten() {
                    let r = &self.runways[i as usize];
                    let (a, b) = (xy(r.ends[0].1, r.ends[0].2), xy(r.ends[1].1, r.ends[1].2));
                    let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
                    if len < 100.0 {
                        continue;
                    }
                    let brg = (b.0 - a.0).atan2(b.1 - a.1).to_degrees().rem_euclid(360.0);
                    // Landing towards the far end, or the other way.
                    let forward = geo::heading_diff(brg, hdg_deg).abs() <= 90.0;
                    let (from, to, ident) = if forward { (a, b, &r.ends[0].0) } else { (b, a, &r.ends[1].0) };
                    let rwy_hdg = if forward { brg } else { (brg + 180.0) % 360.0 };
                    if geo::heading_diff(rwy_hdg, hdg_deg).abs() > 30.0 {
                        continue;
                    }
                    let u = ((to.0 - from.0) / len, (to.1 - from.1) / len);
                    let p = (-from.0, -from.1);
                    let along = p.0 * u.0 + p.1 * u.1;
                    // Right of the direction of travel is positive.
                    let offset = p.0 * u.1 - p.1 * u.0;
                    if along < -1500.0 || along > len + 500.0 || offset.abs() > 400.0 {
                        continue;
                    }
                    if best.as_ref().is_none_or(|h| offset.abs() < h.offset_ft.abs()) {
                        best = Some(RunwayHit {
                            airport: r.airport.clone(),
                            runway: ident.clone(),
                            heading_deg: rwy_hdg,
                            width_ft: r.width_ft,
                            offset_ft: offset,
                            along_ft: along,
                        });
                    }
                }
            }
        }
        best
    }

    /// The airport the aircraft is most likely at. Within 5 nm, larger
    /// airports are preferred; heliports only count when `allow_heliports`.
    /// Falls back to the plain nearest airport within 10 nm.
    pub fn airport_at(&self, lat: f64, lon: f64, allow_heliports: bool) -> Option<&Airport> {
        let mut best: Option<(&Airport, f64)> = None;
        let mut nearest: Option<(&Airport, f64)> = None;
        for a in self.candidates(lat, lon) {
            if a.kind == AirportKind::Heliport && !allow_heliports {
                continue;
            }
            let d = geo::distance_nm(lat, lon, a.lat, a.lon);
            if d <= 10.0 && nearest.is_none_or(|(_, nd)| d < nd) {
                nearest = Some((a, d));
            }
            if d <= 5.0 {
                let score = d / a.kind.weight();
                if best.is_none_or(|(_, bs)| score < bs) {
                    best = Some((a, score));
                }
            }
        }
        best.or(nearest).map(|(a, _)| a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    fn data() -> &'static RefData {
        static D: OnceLock<RefData> = OnceLock::new();
        D.get_or_init(RefData::load)
    }

    #[test]
    fn loads_airports_and_names() {
        let d = data();
        assert!(d.len() > 50_000, "{}", d.len());
        assert_eq!(d.country_name("FR"), Some("France"));
        assert_eq!(d.region_name("US-WA"), Some("Washington"));
        let ksea = d.airport("ksea").unwrap();
        assert_eq!((ksea.country.as_str(), ksea.region.as_str()), ("US", "US-WA"));
    }

    #[test]
    fn finds_the_big_airport_from_its_ramp() {
        // A gate at Seattle-Tacoma, about 0.5 nm from the reference point.
        let a = data().airport_at(47.4440, -122.3010, false).unwrap();
        assert_eq!(a.ident, "KSEA");
        let b = data().airport_at(49.0097, 2.5479, false).unwrap();
        assert_eq!(b.ident, "LFPG");
    }

    #[test]
    fn searches_by_code_and_name() {
        let d = data();
        assert_eq!(d.search("kabq", 5)[0].ident, "KABQ");
        assert!(d.search("EGL", 8).iter().any(|a| a.ident == "EGLL"));
        let heathrow = d.search("heathrow", 5);
        assert_eq!(heathrow[0].ident, "EGLL");
        assert!(d.search("  ", 5).is_empty());
        assert!(d.search("zz", 5).iter().all(|a| a.ident.starts_with("ZZ")));
    }

    #[test]
    fn finds_the_runway_and_the_centerline_offset() {
        let d = data();
        // KSEA 16L/34R ends, from OurAirports.
        let r = d.runways.iter().find(|r| r.airport == "KSEA" && r.ends[0].0 == "16L").expect("KSEA 16L");
        let (a, b) = ((r.ends[0].1, r.ends[0].2), (r.ends[1].1, r.ends[1].2));
        // A point a third of the way down 16L, on the centerline.
        let p = (a.0 + (b.0 - a.0) / 3.0, a.1 + (b.1 - a.1) / 3.0);
        let hdg = geo::bearing_deg(a.0, a.1, b.0, b.1);
        let hit = d.runway_at(p.0, p.1, hdg).unwrap();
        assert_eq!((hit.airport.as_str(), hit.runway.as_str()), ("KSEA", "16L"));
        assert!(hit.offset_ft.abs() < 3.0, "{hit:?}");
        // The other way it is 34R; 30 ft east of the centerline is then
        // to the pilot's right.
        let east = (p.0, p.1 + 30.0 / (60.0 * 6076.12 * p.0.to_radians().cos()));
        let hit = d.runway_at(east.0, east.1, (hdg + 180.0) % 360.0).unwrap();
        assert_eq!(hit.runway, "34R");
        assert!((hit.offset_ft - 30.0).abs() < 3.0, "{hit:?}");
        // Crossing the runway is not landing on it.
        assert!(d.runway_at(p.0, p.1, (hdg + 90.0) % 360.0).is_none_or(|h| h.runway != "16L"));
    }

    #[test]
    fn open_ocean_has_no_airport() {
        assert!(data().airport_at(30.0, -40.0, true).is_none());
    }

    #[test]
    fn works_across_the_antimeridian() {
        // Near Fiji's international airport (NFFN), longitude 177.4 E.
        assert!(data().airport_at(-17.7554, 177.4433, false).is_some());
    }
}
