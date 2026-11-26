# ADR 0024: The Run, Not `done`, Bounds the Replay Stream

**Status**: Accepted
**Date**: 2026-11-26
**Builds on**: [ADR-0013](adr-0013-per-space-runtimes.md)

## Context

A web turn writes its frames to `conversation_events` and broadcasts them to
live subscribers. Two endpoints read them:

- `POST /api/conversations/{id}/messages` streams the turn as it runs and
  closes with the terminal frame (`done` or `agent_error`).
- `GET /api/conversations/{id}/runs/{run_id}/events/stream` replays stored
  frames from a sequence cursor, then tails live ones.

Both treated `done` and `agent_error` as the last frame of a run. The replay
endpoint closed as soon as it had replayed or seen a terminal frame. The turn
status endpoint derived `state` from the kind of the last stored row.

Several changes need to write frames after `done`: follow-up suggestions
(`suggestions`), tool approvals (`interaction_requested`,
`interaction_resolved`) and generative UI (`surface_update`). With the old
rules, the first row written after `done` turns a finished turn back into
`running`, and a replay client never sees it because the stream has already
closed.

## Decision

**The run bounds the replay stream. A terminal row decides the turn status.**

- **Replay closes when the run finishes.** After replaying stored rows,
  `stream_run_events` closes if the run is no longer registered with the
  `RunBroadcaster`. Otherwise it tails until the broadcast channel closes,
  which happens when the run's owner calls `finish_run`. It no longer stops at
  `done` or `agent_error`. A client that disconnects ends the tail at once and
  releases its broadcast receiver, so the owner can tell that nobody listens.
- **The POST stream is unchanged.** It still closes at the terminal frame.
  Clients that buffer a chunked body until close then get the answer without
  waiting for post-`done` work. Frames after `done` reach clients on the
  replay stream.
- **Status reads terminal rows, not the last row.** `errored` if any
  `agent_error` row exists, else `completed` if any `done` row exists, else
  `running`. `last_event_kind` and `last_event_at` still describe the most
  recent row, which may come after `done`.
- **Precedence is fixed for later states.** `errored` > `completed` >
  `awaiting_input` > `running`. A non-terminal state added later ranks below
  `completed`: a turn that has finished is never reported as waiting.
- **`done` is no longer the last frame of a run.** Consumers must not assume
  it is. A client that stops reading at `done` keeps working.

## Alternatives considered

- **Keep `done` last and send later frames on another stream.** The
  conversation list stream carries conversation upserts, not per-run frames,
  and cannot be replayed from a cursor.
- **Keep the POST stream open until the run finishes.** A client that
  buffers the body waits for the whole post-`done` phase before it sees the
  answer. The stall probe on those clients could also route them into
  recovery.
- **Derive status from the last row but skip known non-terminal kinds.** Every
  new frame kind would have to be added to the skip list. Reading for a
  terminal row needs no list.

## Consequences

- A replay client of an active run waits for the run to finish, not for
  `done`, before the stream closes.
- A run whose events were pruned and that is not active returns `404`, the
  same as a run that never existed. The replay endpoint no longer documents a
  `410`.
- `docs/operations/turn-status-endpoint.md` documents the terminal-row rule.
- The owners of the consumers extend this rule instead of replacing it:
  follow-up-suggestions persists `suggestions` after `done`;
  tool-approval-prompts adds `awaiting_input` below `completed`;
  generative-ui-a2ui writes `surface_update` rows.
- Whoever writes frames after `done` must call `finish_run` when done,
  because that is now what ends a replay tail.
