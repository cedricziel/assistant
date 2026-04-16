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

    // ── Media upload ──────────────────────────────────────────────────────────

    /// Upload a file to the Matrix Content Repository.
    ///
    /// Returns the `mxc://` URI of the uploaded media.
    pub async fn upload_media(
        &self,
        data: Vec<u8>,
        filename: &str,
        content_type: &str,
    ) -> Result<String> {
        let url = format!(
            "{}/_matrix/media/v3/upload?filename={}",
            self.homeserver_url,
            urlencoding::encode(filename)
        );
        debug!(url = %url, filename, content_type, "Matrix upload_media");
        let resp = self
            .client
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(data)
            .send()
            .await
            .context("Matrix upload_media request")?;
        let status = resp.status();
        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("Matrix upload_media failed ({status}): {err}");
        }
        let body: serde_json::Value = resp.json().await.context("Matrix upload_media body")?;
        let mxc_uri = body["content_uri"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Matrix upload_media: missing content_uri in response"))?
            .to_string();
        Ok(mxc_uri)
    }

    /// Send an `m.image` message to a room using an already-uploaded `mxc://` URI.
    pub async fn send_image(
        &self,
        room_id: &str,
        mxc_uri: &str,
        filename: &str,
        mime_type: &str,
        size_bytes: usize,
    ) -> Result<()> {
        let txn_id = Uuid::new_v4().to_string();
        let path = format!(
            "rooms/{}/send/m.room.message/{}",
            urlencoding::encode(room_id),
            txn_id
        );
        let url = self.cs_url(&path);
        let body = serde_json::json!({
            "msgtype": "m.image",
            "body": filename,
            "url": mxc_uri,
            "info": {
                "mimetype": mime_type,
                "size": size_bytes
            }
        });
        debug!(url = %url, room_id, filename, "Matrix send_image");
        let resp = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await
            .context("Matrix send_image")?;
        let status = resp.status();
        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("Matrix send_image failed ({status}): {err}");
        }
        Ok(())
    }

    // ── Media download ────────────────────────────────────────────────────────

    /// Download a Matrix Content Repository resource identified by an `mxc://` URI.
    ///
    /// Returns `(bytes, mime_type)`. Enforces `max_bytes` to prevent OOM.
    pub async fn download_media(
        &self,
        mxc_url: &str,
        max_bytes: usize,
    ) -> Result<(Vec<u8>, String)> {
        let rest = mxc_url
            .strip_prefix("mxc://")
            .ok_or_else(|| anyhow::anyhow!("not an mxc:// URI: {mxc_url}"))?;
        let sep = rest
            .find('/')
            .ok_or_else(|| anyhow::anyhow!("malformed mxc:// URI (no media_id): {mxc_url}"))?;
        let server = &rest[..sep];
        let media_id = &rest[sep + 1..];
        let url = format!(
            "{}/_matrix/media/v3/download/{}/{}",
            self.homeserver_url, server, media_id
        );
        debug!(url = %url, "Matrix download_media");
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Matrix download_media request")?;
        let status = resp.status();
        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("Matrix download_media failed ({status}): {err}");
        }
        let mime_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let bytes = resp.bytes().await.context("Matrix download_media body")?;
        if bytes.len() > max_bytes {
            anyhow::bail!(
                "Matrix download_media: response too large ({} > {} bytes)",
                bytes.len(),
                max_bytes
            );
        }
        Ok((bytes.to_vec(), mime_type))
    }

    // ── Typing indicator ──────────────────────────────────────────────────────

    /// Set or clear the typing indicator for the bot in a room.
    ///
    /// `typing: true` sets a 30-second indicator; `false` clears it immediately.
    pub async fn send_typing(&self, room_id: &str, typing: bool) -> Result<()> {
        let path = format!(
            "rooms/{}/typing/{}",
            urlencoding::encode(room_id),
            urlencoding::encode(&self.user_id),
        );
        let url = self.cs_url(&path);
        let body = if typing {
            serde_json::json!({ "typing": true, "timeout": 30_000 })
        } else {
            serde_json::json!({ "typing": false })
        };
        debug!(url = %url, room_id, typing, "Matrix send_typing");
        let resp = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await
            .context("Matrix send_typing")?;
        let status = resp.status();
        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("Matrix send_typing failed ({status}): {err}");
        }
        Ok(())
    }

    // ── Reactions ─────────────────────────────────────────────────────────────

    /// Add an `m.reaction` event to a room targeting `event_id`.
    /// Returns the event ID of the new reaction event (for later redaction).
    pub async fn add_reaction(&self, room_id: &str, event_id: &str, emoji: &str) -> Result<String> {
        let txn_id = Uuid::new_v4().to_string();
        let path = format!(
            "rooms/{}/send/m.reaction/{}",
            urlencoding::encode(room_id),
            txn_id,
        );
        let url = self.cs_url(&path);
        let body = serde_json::json!({
            "m.relates_to": {
                "rel_type": "m.annotation",
                "event_id": event_id,
                "key": emoji,
            }
        });
        debug!(url = %url, room_id, emoji, "Matrix add_reaction");
        let resp = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await
            .context("Matrix add_reaction")?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("Matrix add_reaction failed ({status}): {text}");
        }
        #[derive(serde::Deserialize)]
        struct Resp {
            event_id: String,
        }
        let r: Resp = serde_json::from_str(&text).context("deserialise add_reaction response")?;
        Ok(r.event_id)
    }

    /// Redact an event (e.g. remove a reaction) by sending a redaction event.
    pub async fn redact_event(&self, room_id: &str, event_id: &str) -> Result<()> {
        let txn_id = Uuid::new_v4().to_string();
        let path = format!(
            "rooms/{}/redact/{}/{}",
            urlencoding::encode(room_id),
            urlencoding::encode(event_id),
            txn_id,
        );
        let url = self.cs_url(&path);
        debug!(url = %url, room_id, event_id, "Matrix redact_event");
        let resp = self
            .client
            .put(&url)
            .json(&serde_json::json!({}))
            .send()
            .await
            .context("Matrix redact_event")?;
        let status = resp.status();
        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("Matrix redact_event failed ({status}): {err}");
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

    #[tokio::test]
    async fn download_media_success() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/_matrix/media/v3/download/example.com/abc123"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(b"audio-data".as_ref())
                    .append_header("Content-Type", "audio/ogg"),
            )
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        let (bytes, mime) = client
            .download_media("mxc://example.com/abc123", 1024 * 1024)
            .await
            .unwrap();
        assert_eq!(bytes, b"audio-data");
        assert_eq!(mime, "audio/ogg");
    }

    #[tokio::test]
    async fn download_media_malformed_uri() {
        let server = MockServer::start().await;
        let client = mock_client(&server).await;
        let result = client
            .download_media("https://example.com/not-mxc", 1024)
            .await;
        assert!(result.is_err(), "expected error for non-mxc URI");
    }

    #[tokio::test]
    async fn download_media_non_200() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        let result = client
            .download_media("mxc://example.com/missing", 1024 * 1024)
            .await;
        assert!(result.is_err(), "expected error for 404");
        assert!(
            result.unwrap_err().to_string().contains("404"),
            "error should mention status"
        );
    }

    #[tokio::test]
    async fn download_media_exceeds_size_limit() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(b"ABCDEFGHIJ".as_ref())
                    .append_header("Content-Type", "audio/ogg"),
            )
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        // limit to 5 bytes — body is 10
        let result = client.download_media("mxc://example.com/big", 5).await;
        assert!(
            result.is_err(),
            "expected error when body exceeds max_bytes"
        );
    }

    #[tokio::test]
    async fn upload_media_returns_mxc_uri() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(wiremock::matchers::path_regex(r"/_matrix/media/v3/upload"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({ "content_uri": "mxc://example.com/uploaded1" }),
                ),
            )
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        let mxc = client
            .upload_media(b"fake-png".to_vec(), "image.png", "image/png")
            .await
            .unwrap();
        assert_eq!(mxc, "mxc://example.com/uploaded1");
    }

    #[tokio::test]
    async fn upload_media_fails_on_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(413).set_body_string("too large"))
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        let result = client
            .upload_media(b"data".to_vec(), "big.png", "image/png")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn send_image_calls_put() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(wiremock::matchers::path_regex(
                r"/_matrix/client/v3/rooms/.+/send/m.room.message/.+",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "event_id": "$img1" })),
            )
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        client
            .send_image(
                "!room:example.com",
                "mxc://example.com/uploaded1",
                "photo.png",
                "image/png",
                1024,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn send_typing_true_sends_put_with_timeout() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(wiremock::matchers::path_regex(
                r"/_matrix/client/v3/rooms/.+/typing/.+",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        client.send_typing("!room:example.com", true).await.unwrap();
    }

    #[tokio::test]
    async fn send_typing_false_sends_put() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(wiremock::matchers::path_regex(
                r"/_matrix/client/v3/rooms/.+/typing/.+",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        client
            .send_typing("!room:example.com", false)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn add_reaction_returns_event_id() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(wiremock::matchers::path_regex(
                r"/_matrix/client/v3/rooms/.+/send/m.reaction/.+",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "event_id": "$reaction123" })),
            )
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        let event_id = client
            .add_reaction("!room:example.com", "$msg1", "⏳")
            .await
            .unwrap();
        assert_eq!(event_id, "$reaction123");
    }

    #[tokio::test]
    async fn redact_event_sends_put() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(wiremock::matchers::path_regex(
                r"/_matrix/client/v3/rooms/.+/redact/.+/.+",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "event_id": "$redact1" })),
            )
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        client
            .redact_event("!room:example.com", "$reaction123")
            .await
            .unwrap();
    }
}
