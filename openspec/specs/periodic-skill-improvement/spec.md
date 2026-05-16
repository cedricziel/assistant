# periodic-skill-improvement Specification

## Purpose

TBD - created by archiving change autonomous-skill-learning. Update Purpose after archive.

## Requirements

### Requirement: Improvement cycle runs as scheduled task

The system SHALL register a scheduled task named `skill-self-improve` on startup when learning is enabled. The task SHALL execute on a configurable cron interval (default every 6 hours).

#### Scenario: Startup with learning enabled

- **WHEN** the system starts with `learning.enabled = true`
- **THEN** a scheduled task named "skill-self-improve" SHALL be registered with cron expression from `learning.improvement_cron`

#### Scenario: Startup with learning disabled

- **WHEN** the system starts with `learning.enabled = false` or no `[learning]` config
- **THEN** no "skill-self-improve" scheduled task SHALL be registered

#### Scenario: Task fires on schedule

- **WHEN** the cron interval elapses (default: 6 hours)
- **THEN** the improvement cycle SHALL execute through the orchestrator as a TurnRequest

### Requirement: Improvement cycle identifies underperforming skills

The improvement cycle SHALL query stats for all skills with sufficient execution history and identify those exceeding the error rate threshold.

#### Scenario: Skill with high error rate

- **WHEN** skill "deploy-docker-compose" has 15 executions and 40% error rate, and `error_rate_threshold = 0.2`
- **THEN** the skill SHALL be selected for improvement

#### Scenario: Skill with acceptable error rate

- **WHEN** skill "coding-agent" has 20 executions and 5% error rate, and `error_rate_threshold = 0.2`
- **THEN** the skill SHALL NOT be selected for improvement

#### Scenario: Skill with insufficient executions

- **WHEN** skill "new-skill" has 3 executions and `min_executions_for_analysis = 10`
- **THEN** the skill SHALL NOT be evaluated regardless of error rate

### Requirement: Auto-apply refinements without human review

When `learning.auto_apply_refinements = true`, the improvement cycle SHALL apply generated refinements directly to the skill's SKILL.md file and update the SkillRegistry without requiring the `/review` CLI workflow.

#### Scenario: Refinement auto-applied

- **WHEN** the improvement cycle generates a refinement for "deploy-docker-compose" and auto-apply is enabled
- **THEN** the skill's SKILL.md body SHALL be updated immediately and the refinement status SHALL be set to "accepted"

#### Scenario: Auto-apply disabled

- **WHEN** `learning.auto_apply_refinements = false` and a refinement is generated
- **THEN** the refinement SHALL be stored with status "pending" for manual `/review`

### Requirement: Previous skill body preserved for rollback

Before applying any refinement, the system SHALL store the current SKILL.md body in the `previous_skill_md` column of the `skill_refinements` table.

#### Scenario: Refinement applied with backup

- **WHEN** a refinement is applied to skill "deploy-docker-compose"
- **THEN** the refinement row SHALL contain the full previous SKILL.md body in `previous_skill_md`

### Requirement: Revert on regression

After a refinement is auto-applied, the system SHALL monitor the skill's error rate over the next N executions (configurable via `revert_window`). If the error rate increases by more than the configured threshold, the system SHALL revert to the previous body.

#### Scenario: Error rate improves after refinement

- **WHEN** a refinement is applied and the next 5 executions show 10% error rate (down from 40%)
- **THEN** the refinement SHALL remain active and be marked as "confirmed"

#### Scenario: Error rate regresses after refinement

- **WHEN** a refinement is applied, `revert_window = 5`, `revert_regression_threshold = 0.1`, and the next 5 executions show 55% error rate (up from 40%)
- **THEN** the system SHALL revert the SKILL.md to `previous_skill_md`, mark the refinement as "reverted", and log the reversion

#### Scenario: Insufficient post-apply executions

- **WHEN** a refinement was applied but only 2 of the required 5 `revert_window` executions have occurred
- **THEN** the system SHALL NOT evaluate regression yet and SHALL check again on the next improvement cycle

### Requirement: Refinement status lifecycle

The `skill_refinements` table SHALL support the following status values: `pending`, `accepted`, `rejected`, `reverted`, `confirmed`.

#### Scenario: Auto-applied refinement lifecycle (success)

- **WHEN** a refinement is auto-applied and passes the revert window
- **THEN** its status transitions: `pending` → `accepted` → `confirmed`

#### Scenario: Auto-applied refinement lifecycle (regression)

- **WHEN** a refinement is auto-applied and fails the revert window
- **THEN** its status transitions: `pending` → `accepted` → `reverted`

### Requirement: Configuration section

The system SHALL support a `[learning]` configuration section with all parameters having sensible defaults.

#### Scenario: Full configuration

- **WHEN** config.toml contains a `[learning]` section with all fields
- **THEN** the system SHALL use those values for all learning behavior

#### Scenario: Partial configuration

- **WHEN** config.toml contains `[learning]` with only `enabled = true`
- **THEN** all other fields SHALL use their defaults (`auto_create_skills = true`, `auto_apply_refinements = true`, `min_tool_calls_for_skill = 3`, `min_executions_for_analysis = 10`, `improvement_cron = "0 */6 * * *"`, `error_rate_threshold = 0.2`, `revert_window = 5`, `revert_regression_threshold = 0.1`)
