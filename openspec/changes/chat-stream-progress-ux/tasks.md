## 1. Phase A — currentTurnStatus derived state

- [ ] 1.1 Write failing unit tests for the new `currentTurnStatus` accessor on `ChatNotifier` (or sibling selector): covers turnId/lastEventKind/lastEventAt/currentToolName across every SSE event kind.
- [ ] 1.2 Implement `currentTurnStatus` updates inside `ChatNotifier`'s SSE event handlers — one update per event kind. Cleared on `done` / `agent_error`.
- [ ] 1.3 Run the new unit tests; confirm green.
- [ ] 1.4 Run the full `flutter test` suite — must remain green.

## 2. Phase B — TurnProgressCard widget + integration

- [ ] 2.1 Spike: build a `turnStatusLabel(TurnStatusSnapshot)` pure function with widget tests for each case (run_started / token / status / thinking / tool_result / subagent_started / stalled / unknown).
- [ ] 2.2 Write failing widget tests for `TurnProgressCard` rendering each state.
- [ ] 2.3 Implement `app/lib/features/chat/turn_progress_card.dart`.
- [ ] 2.4 Wire elapsed-seconds via a `Ticker` inside the card; dispose on widget removal.
- [ ] 2.5 Integrate into `chat_screen.dart` above the composer.
- [ ] 2.6 Re-baseline Playwright screenshots for the chat screen.
- [ ] 2.7 Manual smoke on iPhone Simulator, iPad Simulator, Chrome, `flutter run -d macos`.

## 3. Phase C — Queued ghost bubbles

- [ ] 3.1 Write failing widget tests for `QueuedMessageBubble` (rendering + remove action via AdaptiveActionSheet).
- [ ] 3.2 Implement `app/lib/features/chat/queued_message_bubble.dart`.
- [ ] 3.3 Integrate into the conversation list rendering in `chat_screen.dart` — queued entries appear after the most recent committed message, before the streaming placeholder.
- [ ] 3.4 Wire long-press / right-click → AdaptiveActionSheet → "Remove from queue".
- [ ] 3.5 Add `ChatNotifier.removeFromQueue(messageId)` with unit tests for the no-op-on-active-stream case.
- [ ] 3.6 Re-baseline Playwright screenshots.

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
