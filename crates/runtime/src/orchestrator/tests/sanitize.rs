//! Tests for `sanitize_history`: enforces alternation invariants
//! (no trailing user message without an assistant reply, no orphaned
//! tool-call results, etc.) before sending history to the LLM.

use assistant_core::{ChatHistoryMessage, ChatRole, ContentBlock, ToolCallItem};

// ── sanitize_history tests ────────────────────────────────────────────────

#[test]
fn sanitize_history_empty_is_noop() {
    let mut history = vec![];
    crate::history::sanitize_history(&mut history);
    assert!(history.is_empty());
}

#[test]
fn sanitize_history_valid_alternation_is_noop() {
    let mut history = vec![
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "hello".into(),
        },
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            content: "hi".into(),
        },
    ];
    crate::history::sanitize_history(&mut history);
    assert_eq!(history.len(), 2, "valid alternation should not be modified");
}

#[test]
fn sanitize_history_trailing_user_inserts_assistant() {
    let mut history = vec![ChatHistoryMessage::Text {
        role: ChatRole::User,
        content: "orphaned".into(),
    }];
    crate::history::sanitize_history(&mut history);
    assert_eq!(
        history.len(),
        2,
        "should insert a synthetic assistant message"
    );
    match &history[1] {
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            content,
        } => {
            assert!(
                content.contains("error"),
                "synthetic message should mention error"
            );
        }
        other => panic!("expected Text(Assistant), got {:?}", other),
    }
}

#[test]
fn sanitize_history_trailing_multimodal_user_inserts_assistant() {
    let mut history = vec![ChatHistoryMessage::MultimodalUser {
        content: vec![ContentBlock::Text("image msg".into())],
    }];
    crate::history::sanitize_history(&mut history);
    assert_eq!(history.len(), 2);
    assert!(matches!(
        &history[1],
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            ..
        }
    ));
}

#[test]
fn sanitize_history_orphaned_tool_calls_get_synthetic_results() {
    let mut history = vec![
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "do stuff".into(),
        },
        ChatHistoryMessage::AssistantToolCalls(vec![
            ToolCallItem {
                name: "tool-a".into(),
                params: serde_json::json!({}),
                id: None,
            },
            ToolCallItem {
                name: "tool-b".into(),
                params: serde_json::json!({}),
                id: None,
            },
        ]),
        // Only one ToolResult — tool-b is missing.
        ChatHistoryMessage::ToolResult {
            name: "tool-a".into(),
            content: "ok".into(),
        },
    ];
    crate::history::sanitize_history(&mut history);
    // Should have: User, AssistantToolCalls, ToolResult(a), ToolResult(b-synthetic)
    assert_eq!(history.len(), 4, "missing tool result should be inserted");
    match &history[3] {
        ChatHistoryMessage::ToolResult { name, content } => {
            assert_eq!(name, "tool-b");
            assert!(
                content.contains("lost") || content.contains("crash") || content.contains("error"),
                "synthetic result should indicate failure: {content}"
            );
        }
        other => panic!("expected ToolResult, got {:?}", other),
    }
}

#[test]
fn sanitize_history_fully_orphaned_tool_calls_all_results_inserted() {
    let mut history = vec![
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "run tools".into(),
        },
        ChatHistoryMessage::AssistantToolCalls(vec![
            ToolCallItem {
                name: "alpha".into(),
                params: serde_json::json!({}),
                id: None,
            },
            ToolCallItem {
                name: "beta".into(),
                params: serde_json::json!({}),
                id: None,
            },
        ]),
        // No ToolResult at all — process crashed right after persisting tool calls.
    ];
    crate::history::sanitize_history(&mut history);
    // Should have: User, AssistantToolCalls, ToolResult(alpha), ToolResult(beta)
    assert_eq!(
        history.len(),
        4,
        "both missing tool results should be inserted"
    );
    assert!(matches!(&history[2], ChatHistoryMessage::ToolResult { name, .. } if name == "alpha"));
    assert!(matches!(&history[3], ChatHistoryMessage::ToolResult { name, .. } if name == "beta"));
}

#[test]
fn sanitize_history_combined_orphaned_tools_and_trailing_user() {
    // Simulates: process crashed during tool execution on turn 1,
    // then on turn 2 the user message was persisted but LLM failed.
    let mut history = vec![
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "turn 1".into(),
        },
        ChatHistoryMessage::AssistantToolCalls(vec![ToolCallItem {
            name: "my-tool".into(),
            params: serde_json::json!({}),
            id: None,
        }]),
        // Missing ToolResult, then orphaned user from turn 2:
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "turn 2".into(),
        },
    ];
    crate::history::sanitize_history(&mut history);
    // Should have: User, AssistantToolCalls, ToolResult(synthetic), User, Assistant(synthetic)
    assert_eq!(history.len(), 5);
    assert!(
        matches!(&history[2], ChatHistoryMessage::ToolResult { name, .. } if name == "my-tool")
    );
    assert!(matches!(
        &history[4],
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            ..
        }
    ));
}

#[test]
fn sanitize_history_orphaned_tool_result_dropped() {
    // Simulates: a system-injected tool result (e.g. skill-learner) appears
    // at the start of history with no preceding AssistantToolCalls.
    let mut history = vec![
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "hello".into(),
        },
        ChatHistoryMessage::ToolResult {
            name: "skill-learner".into(),
            content: "Auto-created skill 'foo'".into(),
        },
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            content: "hi".into(),
        },
    ];
    crate::history::sanitize_history(&mut history);
    // The orphaned ToolResult should be dropped
    assert_eq!(history.len(), 2);
    assert!(matches!(
        &history[0],
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            ..
        }
    ));
    assert!(matches!(
        &history[1],
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            ..
        }
    ));
}

#[test]
fn sanitize_history_tool_result_after_matched_calls_dropped() {
    // Extra ToolResult beyond what the tool calls declared should be dropped.
    let mut history = vec![
        ChatHistoryMessage::AssistantToolCalls(vec![ToolCallItem {
            name: "my-tool".into(),
            params: serde_json::json!({}),
            id: Some("call_1".into()),
        }]),
        ChatHistoryMessage::ToolResult {
            name: "my-tool".into(),
            content: "result".into(),
        },
        // Spurious extra result with no matching call
        ChatHistoryMessage::ToolResult {
            name: "skill-learner".into(),
            content: "injected".into(),
        },
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            content: "done".into(),
        },
    ];
    crate::history::sanitize_history(&mut history);
    // The extra ToolResult should be dropped
    assert_eq!(history.len(), 3);
    assert!(matches!(
        &history[0],
        ChatHistoryMessage::AssistantToolCalls(_)
    ));
    assert!(matches!(
        &history[1],
        ChatHistoryMessage::ToolResult { name, .. } if name == "my-tool"
    ));
    assert!(matches!(
        &history[2],
        ChatHistoryMessage::Text {
            role: ChatRole::Assistant,
            ..
        }
    ));
}
