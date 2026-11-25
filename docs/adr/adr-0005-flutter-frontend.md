# ADR-0005: Flutter Cross-Platform Frontend

**Status**: Accepted
**Date**: 2026-04-04

## Context

The assistant web UI was originally a server-side rendered (SSR) Axum
application using Askama templates and HTMX for lightweight interactivity.
This worked for the observability dashboard (traces, logs, analytics) but
made it hard to build a real chat interface with streaming responses, and
produced no native macOS experience.

Requirements driving this change:

1. **Streaming chat** — SSE token streaming needs incremental DOM updates
   that HTMX can handle but with significant complexity.
2. **Cross-platform** — users want a native macOS app, not just a web page.
3. **Single binary deployment** — the server must still ship as one binary;
   a separate frontend server is not acceptable.
4. **Shared logic** — auth, routing, and API calls should not be duplicated
   between a web build and a native build.

## Decision

Replace the SSR HTML interface with a **Flutter 3.x application** targeting
both web and macOS, embedded in the Rust binary at compile time.

### Build integration

Cargo never runs Flutter. `make build-web` (`flutter build web --release`
in `app/`) is an explicit step that writes `app/build/web/`;
`crates/web-ui/build.rs` embeds that directory via `rust-embed`, or a
placeholder `index.html` when it does not exist, so the crate always
compiles. Release and Docker workflows run the web build before cargo.
All unmatched HTTP routes fall through to the Flutter SPA handler so
`go_router` can manage client-side navigation.

### API layer

A REST API (`/api/*`) replaces the old SSR page handlers as the primary
interface. The Flutter app communicates exclusively through this API using
a generated Dart client (`assistant_api`, produced by openapi-generator
dart-dio). SSE streaming for chat messages is handled by a thin hand-written
wrapper on top of `Dio` because openapi-generator cannot model streaming
responses.

### Credential storage

- **Web**: server URL is auto-detected from `window.location.origin`; only
  the token is stored in `localStorage` via `flutter_secure_storage`.
- **macOS**: the user enters both server URL and token; both are stored in
  the macOS Keychain via `flutter_secure_storage`.

### Retained SSR pages

The Askama-based management pages for A2A Profiles, webhooks, analytics,
and workflow graphs are retained as-is. They serve a different audience
(operators, not end-users) and do not need the chat-focused UX.

## Consequences

**Good:**

- Real-time streaming chat works cleanly in Flutter via `async*` generators.
- macOS native app with Keychain credential storage ships from the same
  codebase with `flutter build macos`.
- Single binary deployment preserved — `assistant webui serve` still serves
  everything.
- OpenAPI-generated client keeps the Dart types in sync with the Rust server.

**Neutral:**

- A binary built with plain `cargo build` embeds a placeholder page; shipping
  the real UI needs `make build-web` first. CI runners that build release
  artifacts install Flutter with `subosito/flutter-action`.
- Rust-only builds and tests no longer pay for `flutter build web`.

**Bad / trade-offs:**

- The binary is larger (~8 MB of compressed Flutter web assets embedded).
- Two language ecosystems (Rust + Dart) to maintain; contributors need
  both toolchains for full-stack changes.
- The generated client must be regenerated whenever the OpenAPI spec changes
  (`cd app && dart run build_runner build`).
