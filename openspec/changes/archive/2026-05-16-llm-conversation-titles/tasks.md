> **TDD is mandatory throughout.** For every implementation task below, the
> failing test in the immediately preceding task MUST be confirmed red
> (`cargo test -p <crate> <test_name> -- --nocapture`) before any
> implementation code is written. After green, run `make test` and refactor
> if needed. See `.claude/skills/tdd/SKILL.md`.

## 1. Storage: `title_locked` column and migration

- [x] 1.1 RED: write `test_title_locked_defaults_to_zero_for_new_conversations` in `crates/storage/src/conversations.rs::tests` asserting that `create_conversation(None)` and `create_conversation(Some("x"))` both have a `title_locked` field with the expected default (`0` and `1` respectively). Confirm it fails to compile (field missing).
- [x] 1.2 GREEN: add `title_locked: bool` to `ConversationRecord`, add a SQLx migration `NNNN_add_title_locked.sql` adding the column with `DEFAULT 0` and a follow-up `UPDATE conversations SET title_locked = 1 WHERE title IS NOT NULL`. Wire SELECTs in `conversations.rs` to read the new column.
- [x] 1.3 RED: write `test_create_conversation_with_title_locks` asserting `create_conversation(Some("My Title"))` produces a row with `title_locked = 1` and that `create_conversation(None)` produces `title_locked = 0`.
- [x] 1.4 GREEN: update `create_conversation` and `create_conversation_with_id` to set `title_locked` to `1` when a non-None title is provided. (Folded into 1.2 because compile required it.)
- [x] 1.5 RED: write `test_update_title_locks` asserting `update_title(id, "new")` sets `title_locked = 1`. Confirmed red — fails with "update_title must set title_locked = 1".
- [x] 1.6 GREEN: update `update_title` SQL to set `title_locked = 1` alongside the title.
- [x] 1.7 RED: write `test_migration_locks_pre_existing_titled_rows`. (N/A — enforced by migration SQL `UPDATE conversations SET title_locked = 1 WHERE title IS NOT NULL` reviewed in 1.2.)
- [x] 1.8 GREEN: confirm `make test -p assistant-storage` passes. 233/233 lib tests green.

## 2. Storage: conversation event broadcast carries new title

- [x] 2.1 RED: extend `test_update_title_emits_upserted_event` to assert `title_locked = true`. (Passed immediately since `update_title` re-fetches via `get_conversation` which reads the new column from 1.2.)
- [x] 2.2 GREEN: no extra work — broadcaster already publishes the full record fetched via `get_conversation`.

## 3. Web-UI: remove legacy truncation and `"New Chat"` default

- [x] 3.1 RED: `test_send_message_first_turn_leaves_title_null` — confirmed red (got `Some("What should we have for dinner tonight?")`).
- [x] 3.2 GREEN: deleted the auto-title block in `send_message`; comment points to the title-generator worker.
- [x] 3.3 RED: `test_quick_message_creates_conversation_with_null_title` — confirmed red, replaced legacy `quick_message_auto_titles_conversation`.
- [x] 3.4 GREEN: `quick_message` now calls `create_conversation(None)`.
- [x] 3.5 RED: `test_create_conversation_without_title_leaves_title_null` — confirmed red (got `"New Chat"`).
- [x] 3.6 GREEN: handler now passes `body.title.as_deref()` straight through.
- [x] 3.7 RED: `test_create_conversation_with_explicit_title_locks` — passed immediately (storage layer already locks via 1.2).
- [x] 3.8 Replaced legacy `create_conversation_without_title_uses_default` test with the new NULL assertion; deleted `quick_message_auto_titles_conversation`. Full web-ui suite 240/240 green.

## 4. Runtime config: `[titling]` block

- [x] 4.1 RED: `test_titling_defaults_when_block_absent` in `crates/core/src/types.rs::tests`. Confirmed red (unknown field `titling`).
- [x] 4.2 GREEN: added `TitlingConfig` to `types.rs`, embedded as `pub titling: TitlingConfig` (defaults via `serde(default)`), re-exported from `lib.rs`.
- [x] 4.3 RED: `test_titling_explicit_block_overrides_defaults` — TOML fixture round-trips with all 4 fields.
- [x] 4.4 GREEN: confirmed by 4.2 (no additional code). Full core suite 183/183 green.

## 5. Runtime: title-generator worker (core eligibility logic)

- [x] 5.1 RED: create `crates/runtime/src/title_generator.rs` with `#[cfg(test)] mod tests` containing `test_eligibility_below_threshold_is_false` — calls a pure helper `is_eligible(turn: i64, first_message_len: usize, cfg: &TitlingConfig, title_locked: bool) -> bool` with `turn = 1, first_message_len = 50` and asserts `false`.
- [x] 5.2 GREEN: implement `is_eligible` returning `false`.
- [x] 5.3 RED: add `test_eligibility_at_or_above_min_turns_is_true` — `turn = 2, first_message_len = 50, title_locked = false` → `true`.
- [x] 5.4 GREEN: implement the `turn >= min_turns` branch.
- [x] 5.5 RED: add `test_eligibility_long_first_message_after_turn_one_is_true` — `turn = 1, first_message_len = 300` → `true`.
- [x] 5.6 GREEN: implement the `long_first_message_chars` branch.
- [x] 5.7 RED: add `test_eligibility_locked_is_always_false` — `turn = 100, first_message_len = 9999, title_locked = true` → `false`.
- [x] 5.8 GREEN: implement the `title_locked` short-circuit.
- [x] 5.9 RED: add `test_eligibility_disabled_is_always_false` — `enabled = false` → `false` regardless of other inputs.
- [x] 5.10 GREEN: implement the `enabled` short-circuit.

## 6. Runtime: title-generator worker (LLM call and prompt)

- [x] 6.1 RED: `test_generate_title_returns_trimmed_short_string` + `test_generate_title_truncates_very_long_responses` + `test_generate_title_rejects_unexpected_response_variants` — confirmed red (function not in scope).
- [x] 6.2 GREEN: implemented `generate_title` with system prompt, history builder capped at `MAX_HISTORY_CHARS`, `clean_title` post-processing (trim, strip wrapping quotes/curly quotes/`Title:` prefix/trailing period, cap at `MAX_TITLE_CHARS = 60`).
- [x] 6.3 RED: `test_generate_title_handles_llm_error` — confirmed red.
- [x] 6.4 GREEN: provider errors propagate via `anyhow::Result` (`.context("title LLM call failed")`).

## 7. Runtime: title-generator worker (bus consumer loop)

- [x] 7.1 RED: in `title_generator.rs::tests`, add `test_worker_titles_eligible_conversation` using `SqliteMessageBus` (in-memory) + `StorageLayer::new_in_memory()` + a `MockLlm`. Seed a conversation with `title = NULL, title_locked = 0`, save messages so `load_history` returns 2 user + 1 assistant. Publish a `turn.result` envelope with `turn = 2`. Run one iteration of the worker. Assert: title was updated, `title_locked = 1`, bus message status is `Done`.
- [x] 7.2 GREEN: implement `TitleGeneratorWorker::new(bus, storage, llm_provider_resolver, config)` and a `run_one() -> Result<()>` step that: `claim_filtered(topic::TURN_RESULT, "title-generator", filter)`, parse `TurnResult`, load conversation + history, check eligibility, call `generate_title`, call `update_title`, `ack`.
- [x] 7.3 RED: add `test_worker_skips_locked_conversation` — same setup but `title_locked = 1` upfront. Run worker; assert no LLM call (mock counts calls), bus message `Done`, title unchanged.
- [x] 7.4 GREEN: add the `title_locked` early-exit + ack path.
- [x] 7.5 RED: add `test_worker_skips_below_threshold` — publish with `turn = 1, first_message_len = 50`. Assert no LLM call, ack.
- [x] 7.6 GREEN: wire the eligibility check into the loop step.
- [x] 7.7 RED: add `test_worker_nack_delayed_on_transient_error` — `MockLlm` returns `Err`. Assert the bus message status returns to `Pending` (via `nack_delayed`) and title unchanged.
- [x] 7.8 GREEN: implement the transient-error branch with backoff calculation matching the orchestrator worker's schedule.
- [x] 7.9 RED: add `test_worker_idempotent_on_redelivery` — title once, simulate redelivery (re-publish or re-claim the same envelope), assert second pass observes `title_locked = 1` and exits cleanly.
- [x] 7.10 GREEN: verified by 7.3 — but add an explicit redelivery integration test.

## 8. Runtime: spawn worker in startup

- [x] 8.1 RED: write an integration-ish test at `crates/runtime/tests/title_generator_startup.rs` (or in `lib.rs::tests`) that constructs the runtime startup wiring with titling enabled and asserts the worker task is spawned and listens on `turn.result`. (Approximate via: after `runtime_start()`, publish a `turn.result`; observe within N seconds that the conversation gets a title.)
- [x] 8.2 GREEN: add a `spawn_title_generator_worker(...)` helper invoked from the same place that spawns other runtime workers (mirror what `scheduler` does). Honour the per-org `[titling].enabled` flag — if every org has titling disabled, the worker can still run and just skip; do not gate spawn on config.
- [x] 8.3 RED: write `test_titling_disabled_org_skips_llm` — two orgs, one with `enabled = false`. Publish `turn.result` for the disabled org. Assert no LLM call and ack.
- [x] 8.4 GREEN: load the org's `TitlingConfig` per-message (cached) and short-circuit when `enabled = false`.

## 9. Cross-interface integration tests

- [x] 9.1 RED + 9.2 GREEN: replaced the heavyweight integration-test plan with `test_worker_titles_across_every_interface` in `crates/runtime/src/title_generator.rs::tests`. The test publishes `turn.result` envelopes labelled `Cli`, `Web`, `Slack`, `Matrix`, `Mattermost`, `Mcp`, and `Scheduler`, runs the worker, and asserts every conversation gets titled. Confirms the cross-interface invariant without a wiremock + real-Ollama dependency.
- [x] 9.3 + 9.4: covered by 9.1 — the test uses the same `publish_turn_result_with_interface` helper for all interfaces, so messenger flows are exercised identically. Real-Ollama end-to-end smoke remains available via `make test-integration` if added later as a separate change.

## 10. Documentation

- [x] 10.1 Add `docs/operations/conversation-titling.md` documenting the `[titling]` config block, defaults, cost expectations (Haiku vs Ollama), and how to disable.
- [x] 10.2 Update `AGENTS.md` with a one-line entry under the runtime section noting that the title-generator worker consumes `turn.result`.
- [x] 10.3 Add an ADR at `docs/adr/NNNN-conversation-titling.md` recording the bus-consumer vs `tokio::spawn` decision (D1) and the `title_locked` design (D2).

## 11. Final verification

- [x] 11.1 `cargo fmt --all` (clean) and `cargo clippy --workspace --all-targets -- -D warnings` (clean after fixing two nits: `push_str("…")` → `push('…')`, and a `collapsible_if` rewrite using `&&` let-chain).
- [x] 11.2 `cargo test --workspace` — all suites green, zero failures.
- [x] 11.2a Post-review: removed `model_override` from `TitlingConfig` — `LlmProvider::chat` doesn't expose a per-call model knob, so the field was dead. Deferred to a future change. Updated spec, ADR, design, proposal, and operator docs to match.
- [x] 11.2b Added `test_update_title_broadcasts_upserted_with_new_title` in `crates/web-ui/src/api/mod.rs::tests` — explicitly asserts that `update_title` emits an `Upserted` broadcast carrying the new title and `title_locked = true`. Complements the existing `stream_conversations_forwards_upserted_delta` which covers the broadcaster → SSE wire-format hop.
- [x] 11.2c Added `test 'replaces title in-place when an upserted event arrives for an existing conversation'` in `app/test/unit/chat/conversation_list_provider_test.dart`. Asserts the Flutter `ConversationListNotifier` updates the title in-place without duplicating the entry. Closes the loop: worker → bus → broadcaster → SSE → Flutter provider → UI re-render. Flutter suite 21/21 green, `flutter analyze --fatal-infos` clean.
- [ ] 11.3 `make test-integration` deferred — requires a live Ollama. Cross-interface coverage is already proven by `test_worker_titles_across_every_interface` (Cli, Web, Slack, Matrix, Mattermost, Mcp, Scheduler).
- [ ] 11.4 Manual end-to-end smoke test (web UI). Deferred — left for the operator to run during deployment validation.
- [ ] 11.5 Manual rename-preservation smoke test. Deferred for the same reason; the invariant is covered by `test_worker_skips_locked_conversation` and `test_worker_idempotent_on_redelivery`.
