## Why

The assistant has a complete self-improvement pipeline (traces → self-analyze → refinements → /review) but it never activates autonomously. Skills are hand-authored and never refined. Traces collect tool-level data but don't link back to which skill was active. Hermes Agent (NousResearch) demonstrates that autonomous skill creation and improvement is the key differentiator for agents that get better over time. We have 90% of the infrastructure — the missing pieces are skill-scoped tracing, a post-turn skill creation hook, and a periodic improvement cycle.

## What Changes

- **Skill-scoped tracing**: When `load-skill` is called, all subsequent tool spans in that turn are tagged with `active_skill`. Both the SQLite and Iceberg exporters persist this as a new column, enabling querying "how does skill X perform?" regardless of which trace backend is active.
- **Autonomous skill creation**: A post-turn evaluator (LLM-as-judge) decides whether a completed turn was novel and complex enough to codify as a new skill. If yes, generates and registers a SKILL.md automatically.
- **Periodic self-improvement**: A scheduled task (configurable interval, default 6h) reviews skill execution stats and auto-applies refinements to underperforming skills. Stores `previous_skill_md` for revert-on-regression safety.
- **Configuration**: New `[learning]` section in config.toml controls all autonomous behavior (enabled, thresholds, intervals).

## Non-goals

- Cross-agent skill sharing (each persona learns independently for now)
- Population-based / evolutionary optimization (GEPA-style) — single-shot LLM refinement is sufficient for v1
- Progressive skill loading / lazy expansion (future optimization)
- Web UI for reviewing refinements (CLI `/review` remains the manual override)

## Capabilities

### New Capabilities

- `skill-scoped-tracing`: Tag tool execution spans with the active skill name for skill-level performance analytics
- `autonomous-skill-creation`: Post-turn LLM evaluation that generates new skills from novel complex tasks
- `periodic-skill-improvement`: Scheduled task that auto-analyzes and refines underperforming skills with revert safety

### Modified Capabilities

## Impact

- **Crates modified**: `runtime` (orchestrator dispatch, new modules), `storage` (traces, refinements), `opentelemetry-exporter-sqlite` (new column extraction), `opentelemetry-exporter-iceberg` (new Parquet column), `web-ui` (Iceberg backend query update), `tool-executor` (self-analyze query change), `core` (config types)
- **New migration**: adds `active_skill` column to `distributed_traces` + `previous_skill_md` column to `skill_refinements`
- **Iceberg schema**: adds `active_skill` string column to `assistant_spans` table
- **New config section**: `[learning]` in config.toml
- **Dependencies**: No new external crates needed
- **Risk**: Auto-applied refinements could degrade skill quality — mitigated by revert-on-regression
