//! Conversation and message persistence backed by the `conversations` and `messages` tables.

use anyhow::Result;
use assistant_core::{Message, MessageRole};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// A stored conversation record (metadata only — messages are loaded separately).
#[derive(Debug, Clone)]
pub struct ConversationRecord {
    pub id: Uuid,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// SQLite-backed store for conversations and messages.
pub struct ConversationStore {
    pool: SqlitePool,
}

impl ConversationStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // -----------------------------------------------------------------------
    // Conversations
    // -----------------------------------------------------------------------

    /// Create or retrieve a conversation by a specific UUID.
    /// If a row with that ID already exists, return it unchanged.
    pub async fn create_conversation_with_id(
        &self,
        id: Uuid,
        title: Option<&str>,
    ) -> Result<ConversationRecord> {
        let now = Utc::now();
        let id_str = id.to_string();

        sqlx::query(
            "INSERT INTO conversations (id, title, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?3) \
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(&id_str)
        .bind(title)
        .bind(now)
        .execute(&self.pool)
        .await?;

        // Fetch whatever row is there (new or existing).
        self.get_conversation(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Conversation {} not found after upsert", id))
    }

    /// Create a new conversation row and return its metadata.
    pub async fn create_conversation(&self, title: Option<&str>) -> Result<ConversationRecord> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let id_str = id.to_string();

        sqlx::query(
            "INSERT INTO conversations (id, title, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?3)",
        )
        .bind(&id_str)
        .bind(title)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(ConversationRecord {
            id,
            title: title.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
        })
    }

    /// Fetch a conversation by ID. Returns `None` if not found.
    pub async fn get_conversation(&self, id: Uuid) -> Result<Option<ConversationRecord>> {
        let id_str = id.to_string();

        let row = sqlx::query(
            "SELECT id, title, created_at, updated_at \
             FROM conversations \
             WHERE id = ?1",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| {
            let raw_id: String = r.get("id");
            Ok(ConversationRecord {
                id: Uuid::parse_str(&raw_id)?,
                title: r.get("title"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
        })
        .transpose()
    }

    /// List all conversations, most-recently updated first.
    pub async fn list_conversations(&self) -> Result<Vec<ConversationRecord>> {
        let rows = sqlx::query(
            "SELECT id, title, created_at, updated_at \
             FROM conversations \
             ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                let raw_id: String = r.get("id");
                Ok(ConversationRecord {
                    id: Uuid::parse_str(&raw_id)?,
                    title: r.get("title"),
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                })
            })
            .collect()
    }

    /// Delete a conversation and all its messages (cascade).
    pub async fn delete_conversation(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        sqlx::query("DELETE FROM conversations WHERE id = ?1")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Messages
    // -----------------------------------------------------------------------

    /// Persist a message to the database.
    pub async fn save_message(&self, msg: &Message) -> Result<()> {
        let id = msg.id.to_string();
        let conversation_id = msg.conversation_id.to_string();
        let role = msg.role.to_string();

        sqlx::query(
            "INSERT INTO messages \
                (id, conversation_id, role, content, skill_name, turn, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(&id)
        .bind(&conversation_id)
        .bind(&role)
        .bind(&msg.content)
        .bind(&msg.skill_name)
        .bind(msg.turn)
        .bind(msg.created_at)
        .execute(&self.pool)
        .await?;

        // Update the conversation's updated_at timestamp
        let now = Utc::now();
        sqlx::query("UPDATE conversations SET updated_at = ?1 WHERE id = ?2")
            .bind(now)
            .bind(&conversation_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Load all messages for a conversation, ordered by turn then created_at.
    pub async fn load_history(&self, conversation_id: Uuid) -> Result<Vec<Message>> {
        let conv_id_str = conversation_id.to_string();

        let rows = sqlx::query(
            "SELECT id, conversation_id, role, content, skill_name, turn, created_at \
             FROM messages \
             WHERE conversation_id = ?1 \
             ORDER BY turn ASC, created_at ASC",
        )
        .bind(&conv_id_str)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                let id_str: String = r.get("id");
                let conv_str: String = r.get("conversation_id");
                let role_str: String = r.get("role");
                Ok(Message {
                    id: Uuid::parse_str(&id_str)?,
                    conversation_id: Uuid::parse_str(&conv_str)?,
                    role: parse_role(&role_str)?,
                    content: r.get("content"),
                    skill_name: r.get("skill_name"),
                    turn: r.get("turn"),
                    created_at: r.get("created_at"),
                })
            })
            .collect()
    }

    /// Return the last `limit` messages for a conversation, in chronological order.
    pub async fn last_messages(&self, conversation_id: Uuid, limit: i64) -> Result<Vec<Message>> {
        let conv_id_str = conversation_id.to_string();

        // Fetch the newest rows first, then reverse to restore chronological order.
        let rows = sqlx::query(
            "SELECT id, conversation_id, role, content, skill_name, turn, created_at \
             FROM messages \
             WHERE conversation_id = ?1 \
             ORDER BY turn DESC, created_at DESC \
             LIMIT ?2",
        )
        .bind(&conv_id_str)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut messages: Vec<Message> = rows
            .into_iter()
            .map(|r| {
                let id_str: String = r.get("id");
                let conv_str: String = r.get("conversation_id");
                let role_str: String = r.get("role");
                Ok(Message {
                    id: Uuid::parse_str(&id_str)?,
                    conversation_id: Uuid::parse_str(&conv_str)?,
                    role: parse_role(&role_str)?,
                    content: r.get("content"),
                    skill_name: r.get("skill_name"),
                    turn: r.get("turn"),
                    created_at: r.get("created_at"),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // Restore chronological order
        messages.reverse();
        Ok(messages)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_role(s: &str) -> Result<MessageRole> {
    match s {
        "user" => Ok(MessageRole::User),
        "assistant" => Ok(MessageRole::Assistant),
        "system" => Ok(MessageRole::System),
        "tool" => Ok(MessageRole::Tool),
        other => anyhow::bail!("Unknown message role: {}", other),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::StorageLayer;
    use assistant_core::Message;

    #[tokio::test]
    async fn test_create_and_load_conversation() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();

        let conv = store.create_conversation(Some("Hello test")).await.unwrap();
        assert_eq!(conv.title.as_deref(), Some("Hello test"));

        let loaded = store.get_conversation(conv.id).await.unwrap().unwrap();
        assert_eq!(loaded.id, conv.id);
    }

    #[tokio::test]
    async fn test_save_and_load_messages() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();

        let conv = store.create_conversation(None).await.unwrap();

        let mut msg = Message::user(conv.id, "Hello!");
        msg.turn = 1;
        store.save_message(&msg).await.unwrap();

        let mut reply = Message::assistant(conv.id, "Hi there!");
        reply.turn = 2;
        store.save_message(&reply).await.unwrap();

        let history = store.load_history(conv.id).await.unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "Hello!");
        assert_eq!(history[1].content, "Hi there!");
    }

    #[tokio::test]
    async fn test_last_messages() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();

        let conv = store.create_conversation(None).await.unwrap();

        for i in 0..5_i64 {
            let mut msg = Message::user(conv.id, format!("msg {}", i));
            msg.turn = i + 1;
            store.save_message(&msg).await.unwrap();
        }

        let last = store.last_messages(conv.id, 3).await.unwrap();
        assert_eq!(last.len(), 3);
        // Should be in chronological order: msg 2, msg 3, msg 4
        assert_eq!(last[0].content, "msg 2");
        assert_eq!(last[2].content, "msg 4");
    }

    #[tokio::test]
    async fn test_delete_conversation() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();

        let conv = store.create_conversation(None).await.unwrap();
        store.delete_conversation(conv.id).await.unwrap();

        let found = store.get_conversation(conv.id).await.unwrap();
        assert!(found.is_none());
    }
}
