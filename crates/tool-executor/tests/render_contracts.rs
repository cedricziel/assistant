//! Producer tests for tool render data: every built-in that publishes a
//! `kind`-tagged payload must validate against its own `output_schema()`.

use std::collections::HashMap;

use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_core::{ToolHandler, ToolOutput};
use assistant_tool_executor::builtins::bash::BashHandler;
use assistant_tool_executor::builtins::file_read::FileReadHandler;
use assistant_tool_executor::builtins::web_fetch::WebFetchHandler;
use assistant_tool_executor::builtins::web_search::WebSearchHandler;
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
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

// -- web-fetch ---------------------------------------------------------------

async fn fetch(server: &MockServer, route: &str) -> (WebFetchHandler, ToolOutput) {
    let handler = WebFetchHandler::new();
    let out = handler
        .run(
            params(&[("url", json!(format!("{}{route}", server.uri())))]),
            &ctx(),
        )
        .await
        .unwrap();
    (handler, out)
}

#[tokio::test]
async fn web_fetch_publishes_link_preview_data() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            format!(
                "<html><head><title>  Hello page </title></head><body><p>{}</p></body></html>",
                "word ".repeat(200)
            ),
            "text/html; charset=utf-8",
        ))
        .mount(&server)
        .await;
    let (handler, out) = fetch(&server, "/page").await;
    let data = validated_data(&handler, &out);
    assert!(out.success);
    assert_eq!(data["kind"], "link_preview");
    assert_eq!(data["url"], format!("{}/page", server.uri()));
    assert_eq!(data["final_url"], format!("{}/page", server.uri()));
    assert_eq!(data["status"], 200);
    assert_eq!(data["title"], "Hello page");
    assert_eq!(data["content_type"], "text/html; charset=utf-8");
    let excerpt = data["excerpt"].as_str().unwrap();
    assert!(excerpt.starts_with("word word"));
    assert!(excerpt.chars().count() <= 280, "excerpt is too long");
}

#[tokio::test]
async fn web_fetch_caps_the_title_at_200_chars() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(format!(
            "<html><head><title>{}</title></head><body>x</body></html>",
            "é".repeat(500)
        )))
        .mount(&server)
        .await;
    let (handler, out) = fetch(&server, "/").await;
    let data = validated_data(&handler, &out);
    assert_eq!(data["title"].as_str().unwrap().chars().count(), 200);
}

#[tokio::test]
async fn web_fetch_redirect_reports_both_urls() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/old"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/new"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/new"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<p>moved</p>"))
        .mount(&server)
        .await;
    let (handler, out) = fetch(&server, "/old").await;
    let data = validated_data(&handler, &out);
    assert_eq!(data["url"], format!("{}/old", server.uri()));
    assert_eq!(data["final_url"], format!("{}/new", server.uri()));
    assert_eq!(data["status"], 200);
}

#[tokio::test]
async fn web_fetch_404_is_an_error_with_link_preview_data() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404).set_body_string("nope"))
        .mount(&server)
        .await;
    let (handler, out) = fetch(&server, "/missing").await;
    let data = validated_data(&handler, &out);
    assert!(!out.success, "a 404 must be an error");
    assert_eq!(data["kind"], "link_preview");
    assert_eq!(data["status"], 404);
    assert_eq!(data["final_url"], format!("{}/missing", server.uri()));
    assert!(
        data.get("excerpt").is_none(),
        "no excerpt for an error page"
    );
}

// -- file-read ---------------------------------------------------------------

async fn read_file(
    name: &str,
    content: &str,
    limit: Option<u64>,
) -> (FileReadHandler, ToolOutput, TempDir) {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join(name);
    tokio::fs::write(&file, content).await.unwrap();
    let handler =
        FileReadHandler::new().with_extra_roots(vec![dir.path().to_string_lossy().into_owned()]);
    let mut pairs = vec![("path", json!(file.to_string_lossy()))];
    if let Some(l) = limit {
        pairs.push(("limit", json!(l)));
    }
    let out = handler.run(params(&pairs), &ctx()).await.unwrap();
    (handler, out, dir)
}

#[tokio::test]
async fn file_read_publishes_code_file_data() {
    let (handler, out, dir) = read_file("main.rs", "fn main() {}\n", None).await;
    let data = validated_data(&handler, &out);
    assert!(out.success);
    assert_eq!(data["kind"], "code_file");
    assert_eq!(
        data["path"],
        dir.path().join("main.rs").to_string_lossy().as_ref()
    );
    assert_eq!(data["language"], "rust");
    assert_eq!(data["content"], "fn main() {}");
    assert_eq!(data["truncated"], false);
    assert_eq!(data["total_chars"], 13);
}

#[tokio::test]
async fn file_read_unknown_extension_has_no_language() {
    let (handler, out, _dir) = read_file("notes.xyz", "hello", None).await;
    let data = validated_data(&handler, &out);
    assert!(data.get("language").is_none());
}

#[tokio::test]
async fn file_read_over_limit_is_truncated() {
    let (handler, out, _dir) = read_file("big.txt", &"a".repeat(500), Some(100)).await;
    let data = validated_data(&handler, &out);
    assert_eq!(data["truncated"], true);
    assert_eq!(data["content"].as_str().unwrap().chars().count(), 100);
    assert_eq!(data["total_chars"], 500);
    assert!(
        out.content
            .contains("[Content truncated at 100 characters]")
    );
}

#[tokio::test]
async fn file_read_huge_limit_stays_within_the_render_budget() {
    let (handler, out, _dir) = read_file("big.txt", &"é".repeat(100_000), Some(1_000_000)).await;
    let data = validated_data(&handler, &out);
    let content = data["content"].as_str().unwrap();
    assert!(
        content.len() <= 24 * 1024,
        "content is {} bytes",
        content.len()
    );
    assert!(!content.contains('\u{fffd}'));
    assert_eq!(data["truncated"], true);
    assert_eq!(data["total_chars"], 100_000);
    assert!(
        out.content.matches('é').count() == 100_000,
        "the observation must still honour the limit"
    );
}
