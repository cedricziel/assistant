# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

## API Client Generation

The Flutter app's `app/packages/assistant_api/` package is **generated code** — never edit it manually. The workflow when changing API endpoints:

```sh
make dump-openapi           # re-export openapi.json from the running server
make generate-flutter-client # regenerate Dart/dio client from openapi.json
```

Requires Docker: the generator runs from the pinned `openapitools/openapi-generator-cli` image (see `OPENAPI_GENERATOR_VERSION` in the Makefile).

## Build Integration

The Rust build embeds the Flutter web app via `rust-embed`, but cargo never runs Flutter. Building the web app is an explicit step:

- `make build-web` runs `flutter build web --release` in `app/`, producing `app/build/web/`. `make build`, `make build-release` and `make build-macos-binary` run it first.
- `crates/web-ui/build.rs` embeds `app/build/web/` if it exists, otherwise a placeholder page. A bare `cargo build -p assistant-cli` therefore never needs Flutter, but its UI is a placeholder until `make build-web` has run.
- Release and Docker workflows build the web app first (`flutter build web` job or `make build-web`) so shipped binaries embed the real UI.

## Runtime Data

All runtime data lives under `~/.assistant/`. The multi-org layout:

- `server.toml` — process-wide settings, loaded once at startup: message bus (`[bus]`), external MCP servers (`[mcp]`), Web Push keys (`[notifications]`)
- `orgs/{slug}/org.toml` — per-org settings, loaded when the org's first space is used: LLM providers (`[llm]`), Persona, memory, skills, permissions, messenger and speech credentials. A space gets its LLM and settings from its own org.
- `org.db` — org-level database at the install root (users, spaces, memberships, API keys, OAuth clients, integration credentials)
- `secrets.key`, `jwt_secret` — keys for integration credentials and issued JWTs; `assistant backup` includes them
- `agents/{id}/`, `skills/` — install-root persona workspaces and user skills
- `cache/` — regenerable downloads (integrations catalog); not backed up
- `orgs/{slug}/spaces/{space}/space.db` — space-level database (conversations, messages, personas, scheduled tasks)
- `orgs/{slug}/spaces/{space}/agents/{id}/` — per-persona agent workspace
- `orgs/{slug}/spaces/{space}/skills/` — space-local skills
- `orgs/{slug}/catalog/` — org-level shared skills, templates, interfaces

The runtime does not read `config.toml`. See `docs/operations/configuration.md`.

There is no migration from the old single-user layout (`~/.assistant/assistant.db`, `config.toml`); those installs start fresh.

## Authentication

The assistant runs its own OAuth2 Authorization Server (`assistant-auth` crate). Two modes:

- **Password mode** — local email/password credentials, server issues JWTs
- **OIDC mode** — delegates authentication to an external IdP (Keycloak, Auth0, etc.), server still issues its own JWTs for authorization

Key OAuth2 endpoints: `/oauth/authorize`, `/oauth/token`, `/oauth/register` (RFC 7591 dynamic client registration), `/oauth/device` (RFC 8628 device code flow), `/.well-known/oauth-authorization-server` (RFC 8414 metadata).

API keys (`ask_live_...`) provide scoped access as an alternative to OAuth tokens. Both resolve to the same `AuthContext` carrying user identity, org, space roles, and scopes.

## OpenAPI Spec

`openapi.json` at the repo root is the committed spec snapshot. Keep it up to date when modifying routes in `crates/web-ui`.
