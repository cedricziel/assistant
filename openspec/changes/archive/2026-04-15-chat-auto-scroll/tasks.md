## 1. Track "at bottom" state

- [x] 1.1 Add `bool _atBottom = true` field to `_ChatScreenState` in `chat_screen.dart`
- [x] 1.2 Add `const double _kBottomThreshold = 80.0` constant (file-level or class-level)
- [x] 1.3 In `initState`, attach a scroll listener to `_scrollController` that updates `_atBottom` via `setState` when the user's proximity to the bottom changes

## 2. Fix scroll-to-bottom on conversation open

- [x] 2.1 In `_loadConversation`, call `_scrollToBottom()` after loading/clearing the conversation so the view animates to the bottom once the new message list is laid out
- [x] 2.2 Verify that `_scrollToBottom` uses `addPostFrameCallback` (already present) so it fires after the ListView rebuilds

## 3. Guard auto-scroll on new messages

- [x] 3.1 Update the `ref.listen(chatProvider, ...)` block to only call `_scrollToBottom()` when `_atBottom == true`

## 4. Add scroll-to-bottom FAB overlay

- [x] 4.1 Wrap the message list `Expanded` widget in a `Stack`
- [x] 4.2 Add a `Positioned` child inside the `Stack` (bottom-right corner) containing a `FloatingActionButton.small` with a `keyboard_arrow_down` icon
- [x] 4.3 Show the FAB only when `!_atBottom` (use `AnimatedOpacity` or a conditional render)
- [x] 4.4 Wire the FAB's `onPressed` to `_scrollToBottom()`

## 5. Test

- [x] 5.1 Run `flutter analyze` and confirm zero issues
- [x] 5.2 Run `flutter test` and confirm all existing tests still pass
- [ ] 5.3 Manual smoke test: open a long conversation → auto-scrolls to bottom; scroll up → FAB appears; tap FAB → scrolls to bottom; send a message while scrolled up → does NOT auto-scroll
