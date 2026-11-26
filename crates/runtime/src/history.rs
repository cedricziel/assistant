//! Conversation history management helpers for the orchestrator.
//!
//! These functions handle converting persisted [`Message`] records into the
//! [`ChatHistoryMessage`] format the LLM expects, repairing structural issues
//! (orphaned messages, missing tool results), and error recovery.

use std::collections::HashMap;
use std::ops::Deref;

use anyhow::Result;
use tracing::{debug, warn};
use uuid::Uuid;

use assistant_core::types::conversation::{Message, MessageRole};
use assistant_core::{
    ChatHistoryMessage, ChatRole, ContentBlock, ProviderState, ToolCallItem, TurnTiming,
};
use assistant_storage::conversations::ConversationStore;
use assistant_storage::{ConversationSummaryStore, MessageTimingStore};

use crate::orchestrator::TurnStore;

/// Pre-loaded attachment content blocks keyed by message ID.
///
/// Built by callers that load bytes from the [`AttachmentStore`] and convert
/// them into the appropriate [`ContentBlock`] variant (Image, Document, or
/// Text) before replaying history.
pub(crate) type AttachmentMap = HashMap<Uuid, Vec<ContentBlock>>;

/// Convert a sequence of persisted [`Message`] records into
/// [`ChatHistoryMessage`] values suitable for sending to the LLM.
///
/// When `attachment_map` is non-empty, user messages whose ID appears in the
/// map are emitted as [`ChatHistoryMessage::MultimodalUser`] with inline
/// image blocks so the LLM can see previously-sent images on history replay.
#[cfg(test)]
pub(crate) fn messages_to_chat_history(
    messages: Vec<Message>,
    attachment_map: &AttachmentMap,
) -> Vec<ChatHistoryMessage> {
    messages
        .into_iter()
        .filter_map(|m| message_to_chat(m, attachment_map))
        .collect()
}

/// Convert one persisted [`Message`]; `None` for roles the LLM never sees.
fn message_to_chat(m: Message, attachment_map: &AttachmentMap) -> Option<ChatHistoryMessage> {
    match m.role {
        MessageRole::User => {
            // Audio is heard only in its own turn; the text stands in later.
            let replayed: Vec<ContentBlock> = attachment_map
                .get(&m.id)
                .into_iter()
                .flatten()
                .filter(|b| !matches!(b, ContentBlock::Audio { .. }))
                .cloned()
                .collect();
            if !replayed.is_empty() {
                let mut blocks = vec![ContentBlock::Text(m.content)];
                blocks.extend(replayed);
                return Some(ChatHistoryMessage::MultimodalUser { content: blocks });
            }
            Some(ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: m.content,
            })
        }
        MessageRole::Assistant => {
            if let Some(tc_json) = m.tool_calls_json
                && let Ok(items) = serde_json::from_str::<Vec<ToolCallItem>>(&tc_json)
                && !items.is_empty()
            {
                // A corrupt state is dropped; the tool calls themselves stay.
                let provider_state = m
                    .provider_state_json
                    .as_deref()
                    .and_then(|json| serde_json::from_str::<ProviderState>(json).ok());
                return Some(ChatHistoryMessage::AssistantToolCalls {
                    items,
                    provider_state,
                });
            }
            Some(ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: m.content,
            })
        }
        MessageRole::Tool => Some(ChatHistoryMessage::ToolResult {
            name: m.skill_name.unwrap_or_default(),
            content: m.content,
        }),
        _ => None,
    }
}

/// The messages a turn sends to the LLM, with the persisted row behind each.
///
/// `row_ids[i]` is the id of the stored message that produced `messages[i]`,
/// or `None` for entries with no row of their own (the summary, synthetic
/// repairs). The two vectors only change together, through [`push`], [`insert`]
/// and [`remove`], so they cannot drift apart. Compaction uses the ids to say which
/// stored message a summary runs through.
///
/// [`push`]: TurnHistory::push
/// [`insert`]: TurnHistory::insert
/// [`remove`]: TurnHistory::remove
#[derive(Debug, Clone, Default)]
pub(crate) struct TurnHistory {
    messages: Vec<ChatHistoryMessage>,
    row_ids: Vec<Option<Uuid>>,
}

impl TurnHistory {
    /// Wrap messages that have no persisted rows (every id is `None`).
    #[cfg(test)]
    pub(crate) fn unmapped(messages: Vec<ChatHistoryMessage>) -> Self {
        let row_ids = vec![None; messages.len()];
        Self { messages, row_ids }
    }

    #[cfg(test)]
    pub(crate) fn messages(&self) -> &[ChatHistoryMessage] {
        &self.messages
    }

    /// The persisted row behind each entry of [`messages`](Self::messages).
    #[cfg(test)]
    pub(crate) fn row_ids(&self) -> Vec<Option<Uuid>> {
        self.row_ids.clone()
    }

    /// The persisted row behind entry `index`, if it has one.
    pub(crate) fn row_id(&self, index: usize) -> Option<Uuid> {
        self.row_ids.get(index).copied().flatten()
    }

    /// Append `message`, which was persisted as `row_id` when it has a row.
    pub(crate) fn push(&mut self, message: ChatHistoryMessage, row_id: Option<Uuid>) {
        self.messages.push(message);
        self.row_ids.push(row_id);
    }

    /// Insert a synthetic entry, which has no row.
    pub(crate) fn insert(&mut self, index: usize, message: ChatHistoryMessage) {
        self.messages.insert(index, message);
        self.row_ids.insert(index, None);
    }

    /// Whether a user message carries a recording, which makes the turn a
    /// voice turn.
    pub(crate) fn has_audio(&self) -> bool {
        self.messages.iter().any(|m| {
            matches!(m, ChatHistoryMessage::MultimodalUser { content }
                if content.iter().any(|b| matches!(b, ContentBlock::Audio { .. })))
        })
    }

    /// Removes recordings, for a model that doesn't hear them. A user
    /// message left with only its text becomes a plain text message.
    pub(crate) fn drop_audio(&mut self) {
        for message in &mut self.messages {
            let ChatHistoryMessage::MultimodalUser { content } = message else {
                continue;
            };
            content.retain(|b| !matches!(b, ContentBlock::Audio { .. }));
            if let [ContentBlock::Text(text)] = content.as_mut_slice() {
                *message = ChatHistoryMessage::Text {
                    role: ChatRole::User,
                    content: std::mem::take(text),
                };
            }
        }
    }

    pub(crate) fn remove(&mut self, index: usize) {
        self.messages.remove(index);
        self.row_ids.remove(index);
    }

    /// Replace the first `split_at` entries with one summary entry.
    pub(crate) fn replace_prefix_with_summary(
        &mut self,
        split_at: usize,
        summary: ChatHistoryMessage,
    ) {
        self.messages.splice(..split_at, std::iter::once(summary));
        self.row_ids.splice(..split_at, std::iter::once(None));
    }
}

impl Deref for TurnHistory {
    type Target = [ChatHistoryMessage];

    fn deref(&self) -> &Self::Target {
        &self.messages
    }
}

/// Build the history a turn sends to the LLM from the path `path` (root
/// first) of `conversation_id`.
///
/// When the newest summary whose `through_message_id` lies on the path
/// exists, the history is that summary followed by the path rows after it.
/// A summary that covers messages the path branched away from does not apply.
/// Without a summary the history is the whole path.
pub(crate) async fn build_turn_history(
    path: Vec<Message>,
    conversation_id: Uuid,
    summaries: &dyn ConversationSummaryStore,
    attachment_map: &AttachmentMap,
) -> Result<TurnHistory> {
    let path_ids: Vec<Uuid> = path.iter().map(|m| m.id).collect();
    let summary = summaries.newest_on_path(conversation_id, &path_ids).await?;

    let mut history = TurnHistory::default();
    let mut rows = path;
    if let Some(summary) = summary {
        // `newest_on_path` only returns a summary whose message is on the path.
        let after = path_ids
            .iter()
            .position(|id| *id == summary.through_message_id)
            .map_or(0, |i| i + 1);
        rows = rows.split_off(after);
        history.push(crate::compaction::summary_message(&summary.content), None);
    }
    for message in rows {
        let id = message.id;
        if let Some(entry) = message_to_chat(message, attachment_map) {
            history.push(entry, Some(id));
        }
    }
    Ok(history)
}

/// Repair structural problems in a loaded conversation history.
///
/// Two issues are addressed:
///
/// 1. **Trailing orphaned user message** – a prior turn may have failed
///    after persisting the user message but before any assistant response
///    was saved.  A synthetic assistant message is inserted so the caller
///    can safely append a new user message without creating consecutive
///    user entries (which Anthropic rejects outright and which confuse
///    most tool-calling models).
///
/// 2. **Orphaned `AssistantToolCalls`** – the process may have crashed
///    after persisting a tool-call message but before all `ToolResult`
///    messages were written.  Missing results are filled in with a
///    synthetic error result so providers that require tool results
///    (Ollama, Anthropic) do not reject the request.
pub(crate) fn sanitize_history(history: &mut TurnHistory) {
    if history.is_empty() {
        return;
    }

    // --- Pass 1: drop orphaned ToolResult messages -------------------------
    //
    // A ToolResult is valid only when it immediately follows an
    // AssistantToolCalls (or another valid ToolResult within the same
    // batch).  System-injected results (e.g. from skill-learner) that
    // have no preceding AssistantToolCalls cause providers to reject the
    // request because the tool_call_id has no matching tool call.
    {
        let mut i = 0;
        let mut remaining_results: usize = 0; // results still expected from current batch
        while i < history.len() {
            match &history[i] {
                ChatHistoryMessage::AssistantToolCalls { items: calls, .. } => {
                    remaining_results = calls.len();
                    i += 1;
                }
                ChatHistoryMessage::ToolResult { name, .. } => {
                    if remaining_results > 0 {
                        remaining_results -= 1;
                        i += 1;
                    } else {
                        debug!(
                            tool = %name,
                            "Sanitizing history: dropping orphaned ToolResult with no preceding tool call"
                        );
                        history.remove(i);
                        // don't advance i; the next element slid into this position
                    }
                }
                _ => {
                    remaining_results = 0;
                    i += 1;
                }
            }
        }
    }

    // --- Pass 2: fill in missing tool results for orphaned tool calls ------
    //
    // Walk the history and, for every AssistantToolCalls, count how many
    // ToolResult messages follow (before the next non-ToolResult entry or
    // the end of the list).  If fewer results exist than calls, insert
    // synthetic ones.
    let mut i = 0;
    while i < history.len() {
        if let ChatHistoryMessage::AssistantToolCalls { items: calls, .. } = &history[i] {
            let expected = calls.len();
            let call_names: Vec<String> = calls.iter().map(|c| c.name.clone()).collect();

            // Count consecutive ToolResult messages immediately following.
            let mut result_count = 0;
            while i + 1 + result_count < history.len() {
                if matches!(
                    history[i + 1 + result_count],
                    ChatHistoryMessage::ToolResult { .. }
                ) {
                    result_count += 1;
                } else {
                    break;
                }
            }

            if result_count < expected {
                let insert_at = i + 1 + result_count;
                let missing = expected - result_count;
                debug!(
                    expected,
                    result_count, missing, "Sanitizing history: inserting synthetic tool results"
                );
                for j in result_count..expected {
                    let name = call_names
                        .get(j)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    history.insert(
                        insert_at + (j - result_count),
                        ChatHistoryMessage::ToolResult {
                            name,
                            content: "[error: result lost due to a prior crash]".to_string(),
                        },
                    );
                }
                // Advance past the newly inserted results.
                i = insert_at + missing;
                continue;
            }
        }
        i += 1;
    }

    // --- Pass 2: trailing orphaned user message ----------------------------
    let is_trailing_user = matches!(
        history.last(),
        Some(ChatHistoryMessage::Text {
            role: ChatRole::User,
            ..
        }) | Some(ChatHistoryMessage::MultimodalUser { .. })
    );

    if is_trailing_user {
        debug!(
            "Sanitizing history: inserting synthetic assistant message after orphaned user message"
        );
        history.push(
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: "[An error occurred processing the previous message.]".to_string(),
            },
            None,
        );
    }
}

/// Per-reply metadata recorded on a final assistant answer.
///
/// Every final-answer save site goes through [`persist_final_answer`] with one
/// of these, so a new field (timing, model, ...) is added here and in the
/// helper instead of at each site.
#[derive(Debug, Clone, Default)]
pub(crate) struct FinalAnswerMeta {
    /// W3C `traceparent` of the LLM span that produced the answer; `None`
    /// when tracing is off.
    pub traceparent: Option<String>,
    /// How long the turn took; stored under the saved message's id. `None`
    /// for paths that do not measure it.
    pub timing: Option<TurnTiming>,
    /// The model that produced the answer: the provider's `model_name()`.
    pub model: Option<String>,
}

/// Save a final assistant answer, stamping it with `meta`.
///
/// The timing is stored after the message is saved. A failure to store it is
/// logged and does not fail the save.
pub(crate) async fn persist_final_answer(
    conv_store: &dyn ConversationStore,
    timing_store: &dyn MessageTimingStore,
    mut msg: Message,
    meta: FinalAnswerMeta,
) -> anyhow::Result<()> {
    msg.traceparent = meta.traceparent;
    msg.model = meta.model;
    conv_store.save_message(&msg).await?;
    if let Some(timing) = meta.timing
        && let Err(e) = timing_store.put(msg.conversation_id, msg.id, &timing).await
    {
        warn!(message_id = %msg.id, "Failed to persist message timing: {e}");
    }
    Ok(())
}

/// Persist a synthetic assistant message so the conversation history
/// maintains proper User→Assistant alternation after a turn error.
///
/// Called when the tool-calling loop (or the LLM call itself) fails.
/// The user message was already persisted by `prepare_history`; without
/// this recovery message the orphaned user entry would poison subsequent
/// turns.
///
/// A regenerate that has persisted nothing yet already has a reply to fall
/// back on, so it persists no placeholder: the old reply stays active.
pub(crate) async fn persist_error_recovery(conv_store: &TurnStore, conversation_id: Uuid) {
    if conv_store.branch_pending() {
        return;
    }
    let error_msg = Message::assistant(
        conversation_id,
        "[An error occurred processing this message.]",
    );
    if let Err(e) = conv_store.save_message(&error_msg).await {
        warn!("Failed to persist error recovery assistant message: {e}");
    }
}

/// Append a tool result to the running chat history.
///
/// `row_id` is the stored tool-result message, when there is one.
pub(crate) fn append_tool_result(
    history: &mut TurnHistory,
    name: &str,
    content: &str,
    row_id: Option<Uuid>,
) {
    history.push(
        ChatHistoryMessage::ToolResult {
            name: name.to_string(),
            content: content.to_string(),
        },
        row_id,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::types::conversation::MessageRole;

    fn make_user_msg(id: Uuid, content: &str) -> Message {
        Message {
            id,
            conversation_id: Uuid::new_v4(),
            role: MessageRole::User,
            content: content.to_string(),
            skill_name: None,
            tool_calls_json: None,
            provider_state_json: None,
            turn: 0,
            created_at: chrono::Utc::now(),
            sender_user_id: None,
            parent_id: None,
            run_id: None,
            tool_call_id: None,
            tool_status: None,
            tool_data_json: None,
            tool_app_json: None,
            traceparent: None,
            model: None,
            finish_reason: None,
        }
    }

    fn make_assistant_msg(content: &str) -> Message {
        Message {
            id: Uuid::new_v4(),
            conversation_id: Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: content.to_string(),
            skill_name: None,
            tool_calls_json: None,
            provider_state_json: None,
            turn: 1,
            created_at: chrono::Utc::now(),
            sender_user_id: None,
            parent_id: None,
            run_id: None,
            tool_call_id: None,
            tool_status: None,
            tool_data_json: None,
            tool_app_json: None,
            traceparent: None,
            model: None,
            finish_reason: None,
        }
    }

    fn tool_call_msg(provider_state_json: Option<&str>) -> Message {
        let mut m = make_assistant_msg("");
        m.tool_calls_json = Some(r#"[{"name":"bash","params":{"command":"ls"},"id":"c1"}]"#.into());
        m.provider_state_json = provider_state_json.map(str::to_string);
        m
    }

    // -- build_turn_history ---------------------------------------------------

    fn path_of(contents: &[&str]) -> Vec<Message> {
        let conv = Uuid::new_v4();
        contents
            .iter()
            .enumerate()
            .map(|(i, content)| {
                let mut m = if i % 2 == 0 {
                    Message::user(conv, *content)
                } else {
                    Message::assistant(conv, *content)
                };
                m.turn = i as i64;
                m
            })
            .collect()
    }

    fn text_of(msg: &ChatHistoryMessage) -> &str {
        match msg {
            ChatHistoryMessage::Text { content, .. } => content,
            other => panic!("expected a text message, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn build_turn_history_without_a_summary_equals_the_raw_path() {
        let path = path_of(&["q1", "a1", "q2", "a2"]);
        let conv = path[0].conversation_id;
        let summaries = assistant_storage::InMemoryConversationSummaryStore::new();

        let built = build_turn_history(path.clone(), conv, &summaries, &AttachmentMap::new())
            .await
            .unwrap();

        let expected = messages_to_chat_history(path.clone(), &AttachmentMap::new());
        assert_eq!(
            format!("{:?}", built.messages()),
            format!("{expected:?}"),
            "no summary leaves the path untouched"
        );
        assert_eq!(
            built.row_ids(),
            path.iter().map(|m| Some(m.id)).collect::<Vec<_>>(),
            "every history entry maps to its row"
        );
    }

    #[tokio::test]
    async fn build_turn_history_starts_with_the_summary_then_the_rows_after_it() {
        let path = path_of(&["q1", "a1", "q2", "a2", "q3"]);
        let conv = path[0].conversation_id;
        let summaries = assistant_storage::InMemoryConversationSummaryStore::new();
        summaries
            .insert(conv, path[1].id, "talked about X")
            .await
            .unwrap();

        let built = build_turn_history(path.clone(), conv, &summaries, &AttachmentMap::new())
            .await
            .unwrap();

        let texts: Vec<&str> = built.messages().iter().map(text_of).collect();
        assert_eq!(texts.len(), 4, "summary plus the three rows after a1");
        assert!(
            texts[0].starts_with("[Conversation summary — earlier context compacted]"),
            "summary header first, got {:?}",
            texts[0]
        );
        assert!(texts[0].ends_with("talked about X"));
        assert_eq!(&texts[1..], ["q2", "a2", "q3"]);
        assert_eq!(built.row_ids().len(), built.messages().len());
        assert_eq!(built.row_ids()[0], None, "the summary has no row");
        assert_eq!(built.row_ids()[1], Some(path[2].id));
    }

    #[tokio::test]
    async fn build_turn_history_ignores_a_summary_that_is_not_on_the_path() {
        let path = path_of(&["q1", "a1"]);
        let conv = path[0].conversation_id;
        let summaries = assistant_storage::InMemoryConversationSummaryStore::new();
        summaries
            .insert(conv, Uuid::new_v4(), "from another branch")
            .await
            .unwrap();

        let built = build_turn_history(path, conv, &summaries, &AttachmentMap::new())
            .await
            .unwrap();

        let texts: Vec<&str> = built.messages().iter().map(text_of).collect();
        assert_eq!(texts, ["q1", "a1"]);
    }

    // -- TurnHistory ----------------------------------------------------------

    #[test]
    fn turn_history_push_keeps_row_ids_the_same_length() {
        let mut h = TurnHistory::unmapped(vec![ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "q".into(),
        }]);
        assert_eq!(h.row_ids().len(), h.messages().len());

        let id = Uuid::new_v4();
        h.push(
            ChatHistoryMessage::AssistantToolCalls {
                items: vec![],
                provider_state: None,
            },
            Some(id),
        );
        append_tool_result(&mut h, "bash", "ok", None);

        assert_eq!(h.messages().len(), 3);
        assert_eq!(h.row_ids(), vec![None, Some(id), None]);
    }

    #[test]
    fn turn_history_remove_and_insert_keep_row_ids_the_same_length() {
        let a = Uuid::new_v4();
        let mut h = TurnHistory::unmapped(vec![]);
        for (i, id) in [Some(a), None, Some(Uuid::new_v4())]
            .into_iter()
            .enumerate()
        {
            h.push(
                ChatHistoryMessage::Text {
                    role: ChatRole::User,
                    content: format!("m{i}"),
                },
                id,
            );
        }

        h.remove(0);
        h.insert(
            1,
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: "synthetic".into(),
            },
        );

        assert_eq!(h.messages().len(), 3);
        assert_eq!(h.row_ids().len(), 3);
        assert_eq!(h.row_ids()[1], None, "an inserted entry has no row");
    }

    #[test]
    fn history_restores_provider_state_on_tool_calls() {
        let history = messages_to_chat_history(
            vec![tool_call_msg(Some(
                r#"{"provider":"openrouter","model":"m","data":{"reasoning_details":[{"type":"reasoning.text","text":"x"}]}}"#,
            ))],
            &AttachmentMap::new(),
        );
        let ChatHistoryMessage::AssistantToolCalls {
            items,
            provider_state,
        } = &history[0]
        else {
            panic!("expected tool calls, got {:?}", history[0]);
        };
        assert_eq!(items[0].name, "bash");
        let state = provider_state.as_ref().expect("provider state restored");
        assert_eq!(state.provider, "openrouter");
        assert_eq!(state.data["reasoning_details"][0]["text"], "x");
    }

    #[test]
    fn history_keeps_tool_calls_when_provider_state_is_unreadable() {
        let history =
            messages_to_chat_history(vec![tool_call_msg(Some("not json"))], &AttachmentMap::new());
        assert!(
            matches!(
                &history[0],
                ChatHistoryMessage::AssistantToolCalls { items, provider_state: None } if items.len() == 1
            ),
            "a corrupt provider state is dropped, never the tool calls: {:?}",
            history[0]
        );
    }

    #[test]
    fn history_without_attachment_map_produces_plain_text() {
        let msgs = vec![
            make_user_msg(Uuid::new_v4(), "hello"),
            make_assistant_msg("hi there"),
        ];
        let map = AttachmentMap::new();
        let history = messages_to_chat_history(msgs, &map);

        assert_eq!(history.len(), 2);
        assert!(matches!(
            &history[0],
            ChatHistoryMessage::Text { role: ChatRole::User, content } if content == "hello"
        ));
    }

    #[test]
    fn history_with_attachment_map_produces_multimodal_user() {
        let msg_id = Uuid::new_v4();
        let msgs = vec![
            make_user_msg(msg_id, "look at this image"),
            make_assistant_msg("I see it"),
        ];

        let mut map = AttachmentMap::new();
        map.insert(
            msg_id,
            vec![ContentBlock::Image {
                media_type: "image/png".to_string(),
                data: "base64data".to_string(),
            }],
        );

        let history = messages_to_chat_history(msgs, &map);
        assert_eq!(history.len(), 2);

        match &history[0] {
            ChatHistoryMessage::MultimodalUser { content } => {
                assert_eq!(content.len(), 2, "text + image");
                assert!(matches!(&content[0], ContentBlock::Text(t) if t == "look at this image"));
                assert!(matches!(
                    &content[1],
                    ContentBlock::Image { media_type, data }
                    if media_type == "image/png" && data == "base64data"
                ));
            }
            other => panic!("expected MultimodalUser, got {:?}", other),
        }
    }

    #[test]
    fn past_voice_messages_replay_as_text_only() {
        let msg_id = Uuid::new_v4();
        let mut map = AttachmentMap::new();
        map.insert(
            msg_id,
            vec![ContentBlock::Audio {
                media_type: "audio/ogg".to_string(),
                data: "T2dnUw==".to_string(),
            }],
        );

        let history = messages_to_chat_history(vec![make_user_msg(msg_id, "hello")], &map);

        match &history[0] {
            ChatHistoryMessage::Text { content, .. } => assert_eq!(content, "hello"),
            other => panic!("expected the text only, got {other:?}"),
        }
    }

    #[test]
    fn history_with_document_attachment_produces_multimodal_user() {
        let msg_id = Uuid::new_v4();
        let msgs = vec![make_user_msg(msg_id, "summarize this")];

        let mut map = AttachmentMap::new();
        map.insert(
            msg_id,
            vec![ContentBlock::Document {
                media_type: "application/pdf".to_string(),
                data: "JVBERi0=".to_string(),
            }],
        );

        let history = messages_to_chat_history(msgs, &map);
        match &history[0] {
            ChatHistoryMessage::MultimodalUser { content } => {
                assert_eq!(content.len(), 2, "text + document");
                assert!(matches!(
                    &content[1],
                    ContentBlock::Document { media_type, .. }
                    if media_type == "application/pdf"
                ));
            }
            other => panic!("expected MultimodalUser, got {:?}", other),
        }
    }

    #[test]
    fn history_with_text_attachment_produces_multimodal_user() {
        let msg_id = Uuid::new_v4();
        let msgs = vec![make_user_msg(msg_id, "check this")];

        let mut map = AttachmentMap::new();
        map.insert(
            msg_id,
            vec![ContentBlock::Text(
                "--- file: notes.md ---\n# Notes\n--- end file ---".to_string(),
            )],
        );

        let history = messages_to_chat_history(msgs, &map);
        match &history[0] {
            ChatHistoryMessage::MultimodalUser { content } => {
                assert_eq!(content.len(), 2, "text + inlined file");
                assert!(matches!(&content[1], ContentBlock::Text(t) if t.contains("notes.md")));
            }
            other => panic!("expected MultimodalUser, got {:?}", other),
        }
    }

    #[test]
    fn history_with_empty_attachment_list_produces_plain_text() {
        let msg_id = Uuid::new_v4();
        let msgs = vec![make_user_msg(msg_id, "no images")];

        let mut map = AttachmentMap::new();
        map.insert(msg_id, vec![]);

        let history = messages_to_chat_history(msgs, &map);
        assert!(matches!(
            &history[0],
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                ..
            }
        ));
    }
}
