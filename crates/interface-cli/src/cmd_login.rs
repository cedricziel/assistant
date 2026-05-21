//! CLI login / logout / status commands using OAuth2 device code flow.

use std::io::{self, Write as IoWrite};
use std::time::Duration;

use anyhow::{Context, Result};
use assistant_core::clock::{Clock, SystemClock};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::credentials::{self, CliCredentials};

// -- API types (mirrors of server responses) --

/// Dynamic client registration request (RFC 7591).
#[derive(Serialize)]
struct RegisterRequest {
    client_name: String,
    redirect_uris: Vec<String>,
    grant_types: Vec<String>,
    token_endpoint_auth_method: String,
}

/// Dynamic client registration response.
#[derive(Deserialize)]
struct RegisterResponse {
    client_id: String,
}

/// Device code initiation response (RFC 8628).
#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    interval: u64,
    expires_in: u64,
}

/// Token endpoint response.
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

/// Token endpoint error response.
#[derive(Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[serde(default)]
    error_description: String,
}

// -- Commands --

/// Run the `assistant login <server>` flow.
pub async fn cmd_login(server_url: &str) -> Result<()> {
    let server_url = server_url.trim_end_matches('/');
    let http = reqwest::Client::new();
    let creds_path = credentials::credentials_path()?;

    // Check for existing credentials.
    if let Some(existing) = credentials::load(&creds_path)?
        && existing.server_url.trim_end_matches('/') == server_url
        && !existing.is_expired()
    {
        println!("Already logged in to {server_url}.");
        println!("Run `assistant logout` first to re-authenticate.");
        return Ok(());
    }

    // 1. Dynamic client registration.
    println!("Registering CLI client with {server_url}...");
    let reg_url = format!("{server_url}/oauth/register");
    let reg_resp = http
        .post(&reg_url)
        .json(&RegisterRequest {
            client_name: "assistant-cli".into(),
            redirect_uris: vec![],
            grant_types: vec![
                "urn:ietf:params:oauth:grant-type:device_code".into(),
                "refresh_token".into(),
            ],
            token_endpoint_auth_method: "none".into(),
        })
        .send()
        .await
        .context("failed to reach server for client registration")?;

    if !reg_resp.status().is_success() {
        let status = reg_resp.status();
        let body = reg_resp.text().await.unwrap_or_default();
        anyhow::bail!("client registration failed ({status}): {body}");
    }
    let reg: RegisterResponse = reg_resp
        .json()
        .await
        .context("invalid registration response")?;
    let client_id = reg.client_id;
    debug!("registered client_id={client_id}");

    // 2. Initiate device flow.
    let device_url = format!("{server_url}/oauth/device");
    let device_resp = http
        .post(&device_url)
        .form(&[("client_id", client_id.as_str()), ("scope", "")])
        .send()
        .await
        .context("failed to initiate device flow")?;

    if !device_resp.status().is_success() {
        let status = device_resp.status();
        let body = device_resp.text().await.unwrap_or_default();
        anyhow::bail!("device flow initiation failed ({status}): {body}");
    }
    let device: DeviceCodeResponse = device_resp
        .json()
        .await
        .context("invalid device code response")?;

    // 3. Display code and verification URI.
    println!();
    println!("  Open this URL in your browser:");
    println!("    {}", device.verification_uri);
    println!();
    println!("  Enter code: {}", device.user_code);
    println!();
    print!("Waiting for authorization...");
    io::stdout().flush().ok();

    // 4. Poll token endpoint.
    let interval = Duration::from_secs(device.interval.max(5));
    let deadline = SystemClock.now() + chrono::Duration::seconds(device.expires_in as i64);
    let token_url = format!("{server_url}/oauth/token");

    let token_resp = loop {
        tokio::time::sleep(interval).await;

        if SystemClock.now() >= deadline {
            println!();
            anyhow::bail!("device code expired — please run `assistant login` again");
        }

        let resp = http
            .post(&token_url)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &device.device_code),
                ("client_id", &client_id),
            ])
            .send()
            .await
            .context("failed to poll token endpoint")?;

        if resp.status().is_success() {
            let t: TokenResponse = resp.json().await.context("invalid token response")?;
            break t;
        }

        let body = resp.text().await.unwrap_or_default();
        if let Ok(err) = serde_json::from_str::<TokenErrorResponse>(&body) {
            match err.error.as_str() {
                "authorization_pending" => {
                    print!(".");
                    io::stdout().flush().ok();
                    continue;
                }
                "slow_down" => {
                    print!(".");
                    io::stdout().flush().ok();
                    // Back off an extra interval.
                    tokio::time::sleep(interval).await;
                    continue;
                }
                "expired_token" => {
                    println!();
                    anyhow::bail!("device code expired — please run `assistant login` again");
                }
                _ => {
                    println!();
                    anyhow::bail!(
                        "token exchange failed: {} — {}",
                        err.error,
                        err.error_description
                    );
                }
            }
        }

        // Unexpected non-JSON error.
        println!();
        anyhow::bail!("unexpected token response: {body}");
    };

    println!(" authorized!");
    println!();

    // 5. Save credentials.
    let refresh_token = token_resp
        .refresh_token
        .context("server did not return a refresh token")?;

    let creds = CliCredentials {
        server_url: server_url.to_string(),
        client_id,
        access_token: token_resp.access_token,
        refresh_token,
        expires_at: SystemClock.now() + chrono::Duration::seconds(token_resp.expires_in as i64),
    };
    credentials::save(&creds_path, &creds)?;

    println!("Logged in to {server_url}.");
    println!("Credentials saved to {}", creds_path.display());
    Ok(())
}

/// Run the `assistant logout` flow.
pub async fn cmd_logout() -> Result<()> {
    let creds_path = credentials::credentials_path()?;

    match credentials::load(&creds_path)? {
        Some(creds) => {
            // Best-effort server-side revocation.
            let http = reqwest::Client::new();
            let revoke_url = format!("{}/oauth/revoke", creds.server_url.trim_end_matches('/'));
            let _ = http
                .post(&revoke_url)
                .form(&[("token", &creds.refresh_token)])
                .send()
                .await;

            credentials::remove(&creds_path)?;
            println!("Logged out (credentials removed).");
        }
        None => {
            println!("Not currently logged in.");
        }
    }
    Ok(())
}

/// Run the `assistant status` (login status) flow.
pub fn cmd_status() -> Result<()> {
    let creds_path = credentials::credentials_path()?;

    match credentials::load(&creds_path)? {
        Some(creds) => {
            println!("Logged in to {}", creds.server_url);
            if creds.is_expired() {
                println!("  Access token: expired (will refresh on next request)");
            } else {
                let remaining = creds.expires_at - SystemClock.now();
                let mins = remaining.num_minutes();
                println!("  Access token: valid ({mins} min remaining)");
            }
            println!("  Client ID: {}", creds.client_id);
            println!("  Credentials: {}", creds_path.display());
        }
        None => {
            println!("Not logged in.");
            println!("Run `assistant login <server-url>` to authenticate.");
        }
    }
    Ok(())
}

// -- Authenticated helper --

/// Load credentials, refresh if needed, return (server_url, bearer token).
///
/// If `api_key` and `server_url` are provided, uses them directly (no
/// stored credentials needed). Otherwise falls back to `credentials.json`.
pub(crate) async fn authenticated_client(
    api_key: &Option<String>,
    server_override: &Option<String>,
) -> Result<(String, reqwest::Client, String)> {
    let http = reqwest::Client::new();

    if let Some(key) = api_key {
        let server = server_override
            .as_deref()
            .context("--server is required when using --api-key")?;
        return Ok((server.trim_end_matches('/').to_string(), http, key.clone()));
    }

    let creds_path = credentials::credentials_path()?;
    let mut creds = credentials::load(&creds_path)?
        .context("not logged in — run `assistant login <server-url>` first")?;

    credentials::refresh_if_needed(&creds_path, &mut creds, &http).await?;

    let server_url = creds.server_url.clone();
    let token = creds.access_token.clone();
    Ok((server_url, http, token))
}

// -- API key management --

/// API key summary from the server.
#[derive(Deserialize)]
struct ApiKeySummary {
    id: String,
    name: String,
    key_prefix: String,
    scopes: Vec<String>,
    #[allow(dead_code)]
    created_at: String,
    expires_at: Option<String>,
}

/// API key creation response (includes plaintext, shown once).
#[derive(Deserialize)]
struct CreateApiKeyResponse {
    #[allow(dead_code)]
    id: String,
    name: String,
    #[allow(dead_code)]
    key_prefix: String,
    plaintext: String,
    scopes: Vec<String>,
}

/// `assistant api-keys create --name <n> --scopes <s>`
pub async fn cmd_api_keys_create(
    name: &str,
    scopes: &Option<String>,
    api_key: &Option<String>,
    server: &Option<String>,
) -> Result<()> {
    let (server_url, http, token) = authenticated_client(api_key, server).await?;
    let url = format!("{}/api/users/me/api-keys", server_url.trim_end_matches('/'));

    let scopes_vec: Option<Vec<String>> = scopes
        .as_ref()
        .map(|s| s.split(',').map(|v| v.trim().to_string()).collect());

    let body = serde_json::json!({
        "name": name,
        "scopes": scopes_vec,
    });

    let resp = http
        .post(&url)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .context("failed to create API key")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("failed to create API key ({status}): {text}");
    }

    let created: CreateApiKeyResponse = resp.json().await.context("invalid create response")?;

    println!("API key created:");
    println!("  Name:   {}", created.name);
    println!("  Key:    {}", created.plaintext);
    if !created.scopes.is_empty() {
        println!("  Scopes: {}", created.scopes.join(", "));
    }
    println!();
    println!("Save this key — it will not be shown again.");
    Ok(())
}

/// `assistant api-keys list`
pub async fn cmd_api_keys_list(api_key: &Option<String>, server: &Option<String>) -> Result<()> {
    let (server_url, http, token) = authenticated_client(api_key, server).await?;
    let url = format!("{}/api/users/me/api-keys", server_url.trim_end_matches('/'));

    let resp = http
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await
        .context("failed to list API keys")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("failed to list API keys ({status}): {text}");
    }

    let keys: Vec<ApiKeySummary> = resp.json().await.context("invalid list response")?;

    if keys.is_empty() {
        println!("No API keys.");
        return Ok(());
    }

    println!(
        "{:<14} {:<16} {:<12} {:<24} EXPIRES",
        "ID", "NAME", "PREFIX", "SCOPES"
    );
    for key in &keys {
        let scopes = if key.scopes.is_empty() {
            "(none)".to_string()
        } else {
            key.scopes.join(", ")
        };
        let expires = key.expires_at.as_deref().unwrap_or("never");
        println!(
            "{:<14} {:<16} {:<12} {:<24} {}",
            key.id, key.name, key.key_prefix, scopes, expires
        );
    }
    Ok(())
}

/// `assistant api-keys revoke <id>`
pub async fn cmd_api_keys_revoke(
    id: &str,
    api_key: &Option<String>,
    server: &Option<String>,
) -> Result<()> {
    let (server_url, http, token) = authenticated_client(api_key, server).await?;
    let url = format!(
        "{}/api/users/me/api-keys/{}",
        server_url.trim_end_matches('/'),
        id
    );

    let resp = http
        .delete(&url)
        .bearer_auth(&token)
        .send()
        .await
        .context("failed to revoke API key")?;

    match resp.status().as_u16() {
        204 => println!("API key {id} revoked."),
        404 => println!("API key {id} not found."),
        status => {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("failed to revoke API key ({status}): {text}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn authenticated_client_uses_api_key_when_provided() {
        let (server, http, token) = authenticated_client(
            &Some("ask_live_abc".to_string()),
            &Some("http://x/".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(server, "http://x");
        assert_eq!(token, "ask_live_abc");
        let _ = http; // dropped — just constructed.
    }

    #[tokio::test]
    async fn authenticated_client_errors_when_api_key_without_server() {
        let res = authenticated_client(&Some("ask_live_abc".to_string()), &None).await;
        let err = res.err().expect("should error");
        assert!(err.to_string().contains("--server"));
    }

    #[tokio::test]
    async fn api_keys_create_emits_plaintext_on_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/users/me/api-keys"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "k1",
                "name": "ci",
                "key_prefix": "ask_live_",
                "plaintext": "ask_live_full_secret",
                "scopes": ["conversations:read"],
            })))
            .mount(&server)
            .await;

        cmd_api_keys_create(
            "ci",
            &Some("conversations:read".to_string()),
            &Some("ask_live_caller".to_string()),
            &Some(server.uri()),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn api_keys_create_bails_on_non_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/users/me/api-keys"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad request"))
            .mount(&server)
            .await;
        let res = cmd_api_keys_create(
            "ci",
            &None,
            &Some("ask_live_caller".to_string()),
            &Some(server.uri()),
        )
        .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn api_keys_list_emits_table_for_non_empty() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/users/me/api-keys"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": "k1",
                    "name": "ci",
                    "key_prefix": "ask_live_",
                    "scopes": ["a:b"],
                    "created_at": "2026-01-01",
                    "expires_at": null,
                },
                {
                    "id": "k2",
                    "name": "bot",
                    "key_prefix": "ask_live_",
                    "scopes": [],
                    "created_at": "2026-01-02",
                    "expires_at": "2099-01-01",
                }
            ])))
            .mount(&server)
            .await;
        cmd_api_keys_list(&Some("ask_live_caller".to_string()), &Some(server.uri()))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn api_keys_list_handles_empty_response() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/users/me/api-keys"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;
        cmd_api_keys_list(&Some("ask_live_caller".to_string()), &Some(server.uri()))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn api_keys_list_bails_on_error_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/users/me/api-keys"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let res =
            cmd_api_keys_list(&Some("ask_live_caller".to_string()), &Some(server.uri())).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn api_keys_revoke_204_prints_success() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/users/me/api-keys/k1"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        cmd_api_keys_revoke(
            "k1",
            &Some("ask_live_caller".to_string()),
            &Some(server.uri()),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn api_keys_revoke_404_prints_not_found_and_returns_ok() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/users/me/api-keys/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        cmd_api_keys_revoke(
            "missing",
            &Some("ask_live_caller".to_string()),
            &Some(server.uri()),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn api_keys_revoke_other_status_bails() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/users/me/api-keys/k1"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let res = cmd_api_keys_revoke(
            "k1",
            &Some("ask_live_caller".to_string()),
            &Some(server.uri()),
        )
        .await;
        assert!(res.is_err());
    }
}
