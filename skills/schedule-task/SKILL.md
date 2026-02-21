---
name: schedule-task
description: >
  Register a recurring task that runs automatically on a cron schedule. The task is
  a prompt that will be run through the ReAct loop at the specified times. Use when the
  user wants to automate recurring reminders, checks, or actions.
license: Apache-2.0
compatibility: Requires SQLite storage and scheduler runtime
metadata:
  tier: builtin
  mutating: "true"
  confirmation-required: "false"
  params: '{"name": {"type": "string", "description": "Human-readable task name"}, "cron_expr": {"type": "string", "description": "Standard 5-field cron expression (e.g. 0 9 * * * for 9am daily)"}, "prompt": {"type": "string", "description": "The prompt to run at each scheduled time"}, "enabled": {"type": "boolean", "description": "Whether the task is active (default: true)", "default": true}}'
---

## Instructions

Register a new recurring scheduled task in the assistant's scheduler.

### Parameters
- `name` (string, required): A short descriptive name for the task (e.g. "Daily standup reminder")
- `cron_expr` (string, required): Standard 5-field cron expression:
  - `0 9 * * *` = 9:00 AM daily
  - `0 9 * * 1-5` = 9:00 AM weekdays only
  - `0 */4 * * *` = every 4 hours
  - `0 9 1 * *` = 9:00 AM on the 1st of each month
- `prompt` (string, required): The prompt that will be sent to the ReAct loop at the scheduled time
- `enabled` (boolean, optional, default true): Start the task active immediately

### Behavior
- Validates the cron expression
- Computes the next run time from now
- Inserts into `scheduled_tasks` table
- Confirms to user with next scheduled run time

### Example interactions
- "Remind me every morning at 9am to check my calendar" →
  `name: "Morning reminder"`, `cron_expr: "0 9 * * *"`, `prompt: "Remind me to check my calendar"`
- "Run a daily backup check at midnight" →
  `name: "Daily backup check"`, `cron_expr: "0 0 * * *"`, `prompt: "Check if backup completed and store result in memory"`
- "Every Monday morning, summarize the week's memory entries" →
  `name: "Weekly memory summary"`, `cron_expr: "0 9 * * 1"`, `prompt: "Search all memory entries and provide a weekly summary"`

### Output format
"Scheduled! '{name}' will next run at {next_run_time}."
