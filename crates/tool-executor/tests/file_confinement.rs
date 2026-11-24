//! File tools refuse paths outside the workspace roots, independent of the
//! orchestrator's permission pipeline (the MCP server and workflows call the
//! executor directly).

use std::collections::HashMap;
use std::path::Path;

use assistant_core::ToolHandler;
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_tool_executor::builtins::file_edit::FileEditHandler;
use assistant_tool_executor::builtins::file_glob::FileGlobHandler;
use assistant_tool_executor::builtins::file_read::FileReadHandler;
use assistant_tool_executor::builtins::file_write::FileWriteHandler;
use serde_json::{Value, json};
use tempfile::TempDir;

fn ctx() -> ExecutionContext {
    ExecutionContext {
        conversation_id: uuid::Uuid::new_v4(),
        agent_id: "confinement-test".into(),
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

fn params(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn root_of(dir: &TempDir) -> String {
    dir.path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

fn assert_refused(out: &assistant_core::ToolOutput) {
    assert!(!out.success, "expected refusal, got: {}", out.content);
    assert!(
        out.content.contains("outside the allowed workspace roots"),
        "{}",
        out.content
    );
}

#[tokio::test]
async fn file_read_inside_extra_root_succeeds_and_outside_is_refused() {
    let ws = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    std::fs::write(ws.path().join("in.txt"), "inside").unwrap();
    std::fs::write(outside.path().join("out.txt"), "outside").unwrap();
    let h = FileReadHandler::new().with_extra_roots(vec![root_of(&ws)]);

    let ok = h
        .run(
            params(&[("path", json!(format!("{}/in.txt", root_of(&ws))))]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(ok.success, "{}", ok.content);

    let denied = h
        .run(
            params(&[("path", json!(format!("{}/out.txt", root_of(&outside))))]),
            &ctx(),
        )
        .await
        .unwrap();
    assert_refused(&denied);
    assert!(
        !denied.content.contains("outside\n"),
        "must not leak file content"
    );
}

#[tokio::test]
async fn file_read_refuses_traversal_out_of_the_root() {
    let ws = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    std::fs::write(outside.path().join("out.txt"), "outside").unwrap();
    let h = FileReadHandler::new().with_extra_roots(vec![root_of(&ws)]);
    let sneaky = format!(
        "{}/../{}/out.txt",
        root_of(&ws),
        outside.path().file_name().unwrap().to_string_lossy()
    );
    assert_refused(
        &h.run(params(&[("path", json!(sneaky))]), &ctx())
            .await
            .unwrap(),
    );
}

#[cfg(unix)]
#[tokio::test]
async fn file_read_refuses_symlink_escape() {
    let ws = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    std::fs::write(outside.path().join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(outside.path(), Path::new(&root_of(&ws)).join("link")).unwrap();
    let h = FileReadHandler::new().with_extra_roots(vec![root_of(&ws)]);
    let via_link = format!("{}/link/secret.txt", root_of(&ws));
    assert_refused(
        &h.run(params(&[("path", json!(via_link))]), &ctx())
            .await
            .unwrap(),
    );
}

#[tokio::test]
async fn file_write_outside_roots_is_refused_and_creates_nothing() {
    let ws = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let target = Path::new(&root_of(&outside)).join("nested/new.txt");
    let h = FileWriteHandler::new().with_extra_roots(vec![root_of(&ws)]);
    let out = h
        .run(
            params(&[
                ("path", json!(target.to_string_lossy())),
                ("content", json!("x")),
            ]),
            &ctx(),
        )
        .await
        .unwrap();
    assert_refused(&out);
    assert!(!target.exists());
    assert!(
        !target.parent().unwrap().exists(),
        "parent dirs must not be created"
    );
}

#[tokio::test]
async fn file_edit_outside_roots_is_refused() {
    let ws = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let file = outside.path().join("e.txt");
    std::fs::write(&file, "old").unwrap();
    let h = FileEditHandler::new().with_extra_roots(vec![root_of(&ws)]);
    let out = h
        .run(
            params(&[
                ("path", json!(file.to_string_lossy())),
                ("old_string", json!("old")),
                ("new_string", json!("new")),
            ]),
            &ctx(),
        )
        .await
        .unwrap();
    assert_refused(&out);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "old");
}

#[tokio::test]
async fn file_glob_refuses_patterns_outside_roots() {
    let ws = TempDir::new().unwrap();
    let h = FileGlobHandler::new().with_extra_roots(vec![root_of(&ws)]);
    assert_refused(
        &h.run(params(&[("pattern", json!("/etc/*"))]), &ctx())
            .await
            .unwrap(),
    );
}

#[cfg(unix)]
#[tokio::test]
async fn file_glob_omits_matches_that_resolve_outside_roots() {
    let ws = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    std::fs::write(ws.path().join("kept.txt"), "k").unwrap();
    std::fs::write(outside.path().join("leak.txt"), "l").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("leak.txt"),
        Path::new(&root_of(&ws)).join("leak.txt"),
    )
    .unwrap();
    let h = FileGlobHandler::new().with_extra_roots(vec![root_of(&ws)]);
    let out = h
        .run(
            params(&[("pattern", json!(format!("{}/*.txt", root_of(&ws))))]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(out.success, "{}", out.content);
    assert!(out.content.contains("kept.txt"));
    assert!(!out.content.contains("leak.txt"), "{}", out.content);
}
