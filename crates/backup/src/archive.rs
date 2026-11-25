//! tar.gz archive creation and extraction helpers.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use tar::Archive;
use tracing::{debug, warn};

use crate::checksum::sha256_hex;
use crate::manifest::BackupManifest;
use crate::paths::{is_private_file, is_sqlite_db};

/// Write a `.tar.gz` archive to `output_path`.
///
/// The manifest is serialised as `manifest.json` and stored as the **first**
/// entry in the archive.  Each `(archive_path, data)` pair follows in order.
///
/// Returns the compressed size of the produced file in bytes.
pub fn write_tar_gz(
    manifest: &BackupManifest,
    files: &[(&str, &[u8])],
    output_path: &Path,
) -> Result<u64> {
    let tmp_path = output_path.with_extension("tar.gz.tmp");

    let result = (|| -> Result<u64> {
        // The archive holds `secrets.key` and `jwt_secret`, so keep it private.
        let file = create_private(&tmp_path).with_context(|| format!("creating {:?}", tmp_path))?;
        let gz = GzEncoder::new(file, Compression::default());
        let mut tar = tar::Builder::new(gz);

        // 1. Manifest first
        let manifest_bytes = serde_json::to_vec_pretty(manifest).context("serialising manifest")?;
        append_bytes(&mut tar, "manifest.json", &manifest_bytes)?;

        // 2. File entries
        for (archive_path, data) in files {
            append_bytes(&mut tar, archive_path, data)?;
        }

        let gz = tar.into_inner().context("finalising tar")?;
        let file = gz.finish().context("finalising gzip")?;
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        Ok(size)
    })();

    match result {
        Ok(size) => {
            if let Err(e) = std::fs::rename(&tmp_path, output_path) {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(e)
                    .with_context(|| format!("renaming {:?} → {:?}", tmp_path, output_path));
            }
            Ok(size)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            Err(e)
        }
    }
}

/// Mode for key material, and for the archive that holds it.
const PRIVATE_MODE: u32 = 0o600;
/// Mode for archive members and for restored files that did not exist before.
const DEFAULT_MODE: u32 = 0o644;

/// Create the archive's temporary file, readable by the owner only on Unix.
///
/// A leftover file at this path (from a crashed run) is removed first: its
/// mode would survive a plain open, and `create_new` refuses to follow a
/// symlink planted there.
fn create_private(path: &Path) -> std::io::Result<std::fs::File> {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(PRIVATE_MODE);
    }
    options.open(path)
}

/// Mode a restored file gets: `0600` for key material, otherwise the mode of
/// the file it replaces (an `org.toml` the user locked down stays locked
/// down), otherwise [`DEFAULT_MODE`].
#[cfg(unix)]
fn restored_mode(dest: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    if is_private_file(dest) {
        return PRIVATE_MODE;
    }
    std::fs::metadata(dest)
        .map(|m| m.permissions().mode() & 0o7777)
        .unwrap_or(DEFAULT_MODE)
}

/// `path` with `suffix` appended to its file name.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// A file moved out of the way during a database replacement.
struct SetAside {
    original: PathBuf,
    aside: PathBuf,
}

/// Move `db`'s `-wal` and `-shm` files to unique, newly created names next to
/// them. Undoes its own moves if one fails.
fn set_sidecars_aside(db: &Path) -> Result<Vec<SetAside>> {
    let parent = db.parent().context("database path has no parent")?;
    let mut moved: Vec<SetAside> = Vec::new();
    for suffix in ["-wal", "-shm"] {
        let original = with_suffix(db, suffix);
        if std::fs::symlink_metadata(&original).is_err() {
            continue;
        }
        // Claim a unique name by creating it, then rename over our own file.
        let claimed = tempfile::Builder::new()
            .prefix(".pre-restore-")
            .suffix(suffix)
            .tempfile_in(parent)
            .and_then(|f| f.keep().map_err(|e| e.error))
            .map(|(_, path)| path);
        let moved_one = claimed
            .and_then(|aside| std::fs::rename(&original, &aside).map(|()| aside))
            .with_context(|| format!("setting {original:?} aside"));
        match moved_one {
            Ok(aside) => moved.push(SetAside { original, aside }),
            Err(e) => {
                put_back(&moved);
                return Err(e);
            }
        }
    }
    Ok(moved)
}

/// Move files set aside by [`set_sidecars_aside`] back to where they were.
fn put_back(moved: &[SetAside]) {
    for m in moved {
        if let Err(e) = std::fs::rename(&m.aside, &m.original) {
            warn!(
                "could not put {:?} back at {:?}: {}; the file is kept at the first path",
                m.aside, m.original, e
            );
        }
    }
}

/// Write one restored file atomically.
///
/// The data goes to a uniquely named staging file in the destination
/// directory, created with `create_new` so a user file is never touched, and
/// is fsynced. For a database, the old `-wal`/`-shm` files are then renamed
/// aside (never deleted yet), the staging file is renamed over `dest`, and
/// only then are the asides deleted.
///
/// That order is what keeps a crash or a failed rename safe. Until the rename,
/// no byte of the old database or its WAL is destroyed, and a failed rename
/// puts the sidecars back. At no point do the new database and the old WAL sit
/// side by side, which would make SQLite replay stale frames onto it. A crash
/// between the two renames leaves the old database without its WAL, which
/// survives under a `.pre-restore-*` name.
fn write_restored(dest: &Path, data: &[u8]) -> Result<()> {
    let parent = dest.parent().context("restore destination has no parent")?;
    let mut prefix = std::ffi::OsString::from(".");
    prefix.push(dest.file_name().unwrap_or_default());
    prefix.push(".restore-");

    let mut staging = tempfile::Builder::new()
        .prefix(&prefix)
        .suffix(".tmp")
        .tempfile_in(parent)
        .with_context(|| format!("creating a staging file in {parent:?}"))?;
    staging
        .write_all(data)
        .and_then(|()| staging.as_file().sync_all())
        .with_context(|| format!("writing {:?}", staging.path()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        staging
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(restored_mode(dest)))
            .with_context(|| format!("setting the mode of {:?}", staging.path()))?;
    }

    let moved = if is_sqlite_db(dest) {
        set_sidecars_aside(dest)?
    } else {
        Vec::new()
    };
    if let Err(e) = staging.persist(dest) {
        put_back(&moved);
        return Err(e.error).with_context(|| format!("replacing {dest:?}"));
    }
    for m in &moved {
        if let Err(e) = std::fs::remove_file(&m.aside) {
            warn!("could not remove {:?}: {}", m.aside, e);
        }
    }
    Ok(())
}

/// Fsync every directory whose entries restore changed, so renamed files and
/// newly created directories survive a crash. That is each ancestor of a
/// restored file up to `install_dir`, and `install_dir`'s own parent.
#[cfg(unix)]
fn sync_restored_dirs<'a>(
    install_dir: &Path,
    dests: impl Iterator<Item = &'a PathBuf>,
) -> Result<()> {
    let mut dirs: std::collections::BTreeSet<&Path> = std::collections::BTreeSet::new();
    for dest in dests {
        dirs.extend(
            dest.ancestors()
                .skip(1)
                .take_while(|d| d.starts_with(install_dir)),
        );
    }
    dirs.extend(install_dir.parent());
    for dir in dirs {
        std::fs::File::open(dir)
            .and_then(|d| d.sync_all())
            .with_context(|| format!("syncing directory {dir:?}"))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn sync_restored_dirs<'a>(
    _install_dir: &Path,
    _dests: impl Iterator<Item = &'a PathBuf>,
) -> Result<()> {
    Ok(())
}

/// Refuse to restore through a symlink: neither `dest` nor any existing
/// directory between `install_dir` and `dest` may be one. A symlink there
/// would redirect the write outside the installation.
fn ensure_no_symlink(install_dir: &Path, dest: &Path) -> Result<()> {
    let rel = dest
        .strip_prefix(install_dir)
        .with_context(|| format!("{dest:?} is outside {install_dir:?}"))?;
    let mut current = install_dir.to_path_buf();
    for component in rel.components() {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                bail!("refusing to restore {dest:?}: {current:?} is a symlink")
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(e).with_context(|| format!("inspecting {current:?}")),
        }
    }
    Ok(())
}

/// Append raw bytes as a tar entry with the given name.
///
/// Key material gets mode `0600`, because `tar xzf` extracts members with their
/// archived mode (less the umask).
fn append_bytes<W: Write>(tar: &mut tar::Builder<W>, name: &str, data: &[u8]) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(if is_private_file(Path::new(name)) {
        PRIVATE_MODE
    } else {
        DEFAULT_MODE
    });
    header.set_cksum();
    tar.append_data(&mut header, name, data)
        .with_context(|| format!("appending '{}' to archive", name))?;
    Ok(())
}

/// Read **only** the `manifest.json` entry from a `.tar.gz` archive without
/// extracting the rest of the contents.
///
/// This is used for pre-restore integrity checks and for `backup list`.
pub fn read_tar_gz_manifest(archive_path: &Path) -> Result<BackupManifest> {
    let file = std::fs::File::open(archive_path)
        .with_context(|| format!("opening archive {:?}", archive_path))?;
    let gz = GzDecoder::new(file);
    let mut tar = Archive::new(gz);

    for entry in tar.entries().context("reading tar entries")? {
        let mut entry = entry.context("reading tar entry")?;
        let path = entry.path().context("reading entry path")?.into_owned();
        if path.to_str() == Some("manifest.json") {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .context("reading manifest.json")?;
            let manifest: BackupManifest =
                serde_json::from_slice(&buf).context("parsing manifest.json")?;
            return Ok(manifest);
        }
    }

    bail!("archive does not contain manifest.json")
}

/// Extract all entries from a `.tar.gz` archive into `install_dir`.
///
/// Each entry is verified against the corresponding `ManifestEntry` checksum.
/// Entries with `..` path components are skipped with a warning (path-traversal guard).
///
/// Returns `(warnings, written_count)` where `warnings` is a list of non-fatal
/// messages for skipped entries and `written_count` is the number of files
/// actually written to disk (may be less than `manifest.entries.len()` when
/// entries are skipped by the path-traversal guard).
pub fn extract_tar_gz(
    archive_path: &Path,
    install_dir: &Path,
    manifest: &BackupManifest,
) -> Result<(Vec<String>, usize)> {
    let file = std::fs::File::open(archive_path)
        .with_context(|| format!("opening archive {:?}", archive_path))?;
    let gz = GzDecoder::new(file);
    let mut tar = Archive::new(gz);

    let mut warnings: Vec<String> = Vec::new();

    // Phase 1 — verify every entry without touching the live install tree.
    // All file data is collected in `to_write`; the live tree is not modified
    // until every checksum has passed.
    let mut to_write: Vec<(PathBuf, Vec<u8>)> = Vec::new();

    for entry in tar.entries().context("reading tar entries")? {
        let mut entry = entry.context("reading tar entry")?;
        let archive_path_str = entry
            .path()
            .context("reading entry path")?
            .to_string_lossy()
            .to_string();

        // Skip manifest — it is not a file to restore
        if archive_path_str == "manifest.json" {
            continue;
        }

        // Path-traversal guard (first-line: raw entry path)
        if archive_path_str.contains("..") {
            let msg = format!("skipped entry with unsafe path: {}", archive_path_str);
            warnings.push(msg);
            continue;
        }

        // Find the corresponding manifest entry
        let manifest_entry = manifest
            .entries
            .iter()
            .find(|e| e.archive_path == archive_path_str);

        // Read content into memory
        let mut data = Vec::new();
        entry.read_to_end(&mut data).context("reading entry data")?;

        if let Some(me) = manifest_entry {
            // Verify checksum before accepting this entry
            let actual = sha256_hex(&data);
            if actual != me.sha256 {
                bail!(
                    "checksum mismatch for '{}': expected {}, got {}",
                    archive_path_str,
                    me.sha256,
                    actual
                );
            }

            // Resolve destination
            let dest = if me
                .install_path
                .starts_with(install_dir.to_string_lossy().as_ref())
            {
                PathBuf::from(&me.install_path)
            } else {
                install_dir.join(&archive_path_str)
            };

            // Dest-escape guard: reject any path whose components include `..`
            // or that does not sit inside install_dir (catches crafted install_path).
            if dest
                .components()
                .any(|c| c == std::path::Component::ParentDir)
                || !dest.starts_with(install_dir)
            {
                let msg = format!("skipped entry with unsafe path: {}", archive_path_str);
                warnings.push(msg);
                continue;
            }

            ensure_no_symlink(install_dir, &dest)?;
            to_write.push((dest, data));
        } else {
            // Entry not declared in manifest.json — treat as corruption and abort.
            bail!(
                "archive entry '{}' is not declared in manifest.json",
                archive_path_str
            );
        }
    }

    // Phase 2 — all entries verified; now write to the live install tree.
    // If any write fails here the install tree may be partially updated, but
    // all content integrity has already been confirmed.
    for (dest, data) in &to_write {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {:?}", parent))?;
        }
        write_restored(dest, data)?;
        debug!("restored {:?}", dest);
    }
    sync_restored_dirs(install_dir, to_write.iter().map(|(dest, _)| dest))?;

    Ok((warnings, to_write.len()))
}

/// Build a minimal in-memory `.tar.gz` archive for testing.
///
/// Callers provide `(archive_path, content)` pairs; a manifest is auto-generated.
#[cfg(test)]
pub fn make_test_archive(files: &[(&str, &[u8])]) -> Vec<u8> {
    use crate::checksum::sha256_hex;
    use crate::manifest::{BackupManifest, MANIFEST_VERSION, ManifestEntry};

    let entries: Vec<ManifestEntry> = files
        .iter()
        .map(|(path, data)| ManifestEntry {
            archive_path: path.to_string(),
            install_path: format!("/tmp/test/{}", path),
            size_bytes: data.len() as u64,
            sha256: sha256_hex(data),
        })
        .collect();

    let manifest = BackupManifest {
        version: MANIFEST_VERSION,
        app_version: "0.0.0-test".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        install_dir: "/tmp/test".into(),
        entries,
    };

    let buf = Vec::new();
    let gz = GzEncoder::new(buf, Compression::default());
    let mut tar = tar::Builder::new(gz);

    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    append_bytes(&mut tar, "manifest.json", &manifest_bytes).unwrap();

    for (path, data) in files {
        append_bytes(&mut tar, path, data).unwrap();
    }

    let gz = tar.into_inner().unwrap();
    gz.finish().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_and_read_manifest() {
        use crate::manifest::{BackupManifest, MANIFEST_VERSION, ManifestEntry};

        let dir = TempDir::new().unwrap();
        let archive_path = dir.path().join("test.tar.gz");

        let manifest = BackupManifest {
            version: MANIFEST_VERSION,
            app_version: "0.1.0".into(),
            created_at: "2026-03-29T00:00:00Z".into(),
            install_dir: "/tmp".into(),
            entries: vec![ManifestEntry {
                archive_path: "hello.txt".into(),
                install_path: "/tmp/hello.txt".into(),
                size_bytes: 5,
                sha256: sha256_hex(b"hello"),
            }],
        };

        let size = write_tar_gz(&manifest, &[("hello.txt", b"hello")], &archive_path).unwrap();
        assert!(size > 0);
        assert!(archive_path.exists());

        let decoded = read_tar_gz_manifest(&archive_path).unwrap();
        assert_eq!(decoded.version, MANIFEST_VERSION);
        assert_eq!(decoded.entries.len(), 1);
        assert_eq!(decoded.entries[0].archive_path, "hello.txt");
    }

    #[test]
    fn test_path_traversal_guard() {
        use crate::manifest::{BackupManifest, MANIFEST_VERSION, ManifestEntry};

        // The tar crate rejects '..' components on append_data, so we cannot
        // inject unsafe paths via the archive itself.  Instead we craft a manifest
        // whose install_path uses '..' to escape install_dir (e.g. a maliciously
        // crafted backup file).  The dest-escape guard in extract_tar_gz must catch
        // this before any write occurs.

        let archive_dir = TempDir::new().unwrap();
        let install_dir = TempDir::new().unwrap();

        // Build a valid archive with a safe filename
        let archive_bytes = make_test_archive(&[("evil.txt", b"data")]);
        let archive_path = archive_dir.path().join("test.tar.gz");
        std::fs::write(&archive_path, &archive_bytes).unwrap();

        // Craft a manifest whose install_path escapes install_dir via '..'
        // e.g. "<install_dir>/../../etc/passwd" — the string starts with install_dir
        // so the prefix check passes, but the PathBuf contains ParentDir components.
        let escape_install_path = format!("{}/../../etc/passwd", install_dir.path().display());
        let manifest = BackupManifest {
            version: MANIFEST_VERSION,
            app_version: "0.0.0".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            install_dir: install_dir.path().to_string_lossy().to_string(),
            entries: vec![ManifestEntry {
                archive_path: "evil.txt".into(),
                install_path: escape_install_path,
                size_bytes: 4,
                sha256: crate::checksum::sha256_hex(b"data"),
            }],
        };

        let (warnings, written) =
            extract_tar_gz(&archive_path, install_dir.path(), &manifest).unwrap();
        assert!(
            warnings.iter().any(|w| w.contains("unsafe path")),
            "expected path-traversal warning, got: {:?}",
            warnings
        );
        assert_eq!(
            written, 0,
            "no files should be written when entry is skipped"
        );
        // Nothing must have been written inside install_dir either
        assert_eq!(
            std::fs::read_dir(install_dir.path()).unwrap().count(),
            0,
            "install_dir must remain empty"
        );
    }

    #[test]
    fn test_corrupted_archive_fails() {
        let dir = TempDir::new().unwrap();
        let archive_path = dir.path().join("corrupt.tar.gz");
        std::fs::write(&archive_path, b"not a valid gzip stream").unwrap();

        let result = read_tar_gz_manifest(&archive_path);
        assert!(result.is_err(), "corrupted archive should return Err");
    }

    #[test]
    fn test_checksum_mismatch_rejected() {
        use crate::manifest::{BackupManifest, MANIFEST_VERSION, ManifestEntry};

        let dir = TempDir::new().unwrap();
        let archive_path = dir.path().join("mismatch.tar.gz");

        // Build archive with real content "hello"
        let manifest = BackupManifest {
            version: MANIFEST_VERSION,
            app_version: "0.0.0".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            install_dir: "/tmp".into(),
            entries: vec![ManifestEntry {
                archive_path: "file.txt".into(),
                install_path: "/tmp/file.txt".into(),
                size_bytes: 5,
                sha256: "a".repeat(64), // deliberately wrong checksum
            }],
        };
        write_tar_gz(&manifest, &[("file.txt", b"hello")], &archive_path).unwrap();

        let result = extract_tar_gz(&archive_path, dir.path(), &manifest);
        assert!(result.is_err(), "checksum mismatch should return Err");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("checksum mismatch"), "got: {}", err);
    }
}
