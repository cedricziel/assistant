## Architecture

### Data Model

Artifacts are stored server-side in SQLite, associated with conversations and optionally with specific messages. Each artifact tracks its version history.

```
┌──────────────────────────────────────────────────────┐
│                    artifacts                          │
├──────────────────────────────────────────────────────┤
│ id              TEXT PRIMARY KEY  (UUID)              │
│ conversation_id TEXT NOT NULL     (FK → conversations)│
│ message_id      TEXT             (FK → messages, opt) │
│ agent_id        TEXT NOT NULL                         │
│ name            TEXT                                  │
│ description     TEXT                                  │
│ artifact_type   TEXT NOT NULL     (code, markdown,    │
│                                    data, file)        │
│ media_type      TEXT              (MIME type)         │
│ language        TEXT              (for code artifacts) │
│ created_at      DATETIME NOT NULL                     │
│ updated_at      DATETIME NOT NULL                     │
└──────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────┐
│                 artifact_versions                     │
├──────────────────────────────────────────────────────┤
│ id              TEXT PRIMARY KEY  (UUID)              │
│ artifact_id     TEXT NOT NULL     (FK → artifacts)    │
│ message_id      TEXT              (FK → messages)     │
│ version         INTEGER NOT NULL                      │
│ content         TEXT NOT NULL     (full content)      │
│ created_at      DATETIME NOT NULL                     │
└──────────────────────────────────────────────────────┘
```

### Artifact Flow

```
   Tool / LLM produces output
              │
              ▼
   ┌──────────────────────┐
   │  Runtime Orchestrator │
   │                      │
   │  Detects artifact-   │
   │  worthy content:     │
   │  - ToolOutput with   │
   │    attachment/data   │
   │  - Code blocks >N    │
   │    lines in LLM      │
   │    response          │
   │  - Explicit artifact │
   │    tool call         │
   └──────────┬───────────┘
              │
              ▼
   ┌──────────────────────┐
   │  Storage Layer        │
   │                      │
   │  Insert/update       │
   │  artifact +          │
   │  version record      │
   └──────────┬───────────┘
              │
              ├──── SSE: ArtifactEvent ──────► Flutter UI
              │     (artifact_id, name,        (opens side panel,
              │      type, content,             renders by type)
              │      version, append,
              │      last_chunk)
              │
              ├──── Slack formatter ──────────► Code block / file upload
              │
              ├──── Matrix formatter ─────────► Formatted message
              │
              └──── CLI formatter ────────────► Syntax-highlighted output
```

### SSE Event Design

Two new events added to the chat streaming endpoint:

```
event: artifact
data: {
  "artifact_id": "uuid",
  "name": "parse_csv.py",
  "description": "CSV parser with error handling",
  "artifact_type": "code",
  "language": "python",
  "version": 1,
  "content": "import csv\n...",
  "append": false,
  "last_chunk": false
}

event: artifact_update
data: {
  "artifact_id": "uuid",
  "content": "...continued content...",
  "append": true,
  "last_chunk": true
}
```

These mirror the A2A `TaskArtifactUpdateEvent` semantics — `append: true` for streaming chunks within a turn, new `version` for cross-turn updates.

### Artifact Detection Strategy

The orchestrator uses a layered detection approach:

1. **Explicit**: A tool returns `ToolOutput` with `with_artifact(name, type)` — always an artifact
2. **Tool-based**: Certain tools always produce artifacts (e.g. `file-read` → code artifact, `web-fetch` → data artifact)
3. **Heuristic**: LLM response contains a fenced code block >20 lines → candidate artifact (provider-agnostic, works with Anthropic, OpenAI, Ollama)

The heuristic layer is important because it works across all LLM providers without requiring provider-specific system prompts.

### Flutter UI

```
┌──────────────────────────────────────────────────────────┐
│  Chat Screen (modified layout)                            │
│                                                           │
│  ┌─────────────────────┬────────────────────────────────┐ │
│  │   Message List      │    Artifact Panel              │ │
│  │                     │    (conditionally shown)       │ │
│  │   [user msg]        │    ┌────────────────────────┐  │ │
│  │                     │    │ parse_csv.py    v1 ▼   │  │ │
│  │   [assistant msg    │    ├────────────────────────┤  │ │
│  │    with artifact    │    │                        │  │ │
│  │    indicator ►]     │    │  import csv            │  │ │
│  │                     │    │  import sys            │  │ │
│  │   [user msg]        │    │                        │  │ │
│  │                     │    │  def parse(path):      │  │ │
│  │   [assistant msg    │    │      with open(path)   │  │ │
│  │    artifact v2 ►]   │    │          ...           │  │ │
│  │                     │    │                        │  │ │
│  │                     │    ├────────────────────────┤  │ │
│  │                     │    │ [Copy] [Download] [v▼] │  │ │
│  │   ┌──────────────┐  │    └────────────────────────┘  │ │
│  │   │ Input box    │  │                                │ │
│  │   └──────────────┘  │                                │ │
│  └─────────────────────┴────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
```

The panel uses a responsive split layout:

- **Wide screens** (>900px): Side-by-side chat + artifact panel
- **Narrow screens** (<900px): Artifact opens as an overlay/bottom sheet
- **Mobile**: Full-screen artifact view with back navigation

### Interface Degradation

| Interface   | Artifact Rendering                                      |
| ----------- | ------------------------------------------------------- |
| Flutter Web | Full side panel with syntax hl, version picker, copy    |
| Flutter Mac | Same as web                                             |
| CLI         | Syntax-highlighted block in terminal, saved to tempfile |
| Slack       | Code block in thread reply; files >50 lines uploaded    |
| Mattermost  | Code block in thread; file attachment for large content |
| Matrix      | Formatted message with code fence; file for large       |
| Signal      | Plain text (limited formatting available)               |
| Nextcloud   | Rich text message with code blocks                      |
| A2A         | Native `Artifact` in `Task` response (already works)    |

### Relation to A2A Protocol

The core `Artifact` struct in `assistant-core` mirrors the A2A `Artifact` type but is simpler (single content string instead of `Vec<Part>`). Conversion between the two is straightforward:

```
Core Artifact ◄──────► A2A Artifact
  id                     artifact_id
  name                   name
  description            description
  content (String)  ◄──► parts[0].text (for text artifacts)
  media_type             parts[0].media_type
  artifact_type     ──►  metadata["artifact_type"]
```

## Key Decisions

1. **Server-side storage** over client-only: Artifacts persist across sessions and are available to all interfaces
2. **Version-per-turn** over append-only: Each conversation turn that modifies an artifact creates a new version, enabling diff view
3. **Heuristic detection** as fallback: Works with any LLM provider, no special system prompting needed
4. **Full content per version** over diffs: Simpler storage and retrieval, storage is cheap for text
