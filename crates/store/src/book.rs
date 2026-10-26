//! Queries behind the logbook, passport, hangar and settings.

use std::collections::HashMap;

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::passport::{StampFlight, StampPlace};
use crate::{Result, Store};

/// Settings and their defaults.
pub const DEFAULT_SETTINGS: &[(&str, &str)] = &[
    ("distance_unit", "nm"),      // nm | mi | km
    ("logbook_layout", "faa"),    // faa | easa
    ("logbook_hand", "hand"),     // hand | type
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pilot {
    pub name: String,
    pub pilot_id: String,
    pub home_icao: String,
    pub nationality: String,
    pub photo_url: Option<String>,
    pub signature_svg: Option<String>,
}

/// One logbook line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRow {
    pub flight_id: i64,
    pub date_ms: i64,
    pub sim: String,
    pub status: String,
    pub aircraft_type: String,
    pub ident: String,
    pub category_class: String,
    pub engine_count: i64,
    pub from_icao: String,
    pub to_icao: String,
    pub out_ms: Option<i64>,
    pub in_ms: Option<i64>,
    pub total_min: i64,
    pub night_min: i64,
    pub ifr_min: i64,
    pub xc_min: i64,
    pub pic_min: i64,
    pub dual_min: i64,
    pub sim_inst_min: i64,
    pub act_inst_min: i64,
    pub day_landings: i64,
    pub night_landings: i64,
    pub approaches: i64,
    pub remarks: String,
}

/// Fields the pilot may correct on a logbook line. `None` = unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogPatch {
    pub total_min: Option<i64>,
    pub night_min: Option<i64>,
    pub ifr_min: Option<i64>,
    pub xc_min: Option<i64>,
    pub pic_min: Option<i64>,
    pub dual_min: Option<i64>,
    pub sim_inst_min: Option<i64>,
    pub act_inst_min: Option<i64>,
    pub day_landings: Option<i64>,
    pub night_landings: Option<i64>,
    pub approaches: Option<i64>,
    pub remarks: Option<String>,
}

/// Flight facts the pilot may correct from the logbook. `None` = unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlightPatch {
    /// Any time on the new date (UTC); every time of the flight moves with it.
    pub date_ms: Option<i64>,
    /// Block out and block in, minutes after midnight UTC. An in time
    /// earlier than the out time is taken as the next day.
    pub out_min: Option<i64>,
    pub in_min: Option<i64>,
    pub dep: Option<crate::Place>,
    pub arr: Option<crate::Place>,
    pub aircraft_type: Option<String>,
    pub ident: Option<String>,
}

/// One aircraft in the hangar, with its totals and last known place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HangarRow {
    pub id: i64,
    pub title: String,
    pub icao_type: String,
    pub registration: String,
    pub engine_type: String,
    pub engine_count: i64,
    pub category_class: String,
    pub has_photo: bool,
    pub flights: i64,
    pub minutes: i64,
    pub distance_nm: f64,
    pub landings: i64,
    pub first_flight_ms: i64,
    pub last_flight_ms: i64,
    pub best_landing_fpm: Option<f64>,
    pub last_icao: Option<String>,
    pub last_name: Option<String>,
    pub last_country: Option<String>,
    pub last_region: Option<String>,
    pub last_lat: Option<f64>,
    pub last_lon: Option<f64>,
    pub last_sim: String,
}

/// One flown route for the map: where a flight left from and landed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRow {
    pub flight_id: i64,
    pub dep_icao: Option<String>,
    pub arr_icao: Option<String>,
    pub dep_lat: Option<f64>,
    pub dep_lon: Option<f64>,
    pub arr_lat: Option<f64>,
    pub arr_lon: Option<f64>,
    pub date_ms: i64,
    pub aircraft_type: String,
}

impl Store {
    /// Every flight's end points, oldest first.
    pub fn routes(&self) -> Result<Vec<RouteRow>> {
        let mut st = self.conn.prepare(
            "SELECT f.id, f.dep_icao, f.arr_icao, f.dep_lat, f.dep_lon, f.arr_lat, f.arr_lon,
                    COALESCE(f.block_out, f.takeoff, f.start_ms), COALESCE(NULLIF(a.icao_type, ''), a.title, '')
             FROM flight f LEFT JOIN aircraft a ON a.id = f.aircraft_id
             ORDER BY COALESCE(f.block_out, f.takeoff, f.start_ms) ASC",
        )?;
        let rows = st.query_map([], |r| {
            Ok(RouteRow {
                flight_id: r.get(0)?,
                dep_icao: r.get(1)?,
                arr_icao: r.get(2)?,
                dep_lat: r.get(3)?,
                dep_lon: r.get(4)?,
                arr_lat: r.get(5)?,
                arr_lon: r.get(6)?,
                date_ms: r.get(7)?,
                aircraft_type: r.get(8)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn settings(&self) -> Result<HashMap<String, String>> {
        let mut out: HashMap<String, String> =
            DEFAULT_SETTINGS.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let mut st = self.conn.prepare("SELECT key, value FROM settings")?;
        for row in st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (k, v) = row?;
            out.insert(k, v);
        }
        Ok(out)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn pilot(&self) -> Result<Pilot> {
        Ok(self.conn.query_row(
            "SELECT name, pilot_id, home_icao, nationality, photo_url, signature_svg FROM pilot WHERE id = 1",
            [],
            |r| {
                Ok(Pilot {
                    name: r.get(0)?,
                    pilot_id: r.get(1)?,
                    home_icao: r.get(2)?,
                    nationality: r.get(3)?,
                    photo_url: r.get(4)?,
                    signature_svg: r.get(5)?,
                })
            },
        )?)
    }

    pub fn set_pilot(&self, p: &Pilot) -> Result<()> {
        self.conn.execute(
            "UPDATE pilot SET name = ?1, pilot_id = ?2, home_icao = ?3, nationality = ?4, photo_url = ?5, signature_svg = ?6
             WHERE id = 1",
            params![p.name, p.pilot_id, p.home_icao.to_uppercase(), p.nationality, p.photo_url, p.signature_svg],
        )?;
        Ok(())
    }

    /// Every logbook line, oldest first, as a paper logbook is kept.
    pub fn logbook(&self) -> Result<Vec<LogRow>> {
        let mut st = self.conn.prepare(
            "SELECT f.id, COALESCE(f.block_out, f.takeoff, f.start_ms), f.sim, f.status,
                    COALESCE(NULLIF(a.icao_type, ''), a.title, ''), COALESCE(a.registration, ''),
                    COALESCE(a.category_class, ''), COALESCE(a.engine_count, 0),
                    COALESCE(f.dep_icao, ''), COALESCE(f.arr_icao, ''),
                    COALESCE(f.block_out, f.takeoff), COALESCE(f.block_in, f.landing),
                    l.total_min, l.night_min, l.ifr_min, l.xc_min, l.pic_min, l.dual_min,
                    l.sim_inst_min, l.act_inst_min, l.day_landings, l.night_landings, l.approaches, l.remarks
             FROM flight f
             JOIN log_entry l ON l.flight_id = f.id
             LEFT JOIN aircraft a ON a.id = f.aircraft_id
             ORDER BY COALESCE(f.block_out, f.takeoff, f.start_ms) ASC, f.id ASC",
        )?;
        let rows = st.query_map([], |r| {
            Ok(LogRow {
                flight_id: r.get(0)?,
                date_ms: r.get(1)?,
                sim: r.get(2)?,
                status: r.get(3)?,
                aircraft_type: r.get(4)?,
                ident: r.get(5)?,
                category_class: r.get(6)?,
                engine_count: r.get(7)?,
                from_icao: r.get(8)?,
                to_icao: r.get(9)?,
                out_ms: r.get(10)?,
                in_ms: r.get(11)?,
                total_min: r.get(12)?,
                night_min: r.get(13)?,
                ifr_min: r.get(14)?,
                xc_min: r.get(15)?,
                pic_min: r.get(16)?,
                dual_min: r.get(17)?,
                sim_inst_min: r.get(18)?,
                act_inst_min: r.get(19)?,
                day_landings: r.get(20)?,
                night_landings: r.get(21)?,
                approaches: r.get(22)?,
                remarks: r.get(23)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn update_log_entry(&self, flight_id: i64, p: &LogPatch) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE log_entry SET
                total_min = COALESCE(?2, total_min), night_min = COALESCE(?3, night_min),
                ifr_min = COALESCE(?4, ifr_min), xc_min = COALESCE(?5, xc_min), pic_min = COALESCE(?6, pic_min),
                dual_min = COALESCE(?7, dual_min), sim_inst_min = COALESCE(?8, sim_inst_min),
                act_inst_min = COALESCE(?9, act_inst_min), day_landings = COALESCE(?10, day_landings),
                night_landings = COALESCE(?11, night_landings), approaches = COALESCE(?12, approaches),
                remarks = COALESCE(?13, remarks)
             WHERE flight_id = ?1",
            params![
                flight_id,
                p.total_min.map(|v| v.max(0)),
                p.night_min.map(|v| v.max(0)),
                p.ifr_min.map(|v| v.max(0)),
                p.xc_min.map(|v| v.max(0)),
                p.pic_min.map(|v| v.max(0)),
                p.dual_min.map(|v| v.max(0)),
                p.sim_inst_min.map(|v| v.max(0)),
                p.act_inst_min.map(|v| v.max(0)),
                p.day_landings.map(|v| v.max(0)),
                p.night_landings.map(|v| v.max(0)),
                p.approaches.map(|v| v.max(0)),
                p.remarks,
            ],
        )?;
        if n > 0 {
            self.conn.execute(
                "UPDATE flight SET updated_at = ?2 WHERE id = ?1",
                params![flight_id, crate::now_ms()],
            )?;
        }
        Ok(n > 0)
    }

    /// Correct a flight's facts (date, times, airports, aircraft).
    pub fn update_flight(&mut self, flight_id: i64, p: &FlightPatch) -> Result<bool> {
        const DAY: i64 = 86_400_000;
        let tx = self.conn.transaction()?;
        // (date anchor, block out, block in, aircraft)
        type Times = (i64, Option<i64>, Option<i64>, Option<i64>);
        let row: Option<Times> = tx
            .query_row(
                "SELECT COALESCE(block_out, takeoff, start_ms), block_out, block_in, aircraft_id FROM flight WHERE id = ?1",
                params![flight_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let Some((anchor, mut out, mut inn, aircraft_id)) = row else { return Ok(false) };

        // Move the whole flight to the new date, keeping its times of day.
        let mut day = anchor.div_euclid(DAY) * DAY;
        if let Some(d) = p.date_ms {
            let new_day = d.div_euclid(DAY) * DAY;
            let shift = new_day - day;
            if shift != 0 {
                tx.execute(
                    "UPDATE flight SET start_ms = start_ms + ?2, end_ms = end_ms + ?2,
                        block_out = block_out + ?2, takeoff = takeoff + ?2,
                        landing = landing + ?2, block_in = block_in + ?2 WHERE id = ?1",
                    params![flight_id, shift],
                )?;
                tx.execute("UPDATE flight_event SET t = t + ?2 WHERE flight_id = ?1", params![flight_id, shift])?;
                out = out.map(|t| t + shift);
                inn = inn.map(|t| t + shift);
                day = new_day;
            }
        }
        if let Some(m) = p.out_min {
            out = Some(day + m.clamp(0, 1439) * 60_000);
        }
        if let Some(m) = p.in_min {
            let mut t = day + m.clamp(0, 1439) * 60_000;
            if out.is_some_and(|o| t < o) {
                t += DAY;
            }
            inn = Some(t);
        }
        if p.out_min.is_some() || p.in_min.is_some() {
            tx.execute("UPDATE flight SET block_out = ?2, block_in = ?3 WHERE id = ?1", params![flight_id, out, inn])?;
        }

        let opt = |s: &str| (!s.is_empty()).then(|| s.to_string());
        if let Some(d) = &p.dep {
            tx.execute(
                "UPDATE flight SET dep_icao = ?2, dep_name = ?3, dep_country = ?4, dep_region = ?5 WHERE id = ?1",
                params![flight_id, opt(&d.icao), opt(&d.name), opt(&d.country), opt(&d.region)],
            )?;
        }
        if let Some(a) = &p.arr {
            tx.execute(
                "UPDATE flight SET arr_icao = ?2, arr_name = ?3, arr_country = ?4, arr_region = ?5 WHERE id = ?1",
                params![flight_id, opt(&a.icao), opt(&a.name), opt(&a.country), opt(&a.region)],
            )?;
        }

        // A different type or registration points this flight (only) at the
        // matching hangar aircraft, created if needed.
        if p.aircraft_type.is_some() || p.ident.is_some() {
            // (title, type, registration, engines, engine type, category/class)
            type Ac = (String, String, String, i64, String, String);
            let cur: Ac = match aircraft_id {
                Some(id) => tx.query_row(
                    "SELECT title, icao_type, registration, engine_count, engine_type, category_class FROM aircraft WHERE id = ?1",
                    params![id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
                )?,
                None => (String::new(), String::new(), String::new(), 0, "unknown".into(), String::new()),
            };
            let ty = p.aircraft_type.clone().unwrap_or(cur.1.clone()).trim().to_uppercase();
            let reg = p.ident.clone().unwrap_or(cur.2.clone()).trim().to_uppercase();
            if ty != cur.1 || reg != cur.2 {
                let title = if ty == cur.1 && !cur.0.is_empty() { cur.0.clone() } else { ty.clone() };
                let existing: Option<i64> = tx
                    .query_row(
                        "SELECT id FROM aircraft WHERE icao_type = ?1 AND registration = ?2 ORDER BY id LIMIT 1",
                        params![ty, reg],
                        |r| r.get(0),
                    )
                    .optional()?;
                let new_id = match existing {
                    Some(id) => id,
                    None => {
                        tx.execute(
                            "INSERT INTO aircraft (icao_type, title, registration, engine_count, engine_type, category_class)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                             ON CONFLICT(title, registration) DO UPDATE SET icao_type = excluded.icao_type",
                            params![ty, title, reg, cur.3, cur.4, cur.5],
                        )?;
                        tx.query_row(
                            "SELECT id FROM aircraft WHERE title = ?1 AND registration = ?2",
                            params![title, reg],
                            |r| r.get(0),
                        )?
                    }
                };
                tx.execute("UPDATE flight SET aircraft_id = ?2 WHERE id = ?1", params![flight_id, new_id])?;
                tx.execute(
                    "DELETE FROM aircraft WHERE id NOT IN (SELECT DISTINCT aircraft_id FROM flight WHERE aircraft_id IS NOT NULL)",
                    [],
                )?;
            }
        }
        tx.execute("UPDATE flight SET updated_at = ?2 WHERE id = ?1", params![flight_id, crate::now_ms()])?;
        tx.commit()?;
        Ok(true)
    }

    /// Every aircraft flown, most recently flown first.
    pub fn hangar(&self) -> Result<Vec<HangarRow>> {
        let mut st = self.conn.prepare(
            "SELECT a.id, a.title, a.icao_type, a.registration, a.engine_type, a.engine_count, a.category_class,
                    a.photo IS NOT NULL,
                    COUNT(f.id), COALESCE(SUM(l.total_min), 0), COALESCE(SUM(f.distance_nm), 0),
                    COALESCE(SUM(l.day_landings + l.night_landings), 0),
                    MIN(f.start_ms), MAX(f.start_ms), MAX(f.landing_rate_fpm)
             FROM aircraft a
             JOIN flight f ON f.aircraft_id = a.id
             LEFT JOIN log_entry l ON l.flight_id = f.id
             GROUP BY a.id
             ORDER BY MAX(f.start_ms) DESC",
        )?;
        let mut rows: Vec<HangarRow> = st
            .query_map([], |r| {
                Ok(HangarRow {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    icao_type: r.get(2)?,
                    registration: r.get(3)?,
                    engine_type: r.get(4)?,
                    engine_count: r.get(5)?,
                    category_class: r.get(6)?,
                    has_photo: r.get(7)?,
                    flights: r.get(8)?,
                    minutes: r.get(9)?,
                    distance_nm: r.get(10)?,
                    landings: r.get(11)?,
                    first_flight_ms: r.get(12)?,
                    last_flight_ms: r.get(13)?,
                    best_landing_fpm: r.get(14)?,
                    last_icao: None,
                    last_name: None,
                    last_country: None,
                    last_region: None,
                    last_lat: None,
                    last_lon: None,
                    last_sim: String::new(),
                })
            })?
            .collect::<std::result::Result<_, _>>()?;
        let mut last = self.conn.prepare(
            "SELECT arr_icao, arr_name, arr_country, arr_region, arr_lat, arr_lon, sim
             FROM flight WHERE aircraft_id = ?1 ORDER BY start_ms DESC LIMIT 1",
        )?;
        for row in rows.iter_mut() {
            last.query_row(params![row.id], |r| {
                row.last_icao = r.get(0)?;
                row.last_name = r.get(1)?;
                row.last_country = r.get(2)?;
                row.last_region = r.get(3)?;
                row.last_lat = r.get(4)?;
                row.last_lon = r.get(5)?;
                row.last_sim = r.get(6)?;
                Ok(())
            })?;
        }
        Ok(rows)
    }

    pub fn aircraft_photo(&self, id: i64) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT photo FROM aircraft WHERE id = ?1", params![id], |r| r.get(0))
            .optional()?
            .flatten())
    }

    /// Set (or with `None`, remove) an aircraft's picture, as a data URL.
    pub fn set_aircraft_photo(&self, id: i64, data_url: Option<&str>) -> Result<bool> {
        Ok(self.conn.execute("UPDATE aircraft SET photo = ?2 WHERE id = ?1", params![id, data_url])? > 0)
    }

    /// Flights that reached an airport, oldest first, for passport stamps.
    pub fn stamp_flights(&self) -> Result<Vec<StampFlight>> {
        let mut st = self.conn.prepare(
            "SELECT f.id, COALESCE(f.takeoff, f.block_out, f.start_ms), COALESCE(f.landing, f.block_in, f.end_ms),
                    COALESCE(f.dep_icao, ''), COALESCE(f.dep_name, ''), COALESCE(f.dep_country, ''), COALESCE(f.dep_region, ''),
                    COALESCE(f.arr_icao, ''), COALESCE(f.arr_name, ''), COALESCE(f.arr_country, ''), COALESCE(f.arr_region, ''),
                    COALESCE(NULLIF(a.icao_type, ''), a.title, '')
             FROM flight f LEFT JOIN aircraft a ON a.id = f.aircraft_id
             WHERE f.landing IS NOT NULL AND f.dep_country IS NOT NULL AND f.arr_country IS NOT NULL
             ORDER BY COALESCE(f.takeoff, f.block_out, f.start_ms) ASC, f.id ASC",
        )?;
        let rows = st.query_map([], |r| {
            Ok(StampFlight {
                flight_id: r.get(0)?,
                dep_ms: r.get(1)?,
                arr_ms: r.get(2)?,
                dep: StampPlace { icao: r.get(3)?, name: r.get(4)?, country: r.get(5)?, region: r.get(6)? },
                arr: StampPlace { icao: r.get(7)?, name: r.get(8)?, country: r.get(9)?, region: r.get(10)? },
                aircraft_type: r.get(11)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Remove every flight from one sim (used to clear sample flights).
    pub fn delete_flights_from_sim(&mut self, sim: &str) -> Result<usize> {
        let n = self.conn.execute("DELETE FROM flight WHERE sim = ?1", params![sim])?;
        self.conn.execute("DELETE FROM aircraft WHERE id NOT IN (SELECT DISTINCT aircraft_id FROM flight WHERE aircraft_id IS NOT NULL)", [])?;
        Ok(n)
    }
}
