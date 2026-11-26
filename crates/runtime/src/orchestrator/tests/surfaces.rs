//! The surface hook: `render-surface` output becomes a persisted surface and
//! a `SurfaceUpdate` event, and never leaks into the tool result.

use std::sync::Arc;

use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{Interface, MessageRole, TurnIdentity};
use assistant_core::types::features::UiConfig;
use assistant_core::{
    LlmProvider, LlmResponse, LlmResponseMeta, MessageBus, ToolCallItem, ToolCallResponse,
};
use assistant_llm_provider::ScriptedLlmProvider;
use assistant_storage::{
    ConversationStore, InMemorySurfaceStore, NewSurface, StorageLayer, SurfaceStatus, SurfaceStore,
    registry::SkillRegistry,
};
use assistant_tool_executor::ToolExecutor;
use assistant_tool_executor::builtins::RenderSurfaceHandler;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::orchestrator::{Orchestrator, OrchestratorEvent};

fn call(id: &str, params: Value) -> ToolCallItem {
    ToolCallItem {
        name: "render-surface".to_string(),
        params,
        id: Some(id.to_string()),
    }
}

fn calls(items: Vec<ToolCallItem>) -> LlmResponse {
    LlmResponse::ToolCalls(ToolCallResponse {
        items,
        meta: LlmResponseMeta::default(),
        thinking: None,
        provider_state: None,
    })
}

fn answer() -> LlmResponse {
    LlmResponse::FinalAnswer("done".to_string(), LlmResponseMeta::default())
}

fn surface(title: &str) -> Value {
    json!({
        "title": title,
        "fallback_text": "Hello there",
        "components": [
            {"id": "root", "component": "Column", "children": ["t"]},
            {"id": "t", "component": "Text", "text": {"@path": "/greeting"}}
        ],
        "data_model": {"greeting": "hi"}
    })
}

struct Harness {
    orch: Orchestrator,
    storage: Arc<StorageLayer>,
    surfaces: Arc<InMemorySurfaceStore>,
    conversation_id: Uuid,
}

struct Surfaced {
    id: String,
    tool_call_id: Option<String>,
    anchor_message_id: String,
    anchor_call_index: u32,
    status: String,
    title: String,
    messages: Vec<Value>,
}

struct Results {
    /// `(status, observation, data)` of every `ToolResult`.
    tool: Vec<(String, String, Option<Value>)>,
    surfaced: Vec<Surfaced>,
}

impl Harness {
    async fn new(script: Vec<LlmResponse>) -> Self {
        let llm = Arc::new(ScriptedLlmProvider::new().with_canned_responses(script));
        let llm_dyn: Arc<dyn LlmProvider> = llm;
        let mut config = OrgConfig::default();
        config.memory.enabled = false;
        config.permissions.allow = vec!["*".parse().expect("valid rule")];

        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let executor = Arc::new(ToolExecutor::new(
            storage.clone(),
            llm_dyn.clone(),
            registry.clone(),
            Arc::new(config.clone()),
        ));
        // Registered in this test's executor only: production registers it
        // once the clients that render surfaces have shipped.
        executor.register_ambient_tool(Arc::new(RenderSurfaceHandler::new(
            Arc::new(storage.attachment_store()),
            UiConfig::default(),
        )));
        let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
        let surfaces = Arc::new(InMemorySurfaceStore::new());
        let orch = Orchestrator::new(llm_dyn, storage.clone(), executor, registry, bus, &config)
            .with_surface_store(surfaces.clone());
        Self {
            orch,
            storage,
            surfaces,
            conversation_id: Uuid::new_v4(),
        }
    }

    /// Store a surface as an earlier `render-surface` call would have.
    async fn seed(&self, id: Uuid, conversation_id: Uuid, turn: i64, status: SurfaceStatus) {
        let draft = surface("Seeded");
        self.surfaces
            .create(&NewSurface {
                id,
                conversation_id,
                turn,
                anchor_message_id: Uuid::new_v4(),
                anchor_call_index: 0,
                tool_call_id: Some("seed".to_string()),
                title: "Seeded".to_string(),
                fallback_text: "Hello there".to_string(),
                catalog_id: assistant_a2ui::CATALOG_ID.to_string(),
                protocol_version: "1.0".to_string(),
                components: draft["components"].clone(),
                data_model: draft["data_model"].clone(),
                send_data_model: true,
            })
            .await
            .unwrap();
        if status == SurfaceStatus::Submitted {
            self.surfaces
                .mark_submitted(id, &json!({"name": "go"}), Uuid::new_v4())
                .await
                .unwrap();
        }
    }

    /// One interactive web turn on this harness's conversation.
    async fn turn(&self) -> Results {
        self.turn_in(self.conversation_id).await
    }

    async fn turn_in(&self, conversation_id: Uuid) -> Results {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(256);
        self.orch
            .run_turn_with_tools_streaming(
                "go",
                conversation_id,
                Interface::Web,
                vec![],
                None,
                vec![],
                tx,
                vec![],
                TurnIdentity {
                    interactive: true,
                    ..TurnIdentity::default()
                },
            )
            .await
            .unwrap();
        let mut results = Results {
            tool: Vec::new(),
            surfaced: Vec::new(),
        };
        while let Ok(event) = rx.try_recv() {
            match event {
                OrchestratorEvent::ToolResult {
                    status,
                    result,
                    data,
                    ..
                } => results
                    .tool
                    .push((status, result.unwrap_or_default(), data)),
                OrchestratorEvent::SurfaceUpdate {
                    surface_id,
                    tool_call_id,
                    anchor_message_id,
                    anchor_call_index,
                    title,
                    status,
                    messages,
                    ..
                } => results.surfaced.push(Surfaced {
                    id: surface_id,
                    tool_call_id,
                    anchor_message_id,
                    anchor_call_index,
                    status,
                    title,
                    messages,
                }),
                _ => {}
            }
        }
        results
    }

    /// Ids of the persisted tool-call messages, oldest first.
    async fn tool_call_message_ids(&self) -> Vec<String> {
        self.storage
            .conversation_store()
            .load_history(self.conversation_id)
            .await
            .unwrap()
            .into_iter()
            .filter(|m| m.tool_calls_json.is_some())
            .map(|m| m.id.to_string())
            .collect()
    }

    async fn persisted_tool_rows(&self) -> Vec<(String, Option<String>)> {
        self.storage
            .conversation_store()
            .load_history(self.conversation_id)
            .await
            .unwrap()
            .into_iter()
            .filter(|m| matches!(m.role, MessageRole::Tool))
            .map(|m| (m.content, m.tool_data_json))
            .collect()
    }
}

#[tokio::test]
async fn create_emits_one_active_surface_anchored_to_its_call() {
    let h = Harness::new(vec![
        calls(vec![
            call("call_a", json!({"title": "first", "fallback_text": "f", "components": [{"id": "root", "component": "Text", "text": "x"}]})),
            call("call_b", surface("Greeting")),
        ]),
        answer(),
    ])
    .await;
    let r = h.turn().await;

    let anchor = h.tool_call_message_ids().await;
    assert_eq!(anchor.len(), 1, "both calls share one tool-call message");
    assert_eq!(r.surfaced.len(), 2, "one event per created surface");
    let second = &r.surfaced[1];
    assert_eq!(second.status, "active");
    assert_eq!(second.title, "Greeting");
    assert_eq!(second.anchor_message_id, anchor[0]);
    assert_eq!(second.anchor_call_index, 1, "index inside the message");
    assert_eq!(second.tool_call_id.as_deref(), Some("call_b"));
    assert_eq!(r.surfaced[0].anchor_call_index, 0);
    assert_eq!(second.messages.len(), 1);
    assert_eq!(second.messages[0]["createSurface"]["surfaceId"], second.id);

    let stored = h
        .surfaces
        .get(Uuid::parse_str(&second.id).unwrap())
        .await
        .unwrap()
        .expect("row persisted");
    assert_eq!(stored.conversation_id, h.conversation_id);
    assert_eq!(stored.anchor_call_index, 1);
    assert_eq!(stored.status, SurfaceStatus::Active);
    assert_eq!(stored.data_model, json!({"greeting": "hi"}));
    assert_eq!(
        h.surfaces
            .list_for_conversation(h.conversation_id)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn tool_result_and_tool_row_carry_no_surface_data() {
    let h = Harness::new(vec![
        calls(vec![call("call_1", surface("Greeting"))]),
        answer(),
    ])
    .await;
    let r = h.turn().await;

    let (status, observation, data) = &r.tool[0];
    assert_eq!(status, "ok");
    assert_eq!(*data, None, "the tool_result frame carries no data");
    assert!(
        observation.contains(&r.surfaced[0].id),
        "the observation names the surface id: {observation}"
    );
    let rows = h.persisted_tool_rows().await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, None, "the persisted tool row carries no data");
    assert!(
        !rows[0].0.contains("components"),
        "no surface JSON in the row"
    );
}

#[tokio::test]
async fn sixth_successful_call_is_rejected() {
    let mut script: Vec<LlmResponse> = (0..6)
        .map(|i| calls(vec![call("c", surface(&format!("s{i}")))]))
        .collect();
    script.push(answer());
    let h = Harness::new(script).await;
    let r = h.turn().await;

    assert_eq!(r.surfaced.len(), 5);
    assert_eq!(r.tool.len(), 6);
    assert!(r.tool[..5].iter().all(|t| t.0 == "ok"));
    assert_eq!(r.tool[5].0, "error");
    assert!(
        r.tool[5].1.contains('5'),
        "names the limit: {}",
        r.tool[5].1
    );
    assert_eq!(
        h.surfaces
            .list_for_conversation(h.conversation_id)
            .await
            .unwrap()
            .len(),
        5
    );
}

fn update(id: Uuid, value: &str) -> ToolCallItem {
    call(
        "u",
        json!({"surface_id": id.to_string(), "data_updates": [{"path": "/greeting", "value": value}]}),
    )
}

#[tokio::test]
async fn updates_count_toward_the_limit() {
    let id = Uuid::new_v4();
    let mut script: Vec<LlmResponse> = (0..5).map(|_| calls(vec![update(id, "x")])).collect();
    script.push(calls(vec![call("c", surface("late"))]));
    script.push(answer());
    let h = Harness::new(script).await;
    h.seed(id, h.conversation_id, 0, SurfaceStatus::Active)
        .await;
    let r = h.turn().await;

    assert_eq!(r.surfaced.len(), 5, "five updates emit five events");
    assert!(r.tool[..5].iter().all(|t| t.0 == "ok"), "{:?}", r.tool);
    assert_eq!(
        r.tool[5].0, "error",
        "the sixth call is a create and is rejected"
    );
}

#[tokio::test]
async fn update_merges_into_the_stored_surface_and_emits_data_messages() {
    let id = Uuid::new_v4();
    let h = Harness::new(vec![
        calls(vec![call(
            "u1",
            json!({"surface_id": id.to_string(), "data_updates": [{"path": "/greeting", "value": "bye"}], "title": "Renamed"}),
        )]),
        answer(),
    ])
    .await;
    h.seed(id, h.conversation_id, 0, SurfaceStatus::Active)
        .await;
    let r = h.turn().await;

    assert_eq!(r.tool[0].0, "ok", "{:?}", r.tool);
    assert_eq!(r.surfaced.len(), 1);
    assert_eq!(r.surfaced[0].id, id.to_string());
    assert_eq!(r.surfaced[0].title, "Renamed");
    assert_eq!(r.surfaced[0].status, "active");
    assert_eq!(r.surfaced[0].messages[0]["updateDataModel"]["value"], "bye");
    let stored = h.surfaces.get(id).await.unwrap().unwrap();
    assert_eq!(stored.data_model, json!({"greeting": "bye"}));
    assert_eq!(stored.title, "Renamed");
}

#[tokio::test]
async fn update_of_a_foreign_surface_errors() {
    let id = Uuid::new_v4();
    let h = Harness::new(vec![calls(vec![update(id, "x")]), answer()]).await;
    h.seed(id, Uuid::new_v4(), 0, SurfaceStatus::Active).await;
    let r = h.turn().await;

    assert_eq!(r.tool[0].0, "error");
    assert!(r.surfaced.is_empty());
    let stored = h.surfaces.get(id).await.unwrap().unwrap();
    assert_eq!(
        stored.data_model,
        json!({"greeting": "hi"}),
        "left untouched"
    );
}

#[tokio::test]
async fn update_of_a_submitted_surface_errors() {
    let id = Uuid::new_v4();
    let h = Harness::new(vec![calls(vec![update(id, "x")]), answer()]).await;
    h.seed(id, h.conversation_id, 0, SurfaceStatus::Submitted)
        .await;
    let r = h.turn().await;

    assert_eq!(r.tool[0].0, "error");
    assert!(r.surfaced.is_empty());
}

#[tokio::test]
async fn update_of_an_earlier_turn_surface_says_it_is_no_longer_open() {
    let id = Uuid::new_v4();
    // Turn one just answers; the surface was created in it (turn 0).
    let h = Harness::new(vec![answer(), calls(vec![update(id, "x")]), answer()]).await;
    h.seed(id, h.conversation_id, 0, SurfaceStatus::Active)
        .await;
    h.turn().await;
    let second = h.turn().await;

    assert_eq!(second.tool[0].0, "error");
    assert!(
        second.tool[0].1.contains("no longer open") && second.tool[0].1.contains("Create a new"),
        "{}",
        second.tool[0].1
    );
    assert!(second.surfaced.is_empty());
}

#[tokio::test]
async fn validation_failure_then_retry_anchors_only_the_retry() {
    let h = Harness::new(vec![
        calls(vec![call(
            "bad",
            json!({"title": "t", "fallback_text": "f", "components": [{"id": "root", "component": "Nope"}]}),
        )]),
        calls(vec![call("good", surface("Greeting"))]),
        answer(),
    ])
    .await;
    let r = h.turn().await;

    assert_eq!(r.tool[0].0, "error");
    assert_eq!(r.tool[1].0, "ok");
    assert_eq!(r.surfaced.len(), 1, "only the retry makes a surface");
    let anchors = h.tool_call_message_ids().await;
    assert_eq!(anchors.len(), 2);
    assert_eq!(r.surfaced[0].anchor_message_id, anchors[1]);
    assert_eq!(r.surfaced[0].tool_call_id.as_deref(), Some("good"));
    assert_eq!(
        h.surfaces
            .list_for_conversation(h.conversation_id)
            .await
            .unwrap()
            .len(),
        1
    );
}
