## Context

The chat screen (`app/lib/features/chat/chat_screen.dart`) already owns a `ScrollController` and a `_scrollToBottom()` helper. A `ref.listen` on `chatProvider` calls `_scrollToBottom()` on every state change — but this causes two problems:

1. **On open**: loading a conversation sets messages in state, `ref.listen` fires, but the `ListView` may not yet be fully laid out, so `maxScrollExtent` can be 0 and the scroll doesn't actually reach the bottom. This is the root cause of the bug.
2. **While chatting**: the listener unconditionally scrolls to bottom even when the user has intentionally scrolled up to review history. This should only auto-scroll when the user is already near the bottom.

Additionally, there is no visual affordance to return to the latest message when the user has scrolled up.

## Goals / Non-Goals

**Goals:**

- Scroll to the latest message as soon as a conversation is fully loaded (on open and on conversation switch)
- Auto-scroll on new incoming messages only when the user is already at (or near) the bottom
- Show a "scroll to bottom" FAB when the user is scrolled up, allowing one-tap return to latest

**Non-Goals:**

- Preserving exact scroll position across app restarts or navigation
- Infinite scroll / pagination of older messages
- Any backend or API changes

## Decisions

### D1: Track "at bottom" state via scroll position listener

Add a `bool _atBottom = true` field. In `initState`, attach a listener to `_scrollController` that updates `_atBottom` whenever the scroll position changes:

```dart
_scrollController.addListener(() {
  final pos = _scrollController.position;
  final nearBottom = pos.pixels >= pos.maxScrollExtent - _kBottomThreshold;
  if (_atBottom != nearBottom) setState(() => _atBottom = nearBottom);
});
```

A threshold of **80 dp** is used so that "essentially at bottom" still counts (avoids false negatives from rounding or sub-pixel gaps).

**Alternative considered**: Store scroll offset in Riverpod state. Rejected — scroll position is purely local UI state; putting it in the provider adds unnecessary complexity.

### D2: Scroll to bottom after conversation load completes

In `_loadConversation`, after calling `loadConversation(id)`, also call `_scrollToBottom()`. The existing `_scrollToBottom` already wraps the call in `addPostFrameCallback`, but that callback fires before the `ListView` has built all items. To reliably reach the end:

1. First `addPostFrameCallback` schedules the actual scroll
2. The scroll target is `position.maxScrollExtent`, which is correct once the ListView is laid out

This is the standard Flutter pattern and is sufficient for a single-frame delay. If the layout requires multiple frames (very rare, for very long lists), a second callback fallback is added.

**Alternative considered**: `jumpTo` instead of `animateTo` on first open. Rejected — smooth animation is a stated requirement and the 200 ms duration is imperceptible at normal read speed.

### D3: Conditional auto-scroll in `ref.listen`

Change the `ref.listen` block to only call `_scrollToBottom()` when `_atBottom == true`. This ensures the user's manual scroll position is respected during ongoing streaming.

### D4: Scroll-to-bottom FAB via `Stack` overlay

Wrap the message list `Expanded` in a `Stack` and overlay a small `FloatingActionButton` (or `FloatingActionButton.small`) anchored to the bottom-right. The FAB is visible only when `!_atBottom`. Tapping it calls `_scrollToBottom()` and sets `_atBottom = true`.

**Alternative considered**: Using `Scaffold.floatingActionButton`. Rejected — the chat input is below the message list; a global FAB would float above the input row, causing visual conflict.

## Risks / Trade-offs

- **Threshold sensitivity** → 80 dp may feel "sticky" on very short conversations where the list barely exceeds the viewport. Mitigation: initialize `_atBottom = true` so a fresh/short conversation always auto-scrolls.
- **Multiple `addPostFrameCallback` calls** → called on every provider change, but each schedules a single future frame. Slight over-scheduling for streaming messages (many rapid updates). Acceptable given the existing pattern already does this.
- **`setState` in scroll listener** → fires frequently during programmatic and user scroll. The guard (`if (_atBottom != nearBottom)`) limits rebuilds to two transitions (up → not at bottom, down → at bottom), keeping overhead minimal.
