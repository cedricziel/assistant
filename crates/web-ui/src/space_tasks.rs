//! Background tasks the web server runs for every space.
//!
//! The workflow runner and its schedule and event adapters work against one
//! space's database. [`workflow_spawner`] starts a set per space, and each
//! set's assistant-turn actions run on that space's orchestrator as that
//! space's system caller.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use uuid::Uuid;

use assistant_core::auth::AuthContext;
use assistant_core::types::conversation::Interface;
use assistant_runtime::{Orchestrator, SpaceRef, SpaceTaskSpawner};
use assistant_workflow::{
    AssistantTurnActionExecutor, AssistantTurnClient, WorkflowActionExecutor,
    spawn_event_trigger_adapter, spawn_schedule_trigger_adapter, spawn_workflow_runner,
};
use assistant_workflow_http::HttpRequestActionExecutor;

/// Submits workflow turns to one space's orchestrator.
struct OrchestratorTurnClient {
    orchestrator: Arc<Orchestrator>,
    auth: AuthContext,
}

#[async_trait::async_trait]
impl AssistantTurnClient for OrchestratorTurnClient {
    async fn submit_turn(&self, prompt: &str, conversation_id: Uuid) -> Result<String> {
        let result = self
            .orchestrator
            .submit_turn(
                &self.auth,
                prompt,
                conversation_id,
                Interface::Scheduler,
                None,
            )
            .await?;
        Ok(result.answer)
    }
}

/// Spawner for the workflow runner and trigger adapters of one space.
pub(crate) fn workflow_spawner(poll_interval: Duration) -> SpaceTaskSpawner {
    Arc::new(move |space: SpaceRef| {
        let storage = space.runtime.storage.clone();
        let turn_client = Arc::new(OrchestratorTurnClient {
            orchestrator: space.runtime.orchestrator.clone(),
            auth: AuthContext::system_in(space.org_id.clone(), space.space_id.clone()),
        });
        let action_executors: Vec<Arc<dyn WorkflowActionExecutor>> = vec![
            Arc::new(AssistantTurnActionExecutor::new(turn_client)),
            Arc::new(HttpRequestActionExecutor::default()),
        ];
        vec![
            spawn_workflow_runner(storage.clone(), poll_interval, action_executors),
            spawn_schedule_trigger_adapter(storage.clone(), poll_interval),
            spawn_event_trigger_adapter(storage, space.runtime.bus.clone(), poll_interval),
        ]
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use assistant_core::identity::{OrgId, SpaceId};
    use assistant_core::store::{OrgStore as _, Organization, Space, SpaceStore as _};
    use assistant_core::types::agent::AssistantConfig;
    use assistant_llm_provider::scripted::ScriptedLlmProvider;
    use assistant_runtime::bootstrap::AutoDenyConfirmation;
    use assistant_runtime::{SpaceRuntimes, SpaceSupervisor};
    use assistant_storage::{
        ConversationStore as _, OrgPoolFactory, SqliteConversationStore, StorageLayer,
        WorkflowGraph, WorkflowStore as _,
    };

    use super::*;

    fn scheduled_turn_graph() -> WorkflowGraph {
        serde_json::from_value(serde_json::json!({
            "version": 1,
            "nodes": [
                { "id": "t1", "kind": "trigger",
                  "config": { "type": "schedule", "cron": "* * * * * *" } },
                { "id": "a1", "kind": "action",
                  "config": { "type": "assistant_turn", "prompt": "run the report" } },
            ],
            "edges": [{ "from": "t1", "to": "a1" }],
            "execution": { "max_steps": 10, "max_visits_per_node": 2 }
        }))
        .unwrap()
    }

    async fn run_count(storage: &StorageLayer) -> usize {
        let store = storage.workflow_store();
        let mut runs = 0;
        for workflow in store.list().await.unwrap() {
            runs += store.list_runs(workflow.id, 50).await.unwrap().len();
        }
        runs
    }

    async fn conversation_count(storage: &StorageLayer) -> usize {
        SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .list_conversations()
            .await
            .unwrap()
            .len()
    }

    #[tokio::test]
    async fn a_scheduled_workflow_in_space_b_runs_on_b_and_never_touches_a() {
        let tmp = tempfile::tempdir().unwrap();
        let pools = Arc::new(OrgPoolFactory::new(tmp.path()));
        let org_storage = Arc::new(pools.org_storage().await.unwrap());
        let now = chrono::DateTime::UNIX_EPOCH;
        org_storage
            .org_store()
            .create_org(&Organization {
                id: OrgId::from("org_acme"),
                name: "acme".into(),
                slug: "acme".into(),
                auth_mode: "password".into(),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
        for slug in ["alpha", "beta"] {
            org_storage
                .space_store()
                .create_space(&Space {
                    id: SpaceId::from(format!("spc_{slug}")),
                    org_id: OrgId::from("org_acme"),
                    name: slug.into(),
                    slug: slug.into(),
                    created_at: now,
                    updated_at: now,
                })
                .await
                .unwrap();
        }
        let mut config = AssistantConfig::default();
        config.memory.enabled = false;
        let runtimes = Arc::new(
            SpaceRuntimes::new(
                pools.clone(),
                Arc::new(config),
                Arc::new(ScriptedLlmProvider::new()),
                Arc::new(AutoDenyConfirmation {
                    interface_name: "test",
                }),
            )
            .with_submit_timeout(20),
        );
        let alpha = runtimes.get("acme", "alpha").await.unwrap();
        let beta = runtimes.get("acme", "beta").await.unwrap();
        let worker = beta.orchestrator.clone();
        tokio::spawn(async move { worker.run_worker("beta-worker").await });
        beta.storage
            .workflow_store()
            .create("report", "", &scheduled_turn_graph(), true)
            .await
            .unwrap();

        let supervisor = Arc::new(SpaceSupervisor::new(
            org_storage,
            runtimes,
            workflow_spawner(Duration::from_millis(20)),
        ));
        supervisor.reconcile().await.unwrap();

        tokio::time::timeout(Duration::from_secs(15), async {
            while conversation_count(&beta.storage).await == 0 {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("the scheduled turn ran on beta's runtime");

        assert!(run_count(&beta.storage).await >= 1);
        assert_eq!(run_count(&alpha.storage).await, 0, "alpha has no runs");
        assert_eq!(
            conversation_count(&alpha.storage).await,
            0,
            "alpha's database is untouched"
        );
    }
}
