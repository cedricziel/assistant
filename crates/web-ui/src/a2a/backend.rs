//! Per-space services behind the A2A handlers.
//!
//! An A2A request is bound to a space like any data-plane request. This trait
//! hands the handlers that space's turn interface and task store, so tests can
//! stand in a fixed pair while production resolves both per space.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::Result;
use async_trait::async_trait;

use assistant_runtime::{AssistantInterface, SpaceRuntimes};

use super::task_store::{A2aTaskStore, SqliteA2aTaskStore};
use crate::space::SpaceCtx;

#[async_trait]
pub trait A2aSpaceBackend: Send + Sync {
    /// Orchestrator that runs turns for the space.
    async fn interface(&self, space: &SpaceCtx) -> Result<Arc<dyn AssistantInterface>>;

    /// Durable task store of the space. Repeated calls for one space return
    /// the same store, so subscribers registered by one request see updates
    /// made by another.
    async fn task_store(&self, space: &SpaceCtx) -> Result<Arc<dyn A2aTaskStore>>;
}

/// Production backend: each space's orchestrator and its `a2a_tasks` table.
pub struct RuntimeA2aBackend {
    runtimes: Arc<SpaceRuntimes>,
    agent_id: String,
    task_stores: Mutex<HashMap<(String, String), Arc<SqliteA2aTaskStore>>>,
}

impl RuntimeA2aBackend {
    pub fn new(runtimes: Arc<SpaceRuntimes>, agent_id: impl Into<String>) -> Self {
        Self {
            runtimes,
            agent_id: agent_id.into(),
            task_stores: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl A2aSpaceBackend for RuntimeA2aBackend {
    async fn interface(&self, space: &SpaceCtx) -> Result<Arc<dyn AssistantInterface>> {
        let runtime = self
            .runtimes
            .get(&space.org_slug, &space.space_slug)
            .await?;
        Ok(runtime.orchestrator.clone())
    }

    async fn task_store(&self, space: &SpaceCtx) -> Result<Arc<dyn A2aTaskStore>> {
        let mut stores = self
            .task_stores
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let store = stores
            .entry((space.org_slug.clone(), space.space_slug.clone()))
            .or_insert_with(|| {
                Arc::new(SqliteA2aTaskStore::new(
                    space.storage.pool.clone(),
                    self.agent_id.clone(),
                ))
            });
        Ok(store.clone())
    }
}
