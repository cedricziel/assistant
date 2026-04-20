# Autonomous Skill Learning

The assistant can learn new skills from successful tasks and improve
existing skills based on execution data. All learning features are
opt-in.

## Quick Start

Add to `~/.assistant/config.toml`:

```toml
[learning]
enabled = true
```

That's it. With defaults the assistant will:

1. **Auto-create skills** from novel, successful multi-tool tasks.
2. **Self-improve** underperforming skills every 6 hours.
3. **Revert** improvements that cause regressions.

## How It Works

### Skill Creation (Post-Turn)

After each turn that uses 3+ tools, an LLM judge evaluates whether the
task pattern should become a reusable skill. If yes, a new `SKILL.md`
is written to `~/.assistant/skills/<name>/`.

Turns are skipped if they:

- Already had a skill loaded
- Used fewer than `min_tool_calls_for_skill` tools (default: 3)
- Had errors

### Skill Improvement (Periodic)

A background scheduled task runs on a cron schedule (default: every 6
hours). It:

1. Checks each skill with enough execution history.
2. Identifies skills whose error rate exceeds `error_rate_threshold`.
3. Asks an LLM to generate an improved version.
4. Applies the improvement (or queues it for `/review`).

### Revert-on-Regression

When a refinement is auto-applied, the previous skill body is stored.
After `revert_window` more executions, the system checks whether error
rates increased. If the increase exceeds `revert_regression_threshold`,
the skill is automatically rolled back to its previous version.

## Configuration Reference

```toml
[learning]
# Master switch (default: false)
enabled = true

# Auto-create skills from novel tasks (default: true)
auto_create_skills = true

# Auto-apply refinements or queue for /review (default: true)
auto_apply_refinements = true

# Minimum tool calls in a turn to consider skill creation (default: 3)
min_tool_calls_for_skill = 3

# Minimum traced executions before analyzing a skill (default: 10)
min_executions_for_analysis = 10

# Cron schedule for the improvement cycle (default: every 6 hours)
improvement_cron = "0 0 */6 * * *"

# Error rate above which a skill is considered underperforming (default: 0.2)
error_rate_threshold = 0.2

# Executions to observe after applying a refinement (default: 5)
revert_window = 5

# Error rate increase that triggers a revert (default: 0.1)
revert_regression_threshold = 0.1
```

## Manual Review Mode

If you prefer to review improvements before they're applied:

```toml
[learning]
enabled = true
auto_apply_refinements = false
```

Proposed refinements will be queued as pending. Use the `self-analyze`
tool or the `/review` CLI command to inspect and accept/reject them.

## Observability

Skill-scoped execution data is tracked via OpenTelemetry spans. Each
span carries an `active_skill` attribute when a skill is loaded. This
data feeds into the improvement analysis regardless of whether you use
the SQLite or Iceberg trace backend.

## Disabling

Set `enabled = false` or remove the `[learning]` section entirely.
No background tasks will run and no skills will be auto-created.
