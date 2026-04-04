# Research: Cross-Platform Native App Frontend (005-flutter-frontend)

## Technology Stack

### Decision: Flutter + Dart for cross-platform UI

**Decision**: Flutter (stable channel, Dart 3.x) as the cross-platform UI framework.

**Rationale**:

- Single codebase compiles to web (WASM/JS), macOS native, iOS, Android, Windows, Linux.
- First-class SSE support via `dart:async` `Stream` + `http` package.
- Strong widget toolkit that renders identically on all platforms.
- `flutter build web` and `flutter build macos` produce distributable artefacts
  without any additional bundler — exactly the packaging requirement in FR-007.
- The Dart `http` package supports streaming response bodies, enabling token-by-token
  SSE consumption without third-party streaming libraries.

**Alternatives considered**:

- React Native: web support exists but is an afterthought; no single-command macOS
  desktop build.
- Electron + web: heavy runtime, no mobile path, no single codebase story.
- Swift/Kotlin native: platform-specific, contradicts the cross-platform requirement.

---

### Decision: Riverpod for state management

**Decision**: `flutter_riverpod` (Riverpod 2.x) as the state management solution.

**Rationale**:

- Compile-time safe providers avoid the runtime errors of `Provider` package.
- `AsyncNotifier` and `StreamProvider` map cleanly to the SSE streaming model
  (a `StreamProvider` naturally represents the live token stream).
- Works well on all Flutter platforms including web.

**Alternatives considered**:

- BLoC: more boilerplate, no advantage for this size of app.
- `provider` package: runtime type errors, less structured for async flows.

---

### Decision: go_router for navigation

**Decision**: `go_router` for declarative URL-based navigation.

**Rationale**:

- URL-based routing works correctly for Flutter web (deep links, browser back button).
- Also works on macOS desktop without changes.
- Path parameters map naturally to conversation IDs and persona IDs.

---

### Decision: `flutter_secure_storage` for server profile persistence

**Decision**: `flutter_secure_storage` to store the server URL and bearer token.

**Rationale**:

- Encrypts credentials at rest using platform keychain/keystore on macOS and web.
- No credentials stored in plain SharedPreferences.
- Satisfies Principle X (secrets must not appear in logs or unencrypted storage).

---

### Decision: Feature-sliced directory structure

**Decision**: Organise Flutter code by feature slice, not by layer.

```
app/lib/
├── api/           # HTTP client, SSE parser, shared models
├── features/
│   ├── connection/    # Server profile setup (US2)
│   ├── chat/          # Streaming chat (US1)
│   ├── personas/      # Persona picker (US3)
│   ├── traces/        # Trace viewer (US4)
│   ├── logs/          # Log viewer (US4)
│   └── skills/        # Skill discovery (US5)
└── router/        # go_router configuration
```

**Rationale**: Each feature maps 1:1 to a user story. Features can be developed,
tested, and enabled/disabled independently — matching the speckit US-parallel task model.

---

## Backend API Gap Analysis

The existing `assistant-web-ui` crate already exposes:

| Endpoint                                | Status    | Notes                             |
| --------------------------------------- | --------- | --------------------------------- |
| `GET /api/conversations`                | ✅ Exists | Full JSON, auth required          |
| `POST /api/conversations`               | ✅ Exists |                                   |
| `GET /api/conversations/{id}`           | ✅ Exists | Includes message history          |
| `DELETE /api/conversations/{id}`        | ✅ Exists |                                   |
| `PATCH /api/conversations/{id}`         | ✅ Exists | Title update                      |
| `POST /api/conversations/{id}/messages` | ✅ Exists | SSE: `event:token` + `event:done` |
| `GET /health`                           | ✅ Exists | Used for connection validation    |

The following endpoints do NOT yet exist and must be added as part of this feature:

| Endpoint                        | Needed For                                 |
| ------------------------------- | ------------------------------------------ |
| `GET /api/personas`             | US3 — list available personas              |
| `POST /api/personas/active`     | US3 — switch active persona                |
| `GET /api/personas/{id}/skills` | US5 — list skills for a persona            |
| `GET /api/traces`               | US4 — list traces (JSON)                   |
| `GET /api/traces/{id}`          | US4 — trace detail with spans (JSON)       |
| `GET /api/logs`                 | US4 — list logs with keyword filter (JSON) |

All new endpoints MUST:

- Require `Authorization: Bearer <token>` (same as existing API).
- Return `application/json`.
- Follow the same response envelope pattern as the existing conversation API.
- Be registered in the `utoipa` OpenAPI doc.

---

## SSE Stream Format (existing, confirmed)

From `crates/web-ui/src/api/mod.rs`:

```
POST /api/conversations/{id}/messages
Content-Type: application/json
Authorization: Bearer <token>

{"message": "hello"}

→ HTTP 200  text/event-stream

event:token
data: Hello

event:token
data: ,

event:token
data:  world

event:done
data: {"role":"assistant","content":"Hello, world"}
```

The Flutter SSE client MUST handle both `event:token` (incremental append) and
`event:done` (finalize and store the full reply) event types.

---

## CORS Consideration

Flutter web runs as a browser app. Requests to the assistant server from a different
origin will be blocked unless the server adds CORS headers. The `assistant-web-ui`
server MUST be updated to emit `Access-Control-Allow-Origin: *` (or a configurable
list) and `Access-Control-Allow-Headers: Authorization, Content-Type` on API routes
when the Flutter web build is served from a different origin.

This is a new backend requirement. It MUST be addressed alongside the new API endpoints.

---

## Build & Distribution

| Platform | Command               | Output                                                      |
| -------- | --------------------- | ----------------------------------------------------------- |
| Web      | `flutter build web`   | `app/build/web/` (static files, serve with any HTTP server) |
| macOS    | `flutter build macos` | `app/build/macos/Build/Products/Release/*.app`              |

The macOS `.app` bundle can be zipped and distributed directly (unsigned, for
self-hosted/developer use). A notarized App Store build is out of scope for v1.

The Flutter web build is a static site — it is NOT served by the Rust backend.
It is deployed separately (e.g., on the same host via nginx, or as a GitHub Pages
release). The Rust server provides only the API; the UI is a separate static asset.
