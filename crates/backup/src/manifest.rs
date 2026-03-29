//! Backup manifest types — serialised as `manifest.json` inside every archive.

use serde::{Deserialize, Serialize};

/// Current manifest schema version.
pub const MANIFEST_VERSION: u32 = 1;

/// Top-level manifest embedded as the first tar entry (`manifest.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Schema version (currently `1`).
    pub version: u32,
    /// Semver string of the binary that created the backup.
    pub app_version: String,
    /// RFC 3339 UTC timestamp of when the backup was initiated.
    pub created_at: String,
    /// Absolute path of the installation directory at backup time.
    pub install_dir: String,
    /// One entry per file included in the archive.
    pub entries: Vec<ManifestEntry>,
}

/// Metadata for a single file in the backup archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Relative path within the tar archive (no `..` components).
    pub archive_path: String,
    /// Original absolute path on disk (used during restore).
    pub install_path: String,
    /// Uncompressed file size in bytes.
    pub size_bytes: u64,
    /// Lowercase hex-encoded SHA-256 of the uncompressed content.
    pub sha256: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_round_trip() {
        let manifest = BackupManifest {
            version: MANIFEST_VERSION,
            app_version: "0.1.63".to_string(),
            created_at: "2026-03-29T14:30:22Z".to_string(),
            install_dir: "/home/user/.assistant".to_string(),
            entries: vec![ManifestEntry {
                archive_path: "config.toml".to_string(),
                install_path: "/home/user/.assistant/config.toml".to_string(),
                size_bytes: 1024,
                sha256: "a".repeat(64),
            }],
        };

        let json = serde_json::to_string(&manifest).expect("serialize");
        let decoded: BackupManifest = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(decoded.version, manifest.version);
        assert_eq!(decoded.app_version, manifest.app_version);
        assert_eq!(decoded.created_at, manifest.created_at);
        assert_eq!(decoded.install_dir, manifest.install_dir);
        assert_eq!(decoded.entries.len(), 1);
        assert_eq!(decoded.entries[0].archive_path, "config.toml");
        assert_eq!(decoded.entries[0].size_bytes, 1024);
    }

    #[test]
    fn test_manifest_empty_entries_serializes() {
        let manifest = BackupManifest {
            version: 1,
            app_version: "0.1.0".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            install_dir: "/tmp".into(),
            entries: vec![],
        };
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("\"entries\":[]"));
    }
}
