//! The Nav aids map layer: VORs, NDBs and waypoints from the sim's own
//! navigation database, read over SimConnect while MSFS runs and kept in a
//! file on this PC so the layer still shows when the sim is off. The data
//! is the pilot's own copy of the sim's navdata: it is never sent anywhere
//! else (other devices on the home network only see it on the map).
//!
//! MSFS 2024's SimConnect lists the whole world at once; with an older
//! SimConnect.dll only what the sim holds around the aircraft is listed,
//! and the file fills in as the pilot flies.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use telemetry::{NavAid, NavBatch, NavKind};

const FILE: &str = "navaids-msfs.tsv";
/// A whole-world list older than this (one AIRAC cycle) is asked for again.
pub const STALE_MS: i64 = 28 * 86_400_000;

#[derive(Default)]
pub struct NavDb {
    items: Vec<NavAid>,
    index: HashMap<(NavKind, String, String, i32, i32), usize>,
    /// Item numbers in each one-degree cell.
    grid: HashMap<(i16, i16), Vec<u32>>,
    /// When the whole world was last listed (Unix ms), 0 if never.
    pub world_ms: i64,
    /// Kinds of the whole-world list received completely this time.
    world_done: HashSet<NavKind>,
    pub loaded: bool,
    /// The map layer is on (asked for recently): worth asking the sim.
    pub wanted: bool,
    /// The pilot pressed "Reload from the sim".
    pub refresh: bool,
    pub dirty: bool,
}

pub type Shared = Arc<Mutex<NavDb>>;

fn cell(lat: f64, lon: f64) -> (i16, i16) {
    (lat.floor().clamp(-90.0, 89.0) as i16, lon.floor().clamp(-180.0, 179.0) as i16)
}

impl NavDb {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Every aid with this ident.
    pub fn find(&self, ident: &str) -> Vec<&NavAid> {
        self.items.iter().filter(|a| a.ident.eq_ignore_ascii_case(ident)).collect()
    }

    /// How many waypoints are known (the rest are VORs and NDBs).
    pub fn waypoints(&self) -> usize {
        self.items.iter().filter(|a| a.kind == NavKind::Waypoint).count()
    }

    fn insert(&mut self, a: NavAid) {
        // Same ident and region can repeat far apart (terminal waypoints);
        // the position (to about 1 km) tells them apart.
        let key = (a.kind, a.ident.clone(), a.region.clone(), (a.lat * 100.0).round() as i32, (a.lon * 100.0).round() as i32);
        if let Some(&i) = self.index.get(&key) {
            self.items[i] = a;
            return;
        }
        let i = self.items.len();
        self.grid.entry(cell(a.lat, a.lon)).or_default().push(i as u32);
        self.index.insert(key, i);
        self.items.push(a);
    }

    /// Adds a part of a list from the sim. True when a whole-world list is
    /// now complete (worth saving).
    pub fn merge(&mut self, b: NavBatch, now_ms: i64) -> bool {
        let n = b.items.len();
        for a in b.items {
            self.insert(a);
        }
        self.dirty |= n > 0;
        if b.world && b.last {
            self.world_done.insert(b.kind);
            log::info!("nav aids: the world's {:?} list received ({} known now)", b.kind, self.items.len());
            if self.world_done.len() == 3 {
                self.world_done.clear();
                self.world_ms = now_ms;
                return true;
            }
        }
        false
    }

    /// The aids in a box, the most useful first (VORs, NDBs, waypoints),
    /// at most `limit`. Waypoints only when `waypoints`.
    pub fn query(&self, w: f64, s: f64, e: f64, n: f64, waypoints: bool, limit: usize) -> (Vec<NavAid>, bool) {
        let lons: Vec<(f64, f64)> = if w <= e { vec![(w, e)] } else { vec![(w, 180.0), (-180.0, e)] };
        let mut by_kind: [Vec<&NavAid>; 3] = Default::default();
        for (w, e) in lons {
            for la in (s.floor().max(-90.0) as i16)..=(n.floor().min(89.0) as i16) {
                for lo in (w.floor().max(-180.0) as i16)..=(e.floor().min(179.0) as i16) {
                    for &i in self.grid.get(&(la, lo)).map(Vec::as_slice).unwrap_or(&[]) {
                        let a = &self.items[i as usize];
                        if a.lat < s || a.lat > n || a.lon < w || a.lon > e {
                            continue;
                        }
                        let k = match a.kind {
                            NavKind::Vor => 0,
                            NavKind::Ndb => 1,
                            NavKind::Waypoint if waypoints => 2,
                            NavKind::Waypoint => continue,
                        };
                        by_kind[k].push(a);
                    }
                }
            }
        }
        let total: usize = by_kind.iter().map(Vec::len).sum();
        let out: Vec<NavAid> = by_kind.into_iter().flatten().take(limit).cloned().collect();
        (out, total > limit)
    }

    pub fn save(&mut self, dir: &Path) {
        let path = dir.join(FILE);
        let tmp = dir.join(format!("{FILE}.tmp"));
        let r = (|| -> std::io::Result<()> {
            let mut f = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
            writeln!(f, "# FlightDeck nav aids from MSFS\tworld_ms={}", self.world_ms)?;
            for a in &self.items {
                let k = match a.kind {
                    NavKind::Vor => 'V',
                    NavKind::Ndb => 'N',
                    NavKind::Waypoint => 'W',
                };
                writeln!(
                    f,
                    "{k}\t{}\t{}\t{:.6}\t{:.6}\t{}\t{}",
                    a.ident,
                    a.region,
                    a.lat,
                    a.lon,
                    a.freq_hz.unwrap_or(0),
                    u8::from(a.dme) | (u8::from(a.dme_only) << 1)
                )?;
            }
            f.flush()?;
            drop(f);
            std::fs::rename(&tmp, &path)
        })();
        match r {
            Ok(()) => {
                self.dirty = false;
                log::info!("nav aids saved: {} in {}", self.items.len(), path.display());
            }
            Err(e) => log::warn!("nav aids not saved: {e}"),
        }
    }

    pub fn load(&mut self, dir: &Path) {
        self.loaded = true;
        let Ok(f) = std::fs::File::open(dir.join(FILE)) else { return };
        for line in std::io::BufReader::new(f).lines().map_while(Result::ok) {
            if let Some(rest) = line.strip_prefix('#') {
                if let Some(v) = rest.split("world_ms=").nth(1) {
                    self.world_ms = v.trim().parse().unwrap_or(0);
                }
                continue;
            }
            let p: Vec<&str> = line.split('\t').collect();
            if p.len() < 7 {
                continue;
            }
            let kind = match p[0] {
                "V" => NavKind::Vor,
                "N" => NavKind::Ndb,
                "W" => NavKind::Waypoint,
                _ => continue,
            };
            let (Ok(lat), Ok(lon)) = (p[3].parse::<f64>(), p[4].parse::<f64>()) else { continue };
            let freq: u32 = p[5].parse().unwrap_or(0);
            let flags: u8 = p[6].parse().unwrap_or(0);
            self.insert(NavAid {
                kind,
                ident: p[1].into(),
                region: p[2].into(),
                lat,
                lon,
                freq_hz: (freq > 0).then_some(freq),
                dme: flags & 1 != 0,
                dme_only: flags & 2 != 0,
            });
        }
        self.dirty = false;
        log::info!("nav aids loaded: {}", self.items.len());
    }
}

/// Saves on a thread of its own (a whole-world list is a few MB).
pub fn save_soon(db: &Shared, dir: PathBuf) {
    let db = db.clone();
    let _ = std::thread::Builder::new().name("navaids-save".into()).spawn(move || db.lock().unwrap().save(&dir));
}

/// What the map layer gets.
#[derive(Serialize)]
pub struct NavView {
    pub items: Vec<NavAid>,
    /// More were in the box than sent.
    pub more: bool,
    /// Known in all, and when the whole world was last listed (0 never).
    pub known: usize,
    /// Of those, waypoints.
    pub waypoints: usize,
    pub world_ms: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aid(kind: NavKind, ident: &str, lat: f64, lon: f64) -> NavAid {
        NavAid { kind, ident: ident.into(), region: "K1".into(), lat, lon, freq_hz: Some(116_800_000), dme: true, dme_only: false }
    }

    #[test]
    fn merges_queries_and_round_trips() {
        let mut db = NavDb::default();
        let b = |kind, items, last| NavBatch { kind, items, last, world: true };
        assert!(!db.merge(b(NavKind::Vor, vec![aid(NavKind::Vor, "SEA", 47.43, -122.31), aid(NavKind::Vor, "SEA", 47.43, -122.31)], true), 5));
        assert!(!db.merge(b(NavKind::Ndb, vec![aid(NavKind::Ndb, "OS", 47.1, -122.5)], true), 5));
        assert!(db.merge(b(NavKind::Waypoint, vec![aid(NavKind::Waypoint, "HAMUR", 46.9, -121.9), aid(NavKind::Waypoint, "EDGE", 10.0, 179.5)], true), 7));
        assert_eq!(db.len(), 4, "the same VOR twice is kept once");
        assert_eq!(db.world_ms, 7);
        let (v, more) = db.query(-123.0, 46.0, -121.0, 48.0, false, 10);
        assert_eq!((v.len(), more), (2, false));
        let (v, more) = db.query(-123.0, 46.0, -121.0, 48.0, true, 2);
        assert_eq!(v.iter().map(|a| a.ident.as_str()).collect::<Vec<_>>(), ["SEA", "OS"]);
        assert!(more);
        // Across the date line.
        let (v, _) = db.query(179.0, 9.0, -179.0, 11.0, true, 10);
        assert_eq!(v.len(), 1);

        let dir = std::env::temp_dir().join(format!("fd-nav-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        db.save(&dir);
        let mut back = NavDb::default();
        back.load(&dir);
        assert_eq!(back.len(), 4);
        assert_eq!(back.world_ms, 7);
        let (v, _) = back.query(-123.0, 46.0, -121.0, 48.0, false, 10);
        assert_eq!(v[0], aid(NavKind::Vor, "SEA", 47.43, -122.31));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
