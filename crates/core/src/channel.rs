//! Unified channel adapter trait and message types.
//!
//! Every messenger interface implements [`ChannelAdapter`], which provides a
//! `Stream`-based receive path and a `send()` method.  Platform-specific event
//! structs are private implementation details inside each adapter crate.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::tool::{Attachment, ToolHandler};
use crate::types::{ChannelType, Interface};

// -- Types --

/// A user on a messaging platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelUser {
    /// Platform-native user identifier (e.g. Slack user ID, Matrix MXID).
    pub platform_id: String,
    /// Human-readable display name, if available.
    pub display_name: Option<String>,
}

/// The content payload of a channel message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelContent {
    Text(String),
    Image {
        url: String,
        caption: Option<String>,
    },
    File {
        url: String,
        filename: String,
    },
    FileData {
        data: Vec<u8>,
        filename: String,
        mime_type: String,
    },
    Voice {
        url: String,
        duration_seconds: Option<u32>,
    },
}

/// A unified inbound message from any messaging channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// Which platform produced this message.
    pub channel_type: ChannelType,
    /// Platform-native message ID for dedup and threading.
    pub platform_message_id: Option<String>,
    /// Who sent the message.
    pub sender: ChannelUser,
    /// What was sent.
    pub content: ChannelContent,
    /// Thread or conversation anchor on the platform (e.g. Slack `thread_ts`, Matrix room ID).
    pub thread_id: Option<String>,
    /// Timestamp from the platform.
    pub timestamp: DateTime<Utc>,
    /// Platform-specific extras (channel name, workspace, etc.).
    pub metadata: HashMap<String, serde_json::Value>,
}

// -- Trait --

/// A thin adapter that connects a single messaging platform to the assistant.
///
/// Implement this trait for each platform.  The `start()` method returns an
/// async stream of inbound [`ChannelMessage`]s; the adapter is also responsible
/// for sending replies via `send()`.
///
/// Optional methods (`send_typing`, `send_reaction`, `send_in_thread`) default
/// to no-ops so adapters only override what their platform supports.
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    /// Human-readable adapter name (e.g. `"slack"`, `"mattermost"`).
    fn name(&self) -> &str;

    /// Which platform this adapter serves.
    fn channel_type(&self) -> ChannelType;

    /// Start receiving messages.  Returns an owned async stream of inbound
    /// [`ChannelMessage`]s.  The stream runs until `stop()` is called or the
    /// underlying connection terminates permanently.
    async fn start(
        &self,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send + 'static>>>;

    /// Send a message to the user.
    async fn send(&self, user: &ChannelUser, content: ChannelContent) -> anyhow::Result<()>;

    /// Gracefully shut down the adapter.
    async fn stop(&self) -> anyhow::Result<()>;

    /// Send a typing indicator (best-effort; default: no-op).
    async fn send_typing(&self, _user: &ChannelUser) -> anyhow::Result<()> {
        Ok(())
    }

    /// Add an emoji reaction to a message (best-effort; default: no-op).
    async fn send_reaction(
        &self,
        _user: &ChannelUser,
        _message_id: &str,
        _reaction: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Send a reply in a specific thread.  Default: delegates to `send()`.
    async fn send_in_thread(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
        _thread_id: &str,
    ) -> anyhow::Result<()> {
        self.send(user, content).await
    }

    // -- ChannelRunner hooks (all default no-ops) --

    /// Derive a stable conversation key from this message.
    ///
    /// The key is used to group messages into the same conversation UUID.
    /// The default encodes `sender.platform_id`, which adapters encode to
    /// include the channel/room/thread context (e.g. `"{channel_id}/{thread_ts}"`).
    fn conversation_key(&self, msg: &ChannelMessage) -> String {
        msg.sender.platform_id.clone()
    }

    /// Return the runtime [`Interface`] variant for this adapter.
    ///
    /// Used by [`ChannelRunner`] when calling `run_turn_with_tools`.  Derived
    /// automatically from `channel_type()` — override only if needed.
    fn interface(&self) -> Interface {
        match self.channel_type() {
            ChannelType::Slack => Interface::Slack,
            ChannelType::Mattermost => Interface::Mattermost,
            ChannelType::Matrix => Interface::Matrix,
            ChannelType::Nextcloud => Interface::Nextcloud,
            ChannelType::Signal => Interface::Signal,
            ChannelType::Custom(_) => Interface::Web,
        }
    }

    /// Return platform-specific tool handlers for this message's context.
    ///
    /// Called once per turn, before `run_turn_with_tools`.  Adapters override
    /// this to inject reply tools, reaction tools, etc. scoped to the message.
    fn platform_tools(&self, _msg: &ChannelMessage, _conv_id: Uuid) -> Vec<Arc<dyn ToolHandler>> {
        vec![]
    }

    /// Called before dispatching a turn (e.g. add a "thinking" reaction).
    ///
    /// `conv_id` is the stable conversation UUID assigned by [`ChannelRunner`].
    /// Adapters that seed thread history (e.g. Slack) use it to associate
    /// historical messages with the right conversation.
    async fn on_turn_start(&self, _user: &ChannelUser, _conv_id: Uuid) -> anyhow::Result<()> {
        Ok(())
    }

    /// Called after a successful turn.
    ///
    /// `answer` is the assistant's final text.  `attachments` are any file
    /// outputs produced by tools during the turn.
    async fn on_turn_success(
        &self,
        _user: &ChannelUser,
        _answer: &str,
        _attachments: &[Attachment],
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Called after a failed turn (e.g. post an error message to the channel).
    async fn on_turn_error(&self, _user: &ChannelUser, _err: &anyhow::Error) -> anyhow::Result<()> {
        Ok(())
    }
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_message_construction() {
        let msg = ChannelMessage {
            channel_type: ChannelType::Slack,
            platform_message_id: Some("abc123".into()),
            sender: ChannelUser {
                platform_id: "U123".into(),
                display_name: Some("Alice".into()),
            },
            content: ChannelContent::Text("hello".into()),
            thread_id: Some("ts123".into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        assert_eq!(msg.channel_type, ChannelType::Slack);
        assert!(matches!(msg.content, ChannelContent::Text(_)));
    }

    #[test]
    fn channel_content_variants() {
        let text = ChannelContent::Text("hi".into());
        let image = ChannelContent::Image {
            url: "https://example.com/img.png".into(),
            caption: None,
        };
        let file_data = ChannelContent::FileData {
            data: vec![0u8; 4],
            filename: "file.txt".into(),
            mime_type: "text/plain".into(),
        };
        assert!(matches!(text, ChannelContent::Text(_)));
        assert!(matches!(image, ChannelContent::Image { .. }));
        assert!(matches!(file_data, ChannelContent::FileData { .. }));
    }

    #[test]
    fn channel_type_display() {
        assert_eq!(ChannelType::Slack.to_string(), "slack");
        assert_eq!(ChannelType::Custom("discord".into()).to_string(), "discord");
    }
}
