//! In-memory task store for A2A protocol tasks.
//!
//! Manages task lifecycle, history, and artifact storage. Tasks are kept in
//! memory for now; persistence can be added later via the storage layer.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;
use uuid::Uuid;

use assistant_a2a_json_schema::types::{
    Message, PushNotificationConfig, Task, TaskPushNotificationConfig, TaskState, TaskStatus,
};

/// Thread-safe in-memory store for A2A tasks.
#[derive(Debug, Clone)]
pub struct TaskStore {
    inner: Arc<RwLock<TaskStoreInner>>,
}

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

impl TaskStore {
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
                timestamp: Some(Utc::now()),
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
                timestamp: Some(Utc::now()),
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
                timestamp: Some(Utc::now()),
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

impl Default for TaskStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_a2a_json_schema::types::{Part, Role};

    #[tokio::test]
    async fn test_create_and_get_task() {
        let store = TaskStore::new();
        let task = store.create_task(Some("ctx-1".to_string())).await;
        assert_eq!(task.context_id, "ctx-1");
        assert_eq!(task.status.state, TaskState::TaskStateSubmitted);

        let fetched = store.get_task(&task.id).await.unwrap();
        assert_eq!(fetched.id, task.id);
    }

    #[tokio::test]
    async fn test_update_status() {
        let store = TaskStore::new();
        let task = store.create_task(None).await;

        store
            .update_status(&task.id, TaskState::TaskStateWorking, None)
            .await;

        let fetched = store.get_task(&task.id).await.unwrap();
        assert_eq!(fetched.status.state, TaskState::TaskStateWorking);
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let store = TaskStore::new();
        let task = store.create_task(None).await;

        let canceled = store.cancel_task(&task.id).await.unwrap();
        assert_eq!(canceled.status.state, TaskState::TaskStateCanceled);

        // Canceling again should return the same terminal state.
        let again = store.cancel_task(&task.id).await.unwrap();
        assert_eq!(again.status.state, TaskState::TaskStateCanceled);
    }

    #[tokio::test]
    async fn test_list_tasks_filter() {
        let store = TaskStore::new();
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
        let store = TaskStore::new();
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
        let store = TaskStore::new();
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
}
