## Context

The five messenger interface crates share the same architectural pattern:

| Crate                  | Modules                                                      | Unique deps (beyond shared set)                          |
| ---------------------- | ------------------------------------------------------------ | -------------------------------------------------------- |
| `interface-slack`      | adapter, client, config, runner, tools, skills/\* (8 skills) | `lru`, `assistant-storage`                               |
| `interface-mattermost` | adapter, client, config, runner, tools                       | —                                                        |
| `interface-matrix`     | adapter, client, config, runner, tools                       | `dirs`, `urlencoding`                                    |
| `interface-nextcloud`  | adapter, config, runner, signing, tools, types               | `axum`, `hmac`, `sha2`, `hex`, `getrandom`, `tokio-util` |
| `interface-signal`     | adapter, config, runner                                      | `base64`, `toml`                                         |

All five follow the same layered pattern: **config extension** → **client** (HTTP/WS) → **adapter** (implements `ChannelAdapter`) → **runner** (implements `InterfaceRunner`). The shared types (`ChannelAdapter`, `ChannelMessage`, `ChannelContent`, `ChannelUser`, `ChannelType`, config structs) already live in `assistant-core`.

The CLI (`interface-cli`) feature-gates each interface behind `slack`, `mattermost`, `matrix`, `nextcloud`, `signal` flags, all enabled by default.

## Goals / Non-Goals

**Goals:**

- Merge the five interface crates into a single `assistant-interfaces` crate at `crates/interfaces/`.
- Preserve the per-platform module boundary: `slack::`, `mattermost::`, `matrix::`, `nextcloud::`, `signal::`.
- Re-export only `XxxInterface` types publicly (adapters, clients, config extensions stay `pub(crate)` or module-private).
- Simplify the CLI: replace five optional deps + five feature flags with one non-optional dep.
- Consolidate shared helper logic (backoff/retry, transcription download patterns).

**Non-Goals:**

- Moving `ChannelAdapter` or config types out of `assistant-core`.
- Changing `InterfaceRunner`, `ChannelRunner`, or `Orchestrator` in `assistant-runtime`.
- Merging `interface-cli` into this crate.
- Adding new platform adapters.
- Changing any runtime behavior or public constructor signatures.

## Decisions

### 1. Crate name and location

**Decision**: `assistant-interfaces` at `crates/interfaces/`.

**Rationale**: Follows the existing naming pattern (`assistant-core`, `assistant-runtime`, `assistant-storage`). The plural "interfaces" signals it houses multiple platform modules. Path `crates/interfaces/` is short and unambiguous.

**Alternative considered**: `assistant-channels`. Rejected because the codebase already uses "interface" terminology in crate names and the `Interface` enum.

---

### 2. Module structure: flat vs nested

**Decision**: Nested modules, one per platform.

```text
crates/interfaces/src/
  lib.rs              # re-exports XxxInterface + XxxConfigExt
  common.rs           # shared backoff/retry helpers (extracted)
  slack/
    mod.rs            # pub(crate) adapter, client, config, tools; pub runner
    adapter.rs
    client.rs
    config.rs
    runner.rs
    tools.rs
    skills/
      mod.rs
      slack_post.rs
      ... (8 skill files)
  mattermost/
    mod.rs
    adapter.rs
    client.rs
    config.rs
    runner.rs
    tools.rs
  matrix/
    mod.rs
    adapter.rs
    client.rs
    config.rs
    runner.rs
    tools.rs
  nextcloud/
    mod.rs
    adapter.rs
    config.rs
    runner.rs
    signing.rs
    tools.rs
    types.rs
  signal/
    mod.rs
    adapter.rs
    config.rs
    runner.rs
```

**Rationale**: Each platform's code stays self-contained within its own module directory. File names don't need platform prefixes (e.g. `slack/adapter.rs` not `slack_adapter.rs`). Matches how the code is already organized within each crate — files move without renaming.

---

### 3. Visibility: what is public?

**Decision**: Only runner types and config extensions are `pub`.

From `lib.rs`:

```rust
pub use slack::SlackInterface;
pub use mattermost::MattermostInterface;
pub use matrix::MatrixInterface;
pub use nextcloud::{NextcloudInterface, NextcloudConfigExt};
pub use signal::{SignalInterface, SignalAdapter};  // SignalAdapter currently public
```

Config types (`SlackConfig`, `MattermostConfig`, etc.) are **not** re-exported — they live in `assistant-core` and consumers import them from there directly.

**Rationale**: Adapters, clients, and tools are internal implementation details. The CLI only needs `XxxInterface` to call `.new()` and `.run()`. `SignalAdapter` stays public for now to avoid a breaking change, but could be made `pub(crate)` in a follow-up.

---

### 4. Feature flags: keep or remove?

**Decision**: Remove per-interface feature flags from `interface-cli`. The consolidated crate compiles all platforms unconditionally.

**Rationale**: All five interfaces are already enabled by default. The feature flags existed to allow stripping a single interface, but nobody uses this — the binary is always built with all interfaces. Removing the flags simplifies `Cargo.toml` and eliminates 15+ `#[cfg(feature = "...")]` annotations from `main.rs`. If selective compilation is needed later, feature flags can be added to `assistant-interfaces` itself.

---

### 5. Shared helpers: extract common patterns

**Decision**: Create `crates/interfaces/src/common.rs` for shared logic.

Currently duplicated across Slack, Mattermost, and Signal:

- Exponential backoff constants (`BACKOFF_MIN`, `BACKOFF_MAX`) and `sleep_backoff()` + `rand_jitter()` helpers.
- Transcription download + decode patterns.

**Rationale**: Within a single crate, deduplication is straightforward. A `common` module avoids three copies of identical retry logic.

---

### 6. Cargo.toml: dependency union

**Decision**: The new crate's `[dependencies]` is the union of all five crates' deps.

Shared deps (used by all or most):
`assistant-core`, `assistant-runtime`, `assistant-transcription`, `tokio`, `serde`, `serde_json`, `anyhow`, `tracing`, `async-trait`, `uuid`, `chrono`, `reqwest`, `futures`, `tokio-stream`, `tokio-tungstenite`

Platform-specific deps:

- Slack: `lru`, `assistant-storage`
- Matrix: `dirs`, `urlencoding`
- Nextcloud: `axum`, `hmac`, `sha2`, `hex`, `getrandom`, `tokio-util`
- Signal: `base64`, `toml`

**Rationale**: The total dep footprint is unchanged — these deps already compile as part of the workspace. Combining them into one crate actually improves compile times slightly (one compilation unit instead of five).

## Migration Path

The migration is purely structural — no logic changes:

1. Create `crates/interfaces/` with `Cargo.toml` and `src/lib.rs`.
2. Copy each interface's `src/*.rs` into `src/<platform>/`, adjusting `crate::` imports to `crate::<platform>::` or `super::` as needed.
3. Create per-platform `mod.rs` controlling visibility.
4. Extract shared backoff helpers into `common.rs`.
5. Update `interface-cli` to depend on `assistant-interfaces`, remove feature flags.
6. Remove old crate directories and workspace member entries.
7. Update root `Cargo.toml` workspace members and dependency aliases.
8. Run `make lint && make format && make test` to verify.
