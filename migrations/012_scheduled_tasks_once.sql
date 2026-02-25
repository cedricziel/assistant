-- Add `once` flag so tasks can auto-disable after a single execution.
ALTER TABLE scheduled_tasks ADD COLUMN once BOOLEAN NOT NULL DEFAULT FALSE;
