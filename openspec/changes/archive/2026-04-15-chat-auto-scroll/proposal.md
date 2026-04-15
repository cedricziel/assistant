## Why

When a user opens a conversation with many messages, the chat view shows the oldest messages at the top, forcing them to scroll down manually to reach the latest message. This is a UX regression compared to standard chat apps and is especially frustrating for long conversations.

## What Changes

- The chat screen automatically scrolls to the bottom (latest message) when a conversation is first opened
- The chat screen automatically scrolls to the bottom when new messages arrive (only if the user is already near the bottom)
- A "scroll to bottom" floating action button appears when the user has scrolled up, allowing one-tap return to latest messages

## Capabilities

### New Capabilities

- `chat-auto-scroll`: Auto-scroll behavior for the chat conversation view — scrolls to the latest message on open and on new message arrival, with a scroll-to-bottom button when scrolled up

### Modified Capabilities

<!-- No existing spec-level requirements are changing -->

## Impact

- `app/lib/features/chat/` — chat screen and message list widget
- `app/lib/features/chat/` — scroll controller lifecycle and state management
- No API changes, no Rust backend changes
- No breaking changes
