//! The background tasks the orchestrator service runs for each space: turn
//! workers, title generation, the memory indexer and the task scheduler.
//!
//! Which workers a binary role runs is decided by
//! [`assistant_runtime::core_worker_plan`]; this module starts that plan once
//! per space, against that space's runtime and database.

use std::sync::Arc;
use std::time::Duration;

use assistant_runtime::{
    CoreWorkerPlan, SpaceRef, SpaceTaskSpawner, spawn_memory_indexer, spawn_scheduler,
    spawn_title_generator_worker,
};

/// A spawner that runs `plan` for every space it is handed, plus the task
/// scheduler when `scheduler_poll` is set.
pub fn space_task_spawner(
    plan: CoreWorkerPlan,
    scheduler_poll: Option<Duration>,
) -> SpaceTaskSpawner {
    Arc::new(move |space: SpaceRef| {
        let runtime = space.runtime;
        let config = runtime.config.clone();
        let mut tasks = Vec::new();

        for spec in &plan.workers {
            let orchestrator = runtime.orchestrator.clone();
            let (id, filter) = (spec.worker_id, spec.interface_filter);
            tasks.push(tokio::spawn(async move {
                orchestrator.run_worker_filtered(id, filter).await;
            }));
        }
        if let Some(poll) = scheduler_poll {
            tasks.push(spawn_scheduler(
                runtime.storage.clone(),
                runtime.orchestrator.clone(),
                poll,
            ));
        }
        if plan.spawn_memory_indexer {
            tasks.push(spawn_memory_indexer(
                &config.memory,
                runtime.storage.clone(),
                runtime.orchestrator.llm.clone(),
            ));
        }
        if plan.spawn_title_generator {
            tasks.push(spawn_title_generator_worker(
                runtime.bus.clone(),
                runtime.storage.clone(),
                runtime.orchestrator.llm.clone(),
                config.titling.clone(),
                config.agent.id.clone(),
            ));
        }
        tasks
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use uuid::Uuid;

    use assistant_core::auth::AuthContext;
    use assistant_core::identity::{OrgId, SpaceId};
    use assistant_core::store::{OrgStore as _, Organization, Space, SpaceStore as _};
    use assistant_core::types::agent::OrgConfig;
    use assistant_core::types::conversation::Interface;
    use assistant_llm_provider::scripted::ScriptedLlmProvider;
    use assistant_runtime::bootstrap::AutoDenyConfirmation;
    use assistant_runtime::{
        BinaryRole, OrgContexts, SpaceRuntimes, SpaceSupervisor, core_worker_plan,
    };
    use assistant_storage::{ConversationStore as _, OrgPoolFactory, SqliteConversationStore};

    use super::*;

    /// Upper bound only: the turn normally completes in well under a second.
    /// The test waits on the turn result, never on this deadline.
    const TURN_DEADLINE: Duration = Duration::from_secs(30);

    #[tokio::test]
    async fn the_orchestrator_service_works_every_spaces_turn_queue() {
        let tmp = tempfile::tempdir().unwrap();
        let pools = Arc::new(OrgPoolFactory::new(tmp.path()));
        let org_storage = Arc::new(pools.org_storage().await.unwrap());
        let epoch = chrono::DateTime::UNIX_EPOCH;
        org_storage
            .org_store()
            .create_org(&Organization {
                id: OrgId::from("org_acme"),
                name: "acme".into(),
                slug: "acme".into(),
                auth_mode: "password".into(),
                created_at: epoch,
                updated_at: epoch,
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
                    created_at: epoch,
                    updated_at: epoch,
                })
                .await
                .unwrap();
        }
        let mut config = OrgConfig::default();
        config.memory.enabled = false;
        let runtimes = Arc::new(
            SpaceRuntimes::new(
                pools,
                Arc::new(OrgContexts::fixed(
                    config,
                    Arc::new(ScriptedLlmProvider::new()),
                )),
                Arc::new(AutoDenyConfirmation {
                    interface_name: "test",
                }),
            )
            .with_submit_timeout(TURN_DEADLINE.as_secs()),
        );
        let supervisor = SpaceSupervisor::new(
            org_storage,
            runtimes.clone(),
            space_task_spawner(core_worker_plan(BinaryRole::OrchestratorUnfiltered), None),
        );
        supervisor.reconcile().await.unwrap();

        let alpha = runtimes.get("acme", "alpha").await.unwrap();
        let beta = runtimes.get("acme", "beta").await.unwrap();
        let conversation = Uuid::new_v4();
        tokio::time::timeout(
            TURN_DEADLINE,
            beta.orchestrator.submit_turn(
                &AuthContext::system(),
                "hello",
                conversation,
                Interface::Web,
                None,
            ),
        )
        .await
        .expect("a worker for beta claims the turn")
        .unwrap();

        let history = |storage: &Arc<assistant_storage::StorageLayer>| {
            SqliteConversationStore::for_agent(storage.pool.clone(), "default")
        };
        assert!(
            !history(&beta.storage)
                .load_history(conversation)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            history(&alpha.storage)
                .list_conversations()
                .await
                .unwrap()
                .is_empty(),
            "alpha's database is untouched"
        );
    }
}
