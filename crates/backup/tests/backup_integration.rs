//! Integration tests for the backup and restore engines using real filesystem I/O.

use std::path::Path;

use tempfile::TempDir;

use assistant_backup::{
    archive::read_tar_gz_manifest, BackupEngine, BackupOptions, RestoreEngine, RestoreOptions,
};

/// Populate a fake installation directory.
fn seed_install_dir(dir: &Path) {
    std::fs::write(dir.join("config.toml"), b"[llm]\nprovider = \"anthropic\"").unwrap();
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
        db_path: None,
    };

    let result = BackupEngine::new().run(opts).await.unwrap();

    assert!(output_path.exists(), "archive must exist after backup");
    assert!(result.archive_size > 0, "archive must be non-empty");
    assert_eq!(result.entry_count, 4, "4 seeded files expected");

    // Manifest must be readable from the archive
    let manifest = read_tar_gz_manifest(&output_path).unwrap();
    assert_eq!(
        manifest.entries.len(),
        4,
        "manifest should list all 4 seeded files"
    );

    let archive_paths: Vec<&str> = manifest
        .entries
        .iter()
        .map(|e| e.archive_path.as_str())
        .collect();
    assert!(archive_paths.contains(&"config.toml"));
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
        db_path: None,
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
    let original = std::fs::read(install_dir.path().join("config.toml")).unwrap();
    let restored = std::fs::read(restore_dir.path().join("config.toml")).unwrap();
    assert_eq!(original, restored, "config.toml must be byte-identical");

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
