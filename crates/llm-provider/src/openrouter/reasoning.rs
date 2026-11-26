//! What OpenRouter reasoning may be shown, and when it ate the budget.

use serde_json::Value;

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

    use super::visible_text;

    #[test]
    fn plain_reasoning_is_used_only_without_details() {
        assert_eq!(visible_text(&[], Some("plain")), "plain");
        let details = [json!({"type": "reasoning.text", "text": "detailed"})];
        assert_eq!(visible_text(&details, Some("plain")), "detailed");
    }
}
