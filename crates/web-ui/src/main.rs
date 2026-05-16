//! Thin binary entry point. All real logic lives in the library crate
//! (`crates/web-ui/src/lib.rs`).
//!
//! Why this shape: the library surface (`run_from_env`, `run_from_iter`,
//! `install::ensure_migrated`, `api::*`, `auth::*`, `sw_version::*`) is
//! consumed by `assistant-cli`, integration tests, and downstream tooling.
//! Keeping the binary entrypoint as a small shim avoids the
//! `include!("main.rs")` workaround that previously bridged the two.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    assistant_web_ui::run_from_env().await
}
