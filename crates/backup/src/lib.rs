//! `assistant-backup` — backup and restore for the assistant installation.
//!
//! # Overview
//!
//! Provides two high-level engines:
//! - [`BackupEngine`] — creates a `.tar.gz` snapshot of `~/.assistant/`.
//! - [`RestoreEngine`] — restores an installation from such a snapshot.
//!
//! Both accept an [`Arc<dyn BackupFs>`] so the core logic is testable without
//! touching the real filesystem (use [`FakeFs`] in unit tests).

pub mod archive;
pub mod checksum;
pub mod manifest;
pub mod paths;

use std::collections::HashMap;
use std::io::{self, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use tracing::{info, warn};

use crate::archive::{extract_tar_gz, read_tar_gz_manifest, write_tar_gz};
use crate::checksum::sha256_hex;
use crate::manifest::{BackupManifest, ManifestEntry, MANIFEST_VERSION};
use crate::paths::{
    checkpoint_sqlite, default_archive_name, default_backups_dir, default_install_dir,
    discover_files,
};

// -- BackupFs trait --

/// Filesystem abstraction used by [`BackupEngine`] and [`RestoreEngine`].
///
/// `RealFs` delegates to `std::fs`; `FakeFs` uses an in-memory map.
pub trait BackupFs: Send + Sync {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()>;
    fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    fn file_exists(&self, path: &Path) -> bool;
    fn file_size(&self, path: &Path) -> Result<u64>;
}

// -- RealFs --

/// Production [`BackupFs`] implementation backed by `std::fs`.
#[derive(Default)]
pub struct RealFs;

impl BackupFs for RealFs {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        std::fs::read(path).with_context(|| format!("reading {:?}", path))
    }

    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating dir {:?}", parent))?;
        }
        std::fs::write(path, data).with_context(|| format!("writing {:?}", path))
    }

    fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut entries = Vec::new();
        for e in std::fs::read_dir(path).with_context(|| format!("listing {:?}", path))? {
            entries.push(e?.path());
        }
        Ok(entries)
    }

    fn file_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn file_size(&self, path: &Path) -> Result<u64> {
        Ok(std::fs::metadata(path)
            .with_context(|| format!("stat {:?}", path))?
            .len())
    }
}

// -- FakeFs --

/// In-memory [`BackupFs`] for unit tests — no disk I/O.
#[derive(Default)]
pub struct FakeFs {
    files: RwLock<HashMap<PathBuf, Vec<u8>>>,
}

impl FakeFs {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Seed a file into the fake filesystem.
    pub fn seed(&self, path: impl Into<PathBuf>, data: impl Into<Vec<u8>>) {
        self.files.write().unwrap().insert(path.into(), data.into());
    }
}

impl BackupFs for FakeFs {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        self.files
            .read()
            .unwrap()
            .get(path)
            .cloned()
            .with_context(|| format!("FakeFs: file not found: {:?}", path))
    }

    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        self.files
            .write()
            .unwrap()
            .insert(path.to_path_buf(), data.to_vec());
        Ok(())
    }

    fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let guard = self.files.read().unwrap();
        let entries: Vec<PathBuf> = guard
            .keys()
            .filter(|k| k.parent() == Some(path))
            .cloned()
            .collect();
        Ok(entries)
    }

    fn file_exists(&self, path: &Path) -> bool {
        self.files.read().unwrap().contains_key(path)
    }

    fn file_size(&self, path: &Path) -> Result<u64> {
        self.files
            .read()
            .unwrap()
            .get(path)
            .map(|d| d.len() as u64)
            .with_context(|| format!("FakeFs: file not found: {:?}", path))
    }
}

// -- Options and results --

/// Options for [`BackupEngine::run`].
#[derive(Debug, Clone)]
pub struct BackupOptions {
    /// Root of the installation to back up (default: `~/.assistant/`).
    pub install_dir: PathBuf,
    /// Destination path for the `.tar.gz` archive.
    pub output_path: PathBuf,
    /// Override for the SQLite database path (defaults to `install_dir/assistant.db`).
    pub db_path: Option<PathBuf>,
}

impl BackupOptions {
    /// Construct with defaults (uses `~/.assistant/` and a timestamped output in `backups/`).
    pub fn default_paths() -> Self {
        let install_dir = default_install_dir();
        let output_path = default_backups_dir().join(default_archive_name());
        Self {
            install_dir,
            output_path,
            db_path: None,
        }
    }
}

/// Result returned by [`BackupEngine::run`].
#[derive(Debug)]
pub struct BackupResult {
    /// Final path of the created archive.
    pub output_path: PathBuf,
    /// Compressed size of the archive in bytes.
    pub archive_size: u64,
    /// Number of files included.
    pub entry_count: usize,
    /// The manifest that was written into the archive.
    pub manifest: BackupManifest,
}

/// Options for [`RestoreEngine::run`].
#[derive(Debug, Clone)]
pub struct RestoreOptions {
    /// Path to the `.tar.gz` archive to restore from.
    pub archive_path: PathBuf,
    /// Target installation directory (default: `~/.assistant/`).
    pub install_dir: PathBuf,
    /// Skip interactive confirmation when `true`.
    pub force: bool,
}

impl RestoreOptions {
    /// Construct with defaults.
    pub fn new(archive_path: PathBuf) -> Self {
        Self {
            archive_path,
            install_dir: default_install_dir(),
            force: false,
        }
    }
}

/// Result returned by [`RestoreEngine::run`].
#[derive(Debug)]
pub struct RestoreResult {
    /// Number of files written to disk.
    pub restored_count: usize,
    /// Non-fatal issues (e.g., skipped path-traversal entries).
    pub warnings: Vec<String>,
    /// The manifest read from the restored archive.
    pub manifest: BackupManifest,
}

/// Metadata about a single backup archive (used by `backup list`).
#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub archive_size: u64,
    pub created_at: String,
    pub entry_count: usize,
    pub app_version: String,
}

// -- BackupEngine --

/// Creates `.tar.gz` backups of the assistant installation.
pub struct BackupEngine;

impl BackupEngine {
    pub fn new() -> Self {
        Self
    }

    /// Run the backup operation.
    pub async fn run(&self, opts: BackupOptions) -> Result<BackupResult> {
        info!("starting backup of {:?}", opts.install_dir);
        let start = std::time::Instant::now();

        // Ensure output parent directory exists
        if let Some(parent) = opts.output_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating output directory {:?}", parent))?;
        }

        // Checkpoint SQLite WAL before copying
        let db_path = opts
            .db_path
            .clone()
            .unwrap_or_else(|| opts.install_dir.join("assistant.db"));
        if db_path.exists() {
            if let Err(e) = checkpoint_sqlite(&db_path).await {
                warn!("WAL checkpoint failed (continuing): {}", e);
            }
        }

        // Discover files
        let file_paths = discover_files(&opts.install_dir, opts.db_path.as_deref())
            .context("discovering installation files")?;

        // Read all file contents and build manifest entries
        let mut file_data: Vec<(String, Vec<u8>)> = Vec::new();
        let mut manifest_entries: Vec<ManifestEntry> = Vec::new();

        for abs_path in &file_paths {
            let data =
                std::fs::read(abs_path).with_context(|| format!("reading {:?}", abs_path))?;

            // archive_path is relative to install_dir.
            // Files outside install_dir (e.g. a db at a custom path) are placed
            // under an "external/" prefix so the archive entry is always relative
            // and the restore dest-escape guard never triggers on them.
            let rel = match abs_path.strip_prefix(&opts.install_dir) {
                Ok(r) => r.to_string_lossy().to_string(),
                Err(_) => format!(
                    "external/{}",
                    abs_path
                        .components()
                        .filter(|c| matches!(c, std::path::Component::Normal(_)))
                        .collect::<std::path::PathBuf>()
                        .to_string_lossy()
                ),
            };

            let sha = sha256_hex(&data);
            manifest_entries.push(ManifestEntry {
                archive_path: rel.clone(),
                install_path: abs_path.to_string_lossy().to_string(),
                size_bytes: data.len() as u64,
                sha256: sha,
            });
            file_data.push((rel, data));
        }

        let manifest = BackupManifest {
            version: MANIFEST_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: Utc::now().to_rfc3339(),
            install_dir: opts.install_dir.to_string_lossy().to_string(),
            entries: manifest_entries,
        };

        // Build file slice for archive writer
        let refs: Vec<(&str, &[u8])> = file_data
            .iter()
            .map(|(p, d)| (p.as_str(), d.as_slice()))
            .collect();

        let archive_size =
            write_tar_gz(&manifest, &refs, &opts.output_path).context("writing archive")?;

        let elapsed = start.elapsed();
        info!(
            "backup complete: {:?} ({} files, {} bytes) in {:.1}s",
            opts.output_path,
            manifest.entries.len(),
            archive_size,
            elapsed.as_secs_f64()
        );

        Ok(BackupResult {
            output_path: opts.output_path,
            archive_size,
            entry_count: manifest.entries.len(),
            manifest,
        })
    }
}

impl Default for BackupEngine {
    fn default() -> Self {
        Self::new()
    }
}

// -- RestoreEngine --

/// Restores an assistant installation from a `.tar.gz` backup archive.
pub struct RestoreEngine;

impl RestoreEngine {
    pub fn new() -> Self {
        Self
    }

    /// Run the restore operation.
    pub async fn run(&self, opts: RestoreOptions) -> Result<RestoreResult> {
        info!("starting restore from {:?}", opts.archive_path);

        // Validate archive exists
        if !opts.archive_path.exists() {
            bail!("archive not found: {:?}", opts.archive_path);
        }

        // Read and validate manifest
        let manifest =
            read_tar_gz_manifest(&opts.archive_path).context("reading archive manifest")?;

        // Version compatibility check
        if manifest.version > MANIFEST_VERSION {
            bail!(
                "archive was created with a newer manifest version ({}) — \
                 upgrade the assistant binary to restore this archive",
                manifest.version
            );
        }

        // Interactive confirmation unless --force
        if !opts.force {
            let non_empty = opts.install_dir.exists()
                && std::fs::read_dir(&opts.install_dir)
                    .map(|mut d| d.next().is_some())
                    .unwrap_or(false);

            if non_empty {
                confirm_restore(&manifest, &opts.install_dir)?;
            }
        }

        // Extract
        let warnings = extract_tar_gz(&opts.archive_path, &opts.install_dir, &manifest)
            .context("extracting archive")?;

        for w in &warnings {
            warn!("{}", w);
        }

        let count = manifest.entries.len();
        info!(
            "restore complete: {} files restored to {:?}",
            count, opts.install_dir
        );

        Ok(RestoreResult {
            restored_count: count,
            warnings,
            manifest,
        })
    }
}

impl Default for RestoreEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Print confirmation prompt and read `y/N` from stdin.
fn confirm_restore(manifest: &BackupManifest, install_dir: &Path) -> Result<()> {
    eprintln!(
        "\nWARNING: This will overwrite your existing installation at {:?}",
        install_dir
    );
    eprintln!("  Backup created: {}", manifest.created_at);
    eprintln!("  Files:          {}", manifest.entries.len());
    eprintln!("  App version:    {}", manifest.app_version);
    eprint!("\nProceed? [y/N]: ");
    io::stderr().flush().ok();

    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    if !matches!(buf.trim().to_lowercase().as_str(), "y" | "yes") {
        bail!("restore cancelled by user");
    }
    Ok(())
}

// -- backup list --

/// List backup archives found in `backup_dir`.
///
/// Unreadable or malformed archives are skipped with a logged warning.
pub fn list_backups(backup_dir: &Path) -> Result<Vec<BackupInfo>> {
    if !backup_dir.exists() {
        return Ok(vec![]);
    }

    let mut infos: Vec<BackupInfo> = Vec::new();

    for entry in
        std::fs::read_dir(backup_dir).with_context(|| format!("listing {:?}", backup_dir))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("gz") {
            continue;
        }

        let archive_size = entry.metadata().map(|m| m.len()).unwrap_or(0);

        match read_tar_gz_manifest(&path) {
            Ok(manifest) => {
                infos.push(BackupInfo {
                    path,
                    archive_size,
                    created_at: manifest.created_at,
                    entry_count: manifest.entries.len(),
                    app_version: manifest.app_version,
                });
            }
            Err(e) => {
                warn!("skipping unreadable archive {:?}: {}", path, e);
            }
        }
    }

    // Sort newest first by created_at string (RFC 3339 sorts lexicographically)
    infos.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(infos)
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::make_test_archive;
    use tempfile::TempDir;

    // ── BackupEngine unit tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_backup_engine_creates_archive() {
        let install_dir = TempDir::new().unwrap();
        let output_dir = TempDir::new().unwrap();

        // Seed three files
        let config = install_dir.path().join("config.toml");
        let agents_dir = install_dir.path().join("agents").join("default");
        std::fs::create_dir_all(&agents_dir).unwrap();
        std::fs::write(&config, b"[llm]\nprovider = \"anthropic\"").unwrap();
        std::fs::write(agents_dir.join("SOUL.md"), b"# Soul").unwrap();
        std::fs::write(agents_dir.join("IDENTITY.md"), b"# Identity").unwrap();

        let output_path = output_dir.path().join("backup.tar.gz");
        let opts = BackupOptions {
            install_dir: install_dir.path().to_path_buf(),
            output_path: output_path.clone(),
            db_path: None,
        };

        let result = BackupEngine::new().run(opts).await.unwrap();

        assert!(output_path.exists(), "archive should exist");
        assert!(result.archive_size > 0);
        assert_eq!(result.entry_count, 3);

        // Verify manifest is readable
        let manifest = read_tar_gz_manifest(&output_path).unwrap();
        assert_eq!(manifest.entries.len(), 3);
    }

    #[tokio::test]
    async fn test_backup_engine_cleans_up_on_failure() {
        let install_dir = TempDir::new().unwrap();
        let output_dir = TempDir::new().unwrap();

        // Use the TempDir path itself as the output_path — it is an existing
        // directory, so rename(tmp, output_path) will fail with EISDIR on macOS/Linux.
        let bad_output = output_dir.path().to_path_buf();

        let opts = BackupOptions {
            install_dir: install_dir.path().to_path_buf(),
            output_path: bad_output.clone(),
            db_path: None,
        };

        // The backup must fail because we cannot rename a file onto a directory.
        let result = BackupEngine::new().run(opts).await;
        assert!(result.is_err(), "backup to a directory path should fail");

        // The .tmp scratch file must have been cleaned up.
        let tmp = bad_output.with_extension("tar.gz.tmp");
        assert!(
            !tmp.exists(),
            "no partial .tmp file should remain after failure"
        );
    }

    // ── RestoreEngine unit tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_restore_engine_restores_files() {
        let archive_bytes = make_test_archive(&[
            ("config.toml", b"provider = \"anthropic\""),
            ("agents/default/SOUL.md", b"# Soul"),
        ]);

        let dir = TempDir::new().unwrap();
        let archive_path = dir.path().join("backup.tar.gz");
        std::fs::write(&archive_path, &archive_bytes).unwrap();

        let install_dir = TempDir::new().unwrap();
        let opts = RestoreOptions {
            archive_path,
            install_dir: install_dir.path().to_path_buf(),
            force: true,
        };

        let result = RestoreEngine::new().run(opts).await.unwrap();
        assert_eq!(result.restored_count, 2);
        assert!(install_dir.path().join("config.toml").exists());
        assert!(install_dir.path().join("agents/default/SOUL.md").exists());
    }

    #[tokio::test]
    async fn test_restore_aborts_on_corrupted_archive() {
        let dir = TempDir::new().unwrap();
        let archive_path = dir.path().join("corrupt.tar.gz");
        std::fs::write(&archive_path, b"not a valid gzip stream").unwrap();

        let install_dir = TempDir::new().unwrap();
        let opts = RestoreOptions {
            archive_path,
            install_dir: install_dir.path().to_path_buf(),
            force: true,
        };

        let result = RestoreEngine::new().run(opts).await;
        assert!(result.is_err(), "corrupted archive should fail");
        // install_dir should be empty
        let count = std::fs::read_dir(install_dir.path()).unwrap().count();
        assert_eq!(count, 0, "install_dir should be untouched");
    }

    #[tokio::test]
    async fn test_restore_missing_archive_errors() {
        let dir = TempDir::new().unwrap();
        let opts = RestoreOptions {
            archive_path: dir.path().join("nonexistent.tar.gz"),
            install_dir: dir.path().to_path_buf(),
            force: true,
        };
        let result = RestoreEngine::new().run(opts).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // ── list_backups tests ─────────────────────────────────────────────────────

    #[test]
    fn test_list_backups_empty() {
        let dir = TempDir::new().unwrap();
        let result = list_backups(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_list_backups_returns_metadata() {
        let dir = TempDir::new().unwrap();

        let archive1 = make_test_archive(&[("config.toml", b"a")]);
        let archive2 = make_test_archive(&[("config.toml", b"b"), ("agents/SOUL.md", b"c")]);

        std::fs::write(dir.path().join("backup1.tar.gz"), &archive1).unwrap();
        std::fs::write(dir.path().join("backup2.tar.gz"), &archive2).unwrap();

        let mut infos = list_backups(dir.path()).unwrap();
        infos.sort_by_key(|i| i.path.file_name().unwrap().to_owned());

        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].entry_count, 1);
        assert_eq!(infos[1].entry_count, 2);
    }

    #[test]
    fn test_list_backups_nonexistent_dir() {
        let result = list_backups(Path::new("/tmp/does-not-exist-xyz-assistant-test"));
        assert!(result.unwrap().is_empty());
    }
}
