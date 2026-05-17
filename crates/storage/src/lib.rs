//! Storage layer: SQLite-backed implementations of `assistant-core` traits.
//!
//! # Dependency boundary
//!
//! This crate depends only on `assistant-core` (and `assistant-skills` for
//! the skill registry helpers) in production. It must NOT depend on
//! `assistant-auth` or `assistant-backup`:
//!
//! - All auth-domain trait contracts (`ApiKeyStore`, `ClientStore`,
//!   `AuthCodeStore`, `RefreshTokenStore`, `DeviceCodeStore`) live in
//!   [`assistant_core::auth`]; the SQLite-backed implementations here
//!   simply implement them.
//! - The legacy → multi-org migration (see [`migration`]) is intentionally
//!   schema-shaped: filesystem layout, SQLite copy, and seeding the
//!   default org/space. Pre-migration backup creation lives in
//!   `assistant-backup` (`backup_legacy_install`) and the initial admin
//!   user is bootstrapped by `assistant-auth` (`bootstrap::create_admin_user`).
//!   Production callers (e.g. `assistant-web-ui`) compose these four steps.
//!
//! `assistant-auth` and `assistant-backup` are present as `[dev-dependencies]`
//! so the round-trip migration test in [`migration`] can exercise the same
//! composition the production caller uses. The `dep_boundary` integration
//! test enforces this boundary at compile time.
//!
//! The panic-free contract for non-test code is enforced via
//! `[lints.clippy]` overrides in this crate's `Cargo.toml` (managed by the
//! workspace-lint-policy change).

pub mod agents;
pub mod api_key_store;
pub mod attachments;
pub mod auth_state_store;
pub mod binding_store;
pub mod catalog_item_store;
pub mod command_events;
pub mod conversation_broadcaster;
pub mod conversation_events;
pub mod conversations;
pub mod interface_instance_store;
pub mod logs;
pub mod memory_chunks;
pub mod message_bus;
pub mod metrics;
pub mod migration;
pub mod org_storage;
pub mod org_store;
pub mod persona_skill_access;
pub mod personas;
pub mod pool_factory;
pub mod push_subscriptions;
pub mod refinements;
pub mod registry;
pub mod scheduled_tasks;
pub mod slack_threads;
pub mod space_store;
pub mod traces;
pub mod user_store;
pub mod webhooks;
pub mod workflows;

pub use agents::{AgentRecord, AgentStatus, AgentStore};
pub use api_key_store::SqliteApiKeyStore;
pub use attachments::AttachmentStore;
pub use auth_state_store::{
    SqliteAuthCodeStore, SqliteClientStore, SqliteDeviceCodeStore, SqliteRefreshTokenStore,
};
pub use binding_store::SqliteBindingStore;
pub use catalog_item_store::{SqliteCatalogItemStore, SqliteCatalogSubscriptionStore};
pub use command_events::{CommandEventRow, CommandEventStore};
pub use conversation_broadcaster::{
    ConversationBroadcast, ConversationEvent, InMemoryConversationBroadcaster,
};
pub use conversation_events::{
    ConversationEventRow, ConversationEventStore, LiveEvent, RunBroadcaster,
};
pub use conversations::{
    ConversationRecord, ConversationStore, InMemoryConversationStore, SqliteConversationStore,
};
pub use interface_instance_store::SqliteInterfaceInstanceStore;
pub use logs::{LogStats, LogStore, RecordedLog};
pub use memory_chunks::{FtsMatch, MemoryChunkStore, StoredChunk};
pub use message_bus::SqliteMessageBus;
pub use metrics::{
    MetricsStore, MetricsSummary, ModelTokenUsage, ResourceRecord, TimeSeriesPoint, ToolUsageStats,
};
pub use org_storage::OrgStorageLayer;
pub use org_store::SqliteOrgStore;
pub use persona_skill_access::PersonaSkillAccessStore;
pub use personas::{PersonaRecord, PersonaStore};
pub use pool_factory::OrgPoolFactory;
pub use push_subscriptions::{PushSubscription, PushSubscriptionStore};
pub use refinements::{RefinementStatus, RefinementsStore, SkillRefinement};
pub use registry::SkillRegistry;
pub use scheduled_tasks::{ScheduledTask, ScheduledTaskStore};
pub use slack_threads::SlackActiveThreadStore;
pub use space_store::{SqliteMembershipStore, SqliteSpaceStore};
pub use traces::{
    RecordedSpan, SkillStatsProvider, TraceFilter, TraceStats, TraceStore, TraceSummary,
};
pub use user_store::SqliteUserStore;
pub use webhooks::{WebhookRecord, WebhookStore};
pub use workflows::{
    WorkflowEdge, WorkflowEdgeCondition, WorkflowExecutionLimits, WorkflowGraph, WorkflowNode,
    WorkflowNodeKind, WorkflowRecord, WorkflowRunRecord, WorkflowRunStepRecord, WorkflowStore,
    WorkflowTriggerKind, WorkflowWebhookEndpoint,
};

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;
use sqlx::SqlitePool;
use tracing::info;

/// The top-level storage layer — owns the SQLite connection pool and runs migrations.
pub struct StorageLayer {
    pub pool: SqlitePool,
}

impl StorageLayer {
    /// Open (or create) a SQLite database at `db_path`, running all embedded migrations.
    pub async fn new(db_path: &Path) -> Result<Self> {
        // Ensure parent directories exist
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        info!("Opening SQLite database at {}", db_path.display());

        let pool = SqlitePool::connect(&url).await?;
        run_migrations(&pool).await?;

        Ok(Self { pool })
    }

    /// Create an in-memory SQLite database (useful for tests).
    ///
    /// Create an in-memory storage layer for tests.
    ///
    /// Uses a named shared-cache URI (`file:memdbN?mode=memory&cache=shared`)
    /// so that multiple pool connections see the same in-memory database.
    /// This avoids data loss when a connection is invalidated (e.g. after a
    /// `tokio::task::abort`) and the pool opens a replacement.
    pub async fn new_in_memory() -> Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let uri = format!("sqlite:file:memdb{id}?mode=memory&cache=shared");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect(&uri)
            .await?;
        run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    /// Convenience: build a `TraceStore` backed by this pool.
    pub fn trace_store(&self) -> TraceStore {
        TraceStore::new(self.pool.clone())
    }

    /// Convenience: build a [`ConversationEventStore`] backed by this pool.
    pub fn conversation_event_store(&self) -> ConversationEventStore {
        ConversationEventStore::new(self.pool.clone())
    }

    /// Convenience: build a `LogStore` backed by this pool.
    pub fn log_store(&self) -> LogStore {
        LogStore::new(self.pool.clone())
    }

    /// Convenience: build a `SqliteConversationStore` backed by this pool.
    pub fn conversation_store(&self) -> SqliteConversationStore {
        SqliteConversationStore::new(self.pool.clone())
    }

    /// Convenience: build a `SqliteConversationStore` scoped to a Persona.
    pub fn conversation_store_for_agent(&self, agent_id: &str) -> SqliteConversationStore {
        SqliteConversationStore::for_agent(self.pool.clone(), agent_id)
    }

    /// Convenience: build a `PersonaStore` backed by this pool.
    pub fn persona_store(&self) -> PersonaStore {
        PersonaStore::new(self.pool.clone())
    }

    /// Convenience: build a `RefinementsStore` backed by this pool.
    pub fn refinements_store(&self) -> RefinementsStore {
        RefinementsStore::new(self.pool.clone())
    }

    /// Convenience: build a `ScheduledTaskStore` backed by this pool.
    pub fn scheduled_task_store(&self) -> ScheduledTaskStore {
        ScheduledTaskStore::new(self.pool.clone())
    }

    /// Convenience: build a `ScheduledTaskStore` scoped to a Persona.
    pub fn scheduled_task_store_for_agent(&self, agent_id: &str) -> ScheduledTaskStore {
        ScheduledTaskStore::for_agent(self.pool.clone(), agent_id)
    }

    /// Convenience: build a `MemoryChunkStore` backed by this pool.
    pub fn memory_chunks_store(&self) -> MemoryChunkStore {
        MemoryChunkStore::new(self.pool.clone())
    }

    /// Convenience: build an [`AgentStore`] backed by this pool.
    pub fn agent_store(&self) -> AgentStore {
        AgentStore::new(self.pool.clone())
    }

    /// Convenience: build a [`PersonaSkillAccessStore`] backed by this pool.
    pub fn persona_skill_access_store(&self) -> PersonaSkillAccessStore {
        PersonaSkillAccessStore::new(self.pool.clone())
    }

    /// Convenience: build a [`SqliteMessageBus`] backed by this pool.
    pub fn message_bus(&self) -> SqliteMessageBus {
        SqliteMessageBus::new(self.pool.clone())
    }

    /// Convenience: build a [`MetricsStore`] backed by this pool.
    pub fn metrics_store(&self) -> MetricsStore {
        MetricsStore::new(self.pool.clone())
    }

    /// Convenience: build a [`MetricsStore`] scoped to a Persona.
    pub fn metrics_store_for_agent(&self, agent_id: &str) -> MetricsStore {
        MetricsStore::for_agent(self.pool.clone(), agent_id)
    }

    /// Convenience: build a [`WebhookStore`] backed by this pool.
    pub fn webhook_store(&self) -> WebhookStore {
        WebhookStore::new(self.pool.clone())
    }

    /// Convenience: build a [`PushSubscriptionStore`] backed by this pool.
    pub fn push_subscription_store(&self) -> PushSubscriptionStore {
        PushSubscriptionStore::new(self.pool.clone())
    }

    /// Convenience: build a [`WorkflowStore`] backed by this pool.
    pub fn workflow_store(&self) -> WorkflowStore {
        WorkflowStore::new(self.pool.clone())
    }

    /// Convenience: build a [`SlackActiveThreadStore`] backed by this pool.
    pub fn slack_active_thread_store(&self) -> SlackActiveThreadStore {
        SlackActiveThreadStore::new(self.pool.clone())
    }

    /// Convenience: build an [`AttachmentStore`] backed by this pool.
    pub fn attachment_store(&self) -> AttachmentStore {
        AttachmentStore::new(self.pool.clone())
    }

    /// Convenience: build a [`WebhookStore`] scoped to a Persona.
    pub fn webhook_store_for_agent(&self, agent_id: &str) -> WebhookStore {
        WebhookStore::for_agent(self.pool.clone(), agent_id)
    }
}

/// Returns the default database path: `~/.assistant/assistant.db`.
pub fn default_db_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".assistant").join("assistant.db"))
}

/// Run all embedded migrations in order, tracking applied migrations so each
/// runs exactly once.
///
/// A `_migrations` table records which migrations have been applied.
/// Each migration is only executed if it has not yet been recorded, preventing
/// non-idempotent statements (e.g. `ALTER TABLE ADD COLUMN`) from failing on
/// subsequent launches.
async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys=ON;").execute(pool).await?;

    // Wait up to 5 s when the database is locked by another connection instead
    // of returning SQLITE_BUSY immediately.  This is critical because the OTel
    // exporters, the orchestrator, and the message bus all share the same pool
    // and contend for the single SQLite write lock.
    sqlx::query("PRAGMA busy_timeout = 5000;")
        .execute(pool)
        .await?;

    // Migration tracking table — created once, never dropped.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name        TEXT PRIMARY KEY,
            applied_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(pool)
    .await?;

    let migrations: &[(&str, &str)] = &[
        (
            "001_conversations",
            include_str!("../../../migrations/001_conversations.sql"),
        ),
        (
            "002_skills",
            include_str!("../../../migrations/002_skills.sql"),
        ),
        (
            "003_execution_traces",
            include_str!("../../../migrations/003_execution_traces.sql"),
        ),
        (
            "004_memory",
            include_str!("../../../migrations/004_memory.sql"),
        ),
        (
            "005_tool_calls",
            include_str!("../../../migrations/005_tool_calls.sql"),
        ),
        (
            "006_drop_memory_entries",
            include_str!("../../../migrations/006_drop_memory_entries.sql"),
        ),
        (
            "007_memory_chunks",
            include_str!("../../../migrations/007_memory_chunks.sql"),
        ),
        (
            "008_skills_drop_tier_check",
            include_str!("../../../migrations/008_skills_drop_tier_check.sql"),
        ),
        (
            "009_distributed_traces",
            include_str!("../../../migrations/009_distributed_traces.sql"),
        ),
        (
            "010_trace_token_usage",
            include_str!("../../../migrations/010_trace_token_usage.sql"),
        ),
        ("011_logs", include_str!("../../../migrations/011_logs.sql")),
        (
            "012_scheduled_tasks_once",
            include_str!("../../../migrations/012_scheduled_tasks_once.sql"),
        ),
        (
            "013_bus_messages",
            include_str!("../../../migrations/013_bus_messages.sql"),
        ),
        (
            "014_agents",
            include_str!("../../../migrations/014_agents.sql"),
        ),
        (
            "015_metrics",
            include_str!("../../../migrations/015_metrics.sql"),
        ),
        (
            "016_webhooks",
            include_str!("../../../migrations/016_webhooks.sql"),
        ),
        (
            "017_assistant_agents",
            include_str!("../../../migrations/017_assistant_agents.sql"),
        ),
        (
            "018_agent_scope_tasks_webhooks",
            include_str!("../../../migrations/018_agent_scope_tasks_webhooks.sql"),
        ),
        (
            "019_metrics_agent_scope",
            include_str!("../../../migrations/019_metrics_agent_scope.sql"),
        ),
        (
            "020_workflows",
            include_str!("../../../migrations/020_workflows.sql"),
        ),
        (
            "021_workflow_webhook_endpoints",
            include_str!("../../../migrations/021_workflow_webhook_endpoints.sql"),
        ),
        (
            "022_workflow_run_steps",
            include_str!("../../../migrations/022_workflow_run_steps.sql"),
        ),
        (
            "023_workflow_run_step_output",
            include_str!("../../../migrations/023_workflow_run_step_output.sql"),
        ),
        (
            "024_service_name_on_telemetry",
            include_str!("../../../migrations/024_service_name_on_telemetry.sql"),
        ),
        (
            "025_distributed_traces_drop_conversation_fk",
            include_str!("../../../migrations/025_distributed_traces_drop_conversation_fk.sql"),
        ),
        (
            "026_personas",
            include_str!("../../../migrations/026_personas.sql"),
        ),
        (
            "027_slack_active_threads",
            include_str!("../../../migrations/027_slack_active_threads.sql"),
        ),
        (
            "028_skill_body",
            include_str!("../../../migrations/028_skill_body.sql"),
        ),
        (
            "029_persona_skill_access",
            include_str!("../../../migrations/029_persona_skill_access.sql"),
        ),
        (
            "030_persona_turn_timeout",
            include_str!("../../../migrations/030_persona_turn_timeout.sql"),
        ),
        (
            "031_push_subscriptions",
            include_str!("../../../migrations/031_push_subscriptions.sql"),
        ),
        (
            "032_persona_home_channel",
            include_str!("../../../migrations/032_persona_home_channel.sql"),
        ),
        (
            "033_conversation_events",
            include_str!("../../../migrations/033_conversation_events.sql"),
        ),
        (
            "034_message_attachments",
            include_str!("../../../migrations/034_message_attachments.sql"),
        ),
        (
            "035_command_events",
            include_str!("../../../migrations/035_command_events.sql"),
        ),
        (
            "036_learning_active_skill",
            include_str!("../../../migrations/036_learning_active_skill.sql"),
        ),
        (
            "037_learning_refinement_revert",
            include_str!("../../../migrations/037_learning_refinement_revert.sql"),
        ),
        (
            "038_conversation_user_id",
            include_str!("../../../migrations/038_conversation_user_id.sql"),
        ),
        (
            "039_persona_owner",
            include_str!("../../../migrations/039_persona_owner.sql"),
        ),
        (
            "040_message_sender",
            include_str!("../../../migrations/040_message_sender.sql"),
        ),
        (
            "041_conversation_title_locked",
            include_str!("../../../migrations/041_conversation_title_locked.sql"),
        ),
    ];

    for (name, sql) in migrations {
        let already_applied: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM _migrations WHERE name = ?")
                .bind(name)
                .fetch_one(pool)
                .await?;

        if already_applied == 0 {
            sqlx::raw_sql(sql).execute(pool).await?;
            sqlx::query("INSERT INTO _migrations (name) VALUES (?)")
                .bind(name)
                .execute(pool)
                .await?;
            info!(migration = %name, "Applied migration");
        }
    }

    info!("Database migrations applied successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use anyhow::Result;
    use sqlx::{Row, SqlitePool};
    use uuid::Uuid;

    use super::StorageLayer;

    #[tokio::test]
    async fn upgrade_from_pre_agent_scope_db_succeeds() -> Result<()> {
        let db_path = temp_db_path();
        seed_database_through_016(&db_path).await?;

        let storage = StorageLayer::new(&db_path).await?;
        let pool = &storage.pool;

        assert!(column_exists(pool, "conversations", "agent_id").await?);
        assert!(column_exists(pool, "scheduled_tasks", "agent_id").await?);
        assert!(column_exists(pool, "webhooks", "agent_id").await?);
        assert!(column_exists(pool, "metric_points", "agent_id").await?);

        let applied: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM _migrations WHERE name IN ('017_assistant_agents', '018_agent_scope_tasks_webhooks', '019_metrics_agent_scope', '026_personas')",
        )
        .fetch_one(pool)
        .await?;
        assert_eq!(applied, 4, "new persona-scope migrations should be applied");

        let table_exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='personas'",
        )
        .fetch_one(pool)
        .await?;
        assert_eq!(table_exists, 1, "personas table should exist");

        let _ = tokio::fs::remove_file(&db_path).await;
        Ok(())
    }

    async fn seed_database_through_016(db_path: &Path) -> Result<()> {
        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        let pool = SqlitePool::connect(&url).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _migrations (
                name        TEXT PRIMARY KEY,
                applied_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await?;

        let seed_migrations: &[(&str, &str)] = &[
            (
                "001_conversations",
                include_str!("../../../migrations/001_conversations.sql"),
            ),
            (
                "002_skills",
                include_str!("../../../migrations/002_skills.sql"),
            ),
            (
                "003_execution_traces",
                include_str!("../../../migrations/003_execution_traces.sql"),
            ),
            (
                "004_memory",
                include_str!("../../../migrations/004_memory.sql"),
            ),
            (
                "005_tool_calls",
                include_str!("../../../migrations/005_tool_calls.sql"),
            ),
            (
                "006_drop_memory_entries",
                include_str!("../../../migrations/006_drop_memory_entries.sql"),
            ),
            (
                "007_memory_chunks",
                include_str!("../../../migrations/007_memory_chunks.sql"),
            ),
            (
                "008_skills_drop_tier_check",
                include_str!("../../../migrations/008_skills_drop_tier_check.sql"),
            ),
            (
                "009_distributed_traces",
                include_str!("../../../migrations/009_distributed_traces.sql"),
            ),
            (
                "010_trace_token_usage",
                include_str!("../../../migrations/010_trace_token_usage.sql"),
            ),
            ("011_logs", include_str!("../../../migrations/011_logs.sql")),
            (
                "012_scheduled_tasks_once",
                include_str!("../../../migrations/012_scheduled_tasks_once.sql"),
            ),
            (
                "013_bus_messages",
                include_str!("../../../migrations/013_bus_messages.sql"),
            ),
            (
                "014_agents",
                include_str!("../../../migrations/014_agents.sql"),
            ),
            (
                "015_metrics",
                include_str!("../../../migrations/015_metrics.sql"),
            ),
            (
                "016_webhooks",
                include_str!("../../../migrations/016_webhooks.sql"),
            ),
        ];

        for (name, sql) in seed_migrations {
            sqlx::raw_sql(sql).execute(&pool).await?;
            sqlx::query("INSERT INTO _migrations(name) VALUES (?)")
                .bind(name)
                .execute(&pool)
                .await?;
        }

        Ok(())
    }

    async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> Result<bool> {
        let query = format!("PRAGMA table_info({table})");
        let rows = sqlx::query(&query).fetch_all(pool).await?;
        Ok(rows.iter().any(|row| {
            row.try_get::<String, _>("name")
                .is_ok_and(|name| name == column)
        }))
    }

    fn temp_db_path() -> PathBuf {
        std::env::temp_dir().join(format!("assistant-storage-migration-{}.db", Uuid::new_v4()))
    }
}
