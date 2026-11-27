//! The surface hook: `render-surface` output becomes a persisted surface and
//! a `SurfaceUpdate` event, and never leaks into the tool result.

use std::sync::Arc;
use std::time::Duration;

use assistant_core::types::agent::OrgConfig;
use assistant_core::types::conversation::{Interface, MessageRole, TurnIdentity};
use assistant_core::types::features::UiConfig;
use assistant_core::{
    LlmProvider, LlmResponse, LlmResponseMeta, MessageBus, ToolCallItem, ToolCallResponse,
};
use assistant_llm_provider::ScriptedLlmProvider;
use assistant_storage::{
    ConversationStore, InMemoryInteractionStore, InMemorySurfaceStore, InteractionStatus,
    InteractionStore, NewSurface, StorageLayer, SurfaceStatus, SurfaceStore,
    registry::SkillRegistry,
};
use assistant_tool_executor::ToolExecutor;
use assistant_tool_executor::builtins::RenderSurfaceHandler;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::interaction::{InteractionBroker, InteractionKinds, Responder};
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

/// The turn's owner, who may answer its interactions.
const OWNER: &str = "alice";

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
    orch: Arc<Orchestrator>,
    storage: Arc<StorageLayer>,
    surfaces: Arc<InMemorySurfaceStore>,
    conversation_id: Uuid,
    /// Set by [`Harness::with_broker`].
    broker: Option<Arc<InteractionBroker>>,
    interactions: Arc<InMemoryInteractionStore>,
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

impl Results {
    fn from_events(events: Vec<OrchestratorEvent>) -> Self {
        let mut results = Results {
            tool: Vec::new(),
            surfaced: Vec::new(),
        };
        for event in events {
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
}

impl Harness {
    async fn new(script: Vec<LlmResponse>) -> Self {
        Self::build(script, None).await
    }

    /// A harness whose turns can wait on interactions. `timeout` is how long
    /// the broker waits for an answer; `None` keeps its default.
    async fn with_broker(script: Vec<LlmResponse>, timeout: Option<Duration>) -> Self {
        Self::build(script, Some(timeout)).await
    }

    async fn build(script: Vec<LlmResponse>, broker: Option<Option<Duration>>) -> Self {
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
        let interactions = Arc::new(InMemoryInteractionStore::new());
        let mut orch =
            Orchestrator::new(llm_dyn, storage.clone(), executor, registry, bus, &config)
                .with_surface_store(surfaces.clone())
                .with_interaction_store(interactions.clone());
        let broker = broker.map(|timeout| {
            let mut broker = InteractionBroker::new(
                interactions.clone(),
                Arc::new(InteractionKinds::with_builtin()),
            );
            if let Some(timeout) = timeout {
                broker = broker.with_timeout(timeout);
            }
            Arc::new(broker)
        });
        if let Some(broker) = &broker {
            orch = orch.with_interaction_broker(broker.clone());
        }
        Self {
            orch: Arc::new(orch),
            storage,
            surfaces,
            conversation_id: Uuid::new_v4(),
            broker,
            interactions,
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
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        Results::from_events(events)
    }

    /// One interactive web turn that is owned by [`OWNER`] and may wait.
    /// `on_request` runs when the turn announces its first interaction, with
    /// the broker, the interaction id and the run id.
    async fn waiting_turn<F, Fut>(&self, on_request: F) -> Vec<OrchestratorEvent>
    where
        F: FnOnce(Arc<InteractionBroker>, String, Uuid) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let broker = self.broker.clone().expect("a harness with a broker");
        let run_id = Uuid::new_v4();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(256);
        let orch = self.orch.clone();
        let conversation_id = self.conversation_id;
        let turn = tokio::spawn(async move {
            orch.run_turn_with_tools_streaming(
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
                    run_id: Some(run_id),
                    user_id: Some(assistant_core::identity::UserId(OWNER.into())),
                    ..TurnIdentity::default()
                },
            )
            .await
        });
        let mut events = Vec::new();
        let mut on_request = Some(on_request);
        while let Some(event) = rx.recv().await {
            if let OrchestratorEvent::InteractionRequested { interaction_id, .. } = &event
                && let Some(act) = on_request.take()
            {
                act(broker.clone(), interaction_id.clone(), run_id).await;
            }
            events.push(event);
        }
        turn.await.unwrap().unwrap();
        events
    }

    /// The owner answers `interaction_id` with `response`.
    async fn answer(&self, broker: &InteractionBroker, id: &str, run: Uuid, response: Value) {
        let conversation = self.conversation_id.to_string();
        broker
            .respond(
                &Responder {
                    user_id: OWNER,
                    conversation_id: &conversation,
                    run_id: &run.to_string(),
                },
                id,
                response,
            )
            .await
            .unwrap();
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

// -- wait_for_action: the surface_action interaction --

/// A form with one Button declaring `submit_form`, asked to wait.
fn form_call(id: &str) -> ToolCallItem {
    let mut params = json!({
        "title": "Trip form",
        "fallback_text": "A form that asks for a city",
        "components": [
            {"id": "root", "component": "Column", "children": ["go"]},
            {"id": "go", "component": "Button", "child": "label",
             "action": {"event": {"name": "submit_form"}}},
            {"id": "label", "component": "Text", "text": "Go"}
        ],
        "data_model": {"city": "Oslo"}
    });
    params["wait_for_action"] = json!(true);
    call(id, params)
}

/// The turn's frames in order, as `label` strings, keeping only the ones a
/// surface wait produces.
fn trace(events: &[OrchestratorEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            OrchestratorEvent::SurfaceUpdate { status, .. } => Some(format!("surface:{status}")),
            OrchestratorEvent::InteractionRequested { kind, .. } => {
                Some(format!("requested:{kind}"))
            }
            OrchestratorEvent::InteractionResolved { outcome, .. } => {
                Some(format!("resolved:{}", outcome.name()))
            }
            OrchestratorEvent::ToolResult { status, .. } => Some(format!("tool:{status}")),
            _ => None,
        })
        .collect()
}

async fn the_only_surface(h: &Harness) -> assistant_storage::Surface {
    let all = h
        .surfaces
        .list_for_conversation(h.conversation_id)
        .await
        .unwrap();
    assert_eq!(all.len(), 1, "one surface");
    all.into_iter().next().unwrap()
}

#[tokio::test]
async fn the_surface_is_announced_before_the_interaction_that_waits_on_it() {
    let h = Harness::with_broker(vec![calls(vec![form_call("c1")]), answer()], None).await;
    let h = &h;
    let events = h
        .waiting_turn(|broker, id, run| async move {
            h.answer(&broker, &id, run, json!({"declined": true})).await;
        })
        .await;

    assert_eq!(
        &trace(&events)[..2],
        ["surface:active", "requested:surface_action"]
    );
    let surface = the_only_surface(h).await;
    let requested = events
        .iter()
        .find_map(|e| match e {
            OrchestratorEvent::InteractionRequested {
                request,
                tool_call_id,
                renders_card,
                ..
            } => Some((request.clone(), tool_call_id.clone(), *renders_card)),
            _ => None,
        })
        .expect("requested");
    assert_eq!(requested.0["surface_id"], surface.id.to_string());
    assert_eq!(requested.0["actions"], json!(["submit_form"]));
    assert_eq!(requested.1.as_deref(), Some("c1"));
    assert!(!requested.2, "the surface is the card");
}

#[tokio::test]
async fn a_submitted_action_becomes_the_observation_and_closes_the_surface() {
    let h = Harness::with_broker(vec![calls(vec![form_call("c1")]), answer()], None).await;
    let h = &h;
    let events = h
        .waiting_turn(|broker, id, run| async move {
            let response = json!({
                "action": {"name": "submit_form", "context": {"city": "Bergen"}},
                "data_model": {"city": "Bergen"},
            });
            h.answer(&broker, &id, run, response).await;
        })
        .await;

    assert_eq!(
        trace(&events),
        [
            "surface:active",
            "requested:surface_action",
            "resolved:resolved",
            "surface:submitted",
            "tool:ok",
        ]
    );
    let results = Results::from_events(events);
    let observation = &results.tool[0].1;
    for needle in ["submit_form", "Bergen"] {
        assert!(observation.contains(needle), "{needle}: {observation}");
    }
    let status_frame = &results.surfaced[1];
    assert_eq!(status_frame.id, results.surfaced[0].id);
    assert!(status_frame.messages.is_empty(), "a status frame only");

    let surface = the_only_surface(h).await;
    assert_eq!(surface.status, SurfaceStatus::Submitted);
    assert_eq!(surface.action.as_ref().unwrap()["name"], "submit_form");
}

#[tokio::test]
async fn a_dismissed_form_stays_open() {
    let h = Harness::with_broker(vec![calls(vec![form_call("c1")]), answer()], None).await;
    let h = &h;
    let events = h
        .waiting_turn(|broker, id, run| async move {
            h.answer(&broker, &id, run, json!({"declined": true})).await;
        })
        .await;

    assert_eq!(
        trace(&events),
        [
            "surface:active",
            "requested:surface_action",
            "resolved:resolved",
            "tool:ok"
        ]
    );
    let results = Results::from_events(events);
    assert!(
        results.tool[0].1.contains("dismissed"),
        "{}",
        results.tool[0].1
    );
    assert_eq!(the_only_surface(h).await.status, SurfaceStatus::Active);
}

#[tokio::test]
async fn an_unanswered_form_expires_and_stays_open() {
    let h = Harness::with_broker(
        vec![calls(vec![form_call("c1")]), answer()],
        Some(Duration::from_millis(50)),
    )
    .await;
    let h = &h;
    let events = h.waiting_turn(|_, _, _| async {}).await;

    assert_eq!(
        trace(&events),
        [
            "surface:active",
            "requested:surface_action",
            "resolved:expired",
            "tool:ok"
        ]
    );
    let results = Results::from_events(events);
    assert!(
        results.tool[0].1.contains("not answered yet"),
        "{}",
        results.tool[0].1
    );
    assert_eq!(the_only_surface(h).await.status, SurfaceStatus::Active);
}

#[tokio::test]
async fn a_cancelled_wait_stays_open() {
    let h = Harness::with_broker(vec![calls(vec![form_call("c1")]), answer()], None).await;
    let h = &h;
    let events = h
        .waiting_turn(|broker, _, run| async move {
            // What ends a turn, and what a branch does to a pending interaction.
            broker.abandon_run(&run.to_string()).await.unwrap();
        })
        .await;

    assert_eq!(
        trace(&events),
        [
            "surface:active",
            "requested:surface_action",
            "resolved:cancelled",
            "tool:ok"
        ]
    );
    let results = Results::from_events(events);
    assert!(
        results.tool[0].1.contains("moved on"),
        "{}",
        results.tool[0].1
    );
    assert_eq!(the_only_surface(h).await.status, SurfaceStatus::Active);
    let pending = h
        .interactions
        .list_for_conversation(
            &h.conversation_id.to_string(),
            Some(InteractionStatus::Pending),
        )
        .await
        .unwrap();
    assert!(pending.is_empty());
}

#[tokio::test]
async fn losing_the_race_to_a_post_turn_submit_says_so() {
    let h = Harness::with_broker(vec![calls(vec![form_call("c1")]), answer()], None).await;
    let h = &h;
    let events = h
        .waiting_turn(|broker, id, run| async move {
            // The surface is submitted another way before this answer lands.
            let stored = the_only_surface(h).await;
            h.surfaces
                .mark_submitted(stored.id, &json!({"name": "submit_form"}), Uuid::new_v4())
                .await
                .unwrap();
            let response = json!({"action": {"name": "submit_form"}});
            h.answer(&broker, &id, run, response).await;
        })
        .await;

    let results = Results::from_events(events);
    assert_eq!(results.surfaced.len(), 1, "no status frame for the loser");
    let observation = &results.tool[0].1;
    assert!(
        observation.contains("already submitted in a new message"),
        "{observation}"
    );
}

#[tokio::test]
async fn without_a_broker_the_surface_is_shown_and_the_turn_does_not_wait() {
    let h = Harness::new(vec![calls(vec![form_call("c1")]), answer()]).await;
    let r = h.turn().await;

    assert_eq!(r.tool[0].0, "ok");
    assert_eq!(r.surfaced.len(), 1);
    assert!(r.tool[0].1.contains(&r.surfaced[0].id), "{}", r.tool[0].1);
}
