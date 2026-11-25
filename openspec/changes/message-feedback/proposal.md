## Why

Users cannot tell us whether an answer was good. assistant-ui ships thumbs up/down as a standard message action. Our action row offers only copy, read aloud, retry and stop (`app/lib/features/chat/message_meta_actions.dart:15-60`), and the server has no table, route or telemetry for a verdict.

Operators already read turns in their OTLP backend through the `chat {model}` span (`crates/runtime/src/otel_spans.rs:217-292`). The GenAI conventions define `gen_ai.evaluation.result` for this (semantic-conventions-genai v0.1.0), and it SHOULD be parented to the evaluated span.

## What Changes

- **Storage.** A `message_feedback` table in `space.db`: one row per (message, user), rating `positive` | `negative`, optional comment, timestamps, `ON DELETE CASCADE` from `messages`. A `MessageFeedbackStore` trait with `Sqlite*`/`InMemory*` impls (ADR-0009).
- **Turn trace link.** Final-answer rows store the W3C `traceparent` of the `chat {model}` span in a nullable `messages.traceparent` column.
- **REST.** `PUT` and `DELETE /api/conversations/{id}/messages/{message_id}/feedback`. `GET /api/conversations/{id}` adds the caller's vote as `MessageSummary.feedback`.
- **Telemetry.** A `gen_ai.evaluation.result` log event in the reply's trace, labelled `positive`, `negative` or `cleared`. Comment text is exported only when `[observability] trace_content` is on. A counter `assistant.feedback.count{rating, agent_id}`.
- **Client.** Pure `FeedbackButtons` and `FeedbackCommentField` elements in `app/packages/assistant_ui`, with widgetbook stories and goldens. Riverpod wiring in `app/lib/features/chat/`: optimistic updates with rollback, seeded from history.
- **ADR.** An amendment to ADR-0012 records storing trace context on persisted rows for late, span-parented events.

## Capabilities

### New Capabilities

- `message-feedback`: storage, REST contract and OTel emission for per-user ratings of assistant replies.
- `message-feedback-ui`: the feedback elements and their wiring into the chat thread.

### Modified Capabilities

None. chat-element-library is an unarchived change. This change uses its `MessageActions`/`MessageActionButton` (its PR 8a) without amending them and needs no new package dependency.

## Impact

- **Crates:** `assistant-storage` (two migrations, the store), `assistant-core` (`Message.traceparent`), `assistant-runtime` (traceparent capture, telemetry helper, metric), `assistant-web-ui` (routes, history projection, `openapi.json`).
- **App:** two `assistant_ui` elements, the feedback controller and history mapping, the regenerated `assistant_api`.
- **Visual change:** trailing thumbs appear under every finished assistant reply, so the chat Playwright screenshot baselines move.
- **User-facing docs: yes.** A short section in `docs/web-ui.md` on rating replies, and the event and metric reference in `docs/opentelemetry.md`.
- **Siblings:** needs chat-element-library PR 3 and PR 8a. Shares the `Message` struct lane (after tool-result-renderers S3) and the `MessageActions` trailing list (after chat-a11y's timing indicator).

### Non-goals

- An in-app admin or analytics view, or a space-wide listing endpoint. Analysis happens in the OTLP backend (ADR-0012).
- Feedback on user, tool, thinking or subagent entries, or on messenger interfaces.
- Star scales, multi-dimensional ratings or preset reason categories.
- Feeding feedback back into the Persona, skills or prompts.
- Keeping feedback on rows that compaction deletes before message-branching lands.
