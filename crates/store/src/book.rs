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
    /// SEL, MEL, SES or MES for this line; empty = the aircraft's own class.
    #[serde(default)]
    pub class_override: Option<String>,
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
    /// New airport positions, [lat, lon]; used only by flights without a
    /// track (entered by hand), whose line runs between the airports.
    #[serde(default)]
    pub dep_at: Option<[f64; 2]>,
    #[serde(default)]
    pub arr_at: Option<[f64; 2]>,
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
    /// Average landing rate over the flights that recorded one.
    pub avg_landing_fpm: Option<f64>,
    pub favorite: bool,
    /// Callsign of the latest flight (helps tell cargo from passenger).
    pub last_callsign: String,
    pub last_icao: Option<String>,
    pub last_name: Option<String>,
    pub last_country: Option<String>,
    pub last_region: Option<String>,
    pub last_lat: Option<f64>,
    pub last_lon: Option<f64>,
    pub last_sim: String,
    /// The pilot's own model name, if set (shown instead of the sim's).
    pub model: Option<String>,
    /// Airline the aircraft flies for, if the pilot set one.
    pub airline: Option<String>,
    /// The registration the sim reports, kept as the matching key.
    pub sim_registration: String,
    /// Added by hand in the hangar.
    pub manual: bool,
}

/// The pilot's details for an aircraft. `None` = unchanged; an empty
/// string clears the field.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AircraftEdit {
    pub registration: Option<String>,
    pub model: Option<String>,
    pub airline: Option<String>,
}

/// An aircraft added by hand in the hangar.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewAircraft {
    pub registration: String,
    pub model: String,
    pub icao_type: String,
    /// piston | turboprop | jet | helo | glider
    pub engine_type: String,
    pub engine_count: i64,
    pub airline: Option<String>,
}

/// A flight entered by hand in the logbook. It has no track: the map
/// draws it as a line from departure to arrival.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManualFlight {
    /// Any time on the day of the flight (UTC).
    pub date_ms: i64,
    /// Block out and in, minutes after midnight UTC.
    pub out_min: Option<i64>,
    pub in_min: Option<i64>,
    pub dep: crate::Place,
    pub arr: crate::Place,
    pub dep_at: Option<[f64; 2]>,
    pub arr_at: Option<[f64; 2]>,
    /// A hangar aircraft, or else a type and registration.
    pub aircraft_id: Option<i64>,
    pub aircraft_type: String,
    pub ident: String,
    pub callsign: String,
    pub total_min: i64,
    pub night_min: i64,
    pub ifr_min: i64,
    pub day_landings: i64,
    pub night_landings: i64,
    pub remarks: String,
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
                    COALESCE(NULLIF(a.icao_type, ''), a.title, ''), COALESCE(NULLIF(a.user_reg, ''), a.registration, ''),
                    COALESCE(NULLIF(l.class_override, ''), a.category_class, ''), COALESCE(a.engine_count, 0),
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
                remarks = COALESCE(?13, remarks),
                class_override = CASE WHEN ?14 IS NULL THEN class_override ELSE NULLIF(?14, '') END
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
                p.class_override.as_deref().map(|c| match c.trim().to_uppercase().as_str() {
                    c @ ("SEL" | "MEL" | "SES" | "MES") => c.to_string(),
                    _ => String::new(),
                }),
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

        for (at, lat, lon) in [(p.dep_at, "dep_lat", "dep_lon"), (p.arr_at, "arr_lat", "arr_lon")] {
            if let Some([la, lo]) = at {
                tx.execute(
                    &format!(
                        "UPDATE flight SET {lat} = ?2, {lon} = ?3 WHERE id = ?1
                         AND NOT EXISTS (SELECT 1 FROM track WHERE flight_id = ?1)"
                    ),
                    params![flight_id, la, lo],
                )?;
            }
        }

        // A different type or registration points this flight (only) at the
        // hangar aircraft with that registration (the registration is how
        // the hangar knows an aircraft), created if there is none.
        if p.aircraft_type.is_some() || p.ident.is_some() {
            // (title, type, display registration, engines, engine type, category/class)
            type Ac = (String, String, String, i64, String, String);
            let cur: Ac = match aircraft_id {
                Some(id) => tx.query_row(
                    "SELECT title, icao_type, COALESCE(NULLIF(user_reg, ''), registration), engine_count, engine_type, category_class
                     FROM aircraft WHERE id = ?1",
                    params![id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
                )?,
                None => (String::new(), String::new(), String::new(), 0, "unknown".into(), String::new()),
            };
            let ty = p.aircraft_type.clone().unwrap_or(cur.1.clone()).trim().to_uppercase();
            let reg = p.ident.clone().unwrap_or(cur.2.clone()).trim().to_uppercase();
            if !ty.eq_ignore_ascii_case(&cur.1) || !reg.eq_ignore_ascii_case(&cur.2) {
                let existing: Option<i64> = if reg.is_empty() {
                    tx.query_row(
                        "SELECT id FROM aircraft WHERE merged_into IS NULL AND icao_type = ?1
                           AND COALESCE(NULLIF(user_reg, ''), registration) = '' ORDER BY id LIMIT 1",
                        params![ty],
                        |r| r.get(0),
                    )
                    .optional()?
                } else {
                    crate::aircraft_by_reg(&tx, &reg)?
                };
                let new_id = match existing {
                    Some(id) => {
                        // Same airframe, new type code: correct it for the aircraft.
                        if p.aircraft_type.is_some() && !ty.is_empty() {
                            tx.execute("UPDATE aircraft SET icao_type = ?2 WHERE id = ?1", params![id, ty])?;
                        }
                        id
                    }
                    None => {
                        let title = if ty == cur.1 && !cur.0.is_empty() { cur.0.clone() } else { ty.clone() };
                        tx.execute(
                            "INSERT INTO aircraft (icao_type, title, registration, engine_count, engine_type, category_class)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                             ON CONFLICT(title, registration) DO UPDATE SET icao_type = excluded.icao_type, merged_into = NULL",
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
                tx.execute(crate::DELETE_ORPHAN_AIRCRAFT, [])?;
            }
        }
        tx.execute("UPDATE flight SET updated_at = ?2 WHERE id = ?1", params![flight_id, crate::now_ms()])?;
        tx.commit()?;
        Ok(true)
    }

    /// Every aircraft flown, most recently flown first.
    /// Mark an aircraft as a favorite (shown first in the hangar).
    pub fn set_aircraft_favorite(&self, id: i64, favorite: bool) -> Result<bool> {
        Ok(self.conn.execute("UPDATE aircraft SET favorite = ?2 WHERE id = ?1", params![id, favorite as i64])? > 0)
    }

    pub fn hangar(&self) -> Result<Vec<HangarRow>> {
        let mut st = self.conn.prepare(
            "SELECT a.id, a.title, a.icao_type, COALESCE(NULLIF(a.user_reg, ''), a.registration), a.engine_type,
                    a.engine_count, a.category_class, a.photo IS NOT NULL,
                    COUNT(f.id), COALESCE(SUM(l.total_min), 0), COALESCE(SUM(f.distance_nm), 0),
                    COALESCE(SUM(l.day_landings + l.night_landings), 0),
                    COALESCE(MIN(f.start_ms), 0), COALESCE(MAX(f.start_ms), 0), MAX(f.landing_rate_fpm),
                    AVG(f.landing_rate_fpm), a.favorite,
                    COALESCE((SELECT f2.callsign FROM flight f2 WHERE f2.aircraft_id = a.id ORDER BY f2.start_ms DESC LIMIT 1), ''),
                    NULLIF(a.user_model, ''), NULLIF(a.airline, ''), a.registration, a.manual
             FROM aircraft a
             LEFT JOIN flight f ON f.aircraft_id = a.id
             LEFT JOIN log_entry l ON l.flight_id = f.id
             WHERE a.merged_into IS NULL
             GROUP BY a.id
             ORDER BY a.favorite DESC, COALESCE(SUM(l.total_min), 0) DESC, MAX(f.start_ms) DESC, a.id DESC",
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
                    avg_landing_fpm: r.get(15)?,
                    favorite: r.get::<_, i64>(16)? != 0,
                    last_callsign: r.get(17)?,
                    last_icao: None,
                    last_name: None,
                    last_country: None,
                    last_region: None,
                    last_lat: None,
                    last_lon: None,
                    last_sim: String::new(),
                    model: r.get(18)?,
                    airline: r.get(19)?,
                    sim_registration: r.get(20)?,
                    manual: r.get::<_, i64>(21)? != 0,
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
            })
            .optional()?;
        }
        Ok(rows)
    }

    /// Change the pilot's details for an aircraft (registration, model name,
    /// airline). Statistics are never edited.
    pub fn update_aircraft(&self, id: i64, e: &AircraftEdit) -> Result<bool> {
        let clean = |v: &Option<String>| v.as_ref().map(|s| s.trim().to_string());
        let reg = clean(&e.registration).map(|s| s.to_uppercase());
        let n = self.conn.execute(
            "UPDATE aircraft SET
                user_reg = CASE WHEN ?2 IS NULL THEN user_reg WHEN ?2 = '' OR ?2 = registration THEN NULL ELSE ?2 END,
                user_model = CASE WHEN ?3 IS NULL THEN user_model ELSE NULLIF(?3, '') END,
                airline = CASE WHEN ?4 IS NULL THEN airline ELSE NULLIF(?4, '') END
             WHERE id = ?1",
            params![id, reg, clean(&e.model), clean(&e.airline)],
        )?;
        // A registration another aircraft already has: the two become one.
        crate::merge_duplicate_aircraft(&self.conn)?;
        Ok(n > 0)
    }

    /// Add an aircraft by hand. Its registration then matches flights the
    /// sim reports with the same registration.
    pub fn add_aircraft(&self, a: &NewAircraft) -> Result<i64> {
        let reg = a.registration.trim().to_uppercase();
        let model = a.model.trim();
        let ty = a.icao_type.trim().to_uppercase();
        let engine = match a.engine_type.as_str() {
            "piston" | "turboprop" | "jet" | "helo" | "glider" => a.engine_type.as_str(),
            _ => "unknown",
        };
        let class = crate::types::class_for(&ty, model, a.engine_count.clamp(0, 255) as u8, crate::types::engine_from_str(engine));
        let title = if model.is_empty() { ty.clone() } else { model.to_string() };
        self.conn.execute(
            "INSERT INTO aircraft (icao_type, title, registration, engine_count, engine_type, category_class, user_model, airline, manual)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULLIF(?7, ''), NULLIF(?8, ''), 1)",
            params![ty, title, reg, a.engine_count.max(0), engine, class, model, a.airline.as_deref().unwrap_or("").trim()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// The hangar aircraft with this registration, if any.
    pub fn aircraft_with_registration(&self, reg: &str) -> Result<Option<i64>> {
        crate::aircraft_by_reg(&self.conn, reg)
    }

    /// Delete an aircraft from the hangar. Its flights move to `move_to`
    /// when given; otherwise they are deleted with it (logbook lines,
    /// tracks and stamps too). Returns how many flights moved or went.
    pub fn delete_aircraft(&mut self, id: i64, move_to: Option<i64>) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let n = match move_to.filter(|m| *m != id) {
            Some(target) => {
                let n = tx.execute("UPDATE flight SET aircraft_id = ?2 WHERE aircraft_id = ?1", params![id, target])?;
                // The sim's names for it now find the other aircraft.
                tx.execute("UPDATE aircraft SET merged_into = ?2 WHERE merged_into = ?1", params![id, target])?;
                tx.execute(
                    "UPDATE aircraft SET merged_into = ?2, favorite = 0, photo = NULL, manual = 0,
                        user_reg = NULL, user_model = NULL, airline = NULL WHERE id = ?1",
                    params![id, target],
                )?;
                n
            }
            None => {
                let n = tx.execute("DELETE FROM flight WHERE aircraft_id = ?1", params![id])?;
                tx.execute("DELETE FROM aircraft WHERE id = ?1 OR merged_into = ?1", params![id])?;
                n
            }
        };
        tx.commit()?;
        Ok(n)
    }

    /// Log a flight entered by hand. Returns its id.
    pub fn add_manual_flight(&mut self, m: &ManualFlight) -> Result<i64> {
        const DAY: i64 = 86_400_000;
        let tx = self.conn.transaction()?;
        let day = m.date_ms.div_euclid(DAY) * DAY;
        let out = m.out_min.map(|v| day + v.clamp(0, 1439) * 60_000);
        let inn = m.in_min.map(|v| {
            let t = day + v.clamp(0, 1439) * 60_000;
            if out.is_some_and(|o| t < o) { t + DAY } else { t }
        });
        let total = match (out, inn) {
            (Some(o), Some(i)) if m.total_min <= 0 => (i - o) / 60_000,
            _ => m.total_min.max(0),
        };
        let start = out.unwrap_or(day + 12 * 3_600_000);
        let end = inn.unwrap_or(start + total * 60_000);

        let aircraft_id: Option<i64> = match m.aircraft_id {
            Some(id) => Some(id),
            None => {
                let ty = m.aircraft_type.trim().to_uppercase();
                let reg = m.ident.trim().to_uppercase();
                if ty.is_empty() && reg.is_empty() {
                    None
                } else {
                    let found: Option<i64> = if reg.is_empty() {
                        tx.query_row(
                            "SELECT id FROM aircraft WHERE merged_into IS NULL AND registration = '' AND icao_type = ?1
                             ORDER BY id LIMIT 1",
                            params![ty],
                            |r| r.get(0),
                        )
                        .optional()?
                    } else {
                        crate::aircraft_by_reg(&tx, &reg)?
                    };
                    match found {
                        Some(id) => Some(id),
                        None => {
                            tx.execute(
                                "INSERT INTO aircraft (icao_type, title, registration) VALUES (?1, ?1, ?2)
                                 ON CONFLICT(title, registration) DO NOTHING",
                                params![ty, reg],
                            )?;
                            tx.query_row(
                                "SELECT id FROM aircraft WHERE title = ?1 AND registration = ?2",
                                params![ty, reg],
                                |r| r.get(0),
                            )
                            .optional()?
                        }
                    }
                }
            }
        };
        let opt = |s: &str| (!s.trim().is_empty()).then(|| s.trim().to_string());
        let (dla, dlo) = m.dep_at.map(|[a, b]| (Some(a), Some(b))).unwrap_or((None, None));
        let (ala, alo) = m.arr_at.map(|[a, b]| (Some(a), Some(b))).unwrap_or((None, None));
        let distance = match (m.dep_at, m.arr_at) {
            (Some([a, b]), Some([c, d])) => telemetry::geo::distance_nm(a, b, c, d),
            _ => 0.0,
        };
        let now = crate::now_ms();
        tx.execute(
            "INSERT INTO flight (sim, aircraft_id, callsign, dep_icao, dep_name, dep_country, dep_region,
                arr_icao, arr_name, arr_country, arr_region, start_ms, end_ms, block_out, block_in,
                distance_nm, status, end_reason, created_at, updated_at, dep_lat, dep_lon, arr_lat, arr_lon, source)
             VALUES ('manual', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                     'complete', 'entered by hand', ?16, ?16, ?17, ?18, ?19, ?20, 'manual')",
            params![
                aircraft_id,
                m.callsign.trim().to_uppercase(),
                opt(&m.dep.icao),
                opt(&m.dep.name),
                opt(&m.dep.country),
                opt(&m.dep.region),
                opt(&m.arr.icao),
                opt(&m.arr.name),
                opt(&m.arr.country),
                opt(&m.arr.region),
                start,
                end,
                out,
                inn,
                distance,
                now,
                dla,
                dlo,
                ala,
                alo,
            ],
        )?;
        let id = tx.last_insert_rowid();
        let xc = if distance > 50.0 { total } else { 0 };
        tx.execute(
            "INSERT INTO log_entry (flight_id, total_min, night_min, ifr_min, xc_min, pic_min, day_landings, night_landings, remarks)
             VALUES (?1, ?2, ?3, ?4, ?5, ?2, ?6, ?7, ?8)",
            params![
                id,
                total,
                m.night_min.clamp(0, total),
                m.ifr_min.clamp(0, total),
                xc,
                m.day_landings.max(0),
                m.night_landings.max(0),
                m.remarks.trim()
            ],
        )?;
        tx.commit()?;
        Ok(id)
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
        self.conn.execute(crate::DELETE_ORPHAN_AIRCRAFT, [])?;
        Ok(n)
    }
}
