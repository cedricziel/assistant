## 1. Phase A — currentTurnStatus derived state

- [x] 1.1 Tests written (10 tests) — runId/lastEventKind/lastEventAt/currentToolName across every SSE event kind, plus initial state, DoneEvent clear, transient-error clear, and lastEventAt monotonicity.
- [x] 1.2 Implemented: added `TurnEventKind` enum + `TurnStatusSnapshot` class + `ChatState.currentTurnStatus` field. `_streamMessage` opens the snapshot on `RunStartedEvent`, bumps it via a new `_bumpTurnStatus` helper on every subsequent event kind, and clears it on `DoneEvent` / `ErrorEvent` / retries-exhausted / `_clearStalledPlaceholder`. `TokenEvent` clears `currentToolName` inline (the assistant resumed text generation).
- [x] 1.3 New tests: 10/10 pass.
- [x] 1.4 Full suite: 968/968 pass. `flutter analyze --fatal-infos` clean.

## 2. Phase B — TurnProgressCard widget + integration

- [x] 2.1 Built `turnStatusLabel(TurnStatusSnapshot)` pure function in `lib/features/chat/turn_status_label.dart`. 14 unit tests cover every TurnEventKind, tool-name interpolation, generic fallbacks, and the stalled elapsed-time format (`0:34`, `1:35`, padded seconds).
- [x] 2.2 Wrote widget tests for every state (renders-nothing-when-null, per-event-kind labels, stalled-transition at 30 s, recovers-on-fresh-event, disappears-on-clear, ticker fires once per second, ticker disposes cleanly).
- [x] 2.3 Implemented `app/lib/features/chat/turn_progress_card.dart` — Riverpod-driven, watches `chatProvider.select((s) => s.value?.currentTurnStatus)`.
- [x] 2.4 Elapsed-seconds wired via a 1-second `Timer.periodic`. Started on first non-null snapshot, cancelled in `dispose()`. Verified via fake_async test.
- [x] 2.5 Integrated into `chat_screen.dart` above the `_InputRow` composer. Single line: `const TurnProgressCard(),`. Renders to `SizedBox.shrink()` when no turn is in flight.
- [ ] 2.6 Re-baseline Playwright screenshots for the chat screen. Defer to a follow-up — the visual diff is bounded to the composer area; existing baselines will fail until updated.
- [ ] 2.7 Manual smoke on iPhone Simulator, iPad Simulator, Chrome, `flutter run -d macos`. Defer to user verification.

## 3. Phase C — Queued ghost bubbles

- [x] 3.1 Wrote `test/widget/features/chat/queued_message_bubble_test.dart` — 3 tests covering text+badge rendering, long-press → action-sheet, tap-Remove → notifier callback.
- [x] 3.2 Implemented `lib/features/chat/queued_message_bubble.dart`. Right-aligned ghosted bubble (muted italic text, subtle border, "Queued" badge). GestureDetector wraps onLongPress + onSecondaryTap.
- [x] 3.3 Integrated into `chat_screen.dart`'s `ListView.builder` — `itemCount = messages.length + pendingQueue.length`, queued entries render after the committed messages. EmptyChat now requires both lists to be empty.
- [x] 3.4 Long-press / right-click → `showAdaptiveActionSheet` with a "Remove from queue" destructive action (iOS sheet, Material modal bottom sheet).
- [x] 3.5 Added `ChatNotifier.removeFromQueue(int index)`. Unit tests: removes by index, out-of-bounds is a no-op. Active-streaming message is already popped from the queue so it's untouchable by design.
- [ ] 3.6 Re-baseline Playwright screenshots — deferred (combined with Phase B/D baseline updates).

## 4. Phase D — Reconnect banner

- [ ] 4.1 Write failing widget test for the reconnect banner appearance during an `attemptReconnect()` call and its disappearance on resolution.
- [ ] 4.2 Implement the banner. Reuse the `update_banner.dart` pattern if it fits; otherwise build a sibling overlay.
- [ ] 4.3 Integrate into `chat_screen.dart` (or `nav_shell.dart` if the banner should be app-wide).
- [ ] 4.4 Confirm the banner does NOT fire on routine `AppLifecycleState.resumed` events with no interrupted stream (regression test).

## 5. Telemetry

- [ ] 5.1 Emit OpenTelemetry events for turn-state transitions: `turn.started`, `turn.stalled`, `turn.recovered`, `turn.completed`, `turn.errored`, `turn.queue.dropped` (via remove-from-queue).
- [ ] 5.2 Add a brief docs section to `docs/operations/` describing how to query the new events in the SQLite trace store.

## 6. Final verification

- [ ] 6.1 `make lint-flutter && make test-flutter` — green.
- [ ] 6.2 `flutter analyze --fatal-infos` — zero issues.
- [ ] 6.3 Manual visual QA across iPhone 26, iPad 26, Apple Silicon Mac (Designed for iPad), Chrome browser, `flutter run -d macos`.
- [ ] 6.4 Visual regression baselines re-captured and reviewed.
- [ ] 6.5 Confirm telemetry surfaces in the dev SQLite trace store after exercising each transition.
