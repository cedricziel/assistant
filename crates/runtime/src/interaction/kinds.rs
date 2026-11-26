//! The interaction kind registry (ADR-0017, D1).
//!
//! Every kind of interaction a turn can put to its owner registers an
//! [`InteractionKindHandler`]. The respond route, the push path and the
//! progress card go through [`InteractionKinds`], so none of them `match` on
//! a kind. Sibling changes add kinds (`surface_action`) by registering a
//! handler.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use assistant_storage::{InteractionRow, InteractionStatus};

/// Longest free-text answer, in characters.
pub const MAX_ANSWER_CHARS: usize = 4000;

/// Cap on the serialized `arguments` of a tool-approval request (D10).
pub const MAX_ARGUMENTS_BYTES: usize = 16 * 1024;

/// Cap on the serialized `action` plus `data_model` of a `surface_action`
/// answer, matching the post-turn route.
pub const MAX_SURFACE_ACTION_BYTES: usize = 64 * 1024;

const TRUNCATION_MARKER: &str = "…[truncated]";

// -- Types --

/// How an interaction ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionOutcome {
    /// The owner answered. `response` is the validated, kind-specific answer.
    Resolved {
        response: Value,
        rule_id: Option<String>,
    },
    /// The turn ended or was cancelled while the interaction was open.
    Cancelled,
    /// Nobody answered in time, or the waiter went away.
    Expired,
}

impl InteractionOutcome {
    /// The outcome a settled row records, or `None` while it is `pending`.
    pub fn from_row(row: &InteractionRow) -> Option<Self> {
        match row.status {
            InteractionStatus::Pending => None,
            InteractionStatus::Resolved => Some(Self::Resolved {
                response: row.response.clone().unwrap_or(Value::Null),
                rule_id: row.rule_id.clone(),
            }),
            InteractionStatus::Cancelled => Some(Self::Cancelled),
            InteractionStatus::Expired => Some(Self::Expired),
        }
    }

    /// The wire name: `resolved`, `cancelled` or `expired`.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Resolved { .. } => "resolved",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }
}

/// Title and body of the push that tells the owner an interaction waits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushCopy {
    pub title: String,
    pub body: String,
}

/// One kind of interaction.
pub trait InteractionKindHandler: Send + Sync {
    /// The `kind` tag on the wire and in the `interactions` table.
    fn kind(&self) -> &'static str;

    /// Whether `response` is an acceptable answer to `request`.
    fn validate_response(&self, request: &Value, response: &Value) -> Result<(), String>;

    /// What the LLM is told when the interaction ends with `outcome`.
    fn observation(&self, request: &Value, outcome: &InteractionOutcome) -> String;

    /// Copy for the push sent when the interaction goes unanswered.
    fn push_copy(&self, request: &Value) -> PushCopy;

    /// What the progress card says while the interaction waits.
    fn progress_label(&self) -> &'static str;

    /// Whether the kind renders a card in the timeline.
    fn renders_card(&self) -> bool;
}

// -- Registry --

/// The handlers of every registered kind.
#[derive(Default)]
pub struct InteractionKinds {
    handlers: HashMap<&'static str, Arc<dyn InteractionKindHandler>>,
}

impl InteractionKinds {
    /// A registry with `tool_approval`, `question` and `surface_action` registered.
    pub fn with_builtin() -> Self {
        let mut kinds = Self::default();
        kinds.register(Arc::new(ToolApprovalKind));
        kinds.register(Arc::new(QuestionKind));
        kinds.register(Arc::new(SurfaceActionKind));
        kinds
    }

    /// Register `handler`, replacing any handler of the same kind.
    pub fn register(&mut self, handler: Arc<dyn InteractionKindHandler>) {
        self.handlers.insert(handler.kind(), handler);
    }

    pub fn get(&self, kind: &str) -> Option<&Arc<dyn InteractionKindHandler>> {
        self.handlers.get(kind)
    }

    /// Validate `response` through the handler of `kind`. A kind nobody
    /// registered is an error.
    pub fn validate_response(
        &self,
        kind: &str,
        request: &Value,
        response: &Value,
    ) -> Result<(), String> {
        self.get(kind)
            .ok_or_else(|| format!("unknown interaction kind '{kind}'"))?
            .validate_response(request, response)
    }
}

// -- tool_approval --

/// A tool call the permission pipeline left to a human.
///
/// Request: `{tool_name, arguments, arguments_truncated, reason, options,
/// suggested_rule?}`. Response: `{decision}` with a decision from `options`.
pub struct ToolApprovalKind;

impl ToolApprovalKind {
    fn tool(request: &Value) -> &str {
        request["tool_name"].as_str().unwrap_or("the tool")
    }
}

impl InteractionKindHandler for ToolApprovalKind {
    fn kind(&self) -> &'static str {
        "tool_approval"
    }

    fn validate_response(&self, request: &Value, response: &Value) -> Result<(), String> {
        let decision = response["decision"]
            .as_str()
            .ok_or("a tool approval needs a 'decision'")?;
        let offered = request["options"]
            .as_array()
            .is_some_and(|opts| opts.iter().any(|o| o.as_str() == Some(decision)));
        if offered {
            Ok(())
        } else {
            Err(format!("decision '{decision}' was not offered"))
        }
    }

    fn observation(&self, request: &Value, outcome: &InteractionOutcome) -> String {
        let tool = Self::tool(request);
        match outcome {
            InteractionOutcome::Resolved { response, .. } => match response["decision"].as_str() {
                Some("allow_once" | "allow_always") => {
                    format!("The user approved the call to '{tool}'.")
                }
                _ => format!("The user denied the call to '{tool}', so it was not run."),
            },
            InteractionOutcome::Expired => format!(
                "The approval request for '{tool}' expired before the user answered, so it was not run."
            ),
            InteractionOutcome::Cancelled => {
                format!("The approval request for '{tool}' was cancelled, so it was not run.")
            }
        }
    }

    fn push_copy(&self, request: &Value) -> PushCopy {
        PushCopy {
            title: "Approval needed".into(),
            body: format!("The assistant wants to run '{}'.", Self::tool(request)),
        }
    }

    fn progress_label(&self) -> &'static str {
        "Waiting for your approval"
    }

    fn renders_card(&self) -> bool {
        true
    }
}

// -- question --

/// Questions the `ask-user` tool puts to the owner.
///
/// Request: `{questions: [{id, prompt, options: [{id, label, description?}],
/// multiple, free_text}]}`. Response: `{answers: [{question_id,
/// selected_option_ids, text?}]}`, or `{declined: true}`.
pub struct QuestionKind;

impl QuestionKind {
    fn questions(request: &Value) -> &[Value] {
        request["questions"].as_array().map_or(&[], Vec::as_slice)
    }

    fn validate_answer(question: &Value, answer: &Value) -> Result<(), String> {
        let selected: Vec<&str> = answer["selected_option_ids"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .filter_map(Value::as_str)
            .collect();
        let known = |id: &str| {
            question["options"]
                .as_array()
                .is_some_and(|opts| opts.iter().any(|o| o["id"].as_str() == Some(id)))
        };
        if let Some(bad) = selected.iter().find(|id| !known(id)) {
            return Err(format!("option '{bad}' does not exist"));
        }
        if selected.len() > 1 && question["multiple"] != Value::Bool(true) {
            return Err("only one option may be selected".into());
        }
        let text = answer["text"].as_str().filter(|t| !t.is_empty());
        if let Some(text) = text {
            if question["free_text"] != Value::Bool(true) {
                return Err("this question takes no free text".into());
            }
            if text.chars().count() > MAX_ANSWER_CHARS {
                return Err(format!("text is longer than {MAX_ANSWER_CHARS} characters"));
            }
        }
        if selected.is_empty() && text.is_none() {
            return Err("an answer needs a selection or text".into());
        }
        Ok(())
    }

    /// `Q: prompt` / `A: labels, text` lines for one answer.
    fn render_answer(question: &Value, answer: &Value) -> String {
        let label_of = |id: &str| {
            question["options"]
                .as_array()
                .and_then(|opts| opts.iter().find(|o| o["id"].as_str() == Some(id)))
                .and_then(|o| o["label"].as_str())
                .unwrap_or(id)
                .to_owned()
        };
        let mut parts: Vec<String> = answer["selected_option_ids"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .filter_map(Value::as_str)
            .map(label_of)
            .collect();
        parts.extend(
            answer["text"]
                .as_str()
                .filter(|t| !t.is_empty())
                .map(str::to_owned),
        );
        format!(
            "Q: {}\nA: {}",
            question["prompt"].as_str().unwrap_or_default(),
            parts.join(", ")
        )
    }
}

impl InteractionKindHandler for QuestionKind {
    fn kind(&self) -> &'static str {
        "question"
    }

    fn validate_response(&self, request: &Value, response: &Value) -> Result<(), String> {
        if response["declined"] == Value::Bool(true) {
            return Ok(());
        }
        let answers = response["answers"]
            .as_array()
            .filter(|a| !a.is_empty())
            .ok_or("a question needs at least one answer")?;
        let questions = Self::questions(request);
        for answer in answers {
            let id = answer["question_id"].as_str().unwrap_or_default();
            let question = questions
                .iter()
                .find(|q| q["id"].as_str() == Some(id))
                .ok_or_else(|| format!("question '{id}' does not exist"))?;
            Self::validate_answer(question, answer)?;
        }
        Ok(())
    }

    fn observation(&self, request: &Value, outcome: &InteractionOutcome) -> String {
        match outcome {
            InteractionOutcome::Resolved { response, .. } if response["declined"] == true => {
                "The user declined to answer.".into()
            }
            InteractionOutcome::Resolved { response, .. } => {
                let questions = Self::questions(request);
                let lines: Vec<String> = response["answers"]
                    .as_array()
                    .map_or(&[][..], Vec::as_slice)
                    .iter()
                    .filter_map(|a| {
                        let id = a["question_id"].as_str()?;
                        let q = questions.iter().find(|q| q["id"].as_str() == Some(id))?;
                        Some(Self::render_answer(q, a))
                    })
                    .collect();
                format!("The user answered:\n{}", lines.join("\n"))
            }
            InteractionOutcome::Expired => "The question expired before the user answered.".into(),
            InteractionOutcome::Cancelled => "The question was cancelled.".into(),
        }
    }

    fn push_copy(&self, request: &Value) -> PushCopy {
        PushCopy {
            title: "The assistant has a question".into(),
            body: Self::questions(request)
                .first()
                .and_then(|q| q["prompt"].as_str())
                .unwrap_or_default()
                .to_owned(),
        }
    }

    fn progress_label(&self) -> &'static str {
        "Waiting for your answer"
    }

    fn renders_card(&self) -> bool {
        true
    }
}

// -- surface_action --

/// An A2UI surface the turn waits on (`render-surface` with
/// `wait_for_action`).
///
/// Request: `{surface_id, tool_call_id, title, actions, send_data_model}`,
/// where `actions` are the action names the surface's Buttons declare.
/// Response: `{action: {name, context?, ..}, data_model?}`, or
/// `{declined: true}`. The surface itself is the card, so the kind renders
/// none.
pub struct SurfaceActionKind;

impl SurfaceActionKind {
    fn action_name(response: &Value) -> Option<&str> {
        response["action"]["name"].as_str()
    }
}

impl InteractionKindHandler for SurfaceActionKind {
    fn kind(&self) -> &'static str {
        "surface_action"
    }

    fn validate_response(&self, request: &Value, response: &Value) -> Result<(), String> {
        if response["declined"] == Value::Bool(true) {
            return Ok(());
        }
        if !response["action"].is_object() {
            return Err("a surface answer needs an 'action' object".into());
        }
        let name = Self::action_name(response).ok_or("the action needs a 'name'")?;
        let declared = request["actions"]
            .as_array()
            .is_some_and(|names| names.iter().any(|n| n.as_str() == Some(name)));
        if !declared {
            return Err(format!("the surface declares no action '{name}'"));
        }
        if response.get("data_model").is_some_and(|m| !m.is_object()) {
            return Err("'data_model' must be an object".into());
        }
        if response.to_string().len() > MAX_SURFACE_ACTION_BYTES {
            return Err(format!(
                "the answer is larger than {MAX_SURFACE_ACTION_BYTES} bytes"
            ));
        }
        Ok(())
    }

    fn observation(&self, request: &Value, outcome: &InteractionOutcome) -> String {
        match outcome {
            InteractionOutcome::Resolved { response, .. } if response["declined"] == true => {
                "The user dismissed the form without submitting it.".into()
            }
            InteractionOutcome::Resolved { response, .. } => {
                let mut answer = json!({"action": response["action"]});
                if request["send_data_model"] == Value::Bool(true)
                    && let (Some(model), Some(map)) =
                        (response.get("data_model"), answer.as_object_mut())
                {
                    map.insert("data_model".into(), model.clone());
                }
                format!("The user submitted the form: {answer}")
            }
            InteractionOutcome::Expired => "The user has not answered yet. The surface stays \
                open, and they can still answer it after this turn."
                .into(),
            InteractionOutcome::Cancelled => {
                "The user moved on before answering, so nothing was submitted.".into()
            }
        }
    }

    fn push_copy(&self, request: &Value) -> PushCopy {
        PushCopy {
            title: "Your input is needed".into(),
            body: request["title"].as_str().unwrap_or_default().to_owned(),
        }
    }

    fn progress_label(&self) -> &'static str {
        "Waiting for your input"
    }

    fn renders_card(&self) -> bool {
        false
    }
}

// -- Argument size cap --

/// Cut `arguments` to at most [`MAX_ARGUMENTS_BYTES`] of serialized JSON by
/// shortening long strings, halving the per-string budget until it fits.
/// Returns the arguments and whether anything was cut.
pub fn cap_arguments(arguments: &Value) -> (Value, bool) {
    let fits = |v: &Value| v.to_string().len() <= MAX_ARGUMENTS_BYTES;
    if fits(arguments) {
        return (arguments.clone(), false);
    }
    let mut budget = MAX_ARGUMENTS_BYTES / 2;
    while budget >= 8 {
        let cut = shorten_strings(arguments, budget);
        if fits(&cut) {
            return (cut, true);
        }
        budget /= 2;
    }
    (Value::String(TRUNCATION_MARKER.into()), true)
}

/// Cut every string longer than `budget` bytes (on a char boundary) and mark it.
fn shorten_strings(value: &Value, budget: usize) -> Value {
    match value {
        Value::String(s) if s.len() > budget => {
            let end = (0..=budget)
                .rev()
                .find(|&i| s.is_char_boundary(i))
                .unwrap_or(0);
            Value::String(format!("{}{TRUNCATION_MARKER}", &s[..end]))
        }
        Value::Array(items) => {
            Value::Array(items.iter().map(|v| shorten_strings(v, budget)).collect())
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), shorten_strings(v, budget)))
                .collect(),
        ),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn approval_request() -> Value {
        json!({
            "tool_name": "bash",
            "arguments": {"command": "git status"},
            "arguments_truncated": false,
            "reason": "no rule allows 'bash'",
            "options": ["allow_once", "reject_once"],
        })
    }

    fn question_request() -> Value {
        json!({"questions": [
            {"id": "q1", "prompt": "Which branch?", "multiple": false, "free_text": false,
             "options": [{"id": "o1", "label": "main"}, {"id": "o2", "label": "dev"}]},
            {"id": "q2", "prompt": "Which files?", "multiple": true, "free_text": true,
             "options": [{"id": "a", "label": "a.rs"}, {"id": "b", "label": "b.rs"}]},
        ]})
    }

    fn resolved(response: Value) -> InteractionOutcome {
        InteractionOutcome::Resolved {
            response,
            rule_id: None,
        }
    }

    // -- tool_approval --

    #[test]
    fn approval_accepts_only_offered_decisions() {
        let kinds = InteractionKinds::with_builtin();
        let req = approval_request();
        for ok in ["allow_once", "reject_once"] {
            assert_eq!(
                kinds.validate_response("tool_approval", &req, &json!({"decision": ok})),
                Ok(()),
                "{ok} is offered"
            );
        }
        for bad in [
            json!({"decision": "allow_always"}),
            json!({"decision": "maybe"}),
            json!({}),
            json!("allow_once"),
        ] {
            assert!(
                kinds
                    .validate_response("tool_approval", &req, &bad)
                    .is_err(),
                "{bad} must be refused"
            );
        }
    }

    #[test]
    fn approval_observation_tells_the_llm_what_happened() {
        let h = ToolApprovalKind;
        let req = approval_request();
        let say = |o| h.observation(&req, &o);
        assert!(say(resolved(json!({"decision": "reject_once"}))).contains("denied"));
        assert!(say(resolved(json!({"decision": "reject_always"}))).contains("denied"));
        assert!(say(resolved(json!({"decision": "allow_once"}))).contains("approved"));
        assert!(say(InteractionOutcome::Expired).contains("expired"));
        assert!(say(InteractionOutcome::Cancelled).contains("cancelled"));
        assert!(say(InteractionOutcome::Expired).contains("bash"));
    }

    #[test]
    fn approval_presentation() {
        let h = ToolApprovalKind;
        let copy = h.push_copy(&approval_request());
        assert_eq!(copy.title, "Approval needed");
        assert!(copy.body.contains("bash"), "body: {}", copy.body);
        assert_eq!(h.progress_label(), "Waiting for your approval");
        assert!(h.renders_card());
    }

    // -- question --

    #[test]
    fn question_accepts_a_well_formed_answer() {
        let kinds = InteractionKinds::with_builtin();
        let answer = json!({"answers": [
            {"question_id": "q1", "selected_option_ids": ["o1"]},
            {"question_id": "q2", "selected_option_ids": ["a", "b"], "text": "and c.rs"},
        ]});
        assert_eq!(
            kinds.validate_response("question", &question_request(), &answer),
            Ok(())
        );
        assert_eq!(
            kinds.validate_response("question", &question_request(), &json!({"declined": true})),
            Ok(()),
            "declining is always allowed"
        );
    }

    #[test]
    fn question_refuses_answers_that_do_not_fit() {
        let kinds = InteractionKinds::with_builtin();
        let long = "x".repeat(MAX_ANSWER_CHARS + 1);
        let cases = [
            (
                "unknown question",
                json!({"question_id": "zz", "selected_option_ids": []}),
            ),
            (
                "unknown option",
                json!({"question_id": "q1", "selected_option_ids": ["nope"]}),
            ),
            (
                "two selections for a single choice",
                json!({"question_id": "q1", "selected_option_ids": ["o1", "o2"]}),
            ),
            (
                "text where free text is off",
                json!({"question_id": "q1", "selected_option_ids": ["o1"], "text": "hi"}),
            ),
            (
                "oversized text",
                json!({"question_id": "q2", "selected_option_ids": [], "text": long}),
            ),
            (
                "empty answer",
                json!({"question_id": "q1", "selected_option_ids": []}),
            ),
        ];
        for (name, answer) in cases {
            assert!(
                kinds
                    .validate_response(
                        "question",
                        &question_request(),
                        &json!({"answers": [answer]})
                    )
                    .is_err(),
                "{name} must be refused"
            );
        }
        assert!(
            kinds
                .validate_response("question", &question_request(), &json!({"answers": []}))
                .is_err(),
            "no answers at all must be refused"
        );
    }

    #[test]
    fn question_observation_and_presentation() {
        let h = QuestionKind;
        let req = question_request();
        let answered = h.observation(
            &req,
            &resolved(json!({"answers": [
                {"question_id": "q1", "selected_option_ids": ["o2"]},
                {"question_id": "q2", "selected_option_ids": ["a"], "text": "and c.rs"},
            ]})),
        );
        for needle in ["Which branch?", "dev", "Which files?", "a.rs", "and c.rs"] {
            assert!(
                answered.contains(needle),
                "'{needle}' missing from: {answered}"
            );
        }
        assert!(
            h.observation(&req, &resolved(json!({"declined": true})))
                .contains("declined")
        );
        assert!(
            h.observation(&req, &InteractionOutcome::Expired)
                .contains("expired")
        );
        assert!(
            h.observation(&req, &InteractionOutcome::Cancelled)
                .contains("cancelled")
        );
        assert!(h.push_copy(&req).body.contains("Which branch?"));
        assert_eq!(h.progress_label(), "Waiting for your answer");
        assert!(h.renders_card());
    }

    // -- surface_action --

    fn surface_request() -> Value {
        json!({
            "surface_id": "s-1",
            "tool_call_id": "call-1",
            "title": "Trip form",
            "actions": ["submit_form", "cancel"],
            "send_data_model": true,
        })
    }

    #[test]
    fn surface_action_is_a_builtin_kind() {
        let kinds = InteractionKinds::with_builtin();
        assert!(kinds.get("surface_action").is_some());
    }

    #[test]
    fn surface_action_accepts_only_declared_actions() {
        let kinds = InteractionKinds::with_builtin();
        let req = surface_request();
        let check = |response: Value| kinds.validate_response("surface_action", &req, &response);
        assert_eq!(
            check(json!({"action": {"name": "submit_form", "context": {"a": 1}}})),
            Ok(())
        );
        assert_eq!(
            check(json!({"action": {"name": "cancel"}, "data_model": {"x": 1}})),
            Ok(())
        );
        assert_eq!(
            check(json!({"declined": true})),
            Ok(()),
            "decline is allowed"
        );
        for (name, bad) in [
            ("undeclared name", json!({"action": {"name": "nuke"}})),
            ("no name", json!({"action": {"context": {}}})),
            ("no action", json!({})),
            ("action is not an object", json!({"action": "submit_form"})),
            (
                "data model is not an object",
                json!({"action": {"name": "cancel"}, "data_model": [1]}),
            ),
            (
                "oversized context",
                json!({"action": {"name": "cancel", "context": {"k": "x".repeat(MAX_SURFACE_ACTION_BYTES)}}}),
            ),
        ] {
            assert!(check(bad).is_err(), "{name} must be refused");
        }
    }

    #[test]
    fn surface_action_observation_tells_the_llm_what_happened() {
        let h = SurfaceActionKind;
        let req = surface_request();
        let say = |o| h.observation(&req, &o);

        let submitted = say(resolved(json!({
            "action": {"name": "submit_form", "context": {"city": "Oslo"}},
            "data_model": {"guests": 2},
        })));
        for needle in ["submit_form", "Oslo", "guests"] {
            assert!(
                submitted.contains(needle),
                "'{needle}' missing: {submitted}"
            );
        }

        let mut no_model = surface_request();
        no_model["send_data_model"] = json!(false);
        let hidden = h.observation(
            &no_model,
            &resolved(json!({"action": {"name": "cancel"}, "data_model": {"secret": 1}})),
        );
        assert!(!hidden.contains("secret"), "data model withheld: {hidden}");

        assert!(say(resolved(json!({"declined": true}))).contains("dismissed"));
        assert!(say(InteractionOutcome::Expired).contains("not answered yet"));
        assert!(say(InteractionOutcome::Cancelled).contains("moved on"));
    }

    #[test]
    fn surface_action_presentation() {
        let h = SurfaceActionKind;
        let copy = h.push_copy(&surface_request());
        assert_eq!(copy.title, "Your input is needed");
        assert!(copy.body.contains("Trip form"), "body: {}", copy.body);
        assert_eq!(h.progress_label(), "Waiting for your input");
        assert!(!h.renders_card(), "the surface is the card");
    }

    // -- registry --

    #[test]
    fn unregistered_kind_fails_validation() {
        let kinds = InteractionKinds::with_builtin();
        let err = kinds
            .validate_response("future_thing", &json!({}), &json!({}))
            .unwrap_err();
        assert!(err.contains("future_thing"), "{err}");
        assert!(kinds.get("future_thing").is_none());
    }

    #[test]
    fn siblings_register_their_own_kind() {
        struct Silent;
        impl InteractionKindHandler for Silent {
            fn kind(&self) -> &'static str {
                "surface_action"
            }
            fn validate_response(&self, _: &Value, _: &Value) -> Result<(), String> {
                Ok(())
            }
            fn observation(&self, _: &Value, _: &InteractionOutcome) -> String {
                "done".into()
            }
            fn push_copy(&self, _: &Value) -> PushCopy {
                PushCopy {
                    title: "t".into(),
                    body: "b".into(),
                }
            }
            fn progress_label(&self) -> &'static str {
                "Waiting for your input"
            }
            fn renders_card(&self) -> bool {
                false
            }
        }
        let mut kinds = InteractionKinds::with_builtin();
        kinds.register(Arc::new(Silent));
        assert_eq!(
            kinds.validate_response("surface_action", &json!({}), &json!({})),
            Ok(())
        );
        assert!(
            !kinds
                .get("surface_action")
                .is_some_and(|h| h.renders_card())
        );
    }

    // -- cap_arguments --

    #[test]
    fn small_arguments_pass_through() {
        let args = json!({"command": "ls", "n": [1, 2, 3]});
        assert_eq!(cap_arguments(&args), (args, false));
    }

    #[test]
    fn large_arguments_are_cut_and_flagged() {
        let args = json!({"path": "/tmp/x", "content": "y".repeat(2 * 1024 * 1024)});
        let (capped, truncated) = cap_arguments(&args);
        assert!(truncated);
        assert!(capped.to_string().len() <= MAX_ARGUMENTS_BYTES);
        assert_eq!(capped["path"], "/tmp/x", "short values stay intact");
        assert!(
            capped["content"]
                .as_str()
                .is_some_and(|s| s.ends_with(TRUNCATION_MARKER)),
            "cut strings carry the marker"
        );
    }

    #[test]
    fn arguments_with_many_small_values_still_fit() {
        let args = json!({"items": (0..5000).map(|i| format!("item-{i}")).collect::<Vec<_>>()});
        let (capped, truncated) = cap_arguments(&args);
        assert!(truncated);
        assert!(capped.to_string().len() <= MAX_ARGUMENTS_BYTES);
    }

    #[test]
    fn truncation_respects_char_boundaries() {
        let args = json!({"text": "é".repeat(MAX_ARGUMENTS_BYTES)});
        let (capped, truncated) = cap_arguments(&args);
        assert!(truncated);
        assert!(capped.to_string().len() <= MAX_ARGUMENTS_BYTES);
    }
}
