-- Schema version 7: US spelling for the favorite flag.
ALTER TABLE aircraft RENAME COLUMN favourite TO favorite;
-- Aircraft merged into another (same registration) stay as an alias, so
-- the sim's name for them still finds the aircraft they were merged into.
ALTER TABLE aircraft ADD COLUMN merged_into INTEGER;
