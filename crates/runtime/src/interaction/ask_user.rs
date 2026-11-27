//! The `ask-user` tool: the model puts one to four questions to the turn's
//! owner and waits for the answers (ADR-0017, D5).
//!
//! It exists in the tool overlay of interactive web turns only (see
//! `Orchestrator::turn_overlay`). The answer arrives through the same
//! interaction channel as tool approvals.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::warn;

use assistant_core::tool::{ToolHandler, ToolOutput};
use assistant_core::types::conversation::ExecutionContext;
use assistant_storage::InteractionRow;

use crate::interaction::{InteractionBroker, InteractionOutcome, InteractionRequest};
use crate::orchestrator::OrchestratorEvent;

/// The tool's name.
pub const ASK_USER_TOOL: &str = "ask-user";

const KIND: &str = "question";
const MAX_QUESTIONS: usize = 4;
const MAX_OPTIONS: usize = 8;
const MAX_PROMPT_CHARS: usize = 500;
const MAX_LABEL_CHARS: usize = 100;
const MAX_DESCRIPTION_CHARS: usize = 300;

tokio::task_local! {
    static CALL: ToolCallScope;
}

/// What the orchestrator knows about the tool call being run, and a tool's
/// `run` cannot see: where the turn's events go and the provider's id for the
/// call. The orchestrator scopes each tool call with it.
#[derive(Debug, Clone, Default)]
pub struct ToolCallScope {
    pub event_sink: Option<mpsc::Sender<OrchestratorEvent>>,
    pub tool_call_id: Option<String>,
}

impl ToolCallScope {
    /// Run `fut` with `self` as the current call. Reaches everything awaited
    /// in `fut`, but not work moved to another task with `tokio::spawn`.
    pub async fn scope<F: std::future::Future>(self, fut: F) -> F::Output {
        CALL.scope(self, fut).await
    }

    /// The call being run, or `None` outside any tool call.
    pub fn current() -> Option<Self> {
        CALL.try_with(Clone::clone).ok()
    }
}

#[derive(Deserialize)]
struct QuestionInput {
    prompt: String,
    #[serde(default)]
    options: Vec<OptionInput>,
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    free_text: bool,
}

#[derive(Deserialize)]
struct OptionInput {
    label: String,
    description: Option<String>,
}

/// Asks the turn's owner through the [`InteractionBroker`].
pub struct AskUserTool {
    broker: Arc<InteractionBroker>,
}

impl AskUserTool {
    pub fn new(broker: Arc<InteractionBroker>) -> Self {
        Self { broker }
    }
}

/// Check `questions` and give every question and option its id. The error
/// names what is wrong, for the model to fix.
fn build_questions(params: &HashMap<String, Value>) -> Result<Value, String> {
    let raw = params
        .get("questions")
        .ok_or("'questions' is required")?
        .clone();
    let questions: Vec<QuestionInput> =
        serde_json::from_value(raw).map_err(|e| format!("'questions' is malformed: {e}"))?;
    if questions.is_empty() || questions.len() > MAX_QUESTIONS {
        return Err(format!(
            "ask 1 to {MAX_QUESTIONS} questions, not {}",
            questions.len()
        ));
    }
    let mut built = Vec::with_capacity(questions.len());
    for (qi, q) in questions.into_iter().enumerate() {
        let n = qi + 1;
        if q.prompt.trim().is_empty() {
            return Err(format!("question {n} has an empty prompt"));
        }
        if q.prompt.chars().count() > MAX_PROMPT_CHARS {
            return Err(format!(
                "question {n}: the prompt is longer than {MAX_PROMPT_CHARS} characters"
            ));
        }
        if q.options.len() > MAX_OPTIONS {
            return Err(format!(
                "question {n} has {} options; at most {MAX_OPTIONS} are allowed",
                q.options.len()
            ));
        }
        let mut options = Vec::with_capacity(q.options.len());
        for (oi, o) in q.options.iter().enumerate() {
            if o.label.trim().is_empty() {
                return Err(format!("question {n}: option {} has no label", oi + 1));
            }
            if o.label.chars().count() > MAX_LABEL_CHARS {
                return Err(format!(
                    "question {n}: option {} has a label longer than {MAX_LABEL_CHARS} characters",
                    oi + 1
                ));
            }
            let mut option = json!({"id": format!("o{}", oi + 1), "label": o.label});
            if let Some(description) = o.description.as_deref().filter(|d| !d.is_empty()) {
                if description.chars().count() > MAX_DESCRIPTION_CHARS {
                    return Err(format!(
                        "question {n}: option {} has a description longer than \
                         {MAX_DESCRIPTION_CHARS} characters",
                        oi + 1
                    ));
                }
                option["description"] = json!(description);
            }
            options.push(option);
        }
        let free_text = q.free_text || options.is_empty();
        built.push(json!({
            "id": format!("q{n}"),
            "prompt": q.prompt,
            "options": options,
            "multiple": q.multiple,
            "free_text": free_text,
        }));
    }
    Ok(Value::Array(built))
}

/// `{"answers": [...]}`: one object per question, in request order.
fn answers_observation(questions: &Value, response: &Value) -> String {
    let given = response["answers"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let answers: Vec<Value> = questions
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .map(|q| {
            let answer = given
                .iter()
                .find(|a| a["question_id"] == q["id"])
                .unwrap_or(&Value::Null);
            let label_of = |id: &str| {
                q["options"]
                    .as_array()
                    .and_then(|opts| opts.iter().find(|o| o["id"].as_str() == Some(id)))
                    .and_then(|o| o["label"].as_str())
                    .unwrap_or(id)
                    .to_owned()
            };
            let selected: Vec<String> = answer["selected_option_ids"]
                .as_array()
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .filter_map(Value::as_str)
                .map(label_of)
                .collect();
            json!({
                "question": q["prompt"],
                "selected": selected,
                "text": answer["text"].as_str().unwrap_or_default(),
            })
        })
        .collect();
    json!({ "answers": answers }).to_string()
}

/// Seconds the question stayed open before it expired. The row's two stamps
/// come from slightly different instants, so round to the nearest second
/// rather than truncating a timeout of 90 s to 89.
async fn open_seconds(broker: &InteractionBroker, id: &str) -> i64 {
    match broker.store().get(id).await {
        Ok(Some(InteractionRow {
            created_at,
            expires_at,
            ..
        })) => {
            let waited = expires_at - created_at;
            (waited.num_milliseconds() + 500) / 1000
        }
        _ => 0,
    }
}

#[async_trait]
impl ToolHandler for AskUserTool {
    fn name(&self) -> &str {
        ASK_USER_TOOL
    }

    fn description(&self) -> &str {
        "Ask the user one to four clarifying questions and wait for the answers. Use it when you \
         cannot continue without a decision only the user can make. Each question may offer up \
         to eight options, allow several to be chosen, and allow free text."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": MAX_QUESTIONS,
                    "description": "The questions to ask, in order.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "prompt": {
                                "type": "string",
                                "description": format!("The question (at most {MAX_PROMPT_CHARS} characters)."),
                            },
                            "options": {
                                "type": "array",
                                "maxItems": MAX_OPTIONS,
                                "description": "Answers to pick from. Leave empty for a free-text question.",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": {
                                            "type": "string",
                                            "description": format!("Short answer (at most {MAX_LABEL_CHARS} characters)."),
                                        },
                                        "description": {
                                            "type": "string",
                                            "description": format!("Optional detail (at most {MAX_DESCRIPTION_CHARS} characters)."),
                                        },
                                    },
                                    "required": ["label"],
                                },
                            },
                            "multiple": {
                                "type": "boolean",
                                "description": "Let the user choose more than one option. Default false.",
                            },
                            "free_text": {
                                "type": "boolean",
                                "description": "Let the user type an answer as well. Always on when there are no options. Default false.",
                            },
                        },
                        "required": ["prompt"],
                    },
                },
            },
            "required": ["questions"],
        })
    }

    fn requires_rich_client(&self) -> bool {
        true
    }

    async fn run(
        &self,
        params: HashMap<String, Value>,
        ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        let questions = match build_questions(&params) {
            Ok(questions) => questions,
            Err(problem) => return Ok(ToolOutput::error(format!("ask-user: {problem}"))),
        };
        let (Some(user), Some(run_id)) = (&ctx.user_id, ctx.run_id) else {
            return Ok(ToolOutput::error("ask-user: this turn has no user to ask."));
        };
        let scope = ToolCallScope::current().unwrap_or_default();
        let asked = self
            .broker
            .request(
                InteractionRequest {
                    conversation_id: ctx.conversation_id.to_string(),
                    run_id: run_id.to_string(),
                    user_id: user.0.clone(),
                    kind: KIND.into(),
                    tool_call_id: scope.tool_call_id,
                    request: json!({ "questions": questions }),
                    turn_deadline: None,
                },
                scope.event_sink.as_ref(),
            )
            .await;
        Ok(match asked {
            Ok((_, InteractionOutcome::Resolved { response, .. })) => {
                if response["declined"] == Value::Bool(true) {
                    ToolOutput::success("The user declined to answer.")
                } else {
                    ToolOutput::success(answers_observation(&questions, &response))
                }
            }
            Ok((id, InteractionOutcome::Expired)) => ToolOutput::success(format!(
                "No answer within {} s.",
                open_seconds(&self.broker, &id).await
            )),
            Ok((_, InteractionOutcome::Cancelled)) => {
                ToolOutput::success("The question was cancelled before the user answered.")
            }
            Err(e) => {
                warn!(error = %e, "ask-user could not reach the user");
                ToolOutput::error(format!("ask-user: could not ask the user: {e}"))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use uuid::Uuid;

    use assistant_core::identity::UserId;
    use assistant_core::types::conversation::Interface;
    use assistant_storage::{InMemoryInteractionStore, InteractionStatus, InteractionStore};

    use crate::interaction::{InteractionKinds, Responder};

    use super::*;

    struct Rig {
        tool: Arc<AskUserTool>,
        broker: Arc<InteractionBroker>,
        store: Arc<InMemoryInteractionStore>,
    }

    fn rig(timeout: Duration) -> Rig {
        let store = Arc::new(InMemoryInteractionStore::new());
        let broker = Arc::new(
            InteractionBroker::new(store.clone(), Arc::new(InteractionKinds::with_builtin()))
                .with_timeout(timeout),
        );
        Rig {
            tool: Arc::new(AskUserTool::new(broker.clone())),
            broker,
            store,
        }
    }

    fn ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "test".into(),
            turn: 0,
            interface: Interface::Web,
            interactive: true,
            run_id: Some(Uuid::new_v4()),
            allowed_tools: None,
            depth: 0,
            user_id: Some(UserId::from("alice")),
            org_id: None,
            space_id: None,
        }
    }

    fn params(questions: Value) -> HashMap<String, Value> {
        HashMap::from([("questions".to_owned(), questions)])
    }

    fn question(prompt: &str) -> Value {
        json!({"prompt": prompt, "options": [{"label": "yes"}, {"label": "no"}]})
    }

    async fn rows(store: &InMemoryInteractionStore, ctx: &ExecutionContext) -> Vec<InteractionRow> {
        store
            .list_for_conversation(&ctx.conversation_id.to_string(), None)
            .await
            .unwrap()
    }

    async fn assert_invalid(questions: Value, needle: &str) {
        let r = rig(Duration::from_secs(60));
        let ctx = ctx();
        let out = r.tool.run(params(questions), &ctx).await.unwrap();
        assert!(!out.success, "expected an error for {needle}");
        assert!(
            out.content.contains(needle),
            "'{}' should mention '{needle}'",
            out.content
        );
        assert!(
            rows(&r.store, &ctx).await.is_empty(),
            "an invalid call must not create an interaction"
        );
    }

    #[tokio::test]
    async fn rejects_no_questions_and_five_questions() {
        assert_invalid(json!([]), "1 to 4").await;
        let five: Vec<Value> = (0..5).map(|i| question(&format!("q{i}"))).collect();
        assert_invalid(Value::Array(five), "1 to 4").await;
    }

    #[tokio::test]
    async fn rejects_an_empty_prompt_and_a_501_character_prompt() {
        assert_invalid(json!([{"prompt": "  "}]), "empty prompt").await;
        assert_invalid(json!([{"prompt": "x".repeat(501)}]), "500").await;
    }

    #[tokio::test]
    async fn rejects_nine_options_and_a_101_character_label() {
        let nine: Vec<Value> = (0..9).map(|i| json!({"label": format!("o{i}")})).collect();
        assert_invalid(json!([{"prompt": "p", "options": nine}]), "at most 8").await;
        assert_invalid(
            json!([{"prompt": "p", "options": [{"label": "x".repeat(101)}]}]),
            "100",
        )
        .await;
        assert_invalid(
            json!([{"prompt": "p", "options": [{"label": "a", "description": "x".repeat(301)}]}]),
            "300",
        )
        .await;
    }

    #[tokio::test]
    async fn rejects_questions_that_are_not_a_list_of_questions() {
        assert_invalid(json!("which one?"), "malformed").await;
        let r = rig(Duration::from_secs(60));
        let out = r.tool.run(HashMap::new(), &ctx()).await.unwrap();
        assert!(!out.success);
    }

    #[tokio::test]
    async fn a_turn_without_a_user_or_run_cannot_ask() {
        let r = rig(Duration::from_secs(60));
        for mutate in [
            (|c: &mut ExecutionContext| c.user_id = None) as fn(&mut ExecutionContext),
            |c| c.run_id = None,
        ] {
            let mut ctx = ctx();
            mutate(&mut ctx);
            let out = r
                .tool
                .run(params(json!([question("Proceed?")])), &ctx)
                .await
                .unwrap();
            assert!(!out.success);
            assert!(rows(&r.store, &ctx).await.is_empty());
        }
    }

    /// Run the tool and wait until its interaction is pending.
    async fn ask(
        r: &Rig,
        ctx: &ExecutionContext,
        questions: Value,
    ) -> (
        InteractionRow,
        tokio::task::JoinHandle<anyhow::Result<ToolOutput>>,
    ) {
        let tool = r.tool.clone();
        let ctx_owned = ctx.clone();
        let call = ToolCallScope {
            event_sink: None,
            tool_call_id: Some("toolu_ask".into()),
        };
        let task =
            tokio::spawn(call.scope(async move { tool.run(params(questions), &ctx_owned).await }));
        loop {
            if let Some(row) = rows(&r.store, ctx).await.into_iter().next() {
                return (row, task);
            }
            tokio::task::yield_now().await;
        }
    }

    fn owner(ctx: &ExecutionContext) -> (String, String) {
        (
            ctx.conversation_id.to_string(),
            ctx.run_id.unwrap().to_string(),
        )
    }

    async fn answer(r: &Rig, ctx: &ExecutionContext, id: &str, response: Value) {
        let (conversation_id, run_id) = owner(ctx);
        r.broker
            .respond(
                &Responder {
                    user_id: "alice",
                    conversation_id: &conversation_id,
                    run_id: &run_id,
                },
                id,
                response,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn no_options_forces_free_text_and_the_server_assigns_ids() {
        let r = rig(Duration::from_secs(60));
        let ctx = ctx();
        let (row, _task) = ask(
            &r,
            &ctx,
            json!([
                {"prompt": "Which branch?"},
                {"prompt": "Which files?", "multiple": true,
                 "options": [{"label": "a.rs", "description": "the entry point"}, {"label": "b.rs"}]},
            ]),
        )
        .await;
        assert_eq!(row.kind, "question");
        assert_eq!(row.tool_call_id.as_deref(), Some("toolu_ask"));
        let qs = &row.request["questions"];
        assert_eq!(qs[0]["id"], "q1");
        assert_eq!(
            qs[0]["free_text"], true,
            "free text is forced without options"
        );
        assert_eq!(qs[1]["id"], "q2");
        assert_eq!(qs[1]["free_text"], false);
        assert_eq!(qs[1]["multiple"], true);
        assert_eq!(qs[1]["options"][1]["id"], "o2");
        assert_eq!(qs[1]["options"][0]["description"], "the entry point");
    }

    #[tokio::test]
    async fn the_observation_lists_answers_in_request_order() {
        let r = rig(Duration::from_secs(60));
        let ctx = ctx();
        let (row, task) = ask(
            &r,
            &ctx,
            json!([question("Proceed?"), {"prompt": "Which branch?"}]),
        )
        .await;
        // Answered out of order on purpose.
        answer(
            &r,
            &ctx,
            &row.id,
            json!({"answers": [
                {"question_id": "q2", "text": "main"},
                {"question_id": "q1", "selected_option_ids": ["o2"]},
            ]}),
        )
        .await;
        let out = task.await.unwrap().unwrap();
        assert!(out.success);
        let observed: Value = serde_json::from_str(&out.content).unwrap();
        assert_eq!(
            observed,
            json!({"answers": [
                {"question": "Proceed?", "selected": ["no"], "text": ""},
                {"question": "Which branch?", "selected": [], "text": "main"},
            ]})
        );
    }

    #[tokio::test]
    async fn declining_is_a_successful_observation() {
        let r = rig(Duration::from_secs(60));
        let ctx = ctx();
        let (row, task) = ask(&r, &ctx, json!([question("Proceed?")])).await;
        answer(&r, &ctx, &row.id, json!({"declined": true})).await;
        let out = task.await.unwrap().unwrap();
        assert!(out.success);
        assert_eq!(out.content, "The user declined to answer.");
    }

    #[tokio::test(start_paused = true)]
    async fn expiry_is_a_successful_observation_naming_the_wait() {
        let r = rig(Duration::from_secs(90));
        let ctx = ctx();
        let (row, task) = ask(&r, &ctx, json!([question("Proceed?")])).await;
        let out = task.await.unwrap().unwrap();
        assert!(out.success);
        assert_eq!(out.content, "No answer within 90 s.");
        let settled = r.store.get(&row.id).await.unwrap().unwrap();
        assert_eq!(settled.status, InteractionStatus::Expired);
    }
}
