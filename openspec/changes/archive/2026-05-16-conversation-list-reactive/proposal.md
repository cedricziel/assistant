# Reactive Conversation List

## Why

The conversation list is pull-only. It refreshes when `ChatNotifier` explicitly calls `refresh()` after a run completes, but has no mechanism for changes originating elsewhere. When a second device creates a conversation, or an agent completes a run on a conversation the user isn't actively watching, the list stays stale until the next manual interaction. This breaks the expectation that a multi-device assistant keeps itself current.

## What Changes

- Add a `ConversationBroadcaster` in the storage layer that emits events on every conversation write (create, update, delete)
- Add an SSE endpoint `GET /api/conversations/stream` that sends an initial snapshot followed by delta events (`upserted`, `deleted`)
- Replace the pull-based `ConversationListNotifier` in Flutter with a stream-driven notifier that subscribes to the SSE endpoint and patches local state reactively
- Remove explicit `refresh()` calls from `ChatNotifier` — the stream handles it

## Capabilities

### New Capabilities

- **conversation-list-streaming**: The conversation list auto-updates within one second when a conversation is created, updated, or deleted — regardless of which device or process triggered the change.

### Modified Capabilities

- **conversation-list**: No longer requires manual refresh; mutations (create, delete) become fire-and-forget — the stream confirms them as events.

## Impact

- `crates/storage/src/conversation_events.rs` or new file — `ConversationBroadcaster`
- `crates/storage/src/conversations.rs` — emit events on write
- `crates/web-ui/src/api/mod.rs` — new SSE streaming endpoint
- `app/lib/api/api_client.dart` — new `streamConversations()` method
- `app/lib/features/chat/chat_provider.dart` — rewrite `ConversationListNotifier` to be stream-driven
- `openapi.json` — document new endpoint

## Non-goals

- Multi-process broadcasting (NATS, polling) — single-process is sufficient today; broadcaster is behind a trait for future swap.
- Optimistic UI for create/delete — fire-and-forget with stream confirmation is good enough; optimistic updates can be layered later.
- Pagination or filtering on the conversation stream — not needed at current scale.
