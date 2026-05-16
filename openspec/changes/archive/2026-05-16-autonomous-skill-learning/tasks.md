## 1. Configuration & Types

- [x] 1.1 Add `LearningConfig` struct to `crates/core/src/types.rs` with all fields from design (enabled, auto_create_skills, auto_apply_refinements, min_tool_calls_for_skill, min_executions_for_analysis, improvement_cron, error_rate_threshold, revert_window, revert_regression_threshold)
- [x] 1.2 Add `learning: LearningConfig` field to `AssistantConfig` with `#[serde(default)]`
- [x] 1.3 Add `LearningConfig` to `crates/core/src/lib.rs` public exports
- [x] 1.4 Add `[learning]` example section to `config.toml` at repo root

## 2. Database Migrations

- [x] 2.1 Create migration adding `active_skill TEXT` column to `distributed_traces` table with index `idx_distributed_traces_active_skill ON distributed_traces(active_skill, start_time DESC)`
- [x] 2.2 Create migration adding `previous_skill_md TEXT` column to `skill_refinements` table
- [x] 2.3 Create migration adding `status` value support for 'reverted' and 'confirmed' in `skill_refinements` (update `RefinementStatus` enum in code)

## 3. Skill-Scoped Tracing (Orchestrator)

- [x] 3.1 Add `active_skill: Option<String>` field to `Orchestrator` struct
- [x] 3.2 In orchestrator dispatch, detect when `tool_name == "load-skill"` and store the skill name param in `self.active_skill`
- [x] 3.3 Pass `active_skill` to `start_tool_span()` — add it as a span attribute when `Some`
- [x] 3.4 Clear `active_skill` at turn boundaries (on turn start or when returning `TurnResult`)
- [x] 3.5 Write unit test: verify spans carry `active_skill` attribute after `load-skill` is called

## 4. SQLite Exporter Updates

- [x] 4.1 In `opentelemetry-exporter-sqlite/src/span.rs`, extract `active_skill` from span attributes (same pattern as `tool_name`)
- [x] 4.2 Add `active_skill` to the INSERT statement and bind it
- [x] 4.3 Write test: span with `active_skill` attribute persists to the column

## 5. Iceberg Exporter Updates

- [x] 5.1 In `opentelemetry-exporter-iceberg/src/span.rs`, extract `active_skill` from attributes JSON
- [x] 5.2 Add `active_skill` as a string column to the Arrow schema in `schema.rs`
- [x] 5.3 Add `active_skill` to the RecordBatch construction
- [x] 5.4 Write test: span with `active_skill` attribute written to Parquet column (covered by existing e2e test)

## 6. SkillStatsProvider Trait

- [x] 6.1 Define `SkillStatsProvider` trait in `crates/storage/src/traces.rs` with method `stats_for_active_skill(&self, skill_name: &str, window: i64) -> Result<TraceStats>`
- [x] 6.2 Implement `SkillStatsProvider` for `TraceStore` that queries `WHERE active_skill = ?`
- [x] 6.3 Implement `IcebergSkillStats` in `crates/web-ui/src/backends/iceberg.rs` that scans Parquet with a filter on `active_skill`
- [x] 6.4 Update `SelfAnalyzeHandler` to accept `Arc<dyn SkillStatsProvider>` instead of directly using `StorageLayer`
- [x] 6.5 Wire the correct provider implementation based on `ObservabilityConfig.exporter` at startup

## 7. Post-Turn Skill Creation

- [x] 7.1 Create `crates/runtime/src/skill_learner.rs` module with `spawn_post_turn_eval()` function
- [x] 7.2 Implement heuristic gate: check `active_skill`, `tool_count`, `had_errors` against config thresholds
- [x] 7.3 Implement LLM judge call: system prompt + conversation history → structured JSON response (create/name/description/body)
- [x] 7.4 Implement skill registration: write SKILL.md via SkillRegistry, handle name collisions with numeric suffix
- [x] 7.5 Hook `spawn_post_turn_eval()` into orchestrator turn completion (alongside `spawn_index`)
- [x] 7.6 Gate the hook behind `LearningConfig.enabled && LearningConfig.auto_create_skills`
- [x] 7.7 Write test: heuristic gate filters turns with <3 tool calls (+ active_skill, + errors)
- [x] 7.8 Write test: heuristic gate passes valid turn
- [x] 7.9 Write test: name collision resolution (no conflict case)

## 8. Periodic Skill Improvement (Scheduled Task)

- [x] 8.1 Create `crates/runtime/src/skill_improver.rs` module with `register_improvement_task()` and `run_improvement_cycle()` functions
- [x] 8.2 `register_improvement_task()`: check if "skill-self-improve" already exists in scheduled_tasks, if not insert with configured cron
- [x] 8.3 `run_improvement_cycle()`: list all skills, fetch stats via `SkillStatsProvider`, filter by min_executions and error_rate_threshold
- [x] 8.4 For each underperforming skill: run self-analyze logic (reuse prompt from `SelfAnalyzeHandler`), generate improved body
- [x] 8.5 Implement auto-apply: store `previous_skill_md`, update SKILL.md on disk, update SkillRegistry, set refinement status to "accepted"
- [x] 8.6 Call `register_improvement_task()` from orchestrator/bootstrap startup when `learning.enabled = true`
- [x] 8.7 Write test: improvement cycle identifies skill with high error rate and generates refinement
- [x] 8.8 Write test: improvement cycle skips skills below min_executions threshold

## 9. Revert-on-Regression

- [x] 9.1 Add `RefinementStatus::Reverted` and `RefinementStatus::Confirmed` variants to the enum
- [x] 9.2 In `run_improvement_cycle()`, before analyzing new skills, check recently-applied refinements that haven't been confirmed/reverted yet
- [x] 9.3 For each unconfirmed refinement: query post-apply stats (last `revert_window` executions), compare error rate to pre-apply baseline
- [x] 9.4 If regression exceeds threshold: revert SKILL.md from `previous_skill_md`, update registry, set status to "reverted"
- [x] 9.5 If no regression after `revert_window` executions: set status to "confirmed"
- [x] 9.6 Write test: regression detected → skill body reverted to previous
- [x] 9.7 Write test: no regression → status set to "confirmed"

## 10. Integration & Wiring

- [x] 10.1 Export new modules from `crates/runtime/src/lib.rs` (skill_learner, skill_improver)
- [x] 10.2 Wire `SkillStatsProvider` creation in CLI/webui startup based on exporter config
- [x] 10.3 Verify `make lint` and `make format` pass
- [x] 10.4 Run full `make test` — ensure no regressions
- [x] 10.5 Manual smoke test: enable learning, run a multi-tool conversation, verify skill is auto-created
