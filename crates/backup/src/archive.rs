//! tar.gz archive creation and extraction helpers.

use std::io::{Read, Write};
use std::path::Path;

use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Archive;
use tracing::debug;

use crate::checksum::sha256_hex;
use crate::manifest::BackupManifest;

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
        let file =
            std::fs::File::create(&tmp_path).with_context(|| format!("creating {:?}", tmp_path))?;
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

/// Append raw bytes as a tar entry with the given name.
fn append_bytes<W: Write>(tar: &mut tar::Builder<W>, name: &str, data: &[u8]) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
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
/// Returns a list of non-fatal warning strings.
pub fn extract_tar_gz(
    archive_path: &Path,
    install_dir: &Path,
    manifest: &BackupManifest,
) -> Result<Vec<String>> {
    let file = std::fs::File::open(archive_path)
        .with_context(|| format!("opening archive {:?}", archive_path))?;
    let gz = GzDecoder::new(file);
    let mut tar = Archive::new(gz);

    let mut warnings: Vec<String> = Vec::new();

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

        // Path-traversal guard
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

        // Read content
        let mut data = Vec::new();
        entry.read_to_end(&mut data).context("reading entry data")?;

        // Verify checksum
        if let Some(me) = manifest_entry {
            let actual = sha256_hex(&data);
            if actual != me.sha256 {
                bail!(
                    "checksum mismatch for '{}': expected {}, got {}",
                    archive_path_str,
                    me.sha256,
                    actual
                );
            }

            // Restore to the original install_path if it is under install_dir,
            // otherwise restore relative to install_dir using archive_path.
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

            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating directory {:?}", parent))?;
            }
            std::fs::write(&dest, &data).with_context(|| format!("writing {:?}", dest))?;
            debug!("restored {:?}", dest);
        } else {
            // Entry not in manifest — restore relative to install_dir
            let dest = install_dir.join(&archive_path_str);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating directory {:?}", parent))?;
            }
            std::fs::write(&dest, &data).with_context(|| format!("writing {:?}", dest))?;
        }
    }

    Ok(warnings)
}

/// Build a minimal in-memory `.tar.gz` archive for testing.
///
/// Callers provide `(archive_path, content)` pairs; a manifest is auto-generated.
#[cfg(test)]
pub fn make_test_archive(files: &[(&str, &[u8])]) -> Vec<u8> {
    use crate::checksum::sha256_hex;
    use crate::manifest::{BackupManifest, ManifestEntry, MANIFEST_VERSION};

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

use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_and_read_manifest() {
        use crate::manifest::{BackupManifest, ManifestEntry, MANIFEST_VERSION};

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
        use crate::manifest::{BackupManifest, ManifestEntry, MANIFEST_VERSION};

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

        let warnings = extract_tar_gz(&archive_path, install_dir.path(), &manifest).unwrap();
        assert!(
            warnings.iter().any(|w| w.contains("unsafe path")),
            "expected path-traversal warning, got: {:?}",
            warnings
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
        use crate::manifest::{BackupManifest, ManifestEntry, MANIFEST_VERSION};

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
