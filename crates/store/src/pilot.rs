//! What the pilot profile needs: every flight's scoring inputs, career
//! statistics and the profile banner.

use std::collections::HashMap;

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tracker::{Handling, LandingReport};

use crate::{Result, Store};

/// One flight, with what the pilot rating scores it on.
#[derive(Debug, Clone)]
pub struct RatingFlight {
    pub id: i64,
    pub start_ms: i64,
    pub sim: String,
    /// "" when FlightDeck recorded it; "manual", "volanta", ... otherwise.
    pub source: String,
    pub status: String,
    pub icao_type: String,
    pub title: String,
    pub dep_icao: String,
    pub arr_icao: String,
    pub takeoff_ms: Option<i64>,
    pub landing_ms: Option<i64>,
    pub total_min: i64,
    pub landings: Vec<LandingReport>,
    /// A touchdown worked out from the positions, when the flight came
    /// without a landing report.
    pub estimated_landing: Option<LandingReport>,
    /// Recorded from version 0.1.28 on; worked out from the positions for
    /// older and imported flights.
    pub handling: Option<Handling>,
}

/// A flight whose handling has not been worked out yet.
#[derive(Debug, Clone)]
pub struct NeedsEstimate {
    pub id: i64,
    pub start_ms: i64,
    pub source: String,
    pub has_landing: bool,
    pub arr_icao: Option<String>,
}

/// A flight singled out in the statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatFlight {
    pub id: i64,
    pub dep_icao: Option<String>,
    pub arr_icao: Option<String>,
    pub start_ms: i64,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Named {
    pub name: String,
    pub count: i64,
    pub minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthHours {
    /// "2026-10"
    pub month: String,
    pub minutes: i64,
    pub flights: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PilotStats {
    pub first_flight_ms: Option<i64>,
    pub longest: Option<StatFlight>,
    pub farthest: Option<StatFlight>,
    pub softest: Option<StatFlight>,
    pub avg_landing_fpm: Option<f64>,
    pub night_minutes: i64,
    pub aircraft_types: i64,
    pub top_types: Vec<Named>,
    pub top_airports: Vec<Named>,
    /// The last twelve months, oldest first, including empty ones.
    pub months: Vec<MonthHours>,
    /// Every month with flights, oldest first, for picking another year.
    #[serde(default)]
    pub history: Vec<MonthHours>,
}

impl Store {
    /// Every flight, oldest first, with its touchdowns and handling.
    pub fn rating_flights(&self) -> Result<Vec<RatingFlight>> {
        let mut flights: Vec<RatingFlight> = {
            let mut st = self.conn.prepare(
                "SELECT f.id, f.start_ms, f.sim, COALESCE(f.source, ''), f.status, COALESCE(a.icao_type, ''),
                        COALESCE(a.title, ''), COALESCE(f.dep_icao, ''), COALESCE(f.arr_icao, ''), f.takeoff, f.landing,
                        COALESCE(l.total_min, 0)
                 FROM flight f
                 LEFT JOIN aircraft a ON a.id = f.aircraft_id
                 LEFT JOIN log_entry l ON l.flight_id = f.id
                 ORDER BY f.start_ms, f.id",
            )?;
            let rows = st.query_map([], |r| {
                Ok(RatingFlight {
                    id: r.get(0)?,
                    start_ms: r.get(1)?,
                    sim: r.get(2)?,
                    source: r.get(3)?,
                    status: r.get(4)?,
                    icao_type: r.get(5)?,
                    title: r.get(6)?,
                    dep_icao: r.get(7)?,
                    arr_icao: r.get(8)?,
                    takeoff_ms: r.get(9)?,
                    landing_ms: r.get(10)?,
                    total_min: r.get(11)?,
                    landings: vec![],
                    estimated_landing: None,
                    handling: None,
                })
            })?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        let index: HashMap<i64, usize> = flights.iter().enumerate().map(|(i, f)| (f.id, i)).collect();
        let mut st = self.conn.prepare(
            "SELECT flight_id, kind, data FROM flight_event
             WHERE kind IN ('landing_report', 'handling', 'landing_estimate') ORDER BY flight_id, t",
        )?;
        let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
        for row in rows {
            let (id, kind, data) = row?;
            let Some(&i) = index.get(&id) else { continue };
            if kind == "handling" {
                flights[i].handling = serde_json::from_str(&data).ok();
            } else if kind == "landing_estimate" {
                flights[i].estimated_landing = serde_json::from_str(&data).ok();
            } else if let Ok(l) = serde_json::from_str(&data) {
                flights[i].landings.push(l);
            }
        }
        Ok(flights)
    }

    /// Flights flown in a sim whose handling has not been worked out yet:
    /// imported ones and those recorded before FlightDeck kept handling.
    pub fn flights_needing_estimates(&self) -> Result<Vec<NeedsEstimate>> {
        let mut st = self.conn.prepare(
            "SELECT f.id, f.start_ms, COALESCE(f.source, ''),
                    EXISTS (SELECT 1 FROM flight_event e WHERE e.flight_id = f.id AND e.kind = 'landing_report'),
                    f.arr_icao
             FROM flight f
             WHERE f.sim NOT IN ('demo', 'manual') AND COALESCE(f.source, '') <> 'manual'
               AND NOT EXISTS (SELECT 1 FROM flight_event e WHERE e.flight_id = f.id AND e.kind = 'handling')",
        )?;
        let rows = st.query_map([], |r| {
            Ok(NeedsEstimate { id: r.get(0)?, start_ms: r.get(1)?, source: r.get(2)?, has_landing: r.get(3)?, arr_icao: r.get(4)? })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Throw away worked-out handling and touchdowns saved by an older
    /// version, so they are worked out again the current way. Once per
    /// `version`; FlightDeck's own recordings are kept.
    pub fn reset_estimates(&self, version: u32) -> Result<bool> {
        let have: Option<String> =
            self.conn.query_row("SELECT value FROM settings WHERE key = 'estimates_version'", [], |r| r.get(0)).optional()?;
        if have.and_then(|v| v.parse::<u32>().ok()).is_some_and(|v| v >= version) {
            return Ok(false);
        }
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM flight_event WHERE kind = 'landing_estimate'
                OR (kind = 'handling' AND json_extract(data, '$.source') IN ('positions', 'track', 'none'))",
            [],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES ('estimates_version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![version.to_string()],
        )?;
        tx.commit()?;
        Ok(true)
    }

    /// Save what was worked out for a flight: its handling and, if it had
    /// no landing report, an estimated touchdown.
    pub fn save_estimates(&self, id: i64, t_ms: i64, handling: &Handling, landing: Option<&LandingReport>) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO flight_event (flight_id, t, kind, data) VALUES (?1, ?2, 'handling', ?3)",
            params![id, t_ms, serde_json::to_string(handling)?],
        )?;
        if let Some(l) = landing {
            tx.execute(
                "INSERT INTO flight_event (flight_id, t, kind, data) VALUES (?1, ?2, 'landing_estimate', ?3)",
                params![id, l.t_ms, serde_json::to_string(l)?],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Career statistics for the pilot profile.
    pub fn pilot_stats(&self, now_ms: i64) -> Result<PilotStats> {
        let c = &self.conn;
        let stat = |sql: &str| -> Result<Option<StatFlight>> {
            Ok(c.query_row(sql, [], |r| {
                Ok(StatFlight { id: r.get(0)?, dep_icao: r.get(1)?, arr_icao: r.get(2)?, start_ms: r.get(3)?, value: r.get(4)? })
            })
            .optional()?)
        };
        let named = |sql: &str| -> Result<Vec<Named>> {
            let mut st = c.prepare(sql)?;
            let rows = st.query_map([], |r| Ok(Named { name: r.get(0)?, count: r.get(1)?, minutes: r.get(2)? }))?;
            Ok(rows.collect::<std::result::Result<_, _>>()?)
        };
        let mut s = PilotStats {
            first_flight_ms: c.query_row("SELECT MIN(start_ms) FROM flight", [], |r| r.get(0))?,
            longest: stat(
                "SELECT f.id, f.dep_icao, f.arr_icao, f.start_ms, CAST(l.total_min AS REAL) FROM flight f
                 JOIN log_entry l ON l.flight_id = f.id ORDER BY l.total_min DESC LIMIT 1",
            )?,
            farthest: stat(
                "SELECT id, dep_icao, arr_icao, start_ms, distance_nm FROM flight WHERE distance_nm > 0
                 ORDER BY distance_nm DESC LIMIT 1",
            )?,
            softest: stat(
                "SELECT id, dep_icao, arr_icao, start_ms, landing_rate_fpm FROM flight
                 WHERE landing_rate_fpm IS NOT NULL AND landing_rate_fpm < 0 AND sim NOT IN ('demo', 'manual')
                 ORDER BY landing_rate_fpm DESC LIMIT 1",
            )?,
            avg_landing_fpm: c.query_row(
                "SELECT AVG(landing_rate_fpm) FROM flight
                 WHERE landing_rate_fpm IS NOT NULL AND landing_rate_fpm < 0 AND sim NOT IN ('demo', 'manual')",
                [],
                |r| r.get(0),
            )?,
            night_minutes: c.query_row("SELECT COALESCE(SUM(night_min), 0) FROM log_entry", [], |r| r.get(0))?,
            aircraft_types: c.query_row(
                "SELECT COUNT(DISTINCT a.icao_type) FROM flight f JOIN aircraft a ON a.id = f.aircraft_id WHERE a.icao_type <> ''",
                [],
                |r| r.get(0),
            )?,
            top_types: named(
                "SELECT a.icao_type, COUNT(*), COALESCE(SUM(l.total_min), 0) FROM flight f
                 JOIN aircraft a ON a.id = f.aircraft_id LEFT JOIN log_entry l ON l.flight_id = f.id
                 WHERE a.icao_type <> '' GROUP BY a.icao_type ORDER BY 3 DESC, 2 DESC LIMIT 5",
            )?,
            top_airports: named(
                "SELECT icao, COUNT(*), 0 FROM (SELECT dep_icao AS icao FROM flight WHERE dep_icao IS NOT NULL
                                                 UNION ALL SELECT arr_icao FROM flight WHERE arr_icao IS NOT NULL)
                 GROUP BY icao ORDER BY 2 DESC, 1 LIMIT 5",
            )?,
            months: vec![],
            history: vec![],
        };
        // Hours per month over the last year, in UTC.
        let by_month: HashMap<String, (i64, i64)> = {
            let mut st = c.prepare(
                // Every flight counts, recorded or imported; one whose
                // logbook line has no time uses wheels up to touchdown (or start to end).
                "SELECT strftime('%Y-%m', f.start_ms / 1000, 'unixepoch'),
                        COALESCE(SUM(CASE WHEN COALESCE(l.total_min, 0) > 0 THEN l.total_min
                                          ELSE MAX(0, (COALESCE(f.landing, f.end_ms) - COALESCE(f.takeoff, f.start_ms)) / 60000) END), 0),
                        COUNT(*)
                 FROM flight f LEFT JOIN log_entry l ON l.flight_id = f.id
                 GROUP BY 1",
            )?;
            let rows = st.query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        let mut history: Vec<MonthHours> =
            by_month.iter().map(|(k, &(minutes, flights))| MonthHours { month: k.clone(), minutes, flights }).collect();
        history.sort_by(|a, b| a.month.cmp(&b.month));
        s.history = history;
        let (mut y, mut m): (i64, i64) =
            c.query_row("SELECT CAST(strftime('%Y', ?1 / 1000, 'unixepoch') AS INTEGER), CAST(strftime('%m', ?1 / 1000, 'unixepoch') AS INTEGER)", params![now_ms], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
        let mut months = Vec::with_capacity(12);
        for _ in 0..12 {
            let key = format!("{y:04}-{m:02}");
            let (minutes, flights) = by_month.get(&key).copied().unwrap_or((0, 0));
            months.push(MonthHours { month: key, minutes, flights });
            m -= 1;
            if m == 0 {
                m = 12;
                y -= 1;
            }
        }
        months.reverse();
        s.months = months;
        Ok(s)
    }

    /// The profile banner picture, as a data URL.
    pub fn banner(&self) -> Result<Option<String>> {
        Ok(self.conn.query_row("SELECT banner FROM pilot WHERE id = 1", [], |r| r.get(0))?)
    }

    pub fn set_banner(&self, data_url: Option<&str>) -> Result<()> {
        self.conn.execute("UPDATE pilot SET banner = ?1 WHERE id = 1", params![data_url])?;
        Ok(())
    }

    /// Just the profile photo (Settings sets it with the rest of the pilot).
    pub fn set_pilot_photo(&self, data_url: Option<&str>) -> Result<()> {
        self.conn.execute("UPDATE pilot SET photo_url = ?1 WHERE id = 1", params![data_url])?;
        Ok(())
    }
}
