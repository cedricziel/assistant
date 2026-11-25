//! Installation path discovery and the SQLite snapshot helper.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use assistant_core::clock::{Clock, SystemClock};
use tracing::{debug, warn};

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
    let now = SystemClock.now();
    format!("assistant-backup-{}.tar.gz", now.format("%Y%m%d-%H%M%S"))
}

/// Key material and credentials at the install root. A backup is useless
/// without them: `secrets.key` encrypts stored integration credentials, so
/// losing it makes every credential in `org.db` unreadable, and `jwt_secret`
/// signs issued tokens. `openai-oauth.json` holds the OpenAI refresh token and
/// `credentials.json` the CLI's login to a server. Restored copies are written
/// with mode `0600`.
const PRIVATE_FILES: [&str; 4] = [
    "secrets.key",
    "jwt_secret",
    "openai-oauth.json",
    "credentials.json",
];

/// Other files at the install root that are always part of a backup.
const ROOT_FILES: [&str; 2] = ["server.toml", "org.db"];

/// Directories under the install root that are backed up recursively.
///
/// `orgs/` holds every org's config, catalog and spaces (`space.db`, agents,
/// skills). The rest are the install-root persona and skill directories and
/// the interface state stores. Anything else, such as `cache/` (regenerated
/// on demand), `backups/` and `*.legacy` snapshots, is left out.
const ROOT_DIRS: [&str; 5] = ["agents", "skills", "orgs", "matrix-state", "signal-store"];

/// Walk `install_dir` and return all files to include in a backup.
///
/// The returned paths are absolute and in a deterministic order. SQLite
/// `-wal` and `-shm` side files are not listed: databases are archived as
/// consistent snapshots (see [`snapshot_sqlite`]) that already contain the
/// WAL contents, and a stale side file restored next to a snapshot could
/// corrupt it.
pub fn discover_files(install_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();

    for name in ROOT_FILES.into_iter().chain(PRIVATE_FILES) {
        let path = install_dir.join(name);
        if path.is_file() {
            files.push(path);
        }
    }
    for name in ROOT_DIRS {
        let dir = install_dir.join(name);
        // `is_dir` follows symlinks, so look at the link itself first.
        match std::fs::symlink_metadata(&dir) {
            Ok(meta) if meta.is_symlink() => warn_skipped_symlink(&dir),
            Ok(meta) if meta.is_dir() => collect_dir_files(&dir, &mut files)?,
            _ => {}
        }
    }

    debug!(
        "discovered {} files for backup in {:?}",
        files.len(),
        install_dir
    );
    Ok(files)
}

/// Whether `path` is a SQLite database file that must be snapshotted.
pub fn is_sqlite_db(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "db")
}

/// Whether `path` is key material that must not be readable by other users.
pub fn is_private_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| PRIVATE_FILES.contains(&n))
}

/// Whether `path` is a SQLite `-wal` or `-shm` side file.
fn is_sqlite_sidecar(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".db-wal") || n.ends_with(".db-shm"))
}

fn warn_skipped_symlink(path: &Path) {
    warn!(
        "not backing up symlink {:?}: symlinks are not followed",
        path
    );
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
        } else if meta.is_file() && !is_sqlite_sidecar(&path) {
            out.push(path);
        } else if meta.is_symlink() {
            warn!(
                "not backing up symlink {:?}: symlinks are not followed",
                path
            );
        }
    }
    Ok(())
}

/// Return a transactionally consistent copy of the SQLite database at
/// `db_path`, including rows that are still only in its `-wal` file.
///
/// Uses `VACUUM INTO`, which reads inside a single read transaction, so a
/// server that keeps writing cannot produce a torn copy. Copying the main
/// file and its side files one by one cannot give that guarantee.
pub async fn snapshot_sqlite(db_path: &Path) -> Result<Vec<u8>> {
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::{Connection, SqliteConnection};

    // `filename()` rather than a `sqlite://` URL: sqlx's URL parser cannot
    // handle Windows paths with backslashes reliably.
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .read_only(true);
    let mut conn = SqliteConnection::connect_with(&options)
        .await
        .with_context(|| format!("opening SQLite {db_path:?} for snapshot"))?;

    let scratch = tempfile::tempdir().context("creating snapshot scratch directory")?;
    let target = scratch.path().join("snapshot.db");
    let snapshot = sqlx::query("VACUUM INTO ?1")
        .bind(target.to_string_lossy().into_owned())
        .execute(&mut conn)
        .await
        .with_context(|| format!("snapshotting SQLite {db_path:?}"));
    if let Err(e) = conn.close().await {
        debug!("closing snapshot connection for {db_path:?}: {e}");
    }
    snapshot?;

    let bytes = tokio::fs::read(&target)
        .await
        .with_context(|| format!("reading snapshot of {db_path:?}"))?;
    debug!("snapshotted {:?} ({} bytes)", db_path, bytes.len());
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(root: &Path, rel: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"x").unwrap();
    }

    fn relative(root: &Path, files: Vec<PathBuf>) -> Vec<String> {
        files
            .iter()
            .map(|f| f.strip_prefix(root).unwrap().display().to_string())
            .collect()
    }

    /// A 0.2.0-style install: two orgs with two spaces each, plus the
    /// root-level files and the things a backup must leave out.
    fn seed_multi_org_install(root: &Path) {
        for rel in [
            "server.toml",
            "org.db",
            "secrets.key",
            "jwt_secret",
            "agents/default/SOUL.md",
            "skills/triage/SKILL.md",
        ] {
            touch(root, rel);
        }
        for org in ["globex", "acme"] {
            touch(root, &format!("orgs/{org}/org.toml"));
            touch(root, &format!("orgs/{org}/catalog/templates/weekly.md"));
            for space in ["ops", "eng"] {
                let space_dir = format!("orgs/{org}/spaces/{space}");
                touch(root, &format!("{space_dir}/space.db"));
                touch(root, &format!("{space_dir}/agents/p1/SOUL.md"));
                touch(root, &format!("{space_dir}/agents/p1/memory/note.md"));
                touch(root, &format!("{space_dir}/skills/s1/SKILL.md"));
            }
        }
    }

    #[test]
    fn discovers_the_whole_multi_org_tree() {
        let dir = tempfile::tempdir().unwrap();
        seed_multi_org_install(dir.path());

        let found = relative(dir.path(), discover_files(dir.path()).unwrap());

        let mut expected = vec![
            "server.toml".to_string(),
            "org.db".into(),
            "secrets.key".into(),
            "jwt_secret".into(),
            "agents/default/SOUL.md".into(),
            "skills/triage/SKILL.md".into(),
        ];
        for org in ["acme", "globex"] {
            expected.push(format!("orgs/{org}/catalog/templates/weekly.md"));
            expected.push(format!("orgs/{org}/org.toml"));
            for space in ["eng", "ops"] {
                let space_dir = format!("orgs/{org}/spaces/{space}");
                expected.push(format!("{space_dir}/agents/p1/SOUL.md"));
                expected.push(format!("{space_dir}/agents/p1/memory/note.md"));
                expected.push(format!("{space_dir}/skills/s1/SKILL.md"));
                expected.push(format!("{space_dir}/space.db"));
            }
        }
        let mut found_sorted = found.clone();
        found_sorted.sort();
        expected.sort();
        assert_eq!(found_sorted, expected);
        assert_eq!(found.len(), expected.len(), "no file may be listed twice");
    }

    #[test]
    fn leaves_out_regenerable_and_unrelated_paths() {
        let dir = tempfile::tempdir().unwrap();
        seed_multi_org_install(dir.path());
        for rel in [
            "cache/integrations/index.json",
            "backups/assistant-backup-20260101-000000.tar.gz",
            "assistant.db",
            "assistant.db.legacy",
            "config.toml",
        ] {
            touch(dir.path(), rel);
        }

        let found = relative(dir.path(), discover_files(dir.path()).unwrap());

        for excluded in [
            "cache/",
            "backups/",
            "assistant.db",
            "config.toml",
            ".legacy",
        ] {
            assert!(
                !found.iter().any(|f| f.contains(excluded)),
                "{excluded} must not be archived, found {found:?}"
            );
        }
    }

    #[test]
    fn sqlite_sidecars_are_never_listed() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "org.db");
        touch(dir.path(), "org.db-wal");
        touch(dir.path(), "org.db-shm");
        touch(dir.path(), "orgs/acme/spaces/eng/space.db");
        touch(dir.path(), "orgs/acme/spaces/eng/space.db-wal");
        touch(dir.path(), "orgs/acme/spaces/eng/space.db-shm");

        let found = relative(dir.path(), discover_files(dir.path()).unwrap());

        assert_eq!(found, ["org.db", "orgs/acme/spaces/eng/space.db"]);
    }

    #[test]
    fn credential_files_at_the_install_root_are_discovered() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "openai-oauth.json");
        touch(dir.path(), "credentials.json");

        let found = relative(dir.path(), discover_files(dir.path()).unwrap());

        assert_eq!(found, ["openai-oauth.json", "credentials.json"]);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_inside_the_tree_are_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        touch(outside.path(), "secret.txt");
        touch(dir.path(), "orgs/acme/org.toml");
        std::os::unix::fs::symlink(outside.path(), dir.path().join("orgs/acme/linked")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret.txt"),
            dir.path().join("orgs/acme/secret.txt"),
        )
        .unwrap();

        let found = relative(dir.path(), discover_files(dir.path()).unwrap());

        assert_eq!(found, ["orgs/acme/org.toml"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_root_directory_is_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        touch(outside.path(), "acme/org.toml");
        std::os::unix::fs::symlink(outside.path(), dir.path().join("orgs")).unwrap();

        let found = discover_files(dir.path()).unwrap();

        assert!(found.is_empty(), "archived through a symlink: {found:?}");
    }

    #[test]
    fn a_missing_install_dir_yields_no_files() {
        let dir = tempfile::tempdir().unwrap();

        let found = discover_files(&dir.path().join("absent")).unwrap();

        assert!(found.is_empty());
    }
}
