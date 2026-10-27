// The day/night line: where the sun is setting or rising right now, and the
// night side of the Earth as a polygon. Pure maths, no network.

const RAD = Math.PI / 180;

/** The sun's declination and right ascension (degrees) and Greenwich mean
 * sidereal time (degrees) at a moment. Low-precision formulas from the
 * Astronomical Almanac, good to a small fraction of a degree. */
function sun(ms: number): { dec: number; ra: number; gmst: number } {
  const jd = ms / 86400000 + 2440587.5;
  const n = jd - 2451545.0;
  const L = (280.46 + 0.9856474 * n) % 360;
  const g = ((357.528 + 0.9856003 * n) % 360) * RAD;
  const lambda = (L + 1.915 * Math.sin(g) + 0.02 * Math.sin(2 * g)) * RAD;
  const eps = (23.439 - 0.0000004 * n) * RAD;
  const ra = Math.atan2(Math.cos(eps) * Math.sin(lambda), Math.cos(lambda)) / RAD;
  const dec = Math.asin(Math.sin(eps) * Math.sin(lambda)) / RAD;
  const gmst = (280.46061837 + 360.98564736629 * n) % 360;
  return { dec, ra, gmst };
}

/** The night side as one polygon, and the terminator as a line. */
export function nightShade(ms = Date.now()): GeoJSON.FeatureCollection {
  const { dec, ra, gmst } = sun(ms);
  const line: [number, number][] = [];
  for (let lon = -180; lon <= 180; lon += 1) {
    const ha = (gmst + lon - ra) * RAD;
    // Latitude where the sun is on the horizon at this longitude.
    let lat = Math.atan(-Math.cos(ha) / Math.tan(dec * RAD)) / RAD;
    lat = Math.max(-85, Math.min(85, lat));
    line.push([lon, lat]);
  }
  // Night covers the pole that faces away from the sun.
  const pole = dec > 0 ? -85 : 85;
  const ring: [number, number][] = [...line, [180, pole], [-180, pole], line[0]];
  return {
    type: "FeatureCollection",
    features: [
      { type: "Feature", properties: { kind: "night" }, geometry: { type: "Polygon", coordinates: [ring] } },
      { type: "Feature", properties: { kind: "edge" }, geometry: { type: "LineString", coordinates: line } },
    ],
  };
}
