//! Producer tests for tool render data: every built-in that publishes a
//! `kind`-tagged payload must validate against its own `output_schema()`.

use std::collections::HashMap;

use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_core::{ToolHandler, ToolOutput};
use assistant_tool_executor::builtins::bash::BashHandler;
use assistant_tool_executor::builtins::plan_update::PlanUpdateHandler;
use assistant_tool_executor::builtins::web_search::WebSearchHandler;
use serde_json::{Value, json};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ctx() -> ExecutionContext {
    ExecutionContext {
        conversation_id: uuid::Uuid::new_v4(),
        agent_id: "default".into(),
        turn: 0,
        interface: Interface::Cli,
        interactive: false,
        allowed_tools: None,
        depth: 0,
        user_id: None,
        org_id: None,
        space_id: None,
        run_id: None,
    }
}

fn params(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

/// The `data` of `out`, asserted to validate against the handler's schema.
fn validated_data(handler: &dyn ToolHandler, out: &ToolOutput) -> Value {
    let data = out.data.clone().expect("tool output carries render data");
    let schema = handler
        .output_schema()
        .expect("handler has an output schema");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    let errors: Vec<String> = validator
        .iter_errors(&data)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "data fails output_schema: {errors:?}\n{data}"
    );
    data
}

async fn run_bash(command: &str, timeout_secs: Option<u64>) -> (BashHandler, ToolOutput) {
    let handler = BashHandler::new();
    let mut pairs = vec![("command", json!(command))];
    if let Some(t) = timeout_secs {
        pairs.push(("timeout_secs", json!(t)));
    }
    let out = handler.run(params(&pairs), &ctx()).await.unwrap();
    (handler, out)
}

// -- bash --------------------------------------------------------------------

#[tokio::test]
async fn bash_success_publishes_terminal_data() {
    let (handler, out) = run_bash("echo hi", None).await;
    let data = validated_data(&handler, &out);
    assert!(out.success);
    assert_eq!(data["kind"], "terminal");
    assert_eq!(data["command"], "echo hi");
    assert_eq!(data["exit_code"], 0);
    assert_eq!(data["stdout"], "hi\n");
    assert_eq!(data["timed_out"], false);
    assert_eq!(data["stdout_truncated"], false);
}

#[tokio::test]
async fn bash_true_exits_zero() {
    let (handler, out) = run_bash("true", None).await;
    let data = validated_data(&handler, &out);
    assert!(out.success);
    assert_eq!(data["kind"], "terminal");
    assert_eq!(data["exit_code"], 0);
}

#[tokio::test]
async fn bash_nonzero_exit_is_a_failure_with_unchanged_observation() {
    let (handler, out) = run_bash("false", None).await;
    let data = validated_data(&handler, &out);
    assert!(!out.success, "a non-zero exit must report success = false");
    assert_eq!(data["exit_code"], 1);
    assert_eq!(
        out.content, "Exit code: 1\n\n(no output)",
        "the observation text is unchanged from before"
    );
}

#[tokio::test]
async fn bash_timeout_keeps_the_captured_tail() {
    let (handler, out) = run_bash("echo started; sleep 5", Some(1)).await;
    let data = validated_data(&handler, &out);
    assert!(!out.success);
    assert!(out.content.contains("timed out after 1 seconds"));
    assert_eq!(data["timed_out"], true);
    assert!(data["exit_code"].is_null(), "exit_code is null on timeout");
    assert!(
        data["stdout"].as_str().unwrap().contains("started"),
        "stdout tail was lost: {data}"
    );
}

#[tokio::test]
async fn bash_large_output_is_capped_for_rendering() {
    let (handler, out) = run_bash("head -c 100000 /dev/zero | tr '\\0' a", None).await;
    let data = validated_data(&handler, &out);
    assert_eq!(data["stdout_truncated"], true);
    let size = serde_json::to_vec(&data).unwrap().len();
    assert!(size < 32 * 1024, "data is {size} bytes");
}

#[tokio::test]
async fn bash_render_tail_cuts_on_a_char_boundary() {
    // 3-byte characters: a byte cut at 12 KiB would land inside one.
    let (handler, out) = run_bash("yes 'é€' | head -c 50000", None).await;
    let data = validated_data(&handler, &out);
    assert_eq!(data["stdout_truncated"], true);
    assert!(
        !data["stdout"].as_str().unwrap().contains('\u{fffd}'),
        "tail was cut inside a character"
    );
}

// -- web-search --------------------------------------------------------------

fn ddg_page(count: usize, snippet: &str) -> String {
    let results: String = (0..count)
        .map(|i| {
            format!(
                r#"<div class="result"><a class="result__a" href="https://example.com/p/{i}">Title {i}</a><a class="result__snippet">{snippet}</a></div>"#
            )
        })
        .collect();
    format!("<html><body>{results}</body></html>")
}

async fn search(body: String, query: &str, num: Option<u64>) -> (WebSearchHandler, ToolOutput) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;
    let handler = WebSearchHandler::new().with_base_url(server.uri());
    let mut pairs = vec![("query", json!(query))];
    if let Some(n) = num {
        pairs.push(("num_results", json!(n)));
    }
    let out = handler.run(params(&pairs), &ctx()).await.unwrap();
    (handler, out)
}

#[tokio::test]
async fn web_search_publishes_sources_data() {
    let (handler, out) = search(ddg_page(2, "A snippet"), "rust", None).await;
    let data = validated_data(&handler, &out);
    assert!(out.success);
    assert_eq!(data["kind"], "sources");
    assert_eq!(data["query"], "rust");
    let results = data["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["title"], "Title 0");
    assert_eq!(results[0]["url"], "https://example.com/p/0");
    assert_eq!(results[0]["domain"], "example.com");
    assert_eq!(results[0]["snippet"], "A snippet");
}

#[tokio::test]
async fn web_search_caps_results_and_snippets() {
    let long = "x".repeat(1000);
    let (handler, out) = search(ddg_page(30, &long), "many", Some(30)).await;
    let data = validated_data(&handler, &out);
    let results = data["results"].as_array().unwrap();
    assert_eq!(results.len(), 20);
    assert!(results[0]["snippet"].as_str().unwrap().chars().count() <= 300);
}

#[tokio::test]
async fn web_search_empty_results_publish_an_empty_list() {
    let (handler, out) = search("<html></html>".into(), "nothing", None).await;
    let data = validated_data(&handler, &out);
    assert!(out.success);
    assert_eq!(data["results"], json!([]));
    assert_eq!(data["query"], "nothing");
}

// -- plan-update -------------------------------------------------------------

async fn run_plan(args: &[(&str, Value)]) -> (PlanUpdateHandler, ToolOutput) {
    let handler = PlanUpdateHandler::new();
    let out = handler.run(params(args), &ctx()).await.unwrap();
    (handler, out)
}

fn steps(statuses: &[&str]) -> Value {
    Value::Array(
        statuses
            .iter()
            .enumerate()
            .map(|(i, s)| json!({"title": format!("Step {i}"), "status": s}))
            .collect(),
    )
}

#[tokio::test]
async fn plan_update_publishes_plan_data() {
    let (handler, out) = run_plan(&[
        ("title", json!("Ship it")),
        (
            "steps",
            steps(&[
                "completed",
                "completed",
                "completed",
                "in_progress",
                "pending",
            ]),
        ),
    ])
    .await;
    let data = validated_data(&handler, &out);
    assert!(out.success);
    assert_eq!(out.content, "Plan updated: 3/5 steps completed");
    assert_eq!(data["kind"], "plan");
    assert_eq!(data["title"], "Ship it");
    assert_eq!(data["steps"].as_array().unwrap().len(), 5);
    assert_eq!(data["steps"][3]["status"], "in_progress");
}

#[tokio::test]
async fn plan_update_without_title_omits_it() {
    let (handler, out) = run_plan(&[("steps", steps(&["pending"]))]).await;
    let data = validated_data(&handler, &out);
    assert!(data.get("title").is_none());
}

#[tokio::test]
async fn plan_update_accepts_an_empty_plan() {
    let (handler, out) = run_plan(&[("steps", json!([]))]).await;
    let data = validated_data(&handler, &out);
    assert!(out.success);
    assert_eq!(out.content, "Plan updated: 0/0 steps completed");
    assert_eq!(data["steps"], json!([]));
}

#[tokio::test]
async fn plan_update_rejects_an_unknown_status() {
    let (_, out) = run_plan(&[("steps", steps(&["done"]))]).await;
    assert!(!out.success);
    assert!(out.data.is_none());
    assert!(
        out.content.contains("status"),
        "error should name the field: {}",
        out.content
    );
}

#[tokio::test]
async fn plan_update_rejects_missing_or_malformed_steps() {
    for args in [
        vec![],
        vec![("steps", json!("nope"))],
        vec![("steps", json!([{"title": "no status"}]))],
        vec![("steps", json!([{"status": "pending"}]))],
        vec![("title", json!(5)), ("steps", json!([]))],
    ] {
        let (_, out) = run_plan(&args).await;
        assert!(!out.success, "{args:?} should be rejected");
        assert!(out.data.is_none());
    }
}

#[tokio::test]
async fn plan_update_enforces_the_size_budgets() {
    let too_many = steps(&vec!["pending"; 51]);
    let long_step = json!([{"title": "x".repeat(201), "status": "pending"}]);
    for (args, field) in [
        (vec![("steps", too_many)], "steps"),
        (vec![("steps", long_step)], "title"),
        (
            vec![("title", json!("y".repeat(121))), ("steps", json!([]))],
            "title",
        ),
    ] {
        let (_, out) = run_plan(&args).await;
        assert!(!out.success, "{field} budget violation must be rejected");
        assert!(out.data.is_none());
        assert!(out.content.contains(field), "{}", out.content);
    }
}

#[tokio::test]
async fn plan_update_accepts_the_budget_limits() {
    let max_steps: Vec<Value> = (0..50)
        .map(|_| json!({"title": "x".repeat(200), "status": "skipped"}))
        .collect();
    let (handler, out) = run_plan(&[
        ("title", json!("y".repeat(120))),
        ("steps", Value::Array(max_steps)),
    ])
    .await;
    let data = validated_data(&handler, &out);
    let size = serde_json::to_vec(&data).unwrap().len();
    assert!(size < 32 * 1024, "plan data is {size} bytes");
}

#[tokio::test]
async fn plan_update_is_registered_by_the_executor() {
    use std::sync::Arc;

    use assistant_core::types::agent::OrgConfig;
    use assistant_llm_provider::ScriptedLlmProvider;
    use assistant_storage::StorageLayer;
    use assistant_storage::registry::SkillRegistry;
    use assistant_tool_executor::ToolExecutor;

    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let executor = ToolExecutor::new(
        storage,
        Arc::new(ScriptedLlmProvider::new()),
        registry,
        Arc::new(OrgConfig::default()),
    );
    assert!(executor.handler("plan-update").is_some());
    assert!(
        executor.to_specs().iter().any(|s| s.name == "plan-update"),
        "plan-update missing from tool definitions"
    );
}
