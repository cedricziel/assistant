//! Mattermost interface for the AI assistant.

pub mod adapter;
pub mod client;
pub mod config;
pub mod runner;
pub mod tools;

pub use assistant_core::MattermostConfig;
pub use runner::MattermostInterface;
