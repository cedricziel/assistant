//! Coverage for pure helpers on `Orchestrator` in `dispatch.rs`:
//! `filter_tool_specs`, `make_tool_call_message`, `make_tool_result_message`.

use assistant_core::types::conversation::{Interface, MessageRole};
use assistant_core::{
    Capabilities, ChatHistoryMessage, HostedTool, ProviderState, ToolCallItem, ToolSpec,
    ToolSupport,
};
use assistant_storage::InMemoryConversationStore;
use assistant_storage::conversations::ConversationStore;
use uuid::Uuid;

use crate::history::TurnHistory;
use crate::orchestrator::Orchestrator;
use crate::orchestrator::dispatch::ToolResultMeta;

fn make_spec(name: &str) -> ToolSpec {
    ToolSpec {
        name: name.to_string(),
        description: format!("{name} description"),
        params_schema: serde_json::json!({}),
        is_mutating: false,
        requires_confirmation: false,
        requires_rich_client: false,
    }
}

fn rich_spec(name: &str) -> ToolSpec {
    ToolSpec {
        requires_rich_client: true,
        ..make_spec(name)
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

fn kept(interface: Interface, interactive: bool, depth: u32) -> bool {
    let rich_client = Orchestrator::rich_client_turn(&interface, interactive, depth);
    let filtered = Orchestrator::filter_tool_specs(
        vec![rich_spec("render-surface"), make_spec("bash")],
        &caps(vec![]),
        rich_client,
    );
    assert!(
        filtered.iter().any(|s| s.name == "bash"),
        "ordinary tools always survive"
    );
    filtered.iter().any(|s| s.name == "render-surface")
}

#[test]
fn rich_client_tools_are_kept_for_an_interactive_web_turn_at_depth_zero() {
    assert!(kept(Interface::Web, true, 0));
}

#[test]
fn rich_client_tools_are_dropped_for_slack() {
    assert!(!kept(Interface::Slack, true, 0));
}

#[test]
fn rich_client_tools_are_dropped_for_a_non_interactive_web_turn() {
    // quick-message and voice are Interface::Web but cannot show a surface.
    assert!(!kept(Interface::Web, false, 0));
}

#[test]
fn rich_client_tools_are_dropped_for_an_interactive_cli_turn() {
    assert!(!kept(Interface::Cli, true, 0));
}

#[test]
fn rich_client_tools_are_dropped_for_web_subagents() {
    assert!(!kept(Interface::Web, true, 1));
}

#[test]
fn rich_client_tools_are_dropped_for_the_scheduler() {
    assert!(!kept(Interface::Scheduler, true, 0));
    assert!(!kept(Interface::Scheduler, false, 0));
}

#[test]
fn filter_tool_specs_passes_specs_through_when_no_hosted_tools() {
    let specs = vec![
        make_spec("web-search"),
        make_spec("web-fetch"),
        make_spec("bash"),
    ];
    let filtered = Orchestrator::filter_tool_specs(specs.clone(), &caps(vec![]), true);
    assert_eq!(filtered.len(), 3);
}

#[test]
fn filter_tool_specs_suppresses_web_search_when_provider_has_native_web_search() {
    let specs = vec![make_spec("web-search"), make_spec("bash")];
    let filtered = Orchestrator::filter_tool_specs(specs, &caps(vec![HostedTool::WebSearch]), true);
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
    let filtered = Orchestrator::filter_tool_specs(specs, &caps(vec![HostedTool::WebFetch]), true);
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
        true,
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
    let filtered = Orchestrator::filter_tool_specs(specs, &caps(vec![HostedTool::WebSearch]), true);
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

// ── persist_tool_calls ────────────────────────────────────────────────────

#[tokio::test]
async fn persist_tool_calls_keeps_provider_state_in_history_and_storage() {
    let store = InMemoryConversationStore::new();
    let conv = store.create_conversation(None).await.unwrap();
    let items = vec![ToolCallItem {
        name: "bash".to_string(),
        params: serde_json::json!({"command": "ls"}),
        id: Some("c1".to_string()),
    }];
    let state = ProviderState {
        provider: "openrouter".to_string(),
        model: "anthropic/claude-sonnet-5.5".to_string(),
        data: serde_json::json!({"reasoning_details": [{"type": "reasoning.encrypted", "data": "x"}]}),
    };
    let mut history = TurnHistory::default();

    Orchestrator::persist_tool_calls(&mut history, &store, conv.id, 1, &items, Some(&state)).await;

    assert!(
        matches!(
            &history[0],
            ChatHistoryMessage::AssistantToolCalls { provider_state: Some(s), .. } if *s == state
        ),
        "the in-memory history carries the state for the next call: {history:?}"
    );
    let saved = store.load_history(conv.id).await.unwrap();
    let stored: ProviderState =
        serde_json::from_str(saved[0].provider_state_json.as_deref().unwrap()).unwrap();
    assert_eq!(stored, state, "the state is persisted for history rebuilds");
    assert_eq!(
        history.row_ids(),
        vec![Some(saved[0].id)],
        "the history entry maps to the row that was saved"
    );
}

#[tokio::test]
async fn persist_tool_calls_without_provider_state_stores_none() {
    let store = InMemoryConversationStore::new();
    let conv = store.create_conversation(None).await.unwrap();
    let mut history = TurnHistory::default();
    Orchestrator::persist_tool_calls(&mut history, &store, conv.id, 1, &[], None).await;
    let saved = store.load_history(conv.id).await.unwrap();
    assert!(saved[0].provider_state_json.is_none());
}

// ── make_tool_result_message ──────────────────────────────────────────────

#[test]
fn make_tool_result_message_records_tool_name_and_content() {
    let conv = Uuid::new_v4();
    let msg = Orchestrator::make_tool_result_message(
        conv,
        9,
        "bash",
        "stdout: ok",
        ToolResultMeta {
            tool_call_id: Some("call_9"),
            status: "ok",
            data: Some(&serde_json::json!({"kind": "terminal"})),
        },
    );
    assert_eq!(msg.conversation_id, conv);
    assert_eq!(msg.turn, 9);
    assert!(matches!(msg.role, MessageRole::Tool));
    assert_eq!(msg.content, "stdout: ok");
    assert_eq!(msg.skill_name.as_deref(), Some("bash"));
    assert_eq!(msg.tool_call_id.as_deref(), Some("call_9"));
    assert_eq!(msg.tool_status.as_deref(), Some("ok"));
    assert_eq!(
        msg.tool_data_json.as_deref(),
        Some(r#"{"kind":"terminal"}"#)
    );
}

#[test]
fn make_tool_result_message_handles_empty_observation() {
    let msg =
        Orchestrator::make_tool_result_message(Uuid::new_v4(), 0, "noop", "", ToolResultMeta::ok());
    assert_eq!(msg.content, "");
    assert_eq!(msg.skill_name.as_deref(), Some("noop"));
}
