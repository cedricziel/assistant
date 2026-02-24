pub mod conversations;
pub mod memory_chunks;
pub mod refinements;
pub mod registry;
pub mod scheduled_tasks;
pub mod traces;

pub use conversations::{ConversationRecord, ConversationStore};
pub use memory_chunks::{FtsMatch, MemoryChunkStore, StoredChunk};
pub use refinements::{RefinementStatus, RefinementsStore, SkillRefinement};
pub use registry::SkillRegistry;
pub use scheduled_tasks::{ScheduledTask, ScheduledTaskStore};
pub use traces::{TraceStats, TraceStore};

use anyhow::Result;
use sqlx::SqlitePool;
use std::path::Path;
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
    pub async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    /// Convenience: build a `TraceStore` backed by this pool.
    pub fn trace_store(&self) -> TraceStore {
        TraceStore::new(self.pool.clone())
    }

    /// Convenience: build a `ConversationStore` backed by this pool.
    pub fn conversation_store(&self) -> ConversationStore {
        ConversationStore::new(self.pool.clone())
    }

    /// Convenience: build a `RefinementsStore` backed by this pool.
    pub fn refinements_store(&self) -> RefinementsStore {
        RefinementsStore::new(self.pool.clone())
    }

    /// Convenience: build a `ScheduledTaskStore` backed by this pool.
    pub fn scheduled_task_store(&self) -> ScheduledTaskStore {
        ScheduledTaskStore::new(self.pool.clone())
    }

    /// Convenience: build a `MemoryChunkStore` backed by this pool.
    pub fn memory_chunks_store(&self) -> MemoryChunkStore {
        MemoryChunkStore::new(self.pool.clone())
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
    ];

    for (name, sql) in migrations {
        let already_applied: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM _migrations WHERE name = ?")
                .bind(name)
                .fetch_one(pool)
                .await?;

        if already_applied == 0 {
            sqlx::query(sql).execute(pool).await?;
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
