//! Track storage: simplify, delta-encode, compress.
//!
//! A 2-hour flight arrives as ~7,000 points (1 Hz plus high-rate touchdown
//! windows). Ramer–Douglas–Peucker at 15 m drops most cruise points, the rest
//! are stored as zig-zag varint deltas and compressed with zstd.

use telemetry::TrackPoint;

const MAGIC: &[u8; 4] = b"FDT1";

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("not a FlightDeck track blob")]
    BadMagic,
    #[error("track blob is truncated")]
    Truncated,
    #[error("compression: {0}")]
    Zstd(#[from] std::io::Error),
}

/// Simplify a track with Ramer–Douglas–Peucker in 3D (meters), never dropping
/// points for which `protect` returns true.
pub fn simplify(points: &[TrackPoint], epsilon_m: f64, protect: impl Fn(&TrackPoint) -> bool) -> Vec<TrackPoint> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let lat0 = points[0].lat.to_radians();
    let xyz: Vec<[f64; 3]> = points
        .iter()
        .map(|p| [p.lon * 111_320.0 * lat0.cos(), p.lat * 110_540.0, p.alt_ft as f64 * 0.3048])
        .collect();
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    for (i, p) in points.iter().enumerate() {
        if protect(p) || (i > 0 && points[i - 1].on_ground != p.on_ground) {
            keep[i] = true;
        }
    }
    // Run RDP between each pair of consecutive kept points (iterative).
    let anchors: Vec<usize> = (0..points.len()).filter(|&i| keep[i]).collect();
    let mut stack: Vec<(usize, usize)> = anchors.windows(2).map(|w| (w[0], w[1])).collect();
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (mut best, mut best_d) = (0, -1.0);
        for i in a + 1..b {
            let d = dist_to_segment(xyz[i], xyz[a], xyz[b]);
            if d > best_d {
                best = i;
                best_d = d;
            }
        }
        if best_d > epsilon_m {
            keep[best] = true;
            stack.push((a, best));
            stack.push((best, b));
        }
    }
    points.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

fn dist_to_segment(p: [f64; 3], a: [f64; 3], b: [f64; 3]) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let len2 = ab.iter().map(|v| v * v).sum::<f64>();
    let t = if len2 == 0.0 { 0.0 } else { (ab[0] * ap[0] + ab[1] * ap[1] + ab[2] * ap[2]) / len2 };
    let t = t.clamp(0.0, 1.0);
    let c = [a[0] + ab[0] * t - p[0], a[1] + ab[1] * t - p[1], a[2] + ab[2] * t - p[2]];
    c.iter().map(|v| v * v).sum::<f64>().sqrt()
}

fn put_varint(out: &mut Vec<u8>, v: i64) {
    let mut z = ((v << 1) ^ (v >> 63)) as u64;
    loop {
        let byte = (z & 0x7f) as u8;
        z >>= 7;
        if z == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn get_varint(buf: &[u8], pos: &mut usize) -> Result<i64, CodecError> {
    let (mut z, mut shift) = (0u64, 0);
    loop {
        let byte = *buf.get(*pos).ok_or(CodecError::Truncated)?;
        *pos += 1;
        z |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok(((z >> 1) as i64) ^ -((z & 1) as i64));
        }
        shift += 7;
        if shift > 63 {
            return Err(CodecError::Truncated);
        }
    }
}

/// Quantised fields: 1e-7 degrees (~1 cm), 1 ft, 0.1 kt, 1 fpm, 0.1 degree.
fn quantize(p: &TrackPoint) -> [i64; 7] {
    [
        p.t_ms,
        (p.lat * 1e7).round() as i64,
        (p.lon * 1e7).round() as i64,
        p.alt_ft.round() as i64,
        (p.gs_kt * 10.0).round() as i64,
        p.vs_fpm.round() as i64,
        (p.hdg_deg.rem_euclid(360.0) * 10.0).round() as i64,
    ]
}

pub fn encode(points: &[TrackPoint]) -> Result<Vec<u8>, CodecError> {
    let mut raw = Vec::with_capacity(points.len() * 12 + 8);
    raw.extend_from_slice(MAGIC);
    raw.extend_from_slice(&(points.len() as u32).to_le_bytes());
    let mut prev = [0i64; 7];
    for p in points {
        let q = quantize(p);
        for i in 0..7 {
            put_varint(&mut raw, q[i] - prev[i]);
        }
        raw.push(p.on_ground as u8);
        prev = q;
    }
    Ok(zstd::encode_all(raw.as_slice(), 3)?)
}

pub fn decode(blob: &[u8]) -> Result<Vec<TrackPoint>, CodecError> {
    let raw = zstd::decode_all(blob)?;
    if raw.len() < 8 || &raw[..4] != MAGIC {
        return Err(CodecError::BadMagic);
    }
    let n = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
    let mut pos = 8;
    let mut q = [0i64; 7];
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        for v in q.iter_mut() {
            *v += get_varint(&raw, &mut pos)?;
        }
        let flags = *raw.get(pos).ok_or(CodecError::Truncated)?;
        pos += 1;
        out.push(TrackPoint {
            t_ms: q[0],
            lat: q[1] as f64 / 1e7,
            lon: q[2] as f64 / 1e7,
            alt_ft: q[3] as f32,
            gs_kt: q[4] as f32 / 10.0,
            vs_fpm: q[5] as f32,
            hdg_deg: q[6] as f32 / 10.0,
            on_ground: flags & 1 == 1,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(n: usize) -> Vec<TrackPoint> {
        (0..n)
            .map(|i| TrackPoint {
                t_ms: 1_760_000_000_000 + i as i64 * 1000,
                lat: 47.0 + i as f64 * 0.001,
                lon: -122.0,
                alt_ft: 10_000.0,
                gs_kt: 250.0,
                vs_fpm: 0.0,
                hdg_deg: 359.9,
                on_ground: false,
            })
            .collect()
    }

    #[test]
    fn round_trip_is_lossless_at_storage_precision() {
        let mut pts = line(500);
        pts[10].on_ground = true;
        pts[20].vs_fpm = -123.4;
        let back = decode(&encode(&pts).unwrap()).unwrap();
        assert_eq!(back.len(), pts.len());
        for (a, b) in pts.iter().zip(&back) {
            assert_eq!(a.t_ms, b.t_ms);
            assert!((a.lat - b.lat).abs() < 1e-7 && (a.lon - b.lon).abs() < 1e-7);
            assert!((a.hdg_deg - b.hdg_deg).abs() < 0.06);
            assert_eq!(a.on_ground, b.on_ground);
        }
        assert_eq!(back[20].vs_fpm, -123.0);
    }

    #[test]
    fn straight_line_collapses_to_its_ends() {
        let s = simplify(&line(1000), 15.0, |_| false);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn protected_points_survive() {
        let pts = line(1000);
        let s = simplify(&pts, 15.0, |p| p.t_ms == pts[500].t_ms);
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn a_turn_is_kept() {
        let mut pts = line(100);
        for (i, p) in pts.iter_mut().enumerate().skip(50) {
            p.lat = 47.05;
            p.lon = -122.0 + (i - 50) as f64 * 0.001;
        }
        let s = simplify(&pts, 15.0, |_| false);
        assert!(s.len() >= 3 && s.len() < 10, "{}", s.len());
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(decode(b"not zstd").is_err());
    }
}
