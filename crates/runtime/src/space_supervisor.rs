//! Keeps one set of background tasks running per space.
//!
//! A binary that owns background work (turn workers, the scheduler, the
//! workflow runner) hands the supervisor a spawner. The supervisor lists every
//! space in `org.db`, builds its [`SpaceRuntime`], and runs the spawner once
//! per space. A periodic [`reconcile`](SpaceSupervisor::reconcile) starts tasks
//! for new spaces and stops tasks, and drops the runtime, of deleted ones.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use assistant_core::identity::{OrgId, SpaceId};
use assistant_core::store::{OrgStore as _, SpaceStore as _};
use assistant_storage::OrgStorageLayer;

use crate::space_runtime::{SpaceRuntime, SpaceRuntimes};

/// A space known to `org.db` together with its runtime.
#[derive(Clone)]
pub struct SpaceRef {
    pub org_id: OrgId,
    pub space_id: SpaceId,
    pub runtime: Arc<SpaceRuntime>,
}

/// Starts the background tasks of one space.
pub type SpaceTaskSpawner = Arc<dyn Fn(SpaceRef) -> Vec<JoinHandle<()>> + Send + Sync>;

pub struct SpaceSupervisor {
    org_storage: Arc<OrgStorageLayer>,
    runtimes: Arc<SpaceRuntimes>,
    spawner: SpaceTaskSpawner,
    running: Mutex<HashMap<SpaceId, RunningSpace>>,
}

struct RunningSpace {
    org_slug: String,
    space_slug: String,
    tasks: Vec<JoinHandle<()>>,
}

impl SpaceSupervisor {
    pub fn new(
        org_storage: Arc<OrgStorageLayer>,
        runtimes: Arc<SpaceRuntimes>,
        spawner: SpaceTaskSpawner,
    ) -> Self {
        Self {
            org_storage,
            runtimes,
            spawner,
            running: Mutex::new(HashMap::new()),
        }
    }

    /// Ids of the spaces whose tasks are running, sorted.
    pub async fn running_spaces(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .running
            .lock()
            .await
            .keys()
            .map(ToString::to_string)
            .collect();
        ids.sort();
        ids
    }

    /// Bring the running set in line with `org.db`.
    pub async fn reconcile(&self) -> Result<()> {
        let mut present: HashMap<SpaceId, (OrgId, String, String)> = HashMap::new();
        for org in self.org_storage.org_store().list_orgs().await? {
            for space in self.org_storage.space_store().list_spaces(&org.id).await? {
                present.insert(space.id, (org.id.clone(), org.slug.clone(), space.slug));
            }
        }

        let mut running = self.running.lock().await;
        let gone: Vec<SpaceId> = running
            .keys()
            .filter(|id| !present.contains_key(*id))
            .cloned()
            .collect();
        for id in gone {
            if let Some(space) = running.remove(&id) {
                info!(space = %id, "space removed; stopping its tasks");
                for task in &space.tasks {
                    task.abort();
                }
                self.runtimes.evict(&space.org_slug, &space.space_slug);
            }
        }

        for (space_id, (org_id, org_slug, space_slug)) in present {
            if running.contains_key(&space_id) {
                continue;
            }
            let runtime = match self.runtimes.get(&org_slug, &space_slug).await {
                Ok(runtime) => runtime,
                Err(e) => {
                    warn!(space = %space_id, "cannot build space runtime: {e:#}");
                    continue;
                }
            };
            info!(org = %org_slug, space = %space_slug, "starting space tasks");
            let tasks = (self.spawner)(SpaceRef {
                org_id,
                space_id: space_id.clone(),
                runtime,
            });
            running.insert(
                space_id,
                RunningSpace {
                    org_slug,
                    space_slug,
                    tasks,
                },
            );
        }
        Ok(())
    }

    /// Reconcile now, then every `interval`.
    pub async fn run(self: Arc<Self>, interval: Duration) {
        loop {
            if let Err(e) = self.reconcile().await {
                warn!("space reconcile failed: {e:#}");
            }
            tokio::time::sleep(interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::org_context::OrgContexts;
    use crate::space_runtime::SpaceRuntimes;
    use assistant_core::identity::{OrgId, SpaceId};
    use assistant_core::store::{OrgStore as _, Organization, Space, SpaceStore as _};
    use assistant_core::types::agent::OrgConfig;
    use assistant_llm_provider::scripted::ScriptedLlmProvider;
    use assistant_storage::{OrgPoolFactory, OrgStorageLayer};

    use super::*;

    struct Fixture {
        _tmp: tempfile::TempDir,
        org_storage: Arc<OrgStorageLayer>,
        runtimes: Arc<SpaceRuntimes>,
    }

    impl Fixture {
        async fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let pools = Arc::new(OrgPoolFactory::new(tmp.path()));
            let org_storage = Arc::new(pools.org_storage().await.unwrap());
            let mut config = OrgConfig::default();
            config.memory.enabled = false;
            let runtimes = Arc::new(SpaceRuntimes::new(
                pools,
                Arc::new(OrgContexts::fixed(
                    config,
                    Arc::new(ScriptedLlmProvider::new()),
                )),
                None,
            ));
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
            Self {
                _tmp: tmp,
                org_storage,
                runtimes,
            }
        }

        async fn add_space(&self, slug: &str) {
            let now = chrono::DateTime::UNIX_EPOCH;
            self.org_storage
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

        fn supervisor(&self, started: Arc<Mutex<Vec<String>>>) -> SpaceSupervisor {
            SpaceSupervisor::new(
                self.org_storage.clone(),
                self.runtimes.clone(),
                Arc::new(move |space: SpaceRef| {
                    if let Ok(mut started) = started.lock() {
                        started.push(space.space_id.to_string());
                    }
                    vec![tokio::spawn(std::future::pending::<()>())]
                }),
            )
        }
    }

    #[tokio::test]
    async fn starts_one_task_set_per_space_and_only_once() {
        let fixture = Fixture::new().await;
        fixture.add_space("alpha").await;
        fixture.add_space("beta").await;
        let started = Arc::new(Mutex::new(Vec::new()));
        let supervisor = fixture.supervisor(started.clone());

        supervisor.reconcile().await.unwrap();
        supervisor.reconcile().await.unwrap();

        let mut started = started.lock().unwrap().clone();
        started.sort();
        assert_eq!(started, ["spc_alpha", "spc_beta"]);
    }

    #[tokio::test]
    async fn picks_up_spaces_created_later() {
        let fixture = Fixture::new().await;
        fixture.add_space("alpha").await;
        let started = Arc::new(Mutex::new(Vec::new()));
        let supervisor = fixture.supervisor(started.clone());
        supervisor.reconcile().await.unwrap();

        fixture.add_space("beta").await;
        supervisor.reconcile().await.unwrap();

        assert_eq!(started.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn stops_tasks_and_evicts_the_runtime_of_a_deleted_space() {
        let fixture = Fixture::new().await;
        fixture.add_space("alpha").await;
        let supervisor = fixture.supervisor(Arc::new(Mutex::new(Vec::new())));
        supervisor.reconcile().await.unwrap();
        assert_eq!(supervisor.running_spaces().await, ["spc_alpha"]);
        assert_eq!(fixture.runtimes.loaded().len(), 1);

        fixture
            .org_storage
            .space_store()
            .delete_space(&SpaceId::from("spc_alpha"))
            .await
            .unwrap();
        supervisor.reconcile().await.unwrap();

        assert!(supervisor.running_spaces().await.is_empty());
        assert!(fixture.runtimes.loaded().is_empty());
    }
}
