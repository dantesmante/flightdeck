-- FlightDeck user database, schema version 1.
-- Times are Unix milliseconds, UTC. Distances in nm, altitudes in ft, fuel in kg.

CREATE TABLE pilot (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  name TEXT NOT NULL DEFAULT '',
  pilot_id TEXT NOT NULL DEFAULT '',
  home_icao TEXT NOT NULL DEFAULT '',
  nationality TEXT NOT NULL DEFAULT '',
  photo BLOB,
  signature_svg TEXT,
  logbook_layout TEXT NOT NULL DEFAULT 'faa'      -- faa | easa
);
INSERT INTO pilot (id) VALUES (1);

CREATE TABLE aircraft (
  id INTEGER PRIMARY KEY,
  icao_type TEXT NOT NULL DEFAULT '',
  title TEXT NOT NULL DEFAULT '',
  registration TEXT NOT NULL DEFAULT '',
  engine_count INTEGER NOT NULL DEFAULT 0,
  engine_type TEXT NOT NULL DEFAULT 'unknown',
  category_class TEXT NOT NULL DEFAULT '',        -- SEL, MEL, RH, GL
  UNIQUE (title, registration)
);

CREATE TABLE flight (
  id INTEGER PRIMARY KEY,
  sim TEXT NOT NULL,                              -- msfs2020 | msfs2024 | xp11 | xp12 | demo
  aircraft_id INTEGER REFERENCES aircraft(id),
  callsign TEXT NOT NULL DEFAULT '',
  flight_no TEXT NOT NULL DEFAULT '',
  dep_icao TEXT, dep_name TEXT, dep_country TEXT, dep_region TEXT,
  arr_icao TEXT, arr_name TEXT, arr_country TEXT, arr_region TEXT,
  planned_arr_icao TEXT,
  alt_icao TEXT,
  start_ms INTEGER NOT NULL,
  end_ms INTEGER NOT NULL,
  block_out INTEGER, takeoff INTEGER, landing INTEGER, block_in INTEGER,
  distance_nm REAL NOT NULL DEFAULT 0,
  fuel_used_kg REAL NOT NULL DEFAULT 0,
  max_alt_ft INTEGER NOT NULL DEFAULT 0,
  landing_rate_fpm REAL,
  landing_g REAL,
  paused_ms INTEGER NOT NULL DEFAULT 0,
  max_sim_rate REAL NOT NULL DEFAULT 1,
  started_airborne INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL,                           -- complete | partial | diverted
  end_reason TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX flight_by_time ON flight(start_ms);

-- The editable logbook line. Minutes are whole numbers.
CREATE TABLE log_entry (
  flight_id INTEGER PRIMARY KEY REFERENCES flight(id) ON DELETE CASCADE,
  total_min INTEGER NOT NULL DEFAULT 0,
  night_min INTEGER NOT NULL DEFAULT 0,
  ifr_min INTEGER NOT NULL DEFAULT 0,
  xc_min INTEGER NOT NULL DEFAULT 0,
  pic_min INTEGER NOT NULL DEFAULT 0,
  dual_min INTEGER NOT NULL DEFAULT 0,
  sim_inst_min INTEGER NOT NULL DEFAULT 0,
  act_inst_min INTEGER NOT NULL DEFAULT 0,
  day_landings INTEGER NOT NULL DEFAULT 0,
  night_landings INTEGER NOT NULL DEFAULT 0,
  approaches INTEGER NOT NULL DEFAULT 0,
  remarks TEXT NOT NULL DEFAULT '',
  locked INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE track (
  flight_id INTEGER PRIMARY KEY REFERENCES flight(id) ON DELETE CASCADE,
  points BLOB NOT NULL,                           -- simplified, delta-encoded, zstd
  point_count INTEGER NOT NULL,
  bbox TEXT NOT NULL                              -- JSON [min_lon, min_lat, max_lon, max_lat]
);

CREATE TABLE flight_event (
  id INTEGER PRIMARY KEY,
  flight_id INTEGER NOT NULL REFERENCES flight(id) ON DELETE CASCADE,
  t INTEGER NOT NULL,
  kind TEXT NOT NULL,
  data TEXT NOT NULL DEFAULT 'null'
);
CREATE INDEX event_by_flight ON flight_event(flight_id, t);

CREATE TABLE stamp (
  id INTEGER PRIMARY KEY,
  flight_id INTEGER REFERENCES flight(id) ON DELETE CASCADE,
  country_code TEXT NOT NULL,
  region_code TEXT,
  icao TEXT,
  kind TEXT NOT NULL,                             -- arrival | departure | first_entry | transit | regional
  issued_at INTEGER NOT NULL,
  seed INTEGER NOT NULL,
  page INTEGER,
  slot INTEGER
);
CREATE INDEX stamp_by_country ON stamp(country_code);

CREATE TABLE note (
  id INTEGER PRIMARY KEY,
  flight_id INTEGER NOT NULL REFERENCES flight(id) ON DELETE CASCADE,
  body TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE TABLE screenshot (
  id INTEGER PRIMARY KEY,
  flight_id INTEGER NOT NULL REFERENCES flight(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  t INTEGER NOT NULL
);
