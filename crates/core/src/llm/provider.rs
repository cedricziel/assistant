//! `LlmProvider` trait — the single abstraction point for all LLM backends.
//!
//! Implement this trait to plug in a new provider (Ollama, OpenAI, Anthropic, …).
//! All orchestration and skill-execution code works against `Arc<dyn LlmProvider>`
//! so no provider-specific code leaks into the core runtime.

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::options::{ChatOptions, ChatOptionsSupport, ToolChoice};
use super::stream_chunk::StreamChunk;
use super::tool_spec::ToolSpec;
use super::types::{ChatHistoryMessage, ChatRole, LlmResponse};

// ── Capabilities ─────────────────────────────────────────────────────────────

/// Level of tool-calling support offered by a provider.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolSupport {
    /// Provider understands the `tools` / `tool_calls` wire protocol natively.
    Native,
    /// Provider has no structured tool-calling support.
    None,
}

/// Static metadata describing what a provider can do.
#[derive(Debug, Clone)]
pub struct Capabilities {
    /// Whether and how the provider supports tool / function calling.
    pub tools: ToolSupport,
    /// Whether the provider supports streaming token output.
    pub streaming: bool,
    /// Whether the provider accepts image inputs.
    pub vision: bool,
    /// Hosted tools supplied directly by the provider (e.g. Anthropic web search).
    pub hosted_tools: Vec<HostedTool>,
    /// The model's context window size in tokens, if known.
    ///
    /// When set, the orchestrator uses this to override the default compaction
    /// `context_window_tokens` so that history compaction triggers at the right
    /// threshold for the actual model being used.
    pub context_window_tokens: Option<u64>,
}

/// Provider-managed tools that should suppress local equivalents.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostedTool {
    /// Anthropic-managed `web_search` tool.
    WebSearch,
    /// Anthropic-managed `web_fetch` tool.
    WebFetch,
}

// ── LlmProvider trait ─────────────────────────────────────────────────────────

/// Common interface for LLM backends.
///
/// All internal orchestration code works against `Arc<dyn LlmProvider>` so the
/// concrete provider (Ollama, OpenAI, Anthropic, …) is swapped without touching
/// the runtime or tool-executor.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Return static metadata about this provider's capabilities.
    fn capabilities(&self) -> Capabilities;

    /// Send a chat turn and return the model's response.
    ///
    /// # Parameters
    /// * `system_prompt` – base system instructions
    /// * `history` – previous messages in the conversation
    /// * `tools` – tools available for this turn (passed as native tool specs)
    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse>;

    /// Like [`chat`] but streams typed chunks through `chunk_sink` as they are
    /// generated.
    ///
    /// [`StreamChunk::Text`] carries final-answer tokens visible to the user.
    /// [`StreamChunk::Thinking`] carries internal reasoning tokens (when the
    /// provider supports extended thinking).
    ///
    /// The method still returns the complete [`LlmResponse`] once generation
    /// is finished.
    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse>;

    /// Which [`ChatOptions`] this provider sends natively.
    fn chat_options_support(&self) -> ChatOptionsSupport {
        ChatOptionsSupport::default()
    }

    /// Waits until [`capabilities`](Self::capabilities) and
    /// [`chat_options_support`](Self::chat_options_support) are known, for
    /// providers that look them up after construction. Callers that read
    /// them before the first call await this first.
    async fn ready(&self) {}

    /// Like [`chat_streaming`](Self::chat_streaming), with per-call options.
    /// Without a sink this calls [`chat`](Self::chat).
    ///
    /// The default is [`chat_with_fallback`], for providers without native
    /// options. Providers with native support override this method.
    async fn chat_with(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        options: &ChatOptions,
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        chat_with_fallback(self, system_prompt, history, tools, options, chunk_sink).await
    }

    /// Compute a dense vector embedding for `text`.
    ///
    /// Returns an error if the provider does not support embeddings.
    /// The default implementation always returns an error so that existing
    /// providers do not need to be updated until they are ready.
    async fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Err(anyhow::anyhow!("Embedding not supported by this provider"))
    }

    /// Compute dense vector embeddings for a batch of texts in a single
    /// request. The default implementation falls back to calling [`embed`]
    /// once per text — providers that natively batch (Ollama, OpenAI, Voyage)
    /// SHOULD override this for a meaningful speed-up (#30).
    ///
    /// The returned vector MUST have the same length and order as the input
    /// slice. If any single text fails, the whole batch returns the error.
    async fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        let mut out = Vec::with_capacity(texts.len());
        for text in texts {
            out.push(self.embed(text).await?);
        }
        Ok(out)
    }

    // ── Provider identity (for OTel GenAI semantic conventions) ───────────

    /// Short, stable identifier for the provider system (e.g. `"ollama"`,
    /// `"anthropic"`).  Maps to the OTel `gen_ai.provider.name` attribute.
    fn provider_name(&self) -> &str {
        "unknown"
    }

    /// The model name configured for this provider instance (e.g.
    /// `"qwen2.5:7b"`, `"claude-opus-4-6"`).
    /// Maps to the OTel `gen_ai.request.model` attribute.
    fn model_name(&self) -> &str {
        "unknown"
    }

    /// Base URL / address of the inference server (e.g.
    /// `"http://localhost:11434"`, `"https://api.anthropic.com"`).
    /// Maps to the OTel `server.address` attribute.
    fn server_address(&self) -> &str {
        ""
    }
}

/// [`LlmProvider::chat_with`] for providers without native options:
/// `parallel_tool_calls` and `response_format` are ignored, `ToolChoice::None`
/// sends no tools, and `Required` or `Tool` asks for the call in the system
/// prompt, retries once with the request repeated as the last user message,
/// then fails. Providers that support options for some models only call this
/// for the others.
pub async fn chat_with_fallback<L: LlmProvider + ?Sized>(
    llm: &L,
    system_prompt: &str,
    history: &[ChatHistoryMessage],
    tools: &[ToolSpec],
    options: &ChatOptions,
    chunk_sink: Option<mpsc::Sender<StreamChunk>>,
) -> anyhow::Result<LlmResponse> {
    let call = |system: String, history: Vec<ChatHistoryMessage>, tools: Vec<ToolSpec>| {
        let sink = chunk_sink.clone();
        async move {
            match sink {
                Some(sink) => {
                    llm.chat_streaming(&system, &history, &tools, Some(sink))
                        .await
                }
                None => llm.chat(&system, &history, &tools).await,
            }
        }
    };

    let (instruction, offered) = match &options.tool_choice {
        ToolChoice::Auto => {
            return call(system_prompt.to_owned(), history.to_vec(), tools.to_vec()).await;
        }
        ToolChoice::None => {
            return call(system_prompt.to_owned(), history.to_vec(), Vec::new()).await;
        }
        ToolChoice::Required => (
            "You must answer by calling one of the available tools.".to_owned(),
            tools.to_vec(),
        ),
        ToolChoice::Tool(name) => (
            format!("You must answer by calling the tool `{name}`."),
            tools.iter().filter(|t| &t.name == name).cloned().collect(),
        ),
    };
    let satisfied = |response: &LlmResponse| match (response, &options.tool_choice) {
        (LlmResponse::ToolCalls(calls), ToolChoice::Tool(name)) => {
            calls.items.iter().any(|c| &c.name == name)
        }
        (LlmResponse::ToolCalls(calls), _) => !calls.items.is_empty(),
        _ => false,
    };

    let system = format!("{system_prompt}\n\n{instruction}");
    let first = call(system.clone(), history.to_vec(), offered.clone()).await?;
    if satisfied(&first) {
        return Ok(first);
    }
    let mut retry_history = history.to_vec();
    retry_history.push(ChatHistoryMessage::Text {
        role: ChatRole::User,
        content: instruction.clone(),
    });
    let second = call(system, retry_history, offered).await?;
    if satisfied(&second) {
        return Ok(second);
    }
    anyhow::bail!(
        "{} did not follow the tool choice: {instruction}",
        llm.provider_name()
    )
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;
    use crate::llm::options::ToolChoice;
    use crate::llm::types::{ChatRole, LlmResponseMeta, ToolCallItem, ToolCallResponse};

    /// Replays queued responses and records what each call was sent.
    #[derive(Default)]
    struct Fake {
        replies: Mutex<VecDeque<LlmResponse>>,
        calls: Mutex<Vec<Sent>>,
    }

    #[derive(Debug, Clone)]
    struct Sent {
        system: String,
        last_user: Option<String>,
        tools: Vec<String>,
        streamed: bool,
    }

    impl Fake {
        fn replying(replies: Vec<LlmResponse>) -> Self {
            Self {
                replies: Mutex::new(replies.into()),
                ..Default::default()
            }
        }

        fn record(
            &self,
            system: &str,
            history: &[ChatHistoryMessage],
            tools: &[ToolSpec],
            streamed: bool,
        ) -> LlmResponse {
            let last_user = history.iter().rev().find_map(|m| match m {
                ChatHistoryMessage::Text {
                    role: ChatRole::User,
                    content,
                } => Some(content.clone()),
                _ => None,
            });
            self.calls.lock().unwrap().push(Sent {
                system: system.to_string(),
                last_user,
                tools: tools.iter().map(|t| t.name.clone()).collect(),
                streamed,
            });
            self.replies.lock().unwrap().pop_front().unwrap()
        }

        fn sent(&self) -> Vec<Sent> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl LlmProvider for Fake {
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                tools: ToolSupport::Native,
                streaming: true,
                vision: false,
                hosted_tools: vec![],
                context_window_tokens: None,
            }
        }

        async fn chat(
            &self,
            system: &str,
            history: &[ChatHistoryMessage],
            tools: &[ToolSpec],
        ) -> anyhow::Result<LlmResponse> {
            Ok(self.record(system, history, tools, false))
        }

        async fn chat_streaming(
            &self,
            system: &str,
            history: &[ChatHistoryMessage],
            tools: &[ToolSpec],
            sink: Option<mpsc::Sender<StreamChunk>>,
        ) -> anyhow::Result<LlmResponse> {
            Ok(self.record(system, history, tools, sink.is_some()))
        }
    }

    fn tool(name: &str) -> ToolSpec {
        ToolSpec {
            name: name.into(),
            description: String::new(),
            params_schema: serde_json::json!({"type": "object"}),
            is_mutating: false,
            requires_confirmation: false,
            requires_rich_client: false,
        }
    }

    fn text(answer: &str) -> LlmResponse {
        LlmResponse::FinalAnswer(answer.into(), LlmResponseMeta::default())
    }

    fn calls(name: &str) -> LlmResponse {
        LlmResponse::ToolCalls(ToolCallResponse {
            items: vec![ToolCallItem {
                name: name.into(),
                params: serde_json::json!({}),
                id: None,
            }],
            meta: LlmResponseMeta::default(),
            thinking: None,
            provider_state: None,
        })
    }

    fn user(text: &str) -> Vec<ChatHistoryMessage> {
        vec![ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: text.into(),
        }]
    }

    fn with(choice: ToolChoice) -> ChatOptions {
        ChatOptions {
            tool_choice: choice,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn default_options_are_a_plain_call() {
        let fake = Fake::replying(vec![text("a"), text("b")]);
        let tools = [tool("bash")];
        fake.chat_with("sys", &user("hi"), &tools, &ChatOptions::default(), None)
            .await
            .unwrap();
        let (tx, _rx) = mpsc::channel(4);
        fake.chat_with(
            "sys",
            &user("hi"),
            &tools,
            &ChatOptions::default(),
            Some(tx),
        )
        .await
        .unwrap();
        let sent = fake.sent();
        assert_eq!(sent[0].system, "sys", "the prompt is untouched");
        assert_eq!(sent[0].tools, vec!["bash"]);
        assert!(!sent[0].streamed, "no sink means chat()");
        assert!(sent[1].streamed, "a sink means chat_streaming()");
    }

    #[tokio::test]
    async fn tool_choice_none_sends_no_tools() {
        let fake = Fake::replying(vec![text("a")]);
        fake.chat_with(
            "sys",
            &user("hi"),
            &[tool("bash")],
            &with(ToolChoice::None),
            None,
        )
        .await
        .unwrap();
        assert!(fake.sent()[0].tools.is_empty());
    }

    #[tokio::test]
    async fn forced_tool_is_named_and_the_only_tool_offered() {
        let fake = Fake::replying(vec![calls("submit")]);
        let response = fake
            .chat_with(
                "sys",
                &user("hi"),
                &[tool("bash"), tool("submit")],
                &with(ToolChoice::Tool("submit".into())),
                None,
            )
            .await
            .unwrap();
        assert!(matches!(response, LlmResponse::ToolCalls(_)));
        let sent = &fake.sent()[0];
        assert!(
            sent.system.starts_with("sys"),
            "the instruction is appended"
        );
        assert!(sent.system.contains("`submit`"), "{}", sent.system);
        assert_eq!(sent.tools, vec!["submit"]);
    }

    #[tokio::test]
    async fn forced_tool_retries_once_then_fails_naming_it() {
        let fake = Fake::replying(vec![text("nope"), text("still no")]);
        let err = fake
            .chat_with(
                "sys",
                &user("hi"),
                &[tool("submit")],
                &with(ToolChoice::Tool("submit".into())),
                None,
            )
            .await
            .unwrap_err()
            .to_string();
        let sent = fake.sent();
        assert_eq!(sent.len(), 2, "exactly one retry");
        assert!(
            sent[1]
                .last_user
                .as_deref()
                .is_some_and(|u| u.contains("`submit`")),
            "the retry repeats the instruction as the last user message: {:?}",
            sent[1].last_user
        );
        assert!(err.contains("submit"), "{err}");
    }

    #[tokio::test]
    async fn required_accepts_any_tool_after_a_retry() {
        let fake = Fake::replying(vec![text("nope"), calls("bash")]);
        let response = fake
            .chat_with(
                "sys",
                &user("hi"),
                &[tool("bash"), tool("submit")],
                &with(ToolChoice::Required),
                None,
            )
            .await
            .unwrap();
        assert!(matches!(response, LlmResponse::ToolCalls(_)));
        assert_eq!(fake.sent()[0].tools.len(), 2, "all tools stay available");
    }

    #[tokio::test]
    async fn calling_the_wrong_tool_counts_as_a_miss() {
        let fake = Fake::replying(vec![calls("bash"), calls("bash")]);
        let err = fake
            .chat_with(
                "sys",
                &user("hi"),
                &[tool("bash"), tool("submit")],
                &with(ToolChoice::Tool("submit".into())),
                None,
            )
            .await;
        assert!(err.is_err(), "a different tool does not satisfy the choice");
    }
}
