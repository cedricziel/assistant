//! Smoke tests that spin up a real Ollama container via testcontainers and
//! exercise the full tool-calling loop end-to-end.
//!
//! Run with:
//!   cargo test -p assistant-integration-tests --test smoke -- --ignored
//!
//! Or use the Makefile target:
//!   make test-integration

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{skill::SkillSource, types::Interface, AssistantConfig};
use assistant_llm::{LlmClient, LlmClientConfig};
use assistant_runtime::Orchestrator;
use assistant_skills_executor::SkillExecutor;
use assistant_storage::{registry::SkillRegistry, StorageLayer};
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    GenericImage,
};
use uuid::Uuid;

const OLLAMA_PORT: u16 = 11434;
/// Smallest reliably capable model with native tool-calling support: ~934 MB.
const MODEL: &str = "qwen2.5:1.5b";

// ── Container helper ──────────────────────────────────────────────────────────

/// Start an Ollama container, pull the small model, and return the base URL.
async fn start_ollama() -> Result<(impl Drop, String)> {
    let container = GenericImage::new("ollama/ollama", "latest")
        .with_exposed_port(OLLAMA_PORT.tcp())
        .with_wait_for(WaitFor::message_on_stderr("Listening on"))
        .start()
        .await?;

    let host = container.get_host().await?;
    let port = container.get_host_port_ipv4(OLLAMA_PORT.tcp()).await?;
    let base_url = format!("http://{host}:{port}");

    // Pull the model — wait for it to finish before any test starts.
    pull_model(&base_url, MODEL).await?;

    Ok((container, base_url))
}

async fn pull_model(base_url: &str, model: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let resp = client
        .post(format!("{base_url}/api/pull"))
        .json(&serde_json::json!({ "name": model, "stream": false }))
        .send()
        .await?;

    anyhow::ensure!(resp.status().is_success(), "pull failed: {}", resp.status());
    Ok(())
}

// ── Shared test fixture ───────────────────────────────────────────────────────

struct Fixture {
    orchestrator: Arc<Orchestrator>,
    conversation_id: Uuid,
}

async fn build_fixture(base_url: &str) -> Result<Fixture> {
    let storage = Arc::new(StorageLayer::new_in_memory().await?);

    let mut registry = SkillRegistry::new(storage.pool.clone()).await?;

    // Load built-in skills from the repo `skills/` directory.
    let skills_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("skills");

    if skills_root.exists() {
        let dirs = vec![(skills_root.as_path(), SkillSource::Builtin)];
        registry.load_from_dirs(&dirs).await?;
    }

    let registry = Arc::new(registry);

    let llm_config = LlmClientConfig {
        model: MODEL.to_string(),
        base_url: base_url.to_string(),
        timeout_secs: 120,
    };
    let llm = Arc::new(LlmClient::new(llm_config)?);

    let config = AssistantConfig::default();
    let executor = Arc::new(SkillExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));

    let orchestrator = Arc::new(Orchestrator::new(llm, storage, registry, executor, &config));

    Ok(Fixture {
        orchestrator,
        conversation_id: Uuid::new_v4(),
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// memory-write → memory-read round-trip.
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_memory_round_trip() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("warn").try_init();

    let (_container, base_url) = start_ollama().await?;
    let f = build_fixture(&base_url).await?;

    // Store a value.
    f.orchestrator
        .run_turn(
            "Remember this: my favourite colour is indigo",
            f.conversation_id,
            Interface::Cli,
        )
        .await?;

    // Retrieve it.
    let read = f
        .orchestrator
        .run_turn(
            "What is my favourite colour?",
            f.conversation_id,
            Interface::Cli,
        )
        .await?;

    let answer = read.answer.to_lowercase();
    assert!(
        answer.contains("indigo"),
        "expected 'indigo' in answer, got: {answer}"
    );

    Ok(())
}

/// list-skills returns all 8 built-in skills.
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_list_skills() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("warn").try_init();

    let (_container, base_url) = start_ollama().await?;
    let f = build_fixture(&base_url).await?;

    let result = f
        .orchestrator
        .run_turn(
            "List all available skills",
            f.conversation_id,
            Interface::Cli,
        )
        .await?;

    let answer = result.answer.to_lowercase();
    for skill in &[
        "memory-read",
        "memory-write",
        "memory-search",
        "web-fetch",
        "shell-exec",
        "list-skills",
        "self-analyze",
        "schedule-task",
    ] {
        assert!(
            answer.contains(skill),
            "expected skill '{skill}' in answer, got: {answer}"
        );
    }

    Ok(())
}

/// ReAct loop terminates within max_iterations on a simple factual question.
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_react_terminates() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("warn").try_init();

    let (_container, base_url) = start_ollama().await?;
    let f = build_fixture(&base_url).await?;

    let result = f
        .orchestrator
        .run_turn("What is 2 + 2?", f.conversation_id, Interface::Cli)
        .await?;

    assert!(!result.answer.is_empty());
    Ok(())
}

/// self-analyze runs without panicking after some memory traces exist.
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_self_analyze_runs() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("warn").try_init();

    let (_container, base_url) = start_ollama().await?;
    let f = build_fixture(&base_url).await?;

    // Produce some traces for the memory-write skill.
    f.orchestrator
        .run_turn(
            "Store the value 'hello' under key 'smoke-test'",
            f.conversation_id,
            Interface::Cli,
        )
        .await?;

    // Trigger self-analyze — it should complete without error.
    let result = f
        .orchestrator
        .run_turn(
            "Analyse the memory-write skill and suggest improvements",
            f.conversation_id,
            Interface::Cli,
        )
        .await?;

    assert!(!result.answer.is_empty());
    Ok(())
}
