## Context

The assistant records every tool execution as an OpenTelemetry span persisted to either SQLite (`distributed_traces` table) or Apache Iceberg (`assistant_spans` Parquet table), configurable via `[observability].exporter`. A `self-analyze` builtin tool exists that queries trace stats and generates skill improvement proposals, but it only works with SQLite and is never invoked autonomously. Skills are markdown documents with frontmatter, registered in `SkillRegistry` and loaded on demand via the `load-skill` tool. The refinements store holds pending proposals reviewed via the CLI `/review` command.

The system has all the moving parts for a learning loop but no automation connecting them. This design adds three layers: tracing that links tools to skills, a post-turn evaluator that creates skills, and a scheduled task that improves them.

## Goals / Non-Goals

**Goals:**

- Tool spans tagged with the active skill so `stats_for_skill()` queries return meaningful data
- Works with both SQLite and Iceberg trace backends
- Post-turn LLM evaluation that autonomously creates new skills from novel tasks
- Scheduled periodic improvement cycle using existing `self-analyze` logic
- Safe auto-apply with rollback on regression
- All behavior configurable and opt-in via `[learning]` config section

**Non-Goals:**

- Modifying the `TraceBackend` trait in web-ui (read path for dashboards is separate from the write path for self-analyze)
- Requiring Iceberg for the learning features to work (SQLite remains the default and fully supported path)
- Adding API endpoints for learning features (CLI-only for v1)
- Population-based optimization or multi-variant evaluation

## Decisions

### D1: Active skill propagation via turn-scoped state on Orchestrator

When `load-skill` is called, store the skill name on the `Orchestrator` instance (a new `active_skill: Option<String>` field). All subsequent `start_tool_span()` calls within that turn read this field and set `attribute("active_skill", name)`.

**Why not thread-local?** The orchestrator is already the single owner of the turn loop. A field is simpler, testable, and naturally resets between turns. Thread-locals are error-prone with async code.

**Why not a separate "skill execution" span?** Adding a parent span that wraps the whole skill execution would change the span tree structure and break existing trace visualizations. Attributes on leaf spans are additive and non-breaking.

### D2: Dual-backend stats query

`stats_for_skill()` currently lives on `TraceStore` (SQLite-only). Rather than pulling it into the `TraceBackend` trait (which is a web-ui concern), add a new `SkillStatsProvider` trait in `crates/storage` with two implementations:

```rust
#[async_trait]
pub trait SkillStatsProvider: Send + Sync {
    async fn stats_for_skill(&self, skill_name: &str, window: i64) -> Result<TraceStats>;
}
```

- `SqliteSkillStats` — wraps the existing `TraceStore` query, updated to use `active_skill` column
- `IcebergSkillStats` — scans Parquet files from the warehouse directory using `datafusion` (already a transitive dep via iceberg-rust)

The `SelfAnalyzeHandler` and the new `SkillImprover` accept `Arc<dyn SkillStatsProvider>`.

**Alternative considered:** Adding `stats_for_skill` to `TraceBackend`. Rejected because `TraceBackend` is a web-ui-layer abstraction for HTTP handlers; the learning system is a runtime-layer concern.

### D3: Post-turn evaluator as fire-and-forget spawn (like ConversationIndexer)

The skill creation evaluator runs as a background `tokio::spawn` after each successful turn — same pattern as `conversation_indexer::spawn_index`. It receives:

- `conversation_id` + `turn_index` (to load history)
- `tool_count` (number of tools called in this turn)
- `active_skill: Option<String>` (skip if a skill was already active)
- `had_errors: bool` (skip if the turn had tool errors)

Gate logic (heuristic, before LLM):

1. `active_skill.is_some()` → skip (skill already exists for this workflow)
2. `tool_count < 3` → skip (too trivial to codify)
3. `had_errors` → skip (don't codify a failed attempt)

If gate passes → ask LLM (as judge): "Given this conversation turn, should this be a reusable skill? If yes, provide name (kebab-case) and SKILL.md content."

### D4: LLM-as-judge for both skill creation and naming

A single LLM call serves as judge + generator:

- System prompt: "You are evaluating whether a completed task should become a reusable skill..."
- Returns JSON: `{"create": true/false, "name": "kebab-case-name", "description": "...", "body": "..."}`
- If `create: false`, no further action
- If `create: true`, validate name doesn't collide with existing registry, then register

**Collision handling:** If name already exists, append `-2`, `-3`, etc. (same pattern as UUID collision, extremely rare).

### D5: Scheduled task for periodic improvement (not heartbeat)

Register a scheduled task `skill-self-improve` on startup when `learning.enabled = true`. The task uses the existing `scheduled_tasks` infrastructure with a configurable cron expression (default: `0 */6 * * *` — every 6 hours).

When the task fires, it publishes a `TurnRequest` whose prompt instructs the agent to:

1. List all skills with `active_skill` trace data
2. For each with ≥N executions: check error rate and token trends
3. Run the `self-analyze` improvement logic inline for underperformers
4. Auto-apply the result (update `SkillRegistry` + write SKILL.md to disk)

**Why a scheduled task rather than a direct function call?** The scheduled task goes through the full orchestrator loop, meaning the agent has access to all tools, memory, and context. This makes the improvement step itself improvable — the agent can evolve how it self-improves.

### D6: Revert-on-regression safety

Add `previous_skill_md TEXT` column to `skill_refinements`. When auto-applying:

1. Store current body as `previous_skill_md`
2. Apply new body
3. After the next N executions (configurable, default 5): compare error rate
4. If error rate increased by >10pp → revert to `previous_skill_md`, mark refinement as `reverted`

The comparison window is small intentionally — we want fast feedback, not statistical significance. False reverts are cheap (the skill stays as-is); false keeps are expensive (degraded skill stays active).

### D7: Configuration

```toml
[learning]
enabled = true                        # master switch
auto_create_skills = true             # post-turn skill creation
auto_apply_refinements = true         # bypass /review for scheduled improvements
min_tool_calls_for_skill = 3          # gate: minimum tool calls to consider skill creation
min_executions_for_analysis = 10      # minimum traces before self-analyze runs
improvement_cron = "0 */6 * * *"      # how often the improvement task runs
error_rate_threshold = 0.2            # trigger improvement above this rate
revert_window = 5                     # executions to evaluate after applying
revert_regression_threshold = 0.1     # error rate increase that triggers revert
```

All defaults are conservative. `enabled = false` is the default in `LearningConfig::default()` so existing users aren't affected.

## Risks / Trade-offs

- **LLM cost for post-turn evaluation** → Mitigated by heuristic gate (most turns are filtered before the LLM call). Estimated: ~1 LLM call per 5-10 turns for active users.
- **Skill sprawl** → Many auto-created skills that are never used again. Mitigation: future garbage collection (out of scope for v1). The registry is just files on disk — not harmful.
- **Iceberg query performance** → Scanning Parquet for stats is slower than SQLite. Mitigation: cache results, only scan once per improvement cycle. The 6h interval makes this acceptable.
- **Auto-apply without human review** → Could degrade skills. Mitigation: revert-on-regression + `previous_skill_md` stored. Users can also set `auto_apply_refinements = false` to keep the `/review` gate.
- **LLM hallucinating skill names that collide** → Collision check + suffix appending handles this.

## Open Questions

- Should auto-created skills be marked with a `source: auto` frontmatter field to distinguish them from hand-authored ones?
- Should there be a maximum number of auto-created skills (cap) to prevent unbounded growth?
- Should the improvement task also consider token usage trends (not just error rates) for triggering refinement?
