//! Local storage: one SQLite file holds every flight, its logbook line, its
//! compressed track and its events.

pub mod book;
pub mod codec;
pub mod journal;
pub mod night;
pub mod passport;
pub mod pilot;
pub mod types;

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use telemetry::{geo, AircraftInfo, TrackPoint};
use tracker::{FlightStatus, FlightSummary, LandingReport};

const SCHEMA_V1: &str = include_str!("schema.sql");
const MIGRATE_V2: &str = include_str!("migrate_v2.sql");
const MIGRATE_V3: &str = include_str!("migrate_v3.sql");
const MIGRATE_V4: &str = include_str!("migrate_v4.sql");
const MIGRATE_V5: &str = include_str!("migrate_v5.sql");
const MIGRATE_V6: &str = include_str!("migrate_v6.sql");
const MIGRATE_V7: &str = include_str!("migrate_v7.sql");
const MIGRATE_V8: &str = include_str!("migrate_v8.sql");
const MIGRATE_V9: &str = include_str!("migrate_v9.sql");
const MIGRATE_V10: &str = include_str!("migrate_v10.sql");
const MIGRATE_V11: &str = include_str!("migrate_v11.sql");
const MIGRATE_V12: &str = include_str!("migrate_v12.sql");
const SCHEMA_VERSION: i64 = 13;

/// The hangar aircraft with this registration (as the pilot sees it), if any.
pub(crate) fn aircraft_by_reg(conn: &Connection, reg: &str) -> Result<Option<i64>> {
    let reg = reg.trim();
    if reg.is_empty() {
        return Ok(None);
    }
    Ok(conn
        .query_row(
            "SELECT id FROM aircraft
             WHERE merged_into IS NULL AND UPPER(COALESCE(NULLIF(user_reg, ''), registration)) = UPPER(?1)
             ORDER BY manual DESC, (user_reg IS NOT NULL) DESC, id LIMIT 1",
            params![reg],
            |r| r.get(0),
        )
        .optional()?)
}

/// One aircraft per registration: aircraft that share one are merged into
/// a single hangar aircraft with all their flights. The others stay as
/// aliases so the sim keeps finding it. Returns how many were merged.
pub(crate) fn merge_duplicate_aircraft(conn: &Connection) -> Result<usize> {
    let groups: Vec<String> = {
        let mut st = conn.prepare(
            "SELECT UPPER(COALESCE(NULLIF(user_reg, ''), registration)) AS r FROM aircraft
             WHERE merged_into IS NULL AND TRIM(COALESCE(NULLIF(user_reg, ''), registration)) <> ''
             GROUP BY r HAVING COUNT(*) > 1",
        )?;
        let rows = st.query_map([], |r| r.get(0))?;
        rows.collect::<std::result::Result<_, _>>()?
    };
    let mut merged = 0;
    for reg in groups {
        // Keep the aircraft the pilot set up, else the most flown, else the oldest.
        let ids: Vec<i64> = {
            let mut st = conn.prepare(
                "SELECT a.id FROM aircraft a
                 WHERE a.merged_into IS NULL AND UPPER(COALESCE(NULLIF(a.user_reg, ''), a.registration)) = ?1
                 ORDER BY a.manual DESC, (a.user_reg IS NOT NULL) DESC,
                          (SELECT COUNT(*) FROM flight f WHERE f.aircraft_id = a.id) DESC, a.id",
            )?;
            let rows = st.query_map(params![reg], |r| r.get(0))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        let keep = ids[0];
        for dup in &ids[1..] {
            conn.execute("UPDATE flight SET aircraft_id = ?1 WHERE aircraft_id = ?2", params![keep, dup])?;
            conn.execute(
                "UPDATE aircraft SET
                    favorite = MAX(favorite, (SELECT favorite FROM aircraft WHERE id = ?2)),
                    photo = COALESCE(photo, (SELECT photo FROM aircraft WHERE id = ?2)),
                    user_model = COALESCE(user_model, (SELECT user_model FROM aircraft WHERE id = ?2)),
                    airline = COALESCE(airline, (SELECT airline FROM aircraft WHERE id = ?2)),
                    icao_type = CASE WHEN icao_type = '' THEN (SELECT icao_type FROM aircraft WHERE id = ?2) ELSE icao_type END
                 WHERE id = ?1",
                params![keep, dup],
            )?;
            conn.execute(
                "UPDATE aircraft SET merged_into = ?1, favorite = 0, photo = NULL WHERE id = ?2 OR merged_into = ?2",
                params![keep, dup],
            )?;
            merged += 1;
        }
    }
    Ok(merged)
}

/// Remove aircraft no flight uses any more, unless the pilot added them or
/// gave them details, a photo or a star of their own.
pub(crate) const DELETE_ORPHAN_AIRCRAFT: &str = "DELETE FROM aircraft
     WHERE manual = 0 AND favorite = 0 AND photo IS NULL
       AND user_reg IS NULL AND user_model IS NULL AND airline IS NULL AND merged_into IS NULL
       AND id NOT IN (SELECT DISTINCT aircraft_id FROM flight WHERE aircraft_id IS NOT NULL)";
/// Tolerance for track simplification.
const SIMPLIFY_M: f64 = 15.0;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("track: {0}")]
    Codec(#[from] codec::CodecError),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("compression: {0}")]
    Io(#[from] std::io::Error),
    #[error("database was written by a newer FlightDeck (schema {0})")]
    TooNew(i64),
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// An airport matched to a position (from the reference data).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Place {
    pub icao: String,
    pub name: String,
    pub country: String,
    pub region: String,
}

/// Extra facts the caller knows when saving a flight.
#[derive(Debug, Clone, Default)]
pub struct SaveContext {
    pub dep: Option<Place>,
    pub arr: Option<Place>,
    pub planned_arr_icao: Option<String>,
    pub callsign: String,
    pub flight_no: String,
    /// The flight plan as JSON, if one was loaded.
    pub plan: Option<String>,
    /// Imported flights: the tracker it came from (e.g. "volanta"), its id
    /// there, and the original file.
    pub source: Option<Imported>,
    /// Log the flight to this hangar aircraft ("Fly this aircraft"),
    /// whatever aircraft the sim reported.
    pub aircraft_id: Option<i64>,
    /// The planned destination, when the pilot diverted from it.
    pub diverted_from: Option<String>,
}

/// What a backup file holds.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BackupInfo {
    pub flights: i64,
    pub aircraft: i64,
    pub last_flight_ms: Option<i64>,
    pub schema: i64,
}

/// A screenshot: where the file is (never copied) and when it was taken.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Screenshot {
    pub id: i64,
    pub path: String,
    pub taken_ms: i64,
}

/// A screenshot in the Photos tab: with its flight (if any) and where it
/// was taken, from the flight's track.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Photo {
    pub id: i64,
    pub path: String,
    pub taken_ms: i64,
    pub flight_id: Option<i64>,
    pub dep: Option<String>,
    pub arr: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub alt_ft: Option<f64>,
}

/// Where on a track a moment was: latitude, longitude, altitude (feet),
/// between the points either side.
pub fn place_on(track: &[telemetry::TrackPoint], t: i64) -> Option<(f64, f64, f64)> {
    let first = track.first()?;
    let last = track.last()?;
    if t <= first.t_ms {
        return Some((first.lat, first.lon, first.alt_ft as f64));
    }
    if t >= last.t_ms {
        return Some((last.lat, last.lon, last.alt_ft as f64));
    }
    let i = track.partition_point(|p| p.t_ms <= t).max(1);
    let (a, b) = (&track[i - 1], &track[i]);
    let f = if b.t_ms > a.t_ms { (t - a.t_ms) as f64 / (b.t_ms - a.t_ms) as f64 } else { 0.0 };
    Some((a.lat + (b.lat - a.lat) * f, a.lon + (b.lon - a.lon) * f, a.alt_ft as f64 + (b.alt_ft - a.alt_ft) as f64 * f))
}

#[derive(Debug, Clone, Default)]
pub struct Imported {
    pub source: String,
    pub id: String,
    pub file: String,
}

/// One of a set of flights that look like duplicates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateFlight {
    #[serde(flatten)]
    pub flight: FlightRow,
    pub has_track: bool,
    /// Where it came from: "" (recorded), "manual", "volanta", "elevatex".
    pub source: String,
}

/// One row of the flight list, ready for the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightRow {
    pub id: i64,
    pub sim: String,
    pub status: String,
    pub aircraft_title: String,
    pub icao_type: String,
    pub registration: String,
    pub callsign: String,
    pub dep_icao: Option<String>,
    pub dep_name: Option<String>,
    pub dep_country: Option<String>,
    pub arr_icao: Option<String>,
    pub arr_name: Option<String>,
    pub arr_country: Option<String>,
    pub start_ms: i64,
    pub end_ms: i64,
    pub block_out: Option<i64>,
    pub takeoff: Option<i64>,
    pub landing: Option<i64>,
    pub block_in: Option<i64>,
    pub total_min: i64,
    pub distance_nm: f64,
    pub max_alt_ft: i64,
    pub fuel_used_kg: f64,
    pub landing_rate_fpm: Option<f64>,
    pub landing_g: Option<f64>,
    pub landing_grade: Option<String>,
    pub landings: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Totals {
    pub flights: i64,
    pub minutes: i64,
    pub distance_nm: f64,
    pub landings: i64,
    pub airports: i64,
    pub countries: i64,
}

pub struct Store {
    conn: Connection,
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Logbook category and class for an aircraft.
pub fn category_class(a: &AircraftInfo) -> &'static str {
    types::class_of(a)
}

/// Bring every aircraft's class in line with the type table (it grows
/// with new versions), so logbook columns are right for past flights too.
fn reclassify_aircraft(conn: &Connection) -> Result<usize> {
    let rows: Vec<(i64, String, String, i64, String, String)> = {
        let mut st = conn.prepare("SELECT id, icao_type, title, engine_count, engine_type, category_class FROM aircraft")?;
        let r = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?;
        r.collect::<std::result::Result<_, _>>()?
    };
    let mut n = 0;
    for (id, ty, title, count, engine, class) in rows {
        let want = types::class_for(&ty, &title, count.clamp(0, 255) as u8, types::engine_from_str(&engine));
        if want != class {
            conn.execute("UPDATE aircraft SET category_class = ?2 WHERE id = ?1", params![id, want])?;
            n += 1;
        }
    }
    Ok(n)
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(StoreError::TooNew(version));
        }
        if version < 1 {
            conn.execute_batch(&format!("BEGIN; {SCHEMA_V1} PRAGMA user_version = 1; COMMIT;"))?;
        }
        if version < 2 {
            conn.execute_batch(&format!("BEGIN; {MIGRATE_V2} COMMIT;"))?;
            backfill_v2(&conn)?;
            conn.pragma_update(None, "user_version", 2)?;
        }
        if version < 3 {
            conn.execute_batch(&format!("BEGIN; {MIGRATE_V3} PRAGMA user_version = 3; COMMIT;"))?;
        }
        if version < 4 {
            conn.execute_batch(&format!("BEGIN; {MIGRATE_V4} PRAGMA user_version = 4; COMMIT;"))?;
        }
        if version < 5 {
            conn.execute_batch(&format!("BEGIN; {MIGRATE_V5} PRAGMA user_version = 5; COMMIT;"))?;
        }
        if version < 6 {
            conn.execute_batch(&format!("BEGIN; {MIGRATE_V6} PRAGMA user_version = 6; COMMIT;"))?;
        }
        if version < 7 {
            conn.execute_batch(&format!("BEGIN; {MIGRATE_V7} PRAGMA user_version = 7; COMMIT;"))?;
        }
        if version < 8 {
            conn.execute_batch(&format!("BEGIN; {MIGRATE_V8} PRAGMA user_version = 8; COMMIT;"))?;
        }
        if version < 9 {
            conn.execute_batch(&format!("BEGIN; {MIGRATE_V9} PRAGMA user_version = 9; COMMIT;"))?;
        }
        if version < 10 {
            conn.execute_batch(&format!("BEGIN; {MIGRATE_V10} PRAGMA user_version = 10; COMMIT;"))?;
        }
        if version < 11 {
            conn.execute_batch(&format!("BEGIN; {MIGRATE_V11} PRAGMA user_version = 11; COMMIT;"))?;
        }
        if version < 12 {
            conn.execute_batch(&format!("BEGIN; {MIGRATE_V12} PRAGMA user_version = 12; COMMIT;"))?;
        }
        if version < 13 {
            conn.execute_batch("BEGIN;")?;
            air_time_v13(&conn)?;
            conn.execute_batch("PRAGMA user_version = 13; COMMIT;")?;
        }
        merge_duplicate_aircraft(&conn)?;
        reclassify_aircraft(&conn)?;
        Ok(Store { conn })
    }

    /// Save a finished flight with its logbook line, track and events.
    pub fn save_flight(&mut self, f: &FlightSummary, ctx: &SaveContext) -> Result<i64> {
        let tx = self.conn.transaction()?;
        let id = StoreTx { conn: &tx }.save(f, ctx)?;
        tx.commit()?;
        Ok(id)
    }

    pub fn list_flights(&self, limit: i64, offset: i64) -> Result<Vec<FlightRow>> {
        let mut st = self.conn.prepare(&format!("{FLIGHT_ROW_SQL} ORDER BY f.start_ms DESC LIMIT ?1 OFFSET ?2"))?;
        let rows = st.query_map(params![limit, offset], flight_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Flights from or to an airport, newest first.
    pub fn airport_flights(&self, ident: &str) -> Result<Vec<FlightRow>> {
        let mut st = self
            .conn
            .prepare(&format!("{FLIGHT_ROW_SQL} WHERE f.dep_icao = ?1 OR f.arr_icao = ?1 ORDER BY f.start_ms DESC"))?;
        let rows = st.query_map(params![ident], flight_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// The flight plan saved with a flight, as JSON.
    pub fn get_plan(&self, id: i64) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT plan FROM flight WHERE id = ?1", params![id], |r| {
                // Plain JSON text (flights saved before compression) or a
                // zstd blob.
                Ok(match r.get_ref(0)? {
                    rusqlite::types::ValueRef::Text(t) => Some(String::from_utf8_lossy(t).into_owned()),
                    rusqlite::types::ValueRef::Blob(b) => {
                        zstd::bulk::decompress(b, 64 * 1024 * 1024).ok().map(|v| String::from_utf8_lossy(&v).into_owned())
                    }
                    _ => None,
                })
            })
            .optional()?
            .flatten())
    }

    /// The FlightDeck id of a flight imported from `source` with id `source_id`.
    pub fn find_imported(&self, source: &str, source_id: &str) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM flight WHERE source = ?1 AND source_id = ?2",
                params![source, source_id],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Write a complete copy of the database (every flight, the logbook,
    /// the hangar and its pictures, the pilot and the settings) to a file,
    /// consistent even while FlightDeck is running. Written beside the
    /// target first, then moved into place, so a backup is never half
    /// written (a cloud folder never syncs a partial file).
    pub fn backup_to(&self, path: &Path) -> Result<()> {
        let tmp = path.with_extension("partial");
        let _ = std::fs::remove_file(&tmp);
        self.conn.execute("VACUUM INTO ?1", params![tmp.to_string_lossy()])?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Check a backup file before restoring it: what it holds.
    pub fn check_backup(path: &Path) -> Result<BackupInfo> {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(StoreError::TooNew(version));
        }
        let flights: i64 = conn.query_row("SELECT COUNT(*) FROM flight", [], |r| r.get(0))?;
        let last_ms: Option<i64> = conn.query_row("SELECT MAX(start_ms) FROM flight", [], |r| r.get(0))?;
        let aircraft: i64 = conn.query_row("SELECT COUNT(*) FROM aircraft", [], |r| r.get(0)).unwrap_or(0);
        Ok(BackupInfo { flights, aircraft, last_flight_ms: last_ms, schema: version })
    }

    /// Note a new screenshot file (once; the same path again is ignored).
    /// Returns its id when it is new.
    pub fn add_screenshot(&self, path: &str, taken_ms: i64) -> Result<Option<i64>> {
        let n = self.conn.execute("INSERT OR IGNORE INTO screenshot (path, taken_ms) VALUES (?1, ?2)", params![path, taken_ms])?;
        Ok((n > 0).then(|| self.conn.last_insert_rowid()))
    }

    /// A flight's screenshots, in the order taken.
    pub fn screenshots_of(&self, flight_id: i64) -> Result<Vec<Screenshot>> {
        let mut st = self.conn.prepare("SELECT id, path, taken_ms FROM screenshot WHERE flight_id = ?1 ORDER BY taken_ms")?;
        let rows = st.query_map(params![flight_id], |r| Ok(Screenshot { id: r.get(0)?, path: r.get(1)?, taken_ms: r.get(2)? }))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Screenshots not yet part of a saved flight, taken since a time (the
    /// flight under way's).
    pub fn screenshots_since(&self, since_ms: i64) -> Result<Vec<Screenshot>> {
        let mut st = self.conn.prepare("SELECT id, path, taken_ms FROM screenshot WHERE flight_id IS NULL AND taken_ms >= ?1 ORDER BY taken_ms")?;
        let rows = st.query_map(params![since_ms], |r| Ok(Screenshot { id: r.get(0)?, path: r.get(1)?, taken_ms: r.get(2)? }))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Where a screenshot's file is.
    pub fn screenshot_path(&self, id: i64) -> Result<Option<String>> {
        Ok(self.conn.query_row("SELECT path FROM screenshot WHERE id = ?1", params![id], |r| r.get(0)).optional()?)
    }

    /// Every screenshot FlightDeck knows, newest first, placed on its
    /// flight's track (screenshots not in a flight have no place).
    pub fn photos(&self) -> Result<Vec<Photo>> {
        let mut st = self.conn.prepare(
            "SELECT s.id, s.path, s.taken_ms, s.flight_id, f.dep_icao, f.arr_icao
             FROM screenshot s LEFT JOIN flight f ON f.id = s.flight_id
             ORDER BY s.taken_ms DESC",
        )?;
        let mut out: Vec<Photo> = st
            .query_map([], |r| {
                Ok(Photo { id: r.get(0)?, path: r.get(1)?, taken_ms: r.get(2)?, flight_id: r.get(3)?, dep: r.get(4)?, arr: r.get(5)?, lat: None, lon: None, alt_ft: None })
            })?
            .collect::<std::result::Result<_, _>>()?;
        // Each flight's track is read once.
        let mut tracks: std::collections::HashMap<i64, Vec<telemetry::TrackPoint>> = std::collections::HashMap::new();
        for p in &mut out {
            let Some(fid) = p.flight_id else { continue };
            let track = tracks.entry(fid).or_insert_with(|| {
                self.conn
                    .query_row("SELECT points FROM track WHERE flight_id = ?1", params![fid], |r| r.get::<_, Vec<u8>>(0))
                    .ok()
                    .and_then(|b| codec::decode(&b).ok())
                    .unwrap_or_default()
            });
            if let Some((lat, lon, alt)) = place_on(track, p.taken_ms) {
                p.lat = Some(lat);
                p.lon = Some(lon);
                p.alt_ft = Some(alt);
            }
        }
        Ok(out)
    }

    /// Forget a screenshot (its file is the caller's business). Returns its path.
    pub fn forget_screenshot(&self, id: i64) -> Result<Option<String>> {
        let path = self.screenshot_path(id)?;
        self.conn.execute("DELETE FROM screenshot WHERE id = ?1", params![id])?;
        Ok(path)
    }

    /// Forget screenshots noted from folders that hold other programs'
    /// small copies (Steam's "thumbnails", say), noted before those were
    /// left out.
    pub fn forget_screenshots_in(&self, folder_names: &[&str]) -> Result<usize> {
        let mut st = self.conn.prepare("SELECT id, path FROM screenshot")?;
        let rows: Vec<(i64, String)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<std::result::Result<_, _>>()?;
        let mut n = 0;
        for (id, path) in rows {
            let parts: Vec<String> = path.split(['/', '\\']).map(|p| p.to_ascii_lowercase()).collect();
            if parts.len() > 1 && parts[..parts.len() - 1].iter().any(|p| folder_names.contains(&p.as_str())) {
                n += self.conn.execute("DELETE FROM screenshot WHERE id = ?1", params![id])?;
            }
        }
        Ok(n)
    }

    /// Forget screenshots never part of a flight once they are a few days old.
    pub fn prune_screenshots(&self, before_ms: i64) -> Result<usize> {
        Ok(self.conn.execute("DELETE FROM screenshot WHERE flight_id IS NULL AND taken_ms < ?1", params![before_ms])?)
    }

    /// A flight already saved for the same UTC day between the same airports
    /// (as the duplicate finder groups them), if any.
    pub fn similar_flight(&self, day: i64, dep: &str, arr: &str) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM flight
                 WHERE COALESCE(block_out, takeoff, start_ms) / 86400000 = ?1
                   AND COALESCE(dep_icao, '') = ?2 AND COALESCE(arr_icao, '') = ?3
                 LIMIT 1",
                params![day, dep, arr],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// The flight number given when the flight was set up ("" if none).
    pub fn flight_no(&self, id: i64) -> Result<String> {
        Ok(self
            .conn
            .query_row("SELECT COALESCE(flight_no, '') FROM flight WHERE id = ?1", params![id], |r| r.get(0))
            .optional()?
            .unwrap_or_default())
    }

    /// Where an imported flight came from, and its original file.
    pub fn get_imported(&self, id: i64) -> Result<Option<Imported>> {
        let row = self
            .conn
            .query_row("SELECT source, source_id, source_file FROM flight WHERE id = ?1", params![id], |r| {
                Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<Vec<u8>>>(2)?))
            })
            .optional()?;
        let Some((Some(source), Some(sid), Some(blob))) = row else { return Ok(None) };
        let file = String::from_utf8_lossy(&zstd::bulk::decompress(&blob, 256 * 1024 * 1024)?).into_owned();
        Ok(Some(Imported { source, id: sid, file }))
    }

    /// Facts kept for diagnosis: (sim, status, end reason, paused ms,
    /// started airborne, max sim rate, source).
    #[allow(clippy::type_complexity)]
    pub fn flight_diag(&self, id: i64) -> Result<Option<(String, String, String, i64, bool, f64, Option<String>)>> {
        Ok(self
            .conn
            .query_row(
                "SELECT sim, status, COALESCE(end_reason, ''), COALESCE(paused_ms, 0), COALESCE(started_airborne, 0),
                        COALESCE(max_sim_rate, 1), source FROM flight WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get::<_, i64>(4)? != 0, r.get(5)?, r.get(6)?)),
            )
            .optional()?)
    }

    /// Every flight flown in one aircraft, newest first.
    pub fn flights_of_aircraft(&self, aircraft_id: i64) -> Result<Vec<FlightRow>> {
        let mut st = self.conn.prepare(&format!("{FLIGHT_ROW_SQL} WHERE f.aircraft_id = ?1 ORDER BY f.start_ms DESC"))?;
        let rows = st.query_map(params![aircraft_id], flight_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// The most recent flight ids, newest first.
    pub fn recent_flight_ids(&self, n: i64) -> Result<Vec<i64>> {
        let mut st = self.conn.prepare("SELECT id FROM flight ORDER BY start_ms DESC LIMIT ?1")?;
        let rows = st.query_map(params![n], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Every flight id, oldest first.
    pub fn all_flight_ids(&self) -> Result<Vec<i64>> {
        let mut st = self.conn.prepare("SELECT id FROM flight ORDER BY start_ms")?;
        let rows = st.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// The events saved with a flight (kind and JSON data), in time order.
    pub fn get_events(&self, id: i64) -> Result<Vec<(i64, String, String)>> {
        let mut st = self.conn.prepare("SELECT t, kind, data FROM flight_event WHERE flight_id = ?1 ORDER BY t")?;
        let rows = st.query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Flights that look like the same flight logged more than once: same
    /// day (UTC), same aircraft, same departure and arrival. Each group is
    /// ordered with the most complete record first (it has a track, then
    /// the longest), so the others are the ones to delete.
    pub fn duplicate_flights(&self) -> Result<Vec<Vec<DuplicateFlight>>> {
        let mut st = self.conn.prepare(
            "SELECT f.id, COALESCE(f.block_out, f.takeoff, f.start_ms) / 86400000, COALESCE(f.aircraft_id, -1),
                    COALESCE(f.dep_icao, ''), COALESCE(f.arr_icao, ''),
                    EXISTS (SELECT 1 FROM track t WHERE t.flight_id = f.id), COALESCE(l.total_min, 0), COALESCE(f.source, '')
             FROM flight f LEFT JOIN log_entry l ON l.flight_id = f.id",
        )?;
        type Key = (i64, i64, String, String);
        let mut groups: std::collections::BTreeMap<Key, Vec<(i64, bool, i64, String)>> = Default::default();
        let rows = st.query_map([], |r| {
            Ok(((r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?), (r.get(0)?, r.get(5)?, r.get(6)?, r.get(7)?)))
        })?;
        for row in rows {
            let (k, v): (Key, (i64, bool, i64, String)) = row?;
            groups.entry(k).or_default().push(v);
        }
        let mut out = Vec::new();
        for (_, mut g) in groups.into_iter().filter(|(_, g)| g.len() > 1) {
            g.sort_by_key(|(id, track, min, _)| (!*track, -*min, *id));
            let mut list = Vec::new();
            for (id, has_track, _, source) in g {
                if let Some(flight) = self.get_flight(id)? {
                    list.push(DuplicateFlight { flight, has_track, source });
                }
            }
            if list.len() > 1 {
                out.push(list);
            }
        }
        // Newest first.
        out.sort_by_key(|g| std::cmp::Reverse(g[0].flight.start_ms));
        Ok(out)
    }

    /// Delete several flights at once. Returns how many went.
    pub fn delete_flights(&mut self, ids: &[i64]) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let mut n = 0;
        for id in ids {
            n += tx.execute("DELETE FROM flight WHERE id = ?1", params![id])?;
        }
        tx.execute(DELETE_ORPHAN_AIRCRAFT, [])?;
        tx.commit()?;
        Ok(n)
    }

    pub fn get_flight(&self, id: i64) -> Result<Option<FlightRow>> {
        Ok(self
            .conn
            .query_row(&format!("{FLIGHT_ROW_SQL} WHERE f.id = ?1"), params![id], flight_row)
            .optional()?)
    }

    pub fn get_track(&self, id: i64) -> Result<Vec<TrackPoint>> {
        let blob: Option<Vec<u8>> = self
            .conn
            .query_row("SELECT points FROM track WHERE flight_id = ?1", params![id], |r| r.get(0))
            .optional()?;
        match blob {
            Some(b) => Ok(codec::decode(&b)?),
            None => Ok(vec![]),
        }
    }

    pub fn get_landings(&self, id: i64) -> Result<Vec<LandingReport>> {
        let mut st = self
            .conn
            .prepare("SELECT data FROM flight_event WHERE flight_id = ?1 AND kind = 'landing_report' ORDER BY t")?;
        let rows = st.query_map(params![id], |r| r.get::<_, String>(0))?;
        let mut out = vec![];
        for r in rows {
            out.push(serde_json::from_str(&r?)?);
        }
        Ok(out)
    }

    /// Remove a flight from everything: logbook, map, passport and hangar.
    /// An aircraft with no flights left leaves the hangar too.
    pub fn delete_flight(&mut self, id: i64) -> Result<bool> {
        let n = self.conn.execute("DELETE FROM flight WHERE id = ?1", params![id])?;
        self.conn.execute(
            crate::DELETE_ORPHAN_AIRCRAFT,
            [],
        )?;
        Ok(n > 0)
    }

    pub fn totals(&self) -> Result<Totals> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(l.total_min), 0),
                    COALESCE(SUM(f.distance_nm), 0),
                    COALESCE(SUM(l.day_landings + l.night_landings), 0),
                    (SELECT COUNT(*) FROM (SELECT dep_icao AS a FROM flight WHERE dep_icao IS NOT NULL
                                            UNION SELECT arr_icao FROM flight WHERE arr_icao IS NOT NULL)),
                    (SELECT COUNT(DISTINCT arr_country) FROM flight WHERE arr_country IS NOT NULL AND arr_country <> '')
             FROM flight f LEFT JOIN log_entry l ON l.flight_id = f.id",
            [],
            |r| {
                Ok(Totals {
                    flights: r.get(0)?,
                    minutes: r.get(1)?,
                    distance_nm: r.get(2)?,
                    landings: r.get(3)?,
                    airports: r.get(4)?,
                    countries: r.get(5)?,
                })
            },
        )?)
    }
}

/// Borrowed connection used inside a transaction.
struct StoreTx<'a> {
    conn: &'a Connection,
}

impl StoreTx<'_> {
    fn save(&self, f: &FlightSummary, ctx: &SaveContext) -> Result<i64> {
        let chosen: Option<i64> = match ctx.aircraft_id {
            Some(id) => self.conn.query_row("SELECT id FROM aircraft WHERE id = ?1", params![id], |r| r.get(0)).optional()?,
            None => None,
        };
        let aircraft_id = match chosen {
            Some(id) => id,
            None => Store::aircraft_id_with(self.conn, &f.aircraft)?,
        };
        let final_landing = f.final_landing();
        let now = now_ms();
        let status = match (f.status, &ctx.planned_arr_icao, &ctx.arr) {
            (FlightStatus::Partial, _, _) => "partial",
            (_, Some(planned), Some(arr)) if !planned.eq_ignore_ascii_case(&arr.icao) => "diverted",
            _ => "complete",
        };
        let dep = ctx.dep.clone().unwrap_or_default();
        let arr = ctx.arr.clone().unwrap_or_default();
        let opt = |s: &str| (!s.is_empty()).then(|| s.to_string());
        self.conn.execute(
            "INSERT INTO flight (sim, aircraft_id, callsign, flight_no,
                dep_icao, dep_name, dep_country, dep_region, arr_icao, arr_name, arr_country, arr_region,
                planned_arr_icao, start_ms, end_ms, block_out, takeoff, landing, block_in,
                distance_nm, fuel_used_kg, max_alt_ft, landing_rate_fpm, landing_g, paused_ms, max_sim_rate,
                started_airborne, status, end_reason, created_at, updated_at, dep_lat, dep_lon, arr_lat, arr_lon)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19,
                     ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?30, ?31, ?32, ?33, ?34)",
            params![
                f.sim.as_str(),
                aircraft_id,
                ctx.callsign,
                ctx.flight_no,
                opt(&dep.icao),
                opt(&dep.name),
                opt(&dep.country),
                opt(&dep.region),
                opt(&arr.icao),
                opt(&arr.name),
                opt(&arr.country),
                opt(&arr.region),
                ctx.planned_arr_icao,
                f.start_ms,
                f.end_ms,
                f.block_out_ms,
                f.takeoff_ms,
                f.landing_ms,
                f.block_in_ms,
                f.distance_nm,
                f.fuel_used_kg,
                f.max_alt_ft.round() as i64,
                final_landing.map(|l| l.vs_fpm),
                final_landing.map(|l| l.g_peak),
                f.paused_ms,
                f.max_sim_rate,
                f.started_airborne,
                status,
                f.end_reason,
                now,
                f.dep_lat,
                f.dep_lon,
                f.arr_lat,
                f.arr_lon,
            ],
        )?;
        let id = self.conn.last_insert_rowid();

        // Screenshots taken during the flight belong to it.
        self.conn.execute(
            "UPDATE screenshot SET flight_id = ?1 WHERE flight_id IS NULL AND taken_ms BETWEEN ?2 AND ?3",
            params![id, f.start_ms, f.end_ms],
        )?;

        // Logbook line. IFR, approaches and role are left for the pilot.
        let total = f.total_minutes().round() as i64;
        let (span_a, span_b) = f.counted_span();
        let night = night::night_minutes(&f.track, Some(span_a), Some(span_b)).min(total);
        let (day_ldg, night_ldg) = night::landing_split(f.landings.iter().map(|l| (l.t_ms, l.lat, l.lon)));
        let direct = geo::distance_nm(f.dep_lat, f.dep_lon, f.arr_lat, f.arr_lon);
        let xc = if direct > 50.0 { total } else { 0 };
        let mut remarks = String::new();
        if let Some(l) = final_landing {
            remarks.push_str(&format!("{:.0} fpm", l.vs_fpm));
            if l.bounces > 0 {
                remarks.push_str(&format!(", {} bounce{}", l.bounces, if l.bounces > 1 { "s" } else { "" }));
            }
            remarks.push_str(" · ");
        }
        if let Some(d) = ctx.diverted_from.as_deref().filter(|d| !d.is_empty()) {
            remarks.push_str(&format!("Diverted from {d} · "));
        }
        remarks.push_str(f.sim.label());
        if f.max_sim_rate > 1.01 {
            remarks.push_str(&format!(" · sim rate up to {:.0}x", f.max_sim_rate));
        }
        self.conn.execute(
            "INSERT INTO log_entry (flight_id, total_min, xc_min, pic_min, day_landings, night_landings, night_min, remarks)
             VALUES (?1, ?2, ?3, ?2, ?4, ?5, ?6, ?7)",
            params![id, total, xc, day_ldg, night_ldg, night, remarks],
        )?;

        // Track: keep full resolution around each touchdown.
        let windows: Vec<(i64, i64)> = f.landings.iter().map(|l| (l.t_ms - 60_000, l.t_ms + 30_000)).collect();
        let simplified = codec::simplify(&f.track, SIMPLIFY_M, |p| {
            windows.iter().any(|(a, b)| p.t_ms >= *a && p.t_ms <= *b)
        });
        if !simplified.is_empty() {
            let (mut x0, mut y0, mut x1, mut y1) = (180.0f64, 90.0f64, -180.0f64, -90.0f64);
            for p in &simplified {
                x0 = x0.min(p.lon);
                y0 = y0.min(p.lat);
                x1 = x1.max(p.lon);
                y1 = y1.max(p.lat);
            }
            self.conn.execute(
                "INSERT INTO track (flight_id, points, point_count, bbox) VALUES (?1, ?2, ?3, ?4)",
                params![id, codec::encode(&simplified)?, simplified.len() as i64, serde_json::to_string(&[x0, y0, x1, y1])?],
            )?;
        }

        let mut ev = self.conn.prepare("INSERT INTO flight_event (flight_id, t, kind, data) VALUES (?1, ?2, ?3, ?4)")?;
        for e in &f.events {
            ev.execute(params![id, e.t_ms, e.kind, e.data.to_string()])?;
        }
        for l in &f.landings {
            ev.execute(params![id, l.t_ms, "landing_report", serde_json::to_string(l)?])?;
        }
        if let Some(src) = &ctx.source {
            let packed = zstd::bulk::compress(src.file.as_bytes(), 9)?;
            self.conn.execute(
                "UPDATE flight SET source = ?1, source_id = ?2, source_file = ?3 WHERE id = ?4",
                params![src.source, src.id, packed, id],
            )?;
        }
        if let Some(plan) = &ctx.plan {
            // Plans carry the whole briefing (OFP text, NOTAMs), which
            // shrinks several times with zstd. Stored as a blob.
            let packed = zstd::bulk::compress(plan.as_bytes(), 9)?;
            self.conn.execute("UPDATE flight SET plan = ?1 WHERE id = ?2", params![packed, id])?;
        }
        Ok(id)
    }
}

/// Version 2 adds flight end points and night time. Work them out for
/// flights saved before it, from their stored tracks and landings.
/// (id, block out, block in, start, end, total minutes)
type BackfillRow = (i64, Option<i64>, Option<i64>, i64, i64, i64);

fn backfill_v2(conn: &Connection) -> Result<()> {
    let mut st = conn.prepare(
        "SELECT f.id, f.block_out, f.block_in, f.start_ms, f.end_ms, l.total_min
         FROM flight f JOIN log_entry l ON l.flight_id = f.id",
    )?;
    let flights: Vec<BackfillRow> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?
        .collect::<std::result::Result<_, _>>()?;
    for (id, out, inn, start, end, total) in flights {
        let blob: Option<Vec<u8>> =
            conn.query_row("SELECT points FROM track WHERE flight_id = ?1", params![id], |r| r.get(0)).optional()?;
        let track = blob.map(|b| codec::decode(&b)).transpose()?.unwrap_or_default();
        let mut ev = conn.prepare("SELECT data FROM flight_event WHERE flight_id = ?1 AND kind = 'landing_report'")?;
        let landings: Vec<LandingReport> = ev
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok().and_then(|s| serde_json::from_str(&s).ok()))
            .collect();
        let night = night::night_minutes(&track, out.or(Some(start)), inn.or(Some(end))).min(total);
        let (day_ldg, night_ldg) = night::landing_split(landings.iter().map(|l| (l.t_ms, l.lat, l.lon)));
        conn.execute(
            "UPDATE log_entry SET night_min = ?2, day_landings = ?3, night_landings = ?4 WHERE flight_id = ?1",
            params![id, night, day_ldg, night_ldg],
        )?;
        let first = track.first();
        let last = landings.iter().rev().find(|l| !l.touch_and_go).map(|l| (l.lat, l.lon)).or(track.last().map(|p| (p.lat, p.lon)));
        conn.execute(
            "UPDATE flight SET dep_lat = ?2, dep_lon = ?3, arr_lat = ?4, arr_lon = ?5 WHERE id = ?1",
            params![id, first.map(|p| p.lat), first.map(|p| p.lon), last.map(|p| p.0), last.map(|p| p.1)],
        )?;
    }
    Ok(())
}

/// Version 13 counts flight time from wheels up to touchdown only. Logbook
/// lines saved before it counted block out to block in (or the takeoff roll
/// to the end of the rollout): count them again, unless the pilot changed
/// the time. Night time follows the shorter span.
pub(crate) fn air_time_v13(conn: &Connection) -> Result<()> {
    // (id, start, end, block out, block in, takeoff, landing, paused, total, night, pic, xc)
    type Row = (i64, i64, i64, Option<i64>, Option<i64>, Option<i64>, Option<i64>, i64, i64, i64, i64, i64);
    let mut st = conn.prepare(
        "SELECT f.id, f.start_ms, f.end_ms, f.block_out, f.block_in, f.takeoff, f.landing, f.paused_ms,
                l.total_min, l.night_min, l.pic_min, l.xc_min
         FROM flight f JOIN log_entry l ON l.flight_id = f.id
         WHERE (f.takeoff IS NOT NULL OR f.landing IS NOT NULL) AND l.locked = 0",
    )?;
    let rows: Vec<Row> = st
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?))
        })?
        .collect::<std::result::Result<_, _>>()?;
    let min = |ms: i64| (ms.max(0) as f64 / 60_000.0).round() as i64;
    for (id, start, end, out, inn, takeoff, landing, paused, total, night, pic, xc) in rows {
        // What was counted before.
        let old = match (out, inn) {
            (Some(o), Some(i)) => min(i - o - paused),
            _ => {
                let a = out.or(takeoff.map(|t| (t - 120_000).max(start))).unwrap_or(start);
                let b = inn.or(landing.map(|l| (l + 300_000).min(end))).unwrap_or(end).max(a);
                min(b - a - if (a, b) == (start, end) { paused } else { 0 })
            }
        };
        if (total - old).abs() > 1 {
            continue; // changed by the pilot
        }
        let a = takeoff.unwrap_or(start).max(start);
        let b = landing.unwrap_or(end).min(end).max(a);
        let mut ev = conn.prepare("SELECT t, kind FROM flight_event WHERE flight_id = ?1 AND kind IN ('paused', 'resumed') ORDER BY t")?;
        let events: Vec<(i64, String)> = ev.query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<std::result::Result<_, _>>()?;
        let new = min(b - a - tracker::paused_within(events.iter().map(|(t, k)| (*t, k.as_str())), a, b));
        if new == total {
            continue;
        }
        let blob: Option<Vec<u8>> = conn.query_row("SELECT points FROM track WHERE flight_id = ?1", params![id], |r| r.get(0)).optional()?;
        let track = blob.and_then(|b| codec::decode(&b).ok()).unwrap_or_default();
        let new_night = if track.is_empty() { night.min(new) } else { night::night_minutes(&track, Some(a), Some(b)).min(night).min(new) };
        conn.execute(
            "UPDATE log_entry SET total_min = ?2, night_min = ?3, pic_min = ?4, xc_min = ?5 WHERE flight_id = ?1",
            params![id, new, new_night, if pic == total { new } else { pic.min(new) }, if xc == total { new } else { xc.min(new) }],
        )?;
    }
    Ok(())
}

impl Store {
    fn aircraft_id_with(conn: &Connection, a: &AircraftInfo) -> Result<i64> {
        // (id, merged into)
        let exact: Option<(i64, Option<i64>)> = conn
            .query_row(
                "SELECT id, merged_into FROM aircraft WHERE title = ?1 AND registration = ?2",
                params![a.title, a.registration],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        // Merged into another aircraft: fly as that one.
        if let Some((_, Some(into))) = exact {
            return Ok(into);
        }
        let existing = exact.map(|e| e.0);
        // The registration is how the hangar knows an aircraft: a sim
        // registration already in the hangar flies as that aircraft.
        if existing.is_none() {
            if let Some(id) = aircraft_by_reg(conn, &a.registration)? {
                return Ok(id);
            }
        }
        let class = category_class(a);
        if let Some(id) = existing {
            conn.execute(
                "UPDATE aircraft SET icao_type = ?2, engine_count = ?3, engine_type = ?4, category_class = ?5 WHERE id = ?1",
                params![id, a.icao_type, a.engine_count, a.engine_type.as_str(), class],
            )?;
            return Ok(id);
        }
        conn.execute(
            "INSERT INTO aircraft (icao_type, title, registration, engine_count, engine_type, category_class)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![a.icao_type, a.title, a.registration, a.engine_count, a.engine_type.as_str(), class],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

const FLIGHT_ROW_SQL: &str = "
    SELECT f.id, f.sim, f.status, COALESCE(a.title, ''), COALESCE(a.icao_type, ''), COALESCE(NULLIF(a.user_reg, ''), a.registration, ''),
           f.callsign, f.dep_icao, f.dep_name, f.dep_country, f.arr_icao, f.arr_name, f.arr_country,
           f.start_ms, f.end_ms, f.block_out, f.takeoff, f.landing, f.block_in,
           COALESCE(l.total_min, 0), f.distance_nm, f.max_alt_ft, f.fuel_used_kg,
           f.landing_rate_fpm, f.landing_g, COALESCE(l.day_landings + l.night_landings, 0)
    FROM flight f
    LEFT JOIN aircraft a ON a.id = f.aircraft_id
    LEFT JOIN log_entry l ON l.flight_id = f.id";

fn flight_row(r: &rusqlite::Row) -> rusqlite::Result<FlightRow> {
    let rate: Option<f64> = r.get(23)?;
    Ok(FlightRow {
        id: r.get(0)?,
        sim: r.get(1)?,
        status: r.get(2)?,
        aircraft_title: r.get(3)?,
        icao_type: r.get(4)?,
        registration: r.get(5)?,
        callsign: r.get(6)?,
        dep_icao: r.get(7)?,
        dep_name: r.get(8)?,
        dep_country: r.get(9)?,
        arr_icao: r.get(10)?,
        arr_name: r.get(11)?,
        arr_country: r.get(12)?,
        start_ms: r.get(13)?,
        end_ms: r.get(14)?,
        block_out: r.get(15)?,
        takeoff: r.get(16)?,
        landing: r.get(17)?,
        block_in: r.get(18)?,
        total_min: r.get(19)?,
        distance_nm: r.get(20)?,
        max_alt_ft: r.get(21)?,
        fuel_used_kg: r.get(22)?,
        landing_rate_fpm: rate,
        landing_g: r.get(24)?,
        landing_grade: rate.map(|v| tracker::grade_for(v).to_string()),
        landings: r.get(25)?,
    })
}

#[cfg(test)]
mod tests;
