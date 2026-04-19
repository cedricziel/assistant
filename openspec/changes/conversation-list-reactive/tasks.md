# Tasks: Reactive Conversation List

Each task follows TDD (failing test first) and results in one atomic commit.

## Backend: Broadcaster

- [x] Define `ConversationBroadcaster` trait and `ConversationEvent` enum in `crates/storage`. Write a test that subscribes, emits an `Upserted` event, and asserts the receiver gets it. Red first, then implement `InMemoryConversationBroadcaster` using `tokio::sync::broadcast`.
- [x] Wire `Option<Arc<dyn ConversationBroadcaster>>` into `ConversationStore`. Write a test that calls `create_conversation` on a store with a broadcaster and asserts an `Upserted` event is received. Red, then emit from `create_conversation`.
- [x] Add emission to `delete_conversation` in `ConversationStore`. Test: delete a conversation, assert `Deleted` event received.
- [x] Add emission to conversation update paths (`update_title`, `touch_updated_at`, or whichever methods mutate conversations). Test each path emits `Upserted`.

## Backend: SSE Endpoint

- [x] Add `GET /api/conversations/stream` endpoint in `crates/web-ui`. Test: connect to stream, receive `snapshot` event with current conversations as JSON array. Use subscribe-before-snapshot ordering (D4).
- [x] Add delta forwarding to the stream endpoint. Test: connect to stream, create a conversation via the store, assert `upserted` event arrives on the stream within 1 second.
- [x] Add `agent_id` query parameter filtering. Test: emit events for two agents, assert filtered stream only receives events for the requested agent.
- [x] Update `openapi.json` with the new streaming endpoint documentation.

## Frontend: API Client

- [x] Add `streamConversations({String? agentId})` method to `ApiClient` in `app/lib/api/api_client.dart`. Unit test: mock SSE response with snapshot + upserted + deleted events, assert parsed event types.

## Frontend: Provider Rewrite

- [x] Rewrite `ConversationListNotifier.build()` to subscribe to `streamConversations()`. On `snapshot` event, replace state. On `upserted`, insert-or-replace and re-sort. On `deleted`, remove by id. Widget test: mock stream, assert list updates reactively.
- [ ] Add client-side debounce (~300ms) for `upserted` events. Test: emit 5 rapid upserts for the same conversation, assert only one state rebuild occurs.
- [ ] Remove `refresh()` calls from `ChatNotifier` (after `DoneEvent`, voice completion, event replay). The stream handles updates now. Verify existing chat tests still pass.

## Integration

- [ ] End-to-end test: create a conversation via REST, assert it appears on an active SSE stream. Delete it, assert `deleted` event arrives. (Can be a Rust integration test or a Flutter integration test.)
