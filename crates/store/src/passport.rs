//! Passport stamps, derived from the flight history.
//!
//! Rule (decided with the project owner): the passport is for international
//! arrivals only. A flight that lands in a different country from the one it
//! left earns one arrival stamp for the country entered; the first ever
//! landing in a country gets the special first-entry design. Departures and
//! domestic flights earn no stamps, and only landings count as visiting a
//! country: starting a flight somewhere (or spawning there) does not.
//!
//! Stamps are recomputed from flights, so editing or deleting a flight
//! always leaves the passport consistent.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StampPlace {
    pub icao: String,
    pub name: String,
    pub country: String,
    pub region: String,
}

/// The facts about one flight that stamps need.
#[derive(Debug, Clone, Default)]
pub struct StampFlight {
    pub flight_id: i64,
    pub dep_ms: i64,
    pub arr_ms: i64,
    pub dep: StampPlace,
    pub arr: StampPlace,
    pub aircraft_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StampKind {
    Arrival,
    FirstEntry,
}

impl StampKind {
    fn as_str(&self) -> &'static str {
        match self {
            StampKind::Arrival => "arrival",
            StampKind::FirstEntry => "first_entry",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stamp {
    pub flight_id: i64,
    pub kind: StampKind,
    pub country: String,
    pub region: String,
    pub icao: String,
    pub airport_name: String,
    pub t_ms: i64,
    pub aircraft_type: String,
    /// Stable per stamp, so its shape, ink and tilt never change.
    pub seed: u32,
}

/// FNV-1a: tiny, stable across versions and platforms.
fn seed(flight_id: i64, kind: StampKind) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in flight_id.to_le_bytes().iter().chain(kind.as_str().as_bytes()) {
        h ^= *b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

fn stamp(f: &StampFlight, kind: StampKind, place: &StampPlace, t_ms: i64) -> Stamp {
    Stamp {
        flight_id: f.flight_id,
        kind,
        country: place.country.clone(),
        region: place.region.clone(),
        icao: place.icao.clone(),
        airport_name: place.name.clone(),
        t_ms,
        aircraft_type: f.aircraft_type.clone(),
        seed: seed(f.flight_id, kind),
    }
}

/// Stamps for `flights`, which must be in chronological order.
pub fn derive_stamps(flights: &[StampFlight]) -> Vec<Stamp> {
    let mut countries: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for f in flights {
        if f.dep.country.is_empty() || f.arr.country.is_empty() {
            continue;
        }
        if f.dep.country != f.arr.country {
            let kind = if countries.contains(&f.arr.country) { StampKind::Arrival } else { StampKind::FirstEntry };
            out.push(stamp(f, kind, &f.arr, f.arr_ms));
        }
        countries.insert(f.arr.country.clone());
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryVisit {
    pub country: String,
    pub first_ms: i64,
    /// Landings in this country (international or domestic).
    pub arrivals: i64,
}

/// Countries landed in, in the order of the first landing. Departures do
/// not count, so spawning somewhere never marks it as visited.
pub fn country_visits(flights: &[StampFlight]) -> Vec<CountryVisit> {
    let mut out: Vec<CountryVisit> = Vec::new();
    for f in flights {
        let p = &f.arr;
        if p.country.is_empty() {
            continue;
        }
        match out.iter_mut().find(|v| v.country == p.country) {
            Some(v) => v.arrivals += 1,
            None => out.push(CountryVisit { country: p.country.clone(), first_ms: f.arr_ms, arrivals: 1 }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(icao: &str, country: &str, region: &str) -> StampPlace {
        StampPlace { icao: icao.into(), name: format!("{icao} airport"), country: country.into(), region: region.into() }
    }

    fn flight(id: i64, dep: StampPlace, arr: StampPlace) -> StampFlight {
        StampFlight { flight_id: id, dep_ms: id * 1000, arr_ms: id * 1000 + 500, dep, arr, aircraft_type: "C172".into() }
    }

    fn kinds(s: &[Stamp]) -> Vec<(StampKind, &str)> {
        s.iter().map(|s| (s.kind, s.icao.as_str())).collect()
    }

    #[test]
    fn only_arrivals_are_stamped() {
        let kord = || place("KORD", "US", "US-IL");
        let cyyz = || place("CYYZ", "CA", "CA-ON");
        let s = derive_stamps(&[flight(1, kord(), cyyz()), flight(2, cyyz(), kord()), flight(3, kord(), cyyz())]);
        assert_eq!(
            kinds(&s),
            vec![(StampKind::FirstEntry, "CYYZ"), (StampKind::FirstEntry, "KORD"), (StampKind::Arrival, "CYYZ")]
        );
    }

    #[test]
    fn domestic_flights_earn_no_stamps() {
        let kc81 = || place("KC81", "US", "US-IL");
        let kmke = || place("KMKE", "US", "US-WI");
        assert!(derive_stamps(&[flight(1, kc81(), kc81()), flight(2, kc81(), kmke())]).is_empty());
        // ...but a later trip abroad still gets its first-entry seal.
        let s = derive_stamps(&[flight(1, kc81(), kmke()), flight(2, kmke(), place("CYYZ", "CA", "CA-ON"))]);
        assert_eq!(kinds(&s), vec![(StampKind::FirstEntry, "CYYZ")]);
    }

    #[test]
    fn seeds_are_stable_and_distinct() {
        assert_eq!(seed(7, StampKind::Arrival), seed(7, StampKind::Arrival));
        assert_ne!(seed(7, StampKind::Arrival), seed(7, StampKind::FirstEntry));
        assert_ne!(seed(7, StampKind::Arrival), seed(8, StampKind::Arrival));
    }

    #[test]
    fn only_landings_count_as_visits() {
        // Spawned at Heathrow, landed in Paris: France is visited, the UK is not.
        let v = country_visits(&[flight(1, place("EGLL", "GB", ""), place("LFPG", "FR", ""))]);
        let got: Vec<_> = v.iter().map(|v| (v.country.as_str(), v.arrivals)).collect();
        assert_eq!(got, vec![("FR", 1)]);
        let v = country_visits(&[
            flight(1, place("KORD", "US", ""), place("CYYZ", "CA", "")),
            flight(2, place("CYYZ", "CA", ""), place("KORD", "US", "")),
            flight(3, place("KORD", "US", ""), place("CYYZ", "CA", "")),
        ]);
        let got: Vec<_> = v.iter().map(|v| (v.country.as_str(), v.arrivals)).collect();
        assert_eq!(got, vec![("CA", 2), ("US", 1)]);
    }

    #[test]
    fn first_landing_home_is_a_first_entry() {
        // Leaving the US first does not count as having landed there.
        let kord = || place("KORD", "US", "US-IL");
        let cyyz = || place("CYYZ", "CA", "CA-ON");
        let s = derive_stamps(&[flight(1, kord(), cyyz()), flight(2, cyyz(), kord())]);
        assert_eq!(kinds(&s), vec![(StampKind::FirstEntry, "CYYZ"), (StampKind::FirstEntry, "KORD")]);
    }
}
