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
  FlightDeck connects through SimConnect. A flight begins once you are in
  the aircraft (battery or main bus on, or an engine running), never from
  the menus, and is logged from block out to block in with no clicks.
- **Start a flight**: on the map, press **Start a flight** and fill in a
  simple flight plan: callsign and flight number, the aircraft (the one in
  the sim, or any aircraft in your hangar), origin, destination and
  alternate. Airports are found by code or name as you type. Or press
  **Import from SimBrief** to fill it in from your latest dispatched plan.
  A plan can also be added mid-flight with **Add a flight plan**.
- **Boarding pass**: the active flight shows as a boarding pass with the
  callsign and airline, registration and type, the aircraft's picture when
  it is in your hangar, the route, block out and takeoff times, the ETA,
  and a progress bar with the distance flown and left.
- **SimBrief**: add your SimBrief Pilot ID in Settings to import plans. The
  planned route shows as a dashed line with its waypoints, the boarding
  pass gets the callsign and destination, and the plan is saved with the
  flight.
  **Briefing (OFP)** opens the full SimBrief OFP with jump tabs for times,
  weights and fuel, the flight log, winds aloft, the ICAO flight plan,
  weather and NOTAMs.
- **Seven flight phases**: preflight, taxi out, takeoff roll, airborne,
  landing rollout, taxi in, arrived. Every change is debounced, so bounces
  and gusts never split a flight.
- **Landing analysis**: touchdown rate (the sim's own figure when MSFS gives
  one), peak G, bounces, touch-and-goes, cross and head wind, and a grade
  from Butter to Very hard.
- **Live map** with MapLibre GL and OpenFreeMap vector tiles (OpenStreetMap
  data), no API key. If tiles cannot load, the map falls back to a plain
  grid and keeps drawing your flight.
- **Route network**: every flight you have flown stays on the map as a faint
  great-circle line between its airports. Hover a line to see the route and
  date; click it to open that flight. Flights that land where they took off
  show as a ring at the airport.
- **Altitude colors**: the flight path shades from red near the ground,
  through orange, to green at cruise, scaled to each flight's own top
  altitude. A key on the map shows the range. Switch the map to **3D** and
  the flight stands up as a colored curtain from the ground to the
  altitude flown, seen from the side (heights are stretched on long
  flights so they stay visible; the key says by how much).
- **Altitude & speed graph** for the live flight, every saved flight and
  each logbook entry: altitude as a filled profile in the same colors,
  ground speed as a yellow line. Hover for exact values at any moment.
- **Volanta import and export**: Settings, Volanta. Import any number of
  Volanta flight files (.json); each becomes a FlightDeck flight (map,
  logbook, passport, hangar) with Volanta's block times, distance, fuel
  and landing record, and the original file is kept so exporting it gives
  back the identical file. Re-importing is skipped. Export opens a list of
  every flight: tick the ones you want and they are saved as
  Volanta-format files into Downloads\FlightDeck export.
- **Elevatex import**: Settings, Elevatex. Import any number of Elevatex
  flight files (.json); each becomes a FlightDeck flight with its track,
  landing and times. Re-importing is skipped.
- **Map layers** panel: 3D flight, a live day/night line, stronger
  borders, past routes, airports, **parked aircraft** (where you last left
  each aircraft, off by default), and live **rain radar** (RainViewer,
  refreshed every 10 minutes). Your choices are remembered.
- **Cancel a flight**: if tracking has started but you decide not to fly,
  Cancel flight throws it away without saving anything.
- **Flights list**: point at a flight and its route lights up on the map.
- **Its own title bar**: FlightDeck draws its own window frame, with the
  time in UTC (Zulu) and your local time under it, and the simulator
  link.
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
- **Passport book**: a passport whose cover turns open like a real one,
  in a cover color you choose (burgundy, red, navy, royal blue, teal,
  green, brown or black). Inside the cover, a world map spread over both
  pages shades every country you have landed in. Then a data page (your
  photo, details and a machine-readable zone), generated rubber stamps and
  an index of countries. Only landings count: where a flight starts (or
  where you spawn) never counts as a visit. Landing in a different country
  from the one you left earns a stamp, and the first landing in a country
  gets a first-entry seal. Overseas territories such as French Guiana,
  Réunion, Guadeloupe, Martinique, American Samoa and Guam are their own
  places on the map, separate from France or the US. Each stamp carries a
  small inked flag. The photo and data pages carry a holographic film that
  shimmers around the mouse as you move it over them, and the page facing
  the data page opens with a quote by John H. Secondari.
- **Paper logbook**: a two-page ruled logbook in FAA or EASA layout,
  handwritten or typed, with page totals carried forward. Night time and
  night landings come from the sun's position along your track. Click a
  line to see the flight on a mini map, edit any field (date, times,
  airports, aircraft, hours, remarks, and the class: SEL, MEL, SES or
  MES) or delete it everywhere. The class comes from a table of aircraft
  types (a C172 is always SEL, an A320 MEL, floats make it sea), not from
  the engine count the sim reports. Arrows under
  the book turn the pages, and the book grows to fill the window so the
  writing stays sharp. Switch to dark paper, or to a simple list of every
  flight if you prefer a plain table to the paper look. **Add manual
  entry** logs a flight FlightDeck did not record; the map shows it as a
  line from departure to arrival. **Find duplicates** lists flights logged
  more than once (same day, aircraft, departure and arrival), ticks all but
  the most complete of each, and deletes them only after you review the
  list.
- **Hangar**: a card for every aircraft you have flown, favorites first
  and then by hours flown; show all or only favorites. Each card shows the
  registration, a photo (your own: hover the picture and upload one;
  otherwise the latest photo of that registration from Planespotters.net,
  credited to its photographer; otherwise a silhouette of the type), a
  plain model name worked out from the sim's title (so "PMDG 737-700
  Ryanair" reads "Boeing 737-700"), where it is parked, flights, hours and
  average landing rate, and a star for favorites. Edit an aircraft's
  registration, model name and airline (the registration strip then takes
  the airline's colors), or **add an aircraft** by hand: flights the sim
  reports with that registration are logged to it. The registration is
  how the hangar knows an aircraft, so there is only ever one aircraft per
  registration: giving a flight or an aircraft a registration already in
  the hangar merges them. Delete an aircraft from its Edit screen, either
  with its flights or moving them to another aircraft. Click a card to open
  the aircraft with its statistics and every flight, searchable, drawn on
  a map.
- **Fly this aircraft**: with the Parked aircraft layer on, click a parked
  aircraft and press Fly this aircraft. The next flight is logged to that
  aircraft whatever the sim shows.
- **Settings**: pilot details and photo, nautical miles, statute miles or
  kilometers, logbook layout, and **sample flights** to
  fill the passport, logbook and hangar for a quick look.
- **Other devices** (Settings, Other devices): use FlightDeck from a
  laptop, tablet or phone on the same home Wi-Fi, in its web browser, while
  the app keeps running on the flight sim PC. Turn on **Allow access from
  other devices**, open the address shown (or scan the QR code) and enter
  the 6-digit pairing code. Each device pairs once; **Forget all devices**
  signs them all out. Other devices can only view unless **Other devices
  can edit** is on. Exports, diagnostics, updates and the access settings
  themselves stay on the PC. FlightDeck answers only addresses on the home
  network (port 8420) and never the internet: nothing is forwarded from
  the router and no outside service is involved. Windows may ask whether
  FlightDeck can use private networks; choose Allow.
- **Duplicate flights** (Logbook, Find duplicates): lists flights on the
  same day, in the same aircraft, between the same airports. The most
  complete one in each set is kept and the rest are ticked; review the list
  and confirm before anything is deleted.

## Install (the easy way)

Download the latest `FlightDeck-…-setup.exe` from the
[Releases page](https://github.com/dantesmante/flightdeck/releases/latest)
and double-click it. If Windows shows "Windows protected your PC", click
**More info**, then **Run anyway** (the app is not yet code-signed).

After that FlightDeck updates itself: when a newer version is published it
shows "FlightDeck … is ready to install" with an **Update now** button.
Every change pushed to `main` publishes a new version automatically.

## Build it yourself (developers)

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

Before shipping SimConnect.dll inside the installer, check the SDK license
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
4. **Passport book**: generated SVG stamps for international flights
   (first version done).
5. **Polish and v1.0**: SimBrief import, landing analysis card, performance
   tuning, signed installer.

## Credits

- Map data © OpenStreetMap contributors; tiles by OpenFreeMap; rendering
  by MapLibre GL JS.
- Airport data from OurAirports (public domain).
- Country outlines from Natural Earth (public domain), projected with the
  Equal Earth projection by `tools/build_world.py`.
- Stamp flags from country-flag-icons by catamphetamine (MIT), bundled by
  `tools/build_flags.py`.
- Flight plans from SimBrief (Navigraph) via its public OFP fetch.
- Weather radar by RainViewer (free API for personal use).
- Fonts: Barlow and Barlow Condensed (SIL Open Font License).
