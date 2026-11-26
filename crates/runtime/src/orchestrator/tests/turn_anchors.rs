//! Edit and regenerate anchors, and idempotent redelivery of anchored turns.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use uuid::Uuid;

use assistant_core::auth::AuthContext;
use assistant_core::bus_messages::TurnAnchor;
use assistant_core::clock::FakeClock;
use assistant_core::llm::provider::Capabilities;
use assistant_core::tool::{ToolHandler, ToolOutput, TurnScope, TurnToolSource};
use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{ExecutionContext, Interface, Message, MessageRole};
use assistant_core::{
    AttachmentMeta, ChatHistoryMessage, ChatRole, ContentBlock, LlmProvider, LlmResponse,
    LlmResponseMeta, MessageBus, StreamChunk, ToolCallItem, ToolCallResponse, ToolSpec,
};
use assistant_llm_provider::scripted::ScriptedLlmProvider;
use assistant_storage::conversations::ConversationStore;
use assistant_storage::{AttachmentStore, StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;

use super::super::{Orchestrator, SubmitTurn};

// -- Rig ----------------------------------------------------------------------

/// A scripted provider that fails one chosen call before it produces output.
struct Flaky {
    inner: ScriptedLlmProvider,
    calls: AtomicUsize,
    fail_call: Option<usize>,
    error: &'static str,
}

impl Flaky {
    fn check(&self) -> anyhow::Result<()> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if Some(n) == self.fail_call {
            anyhow::bail!("{}", self.error);
        }
        Ok(())
    }
}

#[async_trait]
impl LlmProvider for Flaky {
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }

    async fn chat(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        self.check()?;
        self.inner.chat(system_prompt, history, tools).await
    }

    async fn chat_streaming(
        &self,
        system_prompt: &str,
        history: &[ChatHistoryMessage],
        tools: &[ToolSpec],
        chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        self.check()?;
        self.inner
            .chat_streaming(system_prompt, history, tools, chunk_sink)
            .await
    }
}

/// A tool whose only job is to make a turn persist rows before it fails.
struct Probe;

#[async_trait]
impl ToolHandler for Probe {
    fn name(&self) -> &str {
        "mcp__user_probe__ctx"
    }
    fn description(&self) -> &str {
        "does nothing"
    }
    fn params_schema(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }
    fn is_mutating(&self) -> bool {
        true
    }
    async fn run(
        &self,
        _params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        Ok(ToolOutput::success("ok"))
    }
}

struct ProbeSource;

#[async_trait]
impl TurnToolSource for ProbeSource {
    async fn tools_for_turn(&self, _scope: &TurnScope<'_>) -> Vec<Arc<dyn ToolHandler>> {
        vec![Arc::new(Probe)]
    }
}

fn answer(text: &str) -> LlmResponse {
    LlmResponse::FinalAnswer(text.into(), LlmResponseMeta::default())
}

fn probe_call() -> LlmResponse {
    LlmResponse::ToolCalls(ToolCallResponse {
        items: vec![ToolCallItem {
            name: "mcp__user_probe__ctx".into(),
            params: json!({}),
            id: None,
        }],
        meta: LlmResponseMeta::default(),
        thinking: None,
        provider_state: None,
    })
}

struct AbortOnDrop(JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

struct Rig {
    orch: Arc<Orchestrator>,
    storage: Arc<StorageLayer>,
    llm: Arc<Flaky>,
    store: assistant_storage::conversations::SqliteConversationStore,
    conv: Uuid,
    _tasks: [AbortOnDrop; 2],
}

/// An orchestrator with a running worker. The `fail_call`th provider call
/// (0-based) fails with `error`; the others replay `responses` in order.
async fn rig(responses: Vec<LlmResponse>, fail_call: Option<usize>, error: &'static str) -> Rig {
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.permissions.allow = vec!["mcp__user_probe__ctx".parse().unwrap()];
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm = Arc::new(Flaky {
        inner: ScriptedLlmProvider::new().with_canned_responses(responses),
        calls: AtomicUsize::new(0),
        fail_call,
        error,
    });
    let dyn_llm: Arc<dyn LlmProvider> = llm.clone();
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        dyn_llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    // The bus runs on a fake clock so the 30 s redelivery backoff is instant.
    let clock = Arc::new(FakeClock::new_at_seed());
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus().with_clock(clock.clone()));
    let orch = Arc::new(
        Orchestrator::new(
            dyn_llm,
            storage.clone(),
            executor.clone(),
            registry,
            bus,
            &config,
        )
        .with_turn_tool_source(Arc::new(ProbeSource), "acme", "eng")
        .with_submit_timeout(20),
    );
    executor.set_subagent_runner(orch.clone());
    let worker = orch.clone();
    let worker = tokio::spawn(async move { worker.run_worker("anchor-worker").await });
    let ticker = tokio::spawn(async move {
        loop {
            clock.advance(Duration::from_secs(300));
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    });
    let store = storage.conversation_store_for_agent(&orch.agent_id);
    Rig {
        orch,
        storage,
        llm,
        store,
        conv: Uuid::new_v4(),
        _tasks: [AbortOnDrop(worker), AbortOnDrop(ticker)],
    }
}

fn msg(conv: Uuid, role: MessageRole, text: &str, turn: i64) -> Message {
    let mut m = Message::new(conv, role, text);
    m.turn = turn;
    m
}

/// `U1 → A1 → U2 → A2`, with `A2` the active leaf.
struct Tree {
    u1: Uuid,
    a1: Uuid,
    u2: Uuid,
    a2: Uuid,
}

async fn seed(rig: &Rig) -> Tree {
    rig.store
        .create_conversation_with_id(rig.conv, None)
        .await
        .unwrap();
    let rows = [
        msg(rig.conv, MessageRole::User, "U1", 0),
        msg(rig.conv, MessageRole::Assistant, "A1", 1),
        msg(rig.conv, MessageRole::User, "U2", 2),
        msg(rig.conv, MessageRole::Assistant, "A2", 3),
    ];
    for row in &rows {
        rig.store.save_message(row).await.unwrap();
    }
    Tree {
        u1: rows[0].id,
        a1: rows[1].id,
        u2: rows[2].id,
        a2: rows[3].id,
    }
}

/// An inactive branch `A1 → UB → AB → UC → AC`; the active leaf stays `A2`.
struct Inactive {
    ub: Uuid,
    ab: Uuid,
    uc: Uuid,
}

async fn seed_inactive_branch(rig: &Rig, tree: &Tree) -> Inactive {
    let ub = msg(rig.conv, MessageRole::User, "UB", 2);
    rig.store.branch_message(&ub, Some(tree.a1)).await.unwrap();
    let ab = msg(rig.conv, MessageRole::Assistant, "AB", 3);
    rig.store.save_message(&ab).await.unwrap();
    let uc = msg(rig.conv, MessageRole::User, "UC", 4);
    rig.store.save_message(&uc).await.unwrap();
    let ac = msg(rig.conv, MessageRole::Assistant, "AC", 5);
    rig.store.save_message(&ac).await.unwrap();
    rig.store.set_leaf(rig.conv, tree.a2).await.unwrap();
    Inactive {
        ub: ub.id,
        ab: ab.id,
        uc: uc.id,
    }
}

async fn submit(
    rig: &Rig,
    prompt: &str,
    anchor: TurnAnchor,
    user_message_id: Option<Uuid>,
    run_id: Uuid,
) {
    let turn = SubmitTurn {
        anchor,
        user_message_id,
        ..SubmitTurn::new(prompt, rig.conv, Interface::Web)
            .with_auth(&AuthContext::system())
            .with_request_id(run_id)
    };
    // A terminal failure comes back as a result flagged `had_errors`; the
    // tests read the stored tree, not the result.
    let _ = rig.orch.submit(turn).await;
}

async fn path(rig: &Rig) -> Vec<Message> {
    rig.store.load_history(rig.conv).await.unwrap()
}

fn contents(messages: &[Message]) -> Vec<&str> {
    messages.iter().map(|m| m.content.as_str()).collect()
}

async fn child_ids(rig: &Rig, parent: Uuid) -> Vec<Uuid> {
    rig.store
        .children_of(rig.conv, Some(parent))
        .await
        .unwrap()
        .iter()
        .map(|m| m.id)
        .collect()
}

/// What call `n` sent to the provider, one string per history entry.
fn sent(rig: &Rig, n: usize) -> Vec<String> {
    let calls = rig.llm.inner.recorded_calls();
    calls[n]
        .history
        .iter()
        .map(|h| match h {
            ChatHistoryMessage::Text { role, content } => {
                let who = match role {
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::System => "system",
                    ChatRole::Tool => "tool",
                };
                format!("{who}: {content}")
            }
            ChatHistoryMessage::MultimodalUser { content } => {
                let text: Vec<&str> = content
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text(t) => Some(t.as_str()),
                        _ => None,
                    })
                    .collect();
                format!("user: {}", text.join(" + "))
            }
            ChatHistoryMessage::AssistantToolCalls { .. } => "assistant: <tool calls>".into(),
            ChatHistoryMessage::ToolResult { name, .. } => format!("tool: {name}"),
        })
        .collect()
}

async fn rows_of_run(rig: &Rig, run_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE run_id = ?1")
        .bind(run_id.to_string())
        .fetch_one(&rig.storage.pool)
        .await
        .unwrap()
}

/// Stores a text attachment bound to `message_id`. In-memory storage keeps
/// the bytes in memory, so nothing reaches the disk.
async fn attach(rig: &Rig, message_id: Uuid, text: &str) -> AttachmentMeta {
    let meta = AttachmentMeta {
        id: Uuid::new_v4(),
        message_id: None,
        conversation_id: rig.conv,
        agent_id: rig.orch.agent_id.clone(),
        filename: "notes.txt".into(),
        mime_type: "text/plain".into(),
        size_bytes: text.len() as u64,
        created_at: chrono::Utc::now(),
    };
    let store = rig.storage.attachment_store();
    store.store(&meta, text.as_bytes()).await.unwrap();
    store.link_to_message(meta.id, message_id).await.unwrap();
    meta
}

// -- Edit ---------------------------------------------------------------------

#[tokio::test]
async fn edit_saves_the_new_user_row_under_the_parent_with_the_preassigned_id() {
    let rig = rig(vec![answer("A2 edited")], None, "").await;
    let tree = seed(&rig).await;
    let (new_id, run_id) = (Uuid::new_v4(), Uuid::new_v4());

    submit(
        &rig,
        "U2 edited",
        TurnAnchor::Edit {
            parent_message_id: Some(tree.a1),
            edited_message_id: tree.u2,
        },
        Some(new_id),
        run_id,
    )
    .await;

    let active = path(&rig).await;
    assert_eq!(contents(&active), ["U1", "A1", "U2 edited", "A2 edited"]);
    assert_eq!(active[2].id, new_id, "the pre-assigned id is used");
    assert_eq!(active[2].parent_id, Some(tree.a1));
    assert_eq!(
        active[3].parent_id,
        Some(new_id),
        "the reply chains after U'"
    );
    assert_eq!(
        child_ids(&rig, tree.a1).await,
        [tree.u2, new_id],
        "the original stays as a sibling"
    );
    assert_eq!(
        child_ids(&rig, tree.u2).await,
        [tree.a2],
        "the original keeps its reply"
    );
    assert_eq!(
        sent(&rig, 0),
        ["user: U1", "assistant: A1", "user: U2 edited"],
        "history is P's chain plus U'"
    );
    assert_eq!(active[2].run_id, Some(run_id));
    assert_eq!(active[3].run_id, Some(run_id));
}

#[tokio::test]
async fn editing_the_first_message_creates_a_second_root() {
    let rig = rig(vec![answer("fresh")], None, "").await;
    let tree = seed(&rig).await;
    let new_id = Uuid::new_v4();

    submit(
        &rig,
        "U1 edited",
        TurnAnchor::Edit {
            parent_message_id: None,
            edited_message_id: tree.u1,
        },
        Some(new_id),
        Uuid::new_v4(),
    )
    .await;

    assert_eq!(contents(&path(&rig).await), ["U1 edited", "fresh"]);
    let roots = rig.store.children_of(rig.conv, None).await.unwrap();
    assert_eq!(roots.len(), 2, "the edit is a sibling root");
    assert_eq!(sent(&rig, 0), ["user: U1 edited"]);
}

#[tokio::test]
async fn editing_a_message_on_an_inactive_branch_uses_its_ancestor_chain() {
    let rig = rig(vec![answer("reply")], None, "").await;
    let tree = seed(&rig).await;
    let inactive = seed_inactive_branch(&rig, &tree).await;

    submit(
        &rig,
        "UC edited",
        TurnAnchor::Edit {
            parent_message_id: Some(inactive.ab),
            edited_message_id: inactive.uc,
        },
        Some(Uuid::new_v4()),
        Uuid::new_v4(),
    )
    .await;

    assert_eq!(
        sent(&rig, 0),
        [
            "user: U1",
            "assistant: A1",
            "user: UB",
            "assistant: AB",
            "user: UC edited"
        ]
    );
    let active = path(&rig).await;
    assert_eq!(
        contents(&active),
        ["U1", "A1", "UB", "AB", "UC edited", "reply"],
        "the new branch becomes active"
    );
    assert_ne!(active[2].id, tree.u2, "the other branch is not on the path");
}

#[tokio::test]
async fn edit_carries_the_attachments_forward_into_the_turn() {
    let rig = rig(vec![answer("seen")], None, "").await;
    let tree = seed(&rig).await;
    let meta = attach(&rig, tree.u2, "attached notes").await;
    let new_id = Uuid::new_v4();

    submit(
        &rig,
        "U2 edited",
        TurnAnchor::Edit {
            parent_message_id: Some(tree.a1),
            edited_message_id: tree.u2,
        },
        Some(new_id),
        Uuid::new_v4(),
    )
    .await;

    let store = rig.storage.attachment_store();
    let carried = store.list_for_message(new_id).await.unwrap();
    assert_eq!(carried.iter().map(|a| a.id).collect::<Vec<_>>(), [meta.id]);
    let original = store.list_for_message(tree.u2).await.unwrap();
    assert_eq!(original.len(), 1, "the original keeps its attachment");
    let last = sent(&rig, 0).pop().unwrap();
    assert!(
        last.starts_with("user: U2 edited + ") && last.contains("attached notes"),
        "the turn's request includes the attachment: {last}"
    );
}

// -- Regenerate ---------------------------------------------------------------

#[tokio::test]
async fn regenerate_saves_no_user_row_and_branches_the_reply_under_the_user() {
    let rig = rig(vec![answer("A2 again")], None, "").await;
    let tree = seed(&rig).await;

    submit(
        &rig,
        "U2",
        TurnAnchor::Regenerate {
            user_message_id: tree.u2,
        },
        None,
        Uuid::new_v4(),
    )
    .await;

    let active = path(&rig).await;
    assert_eq!(contents(&active), ["U1", "A1", "U2", "A2 again"]);
    assert_eq!(active[2].id, tree.u2, "U itself is on the path");
    assert_eq!(active[3].parent_id, Some(tree.u2));
    let children = child_ids(&rig, tree.u2).await;
    assert_eq!(children.len(), 2, "old and new reply");
    assert_eq!(children[0], tree.a2);
    assert_eq!(
        child_ids(&rig, tree.a1).await,
        [tree.u2],
        "no new user row appears"
    );
    assert_eq!(sent(&rig, 0), ["user: U1", "assistant: A1", "user: U2"]);
}

#[tokio::test]
async fn regenerate_sends_the_attachments_of_the_user_message() {
    let rig = rig(vec![answer("again")], None, "").await;
    let tree = seed(&rig).await;
    attach(&rig, tree.u2, "attached notes").await;

    submit(
        &rig,
        "U2",
        TurnAnchor::Regenerate {
            user_message_id: tree.u2,
        },
        None,
        Uuid::new_v4(),
    )
    .await;

    let last = sent(&rig, 0).pop().unwrap();
    assert!(
        last.starts_with("user: U2 + ") && last.contains("attached notes"),
        "U's attachments are part of the request: {last}"
    );
}

#[tokio::test]
async fn regenerate_that_fails_before_output_leaves_the_leaf_alone() {
    let rig = rig(vec![], Some(0), "model exploded").await;
    let tree = seed(&rig).await;

    submit(
        &rig,
        "U2",
        TurnAnchor::Regenerate {
            user_message_id: tree.u2,
        },
        None,
        Uuid::new_v4(),
    )
    .await;

    assert_eq!(contents(&path(&rig).await), ["U1", "A1", "U2", "A2"]);
    assert_eq!(
        child_ids(&rig, tree.u2).await,
        [tree.a2],
        "no placeholder reply was added"
    );
}

#[tokio::test]
async fn regenerating_a_message_on_an_inactive_branch_uses_its_ancestor_chain() {
    let rig = rig(vec![answer("again")], None, "").await;
    let tree = seed(&rig).await;
    let inactive = seed_inactive_branch(&rig, &tree).await;

    submit(
        &rig,
        "UC",
        TurnAnchor::Regenerate {
            user_message_id: inactive.uc,
        },
        None,
        Uuid::new_v4(),
    )
    .await;

    assert_eq!(
        sent(&rig, 0),
        [
            "user: U1",
            "assistant: A1",
            "user: UB",
            "assistant: AB",
            "user: UC"
        ]
    );
    assert_eq!(
        contents(&path(&rig).await),
        ["U1", "A1", "UB", "AB", "UC", "again"]
    );
    assert_eq!(inactive.ub, path(&rig).await[2].id);
}

// -- Turn numbering (design D4) ---------------------------------------------------

#[tokio::test]
async fn base_turn_follows_the_anchor() {
    // Append: the active path has four rows, so the user row is turn 4.
    let appended = rig(vec![answer("r")], None, "").await;
    seed(&appended).await;
    submit(&appended, "U3", TurnAnchor::Append, None, Uuid::new_v4()).await;
    let turns: Vec<i64> = path(&appended).await.iter().map(|m| m.turn).collect();
    assert_eq!(turns, [0, 1, 2, 3, 4, 5], "append continues the path");

    // Edit with parent A1: P's chain has two rows, so U' is turn 2.
    let edited = rig(vec![answer("r")], None, "").await;
    let tree = seed(&edited).await;
    let anchor = TurnAnchor::Edit {
        parent_message_id: Some(tree.a1),
        edited_message_id: tree.u2,
    };
    submit(&edited, "U2'", anchor, Some(Uuid::new_v4()), Uuid::new_v4()).await;
    let turns: Vec<i64> = path(&edited).await.iter().map(|m| m.turn).collect();
    assert_eq!(turns, [0, 1, 2, 3], "edit restarts at the parent's depth");

    // Regenerate U2: the reply gets the turn of the reply it replaces.
    let regenerated = rig(vec![answer("r")], None, "").await;
    let tree = seed(&regenerated).await;
    let anchor = TurnAnchor::Regenerate {
        user_message_id: tree.u2,
    };
    submit(&regenerated, "U2", anchor, None, Uuid::new_v4()).await;
    let turns: Vec<i64> = path(&regenerated).await.iter().map(|m| m.turn).collect();
    assert_eq!(turns, [0, 1, 2, 3], "regenerate reuses the reply's turn");
}

// -- Redelivery (design D3a) ------------------------------------------------------

const TRANSIENT: &str = "429 rate limit exceeded";

/// Attempt one runs a tool and then hits a transient error; the nacked turn
/// runs again and answers `fresh`.
fn flaky_script() -> Vec<LlmResponse> {
    vec![probe_call(), answer("fresh")]
}

#[tokio::test]
async fn a_redelivered_regenerate_leaves_exactly_one_new_reply() {
    let rig = rig(flaky_script(), Some(1), TRANSIENT).await;
    let tree = seed(&rig).await;
    let run_id = Uuid::new_v4();

    submit(
        &rig,
        "U2",
        TurnAnchor::Regenerate {
            user_message_id: tree.u2,
        },
        None,
        run_id,
    )
    .await;

    let children = child_ids(&rig, tree.u2).await;
    assert_eq!(children.len(), 2, "the old reply and exactly one new one");
    let active = path(&rig).await;
    assert_eq!(contents(&active), ["U1", "A1", "U2", "fresh"]);
    assert_eq!(
        rows_of_run(&rig, run_id).await,
        1,
        "the first attempt's partial rows are gone"
    );
    assert_eq!(active[3].run_id, Some(run_id));
}

#[tokio::test]
async fn a_redelivered_edit_leaves_one_new_user_row_and_one_link_per_attachment() {
    let rig = rig(flaky_script(), Some(1), TRANSIENT).await;
    let tree = seed(&rig).await;
    let meta = attach(&rig, tree.u2, "attached notes").await;
    let (new_id, run_id) = (Uuid::new_v4(), Uuid::new_v4());

    submit(
        &rig,
        "U2 edited",
        TurnAnchor::Edit {
            parent_message_id: Some(tree.a1),
            edited_message_id: tree.u2,
        },
        Some(new_id),
        run_id,
    )
    .await;

    assert_eq!(child_ids(&rig, tree.a1).await, [tree.u2, new_id]);
    assert_eq!(
        contents(&path(&rig).await),
        ["U1", "A1", "U2 edited", "fresh"]
    );
    let carried = rig
        .storage
        .attachment_store()
        .list_for_message(new_id)
        .await
        .unwrap();
    assert_eq!(carried.iter().map(|a| a.id).collect::<Vec<_>>(), [meta.id]);
    assert_eq!(rows_of_run(&rig, run_id).await, 2, "U' and the reply");
}

#[tokio::test]
async fn a_redelivered_append_keeps_one_user_row() {
    let rig = rig(flaky_script(), Some(1), TRANSIENT).await;
    seed(&rig).await;
    let run_id = Uuid::new_v4();

    submit(&rig, "U3", TurnAnchor::Append, None, run_id).await;

    let active = path(&rig).await;
    assert_eq!(contents(&active), ["U1", "A1", "U2", "A2", "U3", "fresh"]);
    assert_eq!(
        active.iter().map(|m| m.turn).collect::<Vec<_>>(),
        [0, 1, 2, 3, 4, 5],
        "the second attempt numbers from the same base"
    );
    assert_eq!(rows_of_run(&rig, run_id).await, 2, "user row and reply");
    let second_attempt = sent(&rig, 1);
    assert_eq!(
        second_attempt.last().map(String::as_str),
        Some("user: U3"),
        "the prompt is sent once: {second_attempt:?}"
    );
    assert_eq!(
        second_attempt.len(),
        5,
        "U1 A1 U2 A2 U3: {second_attempt:?}"
    );
}
