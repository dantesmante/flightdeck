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
  FlightDeck connects through SimConnect. A flight begins once the
  aircraft is spawned in the world and sits on the ground for a few
  seconds, unpaused, and you are in it (battery or main bus on, or an
  engine running). The menus never start one, whatever position the sim
  reports there. It is logged with no clicks; flight time counts from wheels
  up to touchdown.
- **Start a flight**: on the map, press the amber **Start a flight** button
  at the bottom right; a card slides up over the map with a simple flight
  plan: callsign and flight number, the aircraft (the one in
  the sim, or any aircraft in your hangar), origin, destination and
  alternate. The origin is filled in from where the aircraft is: where the
  sim has it on the ground, or where a hangar aircraft was parked (so
  **Fly this aircraft** on the map starts from its airport). Airports are
  found by code or name as you type. Your latest SimBrief plan is shown
  (callsign, route, aircraft, distance); press it to fill in the form.
  Once origin and destination are chosen, the map zooms to the route and
  previews it as a dashed purple line. During a flight the same card shows
  the flight; fold it down to see more of the map.
  A plan can also be added mid-flight with **Add a flight plan**.
- **Boarding pass**: the active flight shows as a boarding pass with the
  callsign and airline, registration and type, the aircraft's picture when
  it is in your hangar, the route, block out and takeoff times, the ETA,
  and a progress bar with the distance flown and left.
- **SimBrief**: add your SimBrief Pilot ID in Settings to import plans. The
  planned route shows as a dashed line with its waypoints as soon as it is
  imported (before the flight starts), the boarding
  pass gets the callsign and destination, and the plan is saved with the
  flight.
  **View OFP** (under the import, and at the top right of the flight card
  once flying) opens the full SimBrief OFP with jump tabs for times,
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
- **Volanta import**: Settings, Import / export. Import any number of
  Volanta flight files (.json); each becomes a FlightDeck flight (map,
  logbook, passport, hangar) with Volanta's block times, distance, fuel
  and landing record, and the original file is kept so exporting it gives
  back the identical file.
- **Elevatex import**: Settings, Import / export. Import any number of
  Elevatex flight files (.json); each becomes a FlightDeck flight with its
  track, landing and times.
- **Import review**: before a bulk import saves anything, a list shows
  every flight found in the chosen files with its route, date, time and
  aircraft, ticked to import. Flights FlightDeck seems to have already
  (the same file imported before, a flight on the same day between the
  same airports, or the same flight twice in this import) are shown in red
  and left unticked; tick one to import it anyway. Files that cannot be
  read are listed with the reason.
- **Export**: Settings, Import / export. **Export** opens a list of every
  flight: tick the ones you want and each is saved as a flight file
  (.json, the common flight-tracker format) in Downloads\FlightDeck
  export. **CSV export** saves the ticked flights as one CSV table with
  the usual flight-tracker CSV columns: Origin, Destination, DepartureTime,
  Duration, Airline, Callsign, FlightNumber, AircraftType,
  AircraftRegistration, Route, ArrivalTime, Distance and Fuel.
- **Runways and taxiways** (Map layers, Airports and flights; on unless
  switched off): zoomed in on an airport the map draws its layout from
  OpenStreetMap: runways with their numbers and dashed centrelines,
  taxiways as yellow lines with their letters, aprons, and gate numbers
  close in. It comes with the map tiles already used, so it costs nothing
  extra. Each runway end carries its own number (10L at the west end of
  10L/28R, 28R at the east), worked out from the runways in view. How
  complete it is depends on OpenStreetMap (very good at large
  airports, patchy at small fields) and it shows the real airport, which
  may differ from the sim's scenery.
- **Divert**: during a flight with a plan (SimBrief or Start a flight),
  **Divert flight** lists the three nearest airports fit to land at (large
  and regional airports with a hard runway of at least 2,000 ft, so no
  grass strips or short fields), with distance, direction and longest
  runway, or takes any airport you type. The plan's destination becomes
  that airport (the map's planned line follows) and the logbook line notes
  "Diverted from" the airport first planned.
- **Screenshots**: choose your screenshots folder in Settings, Preferences.
  FlightDeck watches it (and its sub-folders) for new pictures; one taken
  during a flight joins that flight, shows in the flight's card and the
  live card, and is pinned on the map where it was taken (from the
  flight's track at that moment). Click one to see it full size. The
  pictures are never copied: FlightDeck links to each file where it is, so
  moving, renaming or deleting one breaks its link. Other devices see them
  too, read from the PC. Folders named "thumbnails" (where Steam keeps a
  small copy of each screenshot) are skipped.
- **Photos** (left bar, under Passport): every screenshot, two to a row,
  in two sections that fold away, **Not in a flight** and **Past flights**
  (grouped by flight, newest first), beside a map that shows nothing but their pins. Pointing at
  a picture brings out its pin, and pointing at a pin brings out its
  picture; clicking a picture or a pin opens it full
  size, with **Delete** (to the Recycle Bin, or only removed from
  FlightDeck, leaving the file) and **Close**. The small pictures are made
  on the PC when first shown and kept in memory only, never written to
  disk. Devices need permission to make changes to delete.
- **Backup data** (Settings): **Save a backup** writes one .fdbackup file
  with everything FlightDeck keeps (flights and tracks, logbook, hangar
  and its pictures, pilot, passport, settings). A **backup folder** (a
  OneDrive, Google Drive or Dropbox folder, say) can be kept up to date,
  refreshed whenever FlightDeck closes, keeping the one before as
  "(previous)". **Restore a backup** replaces everything on this PC with a
  backup when FlightDeck restarts (on a new PC: install, restore, done);
  what was there is kept as flightdeck.before-restore.db. Screenshots are
  not included, as FlightDeck only links to them.
- **Approach lines**: with a flight plan, zoomed in on the destination
  (zoom 10 and in), each runway gets a white dashed line carried 10 nm out
  along its approach, labelled with the runway it leads to. Runways come
  from OpenStreetMap, so they show where OpenStreetMap has them.
- **Flight time**: the logbook, flights, passport and pilot totals count
  from wheels up to touchdown (the final landing), less time paused in the
  air. Time taxiing or parked is not logged; block out and block in are
  still shown. A flight that started in the air counts from its first
  moment. Flights recorded before this are counted again once, unless you
  changed their time yourself.
- **Airports on the map**: pointing at an airport shows its code, name and,
  when its latest METAR is already known, its flight category (VFR, MVFR,
  IFR, LIFR); airports without one show none rather than wait.
  **Right-click** an airport for **Set as origin**, **Set as destination**
  (they fill in Start a flight on the Map tab) and **Zoom to fit**. The
  airport clicked is marked by an amber ring that glows softly in and out
  until you zoom in on it (zoom 12.5).
- **Airspace (US only)** in Airports & Flights: Class B (solid blue), C
  (magenta) and D (dashed blue) around US airports, with ceiling/floor in
  hundreds of feet along the edges as charts print them ("100/30", "[25]"
  for a Class D ceiling). From the FAA's NASR data (public domain), fetched
  once per 28-day cycle and kept in the data folder; other countries have no
  free airspace data.
- **Draw route** (Start a flight): choose the origin (type it or click an
  airport), click the map to add waypoints (an airport clicked is passed
  over), and finish on the destination airport. The route becomes the
  flight plan, drawn on the map, and **Save as .PLN** writes it as a
  Microsoft Flight Simulator flight plan (VFR or IFR, cruise altitude;
  user waypoints) that MSFS 2020 and 2024 load from the world map. On a
  phone or tablet the file downloads. "Enter route manually" comes later.
- **PFD** (beside Layers): a primary flight display in a window you can
  move and size (full screen on a phone): attitude with pitch ladder, bank
  scale and slip, speed and altitude tapes, vertical speed, heading rose
  with track, and the autopilot's selected speed, altitude, heading and
  vertical speed, the altimeter setting, true and ground speed, radio
  height and AP. Fed about 15 times a second from the sim while open (MSFS
  instrument values; the demo shows what it has). On the simulator PC it
  is plain sky and ground; on phones and tablets **SVT** draws the terrain
  ahead (synthetic vision, from the free Terrain Tiles, with water in dark
  blue and airports in concrete grey with runway and taxiway centrelines
  from OpenStreetMap; on the ground the view sits at eye height on the
  runway, never under it, and the runway runs from under the nose to its
  vanishing point), which costs the PC nothing; on the PC a note says so.
  Laid out like the G1000 in MSFS: the horizon sits about a third of the
  way down, the tapes start just under the top, and the heading rose fills
  the lower part. **Split** docks the PFD to the
  right half of the Map tab with the map on the left (the flight card
  folds away until you open it; leaving split brings it back). The PFD
  belongs to the Map tab: other tabs hide it, and it is back when you
  return.
- **Roads** (Layers → Map): roads coloured by importance (motorways
  orange, major roads amber and yellow, smaller ones grey as you zoom in),
  their names along them, and route shields: Interstate, US and state
  highways, E-roads and motorways. From the same OpenFreeMap tiles as the
  map, so nothing more is downloaded. A slider fades them (15–100%).
- **Terrain** (Layers → Map): terrain coloured against your altitude by
  the G1000's TAWS rules. In the air: red above you or within 100 ft
  below, yellow 100–1,000 ft below, green 1,000–2,000 ft below, black
  2,000 ft or more below. On the ground: red only where terrain rises
  400 ft or more above you, black elsewhere. Drawn for about 100 miles
  around the aircraft (from zoom 6 in), following it as it flies, so only
  those elevation tiles are fetched; recoloured every 50 ft. Needs the sim
  connected. From the free Terrain Tiles. With **Topographic** on too, its
  relief shading and contour lines (in white) lie over the colours.
- **Obstacles** (Layers → Map, US only): towers, antennas, wind turbines
  and stacks of 200 ft or more from the FAA's Digital Obstacle File
  (public domain, fetched at most every 28 days), with the chart symbol
  (rays when lit), coloured against your altitude like Terrain, and the
  top's height (height above ground) when zoomed in.
- **Range rings** (Layers → Map): 2 or 3 rings around the aircraft,
  labelled in nautical miles, at 5, 10, 15 nm steps or **Auto** (spaced
  to suit the zoom: 1, 2, 5, 10, 20, 40, 80 or 160 nm). Needs the sim
  connected.
- **Your aircraft** is on the map whenever the sim is connected, even with
  no flight started: white with a dark outline, showing only where it is
  (nothing is recorded, and the map doesn't follow it unless you ask).
  Once a flight is being tracked it turns amber as before.
- **Profile** (left of Measure): the vertical profile of your route in
  the bottom half of the Map tab, with the map above (under the map's
  half when the PFD is split beside it), sized to the window. It shows the
  planned climb, cruise and descent (SimBrief's own altitudes at each fix;
  a drawn route climbs at about 300 ft per nm and descends on a 3° path),
  the ground under the route (coarse, from the free Terrain Tiles: a
  guide, not a survey), FAA obstacles within 2 nm of the route (US only,
  red within 100 ft of the planned altitude, yellow within 1,000 ft), class
  B, C and D airspace it passes through (US only; no MOAs or restricted
  areas), fix names, distance flown and to go, and your aircraft side on,
  flying to the right. **Highest** (ahead of you while you fly the route)
  and **Clearance** (cruise altitude above it) sit in its header, with
  switches for Terrain, Obstacles and Airspace; point at the chart for the
  distance, planned altitude and ground there. It follows the flight's plan,
  or the route being set up in Start a flight.
- **Measure** (left of Layers): click points on the map to
  measure a route, leg by leg, in nautical miles with the course of each
  leg, and the total in miles and kilometres too. Great-circle distances.
  Backspace or right-click takes back the last point; Escape or Done ends.
- **Layers**, beside 2D/3D, Profile, Measure and PFD: one panel with three buttons
  under its title,
  **Airports & Flights** (world airports, your airports, past routes,
  runways and taxiways, US airspace, parked aircraft, VATSIM and IVAO),
  **Weather** (radar, flight category, SIGMETs and AIRMETs, wind, airport
  wind) and **Map** (3D flight, day/night, satellite, topographic,
  borders). It opens on the section used last; a dot marks a section
  with something switched on.
- **Flights** page: the list of flights is a card floating over the left
  of the map, a little see-through (put it away with its arrow to see the
  whole map); it opens on the list whenever you come to it from another
  page, and the map shows
  your past routes and airports there whatever the map's own setting.
- **Map layers**: 3D flight, a live day/night line, **satellite**
  imagery (Esri World Imagery), **topographic** relief shading with
  contour lines in feet (from open elevation data, AWS Terrain Tiles;
  contours from zoom 9, worked out in the background only for what is on
  screen), stronger borders, past routes, airports, **parked aircraft** (where you last left
  each aircraft, off by default), and live **rain radar** (RainViewer,
  refreshed every 10 minutes). **World airports** puts the world's
  airports on the map with a **Detail** slider: 1 major airports only, 2
  adds regional airports, 3 adds small airports and airstrips (smaller
  ones appear as you zoom in). Click any airport, or one of your own, for
  its card: code, name and place, elevation, runways (length, width,
  surface, lights), frequencies, your flights there, and the latest
  **METAR** with flight category, wind, visibility, ceiling, temperature
  and altimeter, plus the TAF. Weather comes from aviationweather.gov,
  fetched only when a card opens (from the nearest reporting airport
  within 30 nm when the airport has none). Your choices are remembered. State,
  province and other region lines show from zoom 3 (the base map draws
  them only from zoom 8).
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
  30 times real speed, so you can try everything without a sim. It sits at
  the bottom of Start a flight while no sim is connected; Settings, Updates
  and diagnostics can hide it.
- **Telemetry recorder**: record a real flight once, then replay it through
  the tracker for testing (see below).
- **Passport book**: a passport whose cover turns open like a real one,
  in a cover color you choose (burgundy, red, navy, royal blue, teal,
  green, brown or black). Inside the cover, a world map spread over both
  pages shades every country you have landed in; scroll or double-click
  to zoom, drag to move, and zoomed in a dot marks every airport you have
  flown from or to (**Full map** goes back). Then a data page (your
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
- **Aviation weather** (Map layers, Weather): **Flight category** rings
  every reporting airport in the world by its current METAR (VFR green,
  MVFR blue, IFR red, LIFR magenta; click one for the airport card with the
  full METAR and TAF), and **SIGMET / AIRMET** draws international and US
  SIGMETs (convective included, solid outlines) and US G-AIRMETs (dashed)
  coloured by hazard: thunderstorms, turbulence, icing, low cloud and
  visibility, volcanic ash, tropical cyclones. Click an area for the
  hazard, validity, levels and the raw text. From the NOAA Aviation Weather
  Center (public domain), downloaded only while a layer is on, every 10
  minutes.
- **Wind** (Map layers, Weather): moving particles show the forecast wind,
  coloured by speed (blue light, green and yellow moderate, orange and red
  strong, magenta jet stream over 140 kt), at the surface or FL050, FL100,
  FL180, FL240, FL300, FL340 or FL390; **Auto** follows your aircraft's
  altitude. The key at the bottom left reads the wind under the mouse.
  From NOAA's GFS forecast (public domain, via NOAA Open Data on AWS): one
  level, about 100 KB, taken every 3 hours while the layer is on. The
  particles are drawn on the processor, not the graphics card, at most 15
  frames a second, and pause while the map moves or is out of sight.
- **Airport wind** (Map layers, Weather): zoomed in, every METAR station
  shows an arrow pointing where the wind blows, longer and warmer in colour
  as it strengthens, with its speed and gusts (a ring when calm or
  variable). In an airport's card, **Zoom to** brings the map in close on
  the airport and draws its wind as a large arrow (from the nearest station
  within 30 nm if the airport has no METAR); **Wind arrow** switches the
  arrows on and off from the card.
- **Online networks** (Map layers): **VATSIM ATC**, **VATSIM pilots**,
  **IVAO ATC** and **IVAO pilots**. Staffed centres and FIRs are shaded
  with their callsigns in the middle; staffed airports show their code and
  position letters (D delivery, G ground, T tower, I ATIS) and a ring for
  approach; pilots are aircraft pointing where they fly (VATSIM green,
  IVAO blue). Click a centre's callsign for its controller, frequency,
  facility, time online and info text; an airport for every position
  there with frequencies and ATIS text (and a link to the airport card);
  a pilot for the aircraft, route, altitude, speed and flight plan, with a
  great-circle line from their departure airport to where they are now. Data
  comes from each network's public feed, downloaded by FlightDeck only
  while one of its layers is on, every 30 seconds; nothing is sent to the
  networks.
- **Cross-country time** follows the FAA definition (14 CFR 61.1): a
  flight landing somewhere other than where it departed, more than 50 nm
  in a straight line, logs all its time as cross-country, whichever way it
  came in (recorded, imported or typed in). Set it by hand on a line and
  FlightDeck keeps your value.
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
  (hover a favorite and it tilts under brushed gold; other cards simply
  lighten)
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
  the aircraft on a full map with its past routes and airports shown, and
  its card floating over it: the picture, where it is parked, flights,
  hours and average landing, then its statistics and every flight,
  searchable; pick a flight to draw its track.
- **Pilot profile** (Pilot, above Settings): a cover picture and your
  photo (click either to change it), your Pilot Rating with its history,
  your rank on a shoulder board with XP to the next one, average scores
  and career statistics: hours per month over the last 12 months or any
  year you have flown (recorded and imported flights alike), records, most flown aircraft and favorite
  airports.
- **Pilot rating**: every flight FlightDeck records from gate to gate is
  scored out of 100 in whole numbers: half **Landing quality** (touchdown
  rate, touchdown G and bounces, 50 : 25 : 25, each 1% higher per knot of
  crosswind up to 25%) and half **Flight stability** (G-force 20%, bank
  15%, pitch 15%, smoothness 20%, stable approach 20%, speed limit 10%).
  Limits: bank 30°, nose up 20°, nose down 10°; roll 8°/s, pitch 3°/s, G
  change 0.5 G/s; on the approach (the last descent through 1,000 ft above
  the ground before touchdown) no more than 1,000 fpm down, 15° of bank
  from 500 ft and, while descending from 500 ft to the flare, airspeed
  within 10 kt of the speed around that moment (a steady slow-down of up
  to 25 kt is fine); 250 kt below 10,000
  ft. Every second past a limit costs points, growing with the square of
  the excess. The score moves your rating (everyone starts at 1,000)
  against what your rating expects. Every completed flight earns XP: its
  score (50 if it could not be rated) plus up to 20 for time from wheels
  up to touchdown. XP sets your rank, from Student Pilot to Fleet Captain.
  The **Scoring guide** button on the Pilot page explains it all in
  full, with examples; it is updated whenever the scoring changes.
- **Scores for imported and older flights**: flights imported from
  Volanta or Elevatex, and flights recorded before the pilot rating, are
  scored from what they have. Bank and pitch come from the positions the
  other tracker logged, or are worked out from the track (bank from the
  rate of turn, pitch from the climb and descent angle). A flight without
  a landing report gets its touchdown rate from the positions in the
  last seconds before touchdown. Parts that were never recorded (touchdown
  G, bounces, G in flight) are left out and the others count for more,
  and the flight counts in full towards your rating. To be rated a flight
  needs a touchdown rate and at least two minutes of positions in the
  air, mostly no more than 30 seconds apart; otherwise it shows "Not
  enough data" and still earns XP.
- **Pilot score on every flight**: the Flights tab shows a flight's score
  when you land and the flight ends, and on every saved flight you open,
  with its landing and safety scores, the change to your rating, XP
  towards your next rank and how it was scored. A flight's summary reads
  top to bottom: the flight, its landing (rate and smoothness), altitude
  and speed, pilot score, then the flight plan.
- **Fly this aircraft**: with the Parked aircraft layer on, click a parked
  aircraft and press Fly this aircraft. The next flight is logged to that
  aircraft whatever the sim shows.
- **Settings**, in sections down the left: Pilot profile, Preferences,
  Third-party connections (SimBrief), Import / export (Volanta, Elevatex),
  EFB integration, Device pairing, and Updates and diagnostics (with the
  sample flights). Pilot details and photo, nautical miles, statute miles or
  kilometers, logbook layout, and **sample flights** to
  fill the passport, logbook and hangar for a quick look.
- **Other devices** (Settings, Other devices): use FlightDeck from a
  laptop, tablet or phone on the same home Wi-Fi, in its web browser, while
  the app keeps running on the flight sim PC. Turn on **Allow access from
  other devices**, open the address shown (or scan the QR code) and enter
  the 6-digit pairing code. On a phone the pages open from the menu
  button at the top left, the map's menus show as icons and its cards
  run across the screen. Each device pairs once; **Forget all devices**
  signs them all out. Other devices can only view unless **Other devices
  can edit** is on. Exports, diagnostics, updates and the access settings
  themselves stay on the PC. FlightDeck answers only devices on the same
  network as the PC's Wi-Fi or Ethernet (port 8420), never the internet
  and never anything coming through a VPN or virtual adapter: nothing is forwarded from
  the router and no outside service is involved. Windows may ask whether
  FlightDeck can use private networks; choose Allow.
- **MSFS 2024 EFB** (Settings, MSFS 2024 EFB): **Install in MSFS 2024 EFB**
  adds a FlightDeck app to the cockpit tablet: the current flight as a
  boarding pass with altitude, speeds, progress and ETA, Start a flight
  (with the latest SimBrief plan or without one) and End flight, the last
  flight's landing and pilot score, and the pilot's rating, rank and XP. It
  has no map, since the EFB has its own. FlightDeck finds the Community
  folder from the sim's settings (or you choose it), writes the
  `flightdeck-efb` package there and keeps it up to date with each
  FlightDeck version; **Uninstall from EFB** removes only that folder. The
  app pairs itself and talks to FlightDeck on this PC only (127.0.0.1), so
  it needs neither Other devices nor the home network.
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
- Live online-network data from the VATSIM data feed and IVAO's Whazzup v2
  API, both public. FIR and sector boundaries, callsign prefixes and
  airports from the VAT-Spy Data Project (VATSIM), CC BY-SA 4.0, bundled
  in `src-tauri/data` by `tools/build_vatspy.py`.
- METARs and TAFs from aviationweather.gov (NOAA Aviation Weather Center).
- Elevation for shading and contours: Terrain Tiles on AWS Open Data
  (Mapzen; USGS, SRTM, GMTED, ETOPO1 and others), contours drawn with
  maplibre-contour (BSD-3-Clause).
- Satellite imagery: Esri World Imagery (Esri, Maxar, Earthstar
  Geographics, and the GIS User Community), shown with its attribution.
- Fonts: Barlow and Barlow Condensed (SIL Open Font License).
