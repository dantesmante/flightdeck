-- Cross-country time is worked out from the airports unless the pilot set
-- it by hand.
ALTER TABLE log_entry ADD COLUMN xc_manual INTEGER NOT NULL DEFAULT 0;
