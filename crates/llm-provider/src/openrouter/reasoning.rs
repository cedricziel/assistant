//! What OpenRouter reasoning may be shown, when it ate the budget, and how
//! its details are kept across tool calls.

use serde_json::{Value, json};

use assistant_core::{ChatHistoryMessage, ProviderState};

/// [`ProviderState::provider`] for state this provider writes.
const PROVIDER: &str = "openrouter";

/// Folds a streamed `reasoning_details` entry into `details`. Entries with
/// the same `index` are pieces of one entry: their `text`, `summary` and
/// `data` are appended, and other fields, such as `signature`, are set.
pub fn merge_detail(details: &mut Vec<Value>, delta: Value) {
    let index = delta.get("index").cloned();
    let existing = index
        .as_ref()
        .and_then(|i| details.iter_mut().rev().find(|d| d.get("index") == Some(i)));
    let (Some(existing), Value::Object(fields)) = (existing, &delta) else {
        details.push(delta);
        return;
    };
    for (key, value) in fields {
        let appended = match (existing.get(key.as_str()), value) {
            (Some(Value::String(old)), Value::String(new))
                if matches!(key.as_str(), "text" | "summary" | "data") =>
            {
                Some(format!("{old}{new}"))
            }
            _ => None,
        };
        existing[key.as_str()] = appended.map_or_else(|| value.clone(), Value::String);
    }
}

/// The state that carries `details` to later requests for `model`.
pub fn provider_state(model: &str, details: Vec<Value>) -> Option<ProviderState> {
    (!details.is_empty()).then(|| ProviderState {
        provider: PROVIDER.to_owned(),
        model: model.to_owned(),
        data: json!({ "reasoning_details": details }),
    })
}

/// Adds stored `reasoning_details` to each assistant tool-call message in
/// `messages`, unchanged and in order, when this provider wrote them for
/// `model`. `messages` holds one such message per
/// [`ChatHistoryMessage::AssistantToolCalls`] in `history`, in the same order.
pub fn attach_details(messages: &mut [Value], history: &[ChatHistoryMessage], model: &str) {
    let states = history.iter().filter_map(|m| match m {
        ChatHistoryMessage::AssistantToolCalls { provider_state, .. } => Some(provider_state),
        _ => None,
    });
    let calls = messages
        .iter_mut()
        .filter(|m| m["role"] == "assistant" && m.get("tool_calls").is_some());
    for (message, state) in calls.zip(states) {
        let Some(state) = state else { continue };
        if state.provider != PROVIDER || state.model != model {
            continue;
        }
        if let Some(details) = state.data.get("reasoning_details") {
            message["reasoning_details"] = details.clone();
        }
    }
}

/// The displayable text of `details`: `reasoning.text` and
/// `reasoning.summary` entries in order. Encrypted entries are skipped.
/// Without details, the plain `reasoning` string is used, since OpenRouter
/// often sends the same text in both.
pub fn visible_text(details: &[Value], plain: Option<&str>) -> String {
    if details.is_empty() {
        return plain.unwrap_or_default().to_owned();
    }
    details
        .iter()
        .filter_map(|d| match d.get("type").and_then(Value::as_str) {
            Some("reasoning.text") => d.get("text").and_then(Value::as_str),
            Some("reasoning.summary") => d.get("summary").and_then(Value::as_str),
            _ => None,
        })
        .collect()
}

/// An error when the model stopped at the token limit without an answer or
/// a tool call.
pub fn budget_exhausted(
    finish_reason: Option<&str>,
    answer: &str,
    has_tool_calls: bool,
    reasoned: bool,
) -> Option<anyhow::Error> {
    if finish_reason != Some("length") || has_tool_calls || !answer.trim().is_empty() {
        return None;
    }
    Some(if reasoned {
        anyhow::anyhow!(
            "OpenRouter stopped at the token budget before answering: the budget was used up \
             by reasoning. Raise [llm.openrouter] max_tokens or lower the reasoning effort."
        )
    } else {
        anyhow::anyhow!(
            "OpenRouter stopped at the token budget before answering. \
             Raise [llm.openrouter] max_tokens."
        )
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{merge_detail, visible_text};

    #[test]
    fn entries_without_index_are_kept_separate() {
        let mut details = Vec::new();
        merge_detail(&mut details, json!({"type": "reasoning.text", "text": "a"}));
        merge_detail(&mut details, json!({"type": "reasoning.text", "text": "b"}));
        assert_eq!(details.len(), 2);
    }

    #[test]
    fn plain_reasoning_is_used_only_without_details() {
        assert_eq!(visible_text(&[], Some("plain")), "plain");
        let details = [json!({"type": "reasoning.text", "text": "detailed"})];
        assert_eq!(visible_text(&details, Some("plain")), "detailed");
    }
}
