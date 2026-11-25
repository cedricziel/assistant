# ADR 0011: Message Bus Delivery Tracking and Delayed Nack

**Status**: Accepted
**Date**: 2026-11-25

## Context

The turn worker retries transient LLM failures (rate limit, overload) by
nacking the `turn.request` message with an exponential delay
(`transient_nack_delay`: 30 s to 240 s). It stops retrying once
`BusMessage::delivery_count` reaches `MAX_TURN_REDELIVERIES`.

That contract was only honoured by the NATS JetStream bus. The SQLite bus
hard-coded `delivery_count: 1` and inherited the default
`MessageBus::nack_delayed`, which ignores the delay. The in-memory bus did the
same. On SQLite, a persistent transient error therefore redelivered the turn
in a tight loop that never hit the cap, until `submit_turn` timed out (3 h in
production).

## Decision

- Migration `043_bus_delivery_tracking.sql` adds two columns to
  `bus_messages`:
  - `delivery_count INTEGER NOT NULL DEFAULT 0`, incremented in the same
    statement that claims the message. The first delivery reports `1`, as
    JetStream does. `nack`, `nack_delayed` and `reap_stale` keep the count.
  - `visible_at DATETIME`, NULL by default. `claim_filtered` only selects rows
    with `visible_at IS NULL OR visible_at <= now`.
- `nack_delayed` on the SQLite and in-memory buses returns the message to
  `pending` with `visible_at = now + delay`. A plain `nack` clears
  `visible_at`, so an operator or reaper release is immediately claimable.
- The in-memory bus increments `delivery_count` on claim, too.
- The trait default for `nack_delayed` (immediate `nack`) stays for backends
  without a delay primitive.

All timestamps come from the injected `Clock`, so tests drive backoff with
`FakeClock` instead of sleeping.

## Consequences

- A persistently transient turn error now terminates after
  `MAX_TURN_REDELIVERIES` deliveries with a terminal `TurnResult`
  (`had_errors: true`, "exceeded retry cap"), on all three bus backends.
- Worst-case SQLite retry time for one turn is the sum of the backoff
  schedule, roughly 30 minutes.
- Messages published before the migration have `delivery_count = 0` and
  `visible_at = NULL`, which behaves as a fresh message.
- `reap_stale` redeliveries now count toward the cap, as they do on JetStream.
