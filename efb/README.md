# FlightDeck in the MSFS 2024 EFB

`flightdeck-efb-test/` is a feasibility test: a Community package that adds a
**FlightDeck** app to the MSFS 2024 EFB tablet. It has two tabs:

- **FlightDeck** shows FlightDeck's own page from the same PC
  (`http://127.0.0.1:8420/`, the server behind Settings, Other devices) in a
  frame.
- **Compatibility check** lists what the EFB's browser engine (Coherent GT)
  can do: CSS variables, grid and flex gap, WebGL for the map, modern
  JavaScript, local storage, and whether FlightDeck's server answers.

The answers decide how the real EFB app is built: a thin frame onto
FlightDeck's page, or a small dedicated app reading FlightDeck's data.

The app is plain JavaScript with no build step. It registers through the
`window.EFB_API` and `window.msfssdk` globals the sim provides, so no part of
the SDK is shipped. The pattern follows the MIT-licensed
[msfs-efb-iframe](https://github.com/RCCSilva/msfs-efb-iframe).

## Build the package

    python3 tools/build_efb_package.py efb/flightdeck-efb-test out/flightdeck-efb-test.zip

This writes `layout.json` (every file with its size and date, which MSFS
needs) and zips the folder.

## Try it

1. In FlightDeck, turn on **Settings, Other devices**.
2. Unzip into the MSFS 2024 Community folder, so it holds
   `Community\flightdeck-efb-test\manifest.json`. Restart MSFS 2024.
3. Start a flight in an aircraft with an EFB, open the EFB and pick
   **FlightDeck**. Pair it with the code from FlightDeck's Settings, as with a
   phone.
4. Look at both tabs and screenshot them.

To remove it, delete the `flightdeck-efb-test` folder and restart the sim.
