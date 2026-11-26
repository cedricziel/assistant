//! The reply of a turn the user stopped.
//!
//! The runtime saved the turn's user row and its tool rows before the cancel,
//! but the streamed text only exists in the caller that forwarded it. The
//! caller hands it back here, so history, reload and the next LLM turn all
//! contain what the user saw (or a short placeholder when nothing streamed).

use anyhow::Result;
use tracing::debug;
use uuid::Uuid;

use assistant_core::bus_messages;
use assistant_core::types::conversation::{Interface, Message, MessageFinishReason, MessageRole};
use assistant_storage::MessageTimingStore;
use assistant_storage::conversations::ConversationStore;

use crate::history::{FinalAnswerMeta, persist_final_answer};

use super::Orchestrator;

/// The assistant row saved when a turn is stopped before any text streamed.
pub const CANCELLED_REPLY_PLACEHOLDER: &str = "_Stopped._";

/// What [`save_cancelled_reply`] did.
#[derive(Debug)]
pub(crate) enum CancelledReply {
    /// A new row was saved.
    Saved {
        id: Uuid,
        content: String,
        turn: i64,
        /// The turn's user message has text, so the conversation can be titled.
        user_has_text: bool,
    },
    /// The turn's own final answer was already saved; nothing was written.
    AlreadyReplied(Uuid),
}

/// Save `text` as the stopped turn's assistant row, unless the turn already
/// has a final answer.
///
/// Only rows after the last user row belong to the turn. Tool-call rows (empty
/// content, `tool_calls_json` set) and tool results do not count as an answer.
pub(crate) async fn save_cancelled_reply(
    conv_store: &dyn ConversationStore,
    timing_store: &dyn MessageTimingStore,
    conversation_id: Uuid,
    text: &str,
) -> Result<CancelledReply> {
    let history = conv_store.load_history(conversation_id).await?;
    let last_user = history.iter().rposition(|m| m.role == MessageRole::User);
    let turn_rows = &history[last_user.map_or(0, |i| i + 1)..];
    if let Some(answer) = turn_rows.iter().find(|m| {
        m.role == MessageRole::Assistant
            && m.tool_calls_json.is_none()
            && !m.content.trim().is_empty()
    }) {
        return Ok(CancelledReply::AlreadyReplied(answer.id));
    }

    let content = if text.trim().is_empty() {
        CANCELLED_REPLY_PLACEHOLDER
    } else {
        text
    };
    let mut message = Message::assistant(conversation_id, content);
    message.turn = history.len() as i64;
    message.finish_reason = Some(MessageFinishReason::Cancelled);
    let (id, turn) = (message.id, message.turn);
    persist_final_answer(
        conv_store,
        timing_store,
        message,
        FinalAnswerMeta::default(),
    )
    .await?;

    let user_has_text = last_user.is_some_and(|i| !history[i].content.trim().is_empty());
    Ok(CancelledReply::Saved {
        id,
        content: content.to_string(),
        turn,
        user_has_text,
    })
}

impl Orchestrator {
    /// Persist the partial reply of a turn the caller saw cancelled.
    ///
    /// Returns the id of the turn's assistant row: the new row, or the final
    /// answer the worker saved just before the cancel won. A failure to save
    /// is an error.
    ///
    /// A saved row for a user message with text also announces `turn.completed`
    /// so an untitled conversation still gets its title. Nothing is published
    /// on `turn.result` or to webhooks.
    pub async fn persist_cancelled_reply(
        &self,
        conversation_id: Uuid,
        text: &str,
        interface: &Interface,
    ) -> Result<Uuid> {
        let store = self.storage.conversation_store_for_agent(&self.agent_id);
        let timing_store = self.storage.message_timing_store();
        match save_cancelled_reply(&store, &timing_store, conversation_id, text).await? {
            CancelledReply::AlreadyReplied(id) => {
                debug!(%conversation_id, message_id = %id, "cancelled turn already has its reply");
                Ok(id)
            }
            CancelledReply::Saved {
                id,
                content,
                turn,
                user_has_text,
            } => {
                if user_has_text {
                    let announcement = bus_messages::TurnResult {
                        conversation_id,
                        content,
                        turn,
                        attachment_ids: Vec::new(),
                        message_id: Some(id),
                        had_errors: false,
                        timing: None,
                    };
                    self.announce_completed_turn(
                        serde_json::to_value(&announcement).unwrap_or_default(),
                        conversation_id,
                        &format!("{interface:?}"),
                    )
                    .await;
                }
                Ok(id)
            }
        }
    }
}
