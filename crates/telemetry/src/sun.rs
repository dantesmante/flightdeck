//! Sun position, accurate to about 0.1 degree: enough to tell day from night
//! for logbook night time.

/// Solar elevation above the horizon in degrees at `t_ms` (Unix ms, UTC).
pub fn elevation_deg(t_ms: i64, lat: f64, lon: f64) -> f64 {
    let d = t_ms as f64 / 86_400_000.0 + 2_440_587.5 - 2_451_545.0; // days since J2000
    let g = (357.529 + 0.985_600_28 * d).to_radians();
    let q = 280.459 + 0.985_647_36 * d;
    let l = (q + 1.915 * g.sin() + 0.020 * (2.0 * g).sin()).to_radians();
    let e = (23.439 - 0.000_000_36 * d).to_radians();
    let ra = (e.cos() * l.sin()).atan2(l.cos()).to_degrees();
    let dec = (e.sin() * l.sin()).asin();
    let gmst = (18.697_374_558 + 24.065_709_824_419_08 * d).rem_euclid(24.0);
    let ha = (gmst * 15.0 + lon - ra).to_radians();
    let lat = lat.to_radians();
    (lat.sin() * dec.sin() + lat.cos() * dec.cos() * ha.cos()).asin().to_degrees()
}

/// Night for the logbook: the sun more than 6 degrees below the horizon
/// (between the end of evening and the start of morning civil twilight).
pub fn is_night(t_ms: i64, lat: f64, lon: f64) -> bool {
    elevation_deg(t_ms, lat, lon) < -6.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const JUNE_21_2026_NOON_UTC: i64 = 1_782_043_200_000;

    #[test]
    fn overhead_on_the_tropic_at_the_solstice() {
        let e = elevation_deg(JUNE_21_2026_NOON_UTC, 23.44, 0.0);
        assert!(e > 88.5, "{e}");
    }

    #[test]
    fn midnight_is_dark_and_noon_is_light() {
        // London, midnight and noon UTC on the same day.
        assert!(is_night(JUNE_21_2026_NOON_UTC - 12 * 3_600_000, 51.47, -0.46));
        assert!(!is_night(JUNE_21_2026_NOON_UTC, 51.47, -0.46));
    }

    #[test]
    fn midnight_sun_in_the_arctic() {
        // Tromsø at local midnight in June: the sun never sets.
        assert!(elevation_deg(JUNE_21_2026_NOON_UTC - 12 * 3_600_000, 69.65, 18.96) > 0.0);
    }

    #[test]
    fn chicago_evening_in_late_october() {
        // Sunset is about 17:59 CDT and civil twilight ends about 18:27 CDT.
        // 23:00 UTC = 18:00 CDT: twilight, not night. 01:00 UTC = 20:00 CDT: night.
        let six_pm = 1_792_969_200_000;
        assert!(!is_night(six_pm, 41.98, -87.90));
        assert!(is_night(six_pm + 120 * 60_000, 41.98, -87.90));
        let e = elevation_deg(six_pm, 41.98, -87.90);
        assert!((-3.0..0.5).contains(&e), "{e}");
    }
}
