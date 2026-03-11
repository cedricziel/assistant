//! Activity Streams 2.0 types for Nextcloud Talk webhook payloads.
//!
//! These types model the JSON structures sent by the Nextcloud Talk server
//! to registered bot webhook endpoints.  See
//! <https://nextcloud-talk.readthedocs.io/en/latest/bots/> for the full spec.

use serde::Deserialize;

/// Top-level webhook event from Nextcloud Talk.
///
/// The `type` field determines the kind of event:
/// - `"Create"` — a new chat message
/// - `"Like"` — a reaction was added
/// - `"Undo"` — a reaction was removed
/// - `"Join"` — the bot was added to a conversation
/// - `"Leave"` — the bot was removed from a conversation
#[derive(Debug, Deserialize)]
pub struct WebhookEvent {
    /// Activity Streams event type.
    #[serde(rename = "type")]
    pub event_type: String,
    /// The actor (user/bot) who triggered the event.
    pub actor: Actor,
    /// The message or conversation object.
    pub object: EventObject,
    /// The conversation the event occurred in.
    pub target: Option<Target>,
    /// For reaction events (`Like`), the emoji that was added.
    pub content: Option<String>,
}

/// The actor who triggered an event.
#[derive(Debug, Deserialize)]
pub struct Actor {
    /// `"Person"` for users/guests, `"Application"` for bots.
    #[serde(rename = "type")]
    pub actor_type: String,
    /// Format: `"users/<user-id>"` or `"bots/bot-<hash>"`.
    pub id: String,
    /// Display name.
    #[allow(dead_code)]
    pub name: String,
}

/// The object in a webhook event (message or conversation).
#[derive(Debug, Deserialize)]
pub struct EventObject {
    /// `"Note"` for messages, `"Collection"` for conversations.
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub object_type: String,
    /// Message ID (for `Note`) or conversation token (for `Collection`).
    pub id: String,
    /// `"message"` for normal messages, system message identifiers otherwise.
    #[serde(default)]
    pub name: String,
    /// JSON-encoded `{"message": "...", "parameters": {...}}` for messages.
    #[serde(default)]
    pub content: Option<String>,
    /// `"text/markdown"` or `"text/plain"`.
    #[serde(rename = "mediaType")]
    #[allow(dead_code)]
    pub media_type: Option<String>,
}

/// The target conversation.
#[derive(Debug, Deserialize)]
pub struct Target {
    /// Always `"Collection"`.
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub target_type: String,
    /// The conversation token (used in API endpoints).
    pub id: String,
    /// The conversation name.
    #[allow(dead_code)]
    pub name: String,
}

/// The inner content of a chat message (`object.content` JSON-decoded).
#[derive(Debug, Deserialize)]
pub struct MessageContent {
    /// The raw message text (may contain `{placeholder}` references).
    pub message: String,
    /// Rich object parameters for placeholder substitution.
    #[serde(default)]
    pub parameters: serde_json::Value,
}

impl Actor {
    /// Returns the user ID extracted from `self.id` (e.g. `"ada-lovelace"`
    /// from `"users/ada-lovelace"`).
    pub fn user_id(&self) -> &str {
        self.id
            .split_once('/')
            .map(|(_, id)| id)
            .unwrap_or(&self.id)
    }

    /// Returns `true` if this actor is a bot.
    pub fn is_bot(&self) -> bool {
        self.actor_type == "Application" || self.id.starts_with("bots/")
    }
}

impl WebhookEvent {
    /// Returns the conversation token, preferring `target.id` and falling
    /// back to `object.id` (for Join/Leave events where object is the
    /// conversation).
    pub fn conversation_token(&self) -> &str {
        self.target
            .as_ref()
            .map(|t| t.id.as_str())
            .unwrap_or(&self.object.id)
    }

    /// Extracts the plain-text message from a `Create` event.
    ///
    /// Parses the JSON-encoded `object.content` and returns the `message`
    /// field with parameter placeholders resolved to display names.
    pub fn extract_message_text(&self) -> Option<String> {
        let raw = self.object.content.as_deref()?;
        let parsed: MessageContent = serde_json::from_str(raw).ok()?;
        Some(resolve_placeholders(&parsed.message, &parsed.parameters))
    }

    /// Returns the message ID for reaction/reply purposes.
    pub fn message_id(&self) -> &str {
        &self.object.id
    }
}

/// Resolve `{placeholder}` references in a message using the parameters map.
///
/// For each `{key}` in the message, looks up `parameters[key]["name"]` and
/// substitutes it.  Unknown placeholders are left as-is.
fn resolve_placeholders(message: &str, parameters: &serde_json::Value) -> String {
    let mut result = message.to_string();
    if let serde_json::Value::Object(map) = parameters {
        for (key, value) in map {
            let placeholder = format!("{{{key}}}");
            if let Some(name) = value.get("name").and_then(|n| n.as_str()) {
                result = result.replace(&placeholder, name);
            }
        }
    }
    result
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_chat_message_event() {
        let json = r#"{
            "type": "Create",
            "actor": {
                "type": "Person",
                "id": "users/ada-lovelace",
                "name": "Ada Lovelace"
            },
            "object": {
                "type": "Note",
                "id": "1567",
                "name": "message",
                "content": "{\"message\":\"hi {mention-call1} !\",\"parameters\":{\"mention-call1\":{\"type\":\"call\",\"id\":\"n3xtc10ud\",\"name\":\"world\"}}}",
                "mediaType": "text/markdown"
            },
            "target": {
                "type": "Collection",
                "id": "n3xtc10ud",
                "name": "world"
            }
        }"#;

        let event: WebhookEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "Create");
        assert_eq!(event.actor.user_id(), "ada-lovelace");
        assert!(!event.actor.is_bot());
        assert_eq!(event.conversation_token(), "n3xtc10ud");
        assert_eq!(event.message_id(), "1567");
        assert_eq!(event.extract_message_text().unwrap(), "hi world !");
    }

    #[test]
    fn parse_join_event() {
        let json = r#"{
            "type": "Join",
            "actor": {
                "type": "Application",
                "id": "bots/bot-abc123",
                "name": "TestBot"
            },
            "object": {
                "type": "Collection",
                "id": "n3xtc10ud",
                "name": "world"
            }
        }"#;

        let event: WebhookEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "Join");
        assert!(event.actor.is_bot());
        assert_eq!(event.conversation_token(), "n3xtc10ud");
    }

    #[test]
    fn parse_reaction_event() {
        let json = r#"{
            "type": "Like",
            "actor": {
                "type": "Person",
                "id": "users/bob",
                "name": "Bob"
            },
            "object": {
                "type": "Note",
                "id": "42",
                "name": "message",
                "content": "{\"message\":\"hello\",\"parameters\":{}}",
                "mediaType": "text/plain"
            },
            "target": {
                "type": "Collection",
                "id": "room123",
                "name": "general"
            },
            "content": "\ud83d\ude06"
        }"#;

        let event: WebhookEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "Like");
        assert_eq!(event.content.as_deref(), Some("\u{1F606}"));
    }

    #[test]
    fn resolve_placeholders_with_params() {
        let params: serde_json::Value = serde_json::json!({
            "mention-user1": {"type": "user", "id": "alice", "name": "Alice"},
            "mention-call1": {"type": "call", "id": "room1", "name": "#general"}
        });
        let msg = "Hello {mention-user1}, welcome to {mention-call1}!";
        let result = resolve_placeholders(msg, &params);
        assert_eq!(result, "Hello Alice, welcome to #general!");
    }

    #[test]
    fn resolve_placeholders_no_params() {
        let params = serde_json::json!({});
        let msg = "Hello world";
        let result = resolve_placeholders(msg, &params);
        assert_eq!(result, "Hello world");
    }
}
