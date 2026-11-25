# Design: message-feedback

## Context

- **Messages.** `messages(id, conversation_id, role, content, skill_name, turn, created_at, tool_calls_json, sender_user_id)` (`migrations/001_conversations.sql:10`, `040_message_sender.sql`). Foreign keys are enforced on every pooled connection because sqlx's `SqliteConnectOptions` enables `foreign_keys` per connection by default; the pool is opened with `SqlitePool::connect(&url)` (`crates/storage/src/lib.rs:143`). The explicit `PRAGMA foreign_keys=ON` at `lib.rs:277` runs on one connection only and is not what makes the cascade work.
- **Ownership.** A conversation belongs to one user. `caller_store` scopes the store to `ctx.auth.user_id` (`crates/web-ui/src/api/conversations.rs:40-44`). `owned_conversation` returns 404 for a conversation owned by someone else, but with plain-text bodies (`"Conversation not found"`, `"Database error"`, `conversations.rs:54-60`). Sibling handlers also reject a malformed path UUID with plain text (`conversations.rs:409`).
- **Assistant rows.** An assistant turn can write several `role = assistant` rows:
  - thinking rows `<think>…</think>` (`orchestrator/mod.rs:1112-1116`, `:1453-1455`);
  - empty-content tool-call rows with `tool_calls_json` (`dispatch.rs:65`);
  - the final-answer row (`orchestrator/mod.rs:1330`, `turn_control.rs:47,70`);
  - the error-recovery row `"[An error occurred processing this message.]"` (`history.rs:208-214`).
    The `done` SSE frame names the final row's id (`messages.rs:392-424`).
- **History.** `get_conversation` builds `MessageSummary` from `load_history` and drops `System` and `Tool` rows (`conversations.rs:401-526`, struct at `:105`). Attachments are preloaded once into a `HashMap` (`conversations.rs:424-439`).
- **Traces.** Each turn runs under `interface.submit_turn`, then `turn`, then `chat {model}` (`worker.rs:747`, `orchestrator/mod.rs:1644-1671`, `:1241`). `start_llm_span` uses `global::tracer`. `traceparent_from_context` and `context_from_traceparent` go through `global::get_text_map_propagator` (`otel_spans.rs:86-102`), which is a no-op until `telemetry::init` installs `TraceContextPropagator`, and that happens only when OTLP export is enabled (`telemetry.rs:98-107`).
- **Telemetry pipeline.** Logs reach OTLP only through `OpenTelemetryTracingBridge` (`telemetry.rs:136-137`). No global logger provider is registered. No test in `crates/` uses `InMemorySpanExporter` or `InMemoryLogExporter` yet; runtime has `opentelemetry_sdk` `testing` as a dev-dependency (`crates/runtime/Cargo.toml:49`). `crates/runtime/tests/` already holds process-isolated telemetry tests (`otlp_sdk_disabled.rs`, `otlp_unconfigured.rs`).
- **Orchestrator internals.** `Orchestrator.trace_content` is `pub(crate)` (`orchestrator/mod.rs:218`). `MetricsRecorder` is created from the global meter and stored on the Orchestrator (`metrics.rs:23-26`, `orchestrator/mod.rs:233`). web-ui reaches the space's orchestrator through `state.runtime(&ctx)` (`messages.rs:148`).
- **Client.** `ChatMessage` (`app/lib/features/chat/chat_models.dart:77`) is mapped from history in `chat_history_mapping.dart:32`. Live replies take `event.messageId` or fall back to `assistant-<ts>` (`chat_provider.dart:206`). `ChatNotifier` replaces the whole message list on `loadConversation` (`chat_provider.dart:290-313`), on stall recovery (`:975-994`) and on reconnect fallback (`:1078-1095`). message-branching reloads the detail after every terminal frame (`message-branching/specs/chat-branch-ui/spec.md`).

## Decision 1: One row per (message, user), replaced in full by an atomic upsert

```sql
CREATE TABLE message_feedback (
    message_id  TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL,
    rating      TEXT NOT NULL CHECK (rating IN ('positive', 'negative')),
    comment     TEXT,
    created_at  DATETIME NOT NULL,
    updated_at  DATETIME NOT NULL,
    PRIMARY KEY (message_id, user_id)
);
CREATE INDEX idx_message_feedback_user ON message_feedback (user_id);
```

- `set` is a single `INSERT … ON CONFLICT (message_id, user_id) DO UPDATE SET rating = excluded.rating, comment = excluded.comment, updated_at = excluded.updated_at`. `created_at` is never in the update list, so concurrent PUTs keep the first `created_at`. No read-modify-write.
- `clear` returns `Result<bool>`: `true` when a row was removed. The handler emits telemetry only on `true` (D3).
- `rating` is an enum rather than a number, which keeps room for other values without reinterpreting the column.
- Timestamps come from the injected `Clock`, following `command_events.rs:64`.
- **Unknown message id.** `set` on a message id that does not exist returns an error from the SQLite impl (FK violation). The in-memory impl cannot see `messages` and accepts it; this is a documented exemption, and the contract test for this case runs against the SQLite impl only (like the cascade case). The handler checks the message before calling `set`, so production behaviour never depends on the store for this.
- Keying on `user_id` means shared conversations, if they ever arrive, need no migration.

**Alternatives:**

- Columns on `messages`. Rejected: not per user, and it mixes product judgement into the transcript row.
- An append-only feedback log. Rejected: the toggle means latest state, and the OTLP events already form the history.

## Decision 2: REST shape follows the sibling `messages/{message_id}/<verb>` paths

| Method & path                                                   | operationId               | Body                                             | Response                      |
| --------------------------------------------------------------- | ------------------------- | ------------------------------------------------ | ----------------------------- |
| `PUT /api/conversations/{id}/messages/{message_id}/feedback`    | `set_message_feedback`    | `SetMessageFeedbackRequest { rating, comment? }` | 200 `MessageFeedbackResponse` |
| `DELETE /api/conversations/{id}/messages/{message_id}/feedback` | `delete_message_feedback` | none                                             | 204                           |

- **`PUT` is a full replace.** Sending `{rating}` without `comment` clears the comment.
- **Path.** It nests under the conversation, like message-branching's `/edit` and `/regenerate`.
- **Error envelope everywhere.** Every error response from both handlers goes through `json_error` (`crates/web-ui/src/errors.rs:25`):
  - a malformed `{id}` or `{message_id}` UUID → `json_error(400, …)`, not the plain-text pattern at `conversations.rs:409`;
  - the ownership check calls `owned_conversation` and maps its `Err` to `json_error(404, "conversation not found")` or `json_error(500, …)`. PR 2a adds `owned_conversation_json`, an enveloped variant beside the existing helper, rather than rewriting the helper's other callers in this change;
  - **every** `JsonRejection` (data error such as an unknown enum variant, syntax error, missing or wrong `Content-Type`) maps explicitly to `json_error(StatusCode::BAD_REQUEST, rejection.body_text())`. We do not use `From<JsonRejection> for ApiError` (`integrations/error.rs:79-82`), which keeps `rejection.status()` and therefore returns 422 for a data error and 415 for a missing content type.
- **Eligibility.** The message must belong to the conversation (otherwise 404). It must be a final-answer assistant row: `role = assistant`, non-empty trimmed content, `tool_calls_json` is `NULL`, and content does not start with `<think>`. Anything else returns 400. The error-recovery row is a visible reply and stays eligible. When message-branching or tool-result-renderers add an explicit final-answer marker, the predicate switches to it.
- **Comment.** Trimmed; empty means absent. The limit is 2000 **Unicode scalar values** (`str::chars().count()` in Rust). The client counts the same unit (`String.runes.length`), so the two sides agree on emoji and combining marks.
- **Authorization.** `Access::Write` plus `caller_can_post` (`messages.rs:43-50`), which allows org admins without the scope. Viewers get 403 on both PUT and DELETE.
- **Response.** `MessageFeedbackResponse { message_id, rating, comment?, created_at, updated_at }`. History embeds `MessageFeedbackSummary { rating, comment?, updated_at }` as `MessageSummary.feedback`, omitted when absent, loaded with one `list_for_conversation(conv_id, user_id)` query into a `HashMap` beside the attachments preload.
- **No SSE event.** Feedback is not part of a run. The client owns optimistic state (D4).

**Alternatives:**

- A flat `POST /api/messages/{id}/feedback` (`/api/messages/{id}/audio`, `api/mod.rs:280`, as precedent). Rejected: needs a message-to-conversation lookup, and siblings nest.
- `POST` plus `PATCH` for the comment. Rejected: two calls for one state change.
- Returning 422 for a bad body, to match `integrations/policy.rs:53`. Rejected: `openspec/config.yaml` and the other conversation routes use 400 for client input errors.

## Decision 3: Telemetry is a `gen_ai.evaluation.result` event in the reply's trace

- **Spec status.** semantic-conventions-genai v0.1.0 (Development) defines `gen_ai.evaluation.result` with `gen_ai.evaluation.name` (Required), `gen_ai.evaluation.score.label` and `gen_ai.evaluation.score.value` (Conditionally Required), and `gen_ai.evaluation.explanation` and `gen_ai.response.id` (Recommended). It SHOULD be parented to the evaluated GenAI span.
- **Capturing the span context without globals.** `traceparent_from_span(&impl Span) -> Option<String>` formats directly from `span.span_context()` as `00-{trace_id}-{span_id}-{flags:02x}` and returns `None` when the context is invalid. It does not use the global propagator, which is a no-op unless OTLP is enabled.
- **Parsing without globals.** `record_user_feedback` parses the stored value with a local `TraceContextPropagator::new()`, not `context_from_traceparent`.
- **Persisting.** PR 3 introduces one helper, `persist_final_answer(conv_store, msg, FinalAnswerMeta { traceparent })`, used by every final-answer save site (`orchestrator/mod.rs:1330`, `turn_control.rs:47,70`, `subagent.rs:323`). `handle_final_answer_with_extensions` (`turn_control.rs:31-42`) gains a `meta: FinalAnswerMeta` parameter carrying the `Option<String>` traceparent. chat-a11y S2 (timing) and message-branching PR 8 (model) extend `FinalAnswerMeta` instead of each editing every save site.
- **Emitting.** `record_user_feedback(FeedbackTelemetry { … })` in `otel_spans.rs`:
  1. builds a context from the stored traceparent with the local propagator and attaches it;
  2. emits `tracing::info!(name: "gen_ai.evaluation.result", target: "gen_ai.evaluation.result", …)`. The bridge turns it into a log record, and the SDK logger fills its trace context from the attached context.
- **Attributes:** `gen_ai.evaluation.name = "user_feedback"`; `gen_ai.evaluation.score.label` ∈ {`positive`, `negative`, `cleared`}; `gen_ai.evaluation.score.value` = 1.0 or −1.0, **omitted on `cleared`** so that averaging backends do not count a clear as a neutral vote; `gen_ai.conversation.id`; `gen_ai.agent.id`; `assistant.message.id`; `assistant.feedback.has_comment`; `gen_ai.evaluation.explanation` (the comment) only when `trace_content` is on. We do not set `gen_ai.response.id` from our message UUID.
- **When to emit.** After every successful PUT, and after a DELETE only when `clear` returned `true`. An idempotent DELETE with nothing stored emits nothing.
- **Metric.** `assistant.feedback.count`, a `u64` counter with `rating` ∈ {`positive`, `negative`, `cleared`} and `agent_id`. It counts feedback **events**, not current votes; current state is the latest event per `assistant.message.id`.
- **Ownership in web-ui.** `ApiState` holds `Arc<dyn FeedbackTelemetrySink>`. The production sink, `OtelFeedbackTelemetrySink` in runtime, owns its own `MetricsRecorder::new()` (built from the global meter, same as the Orchestrator's) and calls `record_user_feedback`. `trace_content` comes from a new public accessor `Orchestrator::trace_content() -> bool` on the space runtime's orchestrator, passed in `FeedbackTelemetry`.
- **Test isolation.** Global OTel state is process-wide and unit tests run in parallel, so:
  - `traceparent_from_span` and `record_user_feedback` are unit-tested with **local** providers only: a local `SdkTracerProvider` for a valid span, a no-op span for the invalid case, and a local `SdkLoggerProvider` with `InMemoryLogExporter` wired through `OpenTelemetryTracingBridge` under `tracing::subscriber::with_default` (thread-scoped), in plain `#[test]` functions;
  - the end-to-end "saved row carries the chat span's traceparent" case needs `global::tracer`, so it lives in its own integration-test binary, `crates/runtime/tests/final_answer_traceparent.rs`, with one test that installs a global provider. The "no provider" case is `crates/runtime/tests/final_answer_traceparent_disabled.rs`, a separate binary that installs nothing.
- **To verify in the first failing test (PR 4).** opentelemetry-appender-tracing 0.33 should set `event_name` from the `tracing` metadata name, and the SDK logger should fill trace context from the attached OTel context (we use no `tracing-opentelemetry` layer). If either fails, the helper sets them explicitly on a `LogRecord` from a logger the sink receives from `telemetry::init`. This stays inside the helper.

**Alternatives:**

- A `user_feedback` span parented to the chat span. Rejected: a child starting hours after its parent ended distorts waterfalls.
- A new root span with a span link. Not "parented" in the convention's sense, and querying needs link support.
- Correlation by attribute only, with no stored traceparent. No schema change, but misses the SHOULD and forces a join in every query.
- Emitting through the OTel Logs API directly. Rejected: no global logger provider (`telemetry.rs:119-147`).
- Using the global propagator for format and parse. Rejected: it is a no-op without OTLP, which makes tests depend on process-global setup.

These choices are recorded as an amendment to ADR-0012 (PR 4): persisted domain rows may carry a W3C traceparent so that later, out-of-band events are emitted as span-parented log records.

## Decision 4: Client elements are pure; state stays in the chat feature

- **`FeedbackButtons`** (`assistant_ui`): `FeedbackButtons({MessageRating? rating, ValueChanged<MessageRating?> onChanged, bool enabled = true})`. Two `MessageActionButton`s with keys `feedback_positive_action` / `feedback_negative_action`, `selected` state, labels "Good response" / "Bad response". Tapping the selected thumb emits `null`. `MessageRating { positive, negative }` lives in the package models.
- **`FeedbackCommentField`** (`assistant_ui`): `FeedbackCommentField({String? initialText, ValueChanged<String> onSubmit, VoidCallback onDismiss, int maxLength = 2000})`. The limit counts Unicode scalar values (`text.runes.length`) with a custom formatter, not `TextField.maxLength`, which counts grapheme clusters. Enter submits, Escape dismisses.
- **Placement.** chat-element-library PR 8a makes `MessageActions.trailing` a `List<Widget>`. The app gets a pure `buildMessageTrailing(message, …) → List<Widget>` beside `buildMessageActions`, composing in a fixed order: timing (chat-a11y C11), then feedback. message-branching keeps `leading` (BranchPicker) and the `actions` list (Edit, Regenerate), so none of the three changes edits the same list.
- **Visibility.** Feedback appears for assistant `TimelineEntryType.message` entries that are not streaming, not failed, have non-empty content, and carry a server id (not the `assistant-` fallback).
- **Wiring** (`app/lib/features/chat/`):
  - `ChatMessage` gains `MessageFeedback? feedback`, filled by `chatMessagesFromHistory`.
  - `MessageFeedbackController` (Riverpod `Notifier`, `message_feedback_provider.dart`) keeps, per message id, `confirmed` (last server-confirmed value) and a serial queue of pending calls. `chat_provider.dart` grows only by `setMessageFeedback(id, feedback)` and a merge hook.
- **Optimistic rules:**
  1. **Pending beats history.** While a message id has queued or in-flight calls, any message list written into `ChatState` (`loadConversation`, stall recovery, reconnect fallback, message-branching's post-turn reload) takes that message's feedback from the controller's latest optimistic value, not from the history seed. `ChatNotifier` applies this through one `_withPendingFeedback(messages)` merge used at every list-replacement site.
  2. **Stale callbacks are no-ops.** A success or rollback callback first checks that the current state's `conversationId` is the one the call started in and that the message id is still present. Otherwise it updates only `confirmed` and shows no snackbar.
  3. **Rollback target.** On failure, the controller drops the rest of that message's queue and restores `confirmed`, the last server-confirmed value, not the immediately preceding optimistic value.
  4. **In-flight state.** While a message has a call in flight, the controller exposes `isPending(id)`. The buttons stay enabled (later taps are queued) and the comment field's Send is disabled.
- **Comment flow.** Choosing negative opens `FeedbackCommentField` below the actions row. Submitting re-PUTs `{rating, comment}`. Positive does not open it; changing or clearing the vote closes it. Open state is local UI state keyed by message id.

**Alternatives:**

- A modal comment dialog. Rejected: interrupts reading; assistant-ui's default is inline.
- A separate feedback `Map` in `ChatState`. Rejected: rendering reads per-message fields, and history mapping is the single seed point.
- A single `trailing: Widget?` with feedback owning it. Rejected: chat-a11y's timing indicator needs the same slot.

## Risks / Trade-offs

- **Compaction deletes voted rows.** `replace_history` deletes and re-inserts messages with new ids (`conversations.rs:369-386`, `compaction.rs:355-370`), and the cascade drops their feedback. OTLP events survive. message-branching D5 removes `replace_history`.
- **Shared `messages` lane.** Ordering is binding: message-branching PR 1, then tool-result-renderers S3, then this change's PR 3, then message-branching PR 8. PR 3 owns `persist_final_answer`/`FinalAnswerMeta`.
- **Shared `MessageSummary`.** The feedback preload follows the attachments-map pattern and rebases freely on the other projections.
- **Telemetry volume.** One event per click is negligible.
- **Spec churn.** Attribute names are constants in `otel_spans.rs`, as the existing `GEN_AI_*` strings are (`:28-32`).
- **Trace retention.** The event may point at an expired trace; it still carries conversation, message and Persona ids.

## Migration Plan

- Two forward-only migrations: `NNN_message_feedback.sql` and `NNN_message_traceparent.sql` (`ALTER TABLE messages ADD COLUMN traceparent TEXT`).
- Numbers come from the cross-change pre-allocation in the stack plan; without one, the next free after `045_skill_executions` (`crates/storage/src/lib.rs:472`). Each is registered in the `run_migrations` list (`lib.rs:297`). The second list in `lib.rs`, `seed_database_through_016` (`:562`), is a test fixture that stops at 016 and is not changed.
- No backfill: existing rows have no traceparent, and their events are emitted unparented.

## Open Questions

- Should a positive vote also offer the comment field? The design opens it only on negative.
- Is `[observability] trace_content` the right gate for exporting comment text, or should comments never leave `space.db`?
