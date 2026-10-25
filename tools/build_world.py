#!/usr/bin/env python3
"""Build the passport's world map from Natural Earth (public domain).

Download ne_50m_admin_0_countries.geojson from
https://github.com/nvkelso/natural-earth-vector/tree/master/geojson
then run:

    python3 tools/build_world.py ne_50m_admin_0_countries.geojson

It writes ui/src/lib/passport/world.json: one SVG path per country (keyed by
ISO 3166 alpha-2, the same codes OurAirports uses), projected with the Equal
Earth projection into a 1000-unit-wide frame and simplified to keep the file
small.
"""

import json
import math
import sys
from pathlib import Path

W = 1000.0
TOLERANCE = 0.75  # simplification tolerance, in map units (about half a pixel in the passport)
TINY_AREA = 6.0   # countries smaller than this also get a dot so they show

A1, A2, A3, A4 = 1.340264, -0.081106, 0.000893, 0.003796
M = math.sqrt(3) / 2
X_MAX = 2.7064737
Y_MAX = 1.3173627
SCALE = W / (2 * X_MAX)
H = round(2 * Y_MAX * SCALE, 1)


def project(lon, lat):
    lam = math.radians(max(-180.0, min(180.0, lon)))
    phi = math.radians(lat)
    t = math.asin(M * math.sin(phi))
    t2 = t * t
    t6 = t2 * t2 * t2
    x = 2 * math.sqrt(3) * lam * math.cos(t) / (3 * (9 * A4 * t6 * t2 + 7 * A3 * t6 + 3 * A2 * t2 + A1))
    y = t * (A4 * t6 * t2 + A3 * t6 + A2 * t2 + A1)
    return ((x + X_MAX) * SCALE, (Y_MAX - y) * SCALE)


def rdp(points, eps):
    if len(points) < 3:
        return points
    keep = [False] * len(points)
    keep[0] = keep[-1] = True
    stack = [(0, len(points) - 1)]
    while stack:
        a, b = stack.pop()
        ax, ay = points[a]
        bx, by = points[b]
        dx, dy = bx - ax, by - ay
        norm = math.hypot(dx, dy) or 1e-9
        best, best_d = None, eps
        for i in range(a + 1, b):
            px, py = points[i]
            d = abs(dy * px - dx * py + bx * ay - by * ax) / norm
            if d > best_d:
                best, best_d = i, d
        if best is not None:
            keep[best] = True
            stack.append((a, best))
            stack.append((best, b))
    return [p for p, k in zip(points, keep) if k]


def simplify_ring(ring, eps):
    """RDP for a closed ring: split it at the point farthest from the start,
    because a closed ring's first and last points coincide."""
    if len(ring) < 4:
        return ring
    x0, y0 = ring[0]
    k = max(range(len(ring)), key=lambda i: (ring[i][0] - x0) ** 2 + (ring[i][1] - y0) ** 2)
    return rdp(ring[: k + 1], eps) + rdp(ring[k:], eps)[1:]


def area(ring):
    s = 0.0
    for (x1, y1), (x2, y2) in zip(ring, ring[1:] + ring[:1]):
        s += x1 * y2 - x2 * y1
    return abs(s) / 2


def centroid(ring):
    xs = [p[0] for p in ring]
    ys = [p[1] for p in ring]
    return (sum(xs) / len(xs), sum(ys) / len(ys))


def main():
    src = Path(sys.argv[1])
    out = Path(__file__).resolve().parent.parent / "ui/src/lib/passport/world.json"
    data = json.loads(src.read_text())
    countries = {}
    for f in data["features"]:
        p = f["properties"]
        code = p.get("ISO_A2_EH") or p.get("ISO_A2")
        if not code or code == "-99":
            continue
        geom = f["geometry"]
        polys = geom["coordinates"] if geom["type"] == "MultiPolygon" else [geom["coordinates"]]
        c = countries.setdefault(code, {"c": code, "n": p.get("NAME_EN") or p["NAME"], "rings": []})
        for poly in polys:
            for ring in poly:
                pts = [project(lon, lat) for lon, lat in ring]
                c["rings"].append(pts)

    result = []
    for c in countries.values():
        rings = []
        total = 0.0
        biggest = None
        for pts in c["rings"]:
            simple = simplify_ring(pts, TOLERANCE)
            a = area(pts)
            total += a
            if biggest is None or a > biggest[0]:
                biggest = (a, pts)
            # Round to whole units and drop repeated points.
            snapped = []
            for x, y in simple:
                q = (round(x), round(y))
                if not snapped or snapped[-1] != q:
                    snapped.append(q)
            if len(snapped) >= 4 and area(snapped) > 0.6:
                rings.append(snapped)
        if not rings and biggest:
            rings.append([(round(x), round(y)) for x, y in simplify_ring(biggest[1], TOLERANCE)])
        d = "".join("M" + "L".join(f"{x} {y}" for x, y in r[:-1]) + "Z" for r in rings)
        entry = {"c": c["c"], "n": c["n"], "d": d}
        if total < TINY_AREA and biggest:
            cx, cy = centroid(biggest[1])
            entry["dot"] = [round(cx, 1), round(cy, 1)]
        result.append(entry)
    result.sort(key=lambda e: e["c"])
    out.write_text(json.dumps({"w": W, "h": H, "countries": result}, separators=(",", ":")))
    print(f"{len(result)} countries, {out.stat().st_size // 1024} KB -> {out}")


if __name__ == "__main__":
    main()
