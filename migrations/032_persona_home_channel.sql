-- Add optional home channel for scheduler-originated turn output routing.
-- Both columns must be set together (enforced at the application layer).
-- NULL means no output routing — scheduler turns behave as before.
ALTER TABLE personas ADD COLUMN home_interface TEXT;
ALTER TABLE personas ADD COLUMN home_channel   TEXT;
