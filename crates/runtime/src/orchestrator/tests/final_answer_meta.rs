//! `handle_final_answer_with_extensions` stamps the saved answer with the
//! caller's [`FinalAnswerMeta`], on both the plain and the already-replied path.

use std::collections::HashMap;

use assistant_core::types::conversation::{Interface, MessageRole};
use assistant_storage::InMemoryMessageTimingStore;
use assistant_storage::conversations::{ConversationStore, InMemoryConversationStore};

use super::super::Orchestrator;
use crate::history::FinalAnswerMeta;

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

async fn saved_traceparent(replied: bool) -> Option<String> {
    let store = InMemoryConversationStore::new();
    let conv = store.create_conversation(Some("t")).await.unwrap();
    Orchestrator::handle_final_answer_with_extensions(
        replied,
        "the answer",
        0,
        0,
        conv.id,
        "agent",
        &Interface::Cli,
        &HashMap::new(),
        &store,
        &InMemoryMessageTimingStore::new(),
        false,
        FinalAnswerMeta {
            traceparent: Some(TRACEPARENT.into()),
            timing: None,
        },
    )
    .await
    .unwrap();
    let history = store.load_history(conv.id).await.unwrap();
    let answer = history
        .iter()
        .find(|m| m.role == MessageRole::Assistant)
        .unwrap();
    answer.traceparent.clone()
}

#[tokio::test]
async fn final_answer_with_extensions_records_the_traceparent() {
    assert_eq!(saved_traceparent(false).await.as_deref(), Some(TRACEPARENT));
}

#[tokio::test]
async fn post_reply_wrap_up_records_the_traceparent() {
    assert_eq!(saved_traceparent(true).await.as_deref(), Some(TRACEPARENT));
}
