# ADR-0006: Autonomous Skill Learning and Self-Improvement

**Status**: Accepted
**Date**: 2026-04-21

## Context

The assistant's skill system is static: skills are authored manually and
never updated based on runtime experience. When a skill underperforms
(high error rate, common failure patterns) the operator must notice and
rewrite it by hand. Similarly, novel multi-tool workflows that the
assistant performs successfully are lost — they don't become reusable
templates for future tasks.

We want the assistant to:

1. **Learn new skills** from successful, novel task completions.
2. **Improve existing skills** by analyzing execution traces and
   generating refinements.
3. **Self-correct** by reverting refinements that cause regressions.

## Decision

Implement a three-phase autonomous learning pipeline, gated behind
`[learning] enabled = true` (off by default):

### Phase 1: Skill-Scoped Tracing

Tag every OTel span with an `active_skill` attribute when a skill is
loaded via the `load-skill` tool. This attribute propagates through
the span tree for that turn and is persisted to the SQLite trace store
via a new `active_skill` column.

A `SkillStatsProvider` trait abstracts querying skill-scoped statistics,
keeping the analysis logic independent of where traces are stored.

> **Superseded in part by ADR-0010.** The Iceberg trace backend that
> originally implemented `SkillStatsProvider` alongside SQLite was
> removed; the trait remains as the seam for a future external-query
> implementation.

### Phase 2: Post-Turn Skill Creation

After each orchestrator turn completes, a background task evaluates
whether the turn should produce a new skill:

1. **Heuristic gate** — fast checks: minimum tool call count, no
   active skill already loaded, no errors in the turn.
2. **LLM judge** — if the gate passes, an LLM reviews the conversation
   history and decides whether to create a skill (structured JSON output
   with name, description, and body).
3. **Registration** — the skill is written to `~/.assistant/skills/`
   via `SkillRegistry::create_user_skill()` with collision-safe naming.

### Phase 3: Periodic Self-Improvement

A scheduled task (default: every 6 hours via cron) runs the improvement
cycle:

1. **Regression check** — recently-applied refinements are evaluated.
   If post-apply error rate exceeds `error_rate_threshold +
revert_regression_threshold`, the skill is reverted to its previous
   body. Otherwise it's confirmed.
2. **Underperformer scan** — all skills with sufficient execution data
   (`min_executions_for_analysis`) are checked. Those exceeding
   `error_rate_threshold` get an LLM-generated refinement.
3. **Apply/queue** — with `auto_apply_refinements = true`, improvements
   are applied immediately (storing `previous_skill_md` for rollback).
   Otherwise they're queued for manual `/review`.

### Configuration

All thresholds are configurable in `[learning]`:

| Field                         | Default         | Purpose                                   |
| ----------------------------- | --------------- | ----------------------------------------- |
| `enabled`                     | `false`         | Master switch                             |
| `auto_create_skills`          | `true`          | Enable post-turn creation                 |
| `auto_apply_refinements`      | `true`          | Auto-apply or queue for review            |
| `min_tool_calls_for_skill`    | `3`             | Heuristic gate minimum                    |
| `min_executions_for_analysis` | `10`            | Data threshold for analysis               |
| `improvement_cron`            | `0 0 */6 * * *` | Improvement cycle schedule                |
| `error_rate_threshold`        | `0.2`           | Underperformance threshold                |
| `revert_window`               | `5`             | Executions before evaluating regression   |
| `revert_regression_threshold` | `0.1`           | Absolute error increase to trigger revert |

## Consequences

### Positive

- Skills improve over time without operator intervention.
- Novel workflows are captured as reusable templates.
- Regression safety: bad refinements are automatically rolled back.
- Fully opt-in; zero impact when disabled.

### Negative

- Additional LLM calls for the judge and refinement generation (cost).
- The quality of auto-created skills depends on the LLM's judgment.
- Scheduled task adds background compute load every 6 hours.

### Risks and Mitigations

| Risk                                | Mitigation                                                                |
| ----------------------------------- | ------------------------------------------------------------------------- |
| Runaway skill creation              | Heuristic gate filters trivial turns; LLM judge provides second check     |
| Bad refinements degrade performance | Revert-on-regression with configurable threshold                          |
| Cost of background LLM calls        | Only fires for skills exceeding error threshold; cron interval is tunable |
| Skill naming collisions             | Numeric suffix resolution (`-2`, `-3`, etc.)                              |

## Alternatives Considered

1. **Manual-only improvement** — Status quo. Rejected because it doesn't
   scale and relies on operator awareness of failure patterns.
2. **Rule-based improvement (no LLM)** — Simpler but can't generate
   meaningful instruction text. Rejected in favor of LLM-generated bodies.
3. **Continuous improvement (every turn)** — Too expensive and noisy.
   Periodic (cron-based) is a better tradeoff for cost vs. freshness.
