use std::io::{self, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use std::time::Duration;

use anyhow::{Context, Result};
use assistant_core::{
    AssistantConfig, EmbeddingConfig, EmbeddingProviderKind, Interface, LlmProviderKind,
    MemoryLoader, MessageBus,
};
use assistant_llm::{
    EmbeddingProvider, LlmEmbedder, LlmProvider, VoyageConfig, VoyageEmbedder,
    WithEmbeddingOverride,
};
use assistant_provider_anthropic::AnthropicProvider;
use assistant_provider_moonshot::MoonshotProvider;
use assistant_provider_ollama::{OllamaConfig, OllamaProvider};
use assistant_provider_openai::{OpenAIProvider, OpenAIProviderConfig};
use assistant_runtime::{
    init_tracing, orchestrator::ConfirmationCallback, scheduler::spawn_scheduler,
    start_conversation_context, Orchestrator,
};
use assistant_skills::SkillSource;
use assistant_storage::{registry::SkillRegistry, RefinementStatus, StorageLayer};
use assistant_tool_executor::{install_skill_from_source, ToolExecutor};
use clap::{Parser, Subcommand};
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

// ── Argument parsing ──────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "assistant", about = "Local AI assistant")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Wipe all assistant data (database, memory files, daily notes) and
    /// re-seed fresh defaults. The next run starts completely clean.
    Reset {
        /// Skip the confirmation prompt (useful for scripts).
        #[arg(short, long)]
        yes: bool,
    },
    /// Run the MCP (Model Context Protocol) server over stdio.
    ///
    /// Exposes assistant skills as JSON-RPC 2.0 tools to Claude Code and other
    /// MCP clients. All logging goes to stderr; stdout is reserved for JSON-RPC.
    #[cfg(feature = "mcp")]
    Mcp,
    /// Run only the Slack interface (no interactive REPL).
    ///
    /// Requires Slack bot_token and app_token configured in ~/.assistant/config.toml.
    #[cfg(feature = "slack")]
    Slack,
    /// Run only the Mattermost interface (no interactive REPL).
    ///
    /// Requires Mattermost server_url and token configured in ~/.assistant/config.toml.
    #[cfg(feature = "mattermost")]
    Mattermost,
}

// ── CLI confirmation callback ─────────────────────────────────────────────────

/// Implements `ConfirmationCallback` by printing a `[y/N]` prompt to stdout
/// and reading a line from stdin.
struct CliConfirmation;

impl ConfirmationCallback for CliConfirmation {
    fn confirm(&self, skill_name: &str, params: &serde_json::Value) -> bool {
        let params_str = serde_json::to_string_pretty(params).unwrap_or_default();
        print!(
            "\nTool '{}' requires confirmation.\nParams: {}\nProceed? [y/N] ",
            skill_name, params_str
        );
        io::stdout().flush().ok();

        let mut buf = String::new();
        if io::stdin().read_line(&mut buf).is_err() {
            return false;
        }
        matches!(buf.trim().to_lowercase().as_str(), "y" | "yes")
    }
}

// ── Config loading ────────────────────────────────────────────────────────────

enum ConfigLoadMessage {
    Info(String),
    Warn(String),
}

fn load_config_messages(config_path: &Path) -> (AssistantConfig, Vec<ConfigLoadMessage>) {
    if !config_path.exists() {
        return (AssistantConfig::default(), Vec::new());
    }

    let mut messages = Vec::new();
    let raw = match std::fs::read_to_string(config_path) {
        Ok(s) => s,
        Err(e) => {
            messages.push(ConfigLoadMessage::Warn(format!(
                "Failed to read config at {}: {e}",
                config_path.display()
            )));
            return (AssistantConfig::default(), messages);
        }
    };

    match toml::from_str::<AssistantConfig>(&raw) {
        Ok(cfg) => {
            messages.push(ConfigLoadMessage::Info(format!(
                "Loaded config from {}",
                config_path.display()
            )));
            (cfg, messages)
        }
        Err(e) => {
            messages.push(ConfigLoadMessage::Warn(format!(
                "Failed to parse config at {}: {e}",
                config_path.display()
            )));
            (AssistantConfig::default(), messages)
        }
    }
}

// ── /review command ───────────────────────────────────────────────────────────

async fn cmd_review(storage: &StorageLayer, registry: &SkillRegistry) -> Result<()> {
    let store = storage.refinements_store();
    let pending = store.list_by_status(&RefinementStatus::Pending).await?;

    if pending.is_empty() {
        println!("No pending skill refinement proposals.");
        return Ok(());
    }

    println!("\nPending skill refinement proposals:\n");
    for r in &pending {
        println!(
            "  id:     {}\n  skill:  {}\n  reason: {}\n",
            r.id, r.target_skill, r.rationale
        );
    }

    println!("Commands: accept <id>  |  reject <id> [note]  |  done");

    loop {
        print!("review> ");
        io::stdout().flush().ok();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            break;
        }
        let line = line.trim();

        if line.is_empty() || line == "done" || line == "q" {
            break;
        }

        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        match parts.as_slice() {
            ["accept", id_str] => {
                let id = match Uuid::parse_str(id_str) {
                    Ok(id) => id,
                    Err(_) => {
                        eprintln!("Invalid UUID: {id_str}");
                        continue;
                    }
                };

                // Find the refinement in the pending list.
                let Some(refinement) = pending.iter().find(|r| r.id == id) else {
                    eprintln!("Refinement {id} not found in pending list.");
                    continue;
                };

                // Find the skill in the registry to get its directory.
                let skill_def = registry.get(&refinement.target_skill).await;
                if let Some(def) = skill_def {
                    let skill_md_path = def.dir.join("SKILL.md");
                    if let Err(e) = std::fs::write(&skill_md_path, &refinement.proposed_skill_md) {
                        eprintln!("Failed to write SKILL.md: {e}");
                        continue;
                    }
                    // Reload the skill from disk.
                    if let Err(e) = registry.reload(&refinement.target_skill).await {
                        eprintln!("Failed to reload skill: {e}");
                    } else {
                        println!("Skill '{}' updated and reloaded.", refinement.target_skill);
                    }
                } else {
                    eprintln!(
                        "Skill '{}' not found in registry; cannot write SKILL.md.",
                        refinement.target_skill
                    );
                }

                store.review(id, true, None).await?;
                println!("Refinement {id} accepted.");
            }

            ["reject", id_str] => {
                let id = match Uuid::parse_str(id_str) {
                    Ok(id) => id,
                    Err(_) => {
                        eprintln!("Invalid UUID: {id_str}");
                        continue;
                    }
                };
                store.review(id, false, None).await?;
                println!("Refinement {id} rejected.");
            }

            ["reject", id_str, note] => {
                let id = match Uuid::parse_str(id_str) {
                    Ok(id) => id,
                    Err(_) => {
                        eprintln!("Invalid UUID: {id_str}");
                        continue;
                    }
                };
                store.review(id, false, Some(note)).await?;
                println!("Refinement {id} rejected with note.");
            }

            _ => {
                eprintln!("Unknown command. Use: accept <id> | reject <id> [note] | done");
            }
        }
    }

    Ok(())
}

// ── Attachment delivery ───────────────────────────────────────────────────────

/// Save attachments from a turn result to `~/.assistant/attachments/` and print
/// their file paths so the user knows where to find them.
fn deliver_attachments(attachments: &[assistant_core::Attachment], assistant_dir: &Path) {
    let attach_dir = assistant_dir.join("attachments");
    if let Err(e) = std::fs::create_dir_all(&attach_dir) {
        eprintln!("Failed to create attachments directory: {e}");
        return;
    }

    for attachment in attachments {
        // Disambiguate filenames by prepending a short UUID prefix.
        let unique_name = format!(
            "{}_{}",
            &Uuid::new_v4().to_string()[..8],
            attachment.filename
        );
        let dest = attach_dir.join(&unique_name);

        match std::fs::write(&dest, &attachment.data) {
            Ok(()) => {
                let size = attachment.data.len();
                let kind = if attachment.is_image() {
                    "image"
                } else {
                    "file"
                };
                println!(
                    "  [{kind}] {} ({}, {size} bytes)",
                    dest.display(),
                    attachment.mime_type,
                );
            }
            Err(e) => {
                eprintln!("Failed to save attachment '{}': {e}", attachment.filename);
            }
        }
    }
}

// ── Token streaming ───────────────────────────────────────────────────────────

/// Spawn a background task that prints tokens from `rx` to stdout as they
/// arrive.  Returns a join handle; the task exits when the channel is closed
/// (i.e. when the orchestrator drops its `Sender`).
fn start_token_printer(mut rx: mpsc::Receiver<String>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut stdout = io::stdout();
        while let Some(token) = rx.recv().await {
            print!("{token}");
            let _ = stdout.flush();
        }
        // Trailing newline so the next prompt appears on its own line.
        println!("\n");
        let _ = stdout.flush();
    })
}

// ── Print help ────────────────────────────────────────────────────────────────

fn print_help() {
    println!(
        "\nAssistant REPL commands:\n\
         \n\
         /skills [name]                 List all skills, or show detail for one\n\
         /review                       Review pending skill refinement proposals\n\
         /install <path|owner/repo>    Install a skill from disk or GitHub\n\
         /model <name>                 Switch model (takes effect on next startup)\n\
         /help                         Show this help message\n\
         /quit | /exit                 Exit the assistant\n\
         \n\
         Any other input is sent to the AI assistant.\n"
    );
}

// ── reset subcommand ─────────────────────────────────────────────────────────

fn cmd_reset(db_path: &Path, config: &AssistantConfig, skip_confirm: bool) -> Result<()> {
    let loader = MemoryLoader::new(config);

    // Collect everything that will be removed so we can show the user upfront.
    let memory_files = [
        loader.soul_path().to_path_buf(),
        loader.identity_path().to_path_buf(),
        loader.user_path().to_path_buf(),
        loader.memory_path().to_path_buf(),
    ];

    println!("This will permanently delete:\n");
    println!("  Database : {}", db_path.display());
    for p in &memory_files {
        println!("  Memory   : {}", p.display());
    }
    // notes_dir is not exposed via a public getter; reconstruct from home.
    let notes_dir = dirs::home_dir()
        .map(|h| h.join(".assistant").join("memory"))
        .unwrap_or_else(|| PathBuf::from(".assistant/memory"));
    println!("  Notes dir: {}", notes_dir.display());
    println!();

    if !skip_confirm {
        print!("Are you sure? [y/N] ");
        io::stdout().flush().ok();
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).ok();
        if !matches!(buf.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Delete the SQLite database file.
    if db_path.exists() {
        std::fs::remove_file(db_path)
            .with_context(|| format!("Failed to remove database at {}", db_path.display()))?;
        println!("Removed: {}", db_path.display());
    }

    // Delete the four core memory files.
    for p in &memory_files {
        if p.exists() {
            std::fs::remove_file(p).with_context(|| format!("Failed to remove {}", p.display()))?;
            println!("Removed: {}", p.display());
        }
    }

    // Delete the daily notes directory.
    if notes_dir.exists() {
        std::fs::remove_dir_all(&notes_dir)
            .with_context(|| format!("Failed to remove {}", notes_dir.display()))?;
        println!("Removed: {}", notes_dir.display());
    }

    // Re-seed default memory files so the next session starts with sensible
    // content rather than an empty directory.
    loader.ensure_defaults();
    println!("\nDefaults restored. Assistant is ready for a fresh start.");

    Ok(())
}

// ── Embedding provider factory ────────────────────────────────────────────────

/// Build a dedicated [`EmbeddingProvider`] from an [`EmbeddingConfig`].
///
/// Falls back to provider-specific env vars for API keys.
fn build_embedding_provider(
    emb_cfg: &EmbeddingConfig,
    main_cfg: &assistant_core::LlmConfig,
) -> Result<Arc<dyn EmbeddingProvider>> {
    match emb_cfg.provider {
        EmbeddingProviderKind::Ollama => {
            let ollama_cfg = OllamaConfig {
                model: "unused".to_string(),
                base_url: emb_cfg
                    .base_url
                    .clone()
                    .unwrap_or_else(|| main_cfg.base_url.clone()),
                timeout_secs: main_cfg.timeout_secs,
                embedding_model: emb_cfg
                    .model
                    .clone()
                    .unwrap_or_else(|| "nomic-embed-text".to_string()),
            };
            let provider = OllamaProvider::new(ollama_cfg)
                .context("Failed to create Ollama embedding provider")?;
            Ok(Arc::new(LlmEmbedder(Arc::new(provider))))
        }
        EmbeddingProviderKind::OpenAI => {
            let api_key = emb_cfg
                .api_key
                .clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "OpenAI embedding provider requires an API key. \
                         Set api_key in [llm.embeddings] or OPENAI_API_KEY env var."
                    )
                })?;
            let provider_cfg = OpenAIProviderConfig {
                model: "unused".to_string(),
                base_url: emb_cfg
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
                timeout_secs: main_cfg.timeout_secs,
                max_tokens: 8192,
                embedding_model: emb_cfg
                    .model
                    .clone()
                    .unwrap_or_else(|| "text-embedding-3-small".to_string()),
            };
            let provider = OpenAIProvider::new(provider_cfg, &api_key)
                .context("Failed to create OpenAI embedding provider")?;
            Ok(Arc::new(LlmEmbedder(Arc::new(provider))))
        }
        EmbeddingProviderKind::Voyage => {
            let api_key = emb_cfg
                .api_key
                .clone()
                .or_else(|| std::env::var("VOYAGE_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Voyage AI embedding provider requires an API key. \
                         Set api_key in [llm.embeddings] or VOYAGE_API_KEY env var."
                    )
                })?;
            let mut voyage_cfg = VoyageConfig::new(api_key);
            if let Some(ref url) = emb_cfg.base_url {
                voyage_cfg = voyage_cfg.with_base_url(url.clone());
            }
            if let Some(ref model) = emb_cfg.model {
                voyage_cfg = voyage_cfg.with_model(model.clone());
            }
            let embedder = VoyageEmbedder::new(voyage_cfg)
                .context("Failed to create Voyage AI embedding provider")?;
            Ok(Arc::new(embedder))
        }
    }
}

// ── Common bootstrap ──────────────────────────────────────────────────────────

struct Bootstrap {
    config: AssistantConfig,
    storage: Arc<StorageLayer>,
    registry: Arc<SkillRegistry>,
    executor: Arc<ToolExecutor>,
    orchestrator: Arc<Orchestrator>,
    user_skills_dir: PathBuf,
}

async fn bootstrap(
    home: &Path,
    confirmation_cb: Arc<dyn ConfirmationCallback>,
    storage: Arc<StorageLayer>,
    config: AssistantConfig,
) -> Result<Bootstrap> {
    let assistant_dir = home.join(".assistant");
    let user_skills_dir = assistant_dir.join("skills");

    // Build skill registry.
    let registry = SkillRegistry::new(storage.pool.clone())
        .await
        .context("Failed to create skill registry")?;

    let project_root = std::env::current_dir().ok();
    let dirs_to_scan = assistant_runtime::bootstrap::skill_dirs(&config, project_root.as_deref());
    let dirs_ref: Vec<(&Path, SkillSource)> = dirs_to_scan
        .iter()
        .map(|(p, s)| (p.as_path(), s.clone()))
        .collect();

    registry
        .load_embedded()
        .await
        .context("Failed to load embedded builtin skills")?;

    registry
        .load_from_dirs(&dirs_ref)
        .await
        .context("Failed to load skills from directories")?;

    let registry = Arc::new(registry);

    // Build LLM client — dispatch on configured provider.
    let llm: Arc<dyn LlmProvider> = match config.llm.provider {
        LlmProviderKind::Ollama => Arc::new(
            OllamaProvider::from_llm_config(&config.llm)
                .context("Failed to create Ollama LLM client")?,
        ),
        LlmProviderKind::Anthropic => Arc::new(
            AnthropicProvider::from_llm_config(&config.llm)
                .context("Failed to create Anthropic LLM client")?,
        ),
        LlmProviderKind::OpenAI => Arc::new(
            OpenAIProvider::from_llm_config(&config.llm)
                .context("Failed to create OpenAI LLM client")?,
        ),
        LlmProviderKind::Moonshot => Arc::new(
            MoonshotProvider::from_llm_config(&config.llm)
                .context("Failed to create Moonshot LLM client")?,
        ),
    };

    // Optionally wrap with a dedicated embedding provider.
    let llm: Arc<dyn LlmProvider> = if let Some(ref emb_cfg) = config.llm.embeddings {
        let embedder = build_embedding_provider(emb_cfg, &config.llm)
            .context("Failed to build dedicated embedding provider")?;
        info!(
            provider = ?emb_cfg.provider,
            model = emb_cfg.model.as_deref().unwrap_or("(default)"),
            "Using dedicated embedding provider"
        );
        Arc::new(WithEmbeddingOverride::new(llm, embedder))
    } else {
        llm
    };

    // Build tool executor.
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));

    // Build message bus.
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());

    // Build orchestrator.
    let orchestrator = Arc::new(
        Orchestrator::new(
            llm,
            storage.clone(),
            executor.clone(),
            registry.clone(),
            bus,
            &config,
        )
        .with_confirmation_callback(confirmation_cb),
    );

    // Wire up subagent support (breaks the init-time circular dep).
    executor.set_subagent_runner(orchestrator.clone());

    Ok(Bootstrap {
        config,
        storage,
        registry,
        executor,
        orchestrator,
        user_skills_dir,
    })
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // Install the default rustls crypto provider (ring) once, before any TLS
    // handshake.  When both `aws-lc-rs` and `ring` features are compiled in
    // via transitive dependencies, rustls cannot auto-select one and panics
    // unless we do this explicitly.
    let _ = rustls::crypto::ring::default_provider().install_default();

    // 1. Parse CLI arguments first so we can configure tracing appropriately.
    let cli = Cli::parse();

    // 2. Resolve home directory and eagerly load config before tracing init.
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    let assistant_dir = home.join(".assistant");
    let config_path = assistant_dir.join("config.toml");
    let (config, config_logs) = load_config_messages(&config_path);
    let db_path: PathBuf = config
        .storage
        .db_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| assistant_dir.join("assistant.db"));

    // 3. Handle Reset early — does not need heavy resources.
    if let Some(Command::Reset { yes }) = &cli.command {
        return cmd_reset(&db_path, &config, *yes);
    }

    // 4. Prepare confirmation behavior before bootstrapping the stack.
    //
    //    MCP and Slack/Mattermost modes use auto-deny confirmation (no terminal
    //    interaction). REPL mode uses the interactive CLI confirmation.
    #[cfg(feature = "mcp")]
    let is_mcp = matches!(cli.command, Some(Command::Mcp));
    #[cfg(not(feature = "mcp"))]
    let is_mcp = false;

    #[cfg(feature = "slack")]
    let is_slack_only = matches!(cli.command, Some(Command::Slack));
    #[cfg(not(feature = "slack"))]
    let is_slack_only = false;

    #[cfg(feature = "mattermost")]
    let is_mattermost_only = matches!(cli.command, Some(Command::Mattermost));
    #[cfg(not(feature = "mattermost"))]
    let is_mattermost_only = false;

    let confirmation_cb: Arc<dyn ConfirmationCallback> =
        if is_mcp || is_slack_only || is_mattermost_only {
            Arc::new(assistant_runtime::bootstrap::AutoDenyConfirmation {
                interface_name: "background",
            })
        } else {
            Arc::new(CliConfirmation)
        };

    let storage = Arc::new(
        StorageLayer::new(&db_path)
            .await
            .with_context(|| format!("Failed to open database at {}", db_path.display()))?,
    );

    let _otel_guard = init_tracing(storage.pool.clone(), config.mirror.trace_enabled)?;
    for msg in config_logs {
        match msg {
            ConfigLoadMessage::Info(text) => info!("{text}"),
            ConfigLoadMessage::Warn(text) => warn!("{text}"),
        }
    }

    let bs = bootstrap(&home, confirmation_cb, storage.clone(), config).await?;

    // 5b. Spawn the turn worker (processes bus messages from scheduler, MCP, etc.).
    let worker_orch = bs.orchestrator.clone();
    let _worker = tokio::spawn(async move {
        worker_orch.run_worker("main-worker").await;
    });

    // 6. MCP mode — run the stdio JSON-RPC server and exit.
    #[cfg(feature = "mcp")]
    if let Some(Command::Mcp) = &cli.command {
        info!("Starting MCP server mode");
        return assistant_mcp_server::run(
            bs.orchestrator,
            bs.executor,
            bs.registry,
            bs.storage,
            bs.user_skills_dir,
        )
        .await;
    }

    // 7. Start the background scheduler (polls every 60 seconds).
    //    Spawned before interface-specific branches so that scheduled tasks
    //    fire regardless of the active interface (Slack, Mattermost, REPL).
    let _scheduler = spawn_scheduler(
        bs.storage.clone(),
        bs.orchestrator.clone(),
        Duration::from_secs(60),
    );

    // 8. Slack-only mode.
    #[cfg(feature = "slack")]
    if let Some(Command::Slack) = &cli.command {
        use assistant_interface_slack::SlackInterface;
        let slack_cfg = bs.config.slack.clone().context(
            "Slack is not configured. Add a [slack] section to ~/.assistant/config.toml",
        )?;
        let iface = SlackInterface::new(slack_cfg, bs.orchestrator, bs.storage);

        // Register ambient tools (slack-post, slack-send-dm, slack-list-channels)
        // so the LLM can see and invoke them during Slack turns.
        for handler in iface.ambient_tools() {
            let tool_name = handler.name().to_string();
            bs.executor.register_ambient_tool(handler);
            info!("Registered ambient tool: {tool_name}");
        }

        info!("Starting Slack-only mode");
        return iface.run().await;
    }

    // 9. Mattermost-only mode.
    #[cfg(feature = "mattermost")]
    if let Some(Command::Mattermost) = &cli.command {
        use assistant_interface_mattermost::MattermostInterface;
        let mm_cfg = bs.config.mattermost.clone().context(
            "Mattermost is not configured. Add a [mattermost] section to ~/.assistant/config.toml",
        )?;
        let iface = MattermostInterface::new(mm_cfg, bs.orchestrator);
        info!("Starting Mattermost-only mode");
        return iface.run().await;
    }

    // 10. Default mode: interactive REPL + background interfaces.
    //
    //     Register ambient tools from configured interfaces first, then spawn
    //     background tasks for those interfaces.

    // 10a. Slack — register slack-post as an ambient tool and start in background.
    #[cfg(feature = "slack")]
    if bs.config.slack.is_some() {
        use assistant_interface_slack::SlackInterface;
        let slack_cfg = bs.config.slack.clone().unwrap_or_default();
        let iface = SlackInterface::new(slack_cfg, bs.orchestrator.clone(), bs.storage.clone());

        // Register proactive Slack posting tool.
        for handler in iface.ambient_tools() {
            let tool_name = handler.name().to_string();
            bs.executor.register_ambient_tool(handler);
            info!("Registered ambient tool: {tool_name}");
        }

        // Spawn the Slack listener in the background.
        tokio::spawn(async move {
            if let Err(e) = iface.run().await {
                tracing::error!("Slack interface error: {e}");
            }
        });
    }

    // 10b. Mattermost — start in background if configured.
    #[cfg(feature = "mattermost")]
    if bs.config.mattermost.is_some() {
        use assistant_interface_mattermost::MattermostInterface;
        let mm_cfg = bs.config.mattermost.clone().unwrap_or_default();
        let iface = MattermostInterface::new(mm_cfg, bs.orchestrator.clone());
        tokio::spawn(async move {
            if let Err(e) = iface.run().await {
                tracing::error!("Mattermost interface error: {e}");
            }
        });
    }

    // 11. One conversation per session.
    let conversation_id = Uuid::new_v4();
    let _conv_cx = start_conversation_context(conversation_id, &Interface::Cli);
    info!(conversation_id = %conversation_id, "Starting CLI session");

    // 12. Run BOOT.md startup hook (if configured and non-empty).
    match bs
        .orchestrator
        .run_boot(conversation_id, Interface::Cli)
        .await
    {
        Ok(true) => info!("BOOT.md startup hook executed"),
        Ok(false) => {}
        Err(e) => warn!("BOOT.md startup hook failed: {e}"),
    }

    println!(
        "Assistant ready. Model: {}  (type /help for commands)\n",
        bs.config.llm.model
    );

    // 13. Build the reedline editor and prompt.
    let mut editor = Reedline::create();
    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::Basic("assistant".to_string()),
        DefaultPromptSegment::Empty,
    );

    // 14. REPL loop.
    loop {
        let sig = editor.read_line(&prompt);

        match sig {
            Ok(Signal::Success(line)) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
                }

                // Handle slash commands.
                if let Some(rest) = input.strip_prefix('/') {
                    let mut parts = rest.splitn(2, ' ');
                    let cmd = parts.next().unwrap_or("");
                    let arg = parts.next().unwrap_or("").trim();

                    match cmd {
                        "skills" => {
                            if arg.is_empty() {
                                let skills = bs.registry.list().await;
                                if skills.is_empty() {
                                    println!("No skills registered.");
                                } else {
                                    println!("\nRegistered skills ({}):\n", skills.len());
                                    for s in &skills {
                                        println!(
                                            "  {:30}  source={:10}  dir={}",
                                            s.name,
                                            s.source,
                                            s.dir.display()
                                        );
                                    }
                                    println!();
                                }
                            } else if let Some(skill) = bs.registry.get(arg).await {
                                println!("\nSkill: {}", skill.name);
                                println!("  Source:      {}", skill.source);
                                println!("  Directory:   {}", skill.dir.display());
                                if !skill.description.is_empty() {
                                    println!("  Description: {}", skill.description);
                                }
                                if skill.has_auxiliary_files() {
                                    println!("\n  Auxiliary files:");
                                    for (_category, path) in skill.auxiliary_files() {
                                        println!("    {}", path.display());
                                    }
                                }
                                println!();
                            } else {
                                eprintln!("Skill '{arg}' not found.");
                            }
                        }

                        "review" => {
                            if let Err(e) = cmd_review(&bs.storage, &bs.registry).await {
                                eprintln!("Error during review: {e}");
                            }
                        }

                        "model" => {
                            if arg.is_empty() {
                                println!("Current model: {}", bs.config.llm.model);
                                println!(
                                    "To switch models, update ~/.assistant/config.toml \
                                     and restart."
                                );
                            } else {
                                println!(
                                    "Model '{}' requested. Update ~/.assistant/config.toml \
                                     with:\n  [llm]\n  model = \"{}\"\nand restart.",
                                    arg, arg
                                );
                            }
                        }

                        "install" => {
                            if arg.is_empty() {
                                eprintln!("Usage: /install <local-path> | <owner/repo[/path]>");
                            } else {
                                println!("Installing skill from '{arg}'...");
                                match install_skill_from_source(
                                    arg,
                                    &bs.user_skills_dir,
                                    bs.registry.clone(),
                                )
                                .await
                                {
                                    Ok(name) => {
                                        println!("Skill '{name}' installed successfully.");
                                    }
                                    Err(e) => {
                                        eprintln!("Install failed: {e}");
                                    }
                                }
                            }
                        }

                        "help" | "?" => {
                            print_help();
                        }

                        "quit" | "exit" | "q" => {
                            println!("Goodbye.");
                            break;
                        }

                        other => {
                            eprintln!(
                                "Unknown command '/{other}'. Type /help for available commands."
                            );
                        }
                    }

                    continue;
                }

                // Normal user input — submit through the message bus with
                // live token streaming via a registered side-channel.
                let (tx, rx) = mpsc::channel::<String>(64);
                let printer = start_token_printer(rx);

                // Register the token sink so the worker streams to it.
                bs.orchestrator
                    .register_token_sink(conversation_id, tx)
                    .await;

                // submit_turn publishes to the bus; the worker claims it,
                // finds the registered sink, and calls run_turn_streaming.
                let orch = bs.orchestrator.clone();
                let prompt = input.to_string();
                let submit = tokio::spawn(async move {
                    orch.submit_turn(&prompt, conversation_id, Interface::Cli)
                        .await
                });

                // Await the submit result first — if it fails, abort the
                // printer to avoid hanging on a never-closed channel.
                let submit_result = submit.await;

                // Flush remaining tokens on success; abort on failure to
                // prevent blocking on a channel that may never close.
                if matches!(&submit_result, Ok(Ok(_))) {
                    let _ = printer.await;
                } else {
                    printer.abort();
                }

                match submit_result {
                    Ok(Ok(result)) => {
                        // Deliver any file attachments returned by tools.
                        if !result.attachments.is_empty() {
                            deliver_attachments(&result.attachments, &assistant_dir);
                        }
                    }
                    Ok(Err(e)) => {
                        eprintln!("Error: {e}\n");
                    }
                    Err(e) => {
                        eprintln!("Error: task panicked: {e}\n");
                    }
                }
            }

            Ok(Signal::CtrlC) => {
                println!("(Ctrl-C — type /exit to quit)");
            }

            Ok(Signal::CtrlD) => {
                println!("Goodbye.");
                break;
            }

            Err(e) => {
                eprintln!("Read error: {e}");
                break;
            }
        }
    }

    Ok(())
}
