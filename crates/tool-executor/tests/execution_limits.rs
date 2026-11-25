//! Resource limits for `bash` and `process`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use assistant_core::ToolHandler;
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_core::types::permissions::LimitsConfig;
use assistant_tool_executor::builtins::bash::BashHandler;
use assistant_tool_executor::builtins::process::ProcessHandler;
use serde_json::{Value, json};
use tempfile::TempDir;

fn ctx() -> ExecutionContext {
    ExecutionContext {
        conversation_id: uuid::Uuid::new_v4(),
        agent_id: "limits-test".into(),
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

// -- bash --

#[tokio::test]
async fn bash_timeout_is_clamped_to_the_configured_maximum() {
    let handler = BashHandler::new().with_limits(LimitsConfig {
        bash_max_timeout_secs: 1,
        ..Default::default()
    });
    let started = Instant::now();
    let out = handler
        .run(
            params(&[
                ("command", json!("sleep 20")),
                ("timeout_secs", json!(86_400)),
            ]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(!out.success);
    assert!(
        out.content.contains("timed out after 1 seconds"),
        "{}",
        out.content
    );
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[tokio::test]
async fn bash_output_is_capped_at_the_configured_size_keeping_the_tail() {
    let handler = BashHandler::new().with_limits(LimitsConfig {
        bash_max_output_chars: 1_000,
        ..Default::default()
    });
    let out = handler
        .run(
            params(&[(
                "command",
                json!("head -c 3000000 /dev/zero | tr '\\0' a; printf END"),
            )]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(out.success, "{}", out.content);
    assert!(
        out.content.contains("showing last 1000 of 3000003 chars"),
        "{}",
        out.content
    );
    assert!(out.content.trim_end().ends_with("END"));
    assert!(out.content.len() < 5_000, "len {}", out.content.len());
}

#[tokio::test]
async fn bash_does_not_attach_images_outside_the_workspace_roots() {
    let inside = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let inside_png = inside.path().canonicalize().unwrap().join("in.png");
    let outside_png = outside.path().canonicalize().unwrap().join("out.png");
    std::fs::write(&inside_png, b"png-in").unwrap();
    std::fs::write(&outside_png, b"png-out").unwrap();

    let handler =
        BashHandler::new().with_extra_roots(vec![inside.path().to_string_lossy().into_owned()]);
    let out = handler
        .run(
            params(&[(
                "command",
                json!(format!(
                    "echo {} {}",
                    inside_png.display(),
                    outside_png.display()
                )),
            )]),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(out.success, "{}", out.content);
    let names: Vec<_> = out
        .attachments
        .iter()
        .map(|a| a.filename.as_str())
        .collect();
    assert_eq!(names, vec!["in.png"]);
}

// -- process --

async fn proc(handler: &ProcessHandler, pairs: &[(&str, Value)]) -> assistant_core::ToolOutput {
    handler.run(params(pairs), &ctx()).await.unwrap()
}

#[tokio::test]
async fn process_lines_are_truncated_at_the_per_line_cap() {
    let handler = ProcessHandler::new().with_limits(LimitsConfig {
        process_max_line_chars: 50,
        ..Default::default()
    });
    let started = proc(
        &handler,
        &[
            ("action", json!("start")),
            (
                "command",
                json!("head -c 100000 /dev/zero | tr '\\0' a; echo"),
            ),
        ],
    )
    .await;
    let sid = started.data.unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let log = proc(
        &handler,
        &[("action", json!("log")), ("session_id", json!(sid))],
    )
    .await;
    let stdout = log.data.unwrap()["stdout"].as_str().unwrap().to_string();
    assert!(stdout.starts_with(&"a".repeat(50)), "{stdout}");
    assert!(
        stdout.len() < 200,
        "line not capped: {} chars",
        stdout.len()
    );
    assert!(stdout.contains("[line truncated]"), "{stdout}");
}

#[tokio::test]
async fn process_start_fails_when_the_session_cap_is_reached() {
    let handler = ProcessHandler::new().with_limits(LimitsConfig {
        process_max_sessions: 2,
        ..Default::default()
    });
    for _ in 0..2 {
        let out = proc(
            &handler,
            &[("action", json!("start")), ("command", json!("sleep 30"))],
        )
        .await;
        assert!(out.success, "{}", out.content);
    }
    let third = proc(
        &handler,
        &[("action", json!("start")), ("command", json!("sleep 30"))],
    )
    .await;
    assert!(!third.success);
    assert!(third.content.contains("session limit"), "{}", third.content);
}

#[tokio::test]
async fn idle_process_sessions_expire_and_the_process_is_killed() {
    let handler = ProcessHandler::new().with_limits(LimitsConfig {
        process_ttl_secs: 1,
        ..Default::default()
    });
    let started = proc(
        &handler,
        &[("action", json!("start")), ("command", json!("sleep 60"))],
    )
    .await;
    let data = started.data.unwrap();
    let sid = data["session_id"].as_str().unwrap().to_string();
    let pid = data["pid"].as_u64().unwrap();

    tokio::time::sleep(Duration::from_millis(2_500)).await;

    let poll = proc(
        &handler,
        &[("action", json!("poll")), ("session_id", json!(sid))],
    )
    .await;
    assert!(!poll.success, "session should be gone: {}", poll.content);
    let alive = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .unwrap()
        .success();
    assert!(!alive, "process {pid} should have been killed");
}

#[tokio::test]
async fn polling_keeps_a_session_alive() {
    let handler = ProcessHandler::new().with_limits(LimitsConfig {
        process_ttl_secs: 2,
        ..Default::default()
    });
    let started = proc(
        &handler,
        &[("action", json!("start")), ("command", json!("sleep 60"))],
    )
    .await;
    let sid = started.data.unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    for _ in 0..4 {
        tokio::time::sleep(Duration::from_millis(1_000)).await;
        let poll = proc(
            &handler,
            &[
                ("action", json!("poll")),
                ("session_id", json!(sid.clone())),
            ],
        )
        .await;
        assert!(poll.success, "{}", poll.content);
    }
    proc(
        &handler,
        &[("action", json!("kill")), ("session_id", json!(sid))],
    )
    .await;
}
