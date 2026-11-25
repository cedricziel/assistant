//! Integration tests for the backup and restore engines using real filesystem I/O.

use std::path::{Path, PathBuf};

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{Connection, SqliteConnection};
use tempfile::TempDir;

use assistant_backup::{
    BackupEngine, BackupOptions, RestoreEngine, RestoreOptions, archive::read_tar_gz_manifest,
    checksum::sha256_hex,
};

/// Populate a fake installation directory.
fn seed_install_dir(dir: &Path) {
    std::fs::write(dir.join("server.toml"), b"[bus]\nkind = \"sqlite\"").unwrap();
    for org in ["acme", "globex"] {
        let org_dir = dir.join("orgs").join(org);
        std::fs::create_dir_all(&org_dir).unwrap();
        std::fs::write(
            org_dir.join("org.toml"),
            format!("[llm]\nprovider = \"anthropic\"\nmodel = \"{org}\""),
        )
        .unwrap();
    }
    let agents_default = dir.join("agents").join("default");
    std::fs::create_dir_all(&agents_default).unwrap();
    std::fs::write(agents_default.join("SOUL.md"), b"# Soul\nYou are helpful.").unwrap();
    std::fs::write(agents_default.join("IDENTITY.md"), b"# Identity\nAssistant").unwrap();
    let memory_dir = agents_default.join("memory");
    std::fs::create_dir_all(&memory_dir).unwrap();
    std::fs::write(memory_dir.join("2026-03-29.md"), b"## Daily note").unwrap();
}

#[tokio::test]
async fn test_backup_round_trip_tempdir() {
    let install_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();

    seed_install_dir(install_dir.path());

    let output_path = output_dir.path().join("backup.tar.gz");
    let opts = BackupOptions {
        install_dir: install_dir.path().to_path_buf(),
        output_path: output_path.clone(),
    };

    let result = BackupEngine::new().run(opts).await.unwrap();

    assert!(output_path.exists(), "archive must exist after backup");
    assert!(result.archive_size > 0, "archive must be non-empty");
    assert_eq!(result.entry_count, 6, "6 seeded files expected");

    // Manifest must be readable from the archive
    let manifest = read_tar_gz_manifest(&output_path).unwrap();
    assert_eq!(
        manifest.entries.len(),
        6,
        "manifest should list all 6 seeded files"
    );

    let archive_paths: Vec<&str> = manifest
        .entries
        .iter()
        .map(|e| e.archive_path.as_str())
        .collect();
    assert!(archive_paths.contains(&"server.toml"));
    assert!(archive_paths.contains(&"orgs/acme/org.toml"));
    assert!(archive_paths.contains(&"orgs/globex/org.toml"));
    assert!(archive_paths.iter().any(|p| p.contains("SOUL.md")));
}

#[tokio::test]
async fn test_restore_round_trip() {
    let install_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();

    seed_install_dir(install_dir.path());

    let archive_path = output_dir.path().join("backup.tar.gz");
    let backup_opts = BackupOptions {
        install_dir: install_dir.path().to_path_buf(),
        output_path: archive_path.clone(),
    };
    let backup_result = BackupEngine::new().run(backup_opts).await.unwrap();

    // Restore to a fresh directory
    let restore_dir = TempDir::new().unwrap();
    let restore_opts = RestoreOptions {
        archive_path: archive_path.clone(),
        install_dir: restore_dir.path().to_path_buf(),
        force: true,
    };

    let restore_result = RestoreEngine::new().run(restore_opts).await.unwrap();
    assert_eq!(
        restore_result.restored_count, backup_result.entry_count,
        "restored file count must match backup entry count"
    );
    assert!(restore_result.warnings.is_empty());

    // Spot-check file content is byte-identical
    for file in ["server.toml", "orgs/acme/org.toml", "orgs/globex/org.toml"] {
        let original = std::fs::read(install_dir.path().join(file)).unwrap();
        let restored = std::fs::read(restore_dir.path().join(file)).unwrap();
        assert_eq!(original, restored, "{file} must be byte-identical");
    }

    let original_soul = std::fs::read(
        install_dir
            .path()
            .join("agents")
            .join("default")
            .join("SOUL.md"),
    )
    .unwrap();
    let restored_soul = std::fs::read(
        restore_dir
            .path()
            .join("agents")
            .join("default")
            .join("SOUL.md"),
    )
    .unwrap();
    assert_eq!(
        original_soul, restored_soul,
        "SOUL.md must be byte-identical"
    );
}

#[tokio::test]
async fn test_restore_corrupted_archive_fails() {
    let dir = TempDir::new().unwrap();
    let archive_path = dir.path().join("corrupt.tar.gz");
    std::fs::write(&archive_path, b"not a valid gzip").unwrap();

    let install_dir = TempDir::new().unwrap();
    let opts = RestoreOptions {
        archive_path,
        install_dir: install_dir.path().to_path_buf(),
        force: true,
    };

    let result = RestoreEngine::new().run(opts).await;
    assert!(result.is_err(), "corrupted archive must fail restore");

    // install_dir must be untouched
    assert_eq!(
        std::fs::read_dir(install_dir.path()).unwrap().count(),
        0,
        "install_dir must remain empty after a failed restore"
    );
}

// -- Multi-org install tree --

/// A SQLite database in WAL mode whose rows are only in the `-wal` file: the
/// returned connection stays open, so nothing has been checkpointed.
async fn seed_live_db(path: &Path, rows: &[&str]) -> SqliteConnection {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);
    let mut conn = SqliteConnection::connect_with(&options).await.unwrap();
    sqlx::query("PRAGMA wal_autocheckpoint = 0")
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE notes (v TEXT NOT NULL)")
        .execute(&mut conn)
        .await
        .unwrap();
    for row in rows {
        sqlx::query("INSERT INTO notes (v) VALUES (?)")
            .bind(row)
            .execute(&mut conn)
            .await
            .unwrap();
    }
    conn
}

async fn read_rows(path: &Path) -> Vec<String> {
    let options = SqliteConnectOptions::new().filename(path).read_only(true);
    let mut conn = SqliteConnection::connect_with(&options).await.unwrap();
    sqlx::query_scalar("SELECT v FROM notes ORDER BY v")
        .fetch_all(&mut conn)
        .await
        .unwrap()
}

/// Key material and credentials written at the install root.
const PRIVATE_FILES: [&str; 4] = [
    "secrets.key",
    "jwt_secret",
    "openai-oauth.json",
    "credentials.json",
];

const DB_PATHS: [&str; 5] = [
    "org.db",
    "orgs/acme/spaces/eng/space.db",
    "orgs/acme/spaces/ops/space.db",
    "orgs/globex/spaces/eng/space.db",
    "orgs/globex/spaces/ops/space.db",
];

const PLAIN_FILES: [&str; 14] = [
    "server.toml",
    "secrets.key",
    "jwt_secret",
    "openai-oauth.json",
    "credentials.json",
    "orgs/acme/org.toml",
    "orgs/globex/org.toml",
    "orgs/acme/catalog/templates/weekly.md",
    "orgs/acme/spaces/eng/agents/p1/SOUL.md",
    "orgs/acme/spaces/eng/agents/p1/memory/note.md",
    "orgs/globex/spaces/ops/skills/s1/SKILL.md",
    "agents/default/SOUL.md",
    "skills/triage/SKILL.md",
    "matrix-state/session.json",
];

/// Seed a 0.2.0-style install and return the open database connections.
async fn seed_multi_org_install(dir: &Path) -> Vec<SqliteConnection> {
    let write = |rel: &str, contents: String| {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    };
    for rel in PLAIN_FILES {
        write(rel, format!("contents of {rel}"));
    }
    for rel in ["cache/integrations/index.json", "assistant.db.legacy"] {
        write(rel, "not part of a backup".into());
    }
    let mut conns = Vec::new();
    for rel in DB_PATHS {
        conns.push(seed_live_db(&dir.join(rel), &["alpha", "beta", rel]).await);
    }
    conns
}

fn backup_opts(install_dir: &Path, output_path: PathBuf) -> BackupOptions {
    BackupOptions {
        install_dir: install_dir.to_path_buf(),
        output_path,
    }
}

#[tokio::test]
async fn test_multi_org_round_trip_restores_every_file_and_database() {
    let install_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    let _live = seed_multi_org_install(install_dir.path()).await;
    let archive = output_dir.path().join("backup.tar.gz");

    let backup = BackupEngine::new()
        .run(backup_opts(install_dir.path(), archive.clone()))
        .await
        .unwrap();

    let manifest = read_tar_gz_manifest(&archive).unwrap();
    let mut archived: Vec<&str> = manifest
        .entries
        .iter()
        .map(|e| e.archive_path.as_str())
        .collect();
    archived.sort();
    let mut expected: Vec<&str> = PLAIN_FILES.iter().chain(DB_PATHS.iter()).copied().collect();
    expected.sort();
    assert_eq!(
        archived, expected,
        "archive must hold exactly the install tree"
    );

    // Restore into an empty directory without --force: no prompt may appear.
    let restore_dir = TempDir::new().unwrap();
    let restore = RestoreEngine::new()
        .run(RestoreOptions {
            archive_path: archive,
            install_dir: restore_dir.path().to_path_buf(),
            force: false,
        })
        .await
        .unwrap();
    assert_eq!(restore.restored_count, backup.entry_count);
    assert!(restore.warnings.is_empty(), "{:?}", restore.warnings);

    for rel in PLAIN_FILES {
        assert_eq!(
            std::fs::read(install_dir.path().join(rel)).unwrap(),
            std::fs::read(restore_dir.path().join(rel)).unwrap(),
            "{rel} must be byte-identical"
        );
    }
    for rel in DB_PATHS {
        assert_eq!(
            read_rows(&restore_dir.path().join(rel)).await,
            read_rows(&install_dir.path().join(rel)).await,
            "{rel} must restore with the same rows, including WAL-only ones"
        );
        for side in ["-wal", "-shm"] {
            assert!(
                !restore_dir.path().join(format!("{rel}{side}")).exists(),
                "{rel}{side} must not be restored"
            );
        }
    }
    for entry in &manifest.entries {
        let restored = std::fs::read(restore_dir.path().join(&entry.archive_path)).unwrap();
        assert_eq!(
            restored.len() as u64,
            entry.size_bytes,
            "{}",
            entry.archive_path
        );
        assert_eq!(
            sha256_hex(&restored),
            entry.sha256,
            "{}",
            entry.archive_path
        );
    }
    assert!(!restore_dir.path().join("cache").exists());
    assert!(!restore_dir.path().join("assistant.db.legacy").exists());
}

#[tokio::test]
async fn test_restore_drops_stale_wal_files_next_to_a_restored_database() {
    let install_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    let _live = seed_live_db(&install_dir.path().join("org.db"), &["kept"]).await;
    let archive = output_dir.path().join("backup.tar.gz");
    BackupEngine::new()
        .run(backup_opts(install_dir.path(), archive.clone()))
        .await
        .unwrap();

    let target = TempDir::new().unwrap();
    std::fs::write(target.path().join("org.db"), b"old install").unwrap();
    std::fs::write(target.path().join("org.db-wal"), b"stale wal").unwrap();
    std::fs::write(target.path().join("org.db-shm"), b"stale shm").unwrap();
    RestoreEngine::new()
        .run(RestoreOptions {
            archive_path: archive,
            install_dir: target.path().to_path_buf(),
            force: true,
        })
        .await
        .unwrap();

    assert!(!target.path().join("org.db-wal").exists());
    assert!(!target.path().join("org.db-shm").exists());
    assert_eq!(read_rows(&target.path().join("org.db")).await, ["kept"]);
}

#[cfg(unix)]
#[tokio::test]
async fn test_archive_and_restored_secrets_are_private() {
    use std::os::unix::fs::PermissionsExt;

    let install_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    for name in PRIVATE_FILES {
        std::fs::write(install_dir.path().join(name), b"secret").unwrap();
    }
    let archive = output_dir.path().join("backup.tar.gz");
    BackupEngine::new()
        .run(backup_opts(install_dir.path(), archive.clone()))
        .await
        .unwrap();
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&archive), 0o600, "the archive holds the secrets");

    let restore_dir = TempDir::new().unwrap();
    RestoreEngine::new()
        .run(RestoreOptions {
            archive_path: archive,
            install_dir: restore_dir.path().to_path_buf(),
            force: true,
        })
        .await
        .unwrap();
    for name in PRIVATE_FILES {
        assert_eq!(mode(&restore_dir.path().join(name)), 0o600, "{name}");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn test_backup_makes_a_leftover_tmp_archive_private() {
    use std::os::unix::fs::PermissionsExt;

    let install_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    std::fs::write(install_dir.path().join("secrets.key"), b"secret").unwrap();
    let archive = output_dir.path().join("backup.tar.gz");
    let leftover = archive.with_extension("tar.gz.tmp");
    std::fs::write(&leftover, b"from a crashed run").unwrap();
    std::fs::set_permissions(&leftover, std::fs::Permissions::from_mode(0o644)).unwrap();

    BackupEngine::new()
        .run(backup_opts(install_dir.path(), archive.clone()))
        .await
        .unwrap();

    let mode = std::fs::metadata(&archive).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[tokio::test]
async fn test_failed_restore_keeps_the_existing_database_and_its_wal() {
    let install_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    let _live = seed_live_db(&install_dir.path().join("org.db"), &["new"]).await;
    let archive = output_dir.path().join("backup.tar.gz");
    BackupEngine::new()
        .run(backup_opts(install_dir.path(), archive.clone()))
        .await
        .unwrap();

    // A directory where the temporary file must go makes the write fail.
    let target = TempDir::new().unwrap();
    std::fs::write(target.path().join("org.db"), b"existing").unwrap();
    std::fs::write(target.path().join("org.db-wal"), b"newest rows").unwrap();
    std::fs::create_dir(target.path().join("org.db.tmp")).unwrap();
    std::fs::write(target.path().join("org.db.tmp").join("x"), b"x").unwrap();

    let result = RestoreEngine::new()
        .run(RestoreOptions {
            archive_path: archive,
            install_dir: target.path().to_path_buf(),
            force: true,
        })
        .await;

    assert!(result.is_err(), "the write must fail");
    assert_eq!(
        std::fs::read(target.path().join("org.db")).unwrap(),
        b"existing"
    );
    assert_eq!(
        std::fs::read(target.path().join("org.db-wal")).unwrap(),
        b"newest rows",
        "the WAL must survive a failed restore"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn test_restore_refuses_to_write_through_symlinks() {
    use std::os::unix::fs::symlink;

    let install_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    for rel in ["server.toml", "orgs/acme/org.toml"] {
        let path = install_dir.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"from the backup").unwrap();
    }
    let archive = output_dir.path().join("backup.tar.gz");
    BackupEngine::new()
        .run(backup_opts(install_dir.path(), archive.clone()))
        .await
        .unwrap();

    let outside = TempDir::new().unwrap();
    std::fs::write(outside.path().join("victim.toml"), b"untouched").unwrap();

    // A symlinked file, then a symlinked parent directory.
    for plant in ["file", "dir"] {
        let target = TempDir::new().unwrap();
        match plant {
            "file" => symlink(
                outside.path().join("victim.toml"),
                target.path().join("server.toml"),
            )
            .unwrap(),
            _ => symlink(outside.path(), target.path().join("orgs")).unwrap(),
        }

        let result = RestoreEngine::new()
            .run(RestoreOptions {
                archive_path: archive.clone(),
                install_dir: target.path().to_path_buf(),
                force: true,
            })
            .await;

        let err = result.expect_err("a symlink at the destination must be refused");
        assert!(format!("{err:#}").contains("symlink"), "{err:#}");
        assert_eq!(
            std::fs::read(outside.path().join("victim.toml")).unwrap(),
            b"untouched",
            "{plant}: the file behind the symlink must not change"
        );
        assert_eq!(
            std::fs::read_dir(outside.path()).unwrap().count(),
            1,
            "{plant}: nothing may be created behind the symlink"
        );
    }
}
