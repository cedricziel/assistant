# API Contracts: Cross-Platform Native App Frontend (005-flutter-frontend)

All `/api/*` endpoints require (public endpoints such as `/health` are exempt):

```http
Authorization: Bearer <token>
```

All responses are `application/json` unless noted.

Error responses follow the shape:

```json
{ "error": "human-readable message" }
```

---

## Existing Endpoints (unchanged, Flutter must consume)

### GET /health

No auth required. Used for connection validation on profile setup.

**Response 200**:

```json
{ "status": "ok" }
```

---

### GET /api/conversations

**Response 200**:

```json
[
  {
    "id": "uuid",
    "title": "My first chat",
    "created_at": "2026-04-04T10:00:00Z",
    "updated_at": "2026-04-04T10:05:00Z"
  }
]
```

---

### POST /api/conversations

**Request**:

```json
{ "title": "Optional title" }
```

**Response 201**:

```json
{
  "id": "uuid",
  "title": "Optional title",
  "created_at": "2026-04-04T10:00:00Z",
  "updated_at": "2026-04-04T10:00:00Z"
}
```

---

### GET /api/conversations/{id}

**Response 200**:

```json
{
  "id": "uuid",
  "title": "My first chat",
  "created_at": "2026-04-04T10:00:00Z",
  "updated_at": "2026-04-04T10:05:00Z",
  "messages": [
    {
      "id": "uuid",
      "role": "user",
      "content": "Hello",
      "turn": 1,
      "created_at": "2026-04-04T10:00:01Z"
    },
    {
      "id": "uuid",
      "role": "assistant",
      "content": "Hi there!",
      "turn": 1,
      "created_at": "2026-04-04T10:00:02Z"
    }
  ]
}
```

**Response 404**: `{ "error": "Conversation not found" }`

---

### DELETE /api/conversations/{id}

**Response 204**: No content.

---

### PATCH /api/conversations/{id}

**Request**:

```json
{ "title": "New title" }
```

**Response 200**: Updated `ConversationSummary`.

---

### POST /api/conversations/{id}/messages

**Request** (`application/json`):

```json
{ "message": "Hello, assistant!" }
```

**Response 200** (`text/event-stream`):

```text
event:token
data: Hello

event:token
data: ,

event:token
data:  world!

event:done
data: {"role":"assistant","content":"Hello, world!"}
```

Flutter SSE client MUST:

1. Accumulate `event:token` data values into a display buffer (append each chunk).
2. On `event:done`, parse the JSON body, discard the buffer, and persist
   the canonical `content` value as the final message.
3. On stream close without `event:done`, treat the accumulated buffer as the
   final content and show a "response may be incomplete" indicator.

---

## New Endpoints (must be implemented in this feature)

### GET /api/personas

List all personas defined on the server.

**Response 200**:

```json
[
  {
    "id": "default",
    "name": "Default Assistant",
    "description": "General-purpose assistant",
    "is_default": true
  },
  {
    "id": "work",
    "name": "Work Mode",
    "description": "Focused on professional tasks",
    "is_default": false
  }
]
```

---

### POST /api/personas/active

Switch the active persona for the current session. Conversations created after
this call will be associated with the new persona.

**Request**:

```json
{ "id": "work" }
```

**Response 200**:

```json
{
  "id": "work",
  "name": "Work Mode",
  "description": "Focused on professional tasks",
  "is_default": false
}
```

**Response 404**: `{ "error": "Persona not found" }`

---

### GET /api/personas/{id}/skills

List skills associated with a persona, with their enabled/disabled state.

**Response 200**:

```json
[
  {
    "name": "web-fetch",
    "description": "Fetch and summarise web pages",
    "enabled": true
  },
  {
    "name": "file-read",
    "description": "Read files from the filesystem",
    "enabled": false
  }
]
```

**Response 404**: `{ "error": "Persona not found" }`

---

### GET /api/traces

List recent traces (orchestration turns), newest first.

**Query parameters**:

| Parameter      | Type            | Description                                |
| -------------- | --------------- | ------------------------------------------ |
| `limit`        | integer         | Max results (default 50, max 200)          |
| `offset`       | integer         | Pagination offset (default 0)              |
| `since`        | ISO 8601        | Filter to traces starting after this time  |
| `until`        | ISO 8601        | Filter to traces starting before this time |
| `skill`        | string          | Filter to traces that used this skill      |
| `status`       | `ok` \| `error` | Filter by outcome status                   |
| `conversation` | UUID            | Filter to traces for this conversation     |

**Response 200**:

```json
[
  {
    "trace_id": "01234abcdef...",
    "persona_id": "default",
    "start_time": "2026-04-04T10:00:00Z",
    "duration_ms": 1432,
    "skill_name": null,
    "status": "ok",
    "conversation_id": "uuid-here"
  }
]
```

---

### GET /api/traces/{trace_id}

Get a single trace with its span breakdown.

**Response 200**:

```json
{
  "trace_id": "01234abcdef...",
  "persona_id": "default",
  "start_time": "2026-04-04T10:00:00Z",
  "duration_ms": 1432,
  "spans": [
    {
      "span_id": "aabbcc...",
      "name": "orchestrator_turn",
      "start_time": "2026-04-04T10:00:00Z",
      "duration_ms": 1432,
      "attributes": {
        "interface": "web",
        "conversation_id": "uuid-here",
        "model": "llama3"
      }
    },
    {
      "span_id": "ddeeff...",
      "name": "tool:web-fetch",
      "start_time": "2026-04-04T10:00:00.400Z",
      "duration_ms": 300,
      "attributes": {
        "url": "https://example.com"
      }
    }
  ]
}
```

**Response 404**: `{ "error": "Trace not found" }`

---

### GET /api/logs

List recent log entries, newest first.

**Query parameters**:

| Parameter      | Type                                      | Description                                    |
| -------------- | ----------------------------------------- | ---------------------------------------------- |
| `limit`        | integer                                   | Max results (default 100, max 500)             |
| `offset`       | integer                                   | Pagination offset (default 0)                  |
| `search`       | string                                    | Keyword filter (applied to message and fields) |
| `severity`     | `TRACE`\|`DEBUG`\|`INFO`\|`WARN`\|`ERROR` | Minimum severity level                         |
| `since`        | ISO 8601                                  | Filter to logs after this time                 |
| `until`        | ISO 8601                                  | Filter to logs before this time                |
| `trace_id`     | string                                    | Filter to logs associated with a trace         |
| `conversation` | UUID                                      | Filter to logs for a conversation              |

**Response 200**:

```json
[
  {
    "id": "01234",
    "timestamp": "2026-04-04T10:00:00.123Z",
    "severity": "INFO",
    "target": "assistant_runtime::orchestrator",
    "message": "Turn complete",
    "fields": {
      "conversation_id": "uuid-here",
      "turns": "3"
    },
    "trace_id": "01234abcdef",
    "conversation_id": "uuid-here"
  }
]
```

---

## CORS Headers (new requirement)

The assistant server MUST emit the following headers on all `/api/*` routes when
the `Origin` request header is present:

```http
Access-Control-Allow-Origin: *
Access-Control-Allow-Headers: Authorization, Content-Type
Access-Control-Allow-Methods: GET, POST, PUT, PATCH, DELETE, OPTIONS
```

For SSE endpoints, additionally:

```http
Access-Control-Allow-Origin: *
```

This is required for the Flutter web build to make cross-origin requests from
a browser. The `--cors-origin` flag (or `ASSISTANT_WEB_CORS_ORIGIN` env var)
SHOULD be added to allow operators to restrict to a specific origin.
