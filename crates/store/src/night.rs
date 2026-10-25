//! Logbook night time from the track and the sun's position.

use telemetry::{sun, TrackPoint};

/// Gaps longer than this between track points are pauses, not flying.
const MAX_GAP_MS: i64 = 120_000;

/// Minutes of the track between `from` and `to` flown at night.
pub fn night_minutes(track: &[TrackPoint], from: Option<i64>, to: Option<i64>) -> i64 {
    let (from, to) = (from.unwrap_or(i64::MIN), to.unwrap_or(i64::MAX));
    let mut ms = 0i64;
    for w in track.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let (t0, t1) = (a.t_ms.max(from), b.t_ms.min(to));
        if t1 <= t0 || b.t_ms - a.t_ms > MAX_GAP_MS {
            continue;
        }
        let mid = (t0 + t1) / 2;
        if sun::is_night(mid, (a.lat + b.lat) / 2.0, (a.lon + b.lon) / 2.0) {
            ms += t1 - t0;
        }
    }
    (ms as f64 / 60_000.0).round() as i64
}

/// (day, night) landing counts.
pub fn landing_split(landings: impl IntoIterator<Item = (i64, f64, f64)>) -> (i64, i64) {
    landings.into_iter().fold((0, 0), |(d, n), (t, lat, lon)| {
        if sun::is_night(t, lat, lon) {
            (d, n + 1)
        } else {
            (d + 1, n)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(t_ms: i64) -> TrackPoint {
        TrackPoint { t_ms, lat: 51.47, lon: -0.46, ..Default::default() }
    }

    // 21 Jun 2026 00:00 UTC: London is dark.
    const MIDNIGHT: i64 = 1_782_000_000_000;
    // 21 Jun 2026 12:00 UTC: broad daylight.
    const NOON: i64 = 1_782_043_200_000;

    #[test]
    fn counts_only_dark_minutes() {
        let night: Vec<_> = (0..=30).map(|i| pt(MIDNIGHT + i * 60_000)).collect();
        assert_eq!(night_minutes(&night, None, None), 30);
        let day: Vec<_> = (0..=30).map(|i| pt(NOON + i * 60_000)).collect();
        assert_eq!(night_minutes(&day, None, None), 0);
    }

    #[test]
    fn pauses_and_limits_are_respected() {
        let mut track: Vec<_> = (0..=10).map(|i| pt(MIDNIGHT + i * 60_000)).collect();
        track.push(pt(MIDNIGHT + 3_600_000)); // a 50-minute pause
        assert_eq!(night_minutes(&track, None, None), 10);
        assert_eq!(night_minutes(&track, Some(MIDNIGHT + 5 * 60_000), None), 5);
    }

    #[test]
    fn landings_split_by_light() {
        let (d, n) = landing_split([(NOON, 51.47, -0.46), (MIDNIGHT, 51.47, -0.46), (MIDNIGHT, 51.47, -0.46)]);
        assert_eq!((d, n), (1, 2));
    }
}
