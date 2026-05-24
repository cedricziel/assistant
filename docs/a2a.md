# A2A (Agent-to-Agent) interface

The assistant exposes an [Agent-to-Agent (A2A)](https://a2aproject.github.io/A2A/)
protocol surface so other agents and A2A-compatible clients can drive it as a
standard agent. A2A is one of several front doors over the single Orchestrator
(alongside the `/api` web client, MCP, and the messengers); see the
`protocol-adapter-platform` design for the broader picture.

## Endpoints

All A2A routes are served by `assistant webui serve`.

| Method | Path                                  | Auth | Description                                 |
| ------ | ------------------------------------- | ---- | ------------------------------------------- |
| GET    | `/.well-known/agent.json`             | none | Public agent card (discovery)               |
| GET    | `/agent/authenticatedExtendedCard`    | yes  | Authenticated agent card                    |
| POST   | `/message/send`                       | yes  | Run one turn, return the final `Task`       |
| POST   | `/message/stream`                     | yes  | Run one turn, stream `StreamResponse` (SSE) |
| GET    | `/tasks` · `/tasks/{id}`              | yes  | List / fetch tasks                          |
| POST   | `/tasks/{id}/cancel`                  | yes  | Cancel an in-flight task                    |
| GET    | `/tasks/{id}/subscribe`               | yes  | Subscribe to task updates (SSE)             |
| \*     | `/tasks/{id}/pushNotificationConfigs` | yes  | Manage push-notification configs            |

The agent card at `/.well-known/agent.json` advertises the auth scheme, so a
caller learns it must present a Bearer token before making authenticated calls.

## Authentication & authorization

A2A calls authenticate exactly like `/api`: present an OAuth2 Bearer token or an
API key (`ask_live_...`). Both resolve to the same `AuthContext`. Posting a
message requires the **`conversations:write`** scope — callers without it get
`403 Forbidden`. Unauthenticated calls get `401`.

## Sending a message

```sh
curl -sN https://assistant.example.com/message/send \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{
        "message": {
          "messageId": "11111111-1111-1111-1111-111111111111",
          "role": "ROLE_USER",
          "parts": [{ "text": "Summarize today's incidents." }]
        }
      }'
```

The response is a `SendMessageResponse` containing the completed `Task`; the
agent's answer is the final message in `task.status.message`.

## Conversations & multi-turn context

Each A2A `Task` is backed by a real conversation. The first message (no
`context_id`) starts a new conversation and the response surfaces its id as the
task's `context_id`. **Send that `context_id` back on subsequent messages** to
continue the same conversation — the assistant maps it to the same conversation
so history accumulates across turns.

## Streaming

`POST /message/stream` returns Server-Sent Events. Each event's `data` is a
JSON `StreamResponse`: incremental `message` chunks and `statusUpdate` events
(tool calls, thinking, subagent activity) as the turn runs, ending with the
final `Completed` `Task` snapshot and a `[DONE]` marker.

## Persistence

A2A tasks are persisted in the space database (`a2a_tasks` table) and survive a
restart — `GET /tasks` and `/tasks/{id}` return them afterwards. Live SSE
subscriptions and push-notification configs are process-local and are **not**
persisted.

## Limitations

- Turns run against the active agent/space; per-request org/space re-scoping is
  not yet wired (turns use the server's active agent).
- Push-notification delivery is configured but not yet dispatched.
