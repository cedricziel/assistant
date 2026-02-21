use regex::Regex;
use std::sync::OnceLock;

/// A single parsed step from the model's ReAct-format text output.
#[derive(Debug, Clone, PartialEq)]
pub enum ReActStep {
    /// The model wants to call a skill.
    ToolCall {
        name: String,
        params: serde_json::Value,
    },
    /// The model has a final answer for the user.
    Answer(String),
    /// An intermediate thought (not yet acted upon).
    Thought(String),
}

// ── Compiled patterns ─────────────────────────────────────────────────────────
// The Rust `regex` crate does not support lookahead/lookbehind assertions, so
// we match each marker with a simple per-line pattern.

/// Matches `THOUGHT: <content>` (case-insensitive) on a single line.
fn thought_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^THOUGHT:\s*(.+)$").unwrap())
}

/// Matches `ACTION: <json>` (case-insensitive) — the JSON must start with `{`.
fn action_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^ACTION:\s*(\{.+\})\s*$").unwrap())
}

/// Matches `ANSWER: <content>` (case-insensitive) on a single line.
fn answer_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^ANSWER:\s*(.+)$").unwrap())
}

/// Detects whether a block of text contains any ReAct marker at the start of
/// any line (used for auto-detection of the mode).
fn any_marker_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?im)^(THOUGHT|ACTION|ANSWER):").unwrap())
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parser for the ReAct text protocol.
///
/// The model is expected to produce text in one of:
///
/// ```text
/// THOUGHT: <reasoning>
/// ACTION: {"name": "skill-name", "params": {"key": "value"}}
/// ```
///
/// or:
///
/// ```text
/// ANSWER: <final response to user>
/// ```
pub struct ReActParser;

impl ReActParser {
    /// Parse a model text output into a [`ReActStep`].
    ///
    /// The parser scans lines in order and applies this priority:
    /// 1. If any line matches `ACTION:` with valid JSON, return [`ReActStep::ToolCall`].
    /// 2. If any line matches `ANSWER:`, return [`ReActStep::Answer`].
    /// 3. If any line matches `THOUGHT:`, return [`ReActStep::Thought`].
    /// 4. Fall back to treating the entire text as a final answer.
    pub fn parse(text: &str) -> ReActStep {
        let mut first_thought: Option<String> = None;
        let mut first_answer: Option<String> = None;

        for line in text.lines() {
            let trimmed = line.trim();

            // 1. Check for ACTION: (highest priority)
            if let Some(caps) = action_line_re().captures(trimmed) {
                let json_str = caps[1].trim();
                match serde_json::from_str::<serde_json::Value>(json_str) {
                    Ok(value) => {
                        let name = value
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string();
                        let params = value
                            .get("params")
                            .cloned()
                            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                        if !name.is_empty() {
                            tracing::debug!(skill = %name, "ReAct parsed ACTION");
                            return ReActStep::ToolCall { name, params };
                        }

                        tracing::warn!(
                            raw = json_str,
                            "ReAct ACTION JSON is missing the 'name' field"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(%err, raw = json_str, "ReAct ACTION JSON parse failed");
                    }
                }
            }

            // 2. Collect first ANSWER:
            if first_answer.is_none() {
                if let Some(caps) = answer_line_re().captures(trimmed) {
                    first_answer = Some(caps[1].trim().to_string());
                }
            }

            // 3. Collect first THOUGHT:
            if first_thought.is_none() {
                if let Some(caps) = thought_line_re().captures(trimmed) {
                    first_thought = Some(caps[1].trim().to_string());
                }
            }
        }

        if let Some(answer) = first_answer {
            tracing::debug!("ReAct parsed ANSWER");
            return ReActStep::Answer(answer);
        }

        if let Some(thought) = first_thought {
            tracing::debug!("ReAct parsed THOUGHT");
            return ReActStep::Thought(thought);
        }

        // Fallback: treat the entire response as a final answer.
        tracing::warn!("ReAct could not find known markers; treating full text as ANSWER");
        ReActStep::Answer(text.trim().to_string())
    }

    /// Return `true` if the text contains any ReAct-style markers at the
    /// start of a line (case-insensitive).
    pub fn looks_like_react(text: &str) -> bool {
        any_marker_re().is_match(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── ACTION parsing ────────────────────────────────────────────────────────

    #[test]
    fn parses_tool_call_with_params() {
        let text = r#"
THOUGHT: I need to fetch some data.
ACTION: {"name": "web-fetch", "params": {"url": "https://example.com"}}
"#;
        let step = ReActParser::parse(text);
        assert_eq!(
            step,
            ReActStep::ToolCall {
                name: "web-fetch".to_string(),
                params: json!({"url": "https://example.com"}),
            }
        );
    }

    #[test]
    fn parses_tool_call_with_empty_params() {
        let text = "THOUGHT: Check memory.\nACTION: {\"name\": \"memory-read\", \"params\": {}}";
        let step = ReActParser::parse(text);
        assert_eq!(
            step,
            ReActStep::ToolCall {
                name: "memory-read".to_string(),
                params: json!({}),
            }
        );
    }

    #[test]
    fn parses_tool_call_without_params_key() {
        // When "params" is absent, defaults to empty object.
        let text = r#"ACTION: {"name": "shell-exec"}"#;
        let step = ReActParser::parse(text);
        assert_eq!(
            step,
            ReActStep::ToolCall {
                name: "shell-exec".to_string(),
                params: json!({}),
            }
        );
    }

    #[test]
    fn falls_back_on_invalid_action_json() {
        // Malformed JSON after ACTION: → falls through to ANSWER.
        let text = "ACTION: {not valid json}\nANSWER: I could not proceed.";
        let step = ReActParser::parse(text);
        assert_eq!(step, ReActStep::Answer("I could not proceed.".to_string()));
    }

    #[test]
    fn falls_back_on_action_missing_name() {
        // Valid JSON but no "name" key → falls through to ANSWER.
        let text = "ACTION: {\"tool\": \"web-fetch\"}\nANSWER: Sorry.";
        let step = ReActParser::parse(text);
        assert_eq!(step, ReActStep::Answer("Sorry.".to_string()));
    }

    // ── ANSWER parsing ────────────────────────────────────────────────────────

    #[test]
    fn parses_answer() {
        let text = "ANSWER: The capital of France is Paris.";
        let step = ReActParser::parse(text);
        assert_eq!(
            step,
            ReActStep::Answer("The capital of France is Paris.".to_string())
        );
    }

    #[test]
    fn answer_trims_whitespace() {
        let text = "ANSWER:   Hello world.   ";
        let step = ReActParser::parse(text);
        assert_eq!(step, ReActStep::Answer("Hello world.".to_string()));
    }

    // ── THOUGHT parsing ───────────────────────────────────────────────────────

    #[test]
    fn parses_thought_only() {
        let text = "THOUGHT: I should think more before acting.";
        let step = ReActParser::parse(text);
        assert_eq!(
            step,
            ReActStep::Thought("I should think more before acting.".to_string())
        );
    }

    // ── Fallback ──────────────────────────────────────────────────────────────

    #[test]
    fn fallback_plain_text_is_answer() {
        let text = "Here is some plain response without any markers.";
        let step = ReActParser::parse(text);
        assert_eq!(
            step,
            ReActStep::Answer("Here is some plain response without any markers.".to_string())
        );
    }

    // ── looks_like_react ──────────────────────────────────────────────────────

    #[test]
    fn detects_react_markers() {
        assert!(ReActParser::looks_like_react("THOUGHT: something"));
        assert!(ReActParser::looks_like_react("ACTION: {\"name\":\"x\"}"));
        assert!(ReActParser::looks_like_react("ANSWER: done"));
        assert!(!ReActParser::looks_like_react("Hello, how are you?"));
    }

    // ── Case insensitivity ────────────────────────────────────────────────────

    #[test]
    fn parses_lowercase_answer() {
        let text = "answer: The sky is blue.";
        let step = ReActParser::parse(text);
        assert_eq!(step, ReActStep::Answer("The sky is blue.".to_string()));
    }

    #[test]
    fn parses_lowercase_action() {
        let text = r#"action: {"name": "memory-search", "params": {"query": "test"}}"#;
        let step = ReActParser::parse(text);
        assert_eq!(
            step,
            ReActStep::ToolCall {
                name: "memory-search".to_string(),
                params: json!({"query": "test"}),
            }
        );
    }

    // ── Priority order ────────────────────────────────────────────────────────

    #[test]
    fn action_takes_priority_over_answer() {
        // Even if ANSWER appears before ACTION in the text, ACTION wins.
        let text = "ANSWER: foo\nACTION: {\"name\": \"shell-exec\", \"params\": {\"cmd\": \"ls\"}}";
        let step = ReActParser::parse(text);
        assert_eq!(
            step,
            ReActStep::ToolCall {
                name: "shell-exec".to_string(),
                params: json!({"cmd": "ls"}),
            }
        );
    }
}
