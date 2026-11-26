//! Follow-up suggestion generator.
//!
//! After a web turn completes, [`generate`] asks the org's LLM for a few short
//! prompts the *user* could send next.  The call is a single non-streamed
//! `chat` with no tools, modelled on the title generator, and runs inside a
//! GenAI span so its cost is visible next to the turn's own spans.

use std::collections::HashSet;

use anyhow::{Context as _, Result};
use opentelemetry::{
    KeyValue, global,
    trace::{Span as _, Status, Tracer as _},
};

use assistant_core::{ChatHistoryMessage, ChatRole, LlmProvider, LlmResponse};

/// Maximum number of characters of each input (last user message, answer)
/// that is sent to the model.
const MAX_INPUT_CHARS: usize = 1_000;

/// Longest suggestion, in characters, that is kept.
const MAX_SUGGESTION_CHARS: usize = 120;

/// Value of the `assistant.feature` span attribute.
const FEATURE: &str = "follow_up_suggestions";

/// How a suggestion call ended, recorded as the `outcome` span attribute.
///
/// Timeouts and aborts happen outside [`generate`] (the caller drops the
/// future) and are recorded by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// At least one suggestion was produced.
    Emitted,
    /// The call succeeded but yielded nothing usable.
    Empty,
    /// The provider call failed.
    Error,
}

impl Outcome {
    /// The span attribute value for this outcome.
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Emitted => "emitted",
            Outcome::Empty => "empty",
            Outcome::Error => "error",
        }
    }
}

/// The result of a successful suggestion call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generated {
    /// Cleaned prompts, at most `max` of them.
    pub prompts: Vec<String>,
    /// How the call ended.
    pub outcome: Outcome,
}

/// Ask `llm` for up to `max` prompts the user could send after `answer`.
///
/// Issues one non-streamed `chat` call with no tools.  Each input is cut to
/// [`MAX_INPUT_CHARS`] characters.  A response that is not a final answer
/// yields an empty list, not an error; only a provider failure is an `Err`.
pub async fn generate(
    last_user_message: &str,
    answer: &str,
    max: usize,
    llm: &dyn LlmProvider,
) -> Result<Generated> {
    let tracer = global::tracer("assistant.follow_ups");
    let model = llm.model_name().to_string();
    let mut span = tracer.start(format!("chat {model}"));
    span.set_attribute(KeyValue::new("gen_ai.operation.name", "chat"));
    span.set_attribute(KeyValue::new(
        "gen_ai.provider.name",
        llm.provider_name().to_string(),
    ));
    span.set_attribute(KeyValue::new("gen_ai.request.model", model));
    span.set_attribute(KeyValue::new("assistant.feature", FEATURE));

    let history = [ChatHistoryMessage::Text {
        role: ChatRole::User,
        content: format!(
            "Last user message:\n{}\n\nAssistant answer:\n{}",
            truncate_chars(last_user_message, MAX_INPUT_CHARS),
            truncate_chars(answer, MAX_INPUT_CHARS),
        ),
    }];

    let response = match llm.chat(&system_prompt(max), &history, &[]).await {
        Ok(response) => response,
        Err(err) => {
            span.set_attribute(KeyValue::new("outcome", Outcome::Error.as_str()));
            span.set_status(Status::error(err.to_string()));
            span.end();
            return Err(err).context("follow-up suggestion LLM call failed");
        }
    };

    let meta = response.meta();
    if let Some(input) = meta.input_tokens {
        span.set_attribute(KeyValue::new("gen_ai.usage.input_tokens", input as i64));
    }
    if let Some(output) = meta.output_tokens {
        span.set_attribute(KeyValue::new("gen_ai.usage.output_tokens", output as i64));
    }

    let prompts = match &response {
        LlmResponse::FinalAnswer(text, _) => parse_suggestions(text, max),
        LlmResponse::ToolCalls(_) | LlmResponse::Thinking(_, _) => Vec::new(),
    };
    let outcome = if prompts.is_empty() {
        Outcome::Empty
    } else {
        Outcome::Emitted
    };
    span.set_attribute(KeyValue::new("outcome", outcome.as_str()));
    span.end();

    Ok(Generated { prompts, outcome })
}

/// Turn the model's raw reply into clean prompts.
///
/// Strips list markers, numbering and wrapping quotes, drops empty lines and
/// lines longer than [`MAX_SUGGESTION_CHARS`] characters, removes
/// case-insensitive duplicates (keeping the first) and keeps the first `max`.
pub fn parse_suggestions(raw: &str, max: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for line in raw.lines() {
        if out.len() >= max {
            break;
        }
        let cleaned = clean_line(line);
        if cleaned.is_empty() || cleaned.chars().count() > MAX_SUGGESTION_CHARS {
            continue;
        }
        if seen.insert(cleaned.to_lowercase()) {
            out.push(cleaned);
        }
    }
    out
}

fn clean_line(line: &str) -> String {
    let s = strip_list_marker(line.trim());
    strip_wrapping_quotes(s.trim()).trim().to_string()
}

/// Strip a leading `-`/`*`/`•` bullet or `1.`/`1)` numbering.
fn strip_list_marker(s: &str) -> &str {
    if let Some(first) = s.chars().next()
        && matches!(first, '-' | '*' | '•')
    {
        let rest = &s[first.len_utf8()..];
        if rest.is_empty() || rest.starts_with(char::is_whitespace) {
            return rest;
        }
    }
    let digits = s.chars().take_while(char::is_ascii_digit).count();
    if digits > 0
        && let Some(after) = s[digits..]
            .strip_prefix('.')
            .or_else(|| s[digits..].strip_prefix(')'))
        && (after.is_empty() || after.starts_with(char::is_whitespace))
    {
        return after;
    }
    s
}

/// Strip one pair of wrapping straight or curly quotes.
fn strip_wrapping_quotes(s: &str) -> &str {
    const PAIRS: [(char, char); 4] = [('"', '"'), ('\'', '\''), ('“', '”'), ('‘', '’')];
    for (open, close) in PAIRS {
        if let Some(inner) = s.strip_prefix(open).and_then(|t| t.strip_suffix(close)) {
            return inner;
        }
    }
    s
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn system_prompt(max: usize) -> String {
    format!(
        "You suggest follow-up prompts for a chat assistant. Given the user's last message \
         and the assistant's answer, write at most {max} short prompts that the USER could \
         send next. Write in the language the user used. Put one prompt per line, with no \
         numbering, no bullets, no quotes and no commentary. Each prompt must be under \
         {MAX_SUGGESTION_CHARS} characters."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::{ChatHistoryMessage, LlmResponse, LlmResponseMeta, ToolCallResponse};
    use assistant_llm_provider::scripted::ScriptedLlmProvider;

    fn answer(text: &str) -> LlmResponse {
        LlmResponse::FinalAnswer(text.to_string(), LlmResponseMeta::default())
    }

    // -- parse_suggestions ----------------------------------------------------

    #[test]
    fn parse_strips_numbering_bullets_and_quotes() {
        let raw = "1. \"Show an example\"\n2) Explain step two\n- “Compare them”\n* 'Go deeper'";
        assert_eq!(
            parse_suggestions(raw, 10),
            vec![
                "Show an example",
                "Explain step two",
                "Compare them",
                "Go deeper"
            ]
        );
    }

    #[test]
    fn parse_drops_blank_lines() {
        assert_eq!(
            parse_suggestions("\n  \nOne\n\n\nTwo\n", 5),
            vec!["One", "Two"]
        );
    }

    #[test]
    fn parse_dedupes_case_insensitively_keeping_first() {
        assert_eq!(
            parse_suggestions("Show an example\nSHOW AN EXAMPLE\nsomething else", 5),
            vec!["Show an example", "something else"]
        );
    }

    #[test]
    fn parse_keeps_120_chars_and_drops_121() {
        let keep = "a".repeat(120);
        let drop = "b".repeat(121);
        let raw = format!("{drop}\n{keep}");
        assert_eq!(parse_suggestions(&raw, 5), vec![keep]);
    }

    #[test]
    fn parse_counts_length_in_chars_not_bytes() {
        let keep = "é".repeat(120);
        assert_eq!(parse_suggestions(&keep, 5), vec![keep.clone()]);
    }

    #[test]
    fn parse_caps_at_max_keeping_first() {
        assert_eq!(parse_suggestions("a\nb\nc\nd", 2), vec!["a", "b"]);
    }

    #[test]
    fn parse_ignores_lines_that_are_only_markers() {
        assert_eq!(
            parse_suggestions("-\n1.\n\"\"\nReal one", 5),
            vec!["Real one"]
        );
    }

    // -- generate -------------------------------------------------------------

    #[tokio::test]
    async fn generate_parses_a_final_answer() {
        let llm = ScriptedLlmProvider::new().with_canned_responses(vec![answer(
            "1. \"Show an example\"\n2. Explain step two\n- show an example",
        )]);
        let out = generate("How do I sort?", "Use sort().", 3, &llm)
            .await
            .unwrap();
        assert_eq!(out.prompts, vec!["Show an example", "Explain step two"]);
        assert_eq!(out.outcome, Outcome::Emitted);
    }

    #[tokio::test]
    async fn generate_issues_one_call_without_tools() {
        let llm = ScriptedLlmProvider::new().with_canned_responses(vec![answer("One")]);
        generate("q", "a", 3, &llm).await.unwrap();
        let calls = llm.recorded_calls();
        assert_eq!(calls.len(), 1, "exactly one LLM call");
        assert!(calls[0].tools.is_empty(), "no tools are offered");
        assert!(!calls[0].streaming, "the call is not streamed");
    }

    #[tokio::test]
    async fn generate_yields_empty_for_tool_call_response() {
        let llm = ScriptedLlmProvider::new().with_canned_responses(vec![LlmResponse::ToolCalls(
            ToolCallResponse {
                items: Vec::new(),
                meta: LlmResponseMeta::default(),
                thinking: None,
                provider_state: None,
            },
        )]);
        let out = generate("q", "a", 3, &llm).await.unwrap();
        assert!(out.prompts.is_empty());
        assert_eq!(out.outcome, Outcome::Empty);
    }

    #[tokio::test]
    async fn generate_yields_empty_for_thinking_only_response() {
        let llm = ScriptedLlmProvider::new().with_canned_responses(vec![LlmResponse::Thinking(
            "hmm".to_string(),
            LlmResponseMeta::default(),
        )]);
        let out = generate("q", "a", 3, &llm).await.unwrap();
        assert!(out.prompts.is_empty());
        assert_eq!(out.outcome, Outcome::Empty);
    }

    #[tokio::test]
    async fn generate_truncates_inputs_to_1000_chars() {
        let llm = ScriptedLlmProvider::new().with_canned_responses(vec![answer("One")]);
        // Multi-byte fixture: 3 000 chars is 6 000 bytes.
        let user = "é".repeat(3_000);
        let ans = "ü".repeat(3_000);
        generate(&user, &ans, 3, &llm).await.unwrap();

        let calls = llm.recorded_calls();
        let ChatHistoryMessage::Text { content, .. } = &calls[0].history[0] else {
            panic!("expected a text message");
        };
        assert_eq!(
            content.matches('é').count(),
            1_000,
            "user input cut to 1000 chars"
        );
        assert_eq!(
            content.matches('ü').count(),
            1_000,
            "answer cut to 1000 chars"
        );
    }

    #[tokio::test]
    async fn generate_asks_for_the_requested_count() {
        let llm = ScriptedLlmProvider::new().with_canned_responses(vec![answer("One")]);
        generate("q", "a", 4, &llm).await.unwrap();
        let calls = llm.recorded_calls();
        assert!(
            calls[0].system_prompt.contains('4'),
            "system prompt mentions the max: {}",
            calls[0].system_prompt
        );
    }

    #[tokio::test]
    async fn generate_caps_the_result_at_max() {
        let llm = ScriptedLlmProvider::new().with_canned_responses(vec![answer("a\nb\nc\nd\ne")]);
        let out = generate("q", "a", 2, &llm).await.unwrap();
        assert_eq!(out.prompts, vec!["a", "b"]);
    }

    #[test]
    fn outcome_labels_are_stable() {
        assert_eq!(Outcome::Emitted.as_str(), "emitted");
        assert_eq!(Outcome::Empty.as_str(), "empty");
        assert_eq!(Outcome::Error.as_str(), "error");
    }
}
