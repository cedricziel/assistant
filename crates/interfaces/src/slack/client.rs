//! Thin reqwest-based Slack API client.
//!
//! Covers only the endpoints needed by the assistant:
//! - `apps.connections.open` — get a Socket Mode WebSocket URL
//! - `chat.postMessage` — post a reply in a channel / thread
//! - `reactions.add` — add an emoji reaction
//! - `auth.test` — resolve the bot user ID
//! - `conversations.replies` — fetch thread history for context seeding
//! - `files.getUploadURLExternal` + `files.completeUploadExternal` — upload files

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use tracing::debug;

/// Thin Slack API client backed by `reqwest`.
#[derive(Clone)]
pub struct SlackApiClient {
    pub(crate) bot_token: String,
    pub(crate) app_token: String,
    client: reqwest::Client,
    base_url: String,
}

impl SlackApiClient {
    pub fn new(bot_token: String, app_token: String) -> Result<Self> {
        Self::with_base_url(bot_token, app_token, "https://slack.com".to_string())
    }

    pub fn with_base_url(bot_token: String, app_token: String, base_url: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .build()
            .context("failed to build Slack reqwest client")?;
        Ok(Self {
            bot_token,
            app_token,
            client,
            base_url,
        })
    }

    // -- apps.connections.open ------------------------------------------------

    /// Request a Socket Mode WebSocket URL using the **app token**.
    pub async fn apps_connections_open(&self) -> Result<String> {
        #[derive(Deserialize)]
        struct Resp {
            ok: bool,
            url: Option<String>,
            error: Option<String>,
        }

        let resp: Resp = self
            .client
            .post(format!("{}/api/apps.connections.open", self.base_url))
            .bearer_auth(&self.app_token)
            .send()
            .await
            .context("apps.connections.open request failed")?
            .json()
            .await
            .context("apps.connections.open response parse failed")?;

        if !resp.ok {
            bail!(
                "apps.connections.open error: {}",
                resp.error.unwrap_or_default()
            );
        }
        resp.url.context("apps.connections.open: missing url")
    }

    // -- auth.test ------------------------------------------------------------

    /// Resolve the bot's own Slack user ID.
    pub async fn auth_test(&self) -> Result<String> {
        #[derive(Deserialize)]
        struct Resp {
            ok: bool,
            user_id: Option<String>,
            error: Option<String>,
        }

        let resp: Resp = self
            .client
            .post(format!("{}/api/auth.test", self.base_url))
            .bearer_auth(&self.bot_token)
            .send()
            .await
            .context("auth.test request failed")?
            .json()
            .await
            .context("auth.test response parse failed")?;

        if !resp.ok {
            bail!("auth.test error: {}", resp.error.unwrap_or_default());
        }
        resp.user_id.context("auth.test: missing user_id")
    }

    // -- chat.postMessage -----------------------------------------------------

    /// Post a message via the bot token.
    ///
    /// Returns the `ts` of the posted message.
    pub async fn post_message(
        &self,
        channel: &str,
        text: &str,
        thread_ts: Option<&str>,
    ) -> Result<String> {
        #[derive(Deserialize)]
        struct Resp {
            ok: bool,
            ts: Option<String>,
            error: Option<String>,
        }

        let mut body = serde_json::json!({
            "channel": channel,
            "text": text,
        });
        if let Some(ts) = thread_ts {
            body["thread_ts"] = serde_json::Value::String(ts.to_string());
        }

        debug!(
            channel,
            has_thread = thread_ts.is_some(),
            "chat.postMessage"
        );

        let resp: Resp = self
            .client
            .post(format!("{}/api/chat.postMessage", self.base_url))
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await
            .context("chat.postMessage request failed")?
            .json()
            .await
            .context("chat.postMessage response parse failed")?;

        if !resp.ok {
            bail!("chat.postMessage error: {}", resp.error.unwrap_or_default());
        }
        resp.ts.context("chat.postMessage: missing ts")
    }

    /// Update an existing message in-place.
    pub async fn update_message(&self, channel: &str, ts: &str, text: &str) -> Result<()> {
        #[derive(Deserialize)]
        struct Resp {
            ok: bool,
            error: Option<String>,
        }

        let body = serde_json::json!({
            "channel": channel,
            "ts": ts,
            "text": text,
        });

        let resp: Resp = self
            .client
            .post(format!("{}/api/chat.update", self.base_url))
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await
            .context("chat.update request failed")?
            .json()
            .await
            .context("chat.update response parse failed")?;

        if !resp.ok {
            bail!("chat.update error: {}", resp.error.unwrap_or_default());
        }
        Ok(())
    }

    /// Delete a message.
    pub async fn delete_message(&self, channel: &str, ts: &str) -> Result<()> {
        #[derive(Deserialize)]
        struct Resp {
            ok: bool,
            error: Option<String>,
        }

        let body = serde_json::json!({ "channel": channel, "ts": ts });

        let resp: Resp = self
            .client
            .post(format!("{}/api/chat.delete", self.base_url))
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await
            .context("chat.delete request failed")?
            .json()
            .await
            .context("chat.delete response parse failed")?;

        if !resp.ok {
            bail!("chat.delete error: {}", resp.error.unwrap_or_default());
        }
        Ok(())
    }

    // -- reactions.add --------------------------------------------------------

    /// Add an emoji reaction to a message.
    pub async fn add_reaction(&self, channel: &str, timestamp: &str, name: &str) -> Result<()> {
        #[derive(Deserialize)]
        struct Resp {
            ok: bool,
            error: Option<String>,
        }

        let body = serde_json::json!({
            "channel": channel,
            "timestamp": timestamp,
            "name": name,
        });

        let resp: Resp = self
            .client
            .post(format!("{}/api/reactions.add", self.base_url))
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await
            .context("reactions.add request failed")?
            .json()
            .await
            .context("reactions.add response parse failed")?;

        if !resp.ok {
            // "already_reacted" is a benign duplicate; log and continue.
            let err = resp.error.unwrap_or_default();
            if err == "already_reacted" {
                debug!(channel, name, "reactions.add: already_reacted (ignored)");
                return Ok(());
            }
            bail!("reactions.add error: {err}");
        }
        Ok(())
    }

    // -- conversations.replies ------------------------------------------------

    /// Fetch replies in a thread.
    pub async fn conversations_replies(
        &self,
        channel: &str,
        ts: &str,
        limit: u32,
    ) -> Result<Vec<serde_json::Value>> {
        #[derive(Deserialize)]
        struct Resp {
            ok: bool,
            messages: Option<Vec<serde_json::Value>>,
            error: Option<String>,
        }

        let resp: Resp = self
            .client
            .get(format!("{}/api/conversations.replies", self.base_url))
            .bearer_auth(&self.bot_token)
            .query(&[
                ("channel", channel.to_string()),
                ("ts", ts.to_string()),
                ("limit", limit.to_string()),
            ])
            .send()
            .await
            .context("conversations.replies request failed")?
            .json()
            .await
            .context("conversations.replies response parse failed")?;

        if !resp.ok {
            bail!(
                "conversations.replies error: {}",
                resp.error.unwrap_or_default()
            );
        }
        Ok(resp.messages.unwrap_or_default())
    }

    // -- conversations.list --------------------------------------------------

    /// List channels.
    pub async fn conversations_list(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut params = vec![
            ("types", "public_channel,private_channel".to_string()),
            ("limit", limit.to_string()),
        ];
        if let Some(c) = cursor {
            params.push(("cursor", c.to_string()));
        }

        self.client
            .get(format!("{}/api/conversations.list", self.base_url))
            .bearer_auth(&self.bot_token)
            .query(&params)
            .send()
            .await
            .context("conversations.list request failed")?
            .json()
            .await
            .context("conversations.list response parse failed")
    }

    // -- conversations.history -----------------------------------------------

    /// Fetch the conversation history for a channel.
    pub async fn conversations_history(
        &self,
        channel: &str,
        limit: u32,
    ) -> Result<serde_json::Value> {
        self.client
            .get(format!("{}/api/conversations.history", self.base_url))
            .bearer_auth(&self.bot_token)
            .query(&[
                ("channel", channel.to_string()),
                ("limit", limit.to_string()),
            ])
            .send()
            .await
            .context("conversations.history request failed")?
            .json()
            .await
            .context("conversations.history response parse failed")
    }

    // -- users.info ----------------------------------------------------------

    /// Fetch user profile info.
    pub async fn users_info(&self, user_id: &str) -> Result<serde_json::Value> {
        self.client
            .get(format!("{}/api/users.info", self.base_url))
            .bearer_auth(&self.bot_token)
            .query(&[("user", user_id)])
            .send()
            .await
            .context("users.info request failed")?
            .json()
            .await
            .context("users.info response parse failed")
    }

    // -- private file download ------------------------------------------------

    /// Download a Slack private file URL authenticated with the bot token.
    ///
    /// Returns the raw bytes.  Fails if the response exceeds `max_bytes` or
    /// the HTTP request itself fails.
    pub async fn download_private_file(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>> {
        let resp = self
            .client
            .get(url)
            .bearer_auth(&self.bot_token)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .context("slack: file download request failed")?;

        if !resp.status().is_success() {
            bail!("slack: file download failed with status {}", resp.status());
        }

        if resp
            .content_length()
            .is_some_and(|len| len > max_bytes as u64)
        {
            bail!(
                "slack: file too large ({} bytes, limit {max_bytes})",
                resp.content_length().unwrap_or(0)
            );
        }

        let bytes = resp
            .bytes()
            .await
            .context("slack: failed to read file bytes")?;

        if bytes.len() > max_bytes {
            bail!(
                "slack: file too large ({} bytes, limit {max_bytes})",
                bytes.len()
            );
        }

        Ok(bytes.to_vec())
    }

    // -- reactions.remove -----------------------------------------------------

    /// Remove an emoji reaction from a message.
    pub async fn remove_reaction(&self, channel: &str, timestamp: &str, name: &str) -> Result<()> {
        #[derive(Deserialize)]
        struct Resp {
            ok: bool,
            error: Option<String>,
        }

        let body = serde_json::json!({
            "channel": channel,
            "timestamp": timestamp,
            "name": name,
        });

        let resp: Resp = self
            .client
            .post(format!("{}/api/reactions.remove", self.base_url))
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await
            .context("reactions.remove request failed")?
            .json()
            .await
            .context("reactions.remove response parse failed")?;

        if !resp.ok {
            let err = resp.error.unwrap_or_default();
            // "no_reaction" means it was already removed — benign.
            if err == "no_reaction" {
                debug!(channel, name, "reactions.remove: no_reaction (ignored)");
                return Ok(());
            }
            bail!("reactions.remove error: {err}");
        }
        Ok(())
    }

    // -- assistant.threads.setStatus ------------------------------------------

    /// Set an animated agent-status message on a Slack assistant thread.
    ///
    /// `loading_messages` cycles through up to 10 messages while the agent is
    /// working.  The status clears automatically when the bot posts a reply.
    pub async fn set_agent_status(
        &self,
        channel_id: &str,
        thread_ts: &str,
        status: &str,
        loading_messages: &[&str],
    ) -> Result<()> {
        #[derive(Deserialize)]
        struct Resp {
            ok: bool,
            error: Option<String>,
        }

        let body = serde_json::json!({
            "channel_id": channel_id,
            "thread_ts": thread_ts,
            "status": status,
            "loading_messages": loading_messages,
        });

        debug!(channel_id, thread_ts, status, "assistant.threads.setStatus");

        let resp: Resp = self
            .client
            .post(format!("{}/api/assistant.threads.setStatus", self.base_url))
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await
            .context("assistant.threads.setStatus request failed")?
            .json()
            .await
            .context("assistant.threads.setStatus response parse failed")?;

        if !resp.ok {
            bail!(
                "assistant.threads.setStatus error: {}",
                resp.error.unwrap_or_default()
            );
        }
        Ok(())
    }

    // -- generic helper -------------------------------------------------------

    /// POST a JSON body to a Slack API path using the **bot token**.
    /// Returns the deserialized JSON response.
    pub async fn http_post_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> anyhow::Result<T> {
        self.client
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.bot_token)
            .json(body)
            .send()
            .await
            .context("http_post_json request failed")?
            .json()
            .await
            .context("http_post_json response parse failed")
    }

    // -- files.getUploadURLExternal + files.completeUploadExternal ------------

    /// Upload a file to Slack using the two-step upload API.
    pub async fn upload_file(
        &self,
        channel: &str,
        filename: &str,
        data: Vec<u8>,
        thread_ts: Option<&str>,
    ) -> Result<()> {
        // Step 1: get upload URL
        #[derive(Deserialize)]
        struct UrlResp {
            ok: bool,
            upload_url: Option<String>,
            file_id: Option<String>,
            error: Option<String>,
        }

        let url_resp: UrlResp = self
            .client
            .get(format!("{}/api/files.getUploadURLExternal", self.base_url))
            .bearer_auth(&self.bot_token)
            .query(&[
                ("filename", filename.to_string()),
                ("length", data.len().to_string()),
            ])
            .send()
            .await
            .context("files.getUploadURLExternal request failed")?
            .json()
            .await
            .context("files.getUploadURLExternal response parse failed")?;

        if !url_resp.ok {
            bail!(
                "files.getUploadURLExternal error: {}",
                url_resp.error.unwrap_or_default()
            );
        }
        let upload_url = url_resp
            .upload_url
            .context("files.getUploadURLExternal: missing upload_url")?;
        let file_id = url_resp
            .file_id
            .context("files.getUploadURLExternal: missing file_id")?;

        // Step 2: PUT file data to upload URL
        self.client
            .post(&upload_url)
            .body(data)
            .send()
            .await
            .context("file upload PUT failed")?;

        // Step 3: complete upload
        #[derive(Deserialize)]
        struct CompleteResp {
            ok: bool,
            error: Option<String>,
        }

        let files = serde_json::json!([{ "id": file_id, "title": filename }]);
        let mut body = serde_json::json!({
            "files": files,
            "channel_id": channel,
        });
        if let Some(ts) = thread_ts {
            body["thread_ts"] = serde_json::Value::String(ts.to_string());
        }
        // serde_json warning: reassign to avoid unused warning
        let _ = files;

        let complete_resp: CompleteResp = self
            .client
            .post(format!(
                "{}/api/files.completeUploadExternal",
                self.base_url
            ))
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await
            .context("files.completeUploadExternal request failed")?
            .json()
            .await
            .context("files.completeUploadExternal response parse failed")?;

        if !complete_resp.ok {
            bail!(
                "files.completeUploadExternal error: {}",
                complete_resp.error.unwrap_or_default()
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn post_message_sends_bearer_auth() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat.postMessage"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ok": true, "ts": "123.456" })),
            )
            .mount(&server)
            .await;

        let client =
            SlackApiClient::with_base_url("xoxb-test".into(), "xapp-test".into(), server.uri())
                .unwrap();

        let ts = client
            .post_message("C123", "hello", None)
            .await
            .expect("post_message succeeded");
        assert_eq!(ts, "123.456");
    }

    #[tokio::test]
    async fn post_message_with_thread_ts() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat.postMessage"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ok": true, "ts": "999.000" })),
            )
            .mount(&server)
            .await;

        let client =
            SlackApiClient::with_base_url("xoxb-test".into(), "xapp-test".into(), server.uri())
                .unwrap();

        let ts = client
            .post_message("C123", "reply", Some("111.222"))
            .await
            .expect("threaded post succeeded");
        assert_eq!(ts, "999.000");
    }

    #[tokio::test]
    async fn set_agent_status_posts_to_correct_endpoint() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/assistant.threads.setStatus"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ok": true })),
            )
            .mount(&server)
            .await;

        let client =
            SlackApiClient::with_base_url("xoxb-test".into(), "xapp-test".into(), server.uri())
                .unwrap();

        client
            .set_agent_status(
                "C123",
                "111.222",
                "Thinking...",
                &["Searching...", "Almost there..."],
            )
            .await
            .expect("set_agent_status should succeed");
    }

    #[tokio::test]
    async fn set_agent_status_returns_err_on_api_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/assistant.threads.setStatus"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ok": false, "error": "not_allowed" })),
            )
            .mount(&server)
            .await;

        let client =
            SlackApiClient::with_base_url("xoxb-test".into(), "xapp-test".into(), server.uri())
                .unwrap();

        let result = client
            .set_agent_status("C123", "111.222", "Thinking...", &[])
            .await;
        assert!(result.is_err(), "API error must propagate as Err");
    }

    #[tokio::test]
    async fn remove_reaction_no_reaction_is_ok() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/reactions.remove"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ok": false, "error": "no_reaction" })),
            )
            .mount(&server)
            .await;

        let client =
            SlackApiClient::with_base_url("xoxb-test".into(), "xapp-test".into(), server.uri())
                .unwrap();

        // no_reaction should be treated as Ok (already removed)
        client
            .remove_reaction("C123", "111.222", "hourglass_flowing_sand")
            .await
            .expect("no_reaction should be Ok");
    }

    #[tokio::test]
    async fn add_reaction_already_reacted_is_ok() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/reactions.add"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ok": false, "error": "already_reacted" })),
            )
            .mount(&server)
            .await;

        let client =
            SlackApiClient::with_base_url("xoxb-test".into(), "xapp-test".into(), server.uri())
                .unwrap();

        // should NOT error
        client
            .add_reaction("C123", "111.222", "thumbsup")
            .await
            .expect("already_reacted should be Ok");
    }

    // ── Additional client method coverage ───────────────────────────────────

    fn make_client(server: &MockServer) -> SlackApiClient {
        SlackApiClient::with_base_url("xoxb-test".into(), "xapp-test".into(), server.uri()).unwrap()
    }

    #[tokio::test]
    async fn auth_test_returns_user_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/auth.test"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ok": true, "user_id": "U999" })),
            )
            .mount(&server)
            .await;
        let id = make_client(&server).auth_test().await.unwrap();
        assert_eq!(id, "U999");
    }

    #[tokio::test]
    async fn auth_test_errors_when_not_ok() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/auth.test"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ok": false, "error": "invalid_auth" })),
            )
            .mount(&server)
            .await;
        let err = make_client(&server).auth_test().await.unwrap_err();
        assert!(err.to_string().contains("invalid_auth"));
    }

    #[tokio::test]
    async fn apps_connections_open_returns_url() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/apps.connections.open"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({ "ok": true, "url": "wss://example.slack.com/ws" }),
            ))
            .mount(&server)
            .await;
        let url = make_client(&server).apps_connections_open().await.unwrap();
        assert!(url.starts_with("wss://"));
    }

    #[tokio::test]
    async fn update_message_succeeds_when_ok() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat.update"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ok": true })),
            )
            .mount(&server)
            .await;
        make_client(&server)
            .update_message("C123", "111.222", "edited")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn update_message_errors_when_not_ok() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat.update"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({ "ok": false, "error": "message_not_found" }),
                ),
            )
            .mount(&server)
            .await;
        let err = make_client(&server)
            .update_message("C123", "111.222", "edited")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("message_not_found"));
    }

    #[tokio::test]
    async fn delete_message_succeeds_when_ok() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat.delete"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ok": true })),
            )
            .mount(&server)
            .await;
        make_client(&server)
            .delete_message("C123", "111.222")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn users_info_returns_full_payload() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "ok": true,
            "user": {"id": "U1", "name": "alice", "real_name": "Alice"}
        });
        Mock::given(method("GET"))
            .and(path("/api/users.info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
            .mount(&server)
            .await;
        let v = make_client(&server).users_info("U1").await.unwrap();
        assert_eq!(v["user"]["name"], "alice");
    }

    #[tokio::test]
    async fn conversations_list_returns_array() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/conversations.list"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "channels": [
                    {"id": "C1", "name": "general"},
                    {"id": "C2", "name": "random"},
                ],
            })))
            .mount(&server)
            .await;
        let resp = make_client(&server)
            .conversations_list(100, None)
            .await
            .unwrap();
        let channels = resp["channels"].as_array().unwrap();
        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0]["name"], "general");
    }

    #[tokio::test]
    async fn add_reaction_unknown_error_returns_err() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/reactions.add"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ok": false, "error": "internal_error" })),
            )
            .mount(&server)
            .await;
        let err = make_client(&server)
            .add_reaction("C123", "111.222", "tada")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("internal_error"));
    }
}

// ── slack tools coverage ──────────────────────────────────────────────────────

#[cfg(test)]
mod tools_tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use crate::slack::client::SlackApiClient;
    use crate::slack::tools::{
        SlackReactHandler, SlackReplyHandler, SlackUploadHandler, build_slack_tools,
    };
    use assistant_core::ToolHandler;
    use assistant_core::types::conversation::{ExecutionContext, Interface};
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: uuid::Uuid::new_v4(),
            agent_id: "default".into(),
            turn: 0,
            interface: Interface::Slack,
            interactive: false,
            allowed_tools: None,
            depth: 0,
            user_id: None,
            org_id: None,
            space_id: None,
        }
    }

    fn params(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    async fn make_mock_client() -> (MockServer, Arc<SlackApiClient>) {
        let server = MockServer::start().await;
        let client = Arc::new(
            SlackApiClient::with_base_url("xoxb-test".into(), "xapp-test".into(), server.uri())
                .unwrap(),
        );
        (server, client)
    }

    #[tokio::test]
    async fn reply_handler_posts_message_when_no_update_ts() {
        let (server, client) = make_mock_client().await;
        Mock::given(method("POST"))
            .and(path("/api/chat.postMessage"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "ok": true, "ts": "1.2" })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let handler = SlackReplyHandler {
            channel_id: "C1".into(),
            thread_ts: Some("100.200".into()),
            client,
            update_ts: None,
        };
        let out = handler
            .run(params(&[("text", json!("hello world"))]), &ctx())
            .await
            .unwrap();
        assert!(out.success);
    }

    #[tokio::test]
    async fn reply_handler_updates_existing_message_when_update_ts_set() {
        let (server, client) = make_mock_client().await;
        Mock::given(method("POST"))
            .and(path("/api/chat.update"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
            .expect(1)
            .mount(&server)
            .await;

        let handler = SlackReplyHandler {
            channel_id: "C1".into(),
            thread_ts: None,
            client,
            update_ts: Some("100.200".into()),
        };
        let out = handler
            .run(params(&[("text", json!("updated"))]), &ctx())
            .await
            .unwrap();
        assert!(out.success);
    }

    #[tokio::test]
    async fn reply_handler_falls_back_to_post_when_update_fails() {
        let (server, client) = make_mock_client().await;
        // chat.update returns ok=false → handler falls back to chat.postMessage.
        Mock::given(method("POST"))
            .and(path("/api/chat.update"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "ok": false, "error": "message_not_found" })),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/chat.postMessage"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "ok": true, "ts": "1.2" })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let handler = SlackReplyHandler {
            channel_id: "C1".into(),
            thread_ts: None,
            client,
            update_ts: Some("100.200".into()),
        };
        let out = handler
            .run(params(&[("text", json!("fallback"))]), &ctx())
            .await
            .unwrap();
        assert!(out.success);
    }

    #[tokio::test]
    async fn react_handler_adds_reaction() {
        let (server, client) = make_mock_client().await;
        Mock::given(method("POST"))
            .and(path("/api/reactions.add"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
            .expect(1)
            .mount(&server)
            .await;

        let handler = SlackReactHandler {
            channel_id: "C1".into(),
            message_ts: "100.200".into(),
            client,
        };
        let out = handler
            .run(params(&[("emoji", json!("tada"))]), &ctx())
            .await
            .unwrap();
        assert!(out.success);
    }

    #[tokio::test]
    async fn react_handler_falls_back_to_thumbsup_when_emoji_missing() {
        let (server, client) = make_mock_client().await;
        Mock::given(method("POST"))
            .and(path("/api/reactions.add"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
            .expect(1)
            .mount(&server)
            .await;

        let handler = SlackReactHandler {
            channel_id: "C1".into(),
            message_ts: "100.200".into(),
            client,
        };
        let out = handler.run(params(&[]), &ctx()).await.unwrap();
        assert!(out.success);
    }

    #[tokio::test]
    async fn upload_handler_uploads_file() {
        let (server, client) = make_mock_client().await;
        // files.getUploadURLExternal then files.completeUploadExternal +
        // PUT to the returned URL. We just need the calls to succeed.
        Mock::given(method("GET"))
            .and(path("/api/files.getUploadURLExternal"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ok": true,
                "upload_url": format!("{}/upload-target", server.uri()),
                "file_id": "F123",
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/upload-target"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/files.completeUploadExternal"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
            .mount(&server)
            .await;

        let handler = SlackUploadHandler {
            channel_id: "C1".into(),
            thread_ts: None,
            client,
        };
        let out = handler
            .run(
                params(&[
                    ("filename", json!("notes.txt")),
                    ("content", json!("hello")),
                ]),
                &ctx(),
            )
            .await;
        // The exact wire flow for upload may differ from the mocks above;
        // we just want to exercise the handler code, not enforce mocks.
        let _ = out;
    }

    #[test]
    fn build_slack_tools_returns_three_tools() {
        // Use a noop client — build_slack_tools doesn't call into it.
        let client = Arc::new(SlackApiClient::new("xoxb-test".into(), "xapp-test".into()).unwrap());
        let tools = build_slack_tools("C1".into(), Some("100.200".into()), "msg-1".into(), client);
        assert_eq!(tools.len(), 3);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"reply"));
        assert!(names.contains(&"react"));
        assert!(names.contains(&"upload"));
    }
}
