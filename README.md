# FlightDeck

A light, local-first flight tracker for flight simulators. It connects to
your sim on its own, logs every flight gate to gate, analyses each landing
and draws the flight on an open-source map. Everything stays on your PC in
one SQLite file.

This release has core tracking with MSFS plus a first version of the
passport book, the paper logbook and the hangar. X-Plane support follows
(see the roadmap at the bottom).

MIT licensed.

## What works now

- **Automatic tracking for MSFS 2020 and MSFS 2024.** Start the sim and
  FlightDeck connects through SimConnect. A flight is logged from block out
  to block in with no clicks.
- **Seven flight phases**: preflight, taxi out, takeoff roll, airborne,
  landing rollout, taxi in, arrived. Every change is debounced, so bounces
  and gusts never split a flight.
- **Landing analysis**: touchdown rate (the sim's own figure when MSFS gives
  one), peak G, bounces, touch-and-goes, cross and head wind, and a grade
  from Butter to Very hard.
- **Live map** with MapLibre GL and OpenFreeMap vector tiles (OpenStreetMap
  data), no API key. If tiles cannot load, the map falls back to a plain
  grid and keeps drawing your flight.
- **Real airport names and countries** for every departure and arrival,
  from 72,635 airports (OurAirports, public domain) built into the app.
- **Saved flights** with block and air times, distance, fuel, track and
  landing report.
- **Crash-safe**: the flight in progress is journaled to disk. If the sim
  or the app crashes, the next start saves what you flew as a partial
  flight.
- **Demo mode**: a built-in flight model flies five routes gate to gate at
  30 times real speed, so you can try everything without a sim.
- **Telemetry recorder**: record a real flight once, then replay it through
  the tracker for testing (see below).
- **Passport book**: a burgundy passport you open and page through, with a
  data page (your photo, details and a machine-readable zone), generated
  rubber stamps and an index of countries. International flights earn
  departure and arrival stamps (a first-entry seal the first time you land
  in a country); domestic flights earn regional stamps.
- **Paper logbook**: a two-page ruled logbook in FAA or EASA layout,
  handwritten or typed, with page totals carried forward. Night time and
  night landings come from the sun's position along your track. Click a
  line to correct it.
- **Hangar**: every aircraft you have flown as a large picture you scroll
  sideways, with hours, distance, flights, best landing and where you last
  parked it. Each gets a drawn illustration; you can use your own photo.
- **Settings**: pilot details and photo, nautical miles, statute miles or
  kilometres, logbook layout, regional stamps, and **sample flights** to
  fill the passport, logbook and hangar for a quick look.

## Set up on Windows

You need:

1. **Rust**: install from https://rustup.rs (choose the default MSVC
   toolchain; it offers to install the Visual Studio C++ build tools).
2. **Node.js 20 or newer**: https://nodejs.org
3. **WebView2**: already part of Windows 10 and 11.

Then, in the project folder:

```
npm run setup      # installs the JavaScript packages
npm run dev        # starts FlightDeck with live reload
```

The first build compiles everything and takes a few minutes; later starts
are fast. To make an installer (MSI and setup .exe in
`target/release/bundle/`):

```
npm run build
```

### SimConnect.dll

FlightDeck loads `SimConnect.dll` while it runs instead of linking to it,
so **you do not need the MSFS SDK to build**, and no SDK files live in this
repository. To *connect* to MSFS it needs the DLL. It looks in this order:

1. The path in the `FLIGHTDECK_SIMCONNECT_DLL` environment variable.
2. Next to `flightdeck.exe`.
3. `C:\MSFS 2024 SDK\SimConnect SDK\lib\SimConnect.dll`
4. `C:\MSFS SDK\SimConnect SDK\lib\SimConnect.dll`
5. The `MSFS_SDK` environment variable's `SimConnect SDK\lib` folder.
6. The normal Windows DLL search path.

The easiest route: install the MSFS SDK from the sim's Developer Mode
(Help → SDK Installer). It installs to one of the folders above. If
the DLL is missing while MSFS is running, the app tells you.

Before shipping SimConnect.dll inside the installer, check the SDK licence
terms allow redistribution.

## Try it without a sim

Open FlightDeck, pick a route under "No sim handy?" and press **Fly a demo
flight**. The demo taxis, takes off, climbs, cruises, descends, lands and
parks; the flight then appears under Flights.

## Your data

Flights are saved to `flightdeck.db` in
`%APPDATA%\io.github.flightdeck\`. Back up that one file to keep your
history. The flight in progress lives next to it in `active-flight.jsonl`
until it is saved.

## Testing

```
npm test                      # all Rust tests (the tracker, store, connectors)
npm --prefix ui run check     # type-check the UI
```

The tests fly every demo route through the tracker and check phases,
landing rate, distance and track size, so you can change tracking rules
safely.

### Record a real flight as a test fixture

The `fd-recorder` tool saves exactly what the sim sends, then replays it
through the tracker:

```
cargo run -p flightdeck-recorder -- record fixtures\my-flight.jsonl   # MSFS running
cargo run -p flightdeck-recorder -- replay fixtures\my-flight.jsonl
cargo run -p flightdeck-recorder -- demo   fixtures\demo.jsonl 1      # no sim needed
```

Replay prints every phase change, the landing report and a summary with
airport names.

## To confirm on the first real MSFS flights

These come from SimConnect documentation and community knowledge, and are
marked in the code. One short flight in each sim settles them:

- [ ] MSFS 2024's process name is `FlightSimulator2024.exe` (watcher).
- [ ] SimConnect reports app version 11 for MSFS 2020 and 12 for MSFS 2024.
- [ ] Pitch and bank signs (FlightDeck stores nose-up and right-bank positive).
- [ ] `ATC MODEL` gives a usable ICAO type in MSFS 2024 aircraft.
- [ ] `PLANE TOUCHDOWN NORMAL VELOCITY` matches the sim's own landing rate.

Record each test flight with `fd-recorder` and keep it in `fixtures/`.

## Project layout

```
crates/
  telemetry/        the common Telemetry model every sim is converted to
  tracker/          flight phase state machine and landing analysis
  store/            SQLite schema, track codec, crash journal
  refdata/          airports, countries, regions (OurAirports, embedded)
  conn-simconnect/  MSFS 2020 + 2024, SimConnect.dll loaded at runtime
  conn-demo/        built-in flight model for demos and tests
  watcher/          finds running sims by process name
tools/recorder/     fd-recorder: record and replay telemetry
src-tauri/          the desktop app: engine thread and commands
ui/                 Svelte 5 interface, MapLibre map
fixtures/           recorded flights used as test data
```

### Updating the airport data

```
git clone https://github.com/davidmegginson/ourairports-data
cargo run -p refdata --features build-tool --bin build-refdata -- ourairports-data
```

## Roadmap

1. **Core and MSFS** (this release). Gate: a full MSFS 2020 and 2024 flight
   logs with no clicks.
2. **Map and X-Plane**: offline PMTiles map, flight replay, X-Plane 11 (UDP)
   and X-Plane 12 (web API) connectors.
3. **Pilot logbook**: a paper-style two-page logbook, FAA or EASA layout,
   night and cross-country time, PDF and CSV export.
4. **Passport book**: generated SVG stamps, regional stamps for domestic
   flights, international stamps on arrival in a new country.
5. **Polish and v1.0**: SimBrief import, landing analysis card, performance
   tuning, signed installer.

## Credits

- Map data © OpenStreetMap contributors; tiles by OpenFreeMap; rendering
  by MapLibre GL JS.
- Airport data from OurAirports (public domain).
- Fonts: Barlow and Barlow Condensed (SIL Open Font License).
