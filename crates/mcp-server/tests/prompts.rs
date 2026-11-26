//! Skill prompts (`prompts/list`, `prompts/get`) and `completion/complete`.

mod common;

use common::*;
use rmcp::model::{ContentBlock, ErrorCode, GetPromptRequestParams, PromptMessage, Role};
use serde_json::json;

const SKILL_TEMPLATE: &str = "skills://{name}/{category}/{+path}";

async fn client_with(names: &[&str]) -> Client {
    default_stubs()
        .skills(names.iter().map(|name| make_skill(name)).collect())
        .connect()
        .await
}

fn message_text(message: &PromptMessage) -> &str {
    match &message.content {
        ContentBlock::Text(text) => &text.text,
        other => panic!("expected a text block, got {other:?}"),
    }
}

#[tokio::test]
async fn initialize_advertises_prompts_and_completions() {
    let client = default_stubs().connect().await;
    let capabilities = &client.peer_info().expect("peer info").capabilities;
    assert!(capabilities.prompts.is_some(), "{capabilities:?}");
    assert!(capabilities.completions.is_some(), "{capabilities:?}");
}

#[tokio::test]
async fn prompts_list_has_one_prompt_per_skill_with_an_optional_input() {
    let client = client_with(&["alpha", "beta"]).await;

    let prompts = client.list_all_prompts().await.unwrap();

    let names: Vec<_> = prompts.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["alpha", "beta"]);
    let alpha = &prompts[0];
    assert_eq!(alpha.description.as_deref(), Some("test skill alpha"));
    let arguments = alpha.arguments.as_deref().expect("arguments");
    assert_eq!(arguments.len(), 1, "{arguments:?}");
    assert_eq!(arguments[0].name, "input");
    assert_ne!(arguments[0].required, Some(true));
}

#[tokio::test]
async fn prompts_get_without_input_returns_the_skill_body_as_a_user_message() {
    let client = client_with(&["alpha"]).await;

    let result = client
        .get_prompt(GetPromptRequestParams::new("alpha"))
        .await
        .unwrap();

    assert_eq!(result.messages.len(), 1, "{:?}", result.messages);
    assert_eq!(result.messages[0].role, Role::User);
    assert_eq!(message_text(&result.messages[0]), "body of alpha");
}

#[tokio::test]
async fn prompts_get_appends_the_input_to_the_skill_body() {
    let client = client_with(&["alpha"]).await;

    let result = client
        .get_prompt(
            GetPromptRequestParams::new("alpha").with_arguments(args(json!({"input": "do it"}))),
        )
        .await
        .unwrap();

    assert_eq!(result.messages.len(), 1, "{:?}", result.messages);
    assert_eq!(result.messages[0].role, Role::User);
    assert_eq!(message_text(&result.messages[0]), "body of alpha\n\ndo it");
}

#[tokio::test]
async fn prompts_get_unknown_prompt_returns_invalid_params() {
    let client = client_with(&["alpha"]).await;

    let error = client
        .get_prompt(GetPromptRequestParams::new("missing"))
        .await
        .unwrap_err();

    assert_eq!(code(error), ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn complete_prompt_names_by_prefix() {
    let client = client_with(&["example", "explain", "other"]).await;

    let values = client
        .complete_prompt_simple("example", "name", "ex")
        .await
        .unwrap();

    assert_eq!(values, ["example", "explain"]);
}

#[tokio::test]
async fn complete_skill_template_name_by_prefix() {
    let client = client_with(&["example", "explain", "other"]).await;

    let values = client
        .complete_resource_simple(SKILL_TEMPLATE, "name", "ex")
        .await
        .unwrap();

    assert_eq!(values, ["example", "explain"]);
}

#[tokio::test]
async fn complete_returns_at_most_100_values() {
    let names: Vec<String> = (0..150).map(|i| format!("skill-{i:03}")).collect();
    let client = client_with(&names.iter().map(String::as_str).collect::<Vec<_>>()).await;

    let completion = client
        .complete_resource_argument(SKILL_TEMPLATE, "name", "skill-", None)
        .await
        .unwrap();

    assert_eq!(completion.values.len(), 100);
    assert_eq!(completion.total, Some(150));
    assert_eq!(completion.has_more, Some(true));
}

#[tokio::test]
async fn complete_other_arguments_returns_nothing() {
    let client = client_with(&["example"]).await;

    let for_input = client
        .complete_prompt_simple("example", "input", "ex")
        .await
        .unwrap();
    let for_other_template = client
        .complete_resource_simple("files://{name}", "name", "ex")
        .await
        .unwrap();

    assert!(for_input.is_empty(), "{for_input:?}");
    assert!(for_other_template.is_empty(), "{for_other_template:?}");
}
