#!/usr/bin/env python3
"""Bundle the passport stamps' little flags.

The flags are the simplified 3:2 SVGs from the country-flag-icons package
(MIT licence, https://gitlab.com/catamphetamine/country-flag-icons):

    npm pack country-flag-icons && tar xzf country-flag-icons-*.tgz
    python3 tools/build_flags.py package/3x2

It writes ui/src/lib/passport/flags.json: {"FR": "<svg ...>", ...} keyed by
ISO 3166 alpha-2, the codes the airport data uses.
"""

import json
import re
import sys
from pathlib import Path

src = Path(sys.argv[1])
out = Path(__file__).resolve().parent.parent / "ui/src/lib/passport/flags.json"
flags = {}
for f in sorted(src.glob("*.svg")):
    code = f.stem.upper()
    if not re.fullmatch(r"[A-Z]{2}", code):
        continue
    svg = re.sub(r">\s+<", "><", f.read_text().strip())
    flags[code] = svg
out.write_text(json.dumps(flags, separators=(",", ":")))
print(f"{len(flags)} flags, {out.stat().st_size // 1024} KB -> {out}")
