//! A2A protocol task store.
//!
//! Defines the [`A2aTaskStore`] trait (ADR-0009 trait pair) with two impls:
//! [`InMemoryA2aTaskStore`] for tests and [`SqliteA2aTaskStore`] for production,
//! the latter persisting tasks in the space db so they survive restart. Live SSE
//! subscribers and push-notification configs are process-local on both impls;
//! only durable task state is persisted.

use assistant_core::clock::{Clock, SystemClock};
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use tokio::sync::RwLock;
use uuid::Uuid;

use assistant_a2a_json_schema::types::{
    Message, PushNotificationConfig, Task, TaskPushNotificationConfig, TaskState, TaskStatus,
};

/// Thread-safe in-memory store for A2A tasks.
///
/// The in-memory half of the [`A2aTaskStore`] trait pair: used by tests (and
/// available as a non-persistent fallback). Production wires
/// [`SqliteA2aTaskStore`], so this is `allow(dead_code)` for non-test builds.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct InMemoryA2aTaskStore {
    inner: Arc<RwLock<TaskStoreInner>>,
}

#[allow(dead_code)]
#[derive(Debug, Default)]
struct TaskStoreInner {
    /// Tasks keyed by task ID.
    tasks: HashMap<String, Task>,
    /// Ordered list of task IDs (newest first) per context.
    context_index: HashMap<String, Vec<String>>,
    /// Push notification configs keyed by (task_id, config_id).
    push_configs: HashMap<(String, String), PushNotificationConfig>,
    /// Subscribers waiting for task updates: task_id -> list of senders.
    subscribers: HashMap<String, Vec<tokio::sync::mpsc::UnboundedSender<Task>>>,
}

#[allow(dead_code)] // in-memory trait-pair impl; constructed in tests
impl InMemoryA2aTaskStore {
    /// Creates a new empty task store.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(TaskStoreInner::default())),
        }
    }

    /// Creates a new task with the given context ID (or generates one).
    ///
    /// Returns the newly created task.
    pub async fn create_task(&self, context_id: Option<String>) -> Task {
        let task_id = Uuid::new_v4().to_string();
        let context_id = context_id.unwrap_or_else(|| Uuid::new_v4().to_string());

        let task = Task {
            id: task_id.clone(),
            context_id: context_id.clone(),
            status: TaskStatus {
                state: TaskState::TaskStateSubmitted,
                message: None,
                timestamp: Some(SystemClock.now()),
            },
            artifacts: vec![],
            history: vec![],
            metadata: None,
        };

        let mut inner = self.inner.write().await;
        inner.tasks.insert(task_id.clone(), task.clone());
        inner
            .context_index
            .entry(context_id)
            .or_default()
            .push(task_id);

        task
    }

    /// Retrieves a task by ID.
    pub async fn get_task(&self, task_id: &str) -> Option<Task> {
        let inner = self.inner.read().await;
        inner.tasks.get(task_id).cloned()
    }

    /// Updates the status of a task and notifies subscribers.
    pub async fn update_status(&self, task_id: &str, state: TaskState, message: Option<Message>) {
        let mut inner = self.inner.write().await;
        if let Some(task) = inner.tasks.get_mut(task_id) {
            task.status = TaskStatus {
                state,
                message,
                timestamp: Some(SystemClock.now()),
            };
        }

        // Notify subscribers (separate borrow scope).
        if let (Some(task), Some(subs)) = (inner.tasks.get(task_id), inner.subscribers.get(task_id))
        {
            let snapshot = task.clone();
            subs.iter().for_each(|tx| {
                let _ = tx.send(snapshot.clone());
            });
        }
    }

    /// Appends a message to the task's history.
    pub async fn append_history(&self, task_id: &str, message: Message) {
        let mut inner = self.inner.write().await;
        if let Some(task) = inner.tasks.get_mut(task_id) {
            task.history.push(message);
        }
    }

    /// Lists tasks, optionally filtered by context ID and/or state.
    pub async fn list_tasks(
        &self,
        context_id: Option<&str>,
        status: Option<TaskState>,
        limit: usize,
    ) -> Vec<Task> {
        let inner = self.inner.read().await;

        let iter: Box<dyn Iterator<Item = &Task> + '_> = if let Some(ctx) = context_id {
            if let Some(ids) = inner.context_index.get(ctx) {
                Box::new(ids.iter().rev().filter_map(|id| inner.tasks.get(id)))
            } else {
                Box::new(std::iter::empty())
            }
        } else {
            Box::new(inner.tasks.values())
        };

        iter.filter(|t| status.is_none() || Some(t.status.state) == status)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Cancels a task if it is not already in a terminal state.
    ///
    /// Returns the updated task, or `None` if the task doesn't exist.
    pub async fn cancel_task(&self, task_id: &str) -> Option<Task> {
        let mut inner = self.inner.write().await;

        {
            let task = inner.tasks.get_mut(task_id)?;
            if task.status.state.is_terminal() {
                return Some(task.clone());
            }

            task.status = TaskStatus {
                state: TaskState::TaskStateCanceled,
                message: None,
                timestamp: Some(SystemClock.now()),
            };
        }

        // Notify subscribers (separate borrow scope).
        if let (Some(task), Some(subs)) = (inner.tasks.get(task_id), inner.subscribers.get(task_id))
        {
            let snapshot = task.clone();
            subs.iter().for_each(|tx| {
                let _ = tx.send(snapshot.clone());
            });
        }

        inner.tasks.get(task_id).cloned()
    }

    /// Subscribes to updates for a task. Returns a receiver that yields task
    /// snapshots on every status change.
    pub async fn subscribe(
        &self,
        task_id: &str,
    ) -> Option<tokio::sync::mpsc::UnboundedReceiver<Task>> {
        let mut inner = self.inner.write().await;

        // Task must exist and not be in a terminal state.
        let task = inner.tasks.get(task_id)?;
        if task.status.state.is_terminal() {
            return None;
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        inner
            .subscribers
            .entry(task_id.to_string())
            .or_default()
            .push(tx);

        Some(rx)
    }

    /// Removes closed subscriber channels for a task.
    pub async fn cleanup_subscribers(&self, task_id: &str) {
        let mut inner = self.inner.write().await;
        if let Some(subs) = inner.subscribers.get_mut(task_id) {
            subs.retain(|tx| !tx.is_closed());
            if subs.is_empty() {
                inner.subscribers.remove(task_id);
            }
        }
    }

    // -- Push Notification Config operations --

    /// Creates a push notification config for a task.
    pub async fn create_push_config(
        &self,
        task_id: &str,
        mut config: PushNotificationConfig,
    ) -> Option<TaskPushNotificationConfig> {
        let inner_read = self.inner.read().await;
        if !inner_read.tasks.contains_key(task_id) {
            return None;
        }
        drop(inner_read);

        let config_id = config
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        config.id = Some(config_id.clone());

        let result = TaskPushNotificationConfig {
            tenant: None,
            task_id: task_id.to_string(),
            push_notification_config: config.clone(),
        };

        let mut inner = self.inner.write().await;
        inner
            .push_configs
            .insert((task_id.to_string(), config_id), config);

        Some(result)
    }

    /// Gets a push notification config.
    pub async fn get_push_config(
        &self,
        task_id: &str,
        config_id: &str,
    ) -> Option<TaskPushNotificationConfig> {
        let inner = self.inner.read().await;
        inner
            .push_configs
            .get(&(task_id.to_string(), config_id.to_string()))
            .map(|config| TaskPushNotificationConfig {
                tenant: None,
                task_id: task_id.to_string(),
                push_notification_config: config.clone(),
            })
    }

    /// Lists push notification configs for a task.
    pub async fn list_push_configs(&self, task_id: &str) -> Vec<TaskPushNotificationConfig> {
        let inner = self.inner.read().await;
        inner
            .push_configs
            .iter()
            .filter(|((tid, _), _)| tid == task_id)
            .map(|((tid, _), config)| TaskPushNotificationConfig {
                tenant: None,
                task_id: tid.clone(),
                push_notification_config: config.clone(),
            })
            .collect()
    }

    /// Deletes a push notification config. Returns `true` if it existed.
    pub async fn delete_push_config(&self, task_id: &str, config_id: &str) -> bool {
        let mut inner = self.inner.write().await;
        inner
            .push_configs
            .remove(&(task_id.to_string(), config_id.to_string()))
            .is_some()
    }
}

impl Default for InMemoryA2aTaskStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Persistence + lifecycle surface for A2A tasks. Implemented in-memory
/// ([`InMemoryA2aTaskStore`]) and SQLite-backed ([`SqliteA2aTaskStore`]).
///
/// All methods are fallible so SQLite/serde failures propagate to callers
/// instead of silently degrading (mirrors the `ConversationStore` convention).
#[async_trait]
pub trait A2aTaskStore: Send + Sync {
    async fn create_task(&self, context_id: Option<String>) -> Result<Task>;
    async fn get_task(&self, task_id: &str) -> Result<Option<Task>>;
    async fn update_status(
        &self,
        task_id: &str,
        state: TaskState,
        message: Option<Message>,
    ) -> Result<()>;
    async fn append_history(&self, task_id: &str, message: Message) -> Result<()>;
    async fn list_tasks(
        &self,
        context_id: Option<&str>,
        status: Option<TaskState>,
        limit: usize,
    ) -> Result<Vec<Task>>;
    async fn cancel_task(&self, task_id: &str) -> Result<Option<Task>>;
    async fn subscribe(
        &self,
        task_id: &str,
    ) -> Result<Option<tokio::sync::mpsc::UnboundedReceiver<Task>>>;
    async fn cleanup_subscribers(&self, task_id: &str) -> Result<()>;
    async fn create_push_config(
        &self,
        task_id: &str,
        config: PushNotificationConfig,
    ) -> Result<Option<TaskPushNotificationConfig>>;
    async fn get_push_config(
        &self,
        task_id: &str,
        config_id: &str,
    ) -> Result<Option<TaskPushNotificationConfig>>;
    async fn list_push_configs(&self, task_id: &str) -> Result<Vec<TaskPushNotificationConfig>>;
    async fn delete_push_config(&self, task_id: &str, config_id: &str) -> Result<bool>;
}

// The in-memory store's inherent methods take precedence, so each trait method
// forwards to them (no recursion); in-memory ops are infallible -> always Ok.
#[async_trait]
impl A2aTaskStore for InMemoryA2aTaskStore {
    async fn create_task(&self, context_id: Option<String>) -> Result<Task> {
        Ok(self.create_task(context_id).await)
    }
    async fn get_task(&self, task_id: &str) -> Result<Option<Task>> {
        Ok(self.get_task(task_id).await)
    }
    async fn update_status(
        &self,
        task_id: &str,
        state: TaskState,
        message: Option<Message>,
    ) -> Result<()> {
        self.update_status(task_id, state, message).await;
        Ok(())
    }
    async fn append_history(&self, task_id: &str, message: Message) -> Result<()> {
        self.append_history(task_id, message).await;
        Ok(())
    }
    async fn list_tasks(
        &self,
        context_id: Option<&str>,
        status: Option<TaskState>,
        limit: usize,
    ) -> Result<Vec<Task>> {
        Ok(self.list_tasks(context_id, status, limit).await)
    }
    async fn cancel_task(&self, task_id: &str) -> Result<Option<Task>> {
        Ok(self.cancel_task(task_id).await)
    }
    async fn subscribe(
        &self,
        task_id: &str,
    ) -> Result<Option<tokio::sync::mpsc::UnboundedReceiver<Task>>> {
        Ok(self.subscribe(task_id).await)
    }
    async fn cleanup_subscribers(&self, task_id: &str) -> Result<()> {
        self.cleanup_subscribers(task_id).await;
        Ok(())
    }
    async fn create_push_config(
        &self,
        task_id: &str,
        config: PushNotificationConfig,
    ) -> Result<Option<TaskPushNotificationConfig>> {
        Ok(self.create_push_config(task_id, config).await)
    }
    async fn get_push_config(
        &self,
        task_id: &str,
        config_id: &str,
    ) -> Result<Option<TaskPushNotificationConfig>> {
        Ok(self.get_push_config(task_id, config_id).await)
    }
    async fn list_push_configs(&self, task_id: &str) -> Result<Vec<TaskPushNotificationConfig>> {
        Ok(self.list_push_configs(task_id).await)
    }
    async fn delete_push_config(&self, task_id: &str, config_id: &str) -> Result<bool> {
        Ok(self.delete_push_config(task_id, config_id).await)
    }
}

/// Process-local state shared by [`SqliteA2aTaskStore`]: live SSE subscribers
/// and push-notification configs. Not persisted.
#[derive(Default)]
struct Ephemeral {
    subscribers: HashMap<String, Vec<tokio::sync::mpsc::UnboundedSender<Task>>>,
    push_configs: HashMap<(String, String), PushNotificationConfig>,
}

/// SQLite-backed A2A task store. Tasks persist in the `a2a_tasks` table of the
/// space db (scoped by `agent_id`); subscribers and push configs stay in memory.
#[derive(Clone)]
pub struct SqliteA2aTaskStore {
    pool: SqlitePool,
    agent_id: String,
    ephemeral: Arc<RwLock<Ephemeral>>,
}

impl SqliteA2aTaskStore {
    /// Create a store scoped to an agent's tasks in the given space pool.
    pub fn new(pool: SqlitePool, agent_id: impl Into<String>) -> Self {
        Self {
            pool,
            agent_id: agent_id.into(),
            ephemeral: Arc::new(RwLock::new(Ephemeral::default())),
        }
    }

    fn state_str(state: &TaskState) -> Result<String> {
        serde_json::to_string(state).context("serializing A2A task state")
    }

    async fn persist_new(&self, task: &Task) -> Result<()> {
        let json = serde_json::to_string(task).context("serializing A2A task")?;
        sqlx::query(
            "INSERT INTO a2a_tasks (id, agent_id, context_id, state, task_json) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&task.id)
        .bind(&self.agent_id)
        .bind(&task.context_id)
        .bind(Self::state_str(&task.status.state)?)
        .bind(json)
        .execute(&self.pool)
        .await
        .with_context(|| format!("persisting A2A task {}", task.id))?;
        Ok(())
    }

    async fn persist_update(&self, task: &Task) -> Result<()> {
        let json = serde_json::to_string(task).context("serializing A2A task")?;
        sqlx::query(
            "UPDATE a2a_tasks SET state = ?, task_json = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ? AND agent_id = ?",
        )
        .bind(Self::state_str(&task.status.state)?)
        .bind(json)
        .bind(&task.id)
        .bind(&self.agent_id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("updating A2A task {}", task.id))?;
        Ok(())
    }

    async fn notify(&self, task_id: &str) -> Result<()> {
        if let Some(task) = self.get_task(task_id).await? {
            let eph = self.ephemeral.read().await;
            if let Some(subs) = eph.subscribers.get(task_id) {
                for tx in subs {
                    let _ = tx.send(task.clone());
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl A2aTaskStore for SqliteA2aTaskStore {
    async fn create_task(&self, context_id: Option<String>) -> Result<Task> {
        let task = Task {
            id: Uuid::new_v4().to_string(),
            context_id: context_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            status: TaskStatus {
                state: TaskState::TaskStateSubmitted,
                message: None,
                timestamp: Some(SystemClock.now()),
            },
            artifacts: vec![],
            history: vec![],
            metadata: None,
        };
        self.persist_new(&task).await?;
        Ok(task)
    }

    async fn get_task(&self, task_id: &str) -> Result<Option<Task>> {
        let row = sqlx::query("SELECT task_json FROM a2a_tasks WHERE id = ? AND agent_id = ?")
            .bind(task_id)
            .bind(&self.agent_id)
            .fetch_optional(&self.pool)
            .await
            .context("querying A2A task")?;
        let Some(row) = row else {
            return Ok(None);
        };
        let json: String = row.try_get("task_json").context("reading task_json")?;
        let task = serde_json::from_str(&json).context("deserializing A2A task")?;
        Ok(Some(task))
    }

    async fn update_status(
        &self,
        task_id: &str,
        state: TaskState,
        message: Option<Message>,
    ) -> Result<()> {
        let Some(mut task) = self.get_task(task_id).await? else {
            return Ok(());
        };
        task.status = TaskStatus {
            state,
            message,
            timestamp: Some(SystemClock.now()),
        };
        self.persist_update(&task).await?;
        self.notify(task_id).await
    }

    async fn append_history(&self, task_id: &str, message: Message) -> Result<()> {
        let Some(mut task) = self.get_task(task_id).await? else {
            return Ok(());
        };
        task.history.push(message);
        self.persist_update(&task).await
    }

    async fn list_tasks(
        &self,
        context_id: Option<&str>,
        status: Option<TaskState>,
        limit: usize,
    ) -> Result<Vec<Task>> {
        let rows: Vec<String> = match context_id {
            Some(ctx) => sqlx::query_scalar(
                "SELECT task_json FROM a2a_tasks WHERE agent_id = ? AND context_id = ? \
                 ORDER BY updated_at DESC",
            )
            .bind(&self.agent_id)
            .bind(ctx)
            .fetch_all(&self.pool)
            .await
            .context("listing A2A tasks")?,
            None => sqlx::query_scalar(
                "SELECT task_json FROM a2a_tasks WHERE agent_id = ? ORDER BY updated_at DESC",
            )
            .bind(&self.agent_id)
            .fetch_all(&self.pool)
            .await
            .context("listing A2A tasks")?,
        };

        let mut tasks = Vec::new();
        for j in &rows {
            let task: Task = serde_json::from_str(j).context("deserializing A2A task")?;
            if status.is_none() || Some(task.status.state) == status {
                tasks.push(task);
                if tasks.len() >= limit {
                    break;
                }
            }
        }
        Ok(tasks)
    }

    async fn cancel_task(&self, task_id: &str) -> Result<Option<Task>> {
        let Some(mut task) = self.get_task(task_id).await? else {
            return Ok(None);
        };
        if task.status.state.is_terminal() {
            return Ok(Some(task));
        }
        task.status = TaskStatus {
            state: TaskState::TaskStateCanceled,
            message: None,
            timestamp: Some(SystemClock.now()),
        };
        self.persist_update(&task).await?;
        self.notify(task_id).await?;
        Ok(Some(task))
    }

    async fn subscribe(
        &self,
        task_id: &str,
    ) -> Result<Option<tokio::sync::mpsc::UnboundedReceiver<Task>>> {
        let Some(task) = self.get_task(task_id).await? else {
            return Ok(None);
        };
        if task.status.state.is_terminal() {
            return Ok(None);
        }
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.ephemeral
            .write()
            .await
            .subscribers
            .entry(task_id.to_string())
            .or_default()
            .push(tx);
        Ok(Some(rx))
    }

    async fn cleanup_subscribers(&self, task_id: &str) -> Result<()> {
        let mut eph = self.ephemeral.write().await;
        if let Some(subs) = eph.subscribers.get_mut(task_id) {
            subs.retain(|tx| !tx.is_closed());
            if subs.is_empty() {
                eph.subscribers.remove(task_id);
            }
        }
        Ok(())
    }

    async fn create_push_config(
        &self,
        task_id: &str,
        mut config: PushNotificationConfig,
    ) -> Result<Option<TaskPushNotificationConfig>> {
        if self.get_task(task_id).await?.is_none() {
            return Ok(None);
        }
        let config_id = config
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        config.id = Some(config_id.clone());
        let result = TaskPushNotificationConfig {
            tenant: None,
            task_id: task_id.to_string(),
            push_notification_config: config.clone(),
        };
        self.ephemeral
            .write()
            .await
            .push_configs
            .insert((task_id.to_string(), config_id), config);
        Ok(Some(result))
    }

    async fn get_push_config(
        &self,
        task_id: &str,
        config_id: &str,
    ) -> Result<Option<TaskPushNotificationConfig>> {
        let eph = self.ephemeral.read().await;
        Ok(eph
            .push_configs
            .get(&(task_id.to_string(), config_id.to_string()))
            .map(|config| TaskPushNotificationConfig {
                tenant: None,
                task_id: task_id.to_string(),
                push_notification_config: config.clone(),
            }))
    }

    async fn list_push_configs(&self, task_id: &str) -> Result<Vec<TaskPushNotificationConfig>> {
        let eph = self.ephemeral.read().await;
        Ok(eph
            .push_configs
            .iter()
            .filter(|((tid, _), _)| tid == task_id)
            .map(|((tid, _), config)| TaskPushNotificationConfig {
                tenant: None,
                task_id: tid.clone(),
                push_notification_config: config.clone(),
            })
            .collect())
    }

    async fn delete_push_config(&self, task_id: &str, config_id: &str) -> Result<bool> {
        Ok(self
            .ephemeral
            .write()
            .await
            .push_configs
            .remove(&(task_id.to_string(), config_id.to_string()))
            .is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_a2a_json_schema::types::{Part, Role};

    #[tokio::test]
    async fn test_create_and_get_task() {
        let store = InMemoryA2aTaskStore::new();
        let task = store.create_task(Some("ctx-1".to_string())).await;
        assert_eq!(task.context_id, "ctx-1");
        assert_eq!(task.status.state, TaskState::TaskStateSubmitted);

        let fetched = store.get_task(&task.id).await.unwrap();
        assert_eq!(fetched.id, task.id);
    }

    #[tokio::test]
    async fn test_update_status() {
        let store = InMemoryA2aTaskStore::new();
        let task = store.create_task(None).await;

        store
            .update_status(&task.id, TaskState::TaskStateWorking, None)
            .await;

        let fetched = store.get_task(&task.id).await.unwrap();
        assert_eq!(fetched.status.state, TaskState::TaskStateWorking);
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let store = InMemoryA2aTaskStore::new();
        let task = store.create_task(None).await;

        let canceled = store.cancel_task(&task.id).await.unwrap();
        assert_eq!(canceled.status.state, TaskState::TaskStateCanceled);

        // Canceling again should return the same terminal state.
        let again = store.cancel_task(&task.id).await.unwrap();
        assert_eq!(again.status.state, TaskState::TaskStateCanceled);
    }

    #[tokio::test]
    async fn test_list_tasks_filter() {
        let store = InMemoryA2aTaskStore::new();
        let t1 = store.create_task(Some("ctx-a".to_string())).await;
        let _t2 = store.create_task(Some("ctx-b".to_string())).await;

        store
            .update_status(&t1.id, TaskState::TaskStateCompleted, None)
            .await;

        let all = store.list_tasks(None, None, 100).await;
        assert_eq!(all.len(), 2);

        let completed = store
            .list_tasks(None, Some(TaskState::TaskStateCompleted), 100)
            .await;
        assert_eq!(completed.len(), 1);

        let ctx_a = store.list_tasks(Some("ctx-a"), None, 100).await;
        assert_eq!(ctx_a.len(), 1);
    }

    #[tokio::test]
    async fn test_append_history() {
        let store = InMemoryA2aTaskStore::new();
        let task = store.create_task(None).await;

        let msg = Message {
            message_id: "m1".to_string(),
            context_id: None,
            task_id: Some(task.id.clone()),
            role: Role::RoleUser,
            parts: vec![Part::text("hello")],
            metadata: None,
            extensions: vec![],
            reference_task_ids: vec![],
        };

        store.append_history(&task.id, msg).await;

        let fetched = store.get_task(&task.id).await.unwrap();
        assert_eq!(fetched.history.len(), 1);
    }

    #[tokio::test]
    async fn test_push_config_crud() {
        let store = InMemoryA2aTaskStore::new();
        let task = store.create_task(None).await;

        let config = PushNotificationConfig {
            id: None,
            url: "https://example.com/notify".to_string(),
            token: Some("tok-1".to_string()),
            authentication: None,
        };

        let created = store.create_push_config(&task.id, config).await.unwrap();
        let config_id = created.push_notification_config.id.clone().unwrap();

        let fetched = store.get_push_config(&task.id, &config_id).await.unwrap();
        assert_eq!(
            fetched.push_notification_config.url,
            "https://example.com/notify"
        );

        let list = store.list_push_configs(&task.id).await;
        assert_eq!(list.len(), 1);

        assert!(store.delete_push_config(&task.id, &config_id).await);
        assert!(store.get_push_config(&task.id, &config_id).await.is_none());
    }

    fn user_msg(task_id: &str, text: &str) -> Message {
        Message {
            message_id: Uuid::new_v4().to_string(),
            context_id: None,
            task_id: Some(task_id.to_string()),
            role: Role::RoleUser,
            parts: vec![Part::text(text)],
            metadata: None,
            extensions: vec![],
            reference_task_ids: vec![],
        }
    }

    #[tokio::test]
    async fn sqlite_persists_task_across_fresh_store() {
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let store = SqliteA2aTaskStore::new(storage.pool.clone(), "default");

        let task = store.create_task(Some("ctx-1".to_string())).await.unwrap();
        store
            .append_history(&task.id, user_msg(&task.id, "hi"))
            .await
            .unwrap();
        store
            .update_status(&task.id, TaskState::TaskStateCompleted, None)
            .await
            .unwrap();

        // A fresh store over the same pool simulates a restart.
        let reopened = SqliteA2aTaskStore::new(storage.pool.clone(), "default");
        let fetched = reopened
            .get_task(&task.id)
            .await
            .unwrap()
            .expect("task persisted");
        assert_eq!(fetched.id, task.id);
        assert_eq!(fetched.context_id, "ctx-1");
        assert_eq!(fetched.status.state, TaskState::TaskStateCompleted);
        assert_eq!(fetched.history.len(), 1);
    }

    #[tokio::test]
    async fn inmemory_and_sqlite_parity() {
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let sqlite = SqliteA2aTaskStore::new(storage.pool.clone(), "default");
        let memory = InMemoryA2aTaskStore::new();

        for store in [&sqlite as &dyn A2aTaskStore, &memory as &dyn A2aTaskStore] {
            let task = store.create_task(Some("c".to_string())).await.unwrap();
            assert_eq!(task.status.state, TaskState::TaskStateSubmitted);

            store
                .append_history(&task.id, user_msg(&task.id, "hello"))
                .await
                .unwrap();
            store
                .update_status(&task.id, TaskState::TaskStateCompleted, None)
                .await
                .unwrap();

            let got = store.get_task(&task.id).await.unwrap().expect("get");
            assert_eq!(got.status.state, TaskState::TaskStateCompleted);
            assert_eq!(got.history.len(), 1);

            let list = store.list_tasks(Some("c"), None, 10).await.unwrap();
            assert_eq!(list.len(), 1);
            let completed = store
                .list_tasks(None, Some(TaskState::TaskStateCompleted), 10)
                .await
                .unwrap();
            assert_eq!(completed.len(), 1);
        }
    }
}
