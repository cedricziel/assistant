pub mod conversations;
pub mod memory;
pub mod refinements;
pub mod registry;
pub mod scheduled_tasks;
pub mod traces;

pub use conversations::{ConversationRecord, ConversationStore};
pub use memory::{MemoryEntry, MemoryStore};
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

    /// Convenience: build a `MemoryStore` backed by this pool.
    pub fn memory_store(&self) -> MemoryStore {
        MemoryStore::new(self.pool.clone())
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
}

/// Returns the default database path: `~/.assistant/assistant.db`.
pub fn default_db_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".assistant").join("assistant.db"))
}

/// Run all embedded migrations in order.
async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    // Enable WAL mode for better concurrency
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys=ON;").execute(pool).await?;

    // Run each migration as a plain SQL string so we avoid compile-time path resolution issues.
    // The migrations directory lives at the workspace root; we embed them inline.
    sqlx::query(include_str!("../../../migrations/001_conversations.sql"))
        .execute(pool)
        .await?;

    sqlx::query(include_str!("../../../migrations/002_skills.sql"))
        .execute(pool)
        .await?;

    sqlx::query(include_str!("../../../migrations/003_execution_traces.sql"))
        .execute(pool)
        .await?;

    sqlx::query(include_str!("../../../migrations/004_memory.sql"))
        .execute(pool)
        .await?;

    info!("Database migrations applied successfully");
    Ok(())
}
