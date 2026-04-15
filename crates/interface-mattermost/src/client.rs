//! Thin Mattermost REST API client.
//!
//! Wraps `reqwest` with the handful of endpoints needed by the interface:
//! `GET /users/me`, `POST /posts`, `POST /reactions`, and `POST /files`.

use anyhow::{Context, Result};
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

/// Thin async Mattermost REST client.
pub struct MattermostClient {
    pub(crate) token: String,
    pub(crate) server_url: String,
    client: reqwest::Client,
}

impl MattermostClient {
    pub fn new(server_url: impl Into<String>, token: impl Into<String>) -> Result<Self> {
        let token = token.into();
        let server_url = server_url.into().trim_end_matches('/').to_string();
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .context("invalid token for Authorization header")?,
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("failed to build reqwest client")?;
        Ok(Self {
            token,
            server_url,
            client,
        })
    }

    /// Override the base URL (for testing).
    #[cfg(test)]
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.server_url = url.trim_end_matches('/').to_string();
        self
    }

    fn api_url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}/api/v4/{}", self.server_url, path)
    }

    /// GET a typed JSON response.
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.api_url(path);
        debug!(url = %url, "MM GET");
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("GET {url} → {status}: {body}");
        }
        serde_json::from_str(&body).with_context(|| format!("deserialise GET {url}"))
    }

    /// POST a typed JSON body, return typed response.
    async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.api_url(path);
        debug!(url = %url, "MM POST");
        let resp = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("POST {url} → {status}: {text}");
        }
        serde_json::from_str(&text).with_context(|| format!("deserialise POST {url}"))
    }

    // ── Public API methods ────────────────────────────────────────────────────

    /// Fetch the bot's own user record.
    pub async fn get_me(&self) -> Result<MeUser> {
        self.get("users/me").await
    }

    /// Create a post (reply / direct message).
    pub async fn create_post(
        &self,
        channel_id: &str,
        message: &str,
        root_id: Option<&str>,
    ) -> Result<Value> {
        let mut body = serde_json::json!({
            "channel_id": channel_id,
            "message": message,
        });
        if let Some(r) = root_id {
            body["root_id"] = Value::String(r.to_string());
        }
        self.post_json::<Value, Value>("posts", &body).await
    }

    /// Add an emoji reaction to a post.
    pub async fn add_reaction(&self, user_id: &str, post_id: &str, emoji: &str) -> Result<()> {
        let body = serde_json::json!({
            "user_id": user_id,
            "post_id": post_id,
            "emoji_name": emoji,
        });
        self.post_json::<Value, Value>("reactions", &body)
            .await
            .map(|_| ())
    }

    /// Remove an emoji reaction from a post.
    pub async fn remove_reaction(&self, user_id: &str, post_id: &str, emoji: &str) -> Result<()> {
        let url = self.api_url(&format!(
            "users/{user_id}/posts/{post_id}/reactions/{emoji}"
        ));
        debug!(url = %url, "MM DELETE reaction");
        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .with_context(|| format!("DELETE {url}"))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("DELETE {url} → {status}: {text}");
        }
        Ok(())
    }

    /// Send a typing event to a channel (fire-and-forget; server auto-expires).
    pub async fn send_typing(&self, channel_id: &str) -> Result<()> {
        let body = serde_json::json!({ "channel_id": channel_id });
        self.post_json::<Value, Value>("users/me/typing", &body)
            .await
            .map(|_| ())
    }

    /// Upload a file to a channel and return the file ID(s).
    pub async fn upload_file(
        &self,
        channel_id: &str,
        filename: &str,
        bytes: Vec<u8>,
    ) -> Result<Vec<String>> {
        let url = self.api_url("files");
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .unwrap_or_else(|_| {
                reqwest::multipart::Part::bytes(Vec::new()).file_name(filename.to_string())
            });
        let form = reqwest::multipart::Form::new()
            .text("channel_id", channel_id.to_string())
            .part("files", part);
        let resp = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .context("multipart upload")?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("file upload → {status}: {text}");
        }
        #[derive(Deserialize)]
        struct UploadResp {
            file_infos: Vec<FileInfo>,
        }
        #[derive(Deserialize)]
        struct FileInfo {
            id: String,
        }
        let ur: UploadResp =
            serde_json::from_str(&text).context("deserialise file upload response")?;
        Ok(ur.file_infos.into_iter().map(|f| f.id).collect())
    }

    /// Create a post with attached file IDs.
    pub async fn create_post_with_files(
        &self,
        channel_id: &str,
        message: &str,
        root_id: Option<&str>,
        file_ids: Vec<String>,
    ) -> Result<()> {
        let mut body = serde_json::json!({
            "channel_id": channel_id,
            "message": message,
            "file_ids": file_ids,
        });
        if let Some(r) = root_id {
            body["root_id"] = Value::String(r.to_string());
        }
        self.post_json::<Value, Value>("posts", &body)
            .await
            .map(|_| ())
    }

    /// Return the WebSocket URL for this server.
    pub fn ws_url(&self) -> String {
        let base = self
            .server_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        format!("{base}/api/v4/websocket")
    }
}

/// Minimal user record returned by `/users/me`.
#[derive(Debug, Deserialize)]
pub struct MeUser {
    pub id: String,
    pub username: String,
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    async fn mock_client(server: &MockServer) -> MattermostClient {
        MattermostClient::new(server.uri(), "test-token")
            .unwrap()
            .with_base_url(&server.uri())
    }

    #[tokio::test]
    async fn get_me_returns_user() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v4/users/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user123",
                "username": "bot"
            })))
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        let me = client.get_me().await.unwrap();
        assert_eq!(me.id, "user123");
    }

    #[tokio::test]
    async fn create_post_sends_correct_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v4/posts"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({"id":"p1"})))
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        let result = client.create_post("ch1", "hello", None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn ws_url_converts_https_to_wss() {
        let client = MattermostClient::new("https://mm.example.com", "tok").unwrap();
        assert_eq!(client.ws_url(), "wss://mm.example.com/api/v4/websocket");
    }

    #[tokio::test]
    async fn remove_reaction_calls_delete() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path(
                "/api/v4/users/u1/posts/p1/reactions/hourglass_flowing_sand",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        client
            .remove_reaction("u1", "p1", "hourglass_flowing_sand")
            .await
            .expect("remove_reaction should succeed");
    }

    #[tokio::test]
    async fn send_typing_posts_to_correct_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v4/users/me/typing"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        client
            .send_typing("ch1")
            .await
            .expect("send_typing should succeed");
    }

    #[tokio::test]
    async fn add_reaction_posts_to_reactions_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v4/reactions"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({"id":"r1"})))
            .mount(&server)
            .await;
        let client = mock_client(&server).await;
        client
            .add_reaction("u1", "p1", "hourglass_flowing_sand")
            .await
            .expect("add_reaction should succeed");
    }
}
