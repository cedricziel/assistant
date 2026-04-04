# Data Model: Cross-Platform Native App Frontend (005-flutter-frontend)

## Scope

This document covers:

1. **Dart client-side models** — data classes in the Flutter app.
2. **New Rust API response types** — JSON shapes for the 6 missing endpoints.

Existing backend database schema is unchanged. No new SQLite migrations are required.

---

## Dart Client Models (`app/lib/api/models/`)

### ServerProfile

Stored in `flutter_secure_storage`. Never sent to the server.

```dart
class ServerProfile {
  final String baseUrl;   // e.g. "http://localhost:8080"
  final String token;     // Bearer token (encrypted at rest)
  final String label;     // Display name, e.g. "Local Dev"
}
```

**Validation rules**:

- `baseUrl` MUST be a non-empty valid URL (http or https).
- `token` MUST be non-empty.
- `label` MAY be empty; defaults to the hostname component of `baseUrl`.

---

### Persona

```dart
class Persona {
  final String id;          // e.g. "default", "work"
  final String name;        // Display name
  final String description; // Short description (may be empty)
  final bool isDefault;     // Whether this is the server's default persona
}
```

**State transitions**: The app holds one `activePersona` at a time. Switching replaces it.

---

### ConversationSummary

Mirrors the existing `ConversationSummary` JSON from `GET /api/conversations`.

```dart
class ConversationSummary {
  final String id;          // UUID string
  final String title;
  final DateTime createdAt;
  final DateTime updatedAt;
}
```

---

### ConversationDetail

Mirrors `GET /api/conversations/{id}` with full message list.

```dart
class ConversationDetail {
  final String id;
  final String title;
  final DateTime createdAt;
  final DateTime updatedAt;
  final List<Message> messages;
}
```

---

### Message

```dart
class Message {
  final String id;        // UUID string
  final String role;      // "user" | "assistant"
  final String content;
  final int turn;
  final DateTime createdAt;
  // Tool calls are embedded in content as markdown; no separate field in v1
}
```

---

### StreamEvent

Represents a single SSE event from `POST /api/conversations/{id}/messages`.

```dart
sealed class StreamEvent {}

class TokenEvent extends StreamEvent {
  final String token;   // Incremental text chunk
}

class DoneEvent extends StreamEvent {
  final String role;
  final String content; // Full assembled reply
}

class ErrorEvent extends StreamEvent {
  final String message;
}
```

---

### Skill

```dart
class Skill {
  final String name;
  final String description; // May be empty
  final bool enabled;       // Whether active for this persona
}
```

---

### TraceSummary

```dart
class TraceSummary {
  final String traceId;
  final String personaId;
  final DateTime startTime;
  final int durationMs;     // Total turn duration
  final String? skillName;  // Skill used, if any
  final String status;      // "ok" | "error"
  final String? conversationId;
}
```

---

### TraceDetail

```dart
class TraceDetail {
  final String traceId;
  final String personaId;
  final DateTime startTime;
  final int durationMs;
  final List<SpanEntry> spans;
}

class SpanEntry {
  final String spanId;
  final String name;          // e.g. "llm_turn", "tool:file-read"
  final DateTime startTime;
  final int durationMs;
  final Map<String, String> attributes;
}
```

---

### LogEntry

```dart
class LogEntry {
  final String id;
  final DateTime timestamp;
  final String severity;    // "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR"
  final String target;      // Rust module path (e.g. "assistant_runtime::orchestrator")
  final String message;
  final Map<String, String> fields;  // Key-value structured fields
  final String? traceId;
  final String? conversationId;
}
```

---

## New Rust API Response Types

### `GET /api/personas` → `Vec<PersonaSummary>`

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

### `POST /api/personas/active` → `PersonaSummary`

Request body:

```json
{ "id": "work" }
```

Response (200):

```json
{
  "id": "work",
  "name": "Work Mode",
  "description": "Focused on professional tasks",
  "is_default": false
}
```

Error (404): persona ID not found.

---

### `GET /api/personas/{id}/skills` → `Vec<SkillEntry>`

```json
[
  {
    "name": "web-fetch",
    "description": "Fetch content from a URL",
    "enabled": true
  },
  {
    "name": "file-read",
    "description": "Read files from the filesystem",
    "enabled": false
  }
]
```

---

### `GET /api/traces?limit=50&offset=0&since=<ISO>&until=<ISO>&skill=<name>&status=ok|error`

→ `Vec<TraceSummary>`

```json
[
  {
    "trace_id": "01234abc...",
    "persona_id": "default",
    "start_time": "2026-04-04T10:00:00Z",
    "duration_ms": 1234,
    "skill_name": null,
    "status": "ok",
    "conversation_id": "uuid-here"
  }
]
```

---

### `GET /api/traces/{trace_id}` → `TraceDetail`

```json
{
  "trace_id": "01234abc...",
  "persona_id": "default",
  "start_time": "2026-04-04T10:00:00Z",
  "duration_ms": 1234,
  "spans": [
    {
      "span_id": "aabb...",
      "name": "orchestrator_turn",
      "start_time": "2026-04-04T10:00:00Z",
      "duration_ms": 1234,
      "attributes": {
        "interface": "web",
        "conversation_id": "uuid-here"
      }
    }
  ]
}
```

---

### `GET /api/logs?limit=100&offset=0&search=<keyword>&severity=INFO&since=<ISO>&until=<ISO>`

→ `Vec<LogEntry>`

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
    "trace_id": "01234abc",
    "conversation_id": "uuid-here"
  }
]
```

---

## Entity Relationships

```
ServerProfile (local)
    └── active persona id (stored locally)

Persona (from server)
    └── has many Skills (read-only)

Conversation (from server, scoped to Persona)
    └── has many Messages

Trace (from server)
    ├── belongs to Persona
    ├── may reference Conversation
    └── has many Spans

LogEntry (from server)
    ├── may reference Trace
    └── may reference Conversation
```
