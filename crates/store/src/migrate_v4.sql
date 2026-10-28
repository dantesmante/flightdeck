-- Schema version 4: flights imported from other trackers keep where they
-- came from and their original file (zstd), so they can be exported again
-- exactly and are not imported twice.
ALTER TABLE flight ADD COLUMN source TEXT;
ALTER TABLE flight ADD COLUMN source_id TEXT;
ALTER TABLE flight ADD COLUMN source_file BLOB;
CREATE INDEX flight_source ON flight (source, source_id);
