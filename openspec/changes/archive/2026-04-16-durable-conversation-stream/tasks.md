## 1. Storage: conversation_events table

- [x] 1.1 Write SQLite migration adding `conversation_events` table with `run_id`, `conversation_id`, `sequence`, `event_type`, `payload`, `created_at`, `expires_at` columns and indexes on `(run_id, sequence)` and `expires_at`
- [x] 1.2 Add `ConversationEventRow` struct and `ConversationEventStore` trait to `crates/storage` with methods: `append_event`, `list_events_since`, `has_active_run`, `prune_expired`
- [x] 1.3 Implement `ConversationEventStore` on `StorageLayer` (SQLite backend)
- [x] 1.4 Write unit tests for `append_event`, `list_events_since`, and `prune_expired` using `StorageLayer::new_in_memory()`

## 2. Runtime: event emission

- [x] 2.1 Add `run_id: Uuid` to the orchestrator invocation context; generate it at `POST /messages` handler before spawning the task
- [x] 2.2 Wire `ConversationEventStore` into the orchestrator so each emitted event (`Token`, `Status`, `ToolCall`, `ToolResult`, `Done`, `AgentError`) is written to the DB alongside the existing mpsc sink
- [x] 2.3 Emit `run_started` as the first event (sequence 0) with `payload = {"run_id": "<uuid>"}` before the LLM call begins
- [x] 2.4 Add a `broadcast::Sender<ConversationEvent>` registry keyed by `run_id` to `StorageLayer` (or new `EventBroadcaster`); orchestrator publishes to it; drop the sender after `done`/`error`
- [x] 2.5 Register the `prune_conversation_events` task in the existing `Scheduler` (every 60 min); also call once on server startup

## 3. API: new endpoints

- [x] 3.1 Add `X-Run-Id` response header to `send_message` and `send_voice_message` handlers in `crates/web-ui/src/api/mod.rs` (set before SSE body begins)
- [x] 3.2 Add `GET /api/conversations/{id}/runs/{run_id}/events/stream` handler: replay stored events from `?since` cursor, then tail live broadcast, close on `done`/`error`; return 404/410 as specified
- [x] 3.3 Register the new route in the axum router
- [x] 3.4 Add `ConversationEvent`, `ConversationRunSummary` to utoipa `#[derive(ToSchema)]` and include in the OpenAPI spec struct
- [x] 3.5 Write integration tests for the replay endpoint: reconnect mid-run scenario, completed run replay, 404 unknown run, 410 expired run

## 4. Flutter: run_id tracking and reconnect

- [x] 4.1 Parse `run_started` SSE event in `api_client.dart` `_parseSse`; surface it as a new `RunStartedEvent` variant in `StreamEvent`
- [x] 4.2 Read `X-Run-Id` response header in `streamMessages` as a fallback; store on `AssistantClient`
- [x] 4.3 Add `streamEventsFrom(conversationId, runId, {int since = 0})` method to `AssistantClient` calling the new replay endpoint
- [x] 4.4 Store `_currentRunId` and `_lastSeq` on `ChatNotifier`; update `_lastSeq` on every received event
- [x] 4.5 In `_streamMessage` catch block: if `_currentRunId` is set, attempt replay before marking the message as failed; fall back to failed state on 404/410
- [x] 4.6 Update `retryMessage` to call replay first if `run_id` is available (per modified `chat-message-retry` spec)
- [x] 4.7 Write widget tests covering: successful reconnect replays tokens into UI, replay 404 falls back to re-send, replay 410 falls back to re-send

## 5. Spec and client codegen

- [x] 5.1 Run `make dump-openapi` to update `openapi.json` with the new routes and schemas
- [x] 5.2 Run `make generate-flutter-client` to regenerate `app/packages/assistant_api/`
- [x] 5.3 Verify `flutter analyze` passes with zero issues
