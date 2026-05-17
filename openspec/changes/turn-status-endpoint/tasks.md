## 1. Runtime layer — expose turn state + abort surface

- [ ] 1.1 Audit the existing turn tracking in `assistant-runtime` (`Orchestrator` + turn-result bus). Identify the data already in scope: turn ID, conversation ID, current state (running/completed/errored), most-recent-event timestamp + kind.
- [ ] 1.2 Write a failing unit test for a `TurnRegistry::status(turn_id) -> TurnStatus` (or equivalent) accessor returning the four-state enum.
- [ ] 1.3 Implement the accessor. Reuse existing structures where possible — don't introduce parallel state if the data already exists.
- [ ] 1.4 Write a failing unit test for `TurnRegistry::cancel(turn_id) -> Result<CancelOutcome, ...>` covering the running / completed / errored / unknown cases.
- [ ] 1.5 Implement cancel — wire it to the existing internal abort hooks. Ensure spawned task tree (LLM + tools + subagents) is torn down. Confirm partial output is saved to the conversation per existing best-effort semantics.
- [ ] 1.6 Write a failing test that confirms cancel emits a final `agent_error` SSE event with a cancellation marker on any open stream.
- [ ] 1.7 Implement the SSE-event emission on cancel.

## 2. HTTP layer — handlers + OpenAPI

- [x] 2.1 Create `crates/web-ui/src/api/turns.rs` with the read handler. Cancel handler deferred until runtime cancellation tokens land (section 1).
- [x] 2.2 Register the GET status route in `crates/web-ui/src/api/mod.rs`. Cancel route deferred with section 1.
- [x] 2.3 Add `TurnStatusResponse` type with the four-state enum (`running`/`completed`/`errored`/`unknown`), `last_event_at` (RFC 3339), `last_event_kind` (open string for forward-compat).
- [x] 2.4 Conversation-scoped auth middleware applies — the route is mounted under the same `/api` sub-router as all other conversation endpoints. No extra permission model needed.
- [x] 2.5 Read endpoint unit tests in `api::turns::tests` cover all four states (running / completed / errored / unknown), plus the cross-conversation guard. Cancel-endpoint tests deferred.
- [x] 2.6 `make dump-openapi` regenerated `openapi.json`; path + components diff committed.
- [ ] 2.7 `make lint-openapi` (Spectral) — runs in CI; local spectral binary not installed.

## 3. Flutter client — generate and integrate

- [ ] 3.1 Run `make generate-flutter-client` to regenerate `app/packages/assistant_api/`. Commit the diff.
- [ ] 3.2 Add thin convenience methods to `app/lib/api/api_client.dart`: `Future<TurnStatus> turnStatus(turnId)` and `Future<void> cancelTurn(turnId)` (the latter throws on 404, returns reconciled terminal state on 409).
- [ ] 3.3 Write failing unit tests in `app/test/unit/chat/chat_provider_test.dart` for the new probe-on-stall flow and the cancel-on-skip flow.
- [ ] 3.4 Modify `ChatNotifier` so the byte-heartbeat timeout calls `turnStatus()` instead of closing the stream directly. Translate the probe result into the appropriate state transition (keep streaming / reconcile / refetch).
- [ ] 3.5 Wire `chat-stream-progress-ux`'s Skip button to `cancelTurn()`. Behind a feature flag for one release.
- [ ] 3.6 Confirm `flutter test` is green.
- [ ] 3.7 Confirm `flutter analyze --fatal-infos` is green.

## 4. Removal of legacy heuristic-cancellation path

- [ ] 4.1 After client phase 3 has shipped for one release and telemetry shows `cancelTurn` works as expected in the wild, remove the byte-heartbeat stream-close path entirely. The probe is now the only trigger.
- [ ] 4.2 Update `chat-message-queue` spec to remove the legacy fallback.
- [ ] 4.3 Delete now-dead test scaffolding around the old watchdog path.

## 5. Documentation

- [ ] 5.1 Add operator docs in `docs/operations/` covering the new endpoints, their state machine, and the cancel semantics (including partial-output preservation).
- [ ] 5.2 Add a developer doc explaining how the client uses the probe and when the byte heartbeat triggers it.

## 6. Final verification

- [ ] 6.1 `make build && make test && make lint && make format` — all green.
- [ ] 6.2 `make test-integration` — green.
- [ ] 6.3 `flutter test && flutter analyze --fatal-infos` — green.
- [ ] 6.4 End-to-end manual test: queue a message during a long tool call, exercise stall → probe → user-skip → cancel → queue advance.
- [ ] 6.5 Verify partial-output preservation by cancelling mid-stream and confirming the bubble appears with the truncated text.
