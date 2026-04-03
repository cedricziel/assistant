-- Add optional per-persona turn timeout (seconds).
-- NULL means use the compiled-in default (10 800 s / 3 h).
ALTER TABLE personas ADD COLUMN turn_timeout_secs INTEGER;
