# AGENTS.md — messenger interfaces

All messenger interface adapters live in `crates/interfaces/` (`assistant-interfaces`) as sub-modules (`slack`, `mattermost`, `matrix`, `nextcloud`, `signal`). They use **thin `reqwest` + `tokio-tungstenite` HTTP/WebSocket clients** — not heavy platform SDKs. This was a deliberate decision (see `openspec/changes/thin-messenger-http-clients/design.md`):

- **Matrix**: uses plain long-poll `/sync` via `reqwest`, _not_ `matrix-sdk`. The old `matrix-sdk` dep was removed entirely (including from `Cargo.toml`) to eliminate its 80k+ line transitive footprint and SQLite state store.
- **Slack**: uses `reqwest` + `tokio-tungstenite` Socket Mode, _not_ `slack-morphism`.
- **Mattermost**: uses `reqwest` + `tokio-tungstenite`, _not_ `mattermost_api`.
- **Signal**: uses a thin `reqwest` + `tokio-tungstenite` client against [signal-cli-rest-api](https://github.com/bbernhard/signal-cli-rest-api). Receives messages via WebSocket `GET /v1/receive/{number}` and sends via `POST /v1/send`. No `presage` dependency; no feature flag required. Operators must run the signal-cli daemon separately.

All messenger interfaces compile unconditionally as part of this crate.
