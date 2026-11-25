# Tasks: message-feedback

This change is a stack of 8 PRs (1, 2a, 2b, 3, 4, 5, 7a, 7b; the former PR 6 client regeneration is folded into 2a and 2b). Each stays under about 500 changed lines, excluding generated `app/packages/assistant_api` code, and leaves `main` shippable. Every PR starts with a failing test (TDD) and runs `make lint` and `make format` before review.

**Cross-change rules (binding):**

- **Migrations.** Use the numbers pre-allocated in `openspec/chat-ui-parity-stack.md` (`050_message_feedback` for PR 1, `057_message_traceparent` for PR 3); the stack plan is authoritative, and if it is silent, take the next free number on `main` at implementation time. Register each migration in the `run_migrations` list (`crates/storage/src/lib.rs`). `seed_database_through_016` (`lib.rs:562`) is a test fixture that stops at 016 and is not changed.
- **`messages` / `Message` lane.** Order: message-branching PR 1, then tool-result-renderers S3, then this change's PR 3, then message-branching PR 8. PR 3 introduces `persist_final_answer(…, FinalAnswerMeta { traceparent })`; chat-a11y S2 (timing) and message-branching PR 8 (model) extend `FinalAnswerMeta` instead of editing each save site.
- **Generated code.** Never hand-merge `openapi.json` or `app/packages/assistant_api`. After a rebase, rerun `make dump-openapi && make generate-flutter-client`. Regeneration ships in the PR that changes the route.
- **Screenshot baselines.** Never merge PNGs by hand. After rebasing, regenerate the baselines (`e2e-testing` skill) and record the diff in the PR body.
- **ADR.** PR 4 amends ADR-0012 (no new ADR number is taken).
- **Ship order (final decision).** PR 7b, which makes the thumbs visible, merges only after message-branching's non-destructive compaction PR (its D5, which deletes `replace_history`) has merged. In message-branching's current `tasks.md` that is PR 5; the stack plan numbers it differently, so match by title. PR 1 to PR 5 and PR 7a may merge earlier because nothing in the app calls them.
- **Comment text never leaves `space.db`.** No task adds an exported comment attribute, and PR 4 carries a test that guards it.

## Parallelism

```
server:
  PR 1 (storage) ──┬── PR 2a (PUT/DELETE + regen) ──┐
                   └── PR 2b (history + regen)      ├── PR 4 (telemetry)
  message-branching PR 1 → tool-result-renderers S3 → PR 3 (traceparent) ──┘

client:
  PR 2a + PR 2b ─────────────────────────── PR 7a (model + controller, no UI) ──┐
  chat-element-library PR 3 + PR 8a ─────── PR 5 (elements) ───────────────────┤
  chat-element-library PR 8b ──────────────────────────────────────────────────┼── PR 7b (wiring, docs, ship)
  message-branching non-destructive compaction (D5) ───────────────────────────┘
```

- **Start immediately:** PR 1 (server-only).
- After PR 1: PR 2a and PR 2b in parallel.
- PR 3 starts once message-branching PR 1 and tool-result-renderers S3 have merged; it runs in parallel with PR 1, 2a, 2b and 5.
- PR 4 follows PR 2a and PR 3, in parallel with PR 5, 7a and 7b.
- PR 5 starts once chat-element-library PR 3 and PR 8a have merged, in parallel with every server PR.
- PR 7a follows PR 2a and PR 2b. PR 7b follows PR 5, PR 7a, chat-element-library PR 8b and message-branching's non-destructive compaction PR (D5), which gates the visible UI. Neither needs PR 4: telemetry is invisible to the client.

---

## PR 1 — feat(storage): message feedback store

Layer: server
Depends on: none
Parallel with: PR 3, PR 5

- [ ] 1.1 RED: add `crates/storage/tests/contract/message_feedback_store.rs`, registered in `tests/contract.rs`, running against both impls:
  - set then get;
  - set twice keeps `created_at` and advances `updated_at` (with a fake `Clock`);
  - two concurrent sets on the same pair both succeed and leave one row whose `created_at` is the first writer's;
  - `clear` returns `true` and removes the row; a second `clear` returns `Ok(false)`;
  - `list_for_conversation(conv, user)` returns only that user's rows;
  - SQLite only: deleting the conversation cascades; `set` on an unknown message id returns an error (in-memory exemption documented in design.md D1).
- [ ] 1.2 Add migration `message_feedback.sql` (design.md D1; number from the stack plan, 050) and register it in `run_migrations` (`crates/storage/src/lib.rs:297`).
- [ ] 1.3 Add `crates/storage/src/message_feedback.rs` with `MessageFeedbackRating { Positive, Negative }`, `MessageFeedbackRow`, the `MessageFeedbackStore` trait (`set`, `clear -> Result<bool>`, `get`, `list_for_conversation`). `SqliteMessageFeedbackStore::set` is one `INSERT … ON CONFLICT (message_id, user_id) DO UPDATE` that never updates `created_at`. Add `InMemoryMessageFeedbackStore`; both take `with_clock`, following `command_events.rs`. Export from `lib.rs`, add `StorageLayer::message_feedback_store()`, and re-export through `assistant_test_support::prelude` (`crates/test-support/src/prelude.rs:28`). Confirm GREEN.
- [ ] 1.4 Panic-free check: no `unwrap`/`expect` outside `#[cfg(test)]`; `cargo clippy -p assistant-storage -D warnings`.

## PR 2a — feat(web-ui): message feedback PUT and DELETE endpoints

Layer: both (server route + generated client)
Depends on: PR 1
Parallel with: PR 2b, PR 3, PR 5

- [ ] 2a.1 RED: handler tests in `crates/web-ui/src/api/feedback.rs` using `test_helpers::seed_conversation` (`test_helpers.rs:248`). Every non-2xx case asserts the `{"error"}` JSON body:
  - PUT positive → 200 with no `comment` field;
  - PUT replace with comment → 200 and the stored comment;
  - a whitespace-only comment is stored as none;
  - a 2001-scalar comment → 400; a 2000-scalar comment of multi-scalar emoji → 200;
  - `{"rating":"meh"}` → 400; malformed JSON → 400; a body without `Content-Type: application/json` → 400;
  - a malformed `{id}` and a malformed `{message_id}` → 400;
  - a user message, a `<think>` row, and an empty assistant row with `tool_calls_json` → 400;
  - a foreign conversation → 404; a message from another conversation → 404;
  - viewer → 403 on PUT and on DELETE;
  - non-admin without `conversations:write` → 403; org admin without the scope → 200;
  - DELETE → 204 twice.
- [ ] 2a.2 Add `owned_conversation_json` beside `owned_conversation` (`conversations.rs:50`), mapping its errors through `json_error` (`crates/web-ui/src/errors.rs:25`).
- [ ] 2a.3 Implement `set_message_feedback` and `delete_message_feedback` (design.md D2) with `SetMessageFeedbackRequest`, `MessageFeedbackResponse` and the rating enum (`#[serde(rename_all = "snake_case")]`, `ToSchema`). Take the body as `Result<Json<_>, JsonRejection>` and map **every** rejection to `json_error(StatusCode::BAD_REQUEST, rejection.body_text())`; do not use `From<JsonRejection> for ApiError` (`integrations/error.rs:79-82`, which yields 422/415). Use `Access::Write` plus `caller_can_post` (`messages.rs:43-50`). Count the comment with `chars()`. Register both routes in `api_router` (`crates/web-ui/src/api/mod.rs:238`) and in the `OpenApi` paths and schemas (`crates/web-ui/src/openapi.rs:276`, `:438`), documenting 400/403/404 with `ErrorBody`. Confirm GREEN.
- [ ] 2a.4 Run `make dump-openapi` and `make generate-flutter-client`. Commit `openapi.json` and the generated diff (generated-only: request, response and rating enum). Verify the `openapi.rs` X-Space-Id test (`openapi.rs:831`) passes and `flutter analyze --fatal-infos` is green.

## PR 2b — feat(web-ui): project caller feedback into conversation history

Layer: both (server projection + generated client)
Depends on: PR 1
Parallel with: PR 2a, PR 3, PR 5

- [ ] 2b.1 RED: a `get_conversation` test. A rated assistant message carries `feedback {rating, comment, updated_at}`. Unrated messages omit the key. Another user's vote is not visible.
- [ ] 2b.2 Add `MessageFeedbackSummary` and `feedback: Option<_>` (`skip_serializing_if`) to `MessageSummary` (`conversations.rs:105`). Load it once per request with `list_for_conversation` into a `HashMap`, next to the attachments preload (`conversations.rs:424`). Register the schema in `openapi.rs`. Confirm GREEN.
- [ ] 2b.3 Run `make dump-openapi` and `make generate-flutter-client`; commit both (generated-only: `MessageFeedbackSummary`, `MessageSummary.feedback`).

## PR 3 — feat(runtime): record LLM span traceparent on assistant replies

Layer: server
Depends on: message-branching PR 1, tool-result-renderers S3 (`Message` lane order)
Parallel with: PR 1, PR 2a, PR 2b, PR 5

- [ ] 3.1 RED, in `crates/storage`: saving a `Message` with `traceparent: Some(..)` round-trips through `load_history`, `get_message` and the third reader, for both `ConversationStore` impls (`contract/conversation_store.rs`).
- [ ] 3.2 Add migration `message_traceparent.sql` (`ALTER TABLE messages ADD COLUMN traceparent TEXT`; number from the stack plan, 057), registered in `run_migrations`. Add `traceparent: Option<String>` to `assistant_core::types::conversation::Message` (`crates/core/src/types/conversation.rs:33`, `None` in `Message::new`). In the SQLite impl, update the INSERT (`crates/storage/src/conversations.rs:422`) and the three SELECT column lists with their row mappers (`:461`/`:486`, `:496`/`:519`, `:531`/`:559`); update the in-memory impl (`:772`). Confirm GREEN.
- [ ] 3.3 RED, unit tests in `crates/runtime/src/otel_spans.rs` with **local** providers only: `traceparent_from_span` on a span from a local `SdkTracerProvider` returns `00-{trace_id}-{span_id}-01`; on a no-op span returns `None`.
- [ ] 3.4 Implement `traceparent_from_span` formatting directly from `span.span_context()` (no global propagator). Confirm GREEN.
- [ ] 3.5 RED, process-isolated integration tests: `crates/runtime/tests/final_answer_traceparent.rs` installs a global `InMemorySpanExporter` tracer provider (dev feature `testing`, `crates/runtime/Cargo.toml:49`) and, with `ScriptedLlmProvider`, asserts the saved final-answer row's `traceparent` matches the exported `chat {model}` span. `crates/runtime/tests/final_answer_traceparent_disabled.rs` installs nothing and asserts `None`. One test per file.
- [ ] 3.6 Add `FinalAnswerMeta { traceparent: Option<String> }` and `persist_final_answer(conv_store, msg, meta)`. Route every final-answer save site through it: `orchestrator/mod.rs:1330`, `turn_control.rs:47` and `:70`, `subagent.rs:323`. Add a `meta: FinalAnswerMeta` parameter to `handle_final_answer_with_extensions` (`turn_control.rs:31-42`) and pass the chat span's traceparent from its callers. Confirm GREEN.

## PR 4 — feat(runtime): emit gen_ai.evaluation.result for user feedback

Layer: server
Depends on: PR 2a, PR 3
Parallel with: PR 2b, PR 5, PR 7a, PR 7b

- [ ] 4.1 RED: plain `#[test]` unit tests in `crates/runtime/src/otel_spans.rs` using a **local** `SdkLoggerProvider` with `InMemoryLogExporter`, wired through `OpenTelemetryTracingBridge` under `tracing::subscriber::with_default` (no global state). `record_user_feedback` with a stored traceparent exports one record with:
  - `event_name` `gen_ai.evaluation.result` and the traceparent's trace and span id;
  - `score.label` `negative` and `score.value` -1.0;
  - `gen_ai.conversation.id`, `gen_ai.agent.id`, `assistant.message.id`, `assistant.feedback.has_comment`;
  - no `gen_ai.evaluation.explanation`, and no attribute value contains the comment text.
    Further cases: a rating with a comment sets `has_comment` true and exports no comment text for either `positive` or `negative`; a rating without a comment sets `has_comment` false; `cleared` → no `score.value` and a `cleared` label; no traceparent → no trace context. If the bridge does not set `event_name` or trace context, apply the fallback from design.md D3.
- [ ] 4.2 Implement `FeedbackTelemetry { …, has_comment }` (no comment field, so comment text cannot reach the exporter) and `record_user_feedback`, parsing the stored value with a local `TraceContextPropagator::new()`. Attribute names are constants next to `GEN_AI_*` (`otel_spans.rs:28`). Add `assistant.feedback.count{rating, agent_id}` to `MetricsRecorder` (`metrics.rs:26`) with a unit test. Confirm GREEN.
- [ ] 4.3 RED: web-ui handler tests with a recording `FeedbackTelemetrySink` on `ApiState`. PUT calls the sink with `positive`/`negative` and the stored traceparent. DELETE of an existing vote calls it with `cleared`. DELETE with nothing stored does not call it. A sink failure still returns 200 or 204. The recorded call carries `has_comment` and no comment text, with and without a comment in the request.
- [ ] 4.4 Add `OtelFeedbackTelemetrySink` in runtime, which owns its own `MetricsRecorder::new()` and calls `record_user_feedback`. Wire the handlers: read the message's `traceparent` and emit only when `set` succeeded or `clear` returned `true`. Confirm GREEN.
- [ ] 4.5 Amend `docs/adr/adr-0012-otlp-only-telemetry.md` with a dated section: persisted domain rows may carry a W3C traceparent so later out-of-band events are emitted as span-parented log records; list the alternatives from design.md D3.
- [ ] 4.6 Document the event, its attributes, the `cleared` semantics (no score value), that comment text is never exported (only `has_comment`) and the metric (counts events, `rating` values) in `docs/opentelemetry.md` and the "What is emitted" list of `.claude/skills/observability/SKILL.md`.

## PR 5 — feat(app): FeedbackButtons and FeedbackCommentField elements

Layer: client
Depends on: chat-element-library PR 3 (widgetbook registry) and PR 8a (`MessageActionButton.selected`, list `trailing`)
Parallel with: PR 1 to PR 4, PR 7a

- [ ] 5.1 RED: `app/packages/assistant_ui/test/feedback_buttons_test.dart`, pumped with literal arguments and no `ProviderScope`:
  - none → tap positive emits `positive`;
  - negative → tap negative emits `null`;
  - positive → tap negative emits `negative`;
  - semantics: selected on "Good response" only;
  - disabled → no callback;
  - keys `feedback_positive_action` and `feedback_negative_action` present.
- [ ] 5.2 Add `MessageRating` to the package models and implement `FeedbackButtons` from `MessageActionButton` (icons `thumb_up_outlined`/`thumb_up` and `thumb_down_outlined`/`thumb_down`). Confirm GREEN.
- [ ] 5.3 RED: `test/feedback_comment_field_test.dart`:
  - Send trims and fires `onSubmit`;
  - Send is disabled while the field is empty and while `sending` is true;
  - Enter submits; Escape fires `onDismiss`;
  - the limit counts Unicode scalar values (2000 multi-scalar emoji scalars accepted, one more rejected);
  - `initialText` prefills.
- [ ] 5.4 Implement `FeedbackCommentField` with a runes-counting input formatter. Confirm GREEN.
- [ ] 5.5 Follow the chat-element-library element checklist: `kAssistantUiElements` entries; widgetbook use-cases (FeedbackButtons none, positive, negative, disabled; comment field empty, prefilled, sending); `golden`-tagged goldens in light and dark; `ELEMENTS.md` rows; the boundary test still green, with no new dependency.

## PR 7a — feat(app): message feedback model and controller

Layer: client
Depends on: PR 2a, PR 2b
Parallel with: PR 4, PR 5

- [ ] 7a.1 RED: `app/test/unit/chat/chat_history_mapping_test.dart`. `MessageSummary.feedback` maps to `ChatMessage.feedback` (rating, comment); absent maps to `null`.
- [ ] 7a.2 Add `MessageFeedback` and `ChatMessage.feedback` (`chat_models.dart:77`, or its package location after chat-element-library PR 4). Map it in `chatMessagesFromHistory` (`chat_history_mapping.dart:32`). Confirm GREEN.
- [ ] 7a.3 RED: `app/test/unit/chat/message_feedback_provider_test.dart` with a fake API:
  - success: a set applies optimistically, then calls PUT; `confirmed` updates;
  - loading: while the call is in flight `isPending(id)` is true, and false after it settles;
  - error: a failure restores the rating and comment and reports an error;
  - a re-tap of the same rating calls DELETE;
  - two quick toggles are applied in order, ending with no feedback;
  - rollback under the queue: confirmed `positive`, tap negative, clear, negative fails → the clear is never sent and the state is `positive`;
  - pending beats history: a `loadConversation` or stall-recovery list landing while a PUT is in flight keeps the optimistic value;
  - stale callbacks: after switching conversation, a failure changes nothing visible and reports no error;
  - a comment submit PUTs `{rating, comment}` for `positive` and for `negative`;
  - switching from `negative` with a stored comment to `positive` PUTs `{rating: positive}` without a comment, and the comment-field state resets;
  - dismissing the field sends nothing.
- [ ] 7a.4 Implement `MessageFeedbackController` in `features/chat/message_feedback_provider.dart` (per-message `confirmed` plus serial queue), `ChatNotifier.setMessageFeedback(id, feedback)`, and one `_withPendingFeedback(messages)` merge applied at every list-replacement site in `chat_provider.dart` (`loadConversation` `:290-313`, stall recovery `:975-994`, reconnect fallback `:1078-1095`). Confirm GREEN.

## PR 7b — feat(app): show message feedback in the chat thread

Layer: client
Depends on: PR 5, PR 7a, chat-element-library PR 8a and PR 8b, message-branching non-destructive compaction PR (D5; gates shipping the UI)
Parallel with: PR 4

- [ ] 7b.1 RED: a unit test for a pure `buildMessageTrailing(message, …) → List<Widget>` in `features/chat/message_actions_builder.dart` (beside chat-element-library's `buildMessageActions`). `FeedbackButtons` appear for a persisted assistant message entry, after any timing indicator (chat-a11y C11) when present. They are absent for user, streaming, failed and empty-content messages, for non-message timeline entries, and for an id starting `assistant-` (`chat_provider.dart:206`). If chat-a11y C11 has not landed, this PR creates the builder and C11 prepends to it.
- [ ] 7b.2 Implement `buildMessageTrailing`, feed it to `MessageActions.trailing`, and render `FeedbackCommentField` (with `sending: isPending(id)`) below `MessageActions` after a vote of either rating. Switching rating reopens it empty; clearing closes it. Open state is local to the chat feature, keyed by message id. Confirm GREEN.
- [ ] 7b.3 RED, then GREEN: a widget test on `ChatScreen` with a seeded history vote. The positive button renders selected. Tapping negative opens the comment field, and submitting closes it. Tapping positive on another message also opens it, and dismissing keeps the rating. A server error shows a snackbar and restores the selection.
- [ ] 7b.4 Regression guard (passes once the dependency has merged, fails if destructive compaction returns): a runtime test that rates a message through `MessageFeedbackStore`, runs `maybe_compact` over the conversation, and asserts the vote still exists on the same message id.
- [ ] 7b.5 Regenerate the Playwright baselines touched by the trailing buttons (`e2e-testing` skill) after rebasing; record the visual diff in the PR body.
- [ ] 7b.6 Add a "Rating replies" section to `docs/web-ui.md`: thumbs, the optional comment after either vote, clearing a vote, and that comment text stays in the space database and is never exported to telemetry (only whether a comment exists).
- [ ] 7b.7 Final phase: `make lint && make format && make test`, then `make lint-flutter && make test-flutter`.
- [ ] 7b.8 After all PRs have merged, archive the change (`openspec archive message-feedback`), promoting `message-feedback` and `message-feedback-ui` into `openspec/specs/`.
