-- Add structured output envelope to workflow step telemetry.
ALTER TABLE workflow_run_steps
    ADD COLUMN output_json TEXT;
