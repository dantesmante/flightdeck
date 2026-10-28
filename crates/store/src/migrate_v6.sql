-- Schema version 6: the pilot's own aircraft details. The sim's title and
-- registration stay as the key that matches future flights; the pilot's
-- registration, model name and airline are shown instead when set.
-- Aircraft the pilot adds by hand are kept even with no flights.
ALTER TABLE aircraft ADD COLUMN user_reg TEXT;
ALTER TABLE aircraft ADD COLUMN user_model TEXT;
ALTER TABLE aircraft ADD COLUMN airline TEXT;
ALTER TABLE aircraft ADD COLUMN manual INTEGER NOT NULL DEFAULT 0;
