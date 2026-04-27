mod cmd_account;
mod cmd_backup;
mod cmd_doctor;
mod cmd_login;
mod cmd_migrate;
mod credentials;

use std::collections::{HashMap, HashSet};
use std::io::{self, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

use assistant_core::{
    AssistantConfig, ConversationConfig, EmbeddingConfig, EmbeddingProviderKind, Interface,
    MemoryLoader, MessageBus, apply_agent_context, default_workspace_dir, set_runtime_agent_root,
    set_runtime_workspace_dir, validate_agent_id,
};
use assistant_core::{EmbeddingProvider, LlmEmbedder, LlmProvider, WithEmbeddingOverride};
use assistant_llm_provider::{OllamaConfig, OllamaProvider, OpenAIProvider, OpenAIProviderConfig};
use assistant_llm_provider::{VoyageConfig, VoyageEmbedder};
use assistant_runtime::{
    CommandContext, CommandRegistry, Orchestrator, init_tracing,
    orchestrator::ConfirmationCallback, spawn_memory_indexer, spawn_scheduler,
    start_conversation_context,
};
use assistant_skills::SkillSource;
use assistant_storage::{
    OrgPoolFactory, PersonaSkillAccessStore, PersonaStore, RefinementStatus, StorageLayer,
    registry::SkillRegistry,
};
use assistant_tool_executor::{ToolExecutor, install_skill_from_source};

// ── Argument parsing ──────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "assistant", about = "Local AI assistant", version)]
struct Cli {
    /// Persona context ID (e.g. "default", "work", "personal").
    #[arg(long, env = "ASSISTANT_PERSONA")]
    persona: Option<String>,

    /// API key for non-interactive authentication (skips OAuth login).
    #[arg(long, env = "ASSISTANT_API_KEY", global = true)]
    api_key: Option<String>,

    /// Server URL when using --api-key (e.g. http://127.0.0.1:8080).
    #[arg(long, env = "ASSISTANT_SERVER", global = true)]
    server: Option<String>,

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
    Slack,
    /// Run only the Mattermost interface (no interactive REPL).
    ///
    /// Requires Mattermost server_url and token configured in ~/.assistant/config.toml.
    Mattermost,
    /// Run only the Nextcloud Talk interface (no interactive REPL).
    ///
    /// Requires Nextcloud server_url and secret configured in ~/.assistant/config.toml.
    /// The bot receives messages via webhooks from the Nextcloud Talk server.
    Nextcloud,
    /// Run only the Matrix bot interface (no interactive REPL).
    ///
    /// Requires homeserver_url and credentials configured in ~/.assistant/config.toml.
    #[command(about = "Start the Matrix bot (requires [matrix] in config.toml)")]
    Matrix,
    /// Run only the Signal interface (no interactive REPL).
    ///
    /// Requires a running signal-cli-rest-api daemon and [signal] section in
    /// ~/.assistant/config.toml.
    Signal,
    /// Manage Persona contexts.
    Persona {
        #[command(subcommand)]
        command: PersonaCommand,
    },
    /// Manage assistant skills (list, show, create, delete).
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
    /// Run orchestrator-managed interfaces and optional REPL.
    Orchestrator {
        #[command(subcommand)]
        command: OrchestratorCommand,
    },
    /// Run only a turn worker process.
    Worker {
        /// Interface filter (e.g. slack, mattermost, matrix, nextcloud, web, signal, any).
        #[arg(long, default_value = "any")]
        interface: String,
        /// Worker ID shown in logs and claim ownership.
        #[arg(long, default_value = "worker")]
        id: String,
    },
    /// Run the web UI through the unified assistant binary.
    Webui {
        #[command(subcommand)]
        command: WebUiCommand,
    },
    /// Back up the assistant installation to a .tar.gz archive.
    Backup(cmd_backup::BackupArgs),
    /// Restore the assistant installation from a backup archive.
    Restore(cmd_backup::RestoreArgs),
    /// Diagnose installation health (config, database, providers, etc.).
    Doctor,
    /// Log in to an assistant server using the device code flow.
    Login {
        /// Server URL (e.g. http://127.0.0.1:8080).
        server: String,
    },
    /// Log out and remove stored credentials.
    Logout,
    /// Show current login status.
    Status,
    /// Manage API keys for non-interactive authentication.
    #[command(name = "api-keys")]
    ApiKeys {
        #[command(subcommand)]
        command: ApiKeysCommand,
    },
    /// Manage your own account (name, email, password).
    Account {
        #[command(subcommand)]
        command: AccountCommand,
    },
    /// Manage migrations between assistant data layouts.
    Migrate {
        #[command(subcommand)]
        command: MigrateCommand,
    },
}

#[derive(Subcommand)]
enum MigrateCommand {
    /// Finalize a partially-migrated install: copy live `assistant.db` into
    /// `space.db`, rename the legacy file to `assistant.db.legacy`, and remove
    /// `*-shm`/`*-wal` sidecars. Refuses to run when an `assistant`
    /// orchestrator or webui process is detected unless `--force` is passed.
    Finalize {
        /// Skip the running-process check.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum AccountCommand {
    /// Show the current user (email, name, org, auth mode).
    Show {
        /// Print the raw `UserDetail` JSON instead of a friendly block.
        #[arg(long)]
        json: bool,
    },
    /// Change your email address.
    SetEmail {
        /// New email.
        email: String,
    },
    /// Change your display name.
    SetName {
        /// New display name.
        name: String,
    },
    /// Change your password (prompts for current + new + confirm).
    ChangePassword,
}

#[derive(Subcommand)]
enum ApiKeysCommand {
    /// Create a new API key.
    Create {
        /// Name for the key (e.g. "CI", "deploy").
        #[arg(long)]
        name: String,
        /// Comma-separated scopes (e.g. "conversations:read,conversations:write").
        #[arg(long)]
        scopes: Option<String>,
    },
    /// List your API keys.
    List,
    /// Revoke an API key by ID.
    Revoke {
        /// API key ID (e.g. "key_abc123").
        id: String,
    },
}

#[derive(Subcommand)]
enum OrchestratorCommand {
    /// Start orchestrator runtime.
    Run {
        /// Optional comma-separated interface list (slack,mattermost,matrix,nextcloud,signal).
        #[arg(long)]
        interfaces: Option<String>,
        /// Run without interactive REPL.
        #[arg(long)]
        no_repl: bool,
    },
}

#[derive(Subcommand)]
enum WebUiCommand {
    /// Serve the web UI and A2A endpoints.
    Serve {
        /// Additional web UI flags (e.g. --listen, --auth-token).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
enum PersonaCommand {
    /// List all configured Personas.
    List,
    /// Create a Persona context.
    Create {
        /// Persona ID (letters, numbers, '-', '_').
        id: String,
    },
    /// Switch to an existing Persona (creates it if needed).
    Use {
        /// Persona ID to activate.
        id: String,
    },
    /// Set the skill access mode for a persona (all, whitelist, blacklist).
    SkillMode {
        /// Persona ID.
        persona_id: String,
        /// Access mode: all | whitelist | blacklist.
        mode: String,
    },
    /// Add a skill to a persona's whitelist/blacklist.
    SkillAdd {
        /// Persona ID.
        persona_id: String,
        /// Skill name to add.
        skill_name: String,
    },
    /// Remove a skill from a persona's whitelist/blacklist.
    SkillRemove {
        /// Persona ID.
        persona_id: String,
        /// Skill name to remove.
        skill_name: String,
    },
    /// Set a per-persona turn timeout (overrides the 3-hour default).
    #[command(name = "timeout-set")]
    TimeoutSet {
        /// Persona ID.
        persona_id: String,
        /// Timeout in seconds (must be > 0). Example: 10800 = 3 h.
        secs: u64,
    },
    /// Clear the per-persona turn timeout, reverting to the default (3 h).
    #[command(name = "timeout-clear")]
    TimeoutClear {
        /// Persona ID.
        persona_id: String,
    },
}

#[derive(Subcommand)]
enum SkillCommand {
    /// List all registered skills.
    List {
        /// Filter by persona skill access (shows only skills visible to this persona).
        #[arg(long)]
        persona: Option<String>,
    },
    /// Show full details of a skill.
    Show {
        /// Skill name.
        name: String,
    },
    /// Create a new user skill.
    Create {
        /// Skill name (kebab-case, e.g. my-skill).
        #[arg(long)]
        name: String,
        /// Short description.
        #[arg(long)]
        description: String,
        /// Path to a file containing the skill body. Required (no editor fallback yet).
        #[arg(long)]
        body_file: PathBuf,
    },
    /// Delete a user or installed skill.
    Delete {
        /// Skill name.
        name: String,
        /// Skip the confirmation prompt.
        #[arg(short, long)]
        yes: bool,
    },
    /// Generate a SKILL.md draft using the AI assistant.
    ///
    /// Submits a prompt to the Orchestrator asking it to produce a valid
    /// SKILL.md for the given description, using the `agentskills-spec`
    /// builtin as the authoritative specification.
    Generate {
        /// Natural-language description of what the skill should do.
        description: String,
    },
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

fn normalize_worker_interface(interface: &str) -> Option<String> {
    match interface.trim().to_lowercase().as_str() {
        "" | "any" | "all" => None,
        "slack" => Some("Slack".to_string()),
        "mattermost" => Some("Mattermost".to_string()),
        "nextcloud" => Some("Nextcloud".to_string()),
        "web" | "webui" => Some("Web".to_string()),
        "signal" => Some("Signal".to_string()),
        other => Some(other.to_string()),
    }
}

fn parse_interface_selection(input: Option<&str>) -> HashSet<String> {
    input
        .map(|raw| {
            raw.split(',')
                .map(|v| v.trim().to_lowercase())
                .filter(|v| !v.is_empty())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

fn interface_selected(selected: &HashSet<String>, interface: &str) -> bool {
    selected.is_empty() || selected.contains(interface)
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod cli_parse_tests {
    use clap::Parser;

    use super::{Cli, Command, OrchestratorCommand, PersonaCommand};

    #[test]
    fn parses_persona_subcommand_list() {
        let cli = Cli::try_parse_from(["assistant", "persona", "list"]).unwrap();
        match cli.command {
            Some(Command::Persona { command }) => {
                assert!(matches!(command, PersonaCommand::List));
            }
            _ => panic!("expected persona list command"),
        }
    }

    #[test]
    fn parses_persona_flag_for_runtime_selection() {
        let cli =
            Cli::try_parse_from(["assistant", "--persona", "marketing", "orchestrator", "run"])
                .unwrap();

        assert_eq!(cli.persona.as_deref(), Some("marketing"));
        match cli.command {
            Some(Command::Orchestrator { command }) => {
                assert!(matches!(command, OrchestratorCommand::Run { .. }));
            }
            _ => panic!("expected orchestrator run command"),
        }
    }

    #[test]
    fn rejects_legacy_agent_subcommand_name() {
        let parse = Cli::try_parse_from(["assistant", "agent", "list"]);
        assert!(
            parse.is_err(),
            "legacy `agent` subcommand should be rejected"
        );
    }

    #[test]
    fn parses_login_command() {
        let cli = Cli::try_parse_from(["assistant", "login", "http://localhost:8080"]).unwrap();
        match cli.command {
            Some(Command::Login { server }) => {
                assert_eq!(server, "http://localhost:8080");
            }
            _ => panic!("expected login command"),
        }
    }

    #[test]
    fn parses_logout_command() {
        let cli = Cli::try_parse_from(["assistant", "logout"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Logout)));
    }

    #[test]
    fn parses_status_command() {
        let cli = Cli::try_parse_from(["assistant", "status"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Status)));
    }

    #[test]
    fn parses_api_keys_list() {
        let cli = Cli::try_parse_from(["assistant", "api-keys", "list"]).unwrap();
        match cli.command {
            Some(Command::ApiKeys { command }) => {
                assert!(matches!(command, super::ApiKeysCommand::List));
            }
            _ => panic!("expected api-keys list command"),
        }
    }

    #[test]
    fn parses_api_keys_create_with_scopes() {
        let cli = Cli::try_parse_from([
            "assistant",
            "api-keys",
            "create",
            "--name",
            "CI",
            "--scopes",
            "conversations:read,conversations:write",
        ])
        .unwrap();
        match cli.command {
            Some(Command::ApiKeys { command }) => match command {
                super::ApiKeysCommand::Create { name, scopes } => {
                    assert_eq!(name, "CI");
                    assert_eq!(
                        scopes.as_deref(),
                        Some("conversations:read,conversations:write")
                    );
                }
                _ => panic!("expected api-keys create"),
            },
            _ => panic!("expected api-keys command"),
        }
    }

    #[test]
    fn parses_api_keys_revoke() {
        let cli = Cli::try_parse_from(["assistant", "api-keys", "revoke", "key_abc123"]).unwrap();
        match cli.command {
            Some(Command::ApiKeys { command }) => match command {
                super::ApiKeysCommand::Revoke { id } => {
                    assert_eq!(id, "key_abc123");
                }
                _ => panic!("expected api-keys revoke"),
            },
            _ => panic!("expected api-keys command"),
        }
    }

    #[test]
    fn parses_global_api_key_flag() {
        let cli = Cli::try_parse_from([
            "assistant",
            "--api-key",
            "ask_live_test123",
            "--server",
            "http://example.com",
            "api-keys",
            "list",
        ])
        .unwrap();
        assert_eq!(cli.api_key.as_deref(), Some("ask_live_test123"));
        assert_eq!(cli.server.as_deref(), Some("http://example.com"));
        assert!(matches!(
            cli.command,
            Some(Command::ApiKeys {
                command: super::ApiKeysCommand::List
            })
        ));
    }

    #[test]
    fn api_key_env_var_name() {
        // Verify the env annotation exists by checking that --api-key accepts a value.
        let cli = Cli::try_parse_from(["assistant", "--api-key", "my_key", "status"]).unwrap();
        assert_eq!(cli.api_key.as_deref(), Some("my_key"));
    }

    #[test]
    fn parses_account_show() {
        let cli = Cli::try_parse_from(["assistant", "account", "show"]).unwrap();
        match cli.command {
            Some(Command::Account { command }) => {
                assert!(matches!(
                    command,
                    super::AccountCommand::Show { json: false }
                ));
            }
            _ => panic!("expected account show command"),
        }
    }

    #[test]
    fn parses_account_show_json() {
        let cli = Cli::try_parse_from(["assistant", "account", "show", "--json"]).unwrap();
        match cli.command {
            Some(Command::Account { command }) => {
                assert!(matches!(
                    command,
                    super::AccountCommand::Show { json: true }
                ));
            }
            _ => panic!("expected account show --json command"),
        }
    }

    #[test]
    fn parses_account_set_email() {
        let cli =
            Cli::try_parse_from(["assistant", "account", "set-email", "foo@bar.com"]).unwrap();
        match cli.command {
            Some(Command::Account { command }) => match command {
                super::AccountCommand::SetEmail { email } => {
                    assert_eq!(email, "foo@bar.com");
                }
                _ => panic!("expected account set-email"),
            },
            _ => panic!("expected account command"),
        }
    }

    #[test]
    fn parses_account_set_name() {
        let cli = Cli::try_parse_from(["assistant", "account", "set-name", "Jane Doe"]).unwrap();
        match cli.command {
            Some(Command::Account { command }) => match command {
                super::AccountCommand::SetName { name } => {
                    assert_eq!(name, "Jane Doe");
                }
                _ => panic!("expected account set-name"),
            },
            _ => panic!("expected account command"),
        }
    }

    #[test]
    fn parses_account_change_password() {
        let cli = Cli::try_parse_from(["assistant", "account", "change-password"]).unwrap();
        match cli.command {
            Some(Command::Account { command }) => {
                assert!(matches!(command, super::AccountCommand::ChangePassword));
            }
            _ => panic!("expected account change-password command"),
        }
    }
}

async fn cmd_webui(command: &WebUiCommand) -> Result<()> {
    let args = match command {
        WebUiCommand::Serve { args } => args,
    };

    let argv = std::iter::once("assistant-web-ui".to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    assistant_web_ui::run_from_iter(argv).await
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
fn start_token_printer(
    mut rx: mpsc::Receiver<assistant_runtime::OrchestratorEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        use assistant_runtime::OrchestratorEvent;
        let mut stdout = io::stdout();
        while let Some(event) = rx.recv().await {
            if let OrchestratorEvent::Token(token) = event {
                print!("{token}");
                let _ = stdout.flush();
            }
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
         /new                          Start a new conversation\n\
         /stop                         Cancel the current turn\n\
         /model [name]                 Show or switch the model for this conversation\n\
         /compact                      Compress conversation context\n\
         /status                       Show conversation status\n\
         /help                         Show this help message\n\
         /skills [name]                List all skills, or show detail for one\n\
         /review                       Review pending skill refinement proposals\n\
         /install <path|owner/repo>    Install a skill from disk or GitHub\n\
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
    let notes_dir = loader.notes_dir_path().to_path_buf();
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

async fn cmd_persona(db_path: &Path, command: &PersonaCommand) -> Result<()> {
    let storage = StorageLayer::new(db_path).await?;
    let store = storage.persona_store();
    store.ensure_exists("default").await?;

    match command {
        PersonaCommand::List => {
            let items = store.list().await?;
            if items.is_empty() {
                println!("No Personas configured.");
                return Ok(());
            }
            println!("Configured Personas:\n");
            for item in items {
                println!("  {:20} {}", item.id, item.name);
            }
        }
        PersonaCommand::Create { id } => {
            if !validate_agent_id(id) {
                anyhow::bail!(
                    "Invalid Persona ID '{}'. Use only letters, numbers, '-' and '_'.",
                    id
                );
            }
            store.ensure_exists(id).await?;

            let root = home_agent_root(id)?;
            let workspace = default_workspace_dir(id);
            tokio::fs::create_dir_all(&root).await?;
            tokio::fs::create_dir_all(&workspace).await?;
            println!("Persona '{}' is ready.", id);
        }
        PersonaCommand::Use { id } => {
            if !validate_agent_id(id) {
                anyhow::bail!(
                    "Invalid Persona ID '{}'. Use only letters, numbers, '-' and '_'.",
                    id
                );
            }
            store.ensure_exists(id).await?;
            let root = home_agent_root(id)?;
            let workspace = default_workspace_dir(id);
            tokio::fs::create_dir_all(&root).await?;
            tokio::fs::create_dir_all(&workspace).await?;
            println!(
                "Persona '{}' activated. Use --agent {} on next run.",
                id, id
            );
        }
        PersonaCommand::SkillMode { persona_id, mode } => {
            let access_store = PersonaSkillAccessStore::new(storage.pool.clone());
            if access_store.has_skill_list_entries(persona_id).await? {
                eprintln!(
                    "Warning: persona '{}' has existing skill list entries. \
                     Changing mode will reinterpret them as {} rules.",
                    persona_id, mode
                );
            }
            access_store.set_mode(persona_id, mode).await?;
            println!(
                "Persona '{}' skill access mode set to '{}'.",
                persona_id, mode
            );
        }
        PersonaCommand::SkillAdd {
            persona_id,
            skill_name,
        } => {
            let persona_store = PersonaStore::new(storage.pool.clone());
            if persona_store.get(persona_id).await?.is_none() {
                anyhow::bail!("Persona '{}' not found", persona_id);
            }
            let access_store = PersonaSkillAccessStore::new(storage.pool.clone());
            let mode = access_store.get_mode(persona_id).await?;
            if mode == "all" {
                eprintln!(
                    "Persona '{}' is in 'all' mode — use `skill-mode` to set whitelist or blacklist first.",
                    persona_id
                );
                std::process::exit(1);
            }
            access_store.add_skill(persona_id, skill_name).await?;
            println!(
                "Skill '{}' added to persona '{}' {} list.",
                skill_name, persona_id, mode
            );
        }
        PersonaCommand::SkillRemove {
            persona_id,
            skill_name,
        } => {
            let persona_store = PersonaStore::new(storage.pool.clone());
            if persona_store.get(persona_id).await?.is_none() {
                anyhow::bail!("Persona '{}' not found", persona_id);
            }
            let access_store = PersonaSkillAccessStore::new(storage.pool.clone());
            access_store.remove_skill(persona_id, skill_name).await?;
            println!(
                "Skill '{}' removed from persona '{}' list.",
                skill_name, persona_id
            );
        }
        PersonaCommand::TimeoutSet { persona_id, secs } => {
            store.set_turn_timeout(persona_id, *secs).await?;
            println!(
                "Persona '{}' turn timeout set to {} s ({:.1} h).",
                persona_id,
                secs,
                *secs as f64 / 3600.0
            );
        }
        PersonaCommand::TimeoutClear { persona_id } => {
            store.clear_turn_timeout(persona_id).await?;
            println!(
                "Persona '{}' turn timeout cleared (reverts to default 3 h).",
                persona_id
            );
        }
    }

    Ok(())
}

async fn cmd_skill(db_path: &Path, config: &AssistantConfig, command: &SkillCommand) -> Result<()> {
    let storage = StorageLayer::new(db_path).await?;
    let registry = SkillRegistry::new(storage.pool.clone()).await?;

    // Load builtin and on-disk skills so commands like `list`, `show`, and
    // `create` (duplicate check) see the full registry — not just SQLite rows.
    registry
        .load_embedded()
        .await
        .context("Failed to load embedded skills")?;
    let project_root = std::env::current_dir().ok();
    let dirs_to_scan = assistant_runtime::bootstrap::skill_dirs(config, project_root.as_deref());
    let dirs_ref: Vec<(&Path, SkillSource)> = dirs_to_scan
        .iter()
        .map(|(p, s)| (p.as_path(), s.clone()))
        .collect();
    registry
        .load_from_dirs(&dirs_ref)
        .await
        .context("Failed to load skills from directories")?;

    match command {
        SkillCommand::List { persona } => {
            let skills = if let Some(persona_id) = persona {
                registry.list_for_persona(persona_id, &storage.pool).await?
            } else {
                registry.list().await
            };

            if skills.is_empty() {
                println!("No skills registered.");
                return Ok(());
            }

            println!("{:<30} {:<12} DESCRIPTION", "NAME", "SOURCE");
            println!("{}", "-".repeat(80));
            for s in skills {
                let source = match s.source {
                    SkillSource::Builtin => "builtin",
                    SkillSource::User => "user",
                    SkillSource::Project => "project",
                    SkillSource::Installed => "installed",
                };
                println!("{:<30} {:<12} {}", s.name, source, s.description);
            }
        }
        SkillCommand::Show { name } => {
            let skill = registry
                .get(name)
                .await
                .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found", name))?;

            println!("name: {}", skill.name);
            println!("description: {}", skill.description);
            if let Some(license) = &skill.license {
                println!("license: {}", license);
            }
            println!("source: {}", skill.source);
            println!("dir: {}", skill.dir.display());
            println!();
            println!("{}", skill.body);
        }
        SkillCommand::Create {
            name,
            description,
            body_file,
        } => {
            let body = tokio::fs::read_to_string(body_file)
                .await
                .with_context(|| format!("Failed to read body file {}", body_file.display()))?;

            let def = registry.create_user_skill(name, description, &body).await?;
            println!("Skill '{}' created at {}.", def.name, def.dir.display());
        }
        SkillCommand::Delete { name, yes } => {
            if !*yes {
                print!("Delete skill '{}'? [y/N] ", name);
                io::stdout().flush().ok();
                let mut buf = String::new();
                if io::stdin().read_line(&mut buf).is_err()
                    || !matches!(buf.trim().to_lowercase().as_str(), "y" | "yes")
                {
                    println!("Aborted.");
                    return Ok(());
                }
            }
            registry.delete_user_skill(name).await?;
            println!("Skill '{}' deleted.", name);
        }
        // Handled after bootstrap in main() — should not be reached here.
        SkillCommand::Generate { .. } => {
            anyhow::bail!(
                "Generate requires a running Orchestrator — this code path should not be reached"
            );
        }
    }

    Ok(())
}

fn home_agent_root(agent_id: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    Ok(home.join(".assistant").join("agents").join(agent_id))
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
                web_search: None,
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
    llm: Arc<dyn LlmProvider>,
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

    // Sync embedded builtin skills to ~/.assistant/skills/ so on-disk copies
    // stay in sync with the binary.  Stale or missing files are overwritten;
    // user skills with different names are never touched.
    if let Some(home) = dirs::home_dir() {
        let builtin_target = home.join(".assistant").join("skills");
        match registry.sync_builtins_to_disk(&builtin_target) {
            Ok(updated) if !updated.is_empty() => {
                tracing::info!(
                    "Synced {} built-in skill(s) to disk: {}",
                    updated.len(),
                    updated.join(", ")
                );
            }
            Err(e) => {
                tracing::warn!("Failed to sync built-in skills to disk: {e}");
            }
            _ => {}
        }
    }

    registry
        .load_from_dirs(&dirs_ref)
        .await
        .context("Failed to load skills from directories")?;

    let registry = Arc::new(registry);

    // Build LLM client — dispatch on configured provider.
    let llm: Arc<dyn LlmProvider> = assistant_llm_provider::create_provider(&config.llm)
        .context("Failed to create LLM provider")?;

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
    // When the `nats` feature is enabled and `[bus] kind = "nats"` is
    // configured, use NATS JetStream to avoid SQLite write-lock contention
    // between the bus and the rest of the storage layer.  Otherwise fall
    // back to the SQLite-backed bus.
    let bus: Arc<dyn MessageBus> = {
        #[cfg(feature = "nats")]
        {
            use assistant_core::BusKind;
            if config.bus.kind == BusKind::Nats {
                info!("Using NATS message bus");
                Arc::new(
                    assistant_bus_nats::NatsMessageBus::connect(&config.bus)
                        .await
                        .context("failed to connect to NATS")?,
                )
            } else {
                Arc::new(storage.message_bus())
            }
        }
        #[cfg(not(feature = "nats"))]
        {
            use assistant_core::BusKind;
            if config.bus.kind == BusKind::Nats {
                anyhow::bail!(
                    "[bus] kind = \"nats\" configured but this binary was built without the `nats` feature"
                );
            }
            Arc::new(storage.message_bus())
        }
    };

    // Build orchestrator, applying the per-persona turn timeout if set.
    let persona_timeout = storage
        .persona_store()
        .get(&config.agent.id)
        .await
        .ok()
        .flatten()
        .and_then(|p| p.turn_timeout_secs);
    let audio_store = Arc::new(assistant_transcription::AudioStore::new());
    let orchestrator = Arc::new({
        let mut o = Orchestrator::new(
            llm,
            storage.clone(),
            executor.clone(),
            registry.clone(),
            bus,
            &config,
        )
        .with_confirmation_callback(confirmation_cb)
        .with_audio_store(audio_store.clone());
        if let Some(secs) = persona_timeout {
            o = o.with_submit_timeout(secs);
        }
        o
    });

    // Wire up subagent support (breaks the init-time circular dep).
    executor.set_subagent_runner(orchestrator.clone());

    // Connect to configured external MCP servers and register their tools.
    if !config.mcp.servers.is_empty() {
        let mcp_manager =
            Arc::new(assistant_mcp_client::McpClientManager::start(&config.mcp.servers).await?);
        let mcp_tools = mcp_manager.tool_handlers().await;
        for handler in &mcp_tools {
            executor.register_ambient_tool(handler.clone());
        }
        info!(
            servers = mcp_manager.server_count().await,
            tools = mcp_tools.len(),
            "registered MCP client tools"
        );

        // Spawn background health-check loop for reconnection.
        let exec_register = executor.clone();
        let exec_unregister = executor.clone();
        mcp_manager.spawn_health_loop(
            Arc::new(move |h| exec_register.register_ambient_tool(h)),
            Arc::new(move |prefix| exec_unregister.unregister_tools_by_prefix(prefix)),
        );
    }

    // Keep a reference to the LLM for the memory indexer.
    let llm = orchestrator.llm.clone();

    Ok(Bootstrap {
        config,
        storage,
        registry,
        executor,
        orchestrator,
        user_skills_dir,
        llm,
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
    let (mut config, config_logs) = load_config_messages(&config_path);

    let cli_persona_override = cli.persona.clone();
    let selected_persona = cli_persona_override
        .clone()
        .unwrap_or_else(|| config.agent.id.clone());
    if !validate_agent_id(&selected_persona) {
        anyhow::bail!(
            "Invalid Persona ID '{}'. Use only letters, numbers, '-' and '_'.",
            selected_persona
        );
    }
    apply_agent_context(&mut config, &selected_persona);
    let agent_root = assistant_dir.join("agents").join(&selected_persona);
    let workspace_dir = default_workspace_dir(&selected_persona);
    set_runtime_agent_root(agent_root);
    set_runtime_workspace_dir(workspace_dir.clone());
    tokio::fs::create_dir_all(&workspace_dir)
        .await
        .with_context(|| format!("Failed to create workspace at {}", workspace_dir.display()))?;

    // `assistant migrate finalize` is the recovery path for installs that
    // were partially migrated to the multi-org layout but never cut over.
    // It must run *before* `ensure_migrated`, which assumes the install is
    // either fully legacy or fully migrated.
    if let Some(Command::Migrate {
        command: MigrateCommand::Finalize { force },
    }) = &cli.command
    {
        return cmd_migrate::cmd_migrate_finalize(*force, &assistant_dir).await;
    }

    // Run the legacy-to-multi-org migration before opening any database. The
    // helper short-circuits when the install is already migrated, so this is
    // a cheap no-op on healthy hosts. Lives in `assistant-web-ui` because it
    // composes auth, backup, and storage helpers.
    if let Some(outcome) = assistant_web_ui::install::ensure_migrated(&assistant_dir).await? {
        info!("created admin user: {}", outcome.admin.user_id);
        match outcome.admin_credentials_file.as_ref() {
            Some(creds_path) => {
                info!("============================================================");
                info!("  Migration complete - initial admin credentials written to:");
                info!("    {}", creds_path.display());
                info!("  (file permissions: 0600 — owner read/write only)");
                info!("  Email: {}", outcome.admin.email);
                info!("  Change this password after first login and delete the file.");
                info!("============================================================");
            }
            None => {
                info!("============================================================");
                info!("  Migration complete - admin user created.");
                info!("    Email:    {}", outcome.admin.email);
                info!("    Password: (your ASSISTANT_WEB_TOKEN value)");
                info!("============================================================");
            }
        }
        info!(
            "migration complete — backup at {}",
            outcome.backup_path.display()
        );
    }

    // Resolve the runtime database path. Production hosts use the per-space
    // path under `orgs/default/spaces/default/space.db` (multi-org layout);
    // `config.storage.db_path` is preserved as a deprecated dev/test override.
    let db_path: PathBuf = match config.storage.db_path.as_deref() {
        Some(p) => {
            warn!(
                path = p,
                "config.storage.db_path is deprecated — runtime should use the per-space \
                 database under orgs/default/spaces/default/space.db. This override is kept \
                 for tests and dev-mode only and will be removed in a future release."
            );
            PathBuf::from(p)
        }
        None => OrgPoolFactory::new(assistant_dir.clone()).space_db_path("default", "default"),
    };

    // 3. Handle Reset early — does not need heavy resources.
    if let Some(Command::Reset { yes }) = &cli.command {
        return cmd_reset(&db_path, &config, *yes);
    }

    if let Some(Command::Persona { command }) = &cli.command {
        return cmd_persona(&db_path, command).await;
    }

    if let Some(Command::Skill { command }) = &cli.command {
        // Generate needs the full Orchestrator — handled after bootstrap.
        if !matches!(command, SkillCommand::Generate { .. }) {
            return cmd_skill(&db_path, &config, command).await;
        }
    }

    if let Some(Command::Backup(args)) = &cli.command {
        return cmd_backup::cmd_backup(args).await;
    }

    if let Some(Command::Restore(args)) = &cli.command {
        return cmd_backup::cmd_restore(args).await;
    }

    if let Some(Command::Webui { command }) = &cli.command {
        return cmd_webui(command).await;
    }

    if matches!(cli.command, Some(Command::Doctor)) {
        return cmd_doctor::cmd_doctor(&config_path, &db_path, &config).await;
    }

    if let Some(Command::Login { server }) = &cli.command {
        return cmd_login::cmd_login(server).await;
    }
    if matches!(cli.command, Some(Command::Logout)) {
        return cmd_login::cmd_logout().await;
    }
    if matches!(cli.command, Some(Command::Status)) {
        return cmd_login::cmd_status();
    }
    if let Some(Command::ApiKeys { command }) = &cli.command {
        return match command {
            ApiKeysCommand::Create { name, scopes } => {
                cmd_login::cmd_api_keys_create(name, scopes, &cli.api_key, &cli.server).await
            }
            ApiKeysCommand::List => cmd_login::cmd_api_keys_list(&cli.api_key, &cli.server).await,
            ApiKeysCommand::Revoke { id } => {
                cmd_login::cmd_api_keys_revoke(id, &cli.api_key, &cli.server).await
            }
        };
    }
    if let Some(Command::Account { command }) = &cli.command {
        return match command {
            AccountCommand::Show { json } => {
                cmd_account::cmd_account_show(*json, &cli.api_key, &cli.server).await
            }
            AccountCommand::SetEmail { email } => {
                cmd_account::cmd_account_set_email(email, &cli.api_key, &cli.server).await
            }
            AccountCommand::SetName { name } => {
                cmd_account::cmd_account_set_name(name, &cli.api_key, &cli.server).await
            }
            AccountCommand::ChangePassword => {
                cmd_account::cmd_account_change_password(&cli.api_key, &cli.server).await
            }
        };
    }

    let (orchestrator_interfaces, orchestrator_no_repl) =
        if let Some(Command::Orchestrator { command }) = &cli.command {
            match command {
                OrchestratorCommand::Run {
                    interfaces,
                    no_repl,
                } => (parse_interface_selection(interfaces.as_deref()), *no_repl),
            }
        } else {
            (HashSet::new(), false)
        };
    let orchestrator_interface_filtered = matches!(cli.command, Some(Command::Orchestrator { .. }))
        && !orchestrator_interfaces.is_empty();

    // 4. Prepare confirmation behavior before bootstrapping the stack.
    //
    //    MCP and Slack/Mattermost modes use auto-deny confirmation (no terminal
    //    interaction). REPL mode uses the interactive CLI confirmation.
    #[cfg(feature = "mcp")]
    let is_mcp = matches!(cli.command, Some(Command::Mcp));
    #[cfg(not(feature = "mcp"))]
    let is_mcp = false;

    let is_slack_only = matches!(cli.command, Some(Command::Slack));

    let is_mattermost_only = matches!(cli.command, Some(Command::Mattermost));

    let is_nextcloud_only = matches!(cli.command, Some(Command::Nextcloud));

    let is_signal_only = matches!(cli.command, Some(Command::Signal));

    let confirmation_cb: Arc<dyn ConfirmationCallback> = if is_mcp
        || is_slack_only
        || is_mattermost_only
        || is_nextcloud_only
        || is_signal_only
        || orchestrator_no_repl
    {
        Arc::new(assistant_runtime::bootstrap::AutoDenyConfirmation {
            interface_name: "background",
        })
    } else {
        Arc::new(CliConfirmation)
    };

    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating db parent directory {}", parent.display()))?;
    }
    let storage = Arc::new(
        StorageLayer::new(&db_path)
            .await
            .with_context(|| format!("Failed to open database at {}", db_path.display()))?,
    );
    let personas = storage.persona_store();
    personas.ensure_exists(&selected_persona).await?;

    let _otel_guard = init_tracing(storage.pool.clone(), &config.observability).await?;
    for msg in config_logs {
        match msg {
            ConfigLoadMessage::Info(text) => info!("{text}"),
            ConfigLoadMessage::Warn(text) => warn!("{text}"),
        }
    }

    let bs = bootstrap(&home, confirmation_cb, storage.clone(), config).await?;

    // 5a. Skill generate — one-shot turn, print result, exit.
    if let Some(Command::Skill {
        command: SkillCommand::Generate { description },
    }) = &cli.command
    {
        let conversation_id = Uuid::new_v4();
        let _conv_cx = start_conversation_context(conversation_id, &Interface::Cli);

        // Spawn the worker so the turn can be processed.
        let worker_orch = bs.orchestrator.clone();
        let _worker = tokio::spawn(async move { worker_orch.run_worker("generate-worker").await });

        // Embed the agentskills-spec body directly so generation works correctly
        // even when the active persona's skill list would otherwise filter it out.
        let spec_body = bs
            .registry
            .get("agentskills-spec")
            .await
            .map(|s| s.body)
            .unwrap_or_default();

        let prompt = format!(
            "You are generating a SKILL.md file.  Use the following authoritative \
             specification as your reference:\n\n<agentskills-spec>\n{spec}\n</agentskills-spec>\n\n\
             Generate a complete and valid SKILL.md for the following description:\n\n{desc}\n\n\
             Output ONLY the raw SKILL.md content — no explanation, no markdown fences.",
            spec = spec_body,
            desc = description
        );

        let result = bs
            .orchestrator
            .submit_turn(&prompt, conversation_id, Interface::Cli, None)
            .await?;
        println!("{}", result.answer);
        return Ok(());
    }

    // 5b. Worker-only mode.
    if let Some(Command::Worker { interface, id }) = &cli.command {
        let iface_filter = normalize_worker_interface(interface);
        info!(worker_id = %id, interface = ?iface_filter, "Starting worker-only mode");
        bs.orchestrator
            .run_worker_filtered(id, iface_filter.as_deref())
            .await;
        return Ok(());
    }

    // 5b. Spawn the main turn worker (processes generic bus messages).
    // In interface-only modes we run a dedicated, interface-filtered worker
    // below to avoid duplicate consumption of the same turn requests.
    let _worker = if !is_slack_only
        && !is_mattermost_only
        && !is_nextcloud_only
        && !orchestrator_interface_filtered
    {
        let worker_orch = bs.orchestrator.clone();
        Some(tokio::spawn(async move {
            worker_orch.run_worker("main-worker").await;
        }))
    } else {
        None
    };

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

    // 7a. Register the periodic skill improvement scheduled task when learning is enabled.
    if bs.config.learning.enabled
        && let Err(e) = assistant_runtime::skill_improver::register_improvement_task(
            &bs.storage,
            &selected_persona,
            &bs.config.learning,
        )
        .await
    {
        warn!(error = %e, "Failed to register skill improvement task");
    }

    // 7b. Start the memory indexer background task.
    let _memory_indexer =
        spawn_memory_indexer(&bs.config.memory, bs.storage.clone(), bs.llm.clone());

    // 7c. Build transcription provider (shared across interfaces).
    let transcription_language = bs
        .config
        .transcription
        .as_ref()
        .and_then(|tc| tc.language.clone());
    let transcription_provider = bs
        .config
        .transcription
        .as_ref()
        .map(|tc| {
            let provider = assistant_transcription::build_provider(tc)?;
            info!(provider = provider.name(), "Audio transcription enabled");
            Ok::<_, anyhow::Error>(provider)
        })
        .transpose()?;

    // 8. Slack-only mode.
    if let Some(Command::Slack) = &cli.command {
        use assistant_interfaces::SlackInterface;
        let slack_cfg = bs.config.slack.clone().context(
            "Slack is not configured. Add a [slack] section to ~/.assistant/config.toml",
        )?;
        let mut iface = SlackInterface::new(slack_cfg, bs.orchestrator.clone(), bs.storage.clone());
        if let Some(ref tp) = transcription_provider {
            iface = iface.with_transcription(tp.clone(), transcription_language.clone());
        }

        // Register ambient tools (slack-post, slack-send-dm, slack-list-channels)
        // so the LLM can see and invoke them during Slack turns.
        for handler in iface.ambient_tools() {
            let tool_name = handler.name().to_string();
            bs.executor.register_ambient_tool(handler);
            info!("Registered ambient tool: {tool_name}");
        }

        // Spawn a Slack-filtered turn worker. Using an interface filter avoids
        // duplicate handling when other workers are active in the same process.
        let worker_orch = bs.orchestrator.clone();
        let _worker = tokio::spawn(async move {
            worker_orch
                .run_worker_filtered("slack-worker", Some("Slack"))
                .await;
        });

        // Spawn a scheduler worker so scheduled tasks are consumed in
        // Slack-only mode (the main unfiltered worker is not started here).
        let sched_orch = bs.orchestrator.clone();
        let _sched_worker = tokio::spawn(async move {
            sched_orch
                .run_worker_filtered("scheduler-worker", Some("Scheduler"))
                .await;
        });

        info!("Starting Slack-only mode");
        return iface.run().await;
    }

    // 9. Mattermost-only mode.
    if let Some(Command::Mattermost) = &cli.command {
        use assistant_interfaces::MattermostInterface;
        let mm_cfg = bs.config.mattermost.clone().context(
            "Mattermost is not configured. Add a [mattermost] section to ~/.assistant/config.toml",
        )?;
        let mut iface = MattermostInterface::new(mm_cfg, bs.orchestrator.clone());
        if let Some(ref tp) = transcription_provider {
            iface = iface.with_transcription(tp.clone(), transcription_language.clone());
        }

        // Spawn a Mattermost-filtered turn worker to prevent duplicate
        // consumption with other workers.
        let worker_orch = bs.orchestrator.clone();
        let _worker = tokio::spawn(async move {
            worker_orch
                .run_worker_filtered("mattermost-worker", Some("Mattermost"))
                .await;
        });

        // Spawn a scheduler worker so scheduled tasks are consumed in
        // Mattermost-only mode (the main unfiltered worker is not started here).
        let sched_orch = bs.orchestrator.clone();
        let _sched_worker = tokio::spawn(async move {
            sched_orch
                .run_worker_filtered("scheduler-worker", Some("Scheduler"))
                .await;
        });

        info!("Starting Mattermost-only mode");
        return iface.run().await;
    }

    // 9b. Nextcloud-only mode.
    if let Some(Command::Nextcloud) = &cli.command {
        use assistant_interfaces::NextcloudInterface;
        let nc_cfg = bs.config.nextcloud.clone().context(
            "Nextcloud is not configured. Add a [nextcloud] section to ~/.assistant/config.toml",
        )?;
        let mut iface = NextcloudInterface::new(nc_cfg, bs.orchestrator.clone());
        if let Some(ref tp) = transcription_provider {
            iface = iface.with_transcription(tp.clone(), transcription_language.clone());
        }

        // Spawn a Nextcloud-filtered turn worker to prevent duplicate
        // consumption with other workers.
        let worker_orch = bs.orchestrator.clone();
        let _worker = tokio::spawn(async move {
            worker_orch
                .run_worker_filtered("nextcloud-worker", Some("Nextcloud"))
                .await;
        });

        // Spawn a scheduler worker so scheduled tasks are consumed in
        // Nextcloud-only mode (the main unfiltered worker is not started here).
        let sched_orch = bs.orchestrator.clone();
        let _sched_worker = tokio::spawn(async move {
            sched_orch
                .run_worker_filtered("scheduler-worker", Some("Scheduler"))
                .await;
        });

        info!("Starting Nextcloud Talk-only mode");
        return iface.run().await;
    }

    // 9c. Matrix-only mode.
    if let Some(Command::Matrix) = &cli.command {
        use assistant_interfaces::MatrixInterface;
        let matrix_cfg = bs.config.matrix.clone().context(
            "Matrix is not configured. Add a [matrix] section to ~/.assistant/config.toml",
        )?;
        let mut iface = MatrixInterface::new(matrix_cfg, bs.orchestrator.clone());
        if let Some(ref tp) = transcription_provider {
            iface = iface.with_transcription(tp.clone(), transcription_language.clone());
        }

        let worker_orch = bs.orchestrator.clone();
        let _worker = tokio::spawn(async move {
            worker_orch
                .run_worker_filtered("matrix-worker", Some("Matrix"))
                .await;
        });

        let sched_orch = bs.orchestrator.clone();
        let _sched_worker = tokio::spawn(async move {
            sched_orch
                .run_worker_filtered("scheduler-worker", Some("Scheduler"))
                .await;
        });

        info!("Starting Matrix-only mode");
        return iface.run().await;
    }

    // 9d. Signal-only mode.
    if let Some(Command::Signal) = &cli.command {
        use assistant_interfaces::SignalInterface;
        let sig_cfg = bs.config.signal.clone().context(
            "Signal is not configured. Add a [signal] section to ~/.assistant/config.toml",
        )?;
        let mut iface = SignalInterface::new(sig_cfg, bs.orchestrator.clone());
        if let Some(ref tp) = transcription_provider {
            iface = iface.with_transcription(tp.clone(), transcription_language.clone());
        }

        let worker_orch = bs.orchestrator.clone();
        let _worker = tokio::spawn(async move {
            worker_orch
                .run_worker_filtered("signal-worker", Some("Signal"))
                .await;
        });

        let sched_orch = bs.orchestrator.clone();
        let _sched_worker = tokio::spawn(async move {
            sched_orch
                .run_worker_filtered("scheduler-worker", Some("Scheduler"))
                .await;
        });

        info!("Starting Signal-only mode");
        return iface.run().await;
    }

    // 10. Default mode: interactive REPL + background interfaces.
    //
    //     Register ambient tools from configured interfaces first, then spawn
    //     background tasks for those interfaces.

    // 10a. Slack — register slack-post as an ambient tool and start in background.
    if bs.config.slack.is_some() && interface_selected(&orchestrator_interfaces, "slack") {
        use assistant_interfaces::SlackInterface;
        let slack_cfg = bs.config.slack.clone().unwrap_or_default();
        let mut iface = SlackInterface::new(slack_cfg, bs.orchestrator.clone(), bs.storage.clone());
        if let Some(ref tp) = transcription_provider {
            iface = iface.with_transcription(tp.clone(), transcription_language.clone());
        }
        // Register proactive Slack posting tool.
        for handler in iface.ambient_tools() {
            let tool_name = handler.name().to_string();
            bs.executor.register_ambient_tool(handler);
            info!("Registered ambient tool: {tool_name}");
        }

        // Spawn a Slack-filtered turn worker so NATS turn requests are
        // consumed when running in orchestrator mode.
        let worker_orch = bs.orchestrator.clone();
        tokio::spawn(async move {
            worker_orch
                .run_worker_filtered("slack-worker", Some("Slack"))
                .await;
        });

        // Spawn the Slack listener in the background.
        tokio::spawn(async move {
            if let Err(e) = iface.run().await {
                tracing::error!("Slack interface error: {e}");
            }
        });
    }

    // 10b. Mattermost — start in background if configured.
    if bs.config.mattermost.is_some() && interface_selected(&orchestrator_interfaces, "mattermost")
    {
        use assistant_interfaces::MattermostInterface;
        let mm_cfg = bs.config.mattermost.clone().unwrap_or_default();
        let mut iface = MattermostInterface::new(mm_cfg, bs.orchestrator.clone());
        if let Some(ref tp) = transcription_provider {
            iface = iface.with_transcription(tp.clone(), transcription_language.clone());
        }

        let worker_orch = bs.orchestrator.clone();
        tokio::spawn(async move {
            worker_orch
                .run_worker_filtered("mattermost-worker", Some("Mattermost"))
                .await;
        });

        tokio::spawn(async move {
            if let Err(e) = iface.run().await {
                tracing::error!("Mattermost interface error: {e}");
            }
        });
    }

    // 10b-ii. Matrix — start in background if configured.
    if bs.config.matrix.is_some() && interface_selected(&orchestrator_interfaces, "matrix") {
        use assistant_interfaces::MatrixInterface;
        let matrix_cfg = bs.config.matrix.clone().unwrap_or_default();
        let mut iface = MatrixInterface::new(matrix_cfg, bs.orchestrator.clone());
        if let Some(ref tp) = transcription_provider {
            iface = iface.with_transcription(tp.clone(), transcription_language.clone());
        }

        let worker_orch = bs.orchestrator.clone();
        tokio::spawn(async move {
            worker_orch
                .run_worker_filtered("matrix-worker", Some("Matrix"))
                .await;
        });

        tokio::spawn(async move {
            if let Err(e) = iface.run().await {
                tracing::error!("Matrix interface error: {e}");
            }
        });
    }

    // 10c. Nextcloud Talk — start in background if configured.
    //      Pass a CancellationToken so the HTTP server shuts down without
    //      installing process-wide signal handlers (which would conflict
    //      with the REPL's Ctrl-C handling).
    if bs.config.nextcloud.is_some() && interface_selected(&orchestrator_interfaces, "nextcloud") {
        use assistant_interfaces::NextcloudInterface;
        let nc_cfg = bs.config.nextcloud.clone().unwrap_or_default();
        let shutdown_token = tokio_util::sync::CancellationToken::new();
        let mut iface =
            NextcloudInterface::new(nc_cfg, bs.orchestrator.clone()).with_shutdown(shutdown_token);
        if let Some(ref tp) = transcription_provider {
            iface = iface.with_transcription(tp.clone(), transcription_language.clone());
        }

        let worker_orch = bs.orchestrator.clone();
        tokio::spawn(async move {
            worker_orch
                .run_worker_filtered("nextcloud-worker", Some("Nextcloud"))
                .await;
        });

        tokio::spawn(async move {
            if let Err(e) = iface.run().await {
                tracing::error!("Nextcloud Talk interface error: {e}");
            }
        });
    }

    // 10d. Signal — start in background if configured.
    if bs.config.signal.is_some() && interface_selected(&orchestrator_interfaces, "signal") {
        use assistant_interfaces::SignalInterface;
        let sig_cfg = bs.config.signal.clone().unwrap_or_default();
        let mut iface = SignalInterface::new(sig_cfg, bs.orchestrator.clone());
        if let Some(ref tp) = transcription_provider {
            iface = iface.with_transcription(tp.clone(), transcription_language.clone());
        }

        if orchestrator_no_repl {
            info!("Running Signal interface in foreground (--no-repl)");
            return iface.run().await;
        }

        tokio::spawn(async move {
            if let Err(e) = iface.run().await {
                tracing::error!("Signal interface error: {e}");
            }
        });
    }

    // Spawn a scheduler worker when running in interface-filtered orchestrator
    // mode. The main unfiltered worker is suppressed in this mode (to avoid
    // double-consuming Slack/Mattermost/etc. turns), so scheduled tasks would
    // never be consumed without a dedicated Scheduler worker.
    if orchestrator_interface_filtered {
        let sched_orch = bs.orchestrator.clone();
        tokio::spawn(async move {
            sched_orch
                .run_worker_filtered("scheduler-worker", Some("Scheduler"))
                .await;
        });
    }

    if orchestrator_no_repl {
        info!("Orchestrator running without REPL; waiting for shutdown signal");
        tokio::signal::ctrl_c().await?;
        info!("Shutdown signal received");
        return Ok(());
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
        "Assistant ready. Persona: {}  Model: {}  (type /help for commands)\n",
        selected_persona, bs.config.llm.model
    );

    // 13. Shared command registry and per-conversation state.
    let command_registry = CommandRegistry::new();
    let conversation_configs: Arc<tokio::sync::RwLock<HashMap<Uuid, ConversationConfig>>> =
        Arc::new(tokio::sync::RwLock::new(HashMap::new()));
    let active_turns: Arc<tokio::sync::RwLock<HashMap<Uuid, Uuid>>> =
        Arc::new(tokio::sync::RwLock::new(HashMap::new()));
    let default_model = bs.config.llm.model.clone();

    // 14. Build the reedline editor and prompt.
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
                // CLI-local commands first, then shared registry commands.
                if let Some(rest) = input.strip_prefix('/') {
                    let mut parts = rest.splitn(2, ' ');
                    let cmd = parts.next().unwrap_or("");
                    let arg = parts.next().unwrap_or("").trim();
                    let mut is_command = true;

                    match cmd {
                        // -- CLI-local commands (not in the shared registry) --
                        "quit" | "exit" | "q" => {
                            println!("Goodbye.");
                            break;
                        }

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

                        "?" => {
                            print_help();
                        }

                        // -- Shared registry commands (/help, /new, /stop, /model, /compact, /status) --
                        _ => {
                            if let Some((cmd_name, args)) = command_registry.parse(input) {
                                let ctx = CommandContext {
                                    conversation_id,
                                    conversation_configs: conversation_configs.clone(),
                                    orchestrator: bs.orchestrator.clone(),
                                    active_turns: active_turns.clone(),
                                    evict_conversation: None,
                                    default_model: default_model.clone(),
                                };
                                let result = command_registry.execute(&cmd_name, &args, ctx).await;
                                println!("{}", result.ack_text);
                            } else {
                                // Not a recognized command — fall through to normal
                                // assistant submission (e.g. "/new-york pizza").
                                is_command = false;
                            }
                        }
                    }

                    if is_command {
                        continue;
                    }
                }

                // Normal user input — submit through the message bus with
                // live event streaming via a registered side-channel.
                let (tx, rx) = mpsc::channel::<assistant_runtime::OrchestratorEvent>(64);
                let printer = start_token_printer(rx);

                // Register the token sink so the worker streams to it.
                bs.orchestrator
                    .register_token_sink(conversation_id, tx)
                    .await;

                // submit_turn publishes to the bus; the worker claims it,
                // finds the registered sink, and calls run_turn_streaming.
                let orch = bs.orchestrator.clone();
                let prompt = input.to_string();
                let msg_ts = Utc::now();
                let submit = tokio::spawn(async move {
                    orch.submit_turn(&prompt, conversation_id, Interface::Cli, Some(msg_ts))
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

            Ok(_) => {}

            Err(e) => {
                eprintln!("Read error: {e}");
                break;
            }
        }
    }

    Ok(())
}
