//! Generative UI surfaces in conversation history.
//!
//! `GET /api/conversations/{id}` preloads the conversation's surfaces next to
//! its messages. This module builds those [`SurfaceSummary`] objects and owns
//! [`surface_answerable`], the single definition of the answerable window
//! (design D12) shared with the post-turn action route.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use tracing::warn;
use uuid::Uuid;

use assistant_a2ui::{Component, SurfaceDraft, SurfaceMessages};
use assistant_core::types::conversation::{Message, MessageRole};
use assistant_storage::{Surface, SurfaceStatus};

/// One surface of a conversation, as the client needs to render it again.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SurfaceSummary {
    pub surface_id: Uuid,
    pub turn: i64,
    /// The tool-call message the surface hangs off.
    pub anchor_message_id: Uuid,
    /// Index of the creating call inside that message's tool calls.
    pub anchor_call_index: u32,
    /// Provider id of the creating tool call, when the provider supplied one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    pub title: String,
    pub fallback_text: String,
    /// `"active"` or `"submitted"`.
    pub status: String,
    /// Whether the user can still answer the surface: it is `active` and no
    /// user message follows its anchor on the returned path.
    pub answerable: bool,
    pub created_at: DateTime<Utc>,
    /// The action that submitted the surface.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub submitted_action: Option<Value>,
    /// The user message that carried the submitting action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_message_id: Option<Uuid>,
    /// A single synthesized A2UI `createSurface` holding the current
    /// components and data model.
    #[schema(value_type = Vec<Object>)]
    pub messages: Vec<Value>,
}

/// Whether `surface` can still be answered on the conversation path
/// `path_messages` (oldest first): it is `active`, its anchor message is on the
/// path, and no user message follows the anchor.
pub fn surface_answerable(path_messages: &[Message], surface: &Surface) -> bool {
    if surface.status != SurfaceStatus::Active {
        return false;
    }
    let Some(anchor) = anchor_position(path_messages, surface) else {
        return false;
    };
    !path_messages[anchor + 1..]
        .iter()
        .any(|m| matches!(m.role, MessageRole::User))
}

fn anchor_position(path_messages: &[Message], surface: &Surface) -> Option<usize> {
    path_messages
        .iter()
        .position(|m| m.id == surface.anchor_message_id)
}

/// Summaries for the surfaces anchored on `path_messages`, in the order given.
/// Surfaces whose anchor message is not on the path (an abandoned branch) are
/// left out.
pub fn surface_summaries(path_messages: &[Message], surfaces: Vec<Surface>) -> Vec<SurfaceSummary> {
    surfaces
        .into_iter()
        .filter(|s| anchor_position(path_messages, s).is_some())
        .filter_map(|s| {
            let answerable = surface_answerable(path_messages, &s);
            summarize(s, answerable)
        })
        .collect()
}

/// `None` when the stored components cannot be read back as A2UI components;
/// the surface is then skipped rather than failing the whole conversation.
fn summarize(s: Surface, answerable: bool) -> Option<SurfaceSummary> {
    let components: Vec<Component> = match serde_json::from_value(s.components.clone()) {
        Ok(components) => components,
        Err(e) => {
            warn!(surface_id = %s.id, error = %e, "Skipping surface with unreadable components");
            return None;
        }
    };
    let draft = SurfaceDraft {
        title: s.title.clone(),
        fallback_text: s.fallback_text.clone(),
        components,
        data_model: Some(s.data_model.clone()),
        send_data_model: s.send_data_model,
    };
    let message = match serde_json::to_value(SurfaceMessages::create(&draft, &s.id.to_string())) {
        Ok(message) => message,
        Err(e) => {
            warn!(surface_id = %s.id, error = %e, "Skipping surface that cannot be encoded");
            return None;
        }
    };
    Some(SurfaceSummary {
        surface_id: s.id,
        turn: s.turn,
        anchor_message_id: s.anchor_message_id,
        anchor_call_index: u32::try_from(s.anchor_call_index).unwrap_or_default(),
        tool_call_id: s.tool_call_id,
        title: s.title,
        fallback_text: s.fallback_text,
        status: s.status.as_str().to_string(),
        answerable,
        created_at: s.created_at,
        submitted_action: s.action,
        action_message_id: s.action_message_id,
        messages: vec![message],
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use assistant_storage::{InMemorySurfaceStore, NewSurface, SurfaceStore};

    use super::*;

    async fn surface_on(anchor: Uuid, store: &InMemorySurfaceStore) -> Surface {
        store
            .create(&NewSurface {
                id: Uuid::new_v4(),
                conversation_id: Uuid::nil(),
                turn: 1,
                anchor_message_id: anchor,
                anchor_call_index: 2,
                tool_call_id: Some("call_1".into()),
                title: "Trip".into(),
                fallback_text: "Pick a date".into(),
                catalog_id: "cat".into(),
                protocol_version: "1.0".into(),
                components: json!([{"id": "root", "component": "Text", "text": "hi"}]),
                data_model: json!({"name": "Ada"}),
                send_data_model: true,
            })
            .await
            .unwrap()
    }

    fn path() -> (Vec<Message>, Uuid) {
        let conv = Uuid::nil();
        let user = Message::user(conv, "plan a trip");
        let anchor = Message::assistant(conv, "");
        let id = anchor.id;
        (vec![user, anchor, Message::assistant(conv, "done")], id)
    }

    #[tokio::test]
    async fn answerable_when_active_and_no_user_message_follows() {
        let (path, anchor) = path();
        let surface = surface_on(anchor, &InMemorySurfaceStore::new()).await;
        assert!(surface_answerable(&path, &surface));
    }

    #[tokio::test]
    async fn not_answerable_after_a_later_user_message() {
        let (mut path, anchor) = path();
        path.push(Message::user(Uuid::nil(), "thanks"));
        let surface = surface_on(anchor, &InMemorySurfaceStore::new()).await;
        assert!(!surface_answerable(&path, &surface));
    }

    #[tokio::test]
    async fn not_answerable_when_submitted() {
        let (path, anchor) = path();
        let store = InMemorySurfaceStore::new();
        let surface = surface_on(anchor, &store).await;
        store
            .mark_submitted(surface.id, &json!({"name": "go"}), Uuid::new_v4())
            .await
            .unwrap();
        let surface = store.get(surface.id).await.unwrap().unwrap();
        assert!(!surface_answerable(&path, &surface));
    }

    #[tokio::test]
    async fn not_answerable_when_the_anchor_is_off_the_path() {
        let (path, _) = path();
        let surface = surface_on(Uuid::new_v4(), &InMemorySurfaceStore::new()).await;
        assert!(!surface_answerable(&path, &surface));
    }

    #[tokio::test]
    async fn summaries_synthesize_one_create_surface_and_drop_off_path_anchors() {
        let (path, anchor) = path();
        let store = InMemorySurfaceStore::new();
        let kept = surface_on(anchor, &store).await;
        let dropped = surface_on(Uuid::new_v4(), &store).await;

        let out = surface_summaries(&path, vec![kept.clone(), dropped]);

        assert_eq!(out.len(), 1, "off-path surface must be omitted");
        let s = &out[0];
        assert_eq!(s.surface_id, kept.id);
        assert_eq!(s.anchor_call_index, 2);
        assert_eq!(s.status, "active");
        assert!(s.answerable);
        assert_eq!(s.messages.len(), 1);
        let create = &s.messages[0]["createSurface"];
        assert_eq!(create["surfaceId"], kept.id.to_string());
        assert_eq!(create["dataModel"], json!({"name": "Ada"}));
        assert_eq!(create["components"][0]["id"], "root");
    }
}
