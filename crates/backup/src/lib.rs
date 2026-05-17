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

use anyhow::{Context, Result, bail};
use assistant_core::clock::{Clock, SystemClock};
use async_trait::async_trait;
use tracing::{info, warn};

use crate::archive::{extract_tar_gz, read_tar_gz_manifest, write_tar_gz};
use crate::checksum::sha256_hex;
use crate::manifest::{BackupManifest, MANIFEST_VERSION, ManifestEntry};
use crate::paths::{
    checkpoint_sqlite, default_archive_name, default_backups_dir, default_install_dir,
    discover_files,
};

// -- BackupFs trait --

/// Filesystem abstraction used by [`BackupEngine`] and [`RestoreEngine`].
///
/// `RealFs` delegates to `tokio::fs`; `FakeFs` uses an in-memory map.
/// The `async_trait` macro is used because async fns in traits require it
/// (per project conventions in AGENTS.md).
#[async_trait]
pub trait BackupFs: Send + Sync {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<()>;
    async fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    async fn file_exists(&self, path: &Path) -> bool;
    async fn file_size(&self, path: &Path) -> Result<u64>;
}

// -- RealFs --

/// Production [`BackupFs`] implementation backed by `tokio::fs`.
#[derive(Default)]
pub struct RealFs;

#[async_trait]
impl BackupFs for RealFs {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        tokio::fs::read(path)
            .await
            .with_context(|| format!("reading {:?}", path))
    }

    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating dir {:?}", parent))?;
        }
        tokio::fs::write(path, data)
            .await
            .with_context(|| format!("writing {:?}", path))
    }

    async fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut rd = tokio::fs::read_dir(path)
            .await
            .with_context(|| format!("listing {:?}", path))?;
        let mut entries = Vec::new();
        while let Some(e) = rd.next_entry().await? {
            entries.push(e.path());
        }
        Ok(entries)
    }

    async fn file_exists(&self, path: &Path) -> bool {
        tokio::fs::metadata(path).await.is_ok()
    }

    async fn file_size(&self, path: &Path) -> Result<u64> {
        Ok(tokio::fs::metadata(path)
            .await
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
        // Recover from a poisoned lock: this is an in-memory test fixture
        // with no invariants beyond the HashMap itself, so a previous panic
        // while holding the lock does not invalidate the data.
        self.files
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(path.into(), data.into());
    }
}

#[async_trait]
impl BackupFs for FakeFs {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        self.files
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(path)
            .cloned()
            .with_context(|| format!("FakeFs: file not found: {:?}", path))
    }

    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        self.files
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(path.to_path_buf(), data.to_vec());
        Ok(())
    }

    async fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let guard = self
            .files
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let entries: Vec<PathBuf> = guard
            .keys()
            .filter(|k| k.parent() == Some(path))
            .cloned()
            .collect();
        Ok(entries)
    }

    async fn file_exists(&self, path: &Path) -> bool {
        self.files
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains_key(path)
    }

    async fn file_size(&self, path: &Path) -> Result<u64> {
        self.files
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
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
pub struct BackupEngine {
    fs: Arc<dyn BackupFs>,
}

impl BackupEngine {
    /// Create with the production (`tokio::fs`) filesystem.
    pub fn new() -> Self {
        Self {
            fs: Arc::new(RealFs),
        }
    }

    /// Create with a custom filesystem implementation (e.g. [`FakeFs`] for tests).
    pub fn with_fs(fs: Arc<dyn BackupFs>) -> Self {
        Self { fs }
    }

    /// Run the backup operation.
    pub async fn run(&self, opts: BackupOptions) -> Result<BackupResult> {
        info!("starting backup of {:?}", opts.install_dir);
        let start = std::time::Instant::now();

        // Guard: reject output paths that target files inside the live installation
        // (other than the dedicated backups/ subdirectory) to prevent overwriting
        // source files with the archive being created.
        if opts.output_path.starts_with(&opts.install_dir) {
            let backups_subdir = opts.install_dir.join("backups");
            if !opts.output_path.starts_with(&backups_subdir) {
                bail!(
                    "output path {:?} is inside the installation directory but not \
                     under backups/ — this would overwrite a source file",
                    opts.output_path
                );
            }
        }

        // Ensure output parent directory exists
        if let Some(parent) = opts.output_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating output directory {:?}", parent))?;
        }

        // Checkpoint SQLite WAL before copying
        let db_path = opts
            .db_path
            .clone()
            .unwrap_or_else(|| opts.install_dir.join("assistant.db"));
        if self.fs.file_exists(&db_path).await
            && let Err(e) = checkpoint_sqlite(&db_path).await
        {
            warn!("WAL checkpoint failed (continuing): {}", e);
        }

        // Discover files
        let file_paths = discover_files(&opts.install_dir, opts.db_path.as_deref())
            .context("discovering installation files")?;

        // Read all file contents and build manifest entries
        let mut file_data: Vec<(String, Vec<u8>)> = Vec::new();
        let mut manifest_entries: Vec<ManifestEntry> = Vec::new();

        for abs_path in &file_paths {
            let data = self
                .fs
                .read_file(abs_path)
                .await
                .with_context(|| format!("reading {:?}", abs_path))?;

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
            created_at: SystemClock.now().to_rfc3339(),
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
pub struct RestoreEngine {
    fs: Arc<dyn BackupFs>,
}

impl RestoreEngine {
    /// Create with the production (`tokio::fs`) filesystem.
    pub fn new() -> Self {
        Self {
            fs: Arc::new(RealFs),
        }
    }

    /// Create with a custom filesystem implementation (e.g. [`FakeFs`] for tests).
    pub fn with_fs(fs: Arc<dyn BackupFs>) -> Self {
        Self { fs }
    }

    /// Run the restore operation.
    pub async fn run(&self, opts: RestoreOptions) -> Result<RestoreResult> {
        info!("starting restore from {:?}", opts.archive_path);

        // Validate archive exists
        if !self.fs.file_exists(&opts.archive_path).await {
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
            let non_empty = self.fs.file_exists(&opts.install_dir).await
                && self
                    .fs
                    .list_dir(&opts.install_dir)
                    .await
                    .map(|entries| !entries.is_empty())
                    .unwrap_or(false);

            if non_empty {
                confirm_restore(&manifest, &opts.install_dir)?;
            }
        }

        // Extract
        let (warnings, restored_count) =
            extract_tar_gz(&opts.archive_path, &opts.install_dir, &manifest)
                .context("extracting archive")?;

        for w in &warnings {
            warn!("{}", w);
        }

        info!(
            "restore complete: {} files restored to {:?}",
            restored_count, opts.install_dir
        );

        Ok(RestoreResult {
            restored_count,
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
pub async fn list_backups(backup_dir: &Path) -> Result<Vec<BackupInfo>> {
    if tokio::fs::metadata(backup_dir).await.is_err() {
        return Ok(vec![]);
    }

    let mut infos: Vec<BackupInfo> = Vec::new();
    let mut rd = tokio::fs::read_dir(backup_dir)
        .await
        .with_context(|| format!("listing {:?}", backup_dir))?;

    while let Some(entry) = rd.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("gz") {
            continue;
        }

        let archive_size = entry.metadata().await.map(|m| m.len()).unwrap_or(0);

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

// -- Pre-migration helper ----------------------------------------------------

/// Create a `pre-migration-<timestamp>.tar.gz` snapshot of a legacy
/// single-user installation directory.
///
/// Used by callers that need to back up `~/.assistant/` (or another install
/// dir) before running the legacy → multi-org migration. Encapsulates the
/// `backups/` directory creation and timestamped naming so the storage crate
/// can perform the migration without taking a backup-crate dependency.
///
/// Returns the path to the created archive.
pub async fn backup_legacy_install(base_path: &Path) -> Result<PathBuf> {
    let backups_dir = base_path.join("backups");
    tokio::fs::create_dir_all(&backups_dir)
        .await
        .with_context(|| format!("creating backups directory: {}", backups_dir.display()))?;

    // Millisecond precision (`%Y%m%d-%H%M%S%.3f` would include the dot;
    // we drop it to keep filenames POSIX-friendly) so two invocations within
    // the same second don't collide on archive name.
    let now = SystemClock.now();
    let archive_name = format!(
        "pre-migration-{}-{:03}.tar.gz",
        now.format("%Y%m%d-%H%M%S"),
        now.timestamp_subsec_millis()
    );
    let output_path = backups_dir.join(&archive_name);

    let opts = BackupOptions {
        install_dir: base_path.to_path_buf(),
        output_path: output_path.clone(),
        db_path: Some(base_path.join("assistant.db")),
    };

    let result = BackupEngine::new()
        .run(opts)
        .await
        .context("creating pre-migration backup")?;

    info!(
        "pre-migration backup created: {} ({} files, {} bytes)",
        result.output_path.display(),
        result.entry_count,
        result.archive_size
    );

    Ok(result.output_path)
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

    #[tokio::test]
    async fn test_list_backups_empty() {
        let dir = TempDir::new().unwrap();
        let result = list_backups(dir.path()).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_list_backups_returns_metadata() {
        let dir = TempDir::new().unwrap();

        let archive1 = make_test_archive(&[("config.toml", b"a")]);
        let archive2 = make_test_archive(&[("config.toml", b"b"), ("agents/SOUL.md", b"c")]);

        std::fs::write(dir.path().join("backup1.tar.gz"), &archive1).unwrap();
        std::fs::write(dir.path().join("backup2.tar.gz"), &archive2).unwrap();

        let mut infos = list_backups(dir.path()).await.unwrap();
        infos.sort_by_key(|i| i.path.file_name().unwrap().to_owned());

        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].entry_count, 1);
        assert_eq!(infos[1].entry_count, 2);
    }

    #[tokio::test]
    async fn test_list_backups_nonexistent_dir() {
        let result = list_backups(Path::new("/tmp/does-not-exist-xyz-assistant-test")).await;
        assert!(result.unwrap().is_empty());
    }
}
