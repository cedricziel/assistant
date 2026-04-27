## Why

On 2026-04-26 the schorschvm production deployment drained Moonshot LLM credits overnight. Root cause: a single conversation's turn returned a permanent `429 Too Many Requests — account suspended due to insufficient balance` error, but three layers of retry treated it as transient and looped forever. By the time it was caught, the NATS consumer for one stuck Morgenübersicht turn had reached delivery sequence 120,000,000 in ~21 hours, and the worker had hammered Moonshot's API for every redelivery. Twelve other messages were ack-pending across scheduler/matrix/slack/web workers in the same state.

The platform currently has no upper bound on retry — neither at the LLM-error classifier, the NATS consumer, nor the worker's nack scheduler. A permanent upstream failure on a paid API translates directly to unbounded spend. This change installs three independent caps so any one of them stops the runaway.

## What Changes

- **Permanent-error classification** — extend the LLM transient/permanent classifier to recognize billing/quota/suspended/insufficient-funds markers and return _permanent_ even when the HTTP code is 429. Substring matching of "429" alone is no longer sufficient.
- **Bounded NATS redelivery** — set an explicit `max_deliver` on every JetStream pull consumer the platform creates. After the cap, JetStream itself stops redelivering and the message goes to the dead-letter path.
- **Worker terminal branch** — when a turn message has been redelivered more than the cap, the worker SHALL `Term` the message (AckKind::Term, no further redelivery) and publish a terminal `TurnResult` with a user-visible error so the conversation surfaces failure instead of silently retrying.
- **Observability** — log a single `error!` line at the terminal branch with conversation_id, delivery_count, and the underlying error so operators can grep for stuck turns.

## Non-goals

- Subject-namespaced filtering to eliminate cross-worker NAK churn (Bug 4 from the incident analysis) — separate change.
- Graceful shutdown nack drain (Bug 5) — separate change.
- Retry budget per provider/account (e.g., circuit breaker on consecutive 4xx) — could be a follow-up if this change proves insufficient.
- UI surfacing of the terminal error beyond the existing `TurnResult` error path — depends on whatever the chat UI already does with errored turn results.

## Capabilities

### New Capabilities

- `bounded-llm-retry`: Defines the bounded-retry contract across the LLM error classifier, the NATS consumer, and the orchestrator worker. Owns the permanent-error markers, the redelivery cap, and the terminal-result shape.

### Modified Capabilities

- (none — no existing spec describes the current unbounded retry behaviour)

## Impact

- **Affected code**: `crates/llm-provider/src/retry.rs` (classifier), `crates/bus-nats/src/lib.rs` (consumer config), `crates/runtime/src/orchestrator/worker.rs` (terminal branch + nack delay).
- **APIs**: no HTTP API changes. `TurnResult` already carries an `error` field; this change populates it for the new terminal case.
- **Operational**: deploys to schorschvm and any future paid-LLM deployment as a credit-safety baseline.
- **Tests**: new unit tests for the classifier (Moonshot/OpenAI/Anthropic insufficient-funds variants), integration test for redelivery cap.

## Documentation

User-facing docs: **not required**. Behaviour change is internal — operators will only notice that previously-silent stuck turns now surface as errored turns. Worth a one-line note in `CHANGELOG.md`.
