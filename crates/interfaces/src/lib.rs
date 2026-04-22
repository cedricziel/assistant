//! Consolidated messenger interface adapters for the AI assistant.
//!
//! Each sub-module implements the [`ChannelAdapter`](assistant_core::ChannelAdapter) /
//! [`InterfaceRunner`](assistant_runtime::InterfaceRunner) pattern for one
//! messaging platform.

pub(crate) mod common;

mod matrix;
mod mattermost;
mod nextcloud;
mod signal;
mod slack;

pub use matrix::MatrixInterface;
pub use mattermost::MattermostInterface;
pub use nextcloud::{NextcloudConfigExt, NextcloudInterface};
pub use signal::{SignalAdapter, SignalInterface};
pub use slack::SlackInterface;
