//! Conversation and message persistence backed by the `conversations` and `messages` tables.

use std::sync::Arc;

use anyhow::{Context, Result};
use assistant_core::{Message, MessageRole};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::conversation_broadcaster::{ConversationBroadcast, ConversationEvent};

/// A stored conversation record (metadata only — messages are loaded separately).
#[derive(Debug, Clone)]
pub struct ConversationRecord {
    pub id: Uuid,
    pub agent_id: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// SQLite-backed store for conversations and messages.
pub struct ConversationStore {
    pool: SqlitePool,
    agent_id: String,
    broadcaster: Option<Arc<dyn ConversationBroadcast>>,
}

impl ConversationStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            agent_id: "default".to_string(),
            broadcaster: None,
        }
    }

    pub fn for_agent(pool: SqlitePool, agent_id: impl Into<String>) -> Self {
        Self {
            pool,
            agent_id: agent_id.into(),
            broadcaster: None,
        }
    }

    /// Attach a broadcaster that will receive events on every conversation mutation.
    pub fn with_broadcaster(mut self, broadcaster: Arc<dyn ConversationBroadcast>) -> Self {
        self.broadcaster = Some(broadcaster);
        self
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
            "INSERT INTO conversations (id, title, agent_id, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?4) \
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(&id_str)
        .bind(title)
        .bind(&self.agent_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.ensure_conversation_agent(id).await?;

        // Fetch whatever row is there (new or existing).
        let conv = self
            .get_conversation(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Conversation {} not found after upsert", id))?;

        if let Some(b) = &self.broadcaster {
            b.emit(ConversationEvent::Upserted(conv.clone()));
        }

        Ok(conv)
    }

    /// Create a new conversation row and return its metadata.
    pub async fn create_conversation(&self, title: Option<&str>) -> Result<ConversationRecord> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let id_str = id.to_string();

        sqlx::query(
            "INSERT INTO conversations (id, title, agent_id, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?4)",
        )
        .bind(&id_str)
        .bind(title)
        .bind(&self.agent_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let conv = ConversationRecord {
            id,
            agent_id: self.agent_id.clone(),
            title: title.map(|s| s.to_string()),
            created_at: now,
            updated_at: now,
        };

        if let Some(b) = &self.broadcaster {
            b.emit(ConversationEvent::Upserted(conv.clone()));
        }

        Ok(conv)
    }

    /// Fetch a conversation by ID. Returns `None` if not found.
    pub async fn get_conversation(&self, id: Uuid) -> Result<Option<ConversationRecord>> {
        let id_str = id.to_string();

        let row = sqlx::query(
            "SELECT id, title, agent_id, created_at, updated_at \
             FROM conversations \
             WHERE id = ?1 AND agent_id = ?2",
        )
        .bind(&id_str)
        .bind(&self.agent_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| {
            let raw_id: String = r.get("id");
            Ok(ConversationRecord {
                id: Uuid::parse_str(&raw_id)?,
                agent_id: r.get("agent_id"),
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
            "SELECT id, title, agent_id, created_at, updated_at \
             FROM conversations \
             WHERE agent_id = ?1 \
             ORDER BY updated_at DESC",
        )
        .bind(&self.agent_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                let raw_id: String = r.get("id");
                Ok(ConversationRecord {
                    id: Uuid::parse_str(&raw_id)?,
                    agent_id: r.get("agent_id"),
                    title: r.get("title"),
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                })
            })
            .collect()
    }

    /// Update the title of an existing conversation.
    ///
    /// Returns an error if the conversation does not exist.
    pub async fn update_title(&self, id: Uuid, title: &str) -> Result<()> {
        let id_str = id.to_string();
        let result = sqlx::query(
            "UPDATE conversations
                 SET title = ?1, updated_at = ?2
                 WHERE id = ?3 AND agent_id = ?4",
        )
        .bind(title)
        .bind(Utc::now())
        .bind(&id_str)
        .bind(&self.agent_id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("failed to update title for conversation {id}"))?;
        if result.rows_affected() == 0 {
            anyhow::bail!("conversation {id} not found");
        }

        if let Some(b) = &self.broadcaster
            && let Ok(Some(conv)) = self.get_conversation(id).await
        {
            b.emit(ConversationEvent::Upserted(conv));
        }

        Ok(())
    }

    /// Delete a conversation and all its messages (cascade).
    /// Replace all persisted messages for a conversation with the provided
    /// set.  Existing messages are deleted first; the new messages are then
    /// inserted in order.  The conversation record itself is kept intact.
    ///
    /// Used by context compaction to persist a shrunk history so subsequent
    /// turns do not reload the full pre-compaction history from storage.
    pub async fn replace_history(&self, conversation_id: Uuid, messages: &[Message]) -> Result<()> {
        let conv_id_str = conversation_id.to_string();

        // Verify the conversation belongs to this agent before mutating.
        self.ensure_conversation_agent(conversation_id).await?;

        sqlx::query("DELETE FROM messages WHERE conversation_id = ?1")
            .bind(&conv_id_str)
            .execute(&self.pool)
            .await
            .with_context(|| format!("deleting messages for conversation {conversation_id}"))?;

        for msg in messages {
            self.save_message(msg).await?;
        }

        Ok(())
    }

    pub async fn delete_conversation(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        sqlx::query("DELETE FROM conversations WHERE id = ?1 AND agent_id = ?2")
            .bind(&id_str)
            .bind(&self.agent_id)
            .execute(&self.pool)
            .await?;

        if let Some(b) = &self.broadcaster {
            b.emit(ConversationEvent::Deleted {
                conversation_id: id,
                agent_id: self.agent_id.clone(),
            });
        }

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

        self.ensure_conversation_agent(msg.conversation_id).await?;

        sqlx::query(
            "INSERT INTO messages \
                (id, conversation_id, role, content, skill_name, tool_calls_json, turn, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(&id)
        .bind(&conversation_id)
        .bind(&role)
        .bind(&msg.content)
        .bind(&msg.skill_name)
        .bind(&msg.tool_calls_json)
        .bind(msg.turn)
        .bind(msg.created_at)
        .execute(&self.pool)
        .await?;

        // Update the conversation's updated_at timestamp
        let now = Utc::now();
        sqlx::query("UPDATE conversations SET updated_at = ?1 WHERE id = ?2 AND agent_id = ?3")
            .bind(now)
            .bind(&conversation_id)
            .bind(&self.agent_id)
            .execute(&self.pool)
            .await?;

        if let Some(b) = &self.broadcaster
            && let Ok(Some(conv)) = self.get_conversation(msg.conversation_id).await
        {
            b.emit(ConversationEvent::Upserted(conv));
        }

        Ok(())
    }

    async fn ensure_conversation_agent(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        let owner =
            sqlx::query_scalar::<_, String>("SELECT agent_id FROM conversations WHERE id = ?1")
                .bind(&id_str)
                .fetch_optional(&self.pool)
                .await?;

        match owner {
            Some(found) if found == self.agent_id => Ok(()),
            Some(found) => anyhow::bail!(
                "conversation {} belongs to agent '{}' (requested '{}')",
                id,
                found,
                self.agent_id
            ),
            None => anyhow::bail!("conversation {} not found", id),
        }
    }

    /// Load all messages for a conversation, ordered by turn then created_at.
    pub async fn load_history(&self, conversation_id: Uuid) -> Result<Vec<Message>> {
        let conv_id_str = conversation_id.to_string();

        let rows = sqlx::query(
            "SELECT m.id, m.conversation_id, m.role, m.content, m.skill_name, m.tool_calls_json, m.turn, m.created_at \
             FROM messages m \
             INNER JOIN conversations c ON c.id = m.conversation_id \
             WHERE m.conversation_id = ?1 AND c.agent_id = ?2 \
             ORDER BY m.turn ASC, m.created_at ASC",
        )
        .bind(&conv_id_str)
        .bind(&self.agent_id)
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
                    tool_calls_json: r.get("tool_calls_json"),
                    turn: r.get("turn"),
                    created_at: r.get("created_at"),
                })
            })
            .collect()
    }

    /// Fetch a single message by its ID (agent-scoped via the conversation join).
    pub async fn get_message(&self, message_id: Uuid) -> Result<Option<Message>> {
        let id_str = message_id.to_string();
        let row = sqlx::query(
            "SELECT m.id, m.conversation_id, m.role, m.content, m.skill_name, m.tool_calls_json, m.turn, m.created_at \
             FROM messages m \
             INNER JOIN conversations c ON c.id = m.conversation_id \
             WHERE m.id = ?1 AND c.agent_id = ?2",
        )
        .bind(&id_str)
        .bind(&self.agent_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| {
            let id_s: String = r.get("id");
            let conv_s: String = r.get("conversation_id");
            let role_s: String = r.get("role");
            Ok(Message {
                id: Uuid::parse_str(&id_s)?,
                conversation_id: Uuid::parse_str(&conv_s)?,
                role: parse_role(&role_s)?,
                content: r.get("content"),
                skill_name: r.get("skill_name"),
                tool_calls_json: r.get("tool_calls_json"),
                turn: r.get("turn"),
                created_at: r.get("created_at"),
            })
        })
        .transpose()
    }

    /// Return the last `limit` messages for a conversation, in chronological order.
    pub async fn last_messages(&self, conversation_id: Uuid, limit: i64) -> Result<Vec<Message>> {
        let conv_id_str = conversation_id.to_string();

        // Fetch the newest rows first, then reverse to restore chronological order.
        let rows = sqlx::query(
            "SELECT m.id, m.conversation_id, m.role, m.content, m.skill_name, m.tool_calls_json, m.turn, m.created_at \
             FROM messages m \
             INNER JOIN conversations c ON c.id = m.conversation_id \
             WHERE m.conversation_id = ?1 AND c.agent_id = ?2 \
             ORDER BY m.turn DESC, m.created_at DESC \
             LIMIT ?3",
        )
        .bind(&conv_id_str)
        .bind(&self.agent_id)
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
                    tool_calls_json: r.get("tool_calls_json"),
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
    use std::sync::Arc;

    use crate::StorageLayer;
    use crate::conversation_broadcaster::{
        ConversationBroadcast, ConversationEvent, InMemoryConversationBroadcaster,
    };
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
    async fn test_update_title() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();

        let conv = store.create_conversation(Some("Old Title")).await.unwrap();
        store.update_title(conv.id, "New Title").await.unwrap();

        let loaded = store.get_conversation(conv.id).await.unwrap().unwrap();
        assert_eq!(
            loaded.title.as_deref(),
            Some("New Title"),
            "title should be updated"
        );
        assert!(
            loaded.updated_at >= conv.updated_at,
            "updated_at should advance"
        );
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

    #[tokio::test]
    async fn test_conversation_cannot_cross_agent_boundary() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let default_store = storage.conversation_store_for_agent("default");
        let work_store = storage.conversation_store_for_agent("work");

        let conv = default_store
            .create_conversation(Some("Default only"))
            .await
            .unwrap();

        let err = work_store
            .create_conversation_with_id(conv.id, Some("Wrong owner"))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("belongs to agent"),
            "expected ownership error, got: {err}"
        );

        let mut msg = Message::user(conv.id, "cross-agent write");
        msg.turn = 1;
        let err = work_store.save_message(&msg).await.unwrap_err();
        assert!(
            err.to_string().contains("belongs to agent"),
            "expected ownership error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_create_conversation_emits_upserted_event() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let broadcaster = Arc::new(InMemoryConversationBroadcaster::new());
        let mut rx = broadcaster.subscribe();

        let store = storage
            .conversation_store()
            .with_broadcaster(broadcaster.clone());

        let conv = store.create_conversation(Some("Hello")).await.unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("timed out waiting for conversation event")
            .expect("should receive event");
        match event {
            ConversationEvent::Upserted(record) => {
                assert_eq!(
                    record.id, conv.id,
                    "event should carry created conversation id"
                );
            }
            _ => panic!("expected Upserted event"),
        }
    }

    #[tokio::test]
    async fn test_delete_conversation_emits_deleted_event() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let broadcaster = Arc::new(InMemoryConversationBroadcaster::new());

        let store = storage
            .conversation_store()
            .with_broadcaster(broadcaster.clone());

        let conv = store.create_conversation(Some("To delete")).await.unwrap();

        // Subscribe after create to skip the Upserted event from creation.
        let mut rx = broadcaster.subscribe();

        store.delete_conversation(conv.id).await.unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("timed out waiting for conversation event")
            .expect("should receive event");
        match event {
            ConversationEvent::Deleted {
                conversation_id,
                agent_id,
            } => {
                assert_eq!(conversation_id, conv.id, "deleted id should match");
                assert_eq!(agent_id, "default", "agent_id should match");
            }
            _ => panic!("expected Deleted event"),
        }
    }

    #[tokio::test]
    async fn test_update_title_emits_upserted_event() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let broadcaster = Arc::new(InMemoryConversationBroadcaster::new());

        let store = storage
            .conversation_store()
            .with_broadcaster(broadcaster.clone());

        let conv = store.create_conversation(Some("Old")).await.unwrap();

        let mut rx = broadcaster.subscribe();
        store.update_title(conv.id, "New").await.unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("timed out waiting for conversation event")
            .expect("should receive event");
        match event {
            ConversationEvent::Upserted(record) => {
                assert_eq!(
                    record.id, conv.id,
                    "updated event should carry conversation id"
                );
                assert_eq!(
                    record.title.as_deref(),
                    Some("New"),
                    "title should be updated"
                );
            }
            _ => panic!("expected Upserted event"),
        }
    }

    #[tokio::test]
    async fn test_save_message_emits_upserted_event() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let broadcaster = Arc::new(InMemoryConversationBroadcaster::new());

        let store = storage
            .conversation_store()
            .with_broadcaster(broadcaster.clone());

        let conv = store.create_conversation(None).await.unwrap();

        let mut rx = broadcaster.subscribe();

        let mut msg = Message::user(conv.id, "Hello");
        msg.turn = 1;
        store.save_message(&msg).await.unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("timed out waiting for conversation event")
            .expect("should receive event");
        match event {
            ConversationEvent::Upserted(record) => {
                assert_eq!(record.id, conv.id, "should emit for the conversation");
            }
            _ => panic!("expected Upserted event"),
        }
    }
}
