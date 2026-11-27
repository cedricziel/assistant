//! `Orchestrator::persist_cancelled_reply`: the partial reply of a stopped turn
//! becomes the turn's assistant message, so reload and the next LLM turn see it.

use std::sync::Arc;

use uuid::Uuid;

use assistant_core::bus_messages::{self, TurnAnchor, topic};
use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{Interface, Message, MessageFinishReason, MessageRole};
use assistant_core::{LlmProvider, LlmResponse, LlmResponseMeta};
use assistant_llm_provider::scripted::ScriptedLlmProvider;
use assistant_storage::StorageLayer;
use assistant_storage::conversations::ConversationStore;

use super::super::{CANCELLED_REPLY_PLACEHOLDER, CancelledTurn, Orchestrator};
use super::{build, build_with_llm};

struct Rig {
    orch: Arc<Orchestrator>,
    storage: Arc<StorageLayer>,
    conv: Uuid,
    /// The run whose cancellation the tests persist.
    run: Uuid,
}

impl Rig {
    async fn new() -> Self {
        let (orch, storage) = build("http://127.0.0.1:1").await;
        Self::over(orch, storage).await
    }

    async fn over(orch: Arc<Orchestrator>, storage: Arc<StorageLayer>) -> Self {
        let conv = Uuid::new_v4();
        storage
            .conversation_store_for_agent(orch.agent_id())
            .create_conversation_with_id(conv, None)
            .await
            .unwrap();
        Self {
            orch,
            storage,
            conv,
            run: Uuid::new_v4(),
        }
    }

    /// `row` as written by the cancelled run.
    fn mine(&self, mut row: Message) -> Message {
        row.run_id = Some(self.run);
        row
    }

    fn store(&self) -> impl ConversationStore + use<> {
        self.storage
            .conversation_store_for_agent(self.orch.agent_id())
    }

    async fn seed(&self, rows: Vec<Message>) {
        let store = self.store();
        for (turn, mut row) in rows.into_iter().enumerate() {
            row.turn = turn as i64;
            store.save_message(&row).await.unwrap();
        }
    }

    async fn persist(&self, text: &str) -> Uuid {
        self.persist_anchored(text, TurnAnchor::Append).await
    }

    async fn persist_anchored(&self, text: &str, anchor: TurnAnchor) -> Uuid {
        let turn = CancelledTurn {
            run_id: self.run,
            anchor,
        };
        self.orch
            .persist_cancelled_reply(self.conv, text, &Interface::Web, &turn)
            .await
            .unwrap()
    }

    async fn history(&self) -> Vec<Message> {
        self.store().load_history(self.conv).await.unwrap()
    }

    async fn announcements(&self) -> Vec<bus_messages::TurnResult> {
        self.orch
            .bus()
            .list(topic::TURN_COMPLETED, None, 10)
            .await
            .unwrap()
            .into_iter()
            .map(|m| serde_json::from_value(m.payload).unwrap())
            .collect()
    }
}

fn user(conv: Uuid, text: &str) -> Message {
    Message::new(conv, MessageRole::User, text)
}

fn tool_call_row(conv: Uuid, content: &str) -> Message {
    let mut row = Message::assistant(conv, content);
    row.tool_calls_json = Some(r#"[{"id":"c1","name":"bash","arguments":{}}]"#.to_string());
    row
}

fn tool_result_row(conv: Uuid) -> Message {
    Message::new(conv, MessageRole::Tool, "ok")
}

#[tokio::test]
async fn the_partial_becomes_the_turns_assistant_row() {
    let rig = Rig::new().await;
    rig.seed(vec![user(rig.conv, "hi")]).await;

    let id = rig.persist("Partial").await;

    let history = rig.history().await;
    assert_eq!(history.len(), 2, "user row plus the partial");
    let saved = &history[1];
    assert_eq!(saved.id, id);
    assert_eq!(saved.role, MessageRole::Assistant);
    assert_eq!(saved.content, "Partial");
    assert_eq!(saved.finish_reason, Some(MessageFinishReason::Cancelled));
    assert_eq!(saved.turn, 1, "the turn index is the history length");
}

#[tokio::test]
async fn no_text_persists_the_stopped_placeholder() {
    let rig = Rig::new().await;
    rig.seed(vec![user(rig.conv, "hi")]).await;

    let id = rig.persist("").await;

    let saved = rig.history().await.pop().unwrap();
    assert_eq!(saved.id, id);
    assert_eq!(saved.content, CANCELLED_REPLY_PLACEHOLDER);
    assert_eq!(CANCELLED_REPLY_PLACEHOLDER, "_Stopped._");
    assert_eq!(saved.finish_reason, Some(MessageFinishReason::Cancelled));
}

#[tokio::test]
async fn whitespace_only_text_counts_as_no_text() {
    let rig = Rig::new().await;
    rig.seed(vec![user(rig.conv, "hi")]).await;

    rig.persist(" \n\t ").await;

    let saved = rig.history().await.pop().unwrap();
    assert_eq!(saved.content, CANCELLED_REPLY_PLACEHOLDER);
}

#[tokio::test]
async fn a_final_answer_that_won_the_race_is_not_duplicated() {
    let rig = Rig::new().await;
    let answer = rig.mine(Message::assistant(rig.conv, "Full"));
    let answer_id = answer.id;
    rig.seed(vec![user(rig.conv, "hi"), answer]).await;

    let id = rig.persist("Partial").await;

    assert_eq!(id, answer_id, "the existing reply is reported");
    let history = rig.history().await;
    assert_eq!(history.len(), 2, "nothing was saved");
    assert_eq!(history[1].finish_reason, None, "the reply stays unmarked");
    assert!(
        rig.announcements().await.is_empty(),
        "the worker already announced that reply"
    );
}

#[tokio::test]
async fn tool_rows_do_not_trip_the_race_guard() {
    let rig = Rig::new().await;
    rig.seed(vec![
        user(rig.conv, "hi"),
        tool_call_row(rig.conv, ""),
        tool_result_row(rig.conv),
    ])
    .await;

    rig.persist("Partial").await;
    assert_eq!(rig.history().await.len(), 4);
}

#[tokio::test]
async fn a_tool_call_row_without_content_does_not_trip_the_race_guard() {
    let rig = Rig::new().await;
    rig.seed(vec![user(rig.conv, "hi"), tool_call_row(rig.conv, "")])
        .await;

    rig.persist("Partial").await;
    assert_eq!(rig.history().await.len(), 3);
}

#[tokio::test]
async fn narration_on_a_tool_call_row_is_not_a_final_answer() {
    let rig = Rig::new().await;
    rig.seed(vec![
        user(rig.conv, "hi"),
        tool_call_row(rig.conv, "Looking..."),
    ])
    .await;

    rig.persist("Partial").await;
    assert_eq!(rig.history().await.len(), 3);
}

#[tokio::test]
async fn the_guard_looks_only_after_the_last_user_row() {
    let rig = Rig::new().await;
    rig.seed(vec![
        user(rig.conv, "one"),
        Message::assistant(rig.conv, "Old"),
        user(rig.conv, "two"),
    ])
    .await;

    let id = rig.persist("Partial").await;

    let history = rig.history().await;
    assert_eq!(history.len(), 4);
    assert_eq!(history[3].id, id);
    assert_eq!(history[1].finish_reason, None, "the old reply is untouched");
}

#[tokio::test]
async fn a_saved_row_announces_turn_completed() {
    let rig = Rig::new().await;
    rig.seed(vec![user(rig.conv, "hi")]).await;

    let id = rig.persist("Partial").await;

    let announced = rig.announcements().await;
    assert_eq!(announced.len(), 1, "one turn.completed");
    let event = &announced[0];
    assert_eq!(event.conversation_id, rig.conv);
    assert_eq!(event.content, "Partial");
    assert_eq!(event.turn, 1);
    assert_eq!(event.message_id, Some(id));
    assert!(!event.had_errors);
    let results = rig
        .orch
        .bus()
        .list(topic::TURN_RESULT, None, 10)
        .await
        .unwrap();
    assert!(results.is_empty(), "no turn.result: nobody waits for it");
}

#[tokio::test]
async fn a_user_message_without_text_is_not_announced() {
    let rig = Rig::new().await;
    rig.seed(vec![user(rig.conv, "  ")]).await;

    rig.persist("Partial").await;

    assert!(rig.announcements().await.is_empty());
}

fn title_llm() -> Arc<dyn LlmProvider> {
    Arc::new(
        ScriptedLlmProvider::new().with_canned_responses(vec![LlmResponse::FinalAnswer(
            "Stopped Chat".to_string(),
            LlmResponseMeta::default(),
        )]),
    )
}

#[tokio::test]
async fn a_stopped_first_turn_still_gets_a_title() {
    let (orch, storage) = build_with_llm(title_llm(), OrgConfig::default(), |o| o).await;
    let rig = Rig::over(orch, storage.clone()).await;
    rig.seed(vec![user(rig.conv, "Plan my week")]).await;

    rig.persist("Sure, here").await;

    // A config where the first turn qualifies.
    let titling = assistant_core::types::features::TitlingConfig {
        min_turns: 1,
        ..Default::default()
    };
    let worker = crate::TitleGeneratorWorker::new(
        rig.orch.bus().clone(),
        storage.clone(),
        title_llm(),
        titling,
        rig.orch.agent_id().to_string(),
    );
    worker.run_one().await.unwrap();

    let conv = rig
        .store()
        .get_conversation(rig.conv)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(conv.title.as_deref(), Some("Stopped Chat"));
}

#[tokio::test]
async fn a_failed_save_is_an_error_and_announces_nothing() {
    let rig = Rig::new().await;
    rig.seed(vec![user(rig.conv, "hi")]).await;
    sqlx::query(
        "CREATE TRIGGER refuse_cancelled BEFORE INSERT ON messages \
         WHEN NEW.finish_reason = 'cancelled' BEGIN SELECT RAISE(ABORT, 'disk full'); END",
    )
    .execute(&rig.storage.pool)
    .await
    .unwrap();

    let turn = CancelledTurn {
        run_id: rig.run,
        anchor: TurnAnchor::Append,
    };
    let result = rig
        .orch
        .persist_cancelled_reply(rig.conv, "Partial", &Interface::Web, &turn)
        .await;

    assert!(result.is_err(), "the save failure reaches the caller");
    assert_eq!(rig.history().await.len(), 1);
    assert!(rig.announcements().await.is_empty());
}

// -- Anchor-aware replies (message-branching PR 7b) --

fn regenerate(user: &Message) -> TurnAnchor {
    TurnAnchor::Regenerate {
        user_message_id: user.id,
    }
}

/// `U` answered by an old reply, the state a regenerate starts from.
async fn regenerate_rig() -> (Rig, Message, Message) {
    let rig = Rig::new().await;
    let u = user(rig.conv, "hi");
    let old = Message::assistant(rig.conv, "Old reply");
    rig.seed(vec![u.clone(), old.clone()]).await;
    (rig, u, old)
}

#[tokio::test]
async fn a_cancelled_regenerate_with_no_rows_saves_the_partial_under_the_user_row() {
    let (rig, u, old) = regenerate_rig().await;

    let id = rig.persist_anchored("Partial", regenerate(&u)).await;

    let saved = rig.store().get_message(id).await.unwrap().unwrap();
    assert_eq!(saved.parent_id, Some(u.id), "a sibling of the old reply");
    assert_eq!(saved.content, "Partial");
    assert_eq!(saved.finish_reason, Some(MessageFinishReason::Cancelled));
    assert_eq!(saved.run_id, Some(rig.run), "the run owns the saved row");
    let siblings = rig.store().children_of(rig.conv, Some(u.id)).await.unwrap();
    assert_eq!(
        siblings.iter().map(|m| m.id).collect::<Vec<_>>(),
        vec![old.id, id],
        "the old reply is kept as the other version"
    );
    let history = rig.history().await;
    assert_eq!(
        history.last().map(|m| m.id),
        Some(id),
        "the new row is active"
    );
    assert_eq!(history.len(), 2, "U then the partial");
}

#[tokio::test]
async fn a_cancelled_regenerate_with_no_text_saves_a_stopped_child_of_the_user_row() {
    let (rig, u, _old) = regenerate_rig().await;

    let id = rig.persist_anchored("", regenerate(&u)).await;

    let saved = rig.store().get_message(id).await.unwrap().unwrap();
    assert_eq!(saved.parent_id, Some(u.id));
    assert_eq!(saved.content, CANCELLED_REPLY_PLACEHOLDER);
    assert_eq!(saved.finish_reason, Some(MessageFinishReason::Cancelled));
}

#[tokio::test]
async fn a_cancelled_regenerate_after_a_tool_call_row_appends_after_it() {
    let (rig, u, old) = regenerate_rig().await;
    // The runner's first row: a child of U, which made it the active leaf.
    let call = rig.mine(tool_call_row(rig.conv, ""));
    rig.store().branch_message(&call, Some(u.id)).await.unwrap();

    let id = rig.persist_anchored("Partial", regenerate(&u)).await;

    let saved = rig.store().get_message(id).await.unwrap().unwrap();
    assert_eq!(saved.parent_id, Some(call.id), "after the run's own row");
    let under_old = rig
        .store()
        .children_of(rig.conv, Some(old.id))
        .await
        .unwrap();
    assert!(under_old.is_empty(), "nothing hangs under the old reply");
    let ids: Vec<_> = rig.history().await.iter().map(|m| m.id).collect();
    assert_eq!(ids, vec![u.id, call.id, id]);
}

#[tokio::test]
async fn a_cancelled_edit_appends_under_the_edited_user_row() {
    let rig = Rig::new().await;
    let original = user(rig.conv, "first");
    rig.seed(vec![original.clone(), Message::assistant(rig.conv, "Old")])
        .await;
    let edited = rig.mine(user(rig.conv, "second"));
    rig.store().branch_message(&edited, None).await.unwrap();
    let anchor = TurnAnchor::Edit {
        parent_message_id: None,
        edited_message_id: original.id,
    };

    let id = rig.persist_anchored("Partial", anchor).await;

    let saved = rig.store().get_message(id).await.unwrap().unwrap();
    assert_eq!(saved.parent_id, Some(edited.id));
    assert_eq!(saved.finish_reason, Some(MessageFinishReason::Cancelled));
}

#[tokio::test]
async fn the_race_guard_ignores_rows_of_other_runs() {
    let (rig, u, old) = regenerate_rig().await;
    assert!(old.run_id.is_none(), "the old reply belongs to no live run");

    let id = rig.persist_anchored("Partial", regenerate(&u)).await;

    assert_ne!(
        id, old.id,
        "the old reply is not reported as this run's answer"
    );
    let kept = rig.store().get_message(old.id).await.unwrap().unwrap();
    assert_eq!(kept.finish_reason, None, "the old reply stays unmarked");
}

#[tokio::test]
async fn the_race_guard_sees_the_final_answer_of_its_own_regenerate() {
    let (rig, u, _old) = regenerate_rig().await;
    let answer = rig.mine(Message::assistant(rig.conv, "Full"));
    rig.store()
        .branch_message(&answer, Some(u.id))
        .await
        .unwrap();

    let id = rig.persist_anchored("Partial", regenerate(&u)).await;

    assert_eq!(id, answer.id);
    assert_eq!(rig.history().await.len(), 2, "nothing was saved");
}

#[tokio::test]
async fn a_regenerate_of_an_earlier_user_row_branches_there() {
    let rig = Rig::new().await;
    let u1 = user(rig.conv, "one");
    let a1 = Message::assistant(rig.conv, "Reply one");
    let u2 = user(rig.conv, "two");
    rig.seed(vec![
        u1.clone(),
        a1.clone(),
        u2.clone(),
        Message::assistant(rig.conv, "Reply two"),
    ])
    .await;

    let id = rig.persist_anchored("Partial", regenerate(&u1)).await;

    let saved = rig.store().get_message(id).await.unwrap().unwrap();
    assert_eq!(saved.parent_id, Some(u1.id));
    let ids: Vec<_> = rig.history().await.iter().map(|m| m.id).collect();
    assert_eq!(
        ids,
        vec![u1.id, id],
        "the later turns stay on the other branch"
    );
    assert_eq!(saved.turn, 1, "the turn follows its user row");
}
