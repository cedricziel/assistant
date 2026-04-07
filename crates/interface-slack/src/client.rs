//! Thin reqwest-based Slack API client.
//!
//! Covers only the endpoints needed by the assistant:
//! - `apps.connections.open` — get a Socket Mode WebSocket URL
//! - `chat.postMessage` — post a reply in a channel / thread
//! - `reactions.add` — add an emoji reaction
//! - `auth.test` — resolve the bot user ID
//! - `conversations.replies` — fetch thread history for context seeding
//! - `files.getUploadURLExternal` + `files.completeUploadExternal` — upload files

use anyhow::{bail, Context, Result};
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
            .query(&[("channel", channel), ("ts", ts)])
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
}
