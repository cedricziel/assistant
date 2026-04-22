//! Nextcloud Talk messenger interface adapter.
//!
//! Webhook adapter, HMAC signing, runner, and platform tools.

mod adapter;
pub(crate) mod config;
pub mod runner;
mod signing;
mod tools;
mod types;

pub use config::NextcloudConfigExt;
pub use runner::NextcloudInterface;
