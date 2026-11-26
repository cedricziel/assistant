//! OpenRouter Chat Completions request and response types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use assistant_core::types::llm::{OpenRouterReasoning, OpenRouterRouting};
use assistant_core::{LlmResponse, LlmResponseMeta, ToolCallItem, ToolCallResponse};

use super::errors::{ErrorBody, body_error};
use super::reasoning::{budget_exhausted, provider_state, visible_text};

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Value>,
    pub max_completion_tokens: u32,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub stream: bool,
    /// The primary model followed by fallbacks; empty when none are set.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<OpenRouterRouting>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<OpenRouterReasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_options: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(default)]
    pub error: Option<ErrorBody>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    #[serde(default)]
    pub finish_reason: Option<String>,
    pub message: Message,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub reasoning_details: Vec<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ToolCall {
    #[serde(default)]
    pub id: Option<String>,
    pub function: FunctionCall,
}

#[derive(Debug, Deserialize)]
pub struct FunctionCall {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub arguments: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: Option<u64>,
    #[serde(default)]
    pub completion_tokens: Option<u64>,
    /// Cost of the call in credits (US dollars).
    #[serde(default)]
    pub cost: Option<f64>,
    #[serde(default)]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    #[serde(default)]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PromptTokensDetails {
    #[serde(default)]
    pub cached_tokens: Option<u64>,
    #[serde(default)]
    pub cache_write_tokens: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CompletionTokensDetails {
    #[serde(default)]
    pub reasoning_tokens: Option<u64>,
}

impl Usage {
    pub fn apply(&self, meta: &mut LlmResponseMeta) {
        meta.input_tokens = self.prompt_tokens;
        meta.output_tokens = self.completion_tokens;
        meta.cost_usd = self.cost;
        if let Some(details) = &self.prompt_tokens_details {
            meta.cached_input_tokens = details.cached_tokens;
            meta.cache_write_input_tokens = details.cache_write_tokens;
        }
        if let Some(details) = &self.completion_tokens_details {
            meta.reasoning_tokens = details.reasoning_tokens;
        }
    }
}

impl ChatResponse {
    /// The response; reasoning becomes the tool calls' thinking unless
    /// `show_reasoning` is off. A final answer has no slot for thinking.
    /// `model` is the requested model, which stored reasoning is keyed by.
    pub fn into_llm_response(
        self,
        model: &str,
        show_reasoning: bool,
    ) -> anyhow::Result<LlmResponse> {
        if let Some(error) = &self.error
            && self.choices.is_empty()
        {
            return Err(body_error(error));
        }
        let mut meta = LlmResponseMeta {
            model: self.model,
            response_id: self.id,
            ..LlmResponseMeta::default()
        };
        if let Some(usage) = &self.usage {
            usage.apply(&mut meta);
        }
        let choice = self
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("OpenRouter returned no choices"))?;
        let message = choice.message;
        let thinking = visible_text(&message.reasoning_details, message.reasoning.as_deref());
        let items = tool_call_items(message.tool_calls);
        let answer = message.content.unwrap_or_default();
        let reasoned = !thinking.is_empty() || meta.reasoning_tokens.unwrap_or(0) > 0;
        if let Some(error) = budget_exhausted(
            choice.finish_reason.as_deref(),
            &answer,
            !items.is_empty(),
            reasoned,
        ) {
            return Err(error);
        }
        meta.finish_reason = choice.finish_reason;

        if !items.is_empty() {
            return Ok(LlmResponse::ToolCalls(ToolCallResponse {
                items,
                meta,
                thinking: (show_reasoning && !thinking.is_empty()).then_some(thinking),
                provider_state: provider_state(model, message.reasoning_details),
            }));
        }
        Ok(LlmResponse::FinalAnswer(answer, meta))
    }
}

/// Calls without a name are dropped; unparseable arguments become `{}`.
fn tool_call_items(calls: Vec<ToolCall>) -> Vec<ToolCallItem> {
    calls
        .into_iter()
        .filter(|c| !c.function.name.is_empty())
        .map(|c| ToolCallItem {
            params: serde_json::from_str(&c.function.arguments)
                .unwrap_or_else(|_| Value::Object(serde_json::Map::new())),
            name: c.function.name,
            id: c.id,
        })
        .collect()
}
