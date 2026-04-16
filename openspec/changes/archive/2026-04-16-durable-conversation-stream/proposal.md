## Why

The SSE stream for chat responses is ephemeral — it is tied to the HTTP connection that initiated it. When a user navigates away while the orchestrator is still running and returns mid-run, there is no way to re-attach to the in-progress stream; the partial response is lost and the UI shows nothing until the final message is persisted. This breaks the intended background-processing UX where the agent continues working after the user leaves.

## What Changes

- Orchestrator emits events (tokens, tool calls, tool results, status, done, error) to a **durable event log** (new DB table) in addition to the in-memory SSE channel
- New REST endpoints allow clients to **replay** missed events from a sequence cursor and **tail** the live log as SSE
- The existing `POST /api/conversations/{id}/messages` SSE endpoint continues to work unchanged for clients that stay connected
- The Flutter chat provider gains reconnection logic: on reconnect, fetch events since the last seen sequence number
- Events are retained for a configurable TTL (default: 24 h) and pruned by a background task

## Capabilities

### New Capabilities

- `conversation-event-log`: Server-side durable log of all orchestrator events per conversation run, identified by sequence number and run ID. Enables replay and live tailing.

### Modified Capabilities

- `chat-message-retry`: Retry behaviour changes slightly — a failed stream with partial content can now be resumed rather than retried from scratch, if a run ID is available.

## Impact

- **`crates/storage`**: new `conversation_events` table, `ConversationEventStore` trait, SQLite migration
- **`crates/runtime`**: orchestrator emits events to `ConversationEventStore` alongside the existing token sink
- **`crates/web-ui`**: two new endpoints — `GET /api/conversations/{id}/runs/{run_id}/events` (replay) and `GET /api/conversations/{id}/runs/{run_id}/events/stream` (live SSE tail)
- **`app/`**: `AssistantClient` gains replay/tail methods; `ChatNotifier` gains reconnect logic using last-seen sequence
- **`openapi.json`**: two new routes, `ConversationEvent` schema, `ConversationRun` schema
