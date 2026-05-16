# Design: Reactive Conversation List

## Context

The conversation list in the Flutter app is managed by `ConversationListNotifier` (Riverpod `AsyncNotifier`). Its `build()` fetches `GET /api/conversations` and returns the full list. After mutations (create, delete) or when `ChatNotifier` receives a `DoneEvent`, it calls `refresh()` which re-fetches the entire list.

The backend already has a proven SSE + broadcast pattern for per-run events: `RunBroadcaster` uses `tokio::sync::broadcast` channels, and `stream_run_events()` implements a subscribe-before-replay pattern that eliminates the race between snapshot and live events. The conversation list stream reuses this pattern.

Conversations are scoped per agent. The current API resolves the agent from request context (`AgentContext`). The stream endpoint follows the same convention, with an optional `agent_id` query param to watch a specific agent (default: current agent).

## Goals

- Conversation list updates within 1 second of any mutation, from any client or backend process
- No full re-fetch after mutations — local state patched via stream events
- Clean upgrade path to multi-user and multi-process

## Non-Goals

- Cross-process broadcasting (single process today; trait abstraction enables future swap)
- Sub-100ms latency
- Conflict resolution or CRDT — conversation list is simple enough for last-write-wins

## Decisions

### D1: Global broadcaster with server-side filtering

**Choice:** Single `ConversationBroadcaster` instance shared across the process. The SSE endpoint filters events by agent (and later by user) before forwarding to clients.

**Why:** Simpler than per-agent channels. "Watch all agents" is trivial (no filter). "Watch one agent" is a server-side predicate. Avoids channel lifecycle management.

**Alternative considered:** Per-agent broadcast channels — natural isolation but requires fan-in for "watch all" and channel lifecycle tracking. Over-engineered for current scale.

### D2: Emit events from `ConversationStore`

**Choice:** `ConversationStore` holds an `Option<Arc<dyn ConversationBroadcaster>>` and emits events after every successful write (create, update title, update `updated_at`, delete).

**Why:** `ConversationStore` is the chokepoint for all conversation mutations — runtime, web-ui handlers, and any future writer all go through it. Emitting here means no caller can forget.

**Alternative considered:** Emit from each caller (web-ui handlers, runtime). Explicit but scattered — easy to miss a callsite. Also couples emission to every consumer of the store.

### D3: Broadcaster behind a trait

**Choice:** Define `ConversationBroadcaster` as a trait with `emit()` and `subscribe()`. Ship an in-memory implementation using `tokio::sync::broadcast`.

**Why:** When the system goes multi-process, the in-memory broadcaster won't work across process boundaries. A trait lets us swap in a NATS-backed or SQLite-polling implementation without touching `ConversationStore` or the SSE endpoint.

**Alternative considered:** Concrete struct only. Simpler now but forces a refactor when multi-process arrives. The trait costs ~10 lines of code and saves a future migration.

### D4: Subscribe-before-snapshot (no race window)

**Choice:** The SSE endpoint subscribes to the broadcast channel first, then fetches the snapshot from the DB, then sends the snapshot to the client, then forwards live events (deduplicating any that arrived between subscribe and snapshot by `id` + `updated_at`).

**Why:** If we snapshot first and subscribe second, an event can slip through between the two operations. The client would have stale state until the next reconnect. This is the same pattern used by `stream_run_events()` in the existing codebase.

### D5: Three event types — `snapshot`, `upserted`, `deleted`

**Choice:** The stream sends a `snapshot` event on connect (full `Vec<ConversationSummary>`), then `upserted` (single `ConversationSummary`) and `deleted` (single `conversation_id: Uuid`) deltas.

**Why:** `upserted` (not separate `created`/`updated`) because the client behavior is identical: insert-or-replace in the local list, re-sort. Fewer event types, same result. `snapshot` is the self-healing mechanism — reconnect always starts clean.

### D6: Client-side debounce for list updates

**Choice:** The Flutter `ConversationListNotifier` buffers incoming `upserted` events and applies them on a ~300ms timer. `deleted` and `snapshot` events apply immediately.

**Why:** During an active run, `updated_at` may bump several times in quick succession (message added, title changed). Without debouncing, the list re-sorts and rebuilds on every event. Debouncing collapses rapid updates into a single state change.

### D7: Optional `agent_id` query parameter

**Choice:** `GET /api/conversations/stream?agent_id={uuid}` filters events to a single agent. Omitting the parameter returns events for all agents accessible to the current user.

**Why:** The sidebar conversation list shows one agent's conversations (filtered view), but a future "all conversations" view needs the unfiltered stream. Supporting both from one endpoint avoids duplication.

## Risks / Trade-offs

| Risk                                                        | Likelihood                                                   | Mitigation                                                            |
| ----------------------------------------------------------- | ------------------------------------------------------------ | --------------------------------------------------------------------- |
| Broadcast channel buffer overflow (256 slots) under burst   | Low — conversation mutations are infrequent vs. token events | Lagged receivers get snapshot on reconnect; increase buffer if needed |
| State drift between stream and DB                           | Low — snapshot on reconnect heals                            | Could add periodic re-snapshot (every 5 min) but not needed initially |
| SSE connection limits in browser (6 per domain in HTTP/1.1) | Low — native app, not browser tabs                           | HTTP/2 multiplexing; only one stream per client anyway                |

## Migration Plan

No data migration. The broadcaster is additive — existing REST endpoints continue to work unchanged. The Flutter app switches from pull to push in the `ConversationListNotifier`. Old `refresh()` calls are removed but the method can be kept as a no-op fallback during transition.
