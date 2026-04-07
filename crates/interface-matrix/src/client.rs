//! Thin Matrix Client-Server API client.
//!
//! Implements just enough of the CS API to drive the interface:
//! - Password login / access token auth
//! - `/sync` long-poll for inbound events
//! - `PUT /rooms/<room>/send/m.room.message/<txn>` for outbound messages
//! - `POST /join/<room>` for auto-accepting invites

use anyhow::{Context, Result};
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::Deserialize;
use serde_json::Value;
use tracing::debug;
use uuid::Uuid;

/// Thin async Matrix client.
pub struct MatrixClient {
    #[allow(dead_code)]
    access_token: String,
    pub(crate) homeserver_url: String,
    /// The bot's own MXID, populated after login or set directly.
    pub(crate) user_id: String,
    client: reqwest::Client,
}

impl MatrixClient {
    /// Create from a pre-issued access token.
    pub fn new_with_token(
        homeserver_url: impl Into<String>,
        access_token: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Result<Self> {
        let access_token = access_token.into();
        let homeserver_url = homeserver_url.into().trim_end_matches('/').to_string();
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {access_token}"))
                .context("invalid access token for Authorization header")?,
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to build reqwest client")?;
        Ok(Self {
            access_token,
            homeserver_url,
            user_id: user_id.into(),
            client,
        })
    }

    /// Login with username + password, return a new client with the access token.
    pub async fn login(
        homeserver_url: impl Into<String>,
        username: &str,
        password: &str,
    ) -> Result<Self> {
        let homeserver_url = homeserver_url.into().trim_end_matches('/').to_string();
        let tmp_client = reqwest::Client::new();
        let url = format!("{homeserver_url}/_matrix/client/v3/login");
        let body = serde_json::json!({
            "type": "m.login.password",
            "identifier": { "type": "m.id.user", "user": username },
            "password": password,
            "initial_device_display_name": "assistant-matrix-bot"
        });
        let resp = tmp_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Matrix login request")?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("Matrix login failed ({status}): {text}");
        }
        #[derive(Deserialize)]
        struct LoginResp {
            access_token: String,
            user_id: String,
        }
        let lr: LoginResp =
            serde_json::from_str(&text).context("deserialise Matrix login response")?;
        Self::new_with_token(homeserver_url, lr.access_token, lr.user_id)
    }

    fn cs_url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}/_matrix/client/v3/{}", self.homeserver_url, path)
    }

    // ── Sync ──────────────────────────────────────────────────────────────────

    /// Long-poll sync. Blocks for up to `timeout_ms` (30 s typical).
    /// Pass `None` for `since` on the first call.
    pub async fn sync(&self, since: Option<&str>, timeout_ms: u64) -> Result<SyncResponse> {
        let mut url = format!(
            "{}?timeout={timeout_ms}&full_state=false",
            self.cs_url("sync")
        );
        if let Some(s) = since {
            url.push_str("&since=");
            url.push_str(s);
        }
        debug!(url = %url, "Matrix sync");
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Matrix sync request")?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("Matrix sync failed ({status}): {text}");
        }
        serde_json::from_str(&text).context("deserialise sync response")
    }

    // ── Send message ─────────────────────────────────────────────────────────

    /// Send a text message to a room. Generates a unique transaction ID.
    pub async fn send_message(&self, room_id: &str, text: &str) -> Result<()> {
        let txn_id = Uuid::new_v4().to_string();
        let path = format!(
            "rooms/{}/send/m.room.message/{}",
            urlencoding::encode(room_id),
            txn_id
        );
        let url = self.cs_url(&path);
        let body = serde_json::json!({ "msgtype": "m.text", "body": text });
        debug!(url = %url, room_id, "Matrix send_message");
        let resp = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await
            .context("Matrix send_message")?;
        let status = resp.status();
        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("Matrix send_message failed ({status}): {err}");
        }
        Ok(())
    }

    // ── Join room ────────────────────────────────────────────────────────────

    /// Accept a room invite.
    pub async fn join_room(&self, room_id: &str) -> Result<()> {
        let path = format!("join/{}", urlencoding::encode(room_id));
        let url = self.cs_url(&path);
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await
            .context("Matrix join_room")?;
        let status = resp.status();
        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("Matrix join_room failed ({status}): {err}");
        }
        Ok(())
    }
}

// ── Sync response types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SyncResponse {
    pub next_batch: String,
    pub rooms: Option<SyncRooms>,
}

#[derive(Debug, Deserialize)]
pub struct SyncRooms {
    pub join: Option<std::collections::HashMap<String, JoinedRoom>>,
    pub invite: Option<std::collections::HashMap<String, Value>>,
}

#[derive(Debug, Deserialize)]
pub struct JoinedRoom {
    pub timeline: Option<Timeline>,
}

#[derive(Debug, Deserialize)]
pub struct Timeline {
    pub events: Vec<TimelineEvent>,
}

#[derive(Debug, Deserialize)]
pub struct TimelineEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub sender: String,
    pub content: Value,
    pub event_id: Option<String>,
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    async fn mock_client(server: &MockServer) -> MatrixClient {
        MatrixClient::new_with_token(server.uri(), "test-token", "@bot:example.com").unwrap()
    }

    #[tokio::test]
    async fn send_message_calls_put() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"event_id":"$abc"})),
            )
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        let result = client.send_message("!room:example.com", "hello").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn sync_parses_next_batch() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "next_batch": "s123",
                "rooms": {}
            })))
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        let resp = client.sync(None, 0).await.unwrap();
        assert_eq!(resp.next_batch, "s123");
    }
}
