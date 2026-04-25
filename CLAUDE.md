# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

## API Client Generation

The Flutter app's `app/packages/assistant_api/` package is **generated code** — never edit it manually. The workflow when changing API endpoints:

```sh
make dump-openapi           # re-export openapi.json from the running server
make generate-flutter-client # regenerate Dart/dio client from openapi.json
```

Requires `openapi-generator` (`brew install openapi-generator`).

## Build Integration

The Rust build embeds the Flutter web app via `rust-embed`. `cargo build -p assistant-cli` (or `make build`) triggers `build.rs`, which runs `flutter build web --release` in `app/`. This means:

- A Flutter SDK must be on `PATH` for a full Rust build.
- Use `make check` (`cargo check`) to skip Flutter and validate Rust only.

## Runtime Data

All runtime data lives under `~/.assistant/`. The multi-org layout:

- `server.toml` — global config: listen address, log level
- `orgs/{slug}/org.toml` — per-org config: auth mode, LLM providers
- `orgs/{slug}/org.db` — org-level database (users, spaces, memberships, API keys, OAuth clients)
- `orgs/{slug}/spaces/{space}/space.db` — space-level database (conversations, messages, personas, scheduled tasks)
- `orgs/{slug}/spaces/{space}/agents/{id}/` — per-persona agent workspace
- `orgs/{slug}/spaces/{space}/skills/` — space-local skills
- `orgs/{slug}/catalog/` — org-level shared skills, templates, interfaces

Legacy single-user installs are auto-migrated to `orgs/default/spaces/default/` on first startup.

## Authentication

The assistant runs its own OAuth2 Authorization Server (`assistant-auth` crate). Two modes:

- **Password mode** — local email/password credentials, server issues JWTs
- **OIDC mode** — delegates authentication to an external IdP (Keycloak, Auth0, etc.), server still issues its own JWTs for authorization

Key OAuth2 endpoints: `/oauth/authorize`, `/oauth/token`, `/oauth/register` (RFC 7591 dynamic client registration), `/oauth/device` (RFC 8628 device code flow), `/.well-known/oauth-authorization-server` (RFC 8414 metadata).

API keys (`ask_live_...`) provide scoped access as an alternative to OAuth tokens. Both resolve to the same `AuthContext` carrying user identity, org, space roles, and scopes.

## OpenAPI Spec

`openapi.json` at the repo root is the committed spec snapshot. Keep it up to date when modifying routes in `crates/web-ui`.
