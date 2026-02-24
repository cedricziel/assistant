use std::io::{self, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use std::time::Duration;

use anyhow::{Context, Result};
use assistant_core::{AssistantConfig, Interface, LlmProviderKind, MemoryLoader};
use assistant_llm::LlmProvider;
use assistant_provider_anthropic::AnthropicProvider;
use assistant_provider_ollama::OllamaProvider;
use assistant_runtime::{
    orchestrator::ConfirmationCallback, scheduler::spawn_scheduler, Orchestrator,
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

fn load_config(config_path: &Path) -> AssistantConfig {
    if !config_path.exists() {
        return AssistantConfig::default();
    }

    let raw = match std::fs::read_to_string(config_path) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to read config at {}: {e}", config_path.display());
            return AssistantConfig::default();
        }
    };

    match toml::from_str::<AssistantConfig>(&raw) {
        Ok(cfg) => {
            info!("Loaded config from {}", config_path.display());
            cfg
        }
        Err(e) => {
            warn!("Failed to parse config at {}: {e}", config_path.display());
            AssistantConfig::default()
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
) -> Result<Bootstrap> {
    let assistant_dir = home.join(".assistant");
    let user_skills_dir = assistant_dir.join("skills");
    let config_path = assistant_dir.join("config.toml");
    let config = load_config(&config_path);

    // Resolve database path.
    let db_path: PathBuf = config
        .storage
        .db_path
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| Some(assistant_dir.join("assistant.db")))
        .unwrap();

    // Open storage layer.
    let storage = Arc::new(
        StorageLayer::new(&db_path)
            .await
            .with_context(|| format!("Failed to open database at {}", db_path.display()))?,
    );

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
    };

    // Build tool executor.
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));

    // Build orchestrator.
    let orchestrator = Arc::new(
        Orchestrator::new(llm, storage.clone(), executor.clone(), &config)
            .with_confirmation_callback(confirmation_cb),
    );

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

    // 2. For MCP mode, all output on stdout is JSON-RPC — route logs to stderr.
    //    For all other modes the default (stderr) writer is also fine.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    // 3. Resolve home directory (needed for both early and late subcommands).
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    let assistant_dir = home.join(".assistant");
    let config_path = assistant_dir.join("config.toml");

    // 4. Handle Reset early — does not need heavy resources.
    if let Some(Command::Reset { yes }) = &cli.command {
        let config = load_config(&config_path);
        let db_path: PathBuf = config
            .storage
            .db_path
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| Some(assistant_dir.join("assistant.db")))
            .unwrap();
        return cmd_reset(&db_path, &config, *yes);
    }

    // 5. Bootstrap all heavy resources.
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

    let bs = bootstrap(&home, confirmation_cb).await?;

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

    // 7. Slack-only mode.
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

    // 8. Mattermost-only mode.
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

    // 9. Default mode: interactive REPL + background interfaces.
    //
    //    Register ambient tools from configured interfaces first, then spawn
    //    background tasks for those interfaces.

    // 9a. Slack — register slack-post as an ambient tool and start in background.
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

    // 9b. Mattermost — start in background if configured.
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

    // 10. Start the background scheduler (polls every 60 seconds).
    let _scheduler = spawn_scheduler(
        bs.storage.clone(),
        bs.orchestrator.clone(),
        Duration::from_secs(60),
    );

    // 11. One conversation per session.
    let conversation_id = Uuid::new_v4();
    info!(conversation_id = %conversation_id, "Starting CLI session");

    println!(
        "Assistant ready. Model: {}  (type /help for commands)\n",
        bs.config.llm.model
    );

    // 12. Build the reedline editor and prompt.
    let mut editor = Reedline::create();
    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::Basic("assistant".to_string()),
        DefaultPromptSegment::Empty,
    );

    // 13. REPL loop.
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

                // Normal user input — run through the orchestrator with
                // live token streaming.
                let (tx, rx) = mpsc::channel::<String>(64);
                let printer = start_token_printer(rx);

                let turn_result = bs
                    .orchestrator
                    .run_turn_streaming(input, conversation_id, Interface::Cli, tx)
                    .await;

                // Wait for the printer to flush all buffered tokens.
                let _ = printer.await;

                if let Err(e) = turn_result {
                    eprintln!("Error: {e}\n");
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
