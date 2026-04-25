//! CLI credential storage and token refresh.
//!
//! Stores OAuth2 tokens in `~/.assistant/credentials.json` (mode 0600).
//! Provides transparent token refresh when the access token has expired.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

// -- Types --

/// Persisted OAuth2 credentials for a CLI session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CliCredentials {
    /// Base URL of the assistant server (e.g. `http://127.0.0.1:8080`).
    pub server_url: String,
    /// OAuth2 client ID obtained from dynamic registration.
    pub client_id: String,
    /// JWT access token.
    pub access_token: String,
    /// Opaque refresh token (used to obtain a new access token).
    pub refresh_token: String,
    /// When the access token expires.
    pub expires_at: DateTime<Utc>,
}

impl CliCredentials {
    /// Returns `true` if the access token has expired (or will expire
    /// within the next 30 seconds).
    pub fn is_expired(&self) -> bool {
        Utc::now() + chrono::Duration::seconds(30) >= self.expires_at
    }
}

// -- Persistence --

/// Default credentials file path: `~/.assistant/credentials.json`.
pub fn credentials_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".assistant").join("credentials.json"))
}

/// Load credentials from disk. Returns `Ok(None)` if the file doesn't exist.
pub fn load(path: &Path) -> Result<Option<CliCredentials>> {
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let creds: CliCredentials = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(creds))
}

/// Save credentials to disk with restrictive permissions (0600).
pub fn save(path: &Path, creds: &CliCredentials) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let data = serde_json::to_string_pretty(creds)?;
    std::fs::write(path, &data).with_context(|| format!("failed to write {}", path.display()))?;

    // Set file permissions to 0600 (owner read/write only).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("failed to set permissions on {}", path.display()))?;
    }

    Ok(())
}

/// Delete the credentials file.
pub fn remove(path: &Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path)
            .with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

// -- Token refresh --

/// Attempt to refresh the access token using the refresh token.
///
/// On success, updates the credentials in-place and persists to disk.
/// Returns `Err` if the refresh fails (caller should re-authenticate).
pub async fn refresh_if_needed(
    path: &Path,
    creds: &mut CliCredentials,
    http: &reqwest::Client,
) -> Result<()> {
    if !creds.is_expired() {
        return Ok(());
    }

    debug!("access token expired, attempting refresh");

    let url = format!("{}/oauth/token", creds.server_url.trim_end_matches('/'));
    let resp = http
        .post(&url)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &creds.refresh_token),
            ("client_id", &creds.client_id),
        ])
        .send()
        .await
        .context("failed to send refresh request")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("token refresh failed ({}): {}", status, body);
    }

    let token_resp: TokenRefreshResponse = resp.json().await.context("invalid refresh response")?;

    creds.access_token = token_resp.access_token;
    if let Some(rt) = token_resp.refresh_token {
        creds.refresh_token = rt;
    }
    creds.expires_at = Utc::now() + chrono::Duration::seconds(token_resp.expires_in as i64);

    save(path, creds)?;
    debug!("token refreshed successfully");
    Ok(())
}

/// Subset of the OAuth2 token response we need for refresh.
#[derive(Deserialize)]
struct TokenRefreshResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample_creds() -> CliCredentials {
        CliCredentials {
            server_url: "http://localhost:8080".into(),
            client_id: "cid_test".into(),
            access_token: "jwt_test".into(),
            refresh_token: "rt_test".into(),
            expires_at: Utc::now() + Duration::hours(1),
        }
    }

    #[test]
    fn is_expired_false_when_fresh() {
        let creds = sample_creds();
        assert!(!creds.is_expired());
    }

    #[test]
    fn is_expired_true_when_past() {
        let mut creds = sample_creds();
        creds.expires_at = Utc::now() - Duration::minutes(1);
        assert!(creds.is_expired());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.json");

        let creds = sample_creds();
        save(&path, &creds).unwrap();

        let loaded = load(&path).unwrap().expect("should load");
        assert_eq!(loaded.server_url, creds.server_url);
        assert_eq!(loaded.client_id, creds.client_id);
        assert_eq!(loaded.access_token, creds.access_token);
        assert_eq!(loaded.refresh_token, creds.refresh_token);
    }

    #[test]
    fn load_nonexistent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does_not_exist.json");
        assert!(load(&path).unwrap().is_none());
    }

    #[test]
    fn remove_deletes_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.json");

        save(&path, &sample_creds()).unwrap();
        assert!(path.exists());

        remove(&path).unwrap();
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn save_sets_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.json");

        save(&path, &sample_creds()).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "credentials file should be 0600");
    }
}
