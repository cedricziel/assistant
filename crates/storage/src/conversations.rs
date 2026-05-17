//! Conversation and message persistence.
//!
//! Defines the [`ConversationStore`] trait (the API consumers depend on),
//! its SQLite-backed implementation [`SqliteConversationStore`], and the
//! in-memory test implementation [`InMemoryConversationStore`].
//!
//! TODO(workspace-test-coverage-floor): the `ConversationStore` trait will
//! eventually move to `assistant-core` once `ConversationRecord` is also
//! relocated there. The trait currently lives alongside the record types
//! to keep the migration tractable.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use assistant_core::clock::{Clock, SystemClock};
use assistant_core::types::conversation::{Message, MessageRole};
use async_trait::async_trait;
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
    /// When true, the title-generator worker MUST NOT overwrite this title.
    /// Set by manual renames, explicit titles at creation, and successful
    /// auto-title runs.
    pub title_locked: bool,
    /// The user who owns this conversation (multi-user scoping).
    /// `None` for legacy/unscoped conversations.
    pub user_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Trait-based interface for conversation + message persistence.
///
/// Consumers in `assistant-runtime`, `assistant-web-ui`, and
/// `assistant-interfaces` depend on this trait rather than the concrete
/// SQLite implementation, so tests can substitute
/// [`InMemoryConversationStore`].
#[async_trait]
pub trait ConversationStore: Send + Sync {
    /// Create or retrieve a conversation by a specific UUID.
    async fn create_conversation_with_id(
        &self,
        id: Uuid,
        title: Option<&str>,
    ) -> Result<ConversationRecord>;

    /// Create a new conversation and return its metadata.
    async fn create_conversation(&self, title: Option<&str>) -> Result<ConversationRecord>;

    /// Fetch a conversation by ID. Returns `None` if not found.
    async fn get_conversation(&self, id: Uuid) -> Result<Option<ConversationRecord>>;

    /// List all conversations for the configured agent (most-recently-updated first).
    async fn list_conversations(&self) -> Result<Vec<ConversationRecord>>;

    /// Set a conversation's title and lock it from auto-titling.
    async fn update_title(&self, id: Uuid, title: &str) -> Result<()>;

    /// Replace the full message history of a conversation.
    async fn replace_history(&self, conversation_id: Uuid, messages: &[Message]) -> Result<()>;

    /// Delete a conversation (and its messages).
    async fn delete_conversation(&self, id: Uuid) -> Result<()>;

    /// Persist a single message.
    async fn save_message(&self, msg: &Message) -> Result<()>;

    /// Load the full message history for a conversation.
    async fn load_history(&self, conversation_id: Uuid) -> Result<Vec<Message>>;

    /// Fetch a single message by ID.
    async fn get_message(&self, message_id: Uuid) -> Result<Option<Message>>;

    /// Return the last N messages for a conversation, oldest first.
    async fn last_messages(&self, conversation_id: Uuid, limit: i64) -> Result<Vec<Message>>;
}

/// SQLite-backed store for conversations and messages.
pub struct SqliteConversationStore {
    pool: SqlitePool,
    agent_id: String,
    /// When set, all queries are scoped to this user.
    user_id: Option<String>,
    broadcaster: Option<Arc<dyn ConversationBroadcast>>,
    /// Clock for row timestamps. Default `Arc::new(SystemClock)`.
    clock: Arc<dyn Clock>,
}

impl SqliteConversationStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            agent_id: "default".to_string(),
            user_id: None,
            broadcaster: None,
            clock: Arc::new(SystemClock),
        }
    }

    pub fn for_agent(pool: SqlitePool, agent_id: impl Into<String>) -> Self {
        Self {
            pool,
            agent_id: agent_id.into(),
            user_id: None,
            broadcaster: None,
            clock: Arc::new(SystemClock),
        }
    }

    /// Scope all queries to a specific user.
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Inject a [`Clock`] implementation. Tests pass `Arc::new(FakeClock::new(...))`
    /// to assert row-timestamp behavior against virtual time.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Attach a broadcaster that will receive events on every conversation mutation.
    pub fn with_broadcaster(mut self, broadcaster: Arc<dyn ConversationBroadcast>) -> Self {
        self.broadcaster = Some(broadcaster);
        self
    }

    /// Private helper: verify that the conversation belongs to this store's
    /// configured agent. Called from `create_conversation_with_id` to enforce
    /// agent scoping on inserts that touch existing rows.
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
}

// -----------------------------------------------------------------------
// ConversationStore trait impl for SqliteConversationStore
// -----------------------------------------------------------------------

#[async_trait]
impl ConversationStore for SqliteConversationStore {
    /// Create or retrieve a conversation by a specific UUID.
    /// If a row with that ID already exists, return it unchanged.
    async fn create_conversation_with_id(
        &self,
        id: Uuid,
        title: Option<&str>,
    ) -> Result<ConversationRecord> {
        let now = self.clock.now();
        let id_str = id.to_string();
        let title_locked = title.is_some();

        sqlx::query(
            "INSERT INTO conversations (id, title, title_locked, agent_id, user_id, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6) \
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(&id_str)
        .bind(title)
        .bind(title_locked)
        .bind(&self.agent_id)
        .bind(&self.user_id)
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
    async fn create_conversation(&self, title: Option<&str>) -> Result<ConversationRecord> {
        let id = Uuid::new_v4();
        let now = self.clock.now();
        let id_str = id.to_string();
        let title_locked = title.is_some();

        sqlx::query(
            "INSERT INTO conversations (id, title, title_locked, agent_id, user_id, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        )
        .bind(&id_str)
        .bind(title)
        .bind(title_locked)
        .bind(&self.agent_id)
        .bind(&self.user_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let conv = ConversationRecord {
            id,
            agent_id: self.agent_id.clone(),
            title: title.map(|s| s.to_string()),
            title_locked,
            user_id: self.user_id.clone(),
            created_at: now,
            updated_at: now,
        };

        if let Some(b) = &self.broadcaster {
            b.emit(ConversationEvent::Upserted(conv.clone()));
        }

        Ok(conv)
    }

    /// Fetch a conversation by ID. Returns `None` if not found.
    async fn get_conversation(&self, id: Uuid) -> Result<Option<ConversationRecord>> {
        let id_str = id.to_string();

        let row = if self.user_id.is_some() {
            sqlx::query(
                "SELECT id, title, title_locked, agent_id, user_id, created_at, updated_at \
                 FROM conversations \
                 WHERE id = ?1 AND agent_id = ?2 AND user_id = ?3",
            )
            .bind(&id_str)
            .bind(&self.agent_id)
            .bind(&self.user_id)
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, title, title_locked, agent_id, user_id, created_at, updated_at \
                 FROM conversations \
                 WHERE id = ?1 AND agent_id = ?2",
            )
            .bind(&id_str)
            .bind(&self.agent_id)
            .fetch_optional(&self.pool)
            .await?
        };

        row.map(|r| {
            let raw_id: String = r.get("id");
            Ok(ConversationRecord {
                id: Uuid::parse_str(&raw_id)?,
                agent_id: r.get("agent_id"),
                title: r.get("title"),
                title_locked: r.get("title_locked"),
                user_id: r.get("user_id"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
        })
        .transpose()
    }

    /// List all conversations, most-recently updated first.
    async fn list_conversations(&self) -> Result<Vec<ConversationRecord>> {
        let rows = if self.user_id.is_some() {
            sqlx::query(
                "SELECT id, title, title_locked, agent_id, user_id, created_at, updated_at \
                 FROM conversations \
                 WHERE agent_id = ?1 AND user_id = ?2 \
                 ORDER BY updated_at DESC",
            )
            .bind(&self.agent_id)
            .bind(&self.user_id)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, title, title_locked, agent_id, user_id, created_at, updated_at \
                 FROM conversations \
                 WHERE agent_id = ?1 \
                 ORDER BY updated_at DESC",
            )
            .bind(&self.agent_id)
            .fetch_all(&self.pool)
            .await?
        };

        rows.into_iter()
            .map(|r| {
                let raw_id: String = r.get("id");
                Ok(ConversationRecord {
                    id: Uuid::parse_str(&raw_id)?,
                    agent_id: r.get("agent_id"),
                    title: r.get("title"),
                    title_locked: r.get("title_locked"),
                    user_id: r.get("user_id"),
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                })
            })
            .collect()
    }

    /// Update the title of an existing conversation.
    ///
    /// Always sets `title_locked = 1` so the title-generator worker will not
    /// overwrite this value on subsequent turns.
    ///
    /// Returns an error if the conversation does not exist.
    async fn update_title(&self, id: Uuid, title: &str) -> Result<()> {
        let id_str = id.to_string();
        let result = sqlx::query(
            "UPDATE conversations
                 SET title = ?1, title_locked = 1, updated_at = ?2
                 WHERE id = ?3 AND agent_id = ?4",
        )
        .bind(title)
        .bind(self.clock.now())
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
    async fn replace_history(&self, conversation_id: Uuid, messages: &[Message]) -> Result<()> {
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

    async fn delete_conversation(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        let result = sqlx::query("DELETE FROM conversations WHERE id = ?1 AND agent_id = ?2")
            .bind(&id_str)
            .bind(&self.agent_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() > 0
            && let Some(b) = &self.broadcaster
        {
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
    async fn save_message(&self, msg: &Message) -> Result<()> {
        let id = msg.id.to_string();
        let conversation_id = msg.conversation_id.to_string();
        let role = msg.role.to_string();

        self.ensure_conversation_agent(msg.conversation_id).await?;

        sqlx::query(
            "INSERT INTO messages \
                (id, conversation_id, role, content, skill_name, tool_calls_json, turn, created_at, sender_user_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
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
        .bind(&msg.sender_user_id)
        .execute(&self.pool)
        .await?;

        // Update the conversation's updated_at timestamp
        let now = self.clock.now();
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

    /// Load all messages for a conversation, ordered by turn then created_at.
    async fn load_history(&self, conversation_id: Uuid) -> Result<Vec<Message>> {
        let conv_id_str = conversation_id.to_string();

        let rows = sqlx::query(
            "SELECT m.id, m.conversation_id, m.role, m.content, m.skill_name, m.tool_calls_json, m.turn, m.created_at, m.sender_user_id \
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
                    sender_user_id: r.get("sender_user_id"),
                })
            })
            .collect()
    }

    /// Fetch a single message by its ID (agent-scoped via the conversation join).
    async fn get_message(&self, message_id: Uuid) -> Result<Option<Message>> {
        let id_str = message_id.to_string();
        let row = sqlx::query(
            "SELECT m.id, m.conversation_id, m.role, m.content, m.skill_name, m.tool_calls_json, m.turn, m.created_at, m.sender_user_id \
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
                sender_user_id: r.get("sender_user_id"),
            })
        })
        .transpose()
    }

    /// Return the last `limit` messages for a conversation, in chronological order.
    async fn last_messages(&self, conversation_id: Uuid, limit: i64) -> Result<Vec<Message>> {
        let conv_id_str = conversation_id.to_string();

        // Fetch the newest rows first, then reverse to restore chronological order.
        let rows = sqlx::query(
            "SELECT m.id, m.conversation_id, m.role, m.content, m.skill_name, m.tool_calls_json, m.turn, m.created_at, m.sender_user_id \
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
                    sender_user_id: r.get("sender_user_id"),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // Restore chronological order
        messages.reverse();
        Ok(messages)
    }
}

// ---------------------------------------------------------------------------
// In-memory implementation (test-only — see workspace_test_impls_in_prod.rs)
// ---------------------------------------------------------------------------

/// In-memory [`ConversationStore`] implementation backed by hash maps.
///
/// Intended for unit tests of upstream consumers that need persistence-
/// shaped storage without booting SQLite. Constructed via
/// `InMemoryConversationStore::new()`; tests typically wrap in `Arc<dyn ConversationStore>`.
///
/// Honors agent scoping and basic referential cascades (delete-conversation
/// removes its messages). Title locking and broadcaster integration are
/// modeled. FTS-style search would land here if/when added to the trait.
///
/// The workspace lint `tests/workspace_test_impls_in_prod.rs` forbids
/// constructing this type outside test code or `assistant-test-support`.
pub struct InMemoryConversationStore {
    state: Arc<Mutex<InMemoryState>>,
    agent_id: String,
    user_id: Option<String>,
    broadcaster: Option<Arc<dyn ConversationBroadcast>>,
    clock: Arc<dyn Clock>,
}

#[derive(Default)]
struct InMemoryState {
    conversations: HashMap<Uuid, ConversationRecord>,
    messages: HashMap<Uuid, Vec<Message>>,
}

impl InMemoryConversationStore {
    /// Create a new empty store with the default agent (`"default"`).
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemoryState::default())),
            agent_id: "default".to_string(),
            user_id: None,
            broadcaster: None,
            clock: Arc::new(SystemClock),
        }
    }

    /// Scope to a specific agent.
    pub fn for_agent(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            ..Self::new()
        }
    }

    /// Scope all queries to a specific user.
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Inject a [`Clock`] implementation.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Attach a broadcaster that will receive events on every conversation mutation.
    pub fn with_broadcaster(mut self, broadcaster: Arc<dyn ConversationBroadcast>) -> Self {
        self.broadcaster = Some(broadcaster);
        self
    }

    /// Lock the inner state for the duration of a single operation.
    /// Recovers from poisoned locks by extracting the inner state.
    fn lock_state(&self) -> std::sync::MutexGuard<'_, InMemoryState> {
        match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl Default for InMemoryConversationStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConversationStore for InMemoryConversationStore {
    async fn create_conversation_with_id(
        &self,
        id: Uuid,
        title: Option<&str>,
    ) -> Result<ConversationRecord> {
        let now = self.clock.now();
        let title_locked = title.is_some();
        let record = {
            let mut state = self.lock_state();
            let entry = state
                .conversations
                .entry(id)
                .or_insert_with(|| ConversationRecord {
                    id,
                    agent_id: self.agent_id.clone(),
                    title: title.map(String::from),
                    title_locked,
                    user_id: self.user_id.clone(),
                    created_at: now,
                    updated_at: now,
                });
            if entry.agent_id != self.agent_id {
                anyhow::bail!(
                    "conversation {} belongs to agent '{}' (requested '{}')",
                    id,
                    entry.agent_id,
                    self.agent_id
                );
            }
            entry.clone()
        };
        if let Some(b) = &self.broadcaster {
            b.emit(ConversationEvent::Upserted(record.clone()));
        }
        Ok(record)
    }

    async fn create_conversation(&self, title: Option<&str>) -> Result<ConversationRecord> {
        self.create_conversation_with_id(Uuid::new_v4(), title)
            .await
    }

    async fn get_conversation(&self, id: Uuid) -> Result<Option<ConversationRecord>> {
        let state = self.lock_state();
        let row = state
            .conversations
            .get(&id)
            .filter(|c| c.agent_id == self.agent_id)
            .filter(|c| {
                self.user_id
                    .as_ref()
                    .is_none_or(|u| c.user_id.as_ref() == Some(u))
            })
            .cloned();
        Ok(row)
    }

    async fn list_conversations(&self) -> Result<Vec<ConversationRecord>> {
        let state = self.lock_state();
        let mut out: Vec<ConversationRecord> = state
            .conversations
            .values()
            .filter(|c| c.agent_id == self.agent_id)
            .filter(|c| {
                self.user_id
                    .as_ref()
                    .is_none_or(|u| c.user_id.as_ref() == Some(u))
            })
            .cloned()
            .collect();
        out.sort_by_key(|c| std::cmp::Reverse(c.updated_at));
        Ok(out)
    }

    async fn update_title(&self, id: Uuid, title: &str) -> Result<()> {
        let now = self.clock.now();
        let updated = {
            let mut state = self.lock_state();
            let conv = state.conversations.get_mut(&id);
            match conv {
                Some(c) if c.agent_id == self.agent_id => {
                    c.title = Some(title.to_string());
                    c.title_locked = true;
                    c.updated_at = now;
                    Some(c.clone())
                }
                _ => None,
            }
        };
        if let Some(rec) = updated
            && let Some(b) = &self.broadcaster
        {
            b.emit(ConversationEvent::Upserted(rec));
        }
        Ok(())
    }

    async fn replace_history(&self, conversation_id: Uuid, messages: &[Message]) -> Result<()> {
        let mut state = self.lock_state();
        state.messages.insert(conversation_id, messages.to_vec());
        Ok(())
    }

    async fn delete_conversation(&self, id: Uuid) -> Result<()> {
        let mut state = self.lock_state();
        state.conversations.remove(&id);
        state.messages.remove(&id);
        if let Some(b) = &self.broadcaster {
            b.emit(ConversationEvent::Deleted {
                conversation_id: id,
                agent_id: self.agent_id.clone(),
            });
        }
        Ok(())
    }

    async fn save_message(&self, msg: &Message) -> Result<()> {
        let mut state = self.lock_state();
        state
            .messages
            .entry(msg.conversation_id)
            .or_default()
            .push(msg.clone());
        if let Some(conv) = state.conversations.get_mut(&msg.conversation_id) {
            conv.updated_at = self.clock.now();
        }
        Ok(())
    }

    async fn load_history(&self, conversation_id: Uuid) -> Result<Vec<Message>> {
        let state = self.lock_state();
        Ok(state
            .messages
            .get(&conversation_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn get_message(&self, message_id: Uuid) -> Result<Option<Message>> {
        let state = self.lock_state();
        Ok(state
            .messages
            .values()
            .flat_map(|v| v.iter())
            .find(|m| m.id == message_id)
            .cloned())
    }

    async fn last_messages(&self, conversation_id: Uuid, limit: i64) -> Result<Vec<Message>> {
        let state = self.lock_state();
        let mut out = state
            .messages
            .get(&conversation_id)
            .cloned()
            .unwrap_or_default();
        if limit > 0 {
            let limit = limit as usize;
            if out.len() > limit {
                let start = out.len() - limit;
                out = out.split_off(start);
            }
        }
        Ok(out)
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

    use super::*;
    use crate::StorageLayer;
    use crate::conversation_broadcaster::{
        ConversationBroadcast, ConversationEvent, InMemoryConversationBroadcaster,
    };
    use assistant_core::types::conversation::Message;
    use uuid::Uuid;

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
    async fn test_create_conversation_with_title_locks() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();

        let titled = store.create_conversation(Some("My Title")).await.unwrap();
        assert!(titled.title_locked);
        assert_eq!(titled.title.as_deref(), Some("My Title"));

        let untitled = store.create_conversation(None).await.unwrap();
        assert!(!untitled.title_locked);
        assert!(untitled.title.is_none());
    }

    #[tokio::test]
    async fn test_create_conversation_with_id_with_title_locks() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();

        let id = Uuid::new_v4();
        let conv = store
            .create_conversation_with_id(id, Some("Pinned"))
            .await
            .unwrap();
        assert!(conv.title_locked);

        let id2 = Uuid::new_v4();
        let conv2 = store.create_conversation_with_id(id2, None).await.unwrap();
        assert!(!conv2.title_locked);
    }

    #[tokio::test]
    async fn test_update_title_locks() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();

        let conv = store.create_conversation(None).await.unwrap();
        assert!(
            !conv.title_locked,
            "fresh untitled conversation is unlocked"
        );

        store.update_title(conv.id, "Fresh Title").await.unwrap();

        let loaded = store.get_conversation(conv.id).await.unwrap().unwrap();
        assert_eq!(loaded.title.as_deref(), Some("Fresh Title"));
        assert!(
            loaded.title_locked,
            "update_title must set title_locked = 1"
        );
    }

    #[tokio::test]
    async fn test_title_locked_defaults_to_zero_for_new_conversations() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();

        let untitled = store.create_conversation(None).await.unwrap();
        assert!(
            !untitled.title_locked,
            "conversation with no title should not be title-locked"
        );

        let titled = store.create_conversation(Some("x")).await.unwrap();
        assert!(
            titled.title_locked,
            "conversation created with a title should be title-locked"
        );

        // Round-trip through storage to ensure the field is persisted, not just
        // returned from the in-memory builder.
        let loaded_untitled = store.get_conversation(untitled.id).await.unwrap().unwrap();
        assert!(!loaded_untitled.title_locked);
        let loaded_titled = store.get_conversation(titled.id).await.unwrap().unwrap();
        assert!(loaded_titled.title_locked);
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

    /// Regression test for #267 — `list_conversations` SHALL return rows in
    /// `updated_at DESC` order. New messages MUST bump `updated_at` so the
    /// affected conversation rises to the top of the list.
    #[tokio::test]
    async fn test_list_orders_by_updated_at_desc() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();

        // Create A, then B. Initially B sorts ahead of A because it was
        // created later.
        let conv_a = store.create_conversation(Some("Older")).await.unwrap();
        // Sleep so the second create's timestamp is strictly greater.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        let conv_b = store.create_conversation(Some("Newer")).await.unwrap();

        let listed = store.list_conversations().await.unwrap();
        assert_eq!(
            listed.first().map(|c| c.id),
            Some(conv_b.id),
            "newest conversation should be first on initial list"
        );
        assert_eq!(
            listed.get(1).map(|c| c.id),
            Some(conv_a.id),
            "older conversation should be second"
        );

        // Add a message to A — its updated_at bumps. A should now lead.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        let mut msg = Message::user(conv_a.id, "ping");
        msg.turn = 1;
        store.save_message(&msg).await.unwrap();

        let listed = store.list_conversations().await.unwrap();
        assert_eq!(
            listed.first().map(|c| c.id),
            Some(conv_a.id),
            "activity on older conversation should bump it to the top (#267)"
        );
        assert_eq!(
            listed.get(1).map(|c| c.id),
            Some(conv_b.id),
            "the previously-newest conversation should now be second",
        );
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
                assert!(
                    record.title_locked,
                    "Upserted broadcast after update_title must carry title_locked = true"
                );
            }
            _ => panic!("expected Upserted event"),
        }
    }

    // -----------------------------------------------------------------------
    // User-scoping tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_user_scoped_conversation_isolation() {
        let storage = StorageLayer::new_in_memory().await.unwrap();

        let alice_store = storage
            .conversation_store()
            .with_user_id("alice".to_string());
        let bob_store = storage.conversation_store().with_user_id("bob".to_string());

        let alice_conv = alice_store
            .create_conversation(Some("Alice's chat"))
            .await
            .unwrap();
        let bob_conv = bob_store
            .create_conversation(Some("Bob's chat"))
            .await
            .unwrap();

        // Alice sees only her conversation
        let alice_list = alice_store.list_conversations().await.unwrap();
        assert_eq!(alice_list.len(), 1, "Alice should see 1 conversation");
        assert_eq!(
            alice_list[0].id, alice_conv.id,
            "Alice's list should contain her conversation"
        );
        assert_eq!(
            alice_list[0].user_id.as_deref(),
            Some("alice"),
            "conversation should retain Alice's user_id"
        );

        // Bob sees only his
        let bob_list = bob_store.list_conversations().await.unwrap();
        assert_eq!(bob_list.len(), 1, "Bob should see 1 conversation");
        assert_eq!(
            bob_list[0].id, bob_conv.id,
            "Bob's list should contain his conversation"
        );
        assert_eq!(
            bob_list[0].user_id.as_deref(),
            Some("bob"),
            "conversation should retain Bob's user_id"
        );

        // Alice cannot get Bob's conversation
        let cross = alice_store.get_conversation(bob_conv.id).await.unwrap();
        assert!(cross.is_none(), "Alice should not see Bob's conversation");
    }

    #[tokio::test]
    async fn test_unscoped_store_sees_all_conversations() {
        let storage = StorageLayer::new_in_memory().await.unwrap();

        let alice_store = storage
            .conversation_store()
            .with_user_id("alice".to_string());
        let unscoped_store = storage.conversation_store();

        alice_store
            .create_conversation(Some("Alice's chat"))
            .await
            .unwrap();
        unscoped_store
            .create_conversation(Some("Legacy chat"))
            .await
            .unwrap();

        // Unscoped store sees everything
        let all = unscoped_store.list_conversations().await.unwrap();
        assert_eq!(all.len(), 2, "unscoped store should see all conversations");
    }

    #[tokio::test]
    async fn test_message_sender_user_id_persisted() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();

        let conv = store.create_conversation(None).await.unwrap();

        let mut msg = Message::user(conv.id, "from alice");
        msg.turn = 1;
        msg.sender_user_id = Some("alice".to_string());
        store.save_message(&msg).await.unwrap();

        let history = store.load_history(conv.id).await.unwrap();
        assert_eq!(history.len(), 1, "should have exactly one message");
        assert_eq!(
            history[0].sender_user_id.as_deref(),
            Some("alice"),
            "sender_user_id should round-trip"
        );
    }

    #[tokio::test]
    async fn test_message_sender_user_id_defaults_to_none() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();

        let conv = store.create_conversation(None).await.unwrap();

        let mut msg = Message::user(conv.id, "no sender");
        msg.turn = 1;
        store.save_message(&msg).await.unwrap();

        let history = store.load_history(conv.id).await.unwrap();
        assert!(
            history[0].sender_user_id.is_none(),
            "sender_user_id should default to None"
        );
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
