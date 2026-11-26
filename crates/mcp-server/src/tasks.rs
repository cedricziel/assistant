//! `run-prompt` tasks for clients that declare the Tasks extension.
//!
//! rmcp's [`TaskManager`] runs the turns and answers `tasks/get` and
//! `tasks/cancel`; [`TaskStore`] adds ownership on top, so a caller only ever
//! sees the tasks it created.

use std::collections::HashMap;
use std::sync::Arc;

use rmcp::ErrorData;
use rmcp::model::{CreateTaskResult, DetailedTask};
use rmcp::task_manager::{TaskContext, TaskFuture, TaskManager, TaskOptions};
use tokio::sync::Mutex;

use assistant_core::auth::AuthContext;
use assistant_core::identity::{OrgId, UserId};

/// How long a task stays observable, in milliseconds.
const TASK_TTL_MS: u64 = 30 * 60 * 1000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Owner {
    user_id: UserId,
    org_id: OrgId,
}

impl Owner {
    fn of(auth: &AuthContext) -> Self {
        Self {
            user_id: auth.user_id.clone(),
            org_id: auth.org_id.clone(),
        }
    }
}

/// The `run-prompt` tasks of every server built for one endpoint.
///
/// Clones share one store. Over HTTP every request gets a new server, so
/// whoever builds the servers owns the store and passes it to each one.
/// Tasks live in memory only.
#[derive(Clone, Default)]
pub struct TaskStore {
    manager: TaskManager,
    owners: Arc<Mutex<HashMap<String, Owner>>>,
}

impl TaskStore {
    /// Runs `make_future` as a task owned by `auth`.
    pub(crate) async fn spawn(
        &self,
        auth: &AuthContext,
        make_future: impl FnOnce(TaskContext) -> TaskFuture,
    ) -> CreateTaskResult {
        let mut owners = self.owners.lock().await;
        // The manager evicts expired tasks; forget their owners too.
        owners.retain(|id, _| self.manager.get_task(id).is_ok());
        let task = self
            .manager
            .spawn(TaskOptions::new().with_ttl_ms(TASK_TTL_MS), make_future);
        owners.insert(task.task_id.clone(), Owner::of(auth));
        CreateTaskResult::new(task)
    }

    /// `tasks/get` for `auth`.
    pub(crate) async fn get(
        &self,
        auth: &AuthContext,
        task_id: &str,
    ) -> Result<DetailedTask, ErrorData> {
        self.check_owner(auth, task_id).await?;
        self.manager.get_task(task_id)
    }

    /// `tasks/cancel` for `auth`.
    pub(crate) async fn cancel(&self, auth: &AuthContext, task_id: &str) -> Result<(), ErrorData> {
        self.check_owner(auth, task_id).await?;
        self.manager.cancel_task(task_id)
    }

    /// Answers another identity's task exactly like an unknown one.
    async fn check_owner(&self, auth: &AuthContext, task_id: &str) -> Result<(), ErrorData> {
        match self.owners.lock().await.get(task_id) {
            Some(owner) if *owner == Owner::of(auth) => Ok(()),
            _ => Err(ErrorData::invalid_params(
                format!("unknown task: {task_id}"),
                None,
            )),
        }
    }
}
