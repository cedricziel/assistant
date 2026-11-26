//! A conversation summary on the active path replaces the messages it covers
//! in every history the model sees: the turn request, `/status` and
//! `/compact`.

use std::sync::Arc;

use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{Interface, Message, TurnIdentity};
use assistant_core::{ChatHistoryMessage, LlmProvider, LlmResponse, LlmResponseMeta};
use assistant_llm_provider::scripted::ScriptedLlmProvider;
use assistant_storage::{ConversationStore, ConversationSummaryStore, StorageLayer};
use uuid::Uuid;
use wiremock::MockServer;

use super::{Orchestrator, allow_all_tools, build, build_with_llm, messages_in, mount_answer};
use crate::{CommandContext, CommandRegistry};

const SUMMARY_MARK: &str = "SUMMARY-MARK";

/// Four stored messages u1, a1, u2, a2 (each padded so token estimates are
/// large) and a summary through a1.
async fn conversation_with_summary(storage: &StorageLayer) -> Uuid {
    let store = storage.conversation_store();
    let conv = store.create_conversation(None).await.unwrap();
    let pad = "x".repeat(4_000);
    let mut ids = Vec::new();
    for (i, text) in ["u1", "a1", "u2", "a2"].into_iter().enumerate() {
        let content = format!("{text} {pad}");
        let msg = if i % 2 == 0 {
            Message::user(conv.id, content)
        } else {
            Message::assistant(conv.id, content)
        };
        ids.push(msg.id);
        store.save_message(&msg).await.unwrap();
    }
    storage
        .conversation_summary_store()
        .insert(conv.id, ids[1], SUMMARY_MARK)
        .await
        .unwrap();
    conv.id
}

fn command_context(orch: &Arc<Orchestrator>, conversation_id: Uuid) -> CommandContext {
    CommandContext {
        conversation_id,
        conversation_configs: Arc::new(Default::default()) as Arc<_>,
        orchestrator: orch.clone(),
        evict_conversation: None,
        default_model: "test".to_string(),
    }
}

fn estimated_tokens(ack: &str) -> u64 {
    ack.lines()
        .find_map(|l| l.strip_prefix("Estimated tokens: ~"))
        .and_then(|n| n.parse().ok())
        .expect("status reports a token estimate")
}

#[tokio::test]
async fn prepare_history_applies_the_summary_and_keeps_base_turn_raw() {
    let server = MockServer::start().await;
    mount_answer(&server, "ok").await;
    let (orch, storage) = build(&server.uri()).await;
    let conv = conversation_with_summary(&storage).await;

    let (_, history, base_turn) = orch
        .prepare_history(
            "u3",
            conv,
            Vec::new(),
            &[],
            &orch.agent_id,
            &TurnIdentity::default(),
        )
        .await
        .unwrap();

    assert_eq!(
        base_turn, 4,
        "base_turn counts the raw path, not the summary"
    );
    let texts: Vec<String> = history
        .messages()
        .iter()
        .map(|m| match m {
            ChatHistoryMessage::Text { content, .. } => content.chars().take(12).collect(),
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert!(
        texts[0].starts_with("[Conversati"),
        "summary first: {texts:?}"
    );
    assert_eq!(texts.len(), 4, "summary, u2, a2 and the new u3: {texts:?}");
    assert!(texts[1].starts_with("u2"));
    assert_eq!(history.row_ids().len(), history.messages().len());
}

#[tokio::test]
async fn the_llm_request_starts_with_the_summary() {
    let server = MockServer::start().await;
    mount_answer(&server, "ok").await;
    let (orch, storage) = build(&server.uri()).await;
    let conv = conversation_with_summary(&storage).await;

    orch.run_turn(
        "u3",
        conv,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let reqs = server.received_requests().await.unwrap();
    let msgs = messages_in(&reqs[0]);
    let body = serde_json::to_string(&msgs).unwrap();
    assert!(body.contains(SUMMARY_MARK), "the summary reaches the model");
    assert!(!body.contains("u1 x"), "summarised rows are not replayed");
    assert!(body.contains("u2 x"), "rows after the summary are replayed");
}

#[tokio::test]
async fn status_estimates_tokens_over_the_summary_applied_history() {
    let server = MockServer::start().await;
    let (orch, storage) = build(&server.uri()).await;
    let conv = conversation_with_summary(&storage).await;

    let with_summary = CommandRegistry::new()
        .execute("status", &[], command_context(&orch, conv))
        .await;

    // The same four messages without a summary.
    let plain = Uuid::new_v4();
    let store = storage.conversation_store();
    store
        .create_conversation_with_id(plain, None)
        .await
        .unwrap();
    for m in store.load_history(conv).await.unwrap() {
        let mut copy = Message::new(plain, m.role.clone(), m.content.clone());
        copy.turn = m.turn;
        store.save_message(&copy).await.unwrap();
    }
    let without_summary = CommandRegistry::new()
        .execute("status", &[], command_context(&orch, plain))
        .await;

    let (a, b) = (
        estimated_tokens(&with_summary.ack_text),
        estimated_tokens(&without_summary.ack_text),
    );
    assert!(
        a < b,
        "the summary replaces two padded rows, so {a} must be below {b}"
    );
}

#[tokio::test]
async fn compact_summarises_the_existing_summary_plus_the_rows_after_it() {
    let scripted =
        Arc::new(
            ScriptedLlmProvider::new().with_canned_responses(vec![LlmResponse::FinalAnswer(
                "new summary".into(),
                LlmResponseMeta::default(),
            )]),
        );
    let llm: Arc<dyn LlmProvider> = scripted.clone();
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.compaction.keep_recent_turns = 1;
    allow_all_tools(&mut config);
    let (orch, storage) = build_with_llm(llm, config, |o| o).await;
    let conv = conversation_with_summary(&storage).await;
    // Add a third exchange so a split exists after the summary.
    let store = storage.conversation_store();
    store
        .save_message(&Message::user(conv, "u3"))
        .await
        .unwrap();
    store
        .save_message(&Message::assistant(conv, "a3"))
        .await
        .unwrap();

    let result = CommandRegistry::new()
        .execute("compact", &[], command_context(&orch, conv))
        .await;

    assert_eq!(result.ack_text, "Context compacted.");
    let calls = scripted.recorded_calls();
    let transcript = match &calls[0].history[0] {
        ChatHistoryMessage::Text { content, .. } => content.clone(),
        other => panic!("unexpected {other:?}"),
    };
    assert!(
        transcript.contains(SUMMARY_MARK),
        "existing summary is the input"
    );
    assert!(
        transcript.contains("u2 x"),
        "rows after the summary are input"
    );
    assert!(
        !transcript.contains("u1 x"),
        "summarised rows are not re-read"
    );
}

/// `/compact` over a conversation of three exchanges, with a scripted model
/// that answers the summary request and then one ordinary turn.
async fn compacted_conversation() -> (
    Arc<Orchestrator>,
    Arc<StorageLayer>,
    Arc<ScriptedLlmProvider>,
    Uuid,
    Vec<Uuid>,
) {
    let scripted = Arc::new(ScriptedLlmProvider::new().with_canned_responses(vec![
        LlmResponse::FinalAnswer("fresh summary".into(), LlmResponseMeta::default()),
        LlmResponse::FinalAnswer("ok".into(), LlmResponseMeta::default()),
    ]));
    let llm: Arc<dyn LlmProvider> = scripted.clone();
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.compaction.keep_recent_turns = 1;
    allow_all_tools(&mut config);
    let (orch, storage) = build_with_llm(llm, config, |o| o).await;

    let store = storage.conversation_store();
    let conv = store.create_conversation(None).await.unwrap();
    for (i, text) in ["u1", "a1", "u2", "a2", "u3", "a3"].into_iter().enumerate() {
        let msg = if i % 2 == 0 {
            Message::user(conv.id, text)
        } else {
            Message::assistant(conv.id, text)
        };
        store.save_message(&msg).await.unwrap();
    }
    let ids: Vec<Uuid> = store
        .load_history(conv.id)
        .await
        .unwrap()
        .iter()
        .map(|m| m.id)
        .collect();

    let result = CommandRegistry::new()
        .execute("compact", &[], command_context(&orch, conv.id))
        .await;
    assert_eq!(result.ack_text, "Context compacted.");
    (orch, storage, scripted, conv.id, ids)
}

#[tokio::test]
async fn compact_keeps_every_row_and_the_next_turn_starts_with_the_summary() {
    let (orch, storage, scripted, conv, ids) = compacted_conversation().await;

    let stored: Vec<Uuid> = storage
        .conversation_store()
        .load_history(conv)
        .await
        .unwrap()
        .iter()
        .map(|m| m.id)
        .collect();
    assert_eq!(stored, ids, "compaction deletes no message");
    let summary = storage
        .conversation_summary_store()
        .newest_on_path(conv, &ids)
        .await
        .unwrap()
        .expect("compaction records a summary");
    assert_eq!(
        summary.through_message_id, ids[3],
        "through the last summarised row (a2)"
    );

    orch.run_turn(
        "u4",
        conv,
        Interface::Cli,
        None,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let calls = scripted.recorded_calls();
    let texts: Vec<String> = calls[1]
        .history
        .iter()
        .map(|m| match m {
            ChatHistoryMessage::Text { content, .. } => content.clone(),
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert!(
        texts[0].contains("fresh summary"),
        "summary first: {texts:?}"
    );
    assert_eq!(texts[1..], ["u3", "a3", "u4"], "post-summary path follows");
}

#[tokio::test]
async fn a_branch_created_before_the_summarised_range_gets_no_summary() {
    let (orch, storage, _scripted, conv, ids) = compacted_conversation().await;
    let store = storage.conversation_store();
    // Edit u2: a sibling under a1 diverges from the summarised range.
    let edited = Message::user(conv, "u2 edited");
    store.branch_message(&edited, Some(ids[1])).await.unwrap();

    let (_, history, _) = orch
        .prepare_history(
            "next",
            conv,
            Vec::new(),
            &[],
            &orch.agent_id,
            &TurnIdentity::default(),
        )
        .await
        .unwrap();

    let texts: Vec<String> = history
        .messages()
        .iter()
        .map(|m| match m {
            ChatHistoryMessage::Text { content, .. } => content.clone(),
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(
        texts[..3],
        ["u1", "a1", "u2 edited"],
        "the raw path: {texts:?}"
    );
    assert!(
        texts.iter().all(|t| !t.contains("fresh summary")),
        "no summary applies to the branch: {texts:?}"
    );
}
