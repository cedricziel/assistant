## Context

The orchestrator's turn-handling pipeline has three independent retry layers, each of which currently lacks an upper bound:

1. **`reqwest_retry` middleware** inside the LLM provider HTTP client — 5 attempts on transient errors per LLM call.
2. **`is_transient_error_message`** in `crates/llm-provider/src/retry.rs:74` — classifies an error as transient if its message contains any of `"408"`, `"429"`, `"500"`, `"502"`, `"503"`, `"504"`, `"529"` as a substring, or matches a small set of keywords. The classifier is consulted by the worker after a turn fails.
3. **NATS JetStream pull consumer** for `bus.turn.request` — created in `crates/bus-nats/src/lib.rs::consumer_for` with `..Default::default()`, leaving `max_deliver = -1` (infinite). The orchestrator worker calls `nack_delayed` on transient errors, scheduling a redelivery after 30/60/120/240 seconds (`transient_nack_delay` in `crates/runtime/src/orchestrator/worker.rs:881`). The default-arm of the match is `_ => 240`, so deliveries past the third stay at 240 s but never terminate.

When Moonshot suspended the schorschvm account on 2026-04-26 it returned `429 Too Many Requests — Your account ... is suspended due to insufficient balance, please recharge`. The classifier saw `"429"` as a substring and reported transient. The worker called `nack_delayed`. JetStream redelivered. The next attempt hit the same permanent error. Repeat for ~21 hours until consumer-sequence reached ~120,000,000 on a single conversation.

Because Moonshot bills per request including the failures, every redelivery cost real money.

## Goals / Non-Goals

**Goals:**

- Any one of the three layers must be sufficient to stop unbounded retry of a permanent failure.
- The user must see _something_ in the conversation when a turn is given up — silent infinite retry is the worst possible UX.
- The fix must be safe to deploy as a hotfix to schorschvm without operator intervention beyond a service restart.
- Behaviour for genuinely transient errors (provider blip, network) must remain unchanged within the cap.

**Non-Goals:**

- Provider-level circuit breakers (e.g. trip after N consecutive 4xx within a window) — could come later.
- Spend tracking / hard credit caps. The fix here is correctness, not budgeting.
- Reworking the NATS subject layout to avoid cross-worker NAK churn (Bug 4 from incident analysis).
- Persisting retry state across restarts. With `max_deliver = 10`, JetStream tracks delivery count for us — restarts that don't reset the consumer preserve the cap; restarts that do reset it (e.g., consumer recreation) restart the count, which is acceptable.

## Decisions

### Decision 1: Classify "insufficient balance / quota / suspended / billing" as permanent

Add a new function `is_permanent_billing_error(msg: &str) -> bool` and have `is_transient_error_message` short-circuit to `false` when it returns true. Markers (case-insensitive substring match):

- `"insufficient balance"` (Moonshot)
- `"insufficient_quota"` (OpenAI's actual code for 429-by-quota)
- `"insufficient quota"`
- `"account is suspended"` / `"account suspended"`
- `"billing"` combined with any of `"required"`, `"hard limit"`, `"exceeded"`
- `"quota exceeded"`
- `"please recharge"`
- `"payment required"` (HTTP 402 in spirit)

**Rationale:** these strings come from upstream provider error bodies and are stable identifiers for permanent states. We accept some false-positive risk (a transient error that happens to mention "billing") in exchange for the much larger benefit of bounding spend.

**Alternative considered:** Inspect HTTP status code separately from message body. Rejected because the worker only sees the stringified `anyhow::Error` after several layers of wrapping; structured codes are not preserved. Adding structured propagation is a bigger refactor than this hotfix warrants.

### Decision 2: Set `max_deliver = MAX_DELIVER` on JetStream consumer config

In `crates/bus-nats/src/lib.rs`, define `const MAX_DELIVER: i64 = 10;` and pass it explicitly in `consumer_for`. Existing consumers on schorschvm need to be deleted-and-recreated for the new config to take effect (or a new consumer name version suffix added). Document the operator step.

**Rationale:** 10 is a reasonable balance — enough to absorb a long provider outage with our 30/60/120/240/240/240/240/240/240/240 backoff schedule (~28 minutes total before terminate), but small enough to bound spend at one cheap conversation's worth of retry per stuck message.

**Alternative considered:** `max_deliver = 5`. Rejected: a real 5-minute provider outage could exhaust 5 attempts on the 30+60+120+240+240 schedule and false-positive a real conversation. 10 gives more headroom.

### Decision 3: Worker terminal branch on delivery_count > MAX_TRANSIENT_DELIVERIES

In `crates/runtime/src/orchestrator/worker.rs`, define `const MAX_TRANSIENT_DELIVERIES: u32 = 10;` (matching `max_deliver`). At the call site that currently calls `nack_delayed`:

```text
if is_transient_turn_error(&e) {
    if msg.delivery_count >= MAX_TRANSIENT_DELIVERIES {
        error!(error = %e, conversation_id = %conv_id, delivery = msg.delivery_count,
               "Transient turn error exceeded retry cap — terminating");
        publish_terminal_turn_result(&self.bus, &conv_id, &e).await;
        let _ = self.bus.fail(msg.id).await;  // AckKind::Term
    } else {
        let delay = transient_nack_delay(msg.delivery_count);
        warn!(...);
        let _ = self.bus.nack_delayed(msg.id, delay).await;
    }
}
```

The terminal `TurnResult` carries `error: "exceeded retry cap: <last error message>"` and is published on the same topic the chat UI already subscribes to. No new event types.

**Rationale:** belt-and-suspenders with `max_deliver`. Even if the consumer's max-deliver is reset (e.g. by recreation during deploy), the worker's own counter halts the loop. The worker publishes the terminal result _before_ terming — JetStream's terminate-and-DLQ path can also be wired up later, but for now the user-visible signal is what matters.

**Alternative considered:** rely solely on `max_deliver` and let JetStream send to DLQ. Rejected: there's no listener for DLQ today, so the user would see nothing. Publishing the terminal result from the worker keeps UX correct without adding new infrastructure.

### Decision 4: Add `bus.fail` (`AckKind::Term`) to the `MessageBus` trait if not present

If `MessageBus` does not currently expose `fail` / terminate, add it. Implementation in `assistant-bus-nats` calls `message.ack_with(AckKind::Term)`. The in-memory bus used by tests can implement it as "remove from inflight map and never re-deliver".

**Rationale:** distinguishing `nack` (will redeliver) from `fail` (won't redeliver) is fundamental to bounded redelivery and should exist on the trait.

## Risks / Trade-offs

- **[False positive on permanent classification]** A transient hiccup that happens to mention "billing" could be terminated early. → Mitigation: keep the marker list narrow and incident-driven; log the matched marker so operators can review false positives.
- **[Recreating the consumer]** Adding `max_deliver` to the consumer config means existing consumers on schorschvm and any other deployment will continue with their old (infinite) settings until they are deleted. → Mitigation: include a one-line nats CLI deletion in the deploy notes, or change the consumer name suffix to force recreation. We will use `get_or_create_consumer` semantics: in NATS, if the existing consumer's config differs, the call updates it. Verify this behaviour in an integration test.
- **[Cap mismatch between bus and worker]** If the worker's `MAX_TRANSIENT_DELIVERIES` and the bus's `MAX_DELIVER` drift apart, behaviour is confusing. → Mitigation: define the constant in one place (`assistant-core` or shared module) and have both crates use it.
- **[Lost messages on terminate]** A `Term`-ed message is gone from the stream. If we later want to inspect failed turns, we have no record. → Mitigation: the terminal `TurnResult` is persisted by `assistant-storage` as part of normal turn-result handling, so the conversation history retains evidence. Acceptable.

## Migration Plan

1. Land code change behind no feature flag — the new behaviour is strictly safer than the old.
2. On deploy to schorschvm: `nats consumer rm ASSISTANT_BUS <consumer-name>` for any existing pull consumer that has accumulated infinite-redelivery state, OR rely on the test that confirms `get_or_create_consumer` updates config on a re-call.
3. Restart the orchestrator units; new consumers come up with `max_deliver = 10`.
4. Verify in `nats consumer info`: `max_deliver: 10`.

**Rollback:** revert the commit; redeploy. The new constants and the `is_permanent_billing_error` function are additive — reverting them restores prior behaviour. No schema or data migration.

## Open Questions

- Should the cap also apply to non-transient (already-permanent) errors? Currently those are dead-lettered immediately, which is correct; this change only adds a path from "transient → terminal after N tries". No change to the permanent path needed.
- Should the worker emit a metric counter for terminal-by-cap events? Useful for ops; not blocking.
