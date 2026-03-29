# Feature Spec: Observability UI Improvements

**Feature**: `003-observability-ui-improvements`
**Branch**: `003-observability-ui-improvements`

## Problem

The web UI traces and logs pages do not provide enough context to diagnose silent failures
in Slack conversations. Specifically: a turn that completed without sending a reply to the
user is indistinguishable from a successful turn. The UI also lacks time-range filtering,
interface filtering, and an easy way to correlate a conversation with its logs.

A real incident on 2026-03-27 (`conversation_id: 06effb79-9644-41f0-9e21-a3312c9d408c`)
showed the LLM calling `list-tasks` and then silently ending the turn without posting a
reply — completely invisible in the current UI.

## Requirements

1. **Replied badge** — On the traces list, show whether a turn sent a reply (`reply` or
   `slack-post` tool was called). Turns that completed without replying should be visually
   distinct.

2. **Interface facet** — Filter traces by originating interface: Slack, Scheduler, CLI, etc.
   The `interface` attribute is already stored in span attributes; it just isn't surfaced.

3. **Conversation ID search input** — A visible sidebar input on `/traces` and `/logs` to
   filter by `conversation_id`. Currently only accessible via hidden query param.

4. **Time range picker** — Date/time range filter on both `/traces` and `/logs` pages.
   Currently the analytics page has time windows but traces/logs do not.

5. **Show logs for conversation** — On the trace detail page, a link to view all logs for
   the entire conversation (spanning multiple turns / trace IDs), not just one trace.

6. **Propagate worker failure to trace status** — When the orchestrator worker catches an
   `Err` from a turn, set `otel.status_code = error` on the root turn span so it appears
   as a failure in the Status filter on `/traces`.

## Out of Scope

- Slack thread ID → conversation ID lookup (requires Slack API integration)
- Real-time streaming updates to the traces/logs pages
- Changes to the NATS redelivery / ack behaviour
