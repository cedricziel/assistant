//! Persistent attachment storage.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use assistant_core::{AttachmentMeta, agent_base_dir};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Trait-based interface for attachment persistence.
#[async_trait]
pub trait AttachmentStore: Send + Sync {
    /// Persist attachment metadata and bytes.
    async fn store(&self, meta: &AttachmentMeta, data: &[u8]) -> Result<()>;

    /// Load attachment bytes by ID.
    async fn load_bytes(&self, id: Uuid) -> Result<Vec<u8>>;

    /// Fetch attachment metadata by ID.
    async fn get_meta(&self, id: Uuid) -> Result<AttachmentMeta>;

    /// List attachments linked to a message.
    async fn list_for_message(&self, message_id: Uuid) -> Result<Vec<AttachmentMeta>>;

    /// List attachments for a conversation.
    async fn list_for_conversation(&self, conversation_id: Uuid) -> Result<Vec<AttachmentMeta>>;

    /// Link an existing attachment to a message.
    async fn link_to_message(&self, attachment_id: Uuid, message_id: Uuid) -> Result<()>;
}

/// SQLite + filesystem attachment store.
#[derive(Clone)]
pub struct SqliteAttachmentStore {
    pool: SqlitePool,
}

impl SqliteAttachmentStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AttachmentStore for SqliteAttachmentStore {
    /// Persist an attachment: write bytes to disk and insert a metadata row.
    async fn store(&self, meta: &AttachmentMeta, data: &[u8]) -> Result<()> {
        let path = attachment_path(meta);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating attachment directory {}", parent.display()))?;
        }
        tokio::fs::write(&path, data)
            .await
            .with_context(|| format!("writing attachment to {}", path.display()))?;

        sqlx::query(
            "INSERT INTO message_attachments (id, message_id, conversation_id, agent_id, filename, mime_type, size_bytes, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(meta.id.to_string())
        .bind(meta.message_id.map(|id| id.to_string()))
        .bind(meta.conversation_id.to_string())
        .bind(&meta.agent_id)
        .bind(&meta.filename)
        .bind(&meta.mime_type)
        .bind(meta.size_bytes as i64)
        .bind(meta.created_at)
        .execute(&self.pool)
        .await
        .context("inserting attachment metadata")?;

        Ok(())
    }

    /// Read raw bytes from disk for the given attachment.
    async fn load_bytes(&self, id: Uuid) -> Result<Vec<u8>> {
        let meta = self.get_meta(id).await?;
        let path = attachment_path(&meta);
        tokio::fs::read(&path)
            .await
            .with_context(|| format!("reading attachment from {}", path.display()))
    }

    /// Look up attachment metadata by ID.
    async fn get_meta(&self, id: Uuid) -> Result<AttachmentMeta> {
        let row = sqlx::query(
            "SELECT id, message_id, conversation_id, agent_id, filename, mime_type, size_bytes, created_at
             FROM message_attachments WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .context("querying attachment metadata")?
        .with_context(|| format!("attachment {id} not found"))?;

        row_to_meta(&row)
    }

    /// List attachments linked to a message.
    async fn list_for_message(&self, message_id: Uuid) -> Result<Vec<AttachmentMeta>> {
        let rows = sqlx::query(
            "SELECT id, message_id, conversation_id, agent_id, filename, mime_type, size_bytes, created_at
             FROM message_attachments WHERE message_id = ? ORDER BY created_at",
        )
        .bind(message_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("listing attachments for message")?;

        rows.iter().map(row_to_meta).collect()
    }

    /// List attachments in a conversation.
    async fn list_for_conversation(&self, conversation_id: Uuid) -> Result<Vec<AttachmentMeta>> {
        let rows = sqlx::query(
            "SELECT id, message_id, conversation_id, agent_id, filename, mime_type, size_bytes, created_at
             FROM message_attachments WHERE conversation_id = ? ORDER BY created_at",
        )
        .bind(conversation_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("listing attachments for conversation")?;

        rows.iter().map(row_to_meta).collect()
    }

    /// Link an unlinked attachment to a message (sets `message_id`).
    async fn link_to_message(&self, attachment_id: Uuid, message_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE message_attachments SET message_id = ? WHERE id = ?")
            .bind(message_id.to_string())
            .bind(attachment_id.to_string())
            .execute(&self.pool)
            .await
            .context("linking attachment to message")?;
        Ok(())
    }
}

// -- Path helpers -------------------------------------------------------------

/// Resolve the filesystem path for an attachment's bytes.
pub fn attachment_path(meta: &AttachmentMeta) -> PathBuf {
    agent_base_dir(&meta.agent_id)
        .join("attachments")
        .join(meta.conversation_id.to_string())
        .join(format!("{}.{}", meta.id, meta.extension()))
}

// -- Row conversion -----------------------------------------------------------

fn row_to_meta(row: &sqlx::sqlite::SqliteRow) -> Result<AttachmentMeta> {
    let id_str: String = row.try_get("id").context("missing id")?;
    let message_id_str: Option<String> = row.try_get("message_id").context("missing message_id")?;
    let conv_id_str: String = row
        .try_get("conversation_id")
        .context("missing conversation_id")?;
    let created_at: DateTime<Utc> = row.try_get("created_at").context("missing created_at")?;

    Ok(AttachmentMeta {
        id: Uuid::parse_str(&id_str).context("invalid attachment id")?,
        message_id: message_id_str
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .context("invalid message_id")?,
        conversation_id: Uuid::parse_str(&conv_id_str).context("invalid conversation_id")?,
        agent_id: row.try_get("agent_id").context("missing agent_id")?,
        filename: row.try_get("filename").context("missing filename")?,
        mime_type: row.try_get("mime_type").context("missing mime_type")?,
        size_bytes: row
            .try_get::<i64, _>("size_bytes")
            .context("missing size_bytes")? as u64,
        created_at,
    })
}

// ---------------------------------------------------------------------------
// In-memory implementation (test-only — see workspace_test_impls_in_prod.rs)
// ---------------------------------------------------------------------------

/// In-memory [`AttachmentStore`] for tests. Stores bytes in a HashMap
/// keyed by ID; metadata in a separate HashMap. No filesystem.
pub struct InMemoryAttachmentStore {
    state: Arc<Mutex<InMemoryAttachmentState>>,
}

#[derive(Default)]
struct InMemoryAttachmentState {
    bytes: HashMap<Uuid, Vec<u8>>,
    metas: HashMap<Uuid, AttachmentMeta>,
}

impl InMemoryAttachmentStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemoryAttachmentState::default())),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, InMemoryAttachmentState> {
        match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl Default for InMemoryAttachmentStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AttachmentStore for InMemoryAttachmentStore {
    async fn store(&self, meta: &AttachmentMeta, data: &[u8]) -> Result<()> {
        let mut state = self.lock();
        state.bytes.insert(meta.id, data.to_vec());
        state.metas.insert(meta.id, meta.clone());
        Ok(())
    }

    async fn load_bytes(&self, id: Uuid) -> Result<Vec<u8>> {
        let state = self.lock();
        state
            .bytes
            .get(&id)
            .cloned()
            .with_context(|| format!("attachment {id} bytes not found"))
    }

    async fn get_meta(&self, id: Uuid) -> Result<AttachmentMeta> {
        let state = self.lock();
        state
            .metas
            .get(&id)
            .cloned()
            .with_context(|| format!("attachment {id} not found"))
    }

    async fn list_for_message(&self, message_id: Uuid) -> Result<Vec<AttachmentMeta>> {
        let state = self.lock();
        let mut out: Vec<AttachmentMeta> = state
            .metas
            .values()
            .filter(|m| m.message_id == Some(message_id))
            .cloned()
            .collect();
        out.sort_by_key(|m| m.created_at);
        Ok(out)
    }

    async fn list_for_conversation(&self, conversation_id: Uuid) -> Result<Vec<AttachmentMeta>> {
        let state = self.lock();
        let mut out: Vec<AttachmentMeta> = state
            .metas
            .values()
            .filter(|m| m.conversation_id == conversation_id)
            .cloned()
            .collect();
        out.sort_by_key(|m| m.created_at);
        Ok(out)
    }

    async fn link_to_message(&self, attachment_id: Uuid, message_id: Uuid) -> Result<()> {
        let mut state = self.lock();
        if let Some(m) = state.metas.get_mut(&attachment_id) {
            m.message_id = Some(message_id);
        }
        Ok(())
    }
}

// -- Tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    async fn setup() -> (StorageLayer, SqliteAttachmentStore) {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.attachment_store();
        (storage, store)
    }

    fn make_meta(conv_id: Uuid, agent_id: &str) -> AttachmentMeta {
        AttachmentMeta {
            id: Uuid::new_v4(),
            message_id: None,
            conversation_id: conv_id,
            agent_id: agent_id.to_string(),
            filename: "test.png".to_string(),
            mime_type: "image/png".to_string(),
            size_bytes: 4,
            created_at: Utc::now(),
        }
    }

    /// Create a conversation + message row so FK constraints pass.
    async fn seed_conv_and_msg(store: &SqliteAttachmentStore, conv_id: Uuid, msg_id: Uuid) {
        sqlx::query("INSERT INTO conversations (id) VALUES (?)")
            .bind(conv_id.to_string())
            .execute(&store.pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO messages (id, conversation_id, role, content, turn) VALUES (?, ?, 'user', '', 0)",
        )
        .bind(msg_id.to_string())
        .bind(conv_id.to_string())
        .execute(&store.pool)
        .await
        .unwrap();
    }

    async fn insert_meta(store: &SqliteAttachmentStore, meta: &AttachmentMeta) {
        sqlx::query(
            "INSERT INTO message_attachments (id, message_id, conversation_id, agent_id, filename, mime_type, size_bytes, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(meta.id.to_string())
        .bind(meta.message_id.map(|id| id.to_string()))
        .bind(meta.conversation_id.to_string())
        .bind(&meta.agent_id)
        .bind(&meta.filename)
        .bind(&meta.mime_type)
        .bind(meta.size_bytes as i64)
        .bind(meta.created_at)
        .execute(&store.pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn store_and_get_meta() {
        let (_storage, store) = setup().await;
        let meta = make_meta(Uuid::new_v4(), "default");

        insert_meta(&store, &meta).await;

        let loaded = store.get_meta(meta.id).await.unwrap();
        assert_eq!(loaded.id, meta.id);
        assert_eq!(loaded.filename, "test.png");
        assert_eq!(loaded.mime_type, "image/png");
        assert!(loaded.message_id.is_none());
    }

    #[tokio::test]
    async fn link_to_message() {
        let (_storage, store) = setup().await;
        let conv_id = Uuid::new_v4();
        let msg_id = Uuid::new_v4();
        seed_conv_and_msg(&store, conv_id, msg_id).await;

        let meta = make_meta(conv_id, "default");
        insert_meta(&store, &meta).await;

        store.link_to_message(meta.id, msg_id).await.unwrap();

        let loaded = store.get_meta(meta.id).await.unwrap();
        assert_eq!(loaded.message_id, Some(msg_id));
    }

    #[tokio::test]
    async fn list_for_message_returns_linked() {
        let (_storage, store) = setup().await;
        let conv_id = Uuid::new_v4();
        let msg_id = Uuid::new_v4();
        seed_conv_and_msg(&store, conv_id, msg_id).await;

        for i in 0..3 {
            let mut meta = make_meta(conv_id, "default");
            meta.filename = format!("img{i}.png");
            meta.message_id = Some(msg_id);
            insert_meta(&store, &meta).await;
        }

        let list = store.list_for_message(msg_id).await.unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn list_for_conversation_returns_all() {
        let (_storage, store) = setup().await;
        let conv_id = Uuid::new_v4();

        for _ in 0..2 {
            let meta = make_meta(conv_id, "default");
            insert_meta(&store, &meta).await;
        }

        let list = store.list_for_conversation(conv_id).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn get_meta_not_found() {
        let (_storage, store) = setup().await;
        let result = store.get_meta(Uuid::new_v4()).await;
        assert!(result.is_err());
    }
}
