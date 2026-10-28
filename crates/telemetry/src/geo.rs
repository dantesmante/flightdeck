//! Small great-circle helpers (spherical earth, good to ~0.5%).

use crate::units::EARTH_RADIUS_NM;

/// Great-circle distance in nautical miles.
pub fn distance_nm(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_NM * a.sqrt().min(1.0).asin()
}

/// Initial true bearing from point 1 to point 2, degrees 0..360.
pub fn bearing_deg(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dl = (lon2 - lon1).to_radians();
    let y = dl.sin() * p2.cos();
    let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
    (y.atan2(x).to_degrees() + 360.0) % 360.0
}

/// Point reached by traveling `dist_nm` from a start point on `brg_deg`.
pub fn destination(lat: f64, lon: f64, brg_deg: f64, dist_nm: f64) -> (f64, f64) {
    let d = dist_nm / EARTH_RADIUS_NM;
    let (p1, l1, b) = (lat.to_radians(), lon.to_radians(), brg_deg.to_radians());
    let p2 = (p1.sin() * d.cos() + p1.cos() * d.sin() * b.cos()).asin();
    let l2 = l1 + (b.sin() * d.sin() * p1.cos()).atan2(d.cos() - p1.sin() * p2.sin());
    (p2.to_degrees(), ((l2.to_degrees() + 540.0) % 360.0) - 180.0)
}

/// Smallest signed difference between two headings, degrees -180..180.
pub fn heading_diff(a: f64, b: f64) -> f64 {
    ((b - a + 540.0) % 360.0) - 180.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_distance() {
        // KJFK to EGLL is about 2,999 nm.
        let d = distance_nm(40.6398, -73.7789, 51.4706, -0.4619);
        assert!((d - 2999.0).abs() < 15.0, "{d}");
    }

    #[test]
    fn destination_round_trip() {
        let (lat, lon) = destination(37.6188, -122.375, 135.0, 100.0);
        let d = distance_nm(37.6188, -122.375, lat, lon);
        assert!((d - 100.0).abs() < 0.01);
        let b = bearing_deg(37.6188, -122.375, lat, lon);
        assert!((b - 135.0).abs() < 0.5);
    }

    #[test]
    fn heading_wraps() {
        assert_eq!(heading_diff(350.0, 10.0), 20.0);
        assert_eq!(heading_diff(10.0, 350.0), -20.0);
    }
}
