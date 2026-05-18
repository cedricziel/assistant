mod args;
mod bootstrap;
mod cli_helpers;
mod cmd_account;
mod cmd_backup;
mod cmd_doctor;
mod cmd_login;
mod cmd_migrate;
mod cmd_persona;
mod cmd_reset;
mod cmd_review;
mod cmd_skill;
mod cmd_webui;
mod credentials;
mod repl_helpers;
mod skill_diff;

use args::{
    AccountCommand, ApiKeysCommand, Cli, Command, MigrateCommand, OrchestratorCommand, SkillCommand,
};
use cli_helpers::{
    CliConfirmation, ConfigLoadMessage, interface_selected, load_config_messages,
    normalize_worker_interface, parse_interface_selection,
};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use assistant_core::clock::{Clock, SystemClock};
use clap::Parser;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

use assistant_core::types::conversation::Interface;
use assistant_core::{
    ConversationConfig, apply_agent_context, default_workspace_dir, set_runtime_agent_root,
    set_runtime_workspace_dir, validate_agent_id,
};
use assistant_runtime::{
    CommandContext, CommandRegistry, init_tracing, orchestrator::ConfirmationCallback,
    spawn_memory_indexer, spawn_scheduler, start_conversation_context,
};
use assistant_storage::{OrgPoolFactory, PersonaStore as _, StorageLayer};
use assistant_tool_executor::install_skill_from_source;

pub fn home_agent_root(agent_id: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    Ok(home.join(".assistant").join("agents").join(agent_id))
}

// ── Embedding provider factory ────────────────────────────────────────────────

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
        return cmd_reset::cmd_reset(&db_path, &config, *yes);
    }

    if let Some(Command::Persona { command }) = &cli.command {
        return cmd_persona::cmd_persona(&db_path, command).await;
    }

    if let Some(Command::Skill { command }) = &cli.command {
        // Generate needs the full Orchestrator — handled after bootstrap.
        if !matches!(command, SkillCommand::Generate { .. }) {
            return cmd_skill::cmd_skill(&db_path, &config, command).await;
        }
    }

    if let Some(Command::Backup(args)) = &cli.command {
        return cmd_backup::cmd_backup(args).await;
    }

    if let Some(Command::Restore(args)) = &cli.command {
        return cmd_backup::cmd_restore(args).await;
    }

    if let Some(Command::Webui { command }) = &cli.command {
        return cmd_webui::cmd_webui(command).await;
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

    let is_matrix_only = matches!(cli.command, Some(Command::Matrix));

    let is_worker_only = matches!(cli.command, Some(Command::Worker { .. }));

    // Derive the binary role and infrastructure-worker plan. The plan is the
    // single source of truth for which `main-worker` / `scheduler-worker` /
    // `web-worker` / `TitleGeneratorWorker` / `memory_indexer` to spawn.
    // See `assistant_runtime::worker_plan` for the rules and their unit tests.
    let binary_role = if is_mcp {
        assistant_runtime::BinaryRole::Mcp
    } else if is_worker_only {
        assistant_runtime::BinaryRole::WorkerOnly
    } else if is_slack_only
        || is_mattermost_only
        || is_nextcloud_only
        || is_signal_only
        || is_matrix_only
    {
        assistant_runtime::BinaryRole::InterfaceOnly
    } else if orchestrator_interface_filtered {
        assistant_runtime::BinaryRole::OrchestratorFiltered
    } else if matches!(cli.command, Some(Command::Orchestrator { .. })) {
        assistant_runtime::BinaryRole::OrchestratorUnfiltered
    } else {
        assistant_runtime::BinaryRole::Repl
    };
    let worker_plan = assistant_runtime::core_worker_plan(binary_role.clone());

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

    let bs = bootstrap::bootstrap(&home, confirmation_cb, storage.clone(), config).await?;

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

    // 5b. Spawn the unfiltered main turn worker if the plan asks for it.
    // The plan rules live in `assistant_runtime::worker_plan` and are unit
    // tested there. Interface-filtered workers (scheduler-worker, web-worker)
    // are spawned later in this function for the orchestrator-filtered role.
    let mut _spawned_workers = Vec::new();
    if let Some(spec) = worker_plan
        .workers
        .iter()
        .find(|w| w.interface_filter.is_none())
    {
        let worker_orch = bs.orchestrator.clone();
        let id = spec.worker_id;
        _spawned_workers.push(tokio::spawn(async move {
            worker_orch.run_worker(id).await;
        }));
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

    // 7b. Start the memory indexer background task (gated by the worker plan).
    let _memory_indexer = worker_plan
        .spawn_memory_indexer
        .then(|| spawn_memory_indexer(&bs.config.memory, bs.storage.clone(), bs.llm.clone()));

    // 7b'. Start the title-generator worker (gated by the worker plan). Consumes
    //      `turn.result` and auto-titles conversations across every interface.
    //      The web-ui binary intentionally skips this to avoid duplicate
    //      consumers racing on the same bus claim (#730).
    let _title_worker = worker_plan.spawn_title_generator.then(|| {
        assistant_runtime::spawn_title_generator_worker(
            bs.orchestrator.bus().clone(),
            bs.storage.clone(),
            bs.llm.clone(),
            bs.config.titling.clone(),
            selected_persona.clone(),
        )
    });

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

    // Spawn the interface-filtered workers (scheduler-worker, web-worker)
    // requested by the worker plan. The plan returns these for
    // `OrchestratorFiltered` so scheduled tasks and Web-interface turns get
    // consumed by this process — the main unfiltered worker is suppressed in
    // that mode to avoid double-consuming Slack/Mattermost/etc. turns.
    for spec in worker_plan
        .workers
        .iter()
        .filter(|w| w.interface_filter.is_some())
    {
        let orch = bs.orchestrator.clone();
        let id = spec.worker_id;
        let filter = spec.interface_filter;
        tokio::spawn(async move {
            orch.run_worker_filtered(id, filter).await;
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
                let printer = repl_helpers::start_token_printer(rx);

                // Register the token sink so the worker streams to it.
                bs.orchestrator
                    .register_token_sink(conversation_id, tx)
                    .await;

                // submit_turn publishes to the bus; the worker claims it,
                // finds the registered sink, and calls run_turn_streaming.
                let orch = bs.orchestrator.clone();
                let prompt = input.to_string();
                let msg_ts = SystemClock.now();
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
