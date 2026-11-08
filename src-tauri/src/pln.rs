//! Flight plans for the simulator: a route drawn on FlightDeck's map, written
//! as a Microsoft Flight Simulator .PLN file (the AceXML flight plan both
//! MSFS 2020 and MSFS 2024 load from their world map, and that Little
//! Navmap and SimBrief also write).
//!
//! A drawn route is the departure airport, the points clicked on the way
//! (user waypoints, or airports passed over), and the destination airport.

use serde::Deserialize;

/// A point of the route.
#[derive(Debug, Clone, Deserialize)]
pub struct Point {
    /// The airport's code, or for a user waypoint the name to show ("WP1").
    pub ident: String,
    #[serde(default)]
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    /// An airport (else a user waypoint).
    #[serde(default)]
    pub airport: bool,
    /// The altitude to cross it at, feet (user waypoints; else the cruise).
    #[serde(default)]
    pub alt_ft: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    /// "VFR" or "IFR".
    pub rules: String,
    /// Cruising altitude, feet.
    pub cruise_ft: f64,
    pub origin: Point,
    pub destination: Point,
    #[serde(default)]
    pub waypoints: Vec<Point>,
}

/// A position as the sim writes it: N47° 26' 56.04",W122° 18' 32.04",+000433.00
pub fn world_position(lat: f64, lon: f64, alt_ft: f64) -> String {
    fn dms(v: f64, pos: char, neg: char) -> String {
        let h = if v < 0.0 { neg } else { pos };
        // Work in hundredths of a second so rounding never gives 60".
        let total = (v.abs() * 360_000.0).round() as i64;
        let (d, rest) = (total / 360_000, total % 360_000);
        let (m, s) = (rest / 6_000, rest % 6_000);
        format!("{h}{d}° {m}' {}.{:02}\"", s / 100, s % 100)
    }
    let sign = if alt_ft < 0.0 { '-' } else { '+' };
    format!("{},{},{sign}{:09.2}", dms(lat, 'N', 'S'), dms(lon, 'E', 'W'), alt_ft.abs())
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// Waypoint names the sim accepts: letters and digits, up to 10.
fn clean_ident(s: &str, n: usize) -> String {
    let v: String = s.chars().filter(|c| c.is_ascii_alphanumeric()).take(10).collect::<String>().to_ascii_uppercase();
    if v.is_empty() { format!("WP{n}") } else { v }
}

/// The .PLN text. `elevation` gives an airport's field elevation in feet.
pub fn write(r: &Route, elevation: impl Fn(&str) -> Option<f64>) -> String {
    let ifr = r.rules.eq_ignore_ascii_case("IFR");
    let cruise = r.cruise_ft.clamp(500.0, 60_000.0).round();
    let dep = r.origin.ident.trim().to_ascii_uppercase();
    let arr = r.destination.ident.trim().to_ascii_uppercase();
    let dep_alt = elevation(&dep).unwrap_or(0.0);
    let arr_alt = elevation(&arr).unwrap_or(0.0);
    let mut o = String::new();
    o.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    o.push_str("<SimBase.Document Type=\"AceXML\" version=\"1,0\">\n");
    o.push_str("    <Descr>AceXML Document</Descr>\n");
    o.push_str("    <FlightPlan.FlightPlan>\n");
    o.push_str(&format!("        <Title>{} to {}</Title>\n", esc(&dep), esc(&arr)));
    o.push_str(&format!("        <FPType>{}</FPType>\n", if ifr { "IFR" } else { "VFR" }));
    if ifr {
        o.push_str(&format!("        <RouteType>{}</RouteType>\n", if cruise >= 18_000.0 { "HighAlt" } else { "LowAlt" }));
    }
    o.push_str(&format!("        <CruisingAlt>{cruise:.3}</CruisingAlt>\n"));
    o.push_str(&format!("        <DepartureID>{}</DepartureID>\n", esc(&dep)));
    o.push_str(&format!("        <DepartureLLA>{}</DepartureLLA>\n", world_position(r.origin.lat, r.origin.lon, dep_alt)));
    o.push_str(&format!("        <DestinationID>{}</DestinationID>\n", esc(&arr)));
    o.push_str(&format!("        <DestinationLLA>{}</DestinationLLA>\n", world_position(r.destination.lat, r.destination.lon, arr_alt)));
    o.push_str(&format!("        <Descr>{}, {}</Descr>\n", esc(&dep), esc(&arr)));
    if !r.origin.name.is_empty() {
        o.push_str(&format!("        <DepartureName>{}</DepartureName>\n", esc(&r.origin.name)));
    }
    if !r.destination.name.is_empty() {
        o.push_str(&format!("        <DestinationName>{}</DestinationName>\n", esc(&r.destination.name)));
    }
    o.push_str("        <AppVersion>\n            <AppVersionMajor>11</AppVersionMajor>\n            <AppVersionBuild>282174</AppVersionBuild>\n        </AppVersion>\n");
    let airport = |o: &mut String, id: &str, lat: f64, lon: f64, alt: f64| {
        o.push_str(&format!("        <ATCWaypoint id=\"{}\">\n", esc(id)));
        o.push_str("            <ATCWaypointType>Airport</ATCWaypointType>\n");
        o.push_str(&format!("            <WorldPosition>{}</WorldPosition>\n", world_position(lat, lon, alt)));
        o.push_str(&format!("            <ICAO>\n                <ICAOIdent>{}</ICAOIdent>\n            </ICAO>\n", esc(id)));
        o.push_str("        </ATCWaypoint>\n");
    };
    airport(&mut o, &dep, r.origin.lat, r.origin.lon, dep_alt);
    let mut used = std::collections::HashSet::new();
    let mut user_n = 0;
    for w in &r.waypoints {
        if w.airport {
            let id = w.ident.trim().to_ascii_uppercase();
            let alt = elevation(&id).unwrap_or(cruise);
            airport(&mut o, &id, w.lat, w.lon, alt);
            continue;
        }
        user_n += 1;
        // Each user waypoint needs its own name.
        let mut id = clean_ident(&w.ident, user_n);
        while !used.insert(id.clone()) {
            user_n += 1;
            id = format!("WP{user_n}");
        }
        o.push_str(&format!("        <ATCWaypoint id=\"{}\">\n", esc(&id)));
        o.push_str("            <ATCWaypointType>User</ATCWaypointType>\n");
        let alt = w.alt_ft.filter(|a| a.is_finite() && *a > 0.0).unwrap_or(cruise);
        o.push_str(&format!("            <WorldPosition>{}</WorldPosition>\n", world_position(w.lat, w.lon, alt)));
        o.push_str("        </ATCWaypoint>\n");
    }
    airport(&mut o, &arr, r.destination.lat, r.destination.lon, arr_alt);
    o.push_str("    </FlightPlan.FlightPlan>\n");
    o.push_str("</SimBase.Document>\n");
    o
}

/// The file name offered when saving: "KSEA-KPDX.pln".
pub fn file_name(r: &Route) -> String {
    let c = |s: &str| s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_uppercase();
    format!("{}-{}.pln", c(&r.origin.ident), c(&r.destination.ident))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route() -> Route {
        let p = |ident: &str, lat: f64, lon: f64, airport: bool| Point { ident: ident.into(), name: String::new(), lat, lon, airport, alt_ft: None };
        Route {
            rules: "VFR".into(),
            cruise_ft: 5500.0,
            origin: Point { name: "Seattle-Tacoma International Airport".into(), ..p("KSEA", 47.449, -122.309, true) },
            destination: Point { name: "Portland International Airport".into(), ..p("KPDX", 45.5887, -122.5975, true) },
            waypoints: vec![p("WP1", 46.9, -122.7, false), p("", 46.2, -122.9, false), p("KTDO", 46.4772, -122.8064, true)],
        }
    }

    #[test]
    fn positions_read_like_the_sims() {
        assert_eq!(world_position(47.449, -122.309, 433.0), "N47° 26' 56.40\",W122° 18' 32.40\",+000433.00");
        assert_eq!(world_position(-33.9461, 151.1772, 21.0), "S33° 56' 45.96\",E151° 10' 37.92\",+000021.00");
        // Never 60 seconds.
        assert_eq!(world_position(10.0 - 1e-9, 0.0, 0.0), "N10° 0' 0.00\",E0° 0' 0.00\",+000000.00");
    }

    #[test]
    fn a_waypoint_can_have_its_own_altitude() {
        let mut r = route();
        r.waypoints[0].alt_ft = Some(3500.0);
        let txt = write(&r, |_| None);
        assert!(txt.contains("<ATCWaypoint id=\"WP1\">\n            <ATCWaypointType>User</ATCWaypointType>\n            <WorldPosition>N46° 54' 0.00\",W122° 42' 0.00\",+003500.00</WorldPosition>"), "{txt}");
        // The others stay at the cruise.
        assert!(txt.contains("W122° 54' 0.00\",+005500.00"), "{txt}");
    }

    #[test]
    fn writes_a_flight_plan_msfs_loads() {
        let txt = write(&route(), |id| match id {
            "KSEA" => Some(433.0),
            "KPDX" => Some(31.0),
            _ => None,
        });
        assert!(txt.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<SimBase.Document Type=\"AceXML\" version=\"1,0\">"));
        for want in [
            "<FPType>VFR</FPType>",
            "<CruisingAlt>5500.000</CruisingAlt>",
            "<DepartureID>KSEA</DepartureID>",
            "<DestinationID>KPDX</DestinationID>",
            "<DepartureLLA>N47° 26' 56.40\",W122° 18' 32.40\",+000433.00</DepartureLLA>",
            "<ATCWaypoint id=\"WP1\">\n            <ATCWaypointType>User</ATCWaypointType>",
            "<ATCWaypoint id=\"WP2\">",
            "<ATCWaypoint id=\"KTDO\">\n            <ATCWaypointType>Airport</ATCWaypointType>",
            "<ICAOIdent>KPDX</ICAOIdent>",
        ] {
            assert!(txt.contains(want), "missing {want}\n{txt}");
        }
        assert!(!txt.contains("RouteType"), "VFR plans carry no route type");
        // Departure first, destination last, in the order drawn.
        let order: Vec<usize> = ["id=\"KSEA\"", "id=\"WP1\"", "id=\"WP2\"", "id=\"KTDO\"", "id=\"KPDX\""].iter().map(|k| txt.find(k).unwrap()).collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]));
        // Well formed: every element closed.
        assert_eq!(txt.matches("<ATCWaypoint ").count(), txt.matches("</ATCWaypoint>").count());
        assert_eq!(file_name(&route()), "KSEA-KPDX.pln");
    }

    #[test]
    fn ifr_high_routes_say_so() {
        let mut r = route();
        r.rules = "IFR".into();
        r.cruise_ft = 35_000.0;
        assert!(write(&r, |_| None).contains("<RouteType>HighAlt</RouteType>"));
    }
}
