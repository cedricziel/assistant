//! Installation path discovery and SQLite WAL checkpoint helper.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use tracing::debug;

/// Default installation directory (`~/.assistant/`).
pub fn default_install_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".assistant")
}

/// Default backups directory (`~/.assistant/backups/`).
pub fn default_backups_dir() -> PathBuf {
    default_install_dir().join("backups")
}

/// Generate a timestamped default archive filename.
///
/// Format: `assistant-backup-YYYYMMDD-HHMMSS.tar.gz`
pub fn default_archive_name() -> String {
    let now = Utc::now();
    format!("assistant-backup-{}.tar.gz", now.format("%Y%m%d-%H%M%S"))
}

/// Walk `install_dir` and return all files to include in a backup.
///
/// The returned paths are absolute. The function skips `backups/` itself to
/// avoid recursive inclusion of previous backup archives.
pub fn discover_files(install_dir: &Path, db_path: Option<&Path>) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();

    // config.toml
    let config = install_dir.join("config.toml");
    if config.exists() {
        files.push(config);
    }

    // SQLite database (potentially at a custom path)
    let db = db_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| install_dir.join("assistant.db"));

    if db.exists() {
        files.push(db.clone());
    }
    // WAL and SHM files alongside the main db.
    // Use with_file_name to append the suffix rather than with_extension, which
    // would replace the extension (e.g. "assistant.sqlite" → "assistant.db-wal"
    // instead of the correct "assistant.sqlite-wal").
    if let Some(name) = db.file_name() {
        let name = name.to_string_lossy();
        let wal = db.with_file_name(format!("{}-wal", name));
        if wal.exists() {
            files.push(wal);
        }
        let shm = db.with_file_name(format!("{}-shm", name));
        if shm.exists() {
            files.push(shm);
        }
    }

    // agents/ directory tree
    let agents_dir = install_dir.join("agents");
    if agents_dir.exists() {
        collect_dir_files(&agents_dir, &mut files)?;
    }

    // skills/ directory tree
    let skills_dir = install_dir.join("skills");
    if skills_dir.exists() {
        collect_dir_files(&skills_dir, &mut files)?;
    }

    // matrix-state/ (optional)
    let matrix_dir = install_dir.join("matrix-state");
    if matrix_dir.exists() {
        collect_dir_files(&matrix_dir, &mut files)?;
    }

    // signal-store/ (optional)
    let signal_dir = install_dir.join("signal-store");
    if signal_dir.exists() {
        collect_dir_files(&signal_dir, &mut files)?;
    }

    debug!(
        "discovered {} files for backup in {:?}",
        files.len(),
        install_dir
    );
    Ok(files)
}

/// Recursively collect all regular files under `dir` in deterministic order.
///
/// Entries are sorted by path before recursing so archives are byte-reproducible
/// across different OS directory-enumeration orders.
fn collect_dir_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("reading directory {:?}", dir))?
        .collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            collect_dir_files(&path, out)?;
        } else if meta.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

/// Issue a WAL checkpoint on the SQLite database at `db_path` using a
/// read-only connection, then close the connection.
///
/// This folds any outstanding WAL data back into the main database file,
/// making a subsequent file copy consistent.
pub async fn checkpoint_sqlite(db_path: &Path) -> Result<()> {
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
    use sqlx::{ConnectOptions, Connection};

    // Use SqliteConnectOptions::new().filename() rather than constructing a
    // "sqlite://" URL to avoid issues with Windows paths containing backslashes
    // that sqlx's URL parser cannot handle reliably.
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .read_only(true)
        .journal_mode(SqliteJournalMode::Wal);

    let mut conn = opts
        .connect()
        .await
        .with_context(|| format!("opening SQLite {:?} for checkpoint", db_path))?;

    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&mut conn)
        .await
        .with_context(|| "executing WAL checkpoint")?;

    conn.close().await.ok();
    debug!("WAL checkpoint completed for {:?}", db_path);
    Ok(())
}
