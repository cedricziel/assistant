//! Generative UI surfaces backed by the `ui_surfaces` table.
//!
//! A surface is one A2UI document the model rendered with `render-surface`.
//! The row anchors to the tool-call message that created it, holds the
//! current components and data model, and records the action that answered
//! it. See `openspec/changes/generative-ui-a2ui` (design D6 and D8).
//!
//! Status moves `active -> submitted` through a compare-and-set
//! ([`SurfaceStore::mark_submitted`]) and back through
//! [`SurfaceStore::reopen`]. Closure by a later user turn is derived by the
//! reader and is never stored.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::Value;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Where a surface is in its single-shot lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceStatus {
    /// The user can still answer it.
    Active,
    /// The user answered it; it is read-only now.
    Submitted,
}

impl SurfaceStatus {
    fn parse(s: &str) -> Result<Self> {
        match s {
            "active" => Ok(Self::Active),
            "submitted" => Ok(Self::Submitted),
            other => anyhow::bail!("unknown surface status '{other}'"),
        }
    }
}

/// A stored surface.
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub turn: i64,
    /// The tool-call message the surface hangs off.
    pub anchor_message_id: Uuid,
    /// Index of the creating call inside that message's `tool_calls`.
    pub anchor_call_index: i64,
    /// Provider tool-call id, when the provider supplies one.
    pub tool_call_id: Option<String>,
    pub title: String,
    pub fallback_text: String,
    pub catalog_id: String,
    pub protocol_version: String,
    pub components: Value,
    pub data_model: Value,
    /// Whether the data model travels back with the user's action.
    pub send_data_model: bool,
    pub status: SurfaceStatus,
    /// The action that submitted the surface.
    pub action: Option<Value>,
    /// The user message that carried the action.
    pub action_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Fields the caller supplies when creating an `active` surface.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSurface {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub turn: i64,
    pub anchor_message_id: Uuid,
    pub anchor_call_index: i64,
    pub tool_call_id: Option<String>,
    pub title: String,
    pub fallback_text: String,
    pub catalog_id: String,
    pub protocol_version: String,
    pub components: Value,
    pub data_model: Value,
    pub send_data_model: bool,
}

/// Replacement values for [`SurfaceStore::update`]; `None` keeps the stored value.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SurfaceChanges {
    pub title: Option<String>,
    pub fallback_text: Option<String>,
    pub components: Option<Value>,
    pub data_model: Option<Value>,
}

/// Result of a compare-and-set status change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    /// The surface changed status.
    Applied,
    /// The surface is missing or was not in the expected state.
    Conflict,
}

impl Transition {
    fn from_rows(rows_affected: u64) -> Self {
        if rows_affected == 1 {
            Self::Applied
        } else {
            Self::Conflict
        }
    }
}

/// Persistence for [`Surface`] rows.
#[async_trait]
pub trait SurfaceStore: Send + Sync {
    /// Insert a new `active` surface and return it.
    async fn create(&self, new: &NewSurface) -> Result<Surface>;

    /// Fetch a surface by id.
    async fn get(&self, id: Uuid) -> Result<Option<Surface>>;

    /// Apply `changes` and return the updated surface, or `None` if it does not exist.
    async fn update(&self, id: Uuid, changes: &SurfaceChanges) -> Result<Option<Surface>>;

    /// Compare-and-set `active -> submitted`, storing the action and the user
    /// message that carried it. A second call returns [`Transition::Conflict`].
    async fn mark_submitted(
        &self,
        id: Uuid,
        action: &Value,
        action_message_id: Uuid,
    ) -> Result<Transition>;

    /// Roll `submitted -> active` back and clear the action, only when the
    /// surface is still linked to `action_message_id`.
    async fn reopen(&self, id: Uuid, action_message_id: Uuid) -> Result<Transition>;

    /// Every surface of a conversation, ordered by `(turn, created_at)`.
    async fn list_for_conversation(&self, conversation_id: Uuid) -> Result<Vec<Surface>>;

    /// Submitted surfaces of a conversation keyed by the user message that
    /// carried their action.
    async fn action_message_map(&self, conversation_id: Uuid) -> Result<HashMap<Uuid, Surface>>;
}

// -- SQLite --

/// SQLite-backed [`SurfaceStore`].
pub struct SqliteSurfaceStore {
    pool: SqlitePool,
}

impl SqliteSurfaceStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn timestamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_uuid(s: &str, column: &str) -> Result<Uuid> {
    Uuid::parse_str(s).with_context(|| format!("invalid {column} in ui_surfaces"))
}

fn parse_time(s: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(s)?.with_timezone(&Utc))
}

fn surface_from_row(row: &SqliteRow) -> Result<Surface> {
    let action_json: Option<String> = row.get("action_json");
    let action_message_id: Option<String> = row.get("action_message_id");
    Ok(Surface {
        id: parse_uuid(&row.get::<String, _>("id"), "id")?,
        conversation_id: parse_uuid(&row.get::<String, _>("conversation_id"), "conversation_id")?,
        turn: row.get("turn"),
        anchor_message_id: parse_uuid(
            &row.get::<String, _>("anchor_message_id"),
            "anchor_message_id",
        )?,
        anchor_call_index: row.get("anchor_call_index"),
        tool_call_id: row.get("tool_call_id"),
        title: row.get("title"),
        fallback_text: row.get("fallback_text"),
        catalog_id: row.get("catalog_id"),
        protocol_version: row.get("protocol_version"),
        components: serde_json::from_str(&row.get::<String, _>("components_json"))
            .context("invalid components_json in ui_surfaces")?,
        data_model: serde_json::from_str(&row.get::<String, _>("data_model_json"))
            .context("invalid data_model_json in ui_surfaces")?,
        send_data_model: row.get::<i64, _>("send_data_model") != 0,
        status: SurfaceStatus::parse(&row.get::<String, _>("status"))?,
        action: action_json
            .map(|json| serde_json::from_str(&json))
            .transpose()
            .context("invalid action_json in ui_surfaces")?,
        action_message_id: action_message_id
            .map(|id| parse_uuid(&id, "action_message_id"))
            .transpose()?,
        created_at: parse_time(&row.get::<String, _>("created_at"))?,
        updated_at: parse_time(&row.get::<String, _>("updated_at"))?,
    })
}

#[async_trait]
impl SurfaceStore for SqliteSurfaceStore {
    async fn create(&self, new: &NewSurface) -> Result<Surface> {
        let now = timestamp(Utc::now());
        sqlx::query(
            "INSERT INTO ui_surfaces \
             (id, conversation_id, turn, anchor_message_id, anchor_call_index, tool_call_id, \
              title, fallback_text, catalog_id, protocol_version, components_json, \
              data_model_json, send_data_model, status, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'active', ?14, ?14)",
        )
        .bind(new.id.to_string())
        .bind(new.conversation_id.to_string())
        .bind(new.turn)
        .bind(new.anchor_message_id.to_string())
        .bind(new.anchor_call_index)
        .bind(&new.tool_call_id)
        .bind(&new.title)
        .bind(&new.fallback_text)
        .bind(&new.catalog_id)
        .bind(&new.protocol_version)
        .bind(serde_json::to_string(&new.components)?)
        .bind(serde_json::to_string(&new.data_model)?)
        .bind(i64::from(new.send_data_model))
        .bind(&now)
        .execute(&self.pool)
        .await?;
        self.get(new.id)
            .await?
            .context("surface missing right after insert")
    }

    async fn get(&self, id: Uuid) -> Result<Option<Surface>> {
        let row = sqlx::query("SELECT * FROM ui_surfaces WHERE id = ?1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(surface_from_row).transpose()
    }

    async fn update(&self, id: Uuid, changes: &SurfaceChanges) -> Result<Option<Surface>> {
        let components = changes
            .components
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let data_model = changes
            .data_model
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let result = sqlx::query(
            "UPDATE ui_surfaces SET \
               title = COALESCE(?2, title), \
               fallback_text = COALESCE(?3, fallback_text), \
               components_json = COALESCE(?4, components_json), \
               data_model_json = COALESCE(?5, data_model_json), \
               updated_at = ?6 \
             WHERE id = ?1",
        )
        .bind(id.to_string())
        .bind(&changes.title)
        .bind(&changes.fallback_text)
        .bind(components)
        .bind(data_model)
        .bind(timestamp(Utc::now()))
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }
        self.get(id).await
    }

    async fn mark_submitted(
        &self,
        id: Uuid,
        action: &Value,
        action_message_id: Uuid,
    ) -> Result<Transition> {
        let result = sqlx::query(
            "UPDATE ui_surfaces SET status = 'submitted', action_json = ?2, \
               action_message_id = ?3, updated_at = ?4 \
             WHERE id = ?1 AND status = 'active'",
        )
        .bind(id.to_string())
        .bind(serde_json::to_string(action)?)
        .bind(action_message_id.to_string())
        .bind(timestamp(Utc::now()))
        .execute(&self.pool)
        .await?;
        Ok(Transition::from_rows(result.rows_affected()))
    }

    async fn reopen(&self, id: Uuid, action_message_id: Uuid) -> Result<Transition> {
        let result = sqlx::query(
            "UPDATE ui_surfaces SET status = 'active', action_json = NULL, \
               action_message_id = NULL, updated_at = ?3 \
             WHERE id = ?1 AND status = 'submitted' AND action_message_id = ?2",
        )
        .bind(id.to_string())
        .bind(action_message_id.to_string())
        .bind(timestamp(Utc::now()))
        .execute(&self.pool)
        .await?;
        Ok(Transition::from_rows(result.rows_affected()))
    }

    async fn list_for_conversation(&self, conversation_id: Uuid) -> Result<Vec<Surface>> {
        let rows = sqlx::query(
            "SELECT * FROM ui_surfaces WHERE conversation_id = ?1 ORDER BY turn, created_at, rowid",
        )
        .bind(conversation_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(surface_from_row).collect()
    }

    async fn action_message_map(&self, conversation_id: Uuid) -> Result<HashMap<Uuid, Surface>> {
        let rows = sqlx::query(
            "SELECT * FROM ui_surfaces \
             WHERE conversation_id = ?1 AND action_message_id IS NOT NULL",
        )
        .bind(conversation_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let surface = surface_from_row(row)?;
                let message_id = surface
                    .action_message_id
                    .context("action_message_id vanished")?;
                Ok((message_id, surface))
            })
            .collect()
    }
}

// -- In memory --

/// In-memory [`SurfaceStore`] for tests.
///
/// It is separate from the in-memory conversation store, so it never cascades
/// on conversation deletion.
#[derive(Default)]
pub struct InMemorySurfaceStore {
    rows: Arc<Mutex<Vec<Surface>>>,
}

impl InMemorySurfaceStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Surface>> {
        match self.rows.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }
}

#[async_trait]
impl SurfaceStore for InMemorySurfaceStore {
    async fn create(&self, new: &NewSurface) -> Result<Surface> {
        let now = Utc::now();
        let surface = Surface {
            id: new.id,
            conversation_id: new.conversation_id,
            turn: new.turn,
            anchor_message_id: new.anchor_message_id,
            anchor_call_index: new.anchor_call_index,
            tool_call_id: new.tool_call_id.clone(),
            title: new.title.clone(),
            fallback_text: new.fallback_text.clone(),
            catalog_id: new.catalog_id.clone(),
            protocol_version: new.protocol_version.clone(),
            components: new.components.clone(),
            data_model: new.data_model.clone(),
            send_data_model: new.send_data_model,
            status: SurfaceStatus::Active,
            action: None,
            action_message_id: None,
            created_at: now,
            updated_at: now,
        };
        let mut rows = self.lock();
        if rows.iter().any(|row| row.id == new.id) {
            anyhow::bail!("surface {} already exists", new.id);
        }
        rows.push(surface.clone());
        Ok(surface)
    }

    async fn get(&self, id: Uuid) -> Result<Option<Surface>> {
        Ok(self.lock().iter().find(|row| row.id == id).cloned())
    }

    async fn update(&self, id: Uuid, changes: &SurfaceChanges) -> Result<Option<Surface>> {
        let mut rows = self.lock();
        let Some(row) = rows.iter_mut().find(|row| row.id == id) else {
            return Ok(None);
        };
        if let Some(title) = &changes.title {
            row.title = title.clone();
        }
        if let Some(fallback_text) = &changes.fallback_text {
            row.fallback_text = fallback_text.clone();
        }
        if let Some(components) = &changes.components {
            row.components = components.clone();
        }
        if let Some(data_model) = &changes.data_model {
            row.data_model = data_model.clone();
        }
        row.updated_at = Utc::now();
        Ok(Some(row.clone()))
    }

    async fn mark_submitted(
        &self,
        id: Uuid,
        action: &Value,
        action_message_id: Uuid,
    ) -> Result<Transition> {
        let mut rows = self.lock();
        let Some(row) = rows
            .iter_mut()
            .find(|row| row.id == id && row.status == SurfaceStatus::Active)
        else {
            return Ok(Transition::Conflict);
        };
        row.status = SurfaceStatus::Submitted;
        row.action = Some(action.clone());
        row.action_message_id = Some(action_message_id);
        row.updated_at = Utc::now();
        Ok(Transition::Applied)
    }

    async fn reopen(&self, id: Uuid, action_message_id: Uuid) -> Result<Transition> {
        let mut rows = self.lock();
        let Some(row) = rows.iter_mut().find(|row| {
            row.id == id
                && row.status == SurfaceStatus::Submitted
                && row.action_message_id == Some(action_message_id)
        }) else {
            return Ok(Transition::Conflict);
        };
        row.status = SurfaceStatus::Active;
        row.action = None;
        row.action_message_id = None;
        row.updated_at = Utc::now();
        Ok(Transition::Applied)
    }

    async fn list_for_conversation(&self, conversation_id: Uuid) -> Result<Vec<Surface>> {
        let mut surfaces: Vec<Surface> = self
            .lock()
            .iter()
            .filter(|row| row.conversation_id == conversation_id)
            .cloned()
            .collect();
        // Stable sort keeps insertion order for equal keys, like SQLite's rowid.
        surfaces.sort_by_key(|row| (row.turn, row.created_at));
        Ok(surfaces)
    }

    async fn action_message_map(&self, conversation_id: Uuid) -> Result<HashMap<Uuid, Surface>> {
        Ok(self
            .lock()
            .iter()
            .filter(|row| row.conversation_id == conversation_id)
            .filter_map(|row| Some((row.action_message_id?, row.clone())))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{ConversationStore, StorageLayer};

    fn new_surface(conversation_id: Uuid, turn: i64) -> NewSurface {
        NewSurface {
            id: Uuid::new_v4(),
            conversation_id,
            turn,
            anchor_message_id: Uuid::new_v4(),
            anchor_call_index: 2,
            tool_call_id: Some("call_1".to_string()),
            title: "Trip details".to_string(),
            fallback_text: "Where are you going?".to_string(),
            catalog_id: "https://assistant.invalid/a2ui/catalogs/assistant/v1/catalog.json"
                .to_string(),
            protocol_version: "1.0".to_string(),
            components: json!([{"id": "root", "component": "Text", "text": "Hi"}]),
            data_model: json!({"name": "Ada"}),
            send_data_model: true,
        }
    }

    /// Behaviour every store must share.
    async fn assert_store_contract(
        store: &impl SurfaceStore,
        conversation_id: Uuid,
        other_conversation_id: Uuid,
    ) {
        // create + get round-trip every field, starting active
        let draft = new_surface(conversation_id, 1);
        let created = store.create(&draft).await.unwrap();
        assert_eq!(created.id, draft.id);
        assert_eq!(created.conversation_id, conversation_id);
        assert_eq!(created.turn, 1);
        assert_eq!(created.anchor_message_id, draft.anchor_message_id);
        assert_eq!(created.anchor_call_index, 2);
        assert_eq!(created.tool_call_id.as_deref(), Some("call_1"));
        assert_eq!(created.title, "Trip details");
        assert_eq!(created.fallback_text, "Where are you going?");
        assert_eq!(created.catalog_id, draft.catalog_id);
        assert_eq!(created.protocol_version, "1.0");
        assert_eq!(created.components, draft.components);
        assert_eq!(created.data_model, draft.data_model);
        assert!(created.send_data_model);
        assert_eq!(created.status, SurfaceStatus::Active);
        assert_eq!(created.action, None);
        assert_eq!(created.action_message_id, None);
        assert_eq!(store.get(draft.id).await.unwrap(), Some(created.clone()));
        assert_eq!(
            store.get(Uuid::new_v4()).await.unwrap(),
            None,
            "an unknown id is absent"
        );

        // update replaces only the supplied fields
        let updated = store
            .update(
                draft.id,
                &SurfaceChanges {
                    components: Some(json!([{"id": "root", "component": "Text", "text": "Yo"}])),
                    data_model: Some(json!({"name": "Grace"})),
                    ..SurfaceChanges::default()
                },
            )
            .await
            .unwrap()
            .expect("surface exists");
        assert_eq!(updated.components[0]["text"], "Yo");
        assert_eq!(updated.data_model, json!({"name": "Grace"}));
        assert_eq!(updated.title, "Trip details", "unset fields are kept");
        assert!(updated.updated_at >= created.updated_at);
        let retitled = store
            .update(
                draft.id,
                &SurfaceChanges {
                    title: Some("New title".to_string()),
                    fallback_text: Some("New fallback".to_string()),
                    ..SurfaceChanges::default()
                },
            )
            .await
            .unwrap()
            .expect("surface exists");
        assert_eq!(retitled.title, "New title");
        assert_eq!(retitled.fallback_text, "New fallback");
        assert_eq!(retitled.data_model, json!({"name": "Grace"}));
        assert!(
            store
                .update(Uuid::new_v4(), &SurfaceChanges::default())
                .await
                .unwrap()
                .is_none(),
            "updating an unknown surface finds nothing"
        );

        // mark_submitted is a compare-and-set
        let action = json!({"name": "submit", "context": {"city": "Oslo"}});
        let message_id = Uuid::new_v4();
        assert_eq!(
            store
                .mark_submitted(draft.id, &action, message_id)
                .await
                .unwrap(),
            Transition::Applied
        );
        assert_eq!(
            store
                .mark_submitted(draft.id, &json!({"name": "other"}), Uuid::new_v4())
                .await
                .unwrap(),
            Transition::Conflict,
            "a second submission loses"
        );
        let submitted = store.get(draft.id).await.unwrap().expect("surface exists");
        assert_eq!(submitted.status, SurfaceStatus::Submitted);
        assert_eq!(submitted.action, Some(action.clone()));
        assert_eq!(submitted.action_message_id, Some(message_id));
        assert_eq!(
            store
                .mark_submitted(Uuid::new_v4(), &action, message_id)
                .await
                .unwrap(),
            Transition::Conflict,
            "an unknown surface cannot be submitted"
        );

        // action_message_map keys submitted surfaces by their message
        let map = store.action_message_map(conversation_id).await.unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map[&message_id].id, draft.id);

        // reopen only works from submitted with the same link
        assert_eq!(
            store.reopen(draft.id, Uuid::new_v4()).await.unwrap(),
            Transition::Conflict,
            "a different message id cannot reopen"
        );
        assert_eq!(
            store.reopen(draft.id, message_id).await.unwrap(),
            Transition::Applied
        );
        let reopened = store.get(draft.id).await.unwrap().expect("surface exists");
        assert_eq!(reopened.status, SurfaceStatus::Active);
        assert_eq!(reopened.action, None, "reopen clears the action");
        assert_eq!(reopened.action_message_id, None);
        assert_eq!(
            store.reopen(draft.id, message_id).await.unwrap(),
            Transition::Conflict,
            "an active surface cannot be reopened"
        );
        assert!(
            store
                .action_message_map(conversation_id)
                .await
                .unwrap()
                .is_empty(),
            "a reopened surface has no action message"
        );
        assert_eq!(
            store
                .mark_submitted(draft.id, &action, Uuid::new_v4())
                .await
                .unwrap(),
            Transition::Applied,
            "a reopened surface can be submitted again"
        );

        // list_for_conversation orders by (turn, created_at) and scopes by conversation
        let late = new_surface(conversation_id, 3);
        let early_a = new_surface(conversation_id, 2);
        let early_b = new_surface(conversation_id, 2);
        let elsewhere = new_surface(other_conversation_id, 1);
        for s in [&late, &early_a, &early_b, &elsewhere] {
            store.create(s).await.unwrap();
        }
        let ids: Vec<Uuid> = store
            .list_for_conversation(conversation_id)
            .await
            .unwrap()
            .iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(
            ids,
            vec![draft.id, early_a.id, early_b.id, late.id],
            "ordered by turn, then creation"
        );
    }

    async fn sqlite_conversation(storage: &StorageLayer) -> Uuid {
        let id = Uuid::new_v4();
        storage
            .conversation_store()
            .create_conversation_with_id(id, Some("surfaces"))
            .await
            .unwrap();
        id
    }

    #[tokio::test]
    async fn sqlite_store_meets_the_contract() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let conversation_id = sqlite_conversation(&storage).await;
        let other = sqlite_conversation(&storage).await;
        assert_store_contract(&storage.surface_store(), conversation_id, other).await;
    }

    #[tokio::test]
    async fn in_memory_store_meets_the_contract() {
        assert_store_contract(&InMemorySurfaceStore::new(), Uuid::new_v4(), Uuid::new_v4()).await;
    }

    #[tokio::test]
    async fn deleting_a_conversation_removes_its_surfaces() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let kept = sqlite_conversation(&storage).await;
        let doomed = sqlite_conversation(&storage).await;
        let store = storage.surface_store();
        let kept_surface = store.create(&new_surface(kept, 1)).await.unwrap();
        let doomed_surface = store.create(&new_surface(doomed, 1)).await.unwrap();

        storage
            .conversation_store()
            .delete_conversation(doomed)
            .await
            .unwrap();

        assert!(store.get(doomed_surface.id).await.unwrap().is_none());
        assert!(store.get(kept_surface.id).await.unwrap().is_some());
    }
}
