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
pub mod conversation_search;
pub mod conversations;
pub mod integration_store;
pub mod interface_instance_store;
pub mod memory_chunks;
pub mod message_bus;
pub mod message_feedback;
pub mod message_timings;
pub mod org_oidc_config_store;
pub mod org_storage;
pub mod org_store;
pub mod persona_skill_access;
pub mod personas;
pub mod pool_factory;
pub mod push_subscriptions;
pub mod refinements;
pub mod registry;
pub mod scheduled_tasks;
pub mod scoped_bus;
pub mod skill_executions;
pub mod skill_stats;
pub mod slack_threads;
pub mod space_store;
pub mod surfaces;
pub mod tool_permission_rules;
pub mod user_store;
pub mod webhooks;
pub mod workflows;

pub use agents::{AgentRecord, AgentStatus, AgentStore, InMemoryAgentStore, SqliteAgentStore};
pub use api_key_store::SqliteApiKeyStore;
pub use attachments::{AttachmentStore, InMemoryAttachmentStore, SqliteAttachmentStore};
pub use auth_state_store::{
    SqliteAuthCodeStore, SqliteClientStore, SqliteDeviceCodeStore, SqliteRefreshTokenStore,
};
pub use binding_store::SqliteBindingStore;
pub use catalog_item_store::{SqliteCatalogItemStore, SqliteCatalogSubscriptionStore};
pub use command_events::{
    CommandEventRow, CommandEventStore, InMemoryCommandEventStore, SqliteCommandEventStore,
};
pub use conversation_broadcaster::{
    ConversationBroadcast, ConversationEvent, InMemoryConversationBroadcaster,
};
pub use conversation_events::{
    ConversationEventRow, ConversationEventStore, InMemoryConversationEventStore, LiveEvent,
    RunBroadcaster, SqliteConversationEventStore,
};
pub use conversations::{
    ConversationPatch, ConversationRecord, ConversationStore, InMemoryConversationStore,
    MetadataUpdate, SqliteConversationStore,
};
pub use integration_store::{
    SqliteIntegrationCredentialStore, SqliteIntegrationHealthStore, SqliteIntegrationStore,
};
pub use interface_instance_store::SqliteInterfaceInstanceStore;
pub use memory_chunks::{
    FtsMatch, InMemoryMemoryChunkStore, MemoryChunkStore, SqliteMemoryChunkStore, StoredChunk,
};
pub use message_bus::{InMemoryMessageBus, SqliteMessageBus};
pub use message_feedback::{
    InMemoryMessageFeedbackStore, MessageFeedbackRating, MessageFeedbackRow, MessageFeedbackStore,
    SqliteMessageFeedbackStore,
};
pub use message_timings::{
    InMemoryMessageTimingStore, MessageTimingStore, SqliteMessageTimingStore,
};
pub use org_oidc_config_store::SqliteOrgOidcConfigStore;
pub use org_storage::OrgStorageLayer;
pub use org_store::SqliteOrgStore;
pub use persona_skill_access::{
    InMemoryPersonaSkillAccessStore, PersonaSkillAccessStore, SqlitePersonaSkillAccessStore,
};
pub use personas::{
    HomeChannel, InMemoryPersonaStore, PersonaRecord, PersonaStore, SqlitePersonaStore,
};
pub use pool_factory::{DEFAULT_ORG_SLUG, DEFAULT_SPACE_SLUG, OrgPoolFactory};
pub use push_subscriptions::{
    InMemoryPushSubscriptionStore, PushSubscription, PushSubscriptionStore,
    SqlitePushSubscriptionStore,
};
pub use refinements::{
    InMemoryRefinementsStore, RefinementStatus, RefinementsStore, SkillRefinement,
    SqliteRefinementsStore,
};
pub use registry::{InMemorySkillCatalog, SkillCatalog, SkillRegistry};
pub use scheduled_tasks::{
    InMemoryScheduledTaskStore, ScheduledTask, ScheduledTaskStore, SqliteScheduledTaskStore,
};
pub use scoped_bus::ScopedMessageBus;
pub use skill_executions::{
    InMemorySkillExecutionStore, SkillExecution, SkillExecutionStore, SkillOutcome,
    SqliteSkillExecutionStore,
};
pub use skill_stats::{SkillStats, SkillStatsProvider};
pub use slack_threads::{
    InMemorySlackActiveThreadStore, SlackActiveThreadStore, SqliteSlackActiveThreadStore,
};
pub use space_store::{SqliteMembershipStore, SqliteSpaceStore};
pub use surfaces::{
    InMemorySurfaceStore, NewSurface, SqliteSurfaceStore, Surface, SurfaceChanges, SurfaceStatus,
    SurfaceStore, Transition,
};
pub use tool_permission_rules::{
    InMemoryToolPermissionRuleStore, RuleEffect, SqliteToolPermissionRuleStore, ToolPermissionRule,
    ToolPermissionRuleStore,
};
pub use user_store::SqliteUserStore;
pub use webhooks::{InMemoryWebhookStore, SqliteWebhookStore, WebhookRecord, WebhookStore};
pub use workflows::{
    InMemoryWorkflowStore, SqliteWorkflowStore, WorkflowEdge, WorkflowEdgeCondition,
    WorkflowExecutionLimits, WorkflowGraph, WorkflowNode, WorkflowNodeKind, WorkflowRecord,
    WorkflowRunRecord, WorkflowRunStepRecord, WorkflowStore, WorkflowTriggerKind,
    WorkflowWebhookEndpoint,
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

    /// Convenience: build a [`ConversationEventStore`] backed by this pool.
    pub fn conversation_event_store(&self) -> SqliteConversationEventStore {
        SqliteConversationEventStore::new(self.pool.clone())
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
    pub fn persona_store(&self) -> SqlitePersonaStore {
        SqlitePersonaStore::new(self.pool.clone())
    }

    /// Convenience: build a `RefinementsStore` backed by this pool.
    pub fn refinements_store(&self) -> SqliteRefinementsStore {
        SqliteRefinementsStore::new(self.pool.clone())
    }

    /// Convenience: build a `SurfaceStore` backed by this pool.
    pub fn surface_store(&self) -> SqliteSurfaceStore {
        SqliteSurfaceStore::new(self.pool.clone())
    }

    /// Convenience: build a `SkillExecutionStore` backed by this pool.
    pub fn skill_execution_store(&self) -> SqliteSkillExecutionStore {
        SqliteSkillExecutionStore::new(self.pool.clone())
    }

    /// Convenience: build a `MessageTimingStore` backed by this pool.
    pub fn message_timing_store(&self) -> SqliteMessageTimingStore {
        SqliteMessageTimingStore::new(self.pool.clone())
    }

    /// Convenience: build a `ToolPermissionRuleStore` backed by this pool.
    pub fn tool_permission_rule_store(&self) -> SqliteToolPermissionRuleStore {
        SqliteToolPermissionRuleStore::new(self.pool.clone())
    }

    /// Convenience: build a `ScheduledTaskStore` backed by this pool.
    pub fn scheduled_task_store(&self) -> SqliteScheduledTaskStore {
        SqliteScheduledTaskStore::new(self.pool.clone())
    }

    /// Convenience: build a `ScheduledTaskStore` scoped to a Persona.
    pub fn scheduled_task_store_for_agent(&self, agent_id: &str) -> SqliteScheduledTaskStore {
        SqliteScheduledTaskStore::for_agent(self.pool.clone(), agent_id)
    }

    /// Convenience: build a `MemoryChunkStore` backed by this pool.
    pub fn memory_chunks_store(&self) -> SqliteMemoryChunkStore {
        SqliteMemoryChunkStore::new(self.pool.clone())
    }

    /// Convenience: build an [`AgentStore`] backed by this pool.
    pub fn agent_store(&self) -> SqliteAgentStore {
        SqliteAgentStore::new(self.pool.clone())
    }

    /// Convenience: build a [`PersonaSkillAccessStore`] backed by this pool.
    pub fn persona_skill_access_store(&self) -> SqlitePersonaSkillAccessStore {
        SqlitePersonaSkillAccessStore::new(self.pool.clone())
    }

    /// Convenience: build a [`SqliteMessageBus`] backed by this pool.
    pub fn message_bus(&self) -> SqliteMessageBus {
        SqliteMessageBus::new(self.pool.clone())
    }

    /// Convenience: build a [`WebhookStore`] backed by this pool.
    pub fn webhook_store(&self) -> SqliteWebhookStore {
        SqliteWebhookStore::new(self.pool.clone())
    }

    /// Convenience: build a [`PushSubscriptionStore`] backed by this pool.
    pub fn push_subscription_store(&self) -> SqlitePushSubscriptionStore {
        SqlitePushSubscriptionStore::new(self.pool.clone())
    }

    /// Convenience: build a [`WorkflowStore`] backed by this pool.
    pub fn workflow_store(&self) -> SqliteWorkflowStore {
        SqliteWorkflowStore::new(self.pool.clone())
    }

    /// Convenience: build a [`SlackActiveThreadStore`] backed by this pool.
    pub fn slack_active_thread_store(&self) -> SqliteSlackActiveThreadStore {
        SqliteSlackActiveThreadStore::new(self.pool.clone())
    }

    /// Convenience: build a [`SqliteAttachmentStore`] backed by this pool.
    pub fn attachment_store(&self) -> SqliteAttachmentStore {
        SqliteAttachmentStore::new(self.pool.clone())
    }

    /// Convenience: build a [`MessageFeedbackStore`] backed by this pool.
    pub fn message_feedback_store(&self) -> SqliteMessageFeedbackStore {
        SqliteMessageFeedbackStore::new(self.pool.clone())
    }

    /// Convenience: build a [`WebhookStore`] scoped to a Persona.
    pub fn webhook_store_for_agent(&self, agent_id: &str) -> SqliteWebhookStore {
        SqliteWebhookStore::for_agent(self.pool.clone(), agent_id)
    }
}

/// Returns the default database path: `~/.assistant/assistant.db`.
pub fn default_db_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".assistant").join("assistant.db"))
}

/// Every space.db migration as `(name, sql)`, sorted by file name.
///
/// Generated by `build.rs` from the top-level `migrations/*.sql` files; adding
/// a migration means adding a file, not editing this list. The name is the
/// file stem and is what `_migrations` tracks, so it must never change.
const SPACE_MIGRATIONS: &[(&str, &str)] =
    include!(concat!(env!("OUT_DIR"), "/space_migrations.rs"));

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

    for &(name, sql) in SPACE_MIGRATIONS {
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

    use super::{SPACE_MIGRATIONS, StorageLayer};

    /// Every `migrations/*.sql` file (top level only) is registered, in file
    /// name order, under its file stem. Adding a migration is adding a file.
    #[test]
    fn space_migrations_match_sql_files_on_disk() -> Result<()> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
        let mut on_disk: Vec<String> = std::fs::read_dir(&dir)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "sql"))
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(str::to_owned)
            })
            .collect();
        on_disk.sort();

        let registered: Vec<String> = SPACE_MIGRATIONS
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .collect();
        assert_eq!(
            registered, on_disk,
            "registered names must equal sorted files"
        );
        assert!(
            SPACE_MIGRATIONS
                .iter()
                .all(|(_, sql)| !sql.trim().is_empty()),
            "every migration must embed non-empty SQL"
        );
        Ok(())
    }

    /// Telemetry is emitted over OTLP only; the tables the SQLite exporter
    /// wrote to must not exist after migration.
    #[tokio::test]
    async fn local_telemetry_tables_are_dropped() -> Result<()> {
        let storage = StorageLayer::new_in_memory().await?;

        for table in [
            "distributed_traces",
            "logs",
            "metric_points",
            "metric_scopes",
            "resources",
        ] {
            let exists: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?",
            )
            .bind(table)
            .fetch_one(&storage.pool)
            .await?;
            assert_eq!(exists, 0, "table `{table}` must be dropped");
        }
        Ok(())
    }

    #[tokio::test]
    async fn upgrade_from_pre_agent_scope_db_succeeds() -> Result<()> {
        let db_path = temp_db_path();
        seed_database_through_016(&db_path).await?;

        let storage = StorageLayer::new(&db_path).await?;
        let pool = &storage.pool;

        assert!(column_exists(pool, "conversations", "agent_id").await?);
        assert!(column_exists(pool, "scheduled_tasks", "agent_id").await?);
        assert!(column_exists(pool, "webhooks", "agent_id").await?);

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

    /// Migration 061 turns the flat message list into a chain: every row's
    /// parent is its predecessor by `(turn, created_at, rowid)` and each
    /// conversation's active leaf is its last row.
    #[tokio::test]
    async fn message_tree_migration_backfills_chain_and_leaf() -> Result<()> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        sqlx::raw_sql("PRAGMA foreign_keys=ON;")
            .execute(&pool)
            .await?;
        for sql in [
            include_str!("../../../migrations/001_conversations.sql"),
            include_str!("../../../migrations/005_tool_calls.sql"),
            include_str!("../../../migrations/040_message_sender.sql"),
        ] {
            sqlx::raw_sql(sql).execute(&pool).await?;
        }

        let conv = Uuid::new_v4().to_string();
        let empty_conv = Uuid::new_v4().to_string();
        for id in [&conv, &empty_conv] {
            sqlx::query("INSERT INTO conversations (id) VALUES (?)")
                .bind(id)
                .execute(&pool)
                .await?;
        }
        // Inserted out of order on purpose: the chain follows (turn, created_at).
        let rows = [
            ("m2", "assistant", 1, "2026-01-01 00:00:02"),
            ("m1", "user", 0, "2026-01-01 00:00:01"),
            ("m3", "user", 2, "2026-01-01 00:00:03"),
        ];
        for (id, role, turn, at) in rows {
            sqlx::query(
                "INSERT INTO messages (id, conversation_id, role, content, turn, created_at) \
                 VALUES (?, ?, ?, 'x', ?, ?)",
            )
            .bind(id)
            .bind(&conv)
            .bind(role)
            .bind(turn)
            .bind(at)
            .execute(&pool)
            .await?;
        }

        sqlx::raw_sql(include_str!("../../../migrations/061_message_tree.sql"))
            .execute(&pool)
            .await?;

        let parents: Vec<(String, Option<String>)> =
            sqlx::query_as("SELECT id, parent_id FROM messages ORDER BY turn")
                .fetch_all(&pool)
                .await?;
        assert_eq!(
            parents,
            vec![
                ("m1".to_string(), None),
                ("m2".to_string(), Some("m1".to_string())),
                ("m3".to_string(), Some("m2".to_string())),
            ],
            "backfilled parent chain"
        );

        let leaf: Option<String> =
            sqlx::query_scalar("SELECT active_leaf_id FROM conversations WHERE id = ?")
                .bind(&conv)
                .fetch_one(&pool)
                .await?;
        assert_eq!(leaf.as_deref(), Some("m3"), "leaf is the last row");

        let empty_leaf: Option<String> =
            sqlx::query_scalar("SELECT active_leaf_id FROM conversations WHERE id = ?")
                .bind(&empty_conv)
                .fetch_one(&pool)
                .await?;
        assert_eq!(
            empty_leaf, None,
            "a conversation without messages has no leaf"
        );
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

        for &(name, sql) in seed_migrations {
            sqlx::raw_sql(sql).execute(&pool).await?;
            sqlx::query("INSERT INTO _migrations(name) VALUES (?)")
                .bind(name)
                .execute(&pool)
                .await?;
        }

        Ok(())
    }

    async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> Result<bool> {
        // `PRAGMA table_info(...)` cannot take a bound parameter for the table
        // name, so the identifier must be interpolated. The table names are
        // hard-coded test fixtures, never user input — safe to assert.
        let query = sqlx::AssertSqlSafe(format!("PRAGMA table_info({table})"));
        let rows = sqlx::query(query).fetch_all(pool).await?;
        Ok(rows.iter().any(|row| {
            row.try_get::<String, _>("name")
                .is_ok_and(|name| name == column)
        }))
    }

    fn temp_db_path() -> PathBuf {
        std::env::temp_dir().join(format!("assistant-storage-migration-{}.db", Uuid::new_v4()))
    }
}
