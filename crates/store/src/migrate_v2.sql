-- Schema version 2: settings, photos, flight end points.
CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
ALTER TABLE aircraft ADD COLUMN photo TEXT;        -- data URL of the owner's picture
ALTER TABLE pilot ADD COLUMN photo_url TEXT;       -- data URL for the passport
ALTER TABLE flight ADD COLUMN dep_lat REAL;
ALTER TABLE flight ADD COLUMN dep_lon REAL;
ALTER TABLE flight ADD COLUMN arr_lat REAL;
ALTER TABLE flight ADD COLUMN arr_lon REAL;
