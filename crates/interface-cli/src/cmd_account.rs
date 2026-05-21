//! CLI commands for managing the caller's own account.
//!
//! Wraps the `/api/users/me` endpoints exposed by `assistant-web-ui`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cmd_login::authenticated_client;

// -- API types (mirrors of server responses) --

#[derive(Debug, Deserialize, Serialize)]
struct UserDetail {
    id: String,
    org_id: String,
    email: String,
    name: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct OrgDetail {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    slug: String,
    auth_mode: String,
    #[allow(dead_code)]
    created_at: String,
    #[allow(dead_code)]
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct UpdateCurrentUserResponse {
    user: UserDetail,
    #[serde(default)]
    previous_email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    #[serde(default)]
    error: String,
}

// -- Helpers --

fn me_url(server_url: &str) -> String {
    format!("{}/api/users/me", server_url.trim_end_matches('/'))
}

fn me_password_url(server_url: &str) -> String {
    format!("{}/api/users/me/password", server_url.trim_end_matches('/'))
}

fn org_url(server_url: &str, org_id: &str) -> String {
    format!("{}/api/orgs/{}", server_url.trim_end_matches('/'), org_id)
}

async fn fetch_me(server_url: &str, http: &reqwest::Client, token: &str) -> Result<UserDetail> {
    let resp = http
        .get(me_url(server_url))
        .bearer_auth(token)
        .send()
        .await
        .context("failed to GET /api/users/me")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("failed to load account ({status}): {text}");
    }

    resp.json::<UserDetail>()
        .await
        .context("invalid /api/users/me response")
}

async fn fetch_org_auth_mode(
    server_url: &str,
    http: &reqwest::Client,
    token: &str,
    org_id: &str,
) -> Option<String> {
    let resp = http
        .get(org_url(server_url, org_id))
        .bearer_auth(token)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<OrgDetail>().await.ok().map(|o| o.auth_mode)
}

// -- Commands --

/// `assistant account show [--json]`
pub async fn cmd_account_show(
    json: bool,
    api_key: &Option<String>,
    server: &Option<String>,
) -> Result<()> {
    let (server_url, http, token) = authenticated_client(api_key, server).await?;
    let me = fetch_me(&server_url, &http, &token).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&me).context("failed to render JSON")?
        );
        return Ok(());
    }

    let auth_mode = fetch_org_auth_mode(&server_url, &http, &token, &me.org_id)
        .await
        .unwrap_or_else(|| "unknown".to_string());

    println!("Email:     {}", me.email);
    println!("Name:      {}", me.name);
    println!("User ID:   {}", me.id);
    println!("Org ID:    {}", me.org_id);
    println!("Auth mode: {auth_mode}");
    Ok(())
}

/// `assistant account set-email <email>`
pub async fn cmd_account_set_email(
    email: &str,
    api_key: &Option<String>,
    server: &Option<String>,
) -> Result<()> {
    let (server_url, http, token) = authenticated_client(api_key, server).await?;
    patch_me(&server_url, &http, &token, Some(email), None).await
}

/// `assistant account set-name <name>`
pub async fn cmd_account_set_name(
    name: &str,
    api_key: &Option<String>,
    server: &Option<String>,
) -> Result<()> {
    let (server_url, http, token) = authenticated_client(api_key, server).await?;
    patch_me(&server_url, &http, &token, None, Some(name)).await
}

async fn patch_me(
    server_url: &str,
    http: &reqwest::Client,
    token: &str,
    email: Option<&str>,
    name: Option<&str>,
) -> Result<()> {
    let mut body = serde_json::Map::new();
    if let Some(e) = email {
        body.insert("email".into(), serde_json::Value::String(e.to_string()));
    }
    if let Some(n) = name {
        body.insert("name".into(), serde_json::Value::String(n.to_string()));
    }

    let resp = http
        .patch(me_url(server_url))
        .bearer_auth(token)
        .json(&serde_json::Value::Object(body))
        .send()
        .await
        .context("failed to PATCH /api/users/me")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp
            .json::<ErrorBody>()
            .await
            .map(|e| e.error)
            .unwrap_or_default();
        if status == reqwest::StatusCode::CONFLICT && body.contains("identity provider") {
            anyhow::bail!(
                "this account is managed by an external identity provider — \
                 update your details there, not via assistant ({body})"
            );
        }
        anyhow::bail!("update failed ({status}): {body}");
    }

    let updated: UpdateCurrentUserResponse = resp.json().await.context("invalid response body")?;

    if let Some(prev) = updated.previous_email {
        println!("Email changed from {prev} to {}.", updated.user.email);
    } else if email.is_some() {
        println!("Email is already {}.", updated.user.email);
    }
    if name.is_some() {
        println!("Name set to {}.", updated.user.name);
    }
    Ok(())
}

/// `assistant account change-password`
pub async fn cmd_account_change_password(
    api_key: &Option<String>,
    server: &Option<String>,
) -> Result<()> {
    let (server_url, http, token) = authenticated_client(api_key, server).await?;

    let current = rpassword::prompt_password("Current password: ")
        .context("failed to read current password")?;
    let new =
        rpassword::prompt_password("New password:     ").context("failed to read new password")?;
    let confirm = rpassword::prompt_password("Confirm new:      ")
        .context("failed to read password confirmation")?;

    if new != confirm {
        anyhow::bail!("new password and confirmation do not match");
    }
    if new.is_empty() {
        anyhow::bail!("new password must not be empty");
    }

    let resp = http
        .post(me_password_url(&server_url))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "current_password": current,
            "new_password": new,
        }))
        .send()
        .await
        .context("failed to POST /api/users/me/password")?;

    let status = resp.status();
    if status == reqwest::StatusCode::NO_CONTENT {
        println!("Password changed.");
        return Ok(());
    }

    let body = resp
        .json::<ErrorBody>()
        .await
        .map(|e| e.error)
        .unwrap_or_default();

    if status == reqwest::StatusCode::CONFLICT && body.contains("identity provider") {
        anyhow::bail!(
            "this account is managed by an external identity provider — \
             change your password there, not via assistant ({body})"
        );
    }
    anyhow::bail!("password change failed ({status}): {body}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn user_body() -> serde_json::Value {
        serde_json::json!({
            "id": "u1",
            "org_id": "o1",
            "email": "alice@example.com",
            "name": "Alice",
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
        })
    }

    #[test]
    fn url_helpers_trim_trailing_slash() {
        assert_eq!(me_url("http://x/"), "http://x/api/users/me");
        assert_eq!(
            me_password_url("http://x/"),
            "http://x/api/users/me/password"
        );
        assert_eq!(org_url("http://x/", "o1"), "http://x/api/orgs/o1");
    }

    #[tokio::test]
    async fn fetch_me_parses_user_detail() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/users/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(user_body()))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let me = fetch_me(&server.uri(), &http, "tok").await.unwrap();
        assert_eq!(me.email, "alice@example.com");
        assert_eq!(me.id, "u1");
    }

    #[tokio::test]
    async fn fetch_me_errors_on_non_success_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/users/me"))
            .respond_with(ResponseTemplate::new(401).set_body_string("nope"))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        assert!(fetch_me(&server.uri(), &http, "tok").await.is_err());
    }

    #[tokio::test]
    async fn fetch_org_auth_mode_returns_value_on_success() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/orgs/o1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "o1",
                "name": "Org",
                "slug": "org",
                "auth_mode": "password",
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z",
            })))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let mode = fetch_org_auth_mode(&server.uri(), &http, "tok", "o1").await;
        assert_eq!(mode.as_deref(), Some("password"));
    }

    #[tokio::test]
    async fn fetch_org_auth_mode_returns_none_on_4xx() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/orgs/o1"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let mode = fetch_org_auth_mode(&server.uri(), &http, "tok", "o1").await;
        assert!(mode.is_none());
    }

    #[tokio::test]
    async fn patch_me_succeeds_with_previous_email_in_response() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/users/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user": user_body(),
                "previous_email": "old@example.com",
            })))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        patch_me(&server.uri(), &http, "tok", Some("alice@example.com"), None)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn patch_me_succeeds_when_name_set() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/users/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user": user_body(),
            })))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        patch_me(&server.uri(), &http, "tok", None, Some("Alice"))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn patch_me_bails_with_idp_hint_on_409() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/users/me"))
            .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
                "error": "this user is managed by an external identity provider",
            })))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let err = patch_me(&server.uri(), &http, "tok", Some("x@y.com"), None)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("identity provider"));
    }

    #[tokio::test]
    async fn patch_me_bails_with_generic_error_on_other_status() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/users/me"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "bad request",
            })))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let err = patch_me(&server.uri(), &http, "tok", Some("x"), None)
            .await
            .unwrap_err();
        let s = err.to_string();
        assert!(s.contains("update failed"), "got: {s}");
    }
}
