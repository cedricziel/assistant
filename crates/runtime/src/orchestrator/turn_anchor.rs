//! Where a turn attaches in the conversation tree, and the cleanup that makes
//! a redelivered turn idempotent (ADR-0018, design D3 and D3a).

use anyhow::{Context, Result};
use tracing::warn;
use uuid::Uuid;

use assistant_core::bus_messages::TurnAnchor;
use assistant_core::types::conversation::{Message, MessageRole, TurnIdentity};
use assistant_storage::AttachmentStore;
use assistant_storage::conversations::ConversationStore;

use super::Orchestrator;
use super::turn_store::TurnStore;

/// The starting state of an anchored turn, after its user row was saved.
pub(super) struct AnchoredTurn {
    /// The rows before the turn's user message on its path. For a regenerate
    /// this excludes the user message it answers.
    pub prior: Vec<Message>,
    /// The turn number of the user row (design D4).
    pub base_turn: i64,
    /// The user row this turn saved. `None` for a regenerate.
    pub user: Option<Message>,
    /// The user message a regenerate answers.
    pub regenerated: Option<Message>,
    /// Attachments of the edited message that now also belong to the new one.
    pub carried: Vec<Uuid>,
}

impl Orchestrator {
    /// Discard the partial rows of an earlier attempt of this run, resolve the
    /// anchor, save the user row under its fixed id and arm a regenerate's
    /// branch parent.
    pub(super) async fn begin_anchored_turn(
        &self,
        store: &TurnStore,
        conversation_id: Uuid,
        user_message: &str,
        identity: &TurnIdentity,
    ) -> Result<AnchoredTurn> {
        let user_id = identity.user_message_id;
        let (keep, anchor_point) = match &identity.anchor {
            TurnAnchor::Append | TurnAnchor::Edit { .. } => (user_id, user_id),
            TurnAnchor::Regenerate { user_message_id } => (None, Some(*user_message_id)),
        };
        discard_partial_rows(store, conversation_id, identity.run_id, keep, anchor_point).await;

        match &identity.anchor {
            TurnAnchor::Append => {
                let mut prior = store.load_history(conversation_id).await?;
                // A redelivery resets the leaf to the user row saved earlier.
                if prior.last().map(|m| m.id) == user_id && user_id.is_some() {
                    prior.pop();
                }
                let base_turn = prior.len() as i64;
                let user = user_row(conversation_id, user_message, user_id, base_turn);
                store.save_message(&user).await?;
                Ok(AnchoredTurn {
                    prior,
                    base_turn,
                    user: Some(user),
                    regenerated: None,
                    carried: Vec::new(),
                })
            }
            TurnAnchor::Edit {
                parent_message_id,
                edited_message_id,
            } => {
                let prior = match parent_message_id {
                    Some(parent) => store
                        .ancestors(conversation_id, *parent)
                        .await?
                        .with_context(|| {
                            format!("edit parent {parent} is not in the conversation")
                        })?,
                    None => Vec::new(),
                };
                let base_turn = prior.len() as i64;
                let user = user_row(conversation_id, user_message, user_id, base_turn);
                store.branch_message(&user, *parent_message_id).await?;

                let attachments = self.storage.attachment_store();
                attachments
                    .carry_forward(*edited_message_id, user.id)
                    .await?;
                let carried = attachments
                    .list_for_message(user.id)
                    .await?
                    .into_iter()
                    .map(|meta| meta.id)
                    .collect();
                Ok(AnchoredTurn {
                    prior,
                    base_turn,
                    user: Some(user),
                    regenerated: None,
                    carried,
                })
            }
            TurnAnchor::Regenerate { user_message_id } => {
                let mut chain = store
                    .ancestors(conversation_id, *user_message_id)
                    .await?
                    .with_context(|| {
                        format!("regenerated message {user_message_id} is not in the conversation")
                    })?;
                let regenerated = chain
                    .pop()
                    .filter(|m| m.role == MessageRole::User)
                    .with_context(|| {
                        format!("regenerated message {user_message_id} is not a user message")
                    })?;
                store.arm_branch(regenerated.id);
                Ok(AnchoredTurn {
                    base_turn: chain.len() as i64,
                    prior: chain,
                    user: None,
                    regenerated: Some(regenerated),
                    carried: Vec::new(),
                })
            }
        }
    }
}

/// A user row under `id`, or under a fresh id when none was pre-assigned.
fn user_row(conversation_id: Uuid, text: &str, id: Option<Uuid>, turn: i64) -> Message {
    let mut row = Message::user(conversation_id, text);
    if let Some(id) = id {
        row.id = id;
    }
    row.turn = turn;
    row
}

/// Remove the rows an earlier attempt of this run persisted, except `keep`,
/// and move the active leaf back to `anchor_point`. When the delete fails,
/// for example because another interface appended under a partial row, only
/// the leaf moves.
async fn discard_partial_rows(
    store: &TurnStore,
    conversation_id: Uuid,
    run_id: Option<Uuid>,
    keep: Option<Uuid>,
    anchor_point: Option<Uuid>,
) {
    let Some(run_id) = run_id else { return };
    match store.delete_run_rows(conversation_id, run_id, keep).await {
        Ok(0) => return,
        Ok(_) => {}
        Err(e) => warn!(
            %conversation_id,
            %run_id,
            error = %e,
            "Failed to discard partial rows of a redelivered turn; resetting the leaf only"
        ),
    }
    let Some(anchor_point) = anchor_point else {
        return;
    };
    if let Err(e) = store.set_leaf(conversation_id, anchor_point).await {
        warn!(%conversation_id, %run_id, error = %e, "Failed to reset the active leaf");
    }
}
