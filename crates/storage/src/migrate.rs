//! Applies tracked SQL migrations, safe for several processes opening one
//! database at the same time.

use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::{Connection, SqlitePool};
use tracing::info;

/// Switches the database to WAL mode.
///
/// SQLite returns `SQLITE_BUSY` without waiting on the busy timeout when
/// several connections switch a new database file to WAL at once, so retry
/// while it is busy. Once a file is in WAL mode the switch is a no-op.
pub(crate) async fn enable_wal(pool: &SqlitePool) -> Result<()> {
    const ATTEMPTS: u32 = 100;
    for attempt in 1..=ATTEMPTS {
        match sqlx::query("PRAGMA journal_mode=WAL;").execute(pool).await {
            Ok(_) => return Ok(()),
            Err(sqlx::Error::Database(e))
                if e.code().as_deref() == Some("5") && attempt < ATTEMPTS =>
            {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(e) => return Err(e).context("enabling WAL journal mode"),
        }
    }
    Ok(())
}

/// Applies each `(name, sql)` migration not yet recorded in `_migrations`.
///
/// Each migration runs in a `BEGIN IMMEDIATE` transaction that re-checks
/// `_migrations` after taking the write lock, so a process that loses the
/// race waits and then skips what the winner applied. Migration SQL must not
/// open its own transaction.
pub(crate) async fn apply_migrations(
    pool: &SqlitePool,
    migrations: &[(&str, &'static str)],
) -> Result<()> {
    let mut conn = pool.acquire().await?;
    // A busy disk can hold the write lock for many seconds during startup.
    sqlx::query("PRAGMA busy_timeout = 60000;")
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name        TEXT PRIMARY KEY,
            applied_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&mut *conn)
    .await?;

    for &(name, sql) in migrations {
        let mut tx = conn.begin_with("BEGIN IMMEDIATE").await?;
        let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _migrations WHERE name = ?")
            .bind(name)
            .fetch_one(&mut *tx)
            .await?;
        if applied > 0 {
            continue;
        }
        sqlx::raw_sql(sql)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("failed to apply migration {name}"))?;
        sqlx::query("INSERT INTO _migrations (name) VALUES (?)")
            .bind(name)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        info!(migration = %name, "Applied migration");
    }

    Ok(())
}
