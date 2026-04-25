//! Shared OpenAI Chat Completions client.
//!
//! Provides reusable request building, SSE streaming, and response parsing
//! for any provider that speaks the OpenAI Chat Completions wire format
//! (Moonshot, OpenRouter, and any future OpenAI-compatible backend).

use async_openai::Client;
use async_openai::config::OpenAIConfig as AsyncOpenAIConfig;
use async_openai::types::chat::{ChatCompletionTools, CreateChatCompletionRequestArgs};
use futures::StreamExt as _;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::debug;

use assistant_core::{
    ChatHistoryMessage, LlmResponse, LlmResponseMeta, StreamChunk, ToolCallItem, ToolCallResponse,
    ToolSpec,
};

use crate::http::build_reqwest_client;
use crate::retry::{RetryConfig, is_transient_error_message, with_retry};

pub mod messages;
pub mod tools;

pub use messages::{build_chat_messages, build_raw_messages};
pub use tools::{extract_chat_meta, parse_tool_calls, tool_spec_to_chat};

// ── Configuration ────────────────────────────────────────────────────────────

/// Shared configuration for any OpenAI Chat Completions-compatible provider.
#[derive(Debug, Clone)]
pub struct ChatCompletionsConfig {
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    pub timeout_secs: u64,
    pub max_tokens: u32,
    /// Extra HTTP headers injected into every request.
    /// Used by OpenRouter for `HTTP-Referer` and `X-Title`.
    pub extra_headers: Vec<(String, String)>,
}

// ── Provider ─────────────────────────────────────────────────────────────────

/// Reusable Chat Completions client.
///
/// Handles message conversion, tool-spec mapping, SSE streaming, response
/// parsing.  Provider-specific wrappers (Moonshot, OpenRouter, …) compose
/// this with their own config and capabilities.
pub struct ChatCompletionsProvider {
    /// `async-openai` client configured for the target endpoint.
    client: Client<AsyncOpenAIConfig>,
    /// Provider configuration.
    pub config: ChatCompletionsConfig,
    /// Pre-configured HTTP client with tracing middleware for raw HTTP paths.
    http_client: reqwest_middleware::ClientWithMiddleware,
}

impl ChatCompletionsProvider {
    /// Create a new Chat Completions provider from configuration.
    pub fn new(config: ChatCompletionsConfig) -> anyhow::Result<Self> {
        let mut oai_cfg = AsyncOpenAIConfig::new()
            .with_api_key(&config.api_key)
            .with_api_base(&config.base_url);

        for (key, value) in &config.extra_headers {
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| anyhow::anyhow!("Invalid extra header name {key}: {e}"))?;
            oai_cfg = oai_cfg
                .with_header(header_name, value.as_str())
                .map_err(|e| anyhow::anyhow!("Failed to set extra header {key}: {e}"))?;
        }

        let http = build_reqwest_client(config.timeout_secs)?;
        let client = Client::with_config(oai_cfg).with_http_client(http);

        let traced_client =
            crate::http::build_http_client(config.timeout_secs, &RetryConfig::default())?;

        debug!(
            model = %config.model,
            base_url = %config.base_url,
            "Chat Completions provider initialised"
        );

        Ok(Self {
            client,
            config,
            http_client: traced_client,
        })
    }

    // ── Non-streaming chat ───────────────────────────────────────────────

    /// Send a non-streaming chat request via `async-openai`.
    pub async fn chat_non_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        debug!(model = %self.config.model, tools = tools.len(), "Sending Chat Completions request");

        let (messages, _pending) = build_chat_messages(system_prompt, history);
        let openai_tools: Vec<ChatCompletionTools> = tools
            .iter()
            .map(|t| ChatCompletionTools::Function(tool_spec_to_chat(t)))
            .collect();

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder
            .model(&self.config.model)
            .messages(messages)
            .max_completion_tokens(self.config.max_tokens);
        if !openai_tools.is_empty() {
            builder.tools(openai_tools);
        }
        let request = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Chat Completions request: {e}"))?;

        let retry_config = RetryConfig::default();
        let response = with_retry(
            &retry_config,
            "ChatCompletions",
            |e: &anyhow::Error| is_transient_error_message(&e.to_string()),
            || {
                let req = request.clone();
                let client = &self.client;
                async move {
                    client
                        .chat()
                        .create(req)
                        .await
                        .map_err(|e| anyhow::anyhow!("Chat Completions request failed: {e}"))
                }
            },
        )
        .await?;

        debug!("Chat Completions response received");

        let meta = extract_chat_meta(&response.model, &response.id, &response.usage);
        let choice = response
            .choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("Chat Completions returned empty choices"))?;

        if let Some(ref tool_calls) = choice.message.tool_calls {
            let items = parse_tool_calls(tool_calls);
            if !items.is_empty() {
                debug!(count = items.len(), "Chat Completions: tool calls received");
                return Ok(LlmResponse::ToolCalls(ToolCallResponse {
                    items,
                    meta,
                    thinking: None,
                }));
            }
        }

        let content = choice.message.content.as_deref().unwrap_or("").to_string();
        Ok(LlmResponse::FinalAnswer(content, meta))
    }

    // ── SSE streaming chat ───────────────────────────────────────────────

    /// Send a streaming chat request via `async-openai` SSE.
    pub async fn chat_sse(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        debug!(model = %self.config.model, "Sending streaming Chat Completions request");

        let (messages, _pending) = build_chat_messages(system_prompt, history);
        let openai_tools: Vec<ChatCompletionTools> = tools
            .iter()
            .map(|t| ChatCompletionTools::Function(tool_spec_to_chat(t)))
            .collect();

        let mut builder = CreateChatCompletionRequestArgs::default();
        builder
            .model(&self.config.model)
            .messages(messages)
            .max_completion_tokens(self.config.max_tokens);
        if !openai_tools.is_empty() {
            builder.tools(openai_tools);
        }
        let request = builder.build().map_err(|e| {
            anyhow::anyhow!("Failed to build streaming Chat Completions request: {e}")
        })?;

        let retry_config = RetryConfig::default();
        let mut stream = with_retry(
            &retry_config,
            "ChatCompletions",
            |e: &anyhow::Error| is_transient_error_message(&e.to_string()),
            || {
                let req = request.clone();
                let client = &self.client;
                async move {
                    client.chat().create_stream(req).await.map_err(|e| {
                        anyhow::anyhow!("Chat Completions streaming request failed: {e}")
                    })
                }
            },
        )
        .await?;

        let mut text_buf = String::new();
        let mut tool_map: Vec<(String, String, String)> = Vec::new();
        let mut model_name = String::new();
        let mut response_id = String::new();
        let mut finish_reason: Option<String> = None;
        let mut input_tokens: Option<u64> = None;
        let mut output_tokens: Option<u64> = None;

        while let Some(result) = stream.next().await {
            let chunk =
                result.map_err(|e| anyhow::anyhow!("Chat Completions stream error: {e}"))?;

            if model_name.is_empty() {
                model_name.clone_from(&chunk.model);
            }
            if response_id.is_empty() {
                response_id.clone_from(&chunk.id);
            }
            if let Some(ref usage) = chunk.usage {
                input_tokens = Some(usage.prompt_tokens as u64);
                output_tokens = Some(usage.completion_tokens as u64);
            }

            for choice in &chunk.choices {
                if let Some(ref reason) = choice.finish_reason {
                    finish_reason = Some(format!("{reason:?}").to_lowercase());
                }

                let delta = &choice.delta;

                if let Some(ref content) = delta.content {
                    text_buf.push_str(content);
                    if let Some(ref sink) = chunk_sink {
                        let _ = sink.send(StreamChunk::Text(content.clone())).await;
                    }
                }

                if let Some(ref tc_chunks) = delta.tool_calls {
                    for tc in tc_chunks {
                        let idx = tc.index as usize;
                        while tool_map.len() <= idx {
                            tool_map.push((String::new(), String::new(), String::new()));
                        }
                        if let Some(ref id) = tc.id {
                            tool_map[idx].0.clone_from(id);
                        }
                        if let Some(ref func) = tc.function {
                            if let Some(ref name) = func.name {
                                tool_map[idx].1.clone_from(name);
                            }
                            if let Some(ref args) = func.arguments {
                                tool_map[idx].2.push_str(args);
                            }
                        }
                    }
                }
            }
        }

        debug!("Chat Completions SSE stream complete");

        let meta = LlmResponseMeta {
            model: if model_name.is_empty() {
                None
            } else {
                Some(model_name)
            },
            response_id: if response_id.is_empty() {
                None
            } else {
                Some(response_id)
            },
            finish_reason,
            input_tokens,
            output_tokens,
        };

        if !tool_map.is_empty() {
            let items: Vec<ToolCallItem> = tool_map
                .into_iter()
                .filter(|(_, name, _)| !name.is_empty())
                .map(|(id, name, args_json)| {
                    let params = serde_json::from_str::<Value>(&args_json)
                        .unwrap_or(Value::Object(serde_json::Map::new()));
                    ToolCallItem {
                        name,
                        params,
                        id: if id.is_empty() { None } else { Some(id) },
                    }
                })
                .collect();
            if !items.is_empty() {
                debug!(
                    count = items.len(),
                    "Chat Completions SSE: tool calls received"
                );
                return Ok(LlmResponse::ToolCalls(ToolCallResponse {
                    items,
                    meta,
                    thinking: None,
                }));
            }
        }

        Ok(LlmResponse::FinalAnswer(text_buf, meta))
    }

    // ── Raw HTTP helper ──────────────────────────────────────────────────

    /// Send a raw HTTP POST request and return the JSON body.
    ///
    /// Used by providers that need non-standard request bodies (e.g. Moonshot
    /// `$web_search` with `builtin_function` tools).
    pub async fn raw_http_post(&self, url: &str, body: &Value) -> anyhow::Result<Value> {
        let resp = self
            .http_client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {e}"))?;

        let status = resp.status();
        let resp_body: Value = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Response parse failed: {e}"))?;

        if !status.is_success() {
            let err_msg = resp_body["error"]["message"]
                .as_str()
                .unwrap_or("unknown error");
            anyhow::bail!("API error ({}): {}", status, err_msg);
        }

        Ok(resp_body)
    }
}

// ── LlmProvider blanket helpers ──────────────────────────────────────────────
//
// We deliberately do NOT implement LlmProvider on ChatCompletionsProvider.
// Each concrete provider (Moonshot, OpenRouter, …) implements LlmProvider
// itself so it can report its own `provider_name()`, `capabilities()`, etc.
