//! Smoke tests that spin up a real Ollama container via testcontainers and
//! exercise the full tool-calling loop end-to-end.
//!
//! Run with:
//!   cargo test -p assistant-integration-tests --test smoke -- --ignored
//!
//! Or use the Makefile target:
//!   make test-integration

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, time::Duration};

use anyhow::Result;
use assistant_core::{
    AssistantConfig, ExecutionContext, MessageBus, TurnIdentity, types::Interface,
};
use assistant_llm_provider::ollama::client::{LlmClient, LlmClientConfig};
use assistant_runtime::Orchestrator;
use assistant_skills::SkillSource;
use assistant_storage::{StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;
use testcontainers::{
    ContainerAsync, GenericImage,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};
use uuid::Uuid;

const OLLAMA_PORT: u16 = 11434;
/// Smallest reliably capable model with native tool-calling support: ~934 MB.
const MODEL: &str = "qwen2.5:1.5b";

// ── Container helper ──────────────────────────────────────────────────────────

/// Start an Ollama container, pull the small model, and return the base URL.
async fn start_ollama_container() -> Result<(ContainerAsync<GenericImage>, String)> {
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

/// Resolve Ollama endpoint.
///
/// If `OLLAMA_BASE_URL` is set, use that endpoint directly (and assume the
/// configured model is already present). Otherwise, start an ephemeral
/// container and pull the model for the test run.
async fn resolve_ollama() -> Result<(Option<ContainerAsync<GenericImage>>, String)> {
    if let Ok(base_url) = env::var("OLLAMA_BASE_URL") {
        let base_url = base_url.trim().to_string();
        anyhow::ensure!(!base_url.is_empty(), "OLLAMA_BASE_URL must not be empty");
        return Ok((None, base_url));
    }

    if env::var("CI").is_ok() {
        anyhow::bail!("OLLAMA_BASE_URL must be set in CI for smoke tests");
    }

    let (container, base_url) = start_ollama_container().await?;
    Ok((Some(container), base_url))
}

async fn pull_model(base_url: &str, model: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
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
    executor: Arc<ToolExecutor>,
    conversation_id: Uuid,
}

async fn build_fixture(base_url: &str) -> Result<Fixture> {
    let storage = Arc::new(StorageLayer::new_in_memory().await?);

    let registry = SkillRegistry::new(storage.pool.clone()).await?;

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
        // CPU-only CI runners need plenty of headroom: a single qwen2.5:1.5b
        // chat call with the full system prompt + tool definitions (~5K tokens)
        // can take 2+ minutes.  The previous 120s value flaked the first slow
        // request every run.
        timeout_secs: 300,
        // Disable HTTP-level retries.  Slow CPU inference is deterministic,
        // not transient, so retrying the same prompt just burns another N min.
        // The orchestrator's tool-calling loop already provides iteration-level
        // "retries" with an evolving prompt.
        retry_config: assistant_llm_provider::retry::RetryConfig::disabled(),
    };
    let llm = Arc::new(LlmClient::new(llm_config)?);

    let mut config = AssistantConfig::default();
    // Disable learning so background skill-eval LLM calls don't compete with
    // the test's own LLM calls on the CPU-only CI Ollama instance.
    config.learning.enabled = false;
    // Cap iteration count so a confused 1.5B model can't spin up to the
    // default 80 iterations and blow the per-test timeout.
    config.llm.max_iterations = 6;
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));

    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orchestrator = Arc::new(Orchestrator::new(
        llm,
        storage.clone(),
        executor.clone(),
        registry.clone(),
        bus,
        &config,
    ));

    // Wire up subagent support.
    executor.set_subagent_runner(orchestrator.clone());

    Ok(Fixture {
        orchestrator,
        executor,
        conversation_id: Uuid::new_v4(),
    })
}

fn exec_ctx(f: &Fixture) -> ExecutionContext {
    ExecutionContext {
        conversation_id: f.conversation_id,
        agent_id: "default".to_string(),
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

// ── Tests ─────────────────────────────────────────────────────────────────────

/// memory-append persists a user preference and memory-get can read it.
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_memory_round_trip() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("warn").try_init();

    let f = build_fixture("http://localhost:11434").await?;
    let ctx = exec_ctx(&f);

    let mut append_params = HashMap::new();
    append_params.insert("target".to_string(), serde_json::json!("user"));
    append_params.insert(
        "text".to_string(),
        serde_json::json!("my favourite colour is indigo"),
    );
    let append = f
        .executor
        .execute("memory-append", append_params, &ctx)
        .await?;
    assert!(append.success, "memory-append failed: {}", append.content);

    let mut get_params = HashMap::new();
    get_params.insert("target".to_string(), serde_json::json!("user"));
    let read = f.executor.execute("memory-get", get_params, &ctx).await?;
    assert!(read.success, "memory-get failed: {}", read.content);

    assert!(
        read.content.to_lowercase().contains("indigo"),
        "expected indigo in memory-get output, got: {}",
        read.content
    );

    Ok(())
}

/// list-skills returns registered repo skills via the tool executor.
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_list_skills() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("warn").try_init();

    let f = build_fixture("http://localhost:11434").await?;
    let out = f
        .executor
        .execute("list-skills", HashMap::new(), &exec_ctx(&f))
        .await?;

    assert!(out.success, "list-skills failed: {}", out.content);
    assert!(
        out.content.to_lowercase().contains("coding-agent"),
        "expected list-skills output to include coding-agent"
    );
    assert!(
        out.content.to_lowercase().contains("playwright-cli"),
        "expected list-skills output to include playwright-cli"
    );

    Ok(())
}

/// Tool-calling loop terminates within max_iterations on a simple factual question.
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_tool_loop_terminates() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("warn").try_init();

    let (_container, base_url) = resolve_ollama().await?;
    let f = build_fixture(&base_url).await?;

    let result = tokio::time::timeout(Duration::from_secs(600), async {
        f.orchestrator
            .run_turn(
                "What is 2 + 2?",
                f.conversation_id,
                Interface::Cli,
                None,
                vec![],
                TurnIdentity::default(),
            )
            .await
    })
    .await
    .map_err(|_| anyhow::anyhow!("test_tool_loop_terminates timed out after 10 minutes"))??;

    assert!(!result.answer.is_empty());
    Ok(())
}

/// self-analyze runs without panicking after some memory traces exist.
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_self_analyze_runs() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("warn").try_init();

    let (_container, base_url) = resolve_ollama().await?;
    let f = build_fixture(&base_url).await?;

    tokio::time::timeout(Duration::from_secs(600), async {
        // Produce some traces for the memory-append tool.
        f.orchestrator
            .run_turn(
                "Use memory-append to store the value 'hello' under key 'smoke-test' in target 'memory'",
                f.conversation_id,
                Interface::Cli,
                None,
                vec![],
                TurnIdentity::default(),
            )
            .await?;

        // Trigger self-analyze — it should complete without error.
        let result = f
            .orchestrator
            .run_turn(
                "Analyse the memory-append skill and suggest improvements",
                f.conversation_id,
                Interface::Cli,
                None,
                vec![],
                TurnIdentity::default(),
            )
            .await?;

        assert!(!result.answer.is_empty());
        Ok(())
    })
    .await
    .map_err(|_| anyhow::anyhow!("test_self_analyze_runs timed out after 10 minutes"))?
}
