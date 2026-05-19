//! Coverage tests for the `bash` builtin.
//!
//! Exercises the happy path, missing-command branch, working-dir, and
//! timeout-expired branch to lift `bash.rs` above its 23% baseline.

use std::collections::HashMap;

use assistant_core::ToolHandler;
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_tool_executor::builtins::bash::BashHandler;
use serde_json::json;
use tempfile::TempDir;

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
    }
}

fn params(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

#[tokio::test]
async fn bash_missing_command_returns_error() {
    let handler = BashHandler::new();
    let out = handler.run(params(&[]), &ctx()).await.unwrap();
    assert!(!out.success);
    assert!(out.content.contains("command"));
}

#[tokio::test]
async fn bash_echo_returns_stdout() {
    let handler = BashHandler::new();
    let out = handler
        .run(
            params(&[("command", json!("echo hello-from-bash"))]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(out.success, "{}", out.content);
    assert!(out.content.contains("hello-from-bash"));
    assert!(out.content.contains("Exit code: 0"));
}

#[tokio::test]
async fn bash_nonzero_exit_is_reported_but_not_an_error() {
    let handler = BashHandler::new();
    let out = handler
        .run(params(&[("command", json!("exit 7"))]), &ctx())
        .await
        .unwrap();
    // The handler returns success=true regardless of exit code; the
    // caller inspects the structured `exit_code` field.
    assert!(out.success);
    assert!(out.content.contains("Exit code: 7"));
}

#[tokio::test]
async fn bash_honors_working_dir() {
    let dir = TempDir::new().unwrap();
    let handler = BashHandler::new();
    let out = handler
        .run(
            params(&[
                ("command", json!("pwd")),
                ("working_dir", json!(dir.path().to_string_lossy())),
            ]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(out.success);
    // macOS may resolve /var/folders -> /private/var/folders; just check
    // the basename matches.
    let basename = dir.path().file_name().unwrap().to_string_lossy();
    assert!(out.content.contains(basename.as_ref()));
}

#[tokio::test]
async fn bash_timeout_returns_error() {
    let handler = BashHandler::new();
    let out = handler
        .run(
            params(&[("command", json!("sleep 5")), ("timeout_secs", json!(1))]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(!out.success);
    assert!(out.content.contains("timed out"));
}

#[tokio::test]
async fn bash_no_output_reports_explicit_no_output_marker() {
    let handler = BashHandler::new();
    let out = handler
        .run(params(&[("command", json!("true"))]), &ctx())
        .await
        .unwrap();
    assert!(out.success);
    assert!(out.content.contains("(no output)"));
}

#[tokio::test]
async fn bash_stderr_is_captured() {
    let handler = BashHandler::new();
    let out = handler
        .run(params(&[("command", json!("echo oops 1>&2"))]), &ctx())
        .await
        .unwrap();
    assert!(out.success);
    assert!(out.content.contains("stderr"));
    assert!(out.content.contains("oops"));
}
