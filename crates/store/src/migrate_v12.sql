-- Screenshots taken during flights. Only where each file is: FlightDeck
-- never copies the pictures. A screenshot belongs to the flight whose time
-- it was taken in, linked when the flight is saved; until then flight_id is
-- empty.
-- The first schema had a placeholder screenshot table that was never used.
DROP TABLE IF EXISTS screenshot;
CREATE TABLE screenshot (
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  taken_ms INTEGER NOT NULL,
  flight_id INTEGER REFERENCES flight(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS screenshot_flight ON screenshot(flight_id);
CREATE INDEX IF NOT EXISTS screenshot_taken ON screenshot(taken_ms);
