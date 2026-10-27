//! Local storage: one SQLite file holds every flight, its logbook line, its
//! compressed track and its events.

pub mod book;
pub mod codec;
pub mod journal;
pub mod night;
pub mod passport;

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use telemetry::{geo, AircraftInfo, EngineType, TrackPoint};
use tracker::{FlightStatus, FlightSummary, LandingReport};

const SCHEMA_V1: &str = include_str!("schema.sql");
const MIGRATE_V2: &str = include_str!("migrate_v2.sql");
const MIGRATE_V3: &str = include_str!("migrate_v3.sql");
const SCHEMA_VERSION: i64 = 3;
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
    match a.engine_type {
        EngineType::Helo => "RH",
        EngineType::Glider => "GL",
        _ if a.engine_count > 1 => "MEL",
        _ => "SEL",
    }
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

    /// The flight plan saved with a flight, as JSON.
    pub fn get_plan(&self, id: i64) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT plan FROM flight WHERE id = ?1", params![id], |r| r.get::<_, Option<String>>(0))
            .optional()?
            .flatten())
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
            "DELETE FROM aircraft WHERE id NOT IN (SELECT DISTINCT aircraft_id FROM flight WHERE aircraft_id IS NOT NULL)",
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
        let aircraft_id = Store::aircraft_id_with(self.conn, &f.aircraft)?;
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

        // Logbook line. IFR, approaches and role are left for the pilot.
        let total = f.total_minutes().round() as i64;
        let night = night::night_minutes(&f.track, f.block_out_ms.or(Some(f.start_ms)), f.block_in_ms.or(Some(f.end_ms)))
            .min(total);
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
        if let Some(plan) = &ctx.plan {
            self.conn.execute("UPDATE flight SET plan = ?1 WHERE id = ?2", params![plan, id])?;
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

impl Store {
    fn aircraft_id_with(conn: &Connection, a: &AircraftInfo) -> Result<i64> {
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM aircraft WHERE title = ?1 AND registration = ?2",
                params![a.title, a.registration],
                |r| r.get(0),
            )
            .optional()?;
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
    SELECT f.id, f.sim, f.status, COALESCE(a.title, ''), COALESCE(a.icao_type, ''), COALESCE(a.registration, ''),
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
