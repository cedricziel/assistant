## 1. Scaffold `assistant-interfaces` Crate

- [x] 1.1 Create `crates/interfaces/Cargo.toml` with package name `assistant-interfaces`, combining all deps from the five interface crates (shared: `assistant-core`, `assistant-runtime`, `assistant-transcription`, `tokio`, `serde`, `serde_json`, `anyhow`, `tracing`, `async-trait`, `uuid`, `chrono`, `reqwest`, `futures`, `tokio-stream`, `tokio-tungstenite`; slack-specific: `lru`, `assistant-storage`; matrix-specific: `dirs`, `urlencoding`; nextcloud-specific: `axum`, `hmac`, `sha2`, `hex`, `getrandom`, `tokio-util`; signal-specific: `base64`, `toml`)
- [x] 1.2 Create `crates/interfaces/src/lib.rs` with module declarations (`mod slack; mod mattermost; mod matrix; mod nextcloud; mod signal; mod common;`) and public re-exports of `SlackInterface`, `MattermostInterface`, `MatrixInterface`, `NextcloudInterface`, `NextcloudConfigExt`, `SignalInterface`, `SignalAdapter`
- [x] 1.3 Add `assistant-interfaces` to workspace members in root `Cargo.toml` and add it to `[workspace.dependencies]`

## 2. Extract Shared Helpers

- [x] 2.1 Create `crates/interfaces/src/common.rs` extracting the duplicated exponential backoff helpers (`BACKOFF_MIN`, `BACKOFF_MAX`, `sleep_backoff`, `rand_jitter`) from slack/mattermost/signal adapters into a shared module

## 3. Move Slack Interface

- [x] 3.1 Copy `crates/interface-slack/src/{adapter,client,config,runner,tools}.rs` → `crates/interfaces/src/slack/`
- [x] 3.2 Copy `crates/interface-slack/src/skills/` directory → `crates/interfaces/src/slack/skills/`
- [x] 3.3 Create `crates/interfaces/src/slack/mod.rs` declaring submodules and controlling visibility (`pub use runner::SlackInterface;` with the rest `pub(crate)` or private)
- [x] 3.4 Update `crate::` imports in moved files to use `super::` or `crate::slack::` as appropriate; update backoff usage to `crate::common::`
- [x] 3.5 Verify `cargo check -p assistant-interfaces` passes for the slack module

## 4. Move Mattermost Interface

- [x] 4.1 Copy `crates/interface-mattermost/src/{adapter,client,config,runner,tools}.rs` → `crates/interfaces/src/mattermost/`
- [x] 4.2 Create `crates/interfaces/src/mattermost/mod.rs` declaring submodules (`pub use runner::MattermostInterface;`)
- [x] 4.3 Update `crate::` imports in moved files; update backoff usage to `crate::common::`
- [x] 4.4 Verify `cargo check -p assistant-interfaces` passes

## 5. Move Matrix Interface

- [x] 5.1 Copy `crates/interface-matrix/src/{adapter,client,config,runner,tools}.rs` → `crates/interfaces/src/matrix/`
- [x] 5.2 Create `crates/interfaces/src/matrix/mod.rs` declaring submodules (`pub use runner::MatrixInterface;`)
- [x] 5.3 Update `crate::` imports in moved files
- [x] 5.4 Verify `cargo check -p assistant-interfaces` passes

## 6. Move Nextcloud Interface

- [x] 6.1 Copy `crates/interface-nextcloud/src/{adapter,config,runner,signing,tools,types}.rs` → `crates/interfaces/src/nextcloud/`
- [x] 6.2 Create `crates/interfaces/src/nextcloud/mod.rs` declaring submodules (`pub use runner::NextcloudInterface; pub use config::NextcloudConfigExt;`)
- [x] 6.3 Update `crate::` imports in moved files
- [x] 6.4 Verify `cargo check -p assistant-interfaces` passes

## 7. Move Signal Interface

- [x] 7.1 Copy `crates/interface-signal/src/{adapter,config,runner}.rs` → `crates/interfaces/src/signal/`
- [x] 7.2 Create `crates/interfaces/src/signal/mod.rs` declaring submodules (`pub use runner::SignalInterface; pub use adapter::SignalAdapter;`)
- [x] 7.3 Update `crate::` imports in moved files; update backoff usage to `crate::common::`
- [x] 7.4 Verify `cargo check -p assistant-interfaces` passes

## 8. Update `interface-cli` to Use Consolidated Crate

- [x] 8.1 Replace the five optional `assistant-interface-*` dependencies in `crates/interface-cli/Cargo.toml` with a single non-optional `assistant-interfaces` dependency
- [x] 8.2 Remove the `slack`, `mattermost`, `matrix`, `nextcloud`, `signal` feature flags from `crates/interface-cli/Cargo.toml` (keep `mcp` and `nats` as-is)
- [x] 8.3 Update `default` features list to remove the five interface features
- [x] 8.4 Update imports in `crates/interface-cli/src/main.rs`: replace `use assistant_interface_slack::SlackInterface;` etc. with `use assistant_interfaces::{SlackInterface, MattermostInterface, MatrixInterface, NextcloudInterface, SignalInterface};`
- [x] 8.5 Remove all `#[cfg(feature = "slack")]`, `#[cfg(feature = "mattermost")]`, etc. annotations from `main.rs`
- [x] 8.6 Verify `cargo check -p assistant-cli` passes

## 9. Remove Old Crates

- [x] 9.1 Remove `crates/interface-slack/`, `crates/interface-mattermost/`, `crates/interface-matrix/`, `crates/interface-nextcloud/`, `crates/interface-signal/` directories
- [x] 9.2 Remove the five old crates from `[workspace.members]` in root `Cargo.toml`
- [x] 9.3 Remove the five old `assistant-interface-*` entries from `[workspace.dependencies]` in root `Cargo.toml`

## 10. Update Documentation and CI

- [x] 10.1 Update `AGENTS.md` workspace structure table: remove the five interface rows, add `assistant-interfaces` row
- [x] 10.2 Update `AGENTS.md` dependency order diagram to show `interfaces -> core, runtime, storage, transcription`
- [x] 10.3 Update `AGENTS.md` "Messenger Interface Clients" section to reference the consolidated crate
- [x] 10.4 Update any CI workflows (`.github/workflows/`) that reference individual interface crate paths

## 11. Verification

- [x] 11.1 Run `make lint` — all clippy warnings clean
- [x] 11.2 Run `make format` — formatting clean
- [x] 11.3 Run `make test` — all tests pass
- [x] 11.4 Run `make build` — full workspace build succeeds
- [x] 11.5 Verify old crate names absent from `cargo tree` output
- [ ] 11.6 Verify `make run` (orchestrator) still launches all interfaces
