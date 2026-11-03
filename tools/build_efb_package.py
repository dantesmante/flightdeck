#!/usr/bin/env python3
"""Build an MSFS 2024 Community package for FlightDeck's EFB app.

Writes the package's layout.json (every file with its size and date, which
MSFS needs to load it) and zips the package folder, ready to unzip into the
Community folder.

    python3 tools/build_efb_package.py efb/flightdeck-efb-test out/flightdeck-efb-test.zip
"""

import json
import os
import sys
import zipfile

# Windows FILETIME: 100-nanosecond steps since 1601-01-01.
EPOCH_DIFF_S = 11_644_473_600
# MSFS's own layout tool leaves these out.
SKIP = {"layout.json", "manifest.json"}


def filetime(path: str) -> int:
    return int((os.path.getmtime(path) + EPOCH_DIFF_S) * 10_000_000)


def write_layout(root: str) -> list:
    content = []
    for folder, _dirs, files in os.walk(root):
        for name in files:
            full = os.path.join(folder, name)
            rel = os.path.relpath(full, root).replace(os.sep, "/")
            if rel in SKIP or name.startswith("."):
                continue
            content.append({"path": rel, "size": os.path.getsize(full), "date": filetime(full)})
    content.sort(key=lambda c: c["path"])
    with open(os.path.join(root, "layout.json"), "w", encoding="utf-8") as f:
        json.dump({"content": content}, f, indent=2)
        f.write("\n")
    return content


def zip_package(root: str, out: str) -> None:
    base = os.path.basename(os.path.normpath(root))
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        for folder, _dirs, files in os.walk(root):
            for name in sorted(files):
                if name.startswith("."):
                    continue
                full = os.path.join(folder, name)
                z.write(full, os.path.join(base, os.path.relpath(full, root)))


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    root, out = sys.argv[1], sys.argv[2]
    content = write_layout(root)
    zip_package(root, out)
    print(f"{len(content)} files in layout.json; package zipped to {out}")


if __name__ == "__main__":
    main()
