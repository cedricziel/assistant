//! OAuth 2.0 PKCE flow for OpenAI Codex subscription authentication.
//!
//! Authenticates via ChatGPT sign-in so usage is billed against the user's
//! Codex plan quota rather than API credits.
//!
//! ## Flow
//!
//! 1. Generate PKCE verifier + challenge.
//! 2. Open browser to `https://auth.openai.com/oauth/authorize`.
//! 3. Capture callback on `http://127.0.0.1:1455/auth/callback`.
//! 4. Exchange authorization code for tokens.
//! 5. Store `{ access_token, refresh_token, expires_at }` locally.
//! 6. Auto-refresh before expiry.

use std::path::PathBuf;

use anyhow::{bail, Context};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// ── Constants ─────────────────────────────────────────────────────────────────

const AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CALLBACK_ADDR: &str = "127.0.0.1:1455";
const REDIRECT_URI: &str = "http://127.0.0.1:1455/auth/callback";
const SCOPE: &str = "openid offline_access";

/// How many seconds before expiry we preemptively refresh.
const REFRESH_MARGIN_SECS: i64 = 120;

// ── Token types ───────────────────────────────────────────────────────────────

/// Persisted OAuth tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub account_id: Option<String>,
}

/// Raw token response from the OpenAI OAuth endpoint.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    /// Token lifetime in seconds.
    expires_in: Option<i64>,
    #[allow(dead_code)]
    token_type: Option<String>,
}

// ── OAuthManager ──────────────────────────────────────────────────────────────

/// Manages the OAuth PKCE flow and token lifecycle.
pub struct OAuthManager {
    /// OAuth client ID (public PKCE client).
    client_id: String,
    /// Path to the JSON file storing tokens.
    token_path: PathBuf,
    /// Cached tokens (loaded lazily).
    tokens: RwLock<Option<OAuthTokens>>,
    /// HTTP client for token exchange / refresh.
    http: reqwest::Client,
}

impl OAuthManager {
    /// Create a new manager with the given OAuth client ID.
    ///
    /// Tokens are stored in `~/.assistant/openai-oauth.json`.
    pub fn new(client_id: String) -> anyhow::Result<Self> {
        let assistant_dir = dirs::home_dir()
            .context("Cannot determine home directory")?
            .join(".assistant");
        std::fs::create_dir_all(&assistant_dir)
            .context("Failed to create ~/.assistant directory")?;
        let token_path = assistant_dir.join("openai-oauth.json");

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client for OAuth")?;

        // Try to load existing tokens.
        let cached = Self::load_tokens_from_disk(&token_path);

        Ok(Self {
            client_id,
            token_path,
            tokens: RwLock::new(cached),
            http,
        })
    }

    /// Return a valid access token, refreshing or performing a full login if needed.
    pub async fn ensure_valid_token(&self) -> anyhow::Result<String> {
        // Fast path: valid cached token.
        {
            let guard = self.tokens.read().await;
            if let Some(ref t) = *guard {
                if t.expires_at > Utc::now() + Duration::seconds(REFRESH_MARGIN_SECS) {
                    return Ok(t.access_token.clone());
                }
            }
        }

        // Slow path: need refresh or login.
        let mut guard = self.tokens.write().await;

        // Double-check under write lock.
        if let Some(ref t) = *guard {
            if t.expires_at > Utc::now() + Duration::seconds(REFRESH_MARGIN_SECS) {
                return Ok(t.access_token.clone());
            }
            // Try refresh.
            debug!("OAuth token expired, attempting refresh");
            match self.refresh(&t.refresh_token).await {
                Ok(new_tokens) => {
                    self.save_tokens_to_disk(&new_tokens)?;
                    let access = new_tokens.access_token.clone();
                    *guard = Some(new_tokens);
                    return Ok(access);
                }
                Err(e) => {
                    warn!("Token refresh failed: {e}; falling through to full login");
                }
            }
        }

        // Full PKCE login.
        info!("Starting OAuth PKCE login flow — a browser window will open");
        let new_tokens = self.perform_pkce_login().await?;
        self.save_tokens_to_disk(&new_tokens)?;
        let access = new_tokens.access_token.clone();
        *guard = Some(new_tokens);
        Ok(access)
    }

    // ── PKCE login flow ───────────────────────────────────────────────────

    async fn perform_pkce_login(&self) -> anyhow::Result<OAuthTokens> {
        // 1. Generate PKCE verifier + challenge.
        let verifier = generate_code_verifier();
        let challenge = compute_code_challenge(&verifier);
        let state = generate_random_string(32);

        // 2. Build authorization URL.
        let auth_url = format!(
            "{AUTH_URL}?\
             response_type=code\
             &client_id={client_id}\
             &redirect_uri={redirect}\
             &scope={scope}\
             &state={state}\
             &code_challenge={challenge}\
             &code_challenge_method=S256",
            client_id = urlencoded(&self.client_id),
            redirect = urlencoded(REDIRECT_URI),
            scope = urlencoded(SCOPE),
            state = urlencoded(&state),
            challenge = urlencoded(&challenge),
        );

        // 3. Start local server before opening browser.
        let listener = TcpListener::bind(CALLBACK_ADDR)
            .await
            .with_context(|| format!("Failed to bind callback server on {CALLBACK_ADDR}"))?;

        // 4. Open browser.
        info!("Opening browser for OpenAI login...");
        if let Err(e) = open_browser(&auth_url) {
            // If browser fails, print the URL for manual copy.
            warn!("Could not open browser: {e}");
            eprintln!("\nPlease open this URL in your browser:\n\n  {auth_url}\n");
        }

        // 5. Wait for the callback.
        let (code, received_state) = wait_for_callback(&listener).await?;

        // 6. Verify state.
        if received_state != state {
            bail!("OAuth state mismatch — possible CSRF attack");
        }

        // 7. Exchange code for tokens.
        self.exchange_code(&code, &verifier).await
    }

    async fn exchange_code(&self, code: &str, verifier: &str) -> anyhow::Result<OAuthTokens> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", &self.client_id),
            ("code_verifier", verifier),
        ];

        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .context("Token exchange request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Token exchange failed ({status}): {body}");
        }

        let token_resp: TokenResponse = resp
            .json()
            .await
            .context("Failed to parse token response")?;

        let expires_at = Utc::now() + Duration::seconds(token_resp.expires_in.unwrap_or(3600));

        Ok(OAuthTokens {
            access_token: token_resp.access_token,
            refresh_token: token_resp.refresh_token.unwrap_or_default(),
            expires_at,
            account_id: None,
        })
    }

    // ── Token refresh ─────────────────────────────────────────────────────

    async fn refresh(&self, refresh_token: &str) -> anyhow::Result<OAuthTokens> {
        debug!("Refreshing OAuth token");

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
        ];

        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .context("Token refresh request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("Token refresh failed ({status}): {body}");
        }

        let token_resp: TokenResponse = resp
            .json()
            .await
            .context("Failed to parse refresh response")?;

        let expires_at = Utc::now() + Duration::seconds(token_resp.expires_in.unwrap_or(3600));

        Ok(OAuthTokens {
            access_token: token_resp.access_token,
            refresh_token: token_resp
                .refresh_token
                .unwrap_or_else(|| refresh_token.to_string()),
            expires_at,
            account_id: None,
        })
    }

    // ── Token persistence ─────────────────────────────────────────────────

    fn load_tokens_from_disk(path: &PathBuf) -> Option<OAuthTokens> {
        let data = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn save_tokens_to_disk(&self, tokens: &OAuthTokens) -> anyhow::Result<()> {
        let json =
            serde_json::to_string_pretty(tokens).context("Failed to serialise OAuth tokens")?;
        std::fs::write(&self.token_path, json)
            .with_context(|| format!("Failed to write {}", self.token_path.display()))?;
        debug!(path = %self.token_path.display(), "OAuth tokens saved");
        Ok(())
    }
}

// ── PKCE helpers ──────────────────────────────────────────────────────────────

/// Generate a random code verifier (43–128 characters, base64url-safe).
fn generate_code_verifier() -> String {
    generate_random_string(64)
}

/// Compute `code_challenge = base64url(SHA-256(code_verifier))`.
fn compute_code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

/// Generate a random base64url-encoded string of the given byte length.
fn generate_random_string(byte_len: usize) -> String {
    let mut buf = vec![0u8; byte_len];
    getrandom::getrandom(&mut buf).expect("getrandom should not fail");
    URL_SAFE_NO_PAD.encode(&buf)
}

/// Percent-encode a value for use in a URL query string.
fn urlencoded(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

// ── Local callback server ─────────────────────────────────────────────────────

/// Wait for the OAuth callback on the given TCP listener.
///
/// Parses the `code` and `state` query parameters from the GET request and
/// sends back a minimal HTML response telling the user to close the tab.
async fn wait_for_callback(listener: &TcpListener) -> anyhow::Result<(String, String)> {
    let (mut stream, _addr) = listener
        .accept()
        .await
        .context("Failed to accept callback connection")?;

    let mut buf = vec![0u8; 8192];
    let n = stream
        .read(&mut buf)
        .await
        .context("Failed to read callback request")?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse the first line: GET /auth/callback?code=...&state=... HTTP/1.1
    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");

    let query = path.split('?').nth(1).unwrap_or("");
    let params: std::collections::HashMap<String, String> =
        url::form_urlencoded::parse(query.as_bytes())
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

    let code = params
        .get("code")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No 'code' parameter in callback URL"))?;
    let state = params.get("state").cloned().unwrap_or_default();

    // Send a minimal response.
    let html = "<html><body><h2>Authentication successful!</h2>\
                <p>You can close this tab and return to the terminal.</p>\
                </body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {html}",
        html.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;

    info!("OAuth callback received");
    Ok((code, state))
}

// ── Browser opener ────────────────────────────────────────────────────────────

/// Attempt to open a URL in the default browser.
fn open_browser(url: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .context("Failed to open browser with 'open'")?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context("Failed to open browser with 'xdg-open'")?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .context("Failed to open browser with 'start'")?;
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_challenge_is_deterministic() {
        let verifier = "test_verifier_12345";
        let c1 = compute_code_challenge(verifier);
        let c2 = compute_code_challenge(verifier);
        assert_eq!(c1, c2);
    }

    #[test]
    fn code_challenge_is_base64url() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = compute_code_challenge(verifier);
        // Must not contain +, /, or = (base64url without padding).
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
        assert!(!challenge.contains('='));
    }

    #[test]
    fn random_strings_are_unique() {
        let a = generate_random_string(32);
        let b = generate_random_string(32);
        assert_ne!(a, b);
    }

    #[test]
    fn urlencoded_encodes_special_chars() {
        assert_eq!(urlencoded("hello world"), "hello+world");
        assert_eq!(urlencoded("a=b&c=d"), "a%3Db%26c%3Dd");
    }
}
