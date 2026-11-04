#!/usr/bin/env python3
"""Build FlightDeck's ATC boundary data from the VAT-Spy Data Project.

The VAT-Spy Data Project (https://github.com/vatsimnetwork/vatspy-data-project,
CC BY-SA 4.0) has every FIR and centre sector's boundary, the callsign
prefixes controllers use for them, and the airports with their codes. Clone
it, then run:

    python3 tools/build_vatspy.py <vatspy-data-project folder>

It writes src-tauri/data/vatspy.json.zst: FIRs, UIRs, airports and the
boundary shapes (rounded to 0.001 degree), compressed.
"""

import json
import sys
from pathlib import Path

import zstandard


def sections(text):
    out, cur = {}, None
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith(";"):
            continue
        if line.startswith("[") and line.endswith("]"):
            cur = out.setdefault(line[1:-1], [])
            continue
        if cur is not None:
            cur.append(line.split("|"))
    return out


def main():
    src = Path(sys.argv[1])
    s = sections((src / "VATSpy.dat").read_text(encoding="utf-8", errors="replace"))
    firs = [{"icao": f[0], "name": f[1], "prefix": f[2], "boundary": f[3] or f[0]} for f in s["FIRs"] if len(f) >= 4]
    uirs = [{"id": u[0], "name": u[1], "firs": [x for x in u[2].split(",") if x]} for u in s["UIRs"] if len(u) >= 3]
    airports = []
    for a in s["Airports"]:
        if len(a) < 7:
            continue
        try:
            lat, lon = round(float(a[2]), 4), round(float(a[3]), 4)
        except ValueError:
            continue
        airports.append({"icao": a[0], "name": a[1], "lat": lat, "lon": lon, "iata": a[4], "fir": a[5], "pseudo": a[6] == "1"})
    gj = json.loads((src / "Boundaries.geojson").read_text(encoding="utf-8"))
    bounds = {}
    for f in gj["features"]:
        p = f["properties"]
        g = f["geometry"]
        polys = g["coordinates"] if g["type"] == "MultiPolygon" else [g["coordinates"]]
        polys = [[[[round(x, 3), round(y, 3)] for x, y in ring] for ring in poly] for poly in polys]
        entry = bounds.get(p["id"])
        if entry:
            # The same id twice (an oceanic and a domestic part): one shape.
            entry["polys"].extend(polys)
            continue
        bounds[p["id"]] = {
            "label": [float(p.get("label_lon") or 0), float(p.get("label_lat") or 0)],
            "oceanic": str(p.get("oceanic")) == "1",
            "polys": polys,
        }
    data = {"firs": firs, "uirs": uirs, "airports": airports, "boundaries": bounds}
    raw = json.dumps(data, separators=(",", ":"), ensure_ascii=False).encode()
    out = Path(__file__).resolve().parent.parent / "src-tauri/data/vatspy.json.zst"
    out.write_bytes(zstandard.ZstdCompressor(level=19).compress(raw))
    print(f"{len(firs)} FIRs, {len(uirs)} UIRs, {len(airports)} airports, {len(bounds)} boundaries; "
          f"{len(raw) // 1024} KB raw, {out.stat().st_size // 1024} KB -> {out}")


if __name__ == "__main__":
    main()
