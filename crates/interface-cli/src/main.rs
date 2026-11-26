mod args;
mod bootstrap;
mod cli_helpers;
mod cmd_account;
mod cmd_admin;
mod cmd_backup;
mod cmd_doctor;
mod cmd_login;
mod cmd_persona;
mod cmd_reset;
mod cmd_review;
mod cmd_skill;
mod cmd_webui;
mod credentials;
mod dispatch;
mod messengers;
mod repl_helpers;
mod skill_diff;
mod space_binding;
mod space_tasks;
mod wiring;

use args::{Cli, Command, OrchestratorCommand, SkillCommand};
use cli_helpers::{CliConfirmation, normalize_worker_interface, parse_interface_selection};

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use assistant_core::clock::{Clock, SystemClock};
use clap::Parser;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

use assistant_core::types::agent::{OrgConfig, ServerConfig};
use assistant_core::types::conversation::Interface;
use assistant_core::{
    agent_base_dir, apply_agent_context, default_workspace_dir, load_org_config,
    load_server_config, set_runtime_agent_root, set_runtime_install_root,
    set_runtime_workspace_dir, validate_agent_id,
};
use assistant_runtime::{
    BinaryRole, CommandContext, CommandRegistry, OrgContexts, SpaceSupervisor, SubmitTurn,
    init_tracing, orchestrator::ConfirmationCallback, start_conversation_context,
};
use assistant_storage::OrgPoolFactory;
use assistant_storage::{DEFAULT_ORG_SLUG, DEFAULT_SPACE_SLUG};
use assistant_tool_executor::install_skill_from_source;

/// How often the orchestrator service looks for spaces that were created or deleted.
const SPACE_RECONCILE_INTERVAL: Duration = Duration::from_secs(30);

/// The persona to run as: the `--persona` override, else the org's own. Fails
/// on an invalid id, and points `config`'s memory paths at the persona.
fn select_persona(
    install_root: &Path,
    cli_override: Option<&str>,
    config: &mut OrgConfig,
) -> Result<String> {
    let persona = cli_override.map_or_else(|| config.agent.id.clone(), str::to_owned);
    if !validate_agent_id(&persona) {
        anyhow::bail!("Invalid Persona ID '{persona}'. Use only letters, numbers, '-' and '_'.");
    }
    apply_agent_context(config, install_root, &persona);
    Ok(persona)
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    assistant_core::tls::install_ring();

    // 1. Parse CLI arguments first so we can configure tracing appropriately.
    let cli = Cli::parse();

    // 2. Resolve home directory and eagerly load config before tracing init.
    let assistant_dir = dirs::home_dir()
        .context("Cannot determine home directory")?
        .join(".assistant");
    // `assistant doctor` reports unreadable config files itself, so it still
    // runs when they do not load.
    let (server, mut config) = match (
        load_server_config(&assistant_dir),
        load_org_config(&assistant_dir, DEFAULT_ORG_SLUG),
    ) {
        (Ok(server), Ok(config)) => (server, config),
        _ if matches!(cli.command, Some(Command::Doctor)) => {
            (ServerConfig::default(), OrgConfig::default())
        }
        (Err(e), _) | (_, Err(e)) => return Err(e),
    };

    let cli_persona_override = cli.persona.clone();
    let selected_persona =
        select_persona(&assistant_dir, cli_persona_override.as_deref(), &mut config)?;
    let workspace_dir = default_workspace_dir(&assistant_dir, &selected_persona);
    set_runtime_install_root(assistant_dir.clone());
    set_runtime_agent_root(agent_base_dir(&assistant_dir, &selected_persona));
    set_runtime_workspace_dir(workspace_dir.clone());
    tokio::fs::create_dir_all(&workspace_dir)
        .await
        .with_context(|| format!("Failed to create workspace at {}", workspace_dir.display()))?;

    // Every space database lives under the install root; one-shot commands
    // work on the default space.
    let pools = Arc::new(OrgPoolFactory::new(assistant_dir.clone()));
    let db_path: PathBuf = pools.space_db_path(DEFAULT_ORG_SLUG, DEFAULT_SPACE_SLUG);

    // 3. One-shot subcommands do not need the heavy runtime.
    if let Some(result) = dispatch::run_one_shot(&cli, &assistant_dir, &db_path, &config).await {
        return result;
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
    //    Background modes use auto-deny confirmation (no terminal
    //    interaction). REPL mode uses the interactive CLI confirmation.
    //
    //    The binary role drives the infrastructure-worker plan, the single
    //    source of truth for which `main-worker` / `scheduler-worker` /
    //    `web-worker` / `TitleGeneratorWorker` / `memory_indexer` to spawn.
    //    See `assistant_runtime::worker_plan` for the rules and their unit tests.
    let binary_role =
        wiring::select_binary_role(cli.command.as_ref(), orchestrator_interface_filtered);
    let worker_plan = assistant_runtime::core_worker_plan(binary_role.clone());

    let confirmation_cb: Option<Arc<dyn ConfirmationCallback>> =
        if wiring::uses_auto_deny_confirmation(cli.command.as_ref(), orchestrator_no_repl) {
            None
        } else {
            Some(Arc::new(CliConfirmation::stdin()))
        };

    let _otel_guard = init_tracing()?;

    // Every org's `org.toml` is loaded on first use; the persona chosen on the
    // command line applies to all of them.
    let persona_install_root = assistant_dir.clone();
    let orgs = Arc::new(
        OrgContexts::from_files(&assistant_dir).with_adjust(Arc::new(move |org_config| {
            select_persona(
                &persona_install_root,
                cli_persona_override.as_deref(),
                org_config,
            )
            .map(drop)
        })),
    );
    let bs =
        bootstrap::bootstrap(&assistant_dir, confirmation_cb, pools.clone(), server, orgs).await?;

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
            .submit(
                SubmitTurn::new(prompt, conversation_id, Interface::Cli)
                    .with_auth(&assistant_core::auth::AuthContext::system())
                    .with_interactive(wiring::cli_turn_is_interactive(
                        cli.command.as_ref(),
                        orchestrator_no_repl,
                    )),
            )
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

    // 7. Org database and the space tasks.
    //
    //    Every space has its own turn queue, so the orchestrator service
    //    supervises all spaces in `org.db` and runs the worker plan once per
    //    space. Other roles run it for the one space they serve: the default
    //    space, or the one a dedicated messenger is bound to.
    let org_storage = Arc::new(pools.org_storage().await.context("Failed to open org.db")?);
    let default_space = assistant_web_ui::install::ensure_default_space(&org_storage)
        .await
        .context("Failed to seed the default org and space")?;
    let messenger_deps = messengers::MessengerDeps {
        config: &bs.config,
        install_root: &assistant_dir,
        runtimes: bs.runtimes.clone(),
        org_storage: org_storage.clone(),
        default_space: default_space.clone(),
        transcription: None,
        transcription_language: None,
    };
    let spawn_space_tasks =
        space_tasks::space_task_spawner(worker_plan, Some(Duration::from_secs(60)));
    if matches!(
        binary_role,
        BinaryRole::OrchestratorFiltered | BinaryRole::OrchestratorUnfiltered
    ) {
        let supervisor = Arc::new(SpaceSupervisor::new(
            org_storage.clone(),
            bs.runtimes.clone(),
            spawn_space_tasks,
        ));
        tokio::spawn(supervisor.run(SPACE_RECONCILE_INTERVAL));
    } else {
        let home = match wiring::Messenger::from_command(cli.command.as_ref()) {
            Some(messenger) => messenger_deps.bound_space(messenger).await?,
            None => space_binding::SpaceBinding::default_space(&default_space),
        };
        // Not awaited: the tasks run for the life of the process.
        let _space_tasks = spawn_space_tasks(home.space_ref(&bs.runtimes).await?);
    }

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

    // 7c. Build transcription provider (shared across interfaces).
    let transcription_language = bs
        .config
        .transcription
        .as_ref()
        .and_then(|tc| tc.language.clone());
    let transcription_provider = match bs.config.transcription.as_ref() {
        Some(tc) => {
            let provider = assistant_transcription::build_provider(tc, &bs.config.llm).await?;
            info!(provider = provider.name(), "Audio transcription enabled");
            Some(provider)
        }
        None => None,
    };

    let messenger_deps = messengers::MessengerDeps {
        transcription: transcription_provider,
        transcription_language,
        ..messenger_deps
    };

    // 8. Single-messenger mode (`assistant slack|mattermost|...`).
    if let Some(messenger) = wiring::Messenger::from_command(cli.command.as_ref()) {
        return messengers::run_dedicated(messenger, &messenger_deps).await;
    }

    // 10. Default mode: interactive REPL + background interfaces.
    //
    //     Register ambient tools from configured interfaces first, then spawn
    //     background tasks for those interfaces.

    for messenger in wiring::background_messengers(&bs.config, &orchestrator_interfaces) {
        if let Some(result) =
            messengers::start_background(messenger, &messenger_deps, orchestrator_no_repl).await?
        {
            return result;
        }
    }

    if orchestrator_no_repl {
        info!("Orchestrator running without REPL; waiting for shutdown signal");
        tokio::signal::ctrl_c().await?;
        info!("Shutdown signal received");
        return Ok(());
    }

    // 11. One conversation per session.
    let repl_interactive =
        wiring::cli_turn_is_interactive(cli.command.as_ref(), orchestrator_no_repl);
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
    let conversation_configs = bs.orchestrator.conversation_configs();
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
                            if let Err(e) = cmd_review::cmd_review(&bs.storage, &bs.registry).await
                            {
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
                            repl_helpers::print_help();
                        }

                        // -- Shared registry commands (/help, /new, /stop, /model, /compact, /status) --
                        _ => {
                            if let Some((cmd_name, args)) = command_registry.parse(input) {
                                let ctx = CommandContext {
                                    conversation_id,
                                    conversation_configs: conversation_configs.clone(),
                                    orchestrator: bs.orchestrator.clone(),
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
                let printer = repl_helpers::start_token_printer(rx);

                // submit_streaming publishes to the bus; the worker claims it,
                // finds the sink, and calls run_turn_streaming.
                let orch = bs.orchestrator.clone();
                let prompt = input.to_string();
                let msg_ts = SystemClock.now();
                let submit_result = tokio::spawn(async move {
                    orch.submit_streaming(
                        SubmitTurn::new(prompt, conversation_id, Interface::Cli)
                            .with_auth(&assistant_core::auth::AuthContext::system())
                            .with_timestamp(Some(msg_ts))
                            .with_interactive(repl_interactive),
                        tx,
                    )
                    .await
                })
                .await;

                // The sink is released with the result, so the printer ends.
                let _ = printer.await;

                match submit_result {
                    Ok(Ok(result)) => {
                        // Deliver any file attachments returned by tools.
                        if !result.attachments.is_empty() {
                            repl_helpers::deliver_attachments(&result.attachments, &assistant_dir);
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

            Ok(Signal::CtrlC(..)) => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_persona_prefers_the_command_line_override() {
        let mut config = OrgConfig::default();
        config.agent.id = "ops".to_string();

        assert_eq!(
            select_persona(Path::new("/srv/assistant"), Some("dev"), &mut config).unwrap(),
            "dev"
        );
        assert_eq!(config.agent.id, "dev");
    }

    #[test]
    fn select_persona_falls_back_to_the_orgs_own_persona() {
        let mut config = OrgConfig::default();
        config.agent.id = "ops".to_string();

        assert_eq!(
            select_persona(Path::new("/srv/assistant"), None, &mut config).unwrap(),
            "ops"
        );
    }

    #[test]
    fn select_persona_rejects_an_invalid_id() {
        let mut config = OrgConfig::default();

        assert!(select_persona(Path::new("/srv/assistant"), Some("../etc"), &mut config).is_err());
    }
}
