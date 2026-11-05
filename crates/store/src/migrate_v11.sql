-- Flights recorded without a block out or block in (the aircraft was not
-- seen taxiing) counted the whole recording, including time sitting on the
-- ground. Count from the takeoff roll (2 minutes before takeoff) and to the
-- end of the landing rollout (5 minutes after landing) instead, as new
-- flights do. Only logbook lines the pilot has not changed, and only when
-- this makes the time shorter.
UPDATE log_entry
SET total_min = s.new_min,
    pic_min = CASE WHEN log_entry.pic_min = log_entry.total_min THEN s.new_min ELSE log_entry.pic_min END,
    xc_min = CASE WHEN log_entry.xc_min = log_entry.total_min THEN s.new_min ELSE log_entry.xc_min END,
    night_min = MIN(log_entry.night_min, s.new_min)
FROM (
  SELECT id, old_min,
         CAST(ROUND(MAX(0, inn - out) / 60000.0) AS INTEGER) AS new_min
  FROM (
    SELECT id,
           COALESCE(block_out, CASE WHEN takeoff IS NOT NULL THEN MAX(start_ms, takeoff - 120000) END, start_ms) AS out,
           COALESCE(block_in, CASE WHEN landing IS NOT NULL THEN MIN(end_ms, landing + 300000) END, end_ms) AS inn,
           CAST(ROUND(MAX(0, end_ms - start_ms - paused_ms) / 60000.0) AS INTEGER) AS old_min
    FROM flight
    WHERE (block_out IS NULL OR block_in IS NULL) AND (takeoff IS NOT NULL OR landing IS NOT NULL)
  )
) AS s
WHERE log_entry.flight_id = s.id
  AND ABS(log_entry.total_min - s.old_min) <= 1
  AND s.new_min < log_entry.total_min;
