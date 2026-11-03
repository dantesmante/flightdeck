# FlightDeck in the MSFS 2024 EFB

`flightdeck-efb/` is FlightDeck's app for the MSFS 2024 EFB tablet: a
Community package with one plain-JavaScript file (no build step) and its
icons. FlightDeck embeds it and installs it from **Settings, MSFS 2024 EFB**
(`src-tauri/src/efb.rs`):

- It finds the Community folder from the sim's `UserCfg.opt`
  (`InstalledPackagesPath`; Microsoft Store and Steam locations), or uses a
  folder the user types in.
- It writes `Community/flightdeck-efb/`, putting the FlightDeck version in
  `manifest.json` and the script (`__FD_VERSION__`), and generates
  `layout.json` (every file with its size and date, which MSFS needs).
- It turns on FlightDeck's server on 127.0.0.1:8420 for the app, even with
  Other devices off. Uninstalling removes only that folder (after checking
  its manifest is ours) and stops the server unless Other devices is on.
- On start, FlightDeck updates an installed app to its own version.

The app:

- **Flight**: the current flight as a boarding pass (callsign, route, out
  and airborne times, time in the air, ETA, progress) with altitude, ground
  speed, vertical speed and heading; End flight. Before a flight: Start a
  flight with the latest SimBrief plan or without one.
- **Last flight**: landing rate, G and grade, the pilot score with landing
  and flight stability, the rating change and XP. It opens by itself when a
  flight is saved.
- **Pilot**: rank, rating, XP to the next rank, averages and totals.

It pairs itself through `POST /api/pair-local`, which FlightDeck answers only
for a program on the same PC with no web origin or the sim's `coui://` one,
and keeps the token in local storage. Calls go to `/api/call/{cmd}` and live
status comes over `/api/events` (polling `get_status` every 2 s when the
socket is down). The server allows cross-origin calls only from `coui://`
pages.

The EFB's browser (Coherent GT) is old: the script keeps to ES2017 (no `?.`,
`??` or class fields), no flexbox `gap`, and leaves the top 56 px (the EFB's
clock and bell) and the bottom 84 px (its dock) clear. It registers through
the `window.EFB_API` and `window.msfssdk` globals the sim provides, so no part
of the SDK is shipped. The pattern follows the MIT-licensed
[msfs-efb-iframe](https://github.com/RCCSilva/msfs-efb-iframe).

## The feasibility test

`flightdeck-efb-test/` is the first test package: a frame onto FlightDeck's
page plus a compatibility check of the EFB browser. It showed the frame
cannot work (no WebGL, old JavaScript), which is why the real app is a small
dedicated one. Build it by hand to check a new sim version:

    python3 tools/build_efb_package.py efb/flightdeck-efb-test out/flightdeck-efb-test.zip
