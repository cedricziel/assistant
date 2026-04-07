## Context

The Flutter app currently targets two distinct deployment modes:

1. **Web SPA** — Flutter web build is compiled into the Rust binary via `rust-embed` and served at runtime. Users access it in a browser pointed at the running server.
2. **Native macOS app** — `flutter build macos` produces a `.app` bundle. Users must still start the Rust server separately and enter its URL + token manually.

Mode 2 creates unnecessary friction for desktop users. A standalone macOS app should work out-of-the-box without a separate server process.

The Rust `assistant` CLI already supports `webui serve --listen <addr> --auth-token <token>` — all functionality needed for an embedded backend is already present.

## Goals / Non-Goals

**Goals:**

- macOS `.app` bundle ships with the `assistant` Rust binary as a bundled resource.
- Flutter app detects it is running in embedded mode and auto-starts the binary on launch.
- App auto-generates an ephemeral token and picks a free port; no user configuration required.
- App gracefully shuts down the child process when the app exits.
- Connection setup screen auto-skips (or offers "Embedded" as the default option) on supported platforms.

**Non-Goals:**

- Embedding on iOS, Android, or Linux (macOS only for this change).
- Updating the Rust binary from within the Flutter app.
- Multiple simultaneous embedded server instances.
- Changes to the Rust server's API surface.

## Decisions

### D1: Binary distribution — bundle resource vs. separate download

**Decision:** Embed the pre-compiled `assistant` binary as a macOS bundle resource (`app/macos/Runner/Resources/assistant`).

**Rationale:** Simplest distribution model — the `.app` is self-contained. No download step, no version mismatch risk.

**Alternative considered:** Download on first launch. Rejected: requires internet access, adds complexity, and complicates code-signing.

---

### D2: Process management — Dart `Process.start` vs. platform channel

**Decision:** Use Dart's `dart:io` `Process.start()` directly from a Riverpod `AsyncNotifier`.

**Rationale:** `dart:io` is available on macOS and provides `ProcessSignal` for graceful shutdown. No native code required.

**Alternative considered:** MethodChannel to a Swift plugin. Rejected: unnecessary complexity when Dart already has process APIs.

---

### D3: Port selection — fixed vs. dynamic

**Decision:** Pick a random free port at startup using a `ServerSocket.bind(InternetAddress.loopbackIPv4, 0)` probe, then release and use that port.

**Rationale:** Avoids port conflicts if the user is also running the server separately. The token is also generated randomly (UUID v4).

**Alternative considered:** Fixed port (e.g., 18080). Rejected: conflicts with a manually running server.

---

### D4: Startup handshake — polling vs. stdout parsing

**Decision:** Poll `GET /health` with exponential backoff (max 10 attempts, starting at 200 ms) before marking the server ready.

**Rationale:** Simple and robust. The `/health` endpoint already exists.

**Alternative considered:** Parse stdout for a "server ready" line. Rejected: coupling to log output format which may change.

---

### D5: Auth token — user-visible vs. ephemeral hidden

**Decision:** Generate a UUID v4 token at each launch; never display it to the user in embedded mode. Store it only in memory for the lifetime of the app session.

**Rationale:** Embedded mode is single-user local; exposing the token adds no security benefit and creates confusion.

---

### D6: Connection screen behavior

**Decision:** Introduce a `ServerMode` enum (`embedded`, `remote`). On macOS with the bundled binary present, default to `embedded`. The setup screen shows a toggle between modes; `embedded` requires no URL/token input.

**Rationale:** Users should still be able to switch to a remote server (e.g., for a shared team instance). The toggle preserves that flexibility.

---

### D7: Build pipeline — when to cross-compile the Rust binary

**Decision:** Add a `make build-macos-bundle` target that:

1. Runs `cargo build --release -p assistant-cli` (cross-compile for `aarch64-apple-darwin` and `x86_64-apple-darwin`, then `lipo` into a universal binary).
2. Copies the result to `app/macos/Runner/Resources/assistant`.
3. Runs `flutter build macos --release`.

The binary is **not** embedded at compile time (unlike the web SPA) — it is copied as a file resource and code-signed as part of the standard Xcode build.

**Alternative considered:** Embedding via `rust-embed` inside another crate. Rejected: macOS app bundles already have a standard resource mechanism; duplicating the binary in memory is wasteful.

## Risks / Trade-offs

| Risk                                                    | Mitigation                                                                                                                           |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| macOS Gatekeeper / notarization rejects unsigned binary | Sign and notarize the `assistant` binary as part of the Xcode archive step; add the binary to `app/macos/Runner/Runner.entitlements` |
| Child process survives app crash                        | Register a `ProcessSignal.sigterm` handler; on macOS the sandbox will also kill child processes when the parent exits                |
| Port probe is a TOCTOU race                             | Highly unlikely on loopback; acceptable risk for a local desktop app                                                                 |
| Binary architecture mismatch (Intel vs Apple Silicon)   | Produce a universal binary via `lipo`; Rosetta 2 is a fallback for x86_64                                                            |
| Flutter web build still works unchanged                 | The web path uses `rust-embed` and is not affected by this change                                                                    |

## Migration Plan

1. Existing users who connect to a remote server are unaffected — `remote` mode is still supported.
2. New macOS installs default to `embedded` mode automatically.
3. No database migration required.
4. Rollback: remove the `assistant` binary from the bundle resources and revert the `EmbeddedServerService` import — the app falls back to `remote`-only mode.

## Open Questions

- Should the embedded binary auto-update (e.g., check a GitHub release)? — **Deferred** to a future change.
- Should we support Windows in a follow-up? — **Out of scope** for this change; the same approach (`Process.start`) works on Windows but requires a `.exe` and different code-signing.
