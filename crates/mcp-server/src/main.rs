use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use assistant_core::{skill::SkillSource, AssistantConfig};
use assistant_llm::{LlmClient, LlmClientConfig};
use assistant_runtime::Orchestrator;
use assistant_skills_executor::SkillExecutor;
use assistant_storage::{registry::SkillRegistry, StorageLayer};
use tracing::{info, warn};

mod protocol;
mod server;

use protocol::JsonRpcRequest;

#[tokio::main]
async fn main() -> Result<()> {
    // MCP uses stdio — write all logs to stderr so they don't corrupt the
    // JSON-RPC stream on stdout.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "assistant_mcp_server=info,assistant=info".into()),
        )
        .init();

    info!("assistant-mcp-server starting");

    // ── Load config ───────────────────────────────────────────────────────────
    let config = load_config();

    // ── Storage ───────────────────────────────────────────────────────────────
    let db_path = config
        .storage
        .db_path
        .as_ref()
        .map(PathBuf::from)
        .or_else(assistant_storage::default_db_path)
        .context("Cannot determine database path")?;

    let storage = Arc::new(StorageLayer::new(&db_path).await?);

    // ── Skill registry ────────────────────────────────────────────────────────
    let mut registry = SkillRegistry::new(storage.pool.clone())
        .await
        .context("Failed to initialise skill registry")?;

    // Collect skill directories to scan.
    let mut skill_dirs: Vec<(std::path::PathBuf, SkillSource)> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            skill_dirs.push((exe_dir.join("skills"), SkillSource::Builtin));
        }
    }
    let user_skills_dir = dirs::home_dir()
        .map(|h| h.join(".assistant").join("skills"))
        .unwrap_or_else(|| PathBuf::from(".assistant/skills"));

    if user_skills_dir.exists() || !skill_dirs.is_empty() {
        skill_dirs.push((user_skills_dir.clone(), SkillSource::User));
    }

    let dirs_ref: Vec<(&Path, SkillSource)> = skill_dirs
        .iter()
        .map(|(p, s)| (p.as_path(), s.clone()))
        .collect();
    registry
        .load_from_dirs(&dirs_ref)
        .await
        .context("Failed to load skills")?;

    let registry = Arc::new(registry);
    info!(count = registry.list().await.len(), "Skills registered");

    // ── LLM client ────────────────────────────────────────────────────────────
    let llm = Arc::new(
        LlmClient::new(LlmClientConfig::from(&config.llm))
            .context("Failed to create LLM client")?,
    );

    // ── Skill executor ────────────────────────────────────────────────────────
    let executor = Arc::new(SkillExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));

    // ── Orchestrator ──────────────────────────────────────────────────────────
    let orchestrator = Arc::new(Orchestrator::new(
        llm,
        storage.clone(),
        registry.clone(),
        executor.clone(),
        &config,
    ));

    info!("MCP server ready — reading JSON-RPC from stdin");

    // ── Stdio JSON-RPC loop ───────────────────────────────────────────────────
    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                warn!("stdin read error: {e}");
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to parse JSON-RPC request: {e}");
                let err = protocol::JsonRpcResponse::err(None, -32700, "Parse error");
                let mut out = stdout.lock();
                serde_json::to_writer(&mut out, &err).ok();
                out.write_all(b"\n").ok();
                out.flush().ok();
                continue;
            }
        };

        let response = server::handle_request(
            request,
            registry.clone(),
            executor.clone(),
            orchestrator.clone(),
            user_skills_dir.clone(),
        )
        .await;

        let mut out = stdout.lock();
        serde_json::to_writer(&mut out, &response).ok();
        out.write_all(b"\n").ok();
        out.flush().ok();
    }

    info!("MCP server shutting down");
    Ok(())
}

fn load_config() -> AssistantConfig {
    let config_path = dirs::home_dir()
        .map(|h| h.join(".assistant").join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("config.toml"));

    if !config_path.exists() {
        return AssistantConfig::default();
    }

    match std::fs::read_to_string(&config_path) {
        Ok(raw) => toml::from_str::<AssistantConfig>(&raw).unwrap_or_else(|e| {
            warn!("Failed to parse config at {}: {e}", config_path.display());
            AssistantConfig::default()
        }),
        Err(e) => {
            warn!("Failed to read config at {}: {e}", config_path.display());
            AssistantConfig::default()
        }
    }
}
