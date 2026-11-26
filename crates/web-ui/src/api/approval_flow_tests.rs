//! Tool approvals end to end (ADR-0017): the HTTP routes and the turn worker
//! live in separate runtimes that share `space.db` and the bus, as the
//! packaged install runs them. Approvals are switched on by the harness, not
//! by `send_message`: web chat does not ask for approvals yet.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use assistant_core::auth::AuthContext;
use assistant_core::bus_messages::TurnAnchor;
use assistant_core::types::agent::OrgConfig;
use assistant_runtime::TurnMode;
use assistant_storage::{
    InteractionRow, InteractionStatus, InteractionStore as _, StorageLayer,
    ToolPermissionRuleStore as _,
};

use super::messages::{RunOptions, start_turn, stream_turn_to_sse};
use super::test_helpers::{
    app, body_bytes, default_space, seed_conversation, split_process_state_with_approvals,
};
use super::{ApiState, SpaceCtx};

/// One SSE frame: `(event, data)`.
type Frame = (String, Value);

fn parse_sse(text: &str) -> Vec<Frame> {
    text.split("\n\n")
        .filter(|block| !block.trim().is_empty())
        .map(|block| {
            let (mut event, mut data) = (String::new(), Value::Null);
            for line in block.lines() {
                if let Some(v) = line.strip_prefix("event:") {
                    event = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("data:") {
                    data = serde_json::from_str(v.trim()).unwrap_or(Value::Null);
                }
            }
            (event, data)
        })
        .collect()
}

fn events(frames: &[Frame]) -> Vec<&str> {
    frames.iter().map(|(event, _)| event.as_str()).collect()
}

/// `/api/chat` replies in order: a `bash` call of `command`, then `answer`,
/// once per entry of `commands`.
async fn mount_script(server: &MockServer, commands: &[&str]) {
    for command in commands {
        let call = json!({
            "model": "test",
            "message": {
                "role": "assistant", "content": null,
                "tool_calls": [{"function": {"name": "bash", "arguments": {"command": command}}}],
            },
            "done": true,
        });
        let answer = json!({
            "model": "test",
            "message": {"role": "assistant", "content": "all done"},
            "done": true,
        });
        for body in [call, answer] {
            Mock::given(method("POST"))
                .and(path("/api/chat"))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .up_to_n_times(1)
                .mount(server)
                .await;
        }
    }
}

struct Harness {
    state: ApiState,
    storage: Arc<StorageLayer>,
    llm: MockServer,
    conversation: Uuid,
    dir: tempfile::TempDir,
}

impl Harness {
    async fn new(commands: &[&str], tune: impl FnOnce(&mut OrgConfig)) -> Self {
        let llm = MockServer::start().await;
        mount_script(&llm, commands).await;
        let mut config = OrgConfig::default();
        config.memory.enabled = false;
        config.suggestions.enabled = false;
        tune(&mut config);
        let (state, storage) = split_process_state_with_approvals(&llm.uri(), config).await;
        let conversation = seed_conversation(&storage).await;
        Self {
            state,
            storage,
            llm,
            conversation,
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn marker(&self) -> std::path::PathBuf {
        self.dir.path().join("marker")
    }

    fn touch_marker(&self) -> String {
        format!("touch {}", self.marker().display())
    }

    fn ctx(&self) -> SpaceCtx {
        let space = default_space(&self.storage);
        SpaceCtx {
            auth: AuthContext::system(),
            space_id: assistant_core::identity::SpaceId::from(space.id),
            org_slug: space.org_slug.into(),
            space_slug: space.slug.into(),
            role: assistant_core::identity::Role::OrgAdmin,
            storage: self.storage.clone(),
        }
    }

    /// Submit an interactive web turn and return its run id and the response
    /// the browser would be reading.
    async fn start_turn(&self, text: &str) -> (Uuid, axum::response::Response) {
        let ctx = self.ctx();
        let orchestrator = match self.state.runtime(&ctx).await {
            Ok(runtime) => runtime.orchestrator.clone(),
            Err(_) => panic!("no runtime"),
        };
        let run_id = Uuid::new_v4();
        let lease = orchestrator
            .turn_registry()
            .try_begin(self.conversation, run_id, TurnMode::Shared)
            .expect("no other turn holds the conversation");
        // Web chat does not ask for approvals yet; the harness opts in.
        let opts = RunOptions::web_turn(TurnAnchor::Append)
            .with_user_message_id(Uuid::new_v4())
            .with_interactive(true);
        let turn = opts.submit_turn(
            text.to_string(),
            self.conversation,
            &ctx.auth,
            run_id,
            lease,
        );
        let (event_rx, turn_rx) = start_turn(orchestrator, turn);
        let response = stream_turn_to_sse(
            &ctx,
            &self.state,
            run_id,
            self.conversation,
            opts,
            text.to_string(),
            event_rx,
            turn_rx,
        )
        .await;
        (run_id, response)
    }

    /// Read the stream to its end.
    fn read(response: axum::response::Response) -> tokio::task::JoinHandle<Vec<Frame>> {
        tokio::spawn(async move {
            let bytes = body_bytes(response.into_body()).await;
            parse_sse(&String::from_utf8_lossy(&bytes))
        })
    }

    async fn pending(&self) -> InteractionRow {
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                let rows = self
                    .storage
                    .interaction_store()
                    .list_for_conversation(
                        &self.conversation.to_string(),
                        Some(InteractionStatus::Pending),
                    )
                    .await
                    .unwrap();
                if let Some(row) = rows.into_iter().next() {
                    return row;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the turn asks for approval")
    }

    async fn respond(&self, run: Uuid, id: &str, decision: &str) -> (StatusCode, Value) {
        let resp = app(self.state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/conversations/{}/turns/{run}/interactions/{id}/respond",
                        self.conversation
                    ))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"kind": "tool_approval", "decision": decision}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let body = body_bytes(resp.into_body()).await;
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }

    async fn replay(&self, run: Uuid) -> Vec<Frame> {
        let resp = app(self.state.clone())
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/conversations/{}/runs/{run}/events/stream?since=0",
                        self.conversation
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = tokio::time::timeout(Duration::from_secs(20), body_bytes(resp.into_body()))
            .await
            .expect("the replay ends with the run");
        parse_sse(&String::from_utf8_lossy(&bytes))
    }

    /// The text the model was sent after the tool call.
    async fn follow_up_text(&self) -> String {
        let requests = self.llm.received_requests().await.unwrap_or_default();
        let last = requests
            .iter()
            .rfind(|r| r.url.path() == "/api/chat")
            .expect("the model was asked");
        String::from_utf8_lossy(&last.body).into_owned()
    }

    async fn saved_rules(&self) -> Vec<assistant_storage::ToolPermissionRule> {
        self.storage
            .tool_permission_rule_store()
            .list_for_space()
            .await
            .unwrap()
    }

    /// Run one turn and answer its approval with `decision`.
    async fn answered_turn(&self, text: &str, decision: &str) -> (Uuid, Vec<Frame>) {
        let (run, response) = self.start_turn(text).await;
        let stream = Self::read(response);
        let asked = self.pending().await;
        let (status, _) = self.respond(run, &asked.id, decision).await;
        assert_eq!(status, StatusCode::OK, "{decision}");
        (
            run,
            tokio::time::timeout(Duration::from_secs(20), stream)
                .await
                .unwrap()
                .unwrap(),
        )
    }
}

fn resolved(frames: &[Frame]) -> &Value {
    &frames
        .iter()
        .find(|(event, _)| event == "interaction_resolved")
        .expect("an interaction_resolved frame")
        .1
}

#[tokio::test]
async fn allow_once_runs_the_tool_and_saves_nothing() {
    let h = Harness::new(&[], |_| {}).await;
    mount_script(&h.llm, &[&h.touch_marker()]).await;

    let (_, frames) = h.answered_turn("touch it", "allow_once").await;

    assert_eq!(
        events(&frames)
            .into_iter()
            .filter(|e| [
                "run_started",
                "interaction_requested",
                "interaction_resolved",
                "done"
            ]
            .contains(e))
            .collect::<Vec<_>>(),
        [
            "run_started",
            "interaction_requested",
            "interaction_resolved",
            "done"
        ]
    );
    assert!(h.marker().exists(), "the approved tool ran");
    assert_eq!(resolved(&frames)["outcome"], "resolved");
    assert!(resolved(&frames).get("rule_id").is_none());
    assert!(h.saved_rules().await.is_empty());
}

#[tokio::test]
async fn reject_once_denies_the_tool_and_tells_the_model() {
    let h = Harness::new(&[], |_| {}).await;
    mount_script(&h.llm, &[&h.touch_marker()]).await;

    let (_, frames) = h.answered_turn("touch it", "reject_once").await;

    assert!(events(&frames).contains(&"done"));
    assert!(!h.marker().exists(), "the denied tool did not run");
    assert!(
        h.follow_up_text()
            .await
            .contains("User denied execution of"),
        "the model saw the denial"
    );
}

#[tokio::test]
async fn an_unanswered_approval_expires_and_the_model_is_told() {
    let h = Harness::new(&[], |config| {
        config.permissions.interaction_timeout_secs = 1
    })
    .await;
    mount_script(&h.llm, &[&h.touch_marker()]).await;

    let (_, response) = h.start_turn("touch it").await;
    let frames = tokio::time::timeout(Duration::from_secs(30), Harness::read(response))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(resolved(&frames)["outcome"], "expired");
    assert!(events(&frames).contains(&"done"));
    assert!(!h.marker().exists());
    assert!(h.follow_up_text().await.contains("expired"));
}

#[tokio::test]
async fn allow_always_saves_the_rule_and_the_next_turn_runs_without_a_prompt() {
    let h = Harness::new(&[], |_| {}).await;
    let command = h.touch_marker();
    mount_script(&h.llm, &[&command, &command]).await;

    let (_, frames) = h.answered_turn("touch it", "allow_always").await;

    let rules = h.saved_rules().await;
    assert_eq!(rules.len(), 1, "one rule saved");
    assert_eq!(rules[0].user_id, "system");
    assert_eq!(
        resolved(&frames)["rule_id"],
        json!(rules[0].id.to_string()),
        "the resolution names the saved rule"
    );
    assert!(h.marker().exists());

    std::fs::remove_file(h.marker()).unwrap();
    let (_, response) = h.start_turn("touch it again").await;
    let frames = tokio::time::timeout(Duration::from_secs(30), Harness::read(response))
        .await
        .unwrap()
        .unwrap();
    assert!(
        !events(&frames).contains(&"interaction_requested"),
        "a saved rule skips the prompt: {:?}",
        events(&frames)
    );
    assert!(h.marker().exists(), "the second turn ran the tool");
}

#[tokio::test]
async fn an_answer_after_the_tab_closed_still_resumes_the_turn() {
    let h = Harness::new(&[], |_| {}).await;
    mount_script(&h.llm, &[&h.touch_marker()]).await;

    let (run, response) = h.start_turn("touch it").await;
    let asked = h.pending().await;
    drop(response); // the browser tab is gone

    let (status, _) = h.respond(run, &asked.id, "allow_once").await;
    assert_eq!(status, StatusCode::OK);

    let frames = h.replay(run).await;
    assert!(h.marker().exists(), "the turn resumed and ran the tool");
    let kinds = events(&frames);
    for expected in ["interaction_requested", "interaction_resolved", "done"] {
        assert!(kinds.contains(&expected), "{expected} in {kinds:?}");
    }
    assert!(
        kinds.iter().position(|e| *e == "interaction_resolved")
            < kinds.iter().position(|e| *e == "done")
    );
}
