-- Schema version 8: the pilot can set a logbook line's class (SEL, MEL,
-- SES, MES) when the aircraft's own class is not right for that flight.
ALTER TABLE log_entry ADD COLUMN class_override TEXT;
