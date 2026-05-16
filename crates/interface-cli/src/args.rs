//! `clap` argument tree for the assistant binary.
//! Mirrors the public CLI surface; `main.rs` matches against these
//! types to dispatch to each subcommand module.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::cmd_backup;

// ── Argument parsing ──────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "assistant", about = "Local AI assistant", version)]
pub struct Cli {
    /// Persona context ID (e.g. "default", "work", "personal").
    #[arg(long, env = "ASSISTANT_PERSONA")]
    pub persona: Option<String>,

    /// API key for non-interactive authentication (skips OAuth login).
    #[arg(long, env = "ASSISTANT_API_KEY", global = true)]
    pub api_key: Option<String>,

    /// Server URL when using --api-key (e.g. http://127.0.0.1:8080).
    #[arg(long, env = "ASSISTANT_SERVER", global = true)]
    pub server: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
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
pub enum MigrateCommand {
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
pub enum AccountCommand {
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
pub enum ApiKeysCommand {
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
pub enum OrchestratorCommand {
    /// Start orchestrator runtime.
    Run {
        /// Optional comma-separated interface list (slack,mattermost,matrix,nextcloud,signal).
        /// Falls back to the `ASSISTANT_INTERFACES` env var when unset, so the
        /// shipped systemd unit can configure interfaces via an env file.
        #[arg(long, env = "ASSISTANT_INTERFACES")]
        interfaces: Option<String>,
        /// Run without interactive REPL.
        #[arg(long)]
        no_repl: bool,
    },
}

#[derive(Subcommand)]
pub enum WebUiCommand {
    /// Serve the web UI and A2A endpoints.
    Serve {
        /// Additional web UI flags (e.g. --listen, --auth-token).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum PersonaCommand {
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
pub enum SkillCommand {
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
