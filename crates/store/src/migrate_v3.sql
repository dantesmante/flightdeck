-- Schema version 3: the flight plan (e.g. imported from SimBrief) as JSON.
ALTER TABLE flight ADD COLUMN plan TEXT;
