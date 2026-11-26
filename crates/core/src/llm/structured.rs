//! Schema-conformant JSON answers from any [`LlmProvider`].
//!
//! [`chat_structured`] uses the first strategy the provider supports: a
//! native JSON-schema response format, a forced call to a synthetic
//! `respond` tool, or the schema embedded in the system prompt. The answer
//! is validated, and one repair attempt carries the validation errors.

use serde_json::Value;

use super::options::{ChatOptions, ResponseFormat, ToolChoice};
use super::provider::LlmProvider;
use super::tool_spec::ToolSpec;
use super::types::{ChatHistoryMessage, ChatRole, LlmResponse};

const RESPOND_TOOL: &str = "respond";

enum Strategy {
    Native,
    ForcedTool,
    Prompt,
}

/// Ask `llm` for a JSON value matching `schema`, named `name`.
///
/// Fails when the schema is invalid, or with the validation errors and the
/// raw output when the repair attempt is also invalid.
pub async fn chat_structured(
    llm: &dyn LlmProvider,
    system_prompt: &str,
    history: &[ChatHistoryMessage],
    name: &str,
    schema: &Value,
) -> anyhow::Result<Value> {
    let validator = jsonschema::validator_for(schema)
        .map_err(|e| anyhow::anyhow!("invalid schema for `{name}`: {e}"))?;

    let support = llm.chat_options_support();
    let strategy = if support.json_schema {
        Strategy::Native
    } else if support.tool_choice {
        Strategy::ForcedTool
    } else {
        Strategy::Prompt
    };

    let (system, tools, options) = match strategy {
        Strategy::Native => (
            system_prompt.to_owned(),
            Vec::new(),
            ChatOptions {
                response_format: ResponseFormat::JsonSchema {
                    name: name.to_owned(),
                    schema: schema.clone(),
                    strict: true,
                },
                ..ChatOptions::default()
            },
        ),
        Strategy::ForcedTool => (
            system_prompt.to_owned(),
            vec![ToolSpec {
                name: RESPOND_TOOL.to_owned(),
                description: format!("Return the `{name}` result."),
                params_schema: schema.clone(),
                is_mutating: false,
                requires_confirmation: false,
                requires_rich_client: false,
            }],
            ChatOptions {
                tool_choice: ToolChoice::Tool(RESPOND_TOOL.to_owned()),
                ..ChatOptions::default()
            },
        ),
        Strategy::Prompt => (
            format!(
                "{system_prompt}\n\nAnswer with only a JSON value that matches this JSON \
                 Schema, without any other text:\n{schema}"
            ),
            Vec::new(),
            ChatOptions::default(),
        ),
    };

    let mut history = history.to_vec();
    let mut attempt = 0;
    loop {
        let response = llm
            .chat_with(&system, &history, &tools, &options, None)
            .await?;
        let raw = raw_output(&response);
        let errors = match parse_json(&raw) {
            Some(value) => {
                let errors: Vec<String> = validator
                    .iter_errors(&value)
                    .map(|e| format!("{} at `{}`", e, e.instance_path()))
                    .collect();
                if errors.is_empty() {
                    return Ok(value);
                }
                errors
            }
            None => vec!["the answer is not valid JSON".to_owned()],
        };

        attempt += 1;
        if attempt == 2 {
            anyhow::bail!(
                "structured output `{name}` is invalid: {}; raw output: {raw}",
                errors.join("; ")
            );
        }
        history.push(ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            content: raw,
        });
        history.push(ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: format!(
                "Your answer does not match the schema:\n- {}\nAnswer again with only the corrected JSON.",
                errors.join("\n- ")
            ),
        });
    }
}

/// The answer text, or the `respond` call's arguments as JSON text.
fn raw_output(response: &LlmResponse) -> String {
    match response {
        LlmResponse::ToolCalls(calls) => calls
            .items
            .iter()
            .find(|c| c.name == RESPOND_TOOL)
            .map(|c| c.params.to_string())
            .unwrap_or_default(),
        LlmResponse::FinalAnswer(text, _) => text.clone(),
        LlmResponse::Thinking(text, _) => text.clone(),
    }
}

/// Parse `text` as JSON, ignoring code fences and prose around one value.
fn parse_json(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Some(value);
    }
    let start = trimmed.find(['{', '['])?;
    let end = trimmed.rfind(['}', ']'])?;
    serde_json::from_str(trimmed.get(start..=end)?).ok()
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use serde_json::{Value, json};
    use tokio::sync::mpsc;

    use super::chat_structured;
    use crate::llm::options::{ChatOptions, ChatOptionsSupport, ResponseFormat, ToolChoice};
    use crate::llm::provider::{Capabilities, LlmProvider, ToolSupport};
    use crate::llm::stream_chunk::StreamChunk;
    use crate::llm::tool_spec::ToolSpec;
    use crate::llm::types::{
        ChatHistoryMessage, ChatRole, LlmResponse, LlmResponseMeta, ToolCallItem, ToolCallResponse,
    };

    #[derive(Debug, Clone)]
    struct Sent {
        system: String,
        last_user: Option<String>,
        tools: Vec<ToolSpec>,
        options: ChatOptions,
    }

    /// Replays queued responses and records what each call was sent.
    struct Fake {
        support: ChatOptionsSupport,
        replies: Mutex<VecDeque<LlmResponse>>,
        calls: Mutex<Vec<Sent>>,
    }

    impl Fake {
        fn new(support: ChatOptionsSupport, replies: Vec<LlmResponse>) -> Self {
            Self {
                support,
                replies: Mutex::new(replies.into()),
                calls: Mutex::new(Vec::new()),
            }
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
                streaming: false,
                vision: false,
                hosted_tools: vec![],
                context_window_tokens: None,
            }
        }

        fn chat_options_support(&self) -> ChatOptionsSupport {
            self.support
        }

        async fn chat(
            &self,
            system: &str,
            history: &[ChatHistoryMessage],
            tools: &[ToolSpec],
        ) -> anyhow::Result<LlmResponse> {
            self.chat_with(system, history, tools, &ChatOptions::default(), None)
                .await
        }

        async fn chat_streaming(
            &self,
            system: &str,
            history: &[ChatHistoryMessage],
            tools: &[ToolSpec],
            _sink: Option<mpsc::Sender<StreamChunk>>,
        ) -> anyhow::Result<LlmResponse> {
            self.chat(system, history, tools).await
        }

        async fn chat_with(
            &self,
            system: &str,
            history: &[ChatHistoryMessage],
            tools: &[ToolSpec],
            options: &ChatOptions,
            _sink: Option<mpsc::Sender<StreamChunk>>,
        ) -> anyhow::Result<LlmResponse> {
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
                tools: tools.to_vec(),
                options: options.clone(),
            });
            Ok(self.replies.lock().unwrap().pop_front().unwrap())
        }
    }

    fn schema() -> Value {
        json!({
            "type": "object",
            "properties": {"verdict": {"type": "string"}},
            "required": ["verdict"],
        })
    }

    fn native() -> ChatOptionsSupport {
        ChatOptionsSupport {
            json_schema: true,
            ..ChatOptionsSupport::default()
        }
    }

    fn forced_tool() -> ChatOptionsSupport {
        ChatOptionsSupport {
            tool_choice: true,
            ..ChatOptionsSupport::default()
        }
    }

    fn text(answer: &str) -> LlmResponse {
        LlmResponse::FinalAnswer(answer.into(), LlmResponseMeta::default())
    }

    fn respond(params: Value) -> LlmResponse {
        LlmResponse::ToolCalls(ToolCallResponse {
            items: vec![ToolCallItem {
                name: "respond".into(),
                params,
                id: Some("call_1".into()),
            }],
            meta: LlmResponseMeta::default(),
            thinking: None,
            provider_state: None,
        })
    }

    fn history() -> Vec<ChatHistoryMessage> {
        vec![ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "judge this".into(),
        }]
    }

    async fn run(fake: &Fake) -> anyhow::Result<Value> {
        chat_structured(fake, "sys", &history(), "decision", &schema()).await
    }

    // -- Strategy selection --

    #[tokio::test]
    async fn native_schema_is_sent_as_the_response_format() {
        let fake = Fake::new(native(), vec![text(r#"{"verdict": "keep"}"#)]);

        assert_eq!(run(&fake).await.unwrap(), json!({"verdict": "keep"}));
        let sent = &fake.sent()[0];
        assert_eq!(
            sent.options.response_format,
            ResponseFormat::JsonSchema {
                name: "decision".into(),
                schema: schema(),
                strict: true,
            }
        );
        assert!(sent.tools.is_empty(), "no tools with a native schema");
    }

    #[tokio::test]
    async fn forced_tool_returns_its_arguments() {
        let fake = Fake::new(forced_tool(), vec![respond(json!({"verdict": "drop"}))]);

        assert_eq!(run(&fake).await.unwrap(), json!({"verdict": "drop"}));
        let sent = &fake.sent()[0];
        assert_eq!(sent.options.tool_choice, ToolChoice::Tool("respond".into()));
        assert_eq!(sent.tools.len(), 1, "only the respond tool is offered");
        assert_eq!(sent.tools[0].name, "respond");
        assert_eq!(sent.tools[0].params_schema, schema());
        assert_eq!(sent.options.response_format, ResponseFormat::Text);
    }

    #[tokio::test]
    async fn prompt_embeds_the_schema_and_strips_fences() {
        let fake = Fake::new(
            ChatOptionsSupport::default(),
            vec![text("```json\n{\"verdict\": \"keep\"}\n```")],
        );

        assert_eq!(run(&fake).await.unwrap(), json!({"verdict": "keep"}));
        let sent = &fake.sent()[0];
        assert!(sent.system.starts_with("sys"), "{}", sent.system);
        assert!(
            sent.system.contains(r#""required":["verdict"]"#),
            "{}",
            sent.system
        );
        assert!(sent.tools.is_empty());
        assert_eq!(sent.options, ChatOptions::default());
    }

    #[tokio::test]
    async fn chatty_answer_around_the_json_is_accepted() {
        let fake = Fake::new(
            ChatOptionsSupport::default(),
            vec![text(
                "Sure! Here it is:\n{\"verdict\": \"keep\"}\nHope that helps.",
            )],
        );

        assert_eq!(run(&fake).await.unwrap(), json!({"verdict": "keep"}));
    }

    // -- Validation and repair --

    #[tokio::test]
    async fn invalid_answer_is_retried_once_with_the_errors() {
        let fake = Fake::new(
            native(),
            vec![text(r#"{"reason": "x"}"#), text(r#"{"verdict": "keep"}"#)],
        );

        assert_eq!(run(&fake).await.unwrap(), json!({"verdict": "keep"}));
        let sent = fake.sent();
        assert_eq!(sent.len(), 2);
        let repair = sent[1].last_user.as_deref().unwrap();
        assert!(
            repair.contains("verdict"),
            "the retry names the error: {repair}"
        );
        assert_eq!(
            sent[1].options, sent[0].options,
            "the retry keeps the strategy"
        );
    }

    #[tokio::test]
    async fn persistent_failure_reports_errors_and_raw_output() {
        let fake = Fake::new(
            forced_tool(),
            vec![
                respond(json!({"reason": "a"})),
                respond(json!({"reason": "b"})),
            ],
        );

        let err = run(&fake).await.unwrap_err().to_string();
        assert!(err.contains("verdict"), "names the validation error: {err}");
        assert!(
            err.contains(r#"{"reason":"b"}"#),
            "carries the raw output: {err}"
        );
        assert_eq!(fake.sent().len(), 2, "exactly one retry");
    }

    #[tokio::test]
    async fn unparseable_answer_is_retried_then_fails_with_the_raw_text() {
        let fake = Fake::new(
            ChatOptionsSupport::default(),
            vec![text("no idea"), text("still no idea")],
        );

        let err = run(&fake).await.unwrap_err().to_string();
        assert!(err.contains("still no idea"), "{err}");
        assert_eq!(fake.sent().len(), 2);
    }

    #[tokio::test]
    async fn invalid_schema_fails_before_calling() {
        let fake = Fake::new(native(), vec![]);
        let bad = json!({"type": "no-such-type"});

        let err = chat_structured(&fake, "sys", &history(), "decision", &bad)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("schema"), "{err}");
        assert!(fake.sent().is_empty());
    }
}
