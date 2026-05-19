//! Coverage for pure helpers on `Orchestrator` in `dispatch.rs`:
//! `filter_tool_specs`, `make_tool_call_message`, `make_tool_result_message`.

use assistant_core::types::conversation::MessageRole;
use assistant_core::{Capabilities, HostedTool, ToolCallItem, ToolSpec, ToolSupport};
use uuid::Uuid;

use crate::orchestrator::Orchestrator;

fn make_spec(name: &str) -> ToolSpec {
    ToolSpec {
        name: name.to_string(),
        description: format!("{name} description"),
        params_schema: serde_json::json!({}),
        is_mutating: false,
        requires_confirmation: false,
    }
}

fn caps(hosted: Vec<HostedTool>) -> Capabilities {
    Capabilities {
        tools: ToolSupport::Native,
        streaming: false,
        vision: false,
        hosted_tools: hosted,
        context_window_tokens: None,
    }
}

#[test]
fn filter_tool_specs_passes_specs_through_when_no_hosted_tools() {
    let specs = vec![
        make_spec("web-search"),
        make_spec("web-fetch"),
        make_spec("bash"),
    ];
    let filtered = Orchestrator::filter_tool_specs(specs.clone(), &caps(vec![]));
    assert_eq!(filtered.len(), 3);
}

#[test]
fn filter_tool_specs_suppresses_web_search_when_provider_has_native_web_search() {
    let specs = vec![make_spec("web-search"), make_spec("bash")];
    let filtered = Orchestrator::filter_tool_specs(specs, &caps(vec![HostedTool::WebSearch]));
    let names: Vec<&str> = filtered.iter().map(|s| s.name.as_str()).collect();
    assert!(
        !names.contains(&"web-search"),
        "web-search must be suppressed"
    );
    assert!(names.contains(&"bash"), "bash must survive");
}

#[test]
fn filter_tool_specs_suppresses_web_fetch_when_provider_has_native_web_fetch() {
    let specs = vec![make_spec("web-fetch"), make_spec("bash")];
    let filtered = Orchestrator::filter_tool_specs(specs, &caps(vec![HostedTool::WebFetch]));
    let names: Vec<&str> = filtered.iter().map(|s| s.name.as_str()).collect();
    assert!(!names.contains(&"web-fetch"));
    assert!(names.contains(&"bash"));
}

#[test]
fn filter_tool_specs_suppresses_both_when_both_hosted_tools_present() {
    let specs = vec![
        make_spec("web-search"),
        make_spec("web-fetch"),
        make_spec("bash"),
    ];
    let filtered = Orchestrator::filter_tool_specs(
        specs,
        &caps(vec![HostedTool::WebSearch, HostedTool::WebFetch]),
    );
    let names: Vec<&str> = filtered.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["bash"]);
}

#[test]
fn filter_tool_specs_keeps_other_tools_with_similar_names_intact() {
    // Hosted WebSearch suppresses only the exact `web-search` builtin —
    // unrelated tools with "search" in their name must pass through.
    let specs = vec![
        make_spec("search-conversations"),
        make_spec("web-search"),
        make_spec("internal-search"),
    ];
    let filtered = Orchestrator::filter_tool_specs(specs, &caps(vec![HostedTool::WebSearch]));
    let names: Vec<&str> = filtered.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["search-conversations", "internal-search"]);
}

// ── make_tool_call_message ────────────────────────────────────────────────

#[test]
fn make_tool_call_message_serialises_items_into_tool_calls_json() {
    let conv = Uuid::new_v4();
    let items = vec![
        ToolCallItem {
            name: "bash".to_string(),
            params: serde_json::json!({"cmd": "ls"}),
            id: Some("call_1".to_string()),
        },
        ToolCallItem {
            name: "web-fetch".to_string(),
            params: serde_json::json!({"url": "https://example.com"}),
            id: Some("call_2".to_string()),
        },
    ];
    let msg = Orchestrator::make_tool_call_message(conv, 7, &items);
    assert_eq!(msg.conversation_id, conv);
    assert_eq!(msg.turn, 7);
    assert!(matches!(msg.role, MessageRole::Assistant));
    let json = msg.tool_calls_json.expect("tool_calls_json must be set");
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"], "bash");
    assert_eq!(arr[1]["params"]["url"], "https://example.com");
}

#[test]
fn make_tool_call_message_with_empty_items_emits_empty_json_array() {
    let msg = Orchestrator::make_tool_call_message(Uuid::new_v4(), 0, &[]);
    let json = msg.tool_calls_json.expect("tool_calls_json must be set");
    assert_eq!(json, "[]");
}

// ── make_tool_result_message ──────────────────────────────────────────────

#[test]
fn make_tool_result_message_records_tool_name_and_content() {
    let conv = Uuid::new_v4();
    let msg = Orchestrator::make_tool_result_message(conv, 9, "bash", "stdout: ok");
    assert_eq!(msg.conversation_id, conv);
    assert_eq!(msg.turn, 9);
    assert!(matches!(msg.role, MessageRole::Tool));
    assert_eq!(msg.content, "stdout: ok");
    assert_eq!(msg.skill_name.as_deref(), Some("bash"));
}

#[test]
fn make_tool_result_message_handles_empty_observation() {
    let msg = Orchestrator::make_tool_result_message(Uuid::new_v4(), 0, "noop", "");
    assert_eq!(msg.content, "");
    assert_eq!(msg.skill_name.as_deref(), Some("noop"));
}
