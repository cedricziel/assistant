//! `handle_final_answer_with_extensions` stamps the saved answer with the
//! caller's [`FinalAnswerMeta`], on both the plain and the already-replied path.

use std::collections::HashMap;

use assistant_core::types::conversation::{Interface, MessageRole};
use assistant_storage::InMemoryMessageTimingStore;
use assistant_storage::conversations::{ConversationStore, InMemoryConversationStore};

use super::super::Orchestrator;
use crate::history::FinalAnswerMeta;

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

async fn saved_answer(replied: bool) -> assistant_core::types::conversation::Message {
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
            model: Some("some-model".into()),
        },
    )
    .await
    .unwrap();
    let history = store.load_history(conv.id).await.unwrap();
    history
        .into_iter()
        .find(|m| m.role == MessageRole::Assistant)
        .unwrap()
}

#[tokio::test]
async fn final_answer_with_extensions_records_the_traceparent() {
    let answer = saved_answer(false).await;
    assert_eq!(answer.traceparent.as_deref(), Some(TRACEPARENT));
    assert_eq!(answer.model.as_deref(), Some("some-model"));
}

#[tokio::test]
async fn post_reply_wrap_up_records_the_traceparent() {
    let answer = saved_answer(true).await;
    assert_eq!(answer.traceparent.as_deref(), Some(TRACEPARENT));
    assert_eq!(answer.model.as_deref(), Some("some-model"));
}
