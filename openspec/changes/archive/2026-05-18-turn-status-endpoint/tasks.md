## 1. Runtime layer — expose turn state + abort surface

- [x] 1.1 Audit complete: the runtime already maintained per-turn `CancellationToken`s in `Orchestrator::turn_cancellations` and a worker `tokio::select!` that aborted on cancel. Only the external surface was missing.
- [x] 1.2 / 1.3 `Orchestrator::cancel_turn(request_id) -> CancelOutcome` (PR #850). `CancelOutcome::Cancelled` / `NotFound` covers all four observable states (running cancels, the other three are no-ops). Status is read via `GET .../status` (PR #836) which derives from the event store, so a separate runtime accessor was unnecessary.
- [x] 1.4 / 1.5 `submit_turn_with_request_id` (PR #850) lets the SSE handler pin the request_id; the worker's existing `tokio::select!` tears down the in-flight LLM/tool/subagent tree on token cancel. Partial tokens streamed before cancel are preserved on the SSE event log (TTL retention).
- [x] 1.6 / 1.7 The `submit_turn` poll loop honors external cancellation by returning `Err` containing `TURN_CANCELLED_MARKER` (PR #850); the SSE handler in `messages.rs` detects the marker and emits a terminal `agent_error` event with `{"reason": "cancelled", "partial_content": "..."}` (PR #853). Subsequent status reads converge on `errored`.

## 2. HTTP layer — handlers + OpenAPI

- [x] 2.1 / 2.2 `crates/web-ui/src/api/turns.rs` ships both `get_turn_status` (PR #836) and `cancel_turn` (PR #853). Routes registered in `crates/web-ui/src/api/mod.rs`.
- [x] 2.3 `TurnStatusResponse` + four-state `TurnState` enum (`running`/`completed`/`errored`/`unknown`), `last_event_at` (RFC 3339), `last_event_kind` (open string for forward-compat).
- [x] 2.4 Conversation-scoped auth via the existing `/api` bearer middleware. Both routes declare `security(("bearer_token" = []))`.
- [x] 2.5 Unit tests in `api::turns::tests` cover every state for both endpoints (running / completed / errored / unknown / cross-conversation guard for read; unknown / completed / mismatched for cancel).
- [x] 2.6 `openapi.json` regenerated; Flutter client regenerated; both diffs committed.
- [x] 2.7 `make lint-openapi` (Spectral) is enforced in CI.

## 3. Flutter client — generate and integrate

- [x] 3.1 `make generate-flutter-client` regenerated `app/packages/assistant_api/` (PRs #836, #853).
- [x] 3.2 `ApiClient.turnStatus(conversationId, runId)` and `ApiClient.cancelTurn(conversationId, runId)` — both return `Future<TurnState?>`, swallow transport errors as `null` so callers don't handle DioException.
- [x] 3.3 Probe-on-stall tests: `chat_provider_test.dart` group `stall probe routing` (PR #842). Cancel-on-skip tests: `turn_progress_card_test.dart` Skip group (PR #853).
- [x] 3.4 `ChatNotifier._probeOrRecover` replaces the byte-watchdog stream-close decision (PR #842). `running` keeps sink open; everything else falls back to legacy recovery.
- [x] 3.5 Skip button on stalled progress card calls `requestCancelTurn` (PR #853), gated by `kSkipButtonEnabled` (default `true`; flip to `false` as a kill switch).
- [x] 3.6 `flutter test` green (1004 tests).
- [x] 3.7 `flutter analyze --fatal-infos` green.

## 4. Removal of legacy heuristic-cancellation path

- [ ] 4.1 Deferred — keeps the byte-heartbeat stream-close path as the safety net for at least one release while telemetry validates `cancelTurn` reliability in the wild.
- [ ] 4.2 Deferred (paired with 4.1).
- [ ] 4.3 Deferred (paired with 4.1).

## 5. Documentation

- [x] 5.1 `docs/operations/turn-status-endpoint.md` covers endpoints, state machine, cancel semantics, partial-output preservation, observability.
- [x] 5.2 `docs/development/chat-stream-probe.md` explains client-side probe routing, the two-watchdog model, Skip button, and test surface.

## 6. Final verification

- [x] 6.1 `make build && make test && make lint && make format` — green on each PR.
- [x] 6.2 `make test-integration` — green on PR #850 (runtime cancellation requires the real worker).
- [x] 6.3 `flutter test && flutter analyze --fatal-infos` — green.
- [ ] 6.4 End-to-end manual test deferred — exercised in unit + widget tests; live exercise belongs to the operator post-deploy.
- [ ] 6.5 Partial-output preservation verified in unit tests (`agent_error.partial_content` carries streamed tokens); live UX validation deferred with 6.4.
