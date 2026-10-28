-- Schema version 5: favourite aircraft in the hangar.
ALTER TABLE aircraft ADD COLUMN favourite INTEGER NOT NULL DEFAULT 0;
