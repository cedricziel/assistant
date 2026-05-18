//! Coverage tests for file/memory builtins.
//!
//! Each tool is exercised end-to-end through `ToolHandler::run` to
//! lift `crates/tool-executor/src/builtins/*.rs` above the 80% line
//! coverage floor. The handlers are constructed directly (most don't
//! need a `StorageLayer`); a few that do are skipped here in favour
//! of inline tests in their own modules.

use std::collections::HashMap;

use assistant_core::ToolHandler;
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_tool_executor::builtins::file_edit::FileEditHandler;
use assistant_tool_executor::builtins::file_glob::FileGlobHandler;
use assistant_tool_executor::builtins::file_read::FileReadHandler;
use assistant_tool_executor::builtins::file_write::FileWriteHandler;
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

// ── file_read ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn file_read_returns_text_content() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("hello.txt");
    tokio::fs::write(&path, "hello world").await.unwrap();

    let handler = FileReadHandler::new();
    let out = handler
        .run(params(&[("path", json!(path.to_string_lossy()))]), &ctx())
        .await
        .unwrap();
    assert!(out.success);
    assert!(out.content.contains("hello world"));
}

#[tokio::test]
async fn file_read_missing_path_returns_error() {
    let handler = FileReadHandler::new();
    let out = handler.run(params(&[]), &ctx()).await.unwrap();
    assert!(!out.success);
    assert!(out.content.contains("path"));
}

#[tokio::test]
async fn file_read_nonexistent_file_returns_error() {
    let handler = FileReadHandler::new();
    let out = handler
        .run(params(&[("path", json!("/no/such/file/12345"))]), &ctx())
        .await
        .unwrap();
    assert!(!out.success);
    assert!(out.content.contains("Failed to read"));
}

#[tokio::test]
async fn file_read_honors_limit_and_appends_truncation_marker() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("long.txt");
    let body = "a".repeat(20_000);
    tokio::fs::write(&path, &body).await.unwrap();

    let handler = FileReadHandler::new();
    let out = handler
        .run(
            params(&[
                ("path", json!(path.to_string_lossy())),
                ("limit", json!(100)),
            ]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(out.success);
    assert!(
        out.content
            .contains("[Content truncated at 100 characters]")
    );
}

// ── file_write ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn file_write_creates_file_with_content() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("out.txt");

    let handler = FileWriteHandler::new();
    let out = handler
        .run(
            params(&[
                ("path", json!(path.to_string_lossy())),
                ("content", json!("hello")),
            ]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(out.success, "{}", out.content);
    let written = tokio::fs::read_to_string(&path).await.unwrap();
    assert_eq!(written, "hello");
}

#[tokio::test]
async fn file_write_missing_required_returns_error() {
    let handler = FileWriteHandler::new();
    // No params at all.
    let out = handler.run(params(&[]), &ctx()).await.unwrap();
    assert!(!out.success);
}

// ── file_edit ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn file_edit_replaces_matching_text() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("doc.md");
    tokio::fs::write(&path, "alpha bravo charlie")
        .await
        .unwrap();

    let handler = FileEditHandler::new();
    let out = handler
        .run(
            params(&[
                ("path", json!(path.to_string_lossy())),
                ("old_string", json!("bravo")),
                ("new_string", json!("BRAVO")),
            ]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(out.success, "{}", out.content);
    let body = tokio::fs::read_to_string(&path).await.unwrap();
    assert_eq!(body, "alpha BRAVO charlie");
}

#[tokio::test]
async fn file_edit_missing_match_returns_error_output() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("doc.md");
    tokio::fs::write(&path, "alpha bravo").await.unwrap();

    let handler = FileEditHandler::new();
    let out = handler
        .run(
            params(&[
                ("path", json!(path.to_string_lossy())),
                ("old_string", json!("ZEBRA")),
                ("new_string", json!("ZULU")),
            ]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(!out.success);
}

// ── file_glob ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn file_glob_finds_matching_files() {
    let dir = TempDir::new().unwrap();
    tokio::fs::write(dir.path().join("a.txt"), "1")
        .await
        .unwrap();
    tokio::fs::write(dir.path().join("b.txt"), "2")
        .await
        .unwrap();
    tokio::fs::write(dir.path().join("c.md"), "3")
        .await
        .unwrap();

    let handler = FileGlobHandler::new();
    let pattern = format!("{}/*.txt", dir.path().to_string_lossy());
    let out = handler
        .run(params(&[("pattern", json!(pattern))]), &ctx())
        .await
        .unwrap();
    assert!(out.success, "{}", out.content);
    assert!(out.content.contains("a.txt"));
    assert!(out.content.contains("b.txt"));
    assert!(!out.content.contains("c.md"));
}

#[tokio::test]
async fn file_glob_missing_pattern_returns_error() {
    let handler = FileGlobHandler::new();
    let out = handler.run(params(&[]), &ctx()).await.unwrap();
    assert!(!out.success);
}
