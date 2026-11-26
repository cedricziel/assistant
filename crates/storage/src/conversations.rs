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
use assistant_core::types::conversation::{Message, MessageFinishReason, MessageRole};
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
    /// When the owner pinned this conversation; `None` when not pinned.
    /// Pinned conversations list first, newest pin first.
    pub pinned_at: Option<DateTime<Utc>>,
}

/// A partial update of conversation metadata. `None` fields are left unchanged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConversationPatch {
    /// New title. Setting it locks the title against auto-titling and
    /// overwrites an already locked title.
    pub title: Option<String>,
    /// `Some(true)` pins (keeping an existing `pinned_at`), `Some(false)` unpins.
    pub pinned: Option<bool>,
}

/// Outcome of [`ConversationStore::update_metadata`].
#[derive(Debug, Clone)]
pub enum MetadataUpdate {
    /// The patch was applied; carries the stored row after the update.
    Updated(ConversationRecord),
    /// No conversation with that id exists for this store's scope.
    NotFound,
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

    /// List all conversations for the configured agent: pinned first (newest
    /// pin first), then most-recently-updated first.
    async fn list_conversations(&self) -> Result<Vec<ConversationRecord>>;

    /// Set a conversation's title and lock it from auto-titling.
    ///
    /// Does not change `updated_at`: renaming must not reorder the list.
    async fn update_title(&self, id: Uuid, title: &str) -> Result<()>;

    /// Apply a partial metadata update in one step and emit one `Upserted`
    /// event. Does not change `updated_at`.
    async fn update_metadata(&self, id: Uuid, patch: ConversationPatch) -> Result<MetadataUpdate>;

    /// Write an auto-generated title, only while the conversation is unlocked.
    ///
    /// Locks the row and leaves `updated_at` unchanged. Returns `false` (and
    /// emits no event) when the row is missing or already locked, so a manual
    /// rename that lands while the LLM is running is never overwritten.
    async fn set_generated_title(&self, id: Uuid, title: &str) -> Result<bool>;

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

    /// Persist `msg` as a child of `parent_id` (`None` creates a new root)
    /// and make it the active leaf.
    ///
    /// Errors when `parent_id` is not a message of the same conversation.
    /// Like [`save_message`](Self::save_message), a conflicting message id
    /// inserts nothing and leaves the active leaf where it is.
    async fn branch_message(&self, msg: &Message, parent_id: Option<Uuid>) -> Result<()>;

    /// The children of `parent_id` in creation order; `None` lists the roots.
    async fn children_of(
        &self,
        conversation_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> Result<Vec<Message>>;

    /// `message_id` and its siblings (same parent, or all roots) in creation
    /// order. `None` when the message is not in the conversation.
    async fn list_siblings(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
    ) -> Result<Option<Vec<Message>>>;

    /// The chain from the root down to and including `message_id`.
    /// `None` when the message is not in the conversation.
    async fn ancestors(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
    ) -> Result<Option<Vec<Message>>>;

    /// Switch the active path to the branch through `message_id`: descend
    /// from it through the most recently created child at each level and make
    /// the reached message the active leaf. Returns the new leaf, or `None`
    /// when the message is not in the conversation.
    ///
    /// Leaves `updated_at` unchanged and emits [`ConversationEvent::Upserted`].
    async fn switch_to(&self, conversation_id: Uuid, message_id: Uuid) -> Result<Option<Uuid>>;

    /// Make `message_id` the active leaf exactly, without descending.
    /// Returns it, or `None` when the message is not in the conversation.
    async fn set_leaf(&self, conversation_id: Uuid, message_id: Uuid) -> Result<Option<Uuid>>;

    /// Delete the rows written by turn run `run_id`, except `keep`, in one
    /// statement. Returns how many rows were removed. Errors, deleting
    /// nothing, when a surviving row has a deleted row as its parent. Does
    /// not move the active leaf; callers reset it with
    /// [`set_leaf`](Self::set_leaf).
    async fn delete_run_rows(
        &self,
        conversation_id: Uuid,
        run_id: Uuid,
        keep: Option<Uuid>,
    ) -> Result<u64>;
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

/// Map a `conversations` row (see the `SELECT` lists above) to a record.
fn conversation_from_row(r: &sqlx::sqlite::SqliteRow) -> Result<ConversationRecord> {
    let raw_id: String = r.get("id");
    Ok(ConversationRecord {
        id: Uuid::parse_str(&raw_id)?,
        agent_id: r.get("agent_id"),
        title: r.get("title"),
        title_locked: r.get("title_locked"),
        user_id: r.get("user_id"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
        pinned_at: r.get("pinned_at"),
    })
}

// -----------------------------------------------------------------------
// ConversationStore trait impl for SqliteConversationStore
// -----------------------------------------------------------------------

impl SqliteConversationStore {
    /// Insert `msg` and make it the active leaf in one `BEGIN IMMEDIATE`
    /// transaction. `parent` is `None` to append under the current leaf, or
    /// `Some(explicit_parent)` to branch (the inner `None` is a new root).
    async fn insert_message(&self, msg: &Message, parent: Option<Option<Uuid>>) -> Result<()> {
        let id = msg.id.to_string();
        let conversation_id = msg.conversation_id.to_string();
        let role = msg.role.to_string();

        self.ensure_conversation_agent(msg.conversation_id).await?;

        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;

        let parent_id: Option<String> = match parent {
            None => {
                sqlx::query_scalar(
                    "SELECT active_leaf_id FROM conversations WHERE id = ?1 AND agent_id = ?2",
                )
                .bind(&conversation_id)
                .bind(&self.agent_id)
                .fetch_one(&mut *tx)
                .await?
            }
            Some(explicit) => {
                if let Some(parent) = explicit {
                    let in_conversation: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM messages WHERE id = ?1 AND conversation_id = ?2)",
                    )
                    .bind(parent.to_string())
                    .bind(&conversation_id)
                    .fetch_one(&mut *tx)
                    .await?;
                    if !in_conversation {
                        anyhow::bail!(
                            "parent message {parent} is not in conversation {}",
                            msg.conversation_id
                        );
                    }
                }
                explicit.map(|p| p.to_string())
            }
        };

        let insert = sqlx::query(
            "INSERT INTO messages \
                (id, conversation_id, role, content, skill_name, tool_calls_json, turn, created_at, sender_user_id, parent_id, run_id, provider_state_json, tool_call_id, tool_status, tool_data_json, traceparent, finish_reason, model) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18) \
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
        .bind(&parent_id)
        .bind(msg.run_id.map(|r| r.to_string()))
        .bind(&msg.provider_state_json)
        .bind(&msg.tool_call_id)
        .bind(&msg.tool_status)
        .bind(&msg.tool_data_json)
        .bind(&msg.traceparent)
        .bind(msg.finish_reason.map(MessageFinishReason::as_str))
        .bind(&msg.model)
        .execute(&mut *tx)
        .await?;
        let inserted = insert.rows_affected() > 0;

        if inserted {
            sqlx::query(
                "UPDATE conversations SET active_leaf_id = ?1, updated_at = ?2 \
                 WHERE id = ?3 AND agent_id = ?4",
            )
            .bind(&id)
            .bind(self.clock.now())
            .bind(&conversation_id)
            .bind(&self.agent_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        if inserted {
            self.emit_upserted(msg.conversation_id).await;
        }

        Ok(())
    }

    /// Set the active leaf to `message_id`, or to the leaf reached by
    /// descending through the newest child at each level when `descend`.
    /// Never touches `updated_at`.
    async fn point_leaf_at(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
        descend: bool,
    ) -> Result<Option<Uuid>> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        if !self
            .owns_message(&mut *tx, conversation_id, message_id)
            .await?
        {
            return Ok(None);
        }
        let leaf = if descend {
            let leaf: String = sqlx::query_scalar(
                "WITH RECURSIVE descent(id, depth) AS ( \
                     SELECT ?1, 0 \
                     UNION ALL \
                     SELECT c.id, d.depth + 1 \
                     FROM descent d \
                     INNER JOIN messages c ON c.id = ( \
                         SELECT m.id FROM messages m \
                         WHERE m.conversation_id = ?2 AND m.parent_id = d.id \
                         ORDER BY m.created_at DESC, m.rowid DESC LIMIT 1) \
                 ) \
                 SELECT id FROM descent ORDER BY depth DESC LIMIT 1",
            )
            .bind(message_id.to_string())
            .bind(conversation_id.to_string())
            .fetch_one(&mut *tx)
            .await?;
            Uuid::parse_str(&leaf)?
        } else {
            message_id
        };
        sqlx::query("UPDATE conversations SET active_leaf_id = ?1 WHERE id = ?2 AND agent_id = ?3")
            .bind(leaf.to_string())
            .bind(conversation_id.to_string())
            .bind(&self.agent_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Some(leaf))
    }

    /// Broadcast the conversation's current record, when a broadcaster is set.
    async fn emit_upserted(&self, conversation_id: Uuid) {
        if let Some(b) = &self.broadcaster
            && let Ok(Some(conv)) = self.get_conversation(conversation_id).await
        {
            b.emit(ConversationEvent::Upserted(conv));
        }
    }

    /// Whether `message_id` is a message of `conversation_id` that this
    /// store's agent owns.
    async fn owns_message(
        &self,
        executor: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
        conversation_id: Uuid,
        message_id: Uuid,
    ) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS( \
                SELECT 1 FROM messages m \
                INNER JOIN conversations c ON c.id = m.conversation_id \
                WHERE m.id = ?1 AND m.conversation_id = ?2 AND c.agent_id = ?3)",
        )
        .bind(message_id.to_string())
        .bind(conversation_id.to_string())
        .bind(&self.agent_id)
        .fetch_one(executor)
        .await?)
    }
}

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
            pinned_at: None,
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
                "SELECT id, title, title_locked, agent_id, user_id, created_at, updated_at, pinned_at \
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
                "SELECT id, title, title_locked, agent_id, user_id, created_at, updated_at, pinned_at \
                 FROM conversations \
                 WHERE id = ?1 AND agent_id = ?2",
            )
            .bind(&id_str)
            .bind(&self.agent_id)
            .fetch_optional(&self.pool)
            .await?
        };

        row.as_ref().map(conversation_from_row).transpose()
    }

    /// List all conversations, most-recently updated first.
    async fn list_conversations(&self) -> Result<Vec<ConversationRecord>> {
        let rows = if self.user_id.is_some() {
            sqlx::query(
                "SELECT id, title, title_locked, agent_id, user_id, created_at, updated_at, pinned_at \
                 FROM conversations \
                 WHERE agent_id = ?1 AND user_id = ?2 \
                 ORDER BY pinned_at IS NULL, pinned_at DESC, updated_at DESC",
            )
            .bind(&self.agent_id)
            .bind(&self.user_id)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, title, title_locked, agent_id, user_id, created_at, updated_at, pinned_at \
                 FROM conversations \
                 WHERE agent_id = ?1 \
                 ORDER BY pinned_at IS NULL, pinned_at DESC, updated_at DESC",
            )
            .bind(&self.agent_id)
            .fetch_all(&self.pool)
            .await?
        };

        rows.iter().map(conversation_from_row).collect()
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
                 SET title = ?1, title_locked = 1
                 WHERE id = ?2 AND agent_id = ?3",
        )
        .bind(title)
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

    async fn update_metadata(&self, id: Uuid, patch: ConversationPatch) -> Result<MetadataUpdate> {
        let id_str = id.to_string();
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .with_context(|| format!("failed to begin metadata update for conversation {id}"))?;

        let result = sqlx::query(
            "UPDATE conversations
                 SET title = COALESCE(?1, title),
                     title_locked = CASE WHEN ?1 IS NULL THEN title_locked ELSE 1 END,
                     pinned_at = CASE ?2
                         WHEN 1 THEN COALESCE(pinned_at, ?3)
                         WHEN 0 THEN NULL
                         ELSE pinned_at
                     END
                 WHERE id = ?4 AND agent_id = ?5 AND (?6 IS NULL OR user_id = ?6)",
        )
        .bind(&patch.title)
        .bind(patch.pinned)
        .bind(self.clock.now())
        .bind(&id_str)
        .bind(&self.agent_id)
        .bind(&self.user_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("failed to update metadata for conversation {id}"))?;
        tx.commit().await?;

        if result.rows_affected() == 0 {
            return Ok(MetadataUpdate::NotFound);
        }
        let Some(conv) = self.get_conversation(id).await? else {
            return Ok(MetadataUpdate::NotFound);
        };
        if let Some(b) = &self.broadcaster {
            b.emit(ConversationEvent::Upserted(conv.clone()));
        }
        Ok(MetadataUpdate::Updated(conv))
    }

    async fn set_generated_title(&self, id: Uuid, title: &str) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE conversations
                 SET title = ?1, title_locked = 1
                 WHERE id = ?2 AND agent_id = ?3 AND title_locked = 0",
        )
        .bind(title)
        .bind(id.to_string())
        .bind(&self.agent_id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("failed to set generated title for conversation {id}"))?;
        if result.rows_affected() == 0 {
            return Ok(false);
        }

        if let Some(b) = &self.broadcaster
            && let Ok(Some(conv)) = self.get_conversation(id).await
        {
            b.emit(ConversationEvent::Upserted(conv));
        }

        Ok(true)
    }

    /// Delete a conversation and all its messages (cascade).
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

    /// Persist a message, appending it at the conversation's active leaf.
    ///
    /// The leaf is read, the row inserted under it and the leaf moved in one
    /// `BEGIN IMMEDIATE` transaction, so concurrent writers cannot fork the
    /// chain. Re-saving an existing id inserts nothing and leaves the leaf
    /// where it is.
    async fn save_message(&self, msg: &Message) -> Result<()> {
        self.insert_message(msg, None).await
    }

    /// Load the active path of a conversation, root first.
    async fn load_history(&self, conversation_id: Uuid) -> Result<Vec<Message>> {
        let rows = sqlx::query(ACTIVE_PATH_SQL)
            .bind(conversation_id.to_string())
            .bind(&self.agent_id)
            .bind(-1_i64)
            .fetch_all(&self.pool)
            .await?;

        let mut messages = rows
            .iter()
            .map(message_from_row)
            .collect::<Result<Vec<_>>>()?;
        messages.reverse();
        Ok(messages)
    }

    /// Fetch a single message by its ID (agent-scoped via the conversation join).
    async fn get_message(&self, message_id: Uuid) -> Result<Option<Message>> {
        let row = sqlx::query(
            "SELECT m.id, m.conversation_id, m.role, m.content, m.skill_name, m.tool_calls_json, m.turn, m.created_at, m.sender_user_id, m.parent_id, m.run_id, m.provider_state_json, m.tool_call_id, m.tool_status, m.tool_data_json, m.traceparent, m.finish_reason, m.model \
             FROM messages m \
             INNER JOIN conversations c ON c.id = m.conversation_id \
             WHERE m.id = ?1 AND c.agent_id = ?2",
        )
        .bind(message_id.to_string())
        .bind(&self.agent_id)
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref().map(message_from_row).transpose()
    }

    /// Return the last `limit` messages of the active path, in chronological order.
    async fn last_messages(&self, conversation_id: Uuid, limit: i64) -> Result<Vec<Message>> {
        // The query walks leaf-first, so `LIMIT` keeps the tail of the path.
        let rows = sqlx::query(ACTIVE_PATH_SQL)
            .bind(conversation_id.to_string())
            .bind(&self.agent_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        let mut messages = rows
            .iter()
            .map(message_from_row)
            .collect::<Result<Vec<_>>>()?;
        messages.reverse();
        Ok(messages)
    }

    async fn branch_message(&self, msg: &Message, parent_id: Option<Uuid>) -> Result<()> {
        self.insert_message(msg, Some(parent_id)).await
    }

    async fn children_of(
        &self,
        conversation_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> Result<Vec<Message>> {
        let rows = sqlx::query(
            "SELECT m.id, m.conversation_id, m.role, m.content, m.skill_name, m.tool_calls_json, m.turn, m.created_at, m.sender_user_id, m.parent_id, m.run_id, m.provider_state_json, m.tool_call_id, m.tool_status, m.tool_data_json, m.traceparent, m.finish_reason, m.model \
             FROM messages m \
             INNER JOIN conversations c ON c.id = m.conversation_id \
             WHERE m.conversation_id = ?1 AND c.agent_id = ?2 AND m.parent_id IS ?3 \
             ORDER BY m.created_at ASC, m.rowid ASC",
        )
        .bind(conversation_id.to_string())
        .bind(&self.agent_id)
        .bind(parent_id.map(|p| p.to_string()))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(message_from_row).collect()
    }

    async fn list_siblings(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
    ) -> Result<Option<Vec<Message>>> {
        if !self
            .owns_message(&self.pool, conversation_id, message_id)
            .await?
        {
            return Ok(None);
        }
        let parent: Option<String> =
            sqlx::query_scalar("SELECT parent_id FROM messages WHERE id = ?1")
                .bind(message_id.to_string())
                .fetch_one(&self.pool)
                .await?;
        let parent = parent.as_deref().map(Uuid::parse_str).transpose()?;
        Ok(Some(self.children_of(conversation_id, parent).await?))
    }

    async fn ancestors(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
    ) -> Result<Option<Vec<Message>>> {
        if !self
            .owns_message(&self.pool, conversation_id, message_id)
            .await?
        {
            return Ok(None);
        }
        let rows = sqlx::query(
            "WITH RECURSIVE chain(id, parent_id, depth) AS ( \
                 SELECT id, parent_id, 0 FROM messages WHERE id = ?1 \
                 UNION ALL \
                 SELECT m.id, m.parent_id, ch.depth + 1 \
                 FROM chain ch INNER JOIN messages m ON m.id = ch.parent_id \
             ) \
             SELECT m.id, m.conversation_id, m.role, m.content, m.skill_name, m.tool_calls_json, m.turn, m.created_at, m.sender_user_id, m.parent_id, m.run_id, m.provider_state_json, m.tool_call_id, m.tool_status, m.tool_data_json, m.traceparent, m.finish_reason, m.model \
             FROM chain INNER JOIN messages m ON m.id = chain.id \
             ORDER BY chain.depth DESC",
        )
        .bind(message_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        Ok(Some(
            rows.iter().map(message_from_row).collect::<Result<_>>()?,
        ))
    }

    async fn switch_to(&self, conversation_id: Uuid, message_id: Uuid) -> Result<Option<Uuid>> {
        let leaf = self
            .point_leaf_at(conversation_id, message_id, true)
            .await?;
        if leaf.is_some() {
            self.emit_upserted(conversation_id).await;
        }
        Ok(leaf)
    }

    async fn set_leaf(&self, conversation_id: Uuid, message_id: Uuid) -> Result<Option<Uuid>> {
        self.point_leaf_at(conversation_id, message_id, false).await
    }

    async fn delete_run_rows(
        &self,
        conversation_id: Uuid,
        run_id: Uuid,
        keep: Option<Uuid>,
    ) -> Result<u64> {
        self.ensure_conversation_agent(conversation_id).await?;
        let result = sqlx::query(
            "DELETE FROM messages \
             WHERE conversation_id = ?1 AND run_id = ?2 AND (?3 IS NULL OR id <> ?3)",
        )
        .bind(conversation_id.to_string())
        .bind(run_id.to_string())
        .bind(keep.map(|k| k.to_string()))
        .execute(&self.pool)
        .await
        .with_context(|| format!("deleting rows of run {run_id}"))?;
        Ok(result.rows_affected())
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
    state: Arc<Mutex<MemoryState>>,
    agent_id: String,
    user_id: Option<String>,
    broadcaster: Option<Arc<dyn ConversationBroadcast>>,
    clock: Arc<dyn Clock>,
}

#[derive(Default)]
struct MemoryState {
    conversations: HashMap<Uuid, ConversationRecord>,
    messages: HashMap<Uuid, Vec<Message>>,
    /// Active leaf per conversation (the tip of the active path).
    leaves: HashMap<Uuid, Uuid>,
}

impl InMemoryConversationStore {
    /// Create a new empty store with the default agent (`"default"`).
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MemoryState::default())),
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
    fn lock_state(&self) -> std::sync::MutexGuard<'_, MemoryState> {
        match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl InMemoryConversationStore {
    /// Insert `msg` and make it the active leaf. `parent` is `None` to append
    /// under the current leaf, or `Some(explicit_parent)` to branch.
    fn insert_message(&self, msg: &Message, parent: Option<Option<Uuid>>) -> Result<()> {
        let mut state = self.lock_state();
        let rows = state.messages.get(&msg.conversation_id);
        let parent_id = match parent {
            None => state.leaves.get(&msg.conversation_id).copied(),
            Some(explicit) => {
                if let Some(p) = explicit
                    && !rows.is_some_and(|rows| rows.iter().any(|m| m.id == p))
                {
                    anyhow::bail!(
                        "parent message {p} is not in conversation {}",
                        msg.conversation_id
                    );
                }
                explicit
            }
        };
        if rows.is_some_and(|rows| rows.iter().any(|m| m.id == msg.id)) {
            return Ok(());
        }
        let mut stored = msg.clone();
        stored.parent_id = parent_id;
        state
            .messages
            .entry(msg.conversation_id)
            .or_default()
            .push(stored);
        state.leaves.insert(msg.conversation_id, msg.id);
        if let Some(conv) = state.conversations.get_mut(&msg.conversation_id) {
            conv.updated_at = self.clock.now();
        }
        Ok(())
    }

    /// Set the active leaf to `message_id`, descending through the newest
    /// child at each level when `descend`. Never touches `updated_at`.
    fn point_leaf_at(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
        descend: bool,
    ) -> Option<Uuid> {
        let mut state = self.lock_state();
        let rows = state.messages.get(&conversation_id)?;
        rows.iter().find(|m| m.id == message_id)?;
        let mut leaf = message_id;
        while descend && let Some(child) = rows.iter().rev().find(|m| m.parent_id == Some(leaf)) {
            leaf = child.id;
        }
        state.leaves.insert(conversation_id, leaf);
        Some(leaf)
    }

    fn emit_upserted(&self, conversation_id: Uuid) {
        let record = self
            .lock_state()
            .conversations
            .get(&conversation_id)
            .cloned();
        if let (Some(b), Some(rec)) = (&self.broadcaster, record) {
            b.emit(ConversationEvent::Upserted(rec));
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
                    pinned_at: None,
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
        out.sort_by_key(|c| {
            (
                c.pinned_at.is_none(),
                std::cmp::Reverse((c.pinned_at, c.updated_at)),
            )
        });
        Ok(out)
    }

    async fn update_title(&self, id: Uuid, title: &str) -> Result<()> {
        let updated = {
            let mut state = self.lock_state();
            let conv = state.conversations.get_mut(&id);
            match conv {
                Some(c) if c.agent_id == self.agent_id => {
                    c.title = Some(title.to_string());
                    c.title_locked = true;
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

    async fn update_metadata(&self, id: Uuid, patch: ConversationPatch) -> Result<MetadataUpdate> {
        let now = self.clock.now();
        let updated = {
            let mut state = self.lock_state();
            state
                .conversations
                .get_mut(&id)
                .filter(|c| c.agent_id == self.agent_id)
                .filter(|c| {
                    self.user_id
                        .as_ref()
                        .is_none_or(|u| c.user_id.as_ref() == Some(u))
                })
                .map(|c| {
                    if let Some(title) = patch.title {
                        c.title = Some(title);
                        c.title_locked = true;
                    }
                    match patch.pinned {
                        Some(true) => c.pinned_at = c.pinned_at.or(Some(now)),
                        Some(false) => c.pinned_at = None,
                        None => {}
                    }
                    c.clone()
                })
        };
        let Some(rec) = updated else {
            return Ok(MetadataUpdate::NotFound);
        };
        if let Some(b) = &self.broadcaster {
            b.emit(ConversationEvent::Upserted(rec.clone()));
        }
        Ok(MetadataUpdate::Updated(rec))
    }

    async fn set_generated_title(&self, id: Uuid, title: &str) -> Result<bool> {
        let updated = {
            let mut state = self.lock_state();
            match state.conversations.get_mut(&id) {
                Some(c) if c.agent_id == self.agent_id && !c.title_locked => {
                    c.title = Some(title.to_string());
                    c.title_locked = true;
                    Some(c.clone())
                }
                _ => None,
            }
        };
        let Some(rec) = updated else {
            return Ok(false);
        };
        if let Some(b) = &self.broadcaster {
            b.emit(ConversationEvent::Upserted(rec));
        }
        Ok(true)
    }

    async fn delete_conversation(&self, id: Uuid) -> Result<()> {
        let mut state = self.lock_state();
        state.conversations.remove(&id);
        state.messages.remove(&id);
        state.leaves.remove(&id);
        if let Some(b) = &self.broadcaster {
            b.emit(ConversationEvent::Deleted {
                conversation_id: id,
                agent_id: self.agent_id.clone(),
            });
        }
        Ok(())
    }

    async fn save_message(&self, msg: &Message) -> Result<()> {
        self.insert_message(msg, None)
    }

    async fn load_history(&self, conversation_id: Uuid) -> Result<Vec<Message>> {
        let state = self.lock_state();
        Ok(active_path(&state, conversation_id))
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
        let mut out = active_path(&state, conversation_id);
        if limit > 0 {
            let limit = limit as usize;
            if out.len() > limit {
                let start = out.len() - limit;
                out = out.split_off(start);
            }
        }
        Ok(out)
    }

    async fn branch_message(&self, msg: &Message, parent_id: Option<Uuid>) -> Result<()> {
        self.insert_message(msg, Some(parent_id))
    }

    async fn children_of(
        &self,
        conversation_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> Result<Vec<Message>> {
        let state = self.lock_state();
        Ok(children(&state, conversation_id, parent_id))
    }

    async fn list_siblings(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
    ) -> Result<Option<Vec<Message>>> {
        let state = self.lock_state();
        let Some(msg) = find_message(&state, conversation_id, message_id) else {
            return Ok(None);
        };
        Ok(Some(children(&state, conversation_id, msg.parent_id)))
    }

    async fn ancestors(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
    ) -> Result<Option<Vec<Message>>> {
        let state = self.lock_state();
        let Some(start) = find_message(&state, conversation_id, message_id) else {
            return Ok(None);
        };
        let mut chain = vec![start.clone()];
        let mut cursor = start.parent_id;
        while let Some(id) = cursor {
            let Some(parent) = find_message(&state, conversation_id, id) else {
                break;
            };
            chain.push(parent.clone());
            cursor = parent.parent_id;
        }
        chain.reverse();
        Ok(Some(chain))
    }

    async fn switch_to(&self, conversation_id: Uuid, message_id: Uuid) -> Result<Option<Uuid>> {
        let leaf = self.point_leaf_at(conversation_id, message_id, true);
        if leaf.is_some() {
            self.emit_upserted(conversation_id);
        }
        Ok(leaf)
    }

    async fn set_leaf(&self, conversation_id: Uuid, message_id: Uuid) -> Result<Option<Uuid>> {
        Ok(self.point_leaf_at(conversation_id, message_id, false))
    }

    async fn delete_run_rows(
        &self,
        conversation_id: Uuid,
        run_id: Uuid,
        keep: Option<Uuid>,
    ) -> Result<u64> {
        let mut state = self.lock_state();
        let Some(rows) = state.messages.get_mut(&conversation_id) else {
            return Ok(0);
        };
        let doomed: Vec<Uuid> = rows
            .iter()
            .filter(|m| m.run_id == Some(run_id) && Some(m.id) != keep)
            .map(|m| m.id)
            .collect();
        if let Some(orphan) = rows
            .iter()
            .find(|m| !doomed.contains(&m.id) && m.parent_id.is_some_and(|p| doomed.contains(&p)))
        {
            anyhow::bail!(
                "message {} still hangs under a row of run {run_id}",
                orphan.id
            );
        }
        rows.retain(|m| !doomed.contains(&m.id));
        Ok(doomed.len() as u64)
    }
}

fn find_message(state: &MemoryState, conversation_id: Uuid, id: Uuid) -> Option<&Message> {
    state
        .messages
        .get(&conversation_id)?
        .iter()
        .find(|m| m.id == id)
}

/// The children of `parent` (the roots for `None`) in insertion order.
fn children(state: &MemoryState, conversation_id: Uuid, parent: Option<Uuid>) -> Vec<Message> {
    state
        .messages
        .get(&conversation_id)
        .map(|rows| {
            rows.iter()
                .filter(|m| m.parent_id == parent)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Walk from the conversation's active leaf up the parent chain, root first.
fn active_path(state: &MemoryState, conversation_id: Uuid) -> Vec<Message> {
    let Some(rows) = state.messages.get(&conversation_id) else {
        return Vec::new();
    };
    let by_id: HashMap<Uuid, &Message> = rows.iter().map(|m| (m.id, m)).collect();
    let mut path = Vec::new();
    let mut cursor = state.leaves.get(&conversation_id).copied();
    while let Some(id) = cursor {
        let Some(msg) = by_id.get(&id) else { break };
        path.push((*msg).clone());
        cursor = msg.parent_id;
    }
    path.reverse();
    path
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The active path of a conversation, newest message first: a recursive walk
/// from `conversations.active_leaf_id` up the `parent_id` chain.
/// Binds: `?1` conversation id, `?2` agent id, `?3` row limit (`-1` for all).
const ACTIVE_PATH_SQL: &str = "\
    WITH RECURSIVE path(id, parent_id, depth) AS ( \
        SELECT m.id, m.parent_id, 0 \
        FROM conversations c \
        INNER JOIN messages m ON m.id = c.active_leaf_id \
        WHERE c.id = ?1 AND c.agent_id = ?2 \
        UNION ALL \
        SELECT m.id, m.parent_id, p.depth + 1 \
        FROM path p \
        INNER JOIN messages m ON m.id = p.parent_id \
    ) \
    SELECT m.id, m.conversation_id, m.role, m.content, m.skill_name, m.tool_calls_json, m.turn, m.created_at, m.sender_user_id, m.parent_id, m.run_id, m.provider_state_json, m.tool_call_id, m.tool_status, m.tool_data_json, m.traceparent, m.finish_reason, m.model \
    FROM path \
    INNER JOIN messages m ON m.id = path.id \
    ORDER BY path.depth ASC \
    LIMIT ?3";

fn message_from_row(r: &sqlx::sqlite::SqliteRow) -> Result<Message> {
    let id_str: String = r.get("id");
    let conv_str: String = r.get("conversation_id");
    let role_str: String = r.get("role");
    let parent_str: Option<String> = r.get("parent_id");
    let run_str: Option<String> = r.get("run_id");
    let finish_str: Option<String> = r.get("finish_reason");
    Ok(Message {
        id: Uuid::parse_str(&id_str)?,
        conversation_id: Uuid::parse_str(&conv_str)?,
        role: parse_role(&role_str)?,
        content: r.get("content"),
        skill_name: r.get("skill_name"),
        tool_calls_json: r.get("tool_calls_json"),
        provider_state_json: r.get("provider_state_json"),
        turn: r.get("turn"),
        created_at: r.get("created_at"),
        sender_user_id: r.get("sender_user_id"),
        parent_id: parent_str.as_deref().map(Uuid::parse_str).transpose()?,
        run_id: run_str.as_deref().map(Uuid::parse_str).transpose()?,
        tool_call_id: r.get("tool_call_id"),
        tool_status: r.get("tool_status"),
        tool_data_json: r.get("tool_data_json"),
        traceparent: r.get("traceparent"),
        model: r.get("model"),
        finish_reason: finish_str
            .as_deref()
            .and_then(MessageFinishReason::from_stored),
    })
}

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

    #[tokio::test]
    async fn test_update_metadata_emits_one_upserted_event_with_pin() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let broadcaster = Arc::new(InMemoryConversationBroadcaster::new());
        let store = storage
            .conversation_store()
            .with_broadcaster(broadcaster.clone());
        let conv = store.create_conversation(None).await.unwrap();

        let mut rx = broadcaster.subscribe();
        let patch = ConversationPatch {
            title: Some("Pinned".to_string()),
            pinned: Some(true),
        };
        store.update_metadata(conv.id, patch).await.unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("timed out waiting for conversation event")
            .expect("should receive event");
        match event {
            ConversationEvent::Upserted(record) => {
                assert_eq!(record.title.as_deref(), Some("Pinned"));
                assert!(record.pinned_at.is_some(), "event must carry pinned_at");
            }
            other => panic!("expected Upserted event, got {other:?}"),
        }
        assert!(
            rx.try_recv().is_err(),
            "one patch must emit exactly one event"
        );
    }

    #[tokio::test]
    async fn test_update_metadata_is_scoped_to_user() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let alice = storage.conversation_store().with_user_id("alice");
        let bob = storage.conversation_store().with_user_id("bob");
        let conv = alice.create_conversation(Some("Mine")).await.unwrap();

        let patch = ConversationPatch {
            title: None,
            pinned: Some(true),
        };
        let res = bob.update_metadata(conv.id, patch).await.unwrap();
        assert!(
            matches!(res, MetadataUpdate::NotFound),
            "another user's conversation must not be patched"
        );
        let loaded = alice.get_conversation(conv.id).await.unwrap().unwrap();
        assert!(loaded.pinned_at.is_none());
    }

    /// Assert the receiver holds no event, i.e. the last write was silent.
    fn assert_no_event(rx: &mut tokio::sync::broadcast::Receiver<ConversationEvent>) {
        assert!(
            matches!(
                rx.try_recv(),
                Err(tokio::sync::broadcast::error::TryRecvError::Empty)
            ),
            "no event expected"
        );
    }

    async fn generated_title_event_contract(
        store: &dyn ConversationStore,
        rx: &mut tokio::sync::broadcast::Receiver<ConversationEvent>,
    ) {
        let conv = store.create_conversation(None).await.unwrap();
        // Drain the creation event.
        let _ = rx.try_recv();

        assert!(
            store
                .set_generated_title(conv.id, "Generated")
                .await
                .unwrap()
        );
        match rx.try_recv() {
            Ok(ConversationEvent::Upserted(record)) => {
                assert_eq!(record.id, conv.id, "event must carry the conversation id");
                assert_eq!(record.title.as_deref(), Some("Generated"));
                assert!(record.title_locked, "event must carry title_locked = true");
            }
            other => panic!("expected Upserted event, got {other:?}"),
        }

        // The row is now locked: a second generated write is a silent no-op.
        assert!(!store.set_generated_title(conv.id, "Again").await.unwrap());
        assert_no_event(rx);
    }

    #[tokio::test]
    async fn test_set_generated_title_emits_upserted_only_when_written_sqlite() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let broadcaster = Arc::new(InMemoryConversationBroadcaster::new());
        let store = storage
            .conversation_store()
            .with_broadcaster(broadcaster.clone());
        let mut rx = broadcaster.subscribe();
        generated_title_event_contract(&store, &mut rx).await;
    }

    #[tokio::test]
    async fn test_set_generated_title_emits_upserted_only_when_written_in_memory() {
        let broadcaster = Arc::new(InMemoryConversationBroadcaster::new());
        let store = InMemoryConversationStore::new().with_broadcaster(broadcaster.clone());
        let mut rx = broadcaster.subscribe();
        generated_title_event_contract(&store, &mut rx).await;
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
    async fn test_message_provider_state_round_trips() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.conversation_store();
        let conv = store.create_conversation(None).await.unwrap();

        let mut with_state = Message::assistant(conv.id, "");
        with_state.turn = 1;
        with_state.tool_calls_json = Some(r#"[{"name":"bash","params":{}}]"#.into());
        with_state.provider_state_json =
            Some(r#"{"provider":"openrouter","model":"m","data":{"reasoning_details":[]}}"#.into());
        store.save_message(&with_state).await.unwrap();

        let mut without = Message::assistant(conv.id, "done");
        without.turn = 2;
        store.save_message(&without).await.unwrap();

        let history = store.load_history(conv.id).await.unwrap();
        assert_eq!(
            history[0].provider_state_json, with_state.provider_state_json,
            "provider state should round-trip unchanged"
        );
        assert!(
            history[1].provider_state_json.is_none(),
            "messages without provider state read back as None"
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

    #[tokio::test]
    async fn test_switch_to_emits_upserted_event() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let broadcaster = Arc::new(InMemoryConversationBroadcaster::new());
        let store = storage
            .conversation_store()
            .with_broadcaster(broadcaster.clone());
        let conv = store.create_conversation(None).await.unwrap();
        let a = Message::user(conv.id, "a");
        store.save_message(&a).await.unwrap();
        store
            .branch_message(&Message::assistant(conv.id, "b"), Some(a.id))
            .await
            .unwrap();

        let mut rx = broadcaster.subscribe();
        store.switch_to(conv.id, a.id).await.unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("timed out waiting for conversation event")
            .expect("should receive event");
        match event {
            ConversationEvent::Upserted(record) => assert_eq!(record.id, conv.id),
            _ => panic!("expected Upserted event"),
        }
        assert_no_event(&mut rx);

        store.set_leaf(conv.id, a.id).await.unwrap();
        assert_no_event(&mut rx);
    }
}
