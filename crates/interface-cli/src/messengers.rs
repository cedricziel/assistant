//! Launching messenger interfaces (Slack, Mattermost, Matrix, Nextcloud,
//! Signal) as the dedicated process or as background tasks.
//!
//! Which messengers run, and which workers they need, is decided by the pure
//! logic in [`crate::wiring`]; this module only builds and starts them.

use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::{error, info};

use assistant_core::ToolHandler;
use assistant_interfaces::{
    MatrixInterface, MattermostInterface, NextcloudInterface, SignalInterface, SlackInterface,
};
use assistant_runtime::{InterfaceRunner, Orchestrator, SpaceRuntime, SpaceRuntimes};
use assistant_storage::OrgStorageLayer;
use assistant_tool_executor::ToolExecutor;
use assistant_transcription::TranscriptionProvider;
use assistant_web_ui::install::DefaultSpace;

use crate::space_binding::{SpaceBinding, messenger_space};
use crate::wiring::{Messenger, RunMode};

/// Shared inputs every messenger is built from.
pub struct MessengerDeps<'a> {
    pub config: &'a assistant_core::types::agent::AssistantConfig,
    pub runtimes: Arc<SpaceRuntimes>,
    pub org_storage: Arc<OrgStorageLayer>,
    pub default_space: DefaultSpace,
    pub transcription: Option<Arc<dyn TranscriptionProvider>>,
    pub transcription_language: Option<String>,
}

impl MessengerDeps<'_> {
    /// The space `messenger` is bound to, and its runtime.
    pub async fn bound_space(&self, messenger: Messenger) -> Result<SpaceBinding> {
        messenger_space(&self.org_storage, messenger.key(), &self.default_space).await
    }

    async fn runtime_for(&self, messenger: Messenger) -> Result<Arc<SpaceRuntime>> {
        let binding = self.bound_space(messenger).await?;
        info!(
            interface = messenger.key(),
            org = %binding.org_slug,
            space = %binding.space_slug,
            "messenger bound to space"
        );
        Ok(binding.space_ref(&self.runtimes).await?.runtime)
    }
}

struct Built {
    runner: Box<dyn InterfaceRunner>,
    ambient_tools: Vec<Arc<dyn ToolHandler>>,
}

impl Messenger {
    fn display_name(self) -> &'static str {
        match self {
            Self::Nextcloud => "Nextcloud Talk",
            other => other.interface_name(),
        }
    }

    /// With `--no-repl` this messenger is the foreground process instead of a
    /// background task.
    fn foreground_without_repl(self) -> bool {
        self == Self::Signal
    }
}

fn not_configured(messenger: Messenger) -> String {
    format!(
        "{} is not configured. Add a [{}] section to ~/.assistant/config.toml",
        messenger.interface_name(),
        messenger.key()
    )
}

macro_rules! with_transcription {
    ($iface:expr, $deps:expr) => {{
        let mut iface = $iface;
        if let Some(tp) = &$deps.transcription {
            iface = iface.with_transcription(tp.clone(), $deps.transcription_language.clone());
        }
        iface
    }};
}

fn build(
    messenger: Messenger,
    deps: &MessengerDeps<'_>,
    runtime: &SpaceRuntime,
    mode: RunMode,
) -> Result<Built> {
    let orch = runtime.orchestrator.clone();
    let missing = || not_configured(messenger);
    let built = match messenger {
        Messenger::Slack => {
            let cfg = deps.config.slack.clone().with_context(missing)?;
            let iface = with_transcription!(
                SlackInterface::new(cfg, orch, runtime.storage.clone()),
                deps
            );
            Built {
                ambient_tools: iface.ambient_tools(),
                runner: Box::new(iface),
            }
        }
        Messenger::Mattermost => {
            let cfg = deps.config.mattermost.clone().with_context(missing)?;
            let iface = with_transcription!(MattermostInterface::new(cfg, orch), deps);
            Built {
                runner: Box::new(iface),
                ambient_tools: Vec::new(),
            }
        }
        Messenger::Matrix => {
            let cfg = deps.config.matrix.clone().with_context(missing)?;
            let iface = with_transcription!(MatrixInterface::new(cfg, orch), deps);
            Built {
                runner: Box::new(iface),
                ambient_tools: Vec::new(),
            }
        }
        Messenger::Nextcloud => {
            let cfg = deps.config.nextcloud.clone().with_context(missing)?;
            let mut iface = NextcloudInterface::new(cfg, orch);
            if mode == RunMode::Background {
                // A shutdown token lets the HTTP server stop without
                // installing process-wide signal handlers that would clash
                // with the REPL's Ctrl-C handling.
                iface = iface.with_shutdown(tokio_util::sync::CancellationToken::new());
            }
            let iface = with_transcription!(iface, deps);
            Built {
                runner: Box::new(iface),
                ambient_tools: Vec::new(),
            }
        }
        Messenger::Signal => {
            let cfg = deps.config.signal.clone().with_context(missing)?;
            let iface = with_transcription!(SignalInterface::new(cfg, orch), deps);
            Built {
                runner: Box::new(iface),
                ambient_tools: Vec::new(),
            }
        }
    };
    Ok(built)
}

fn register_ambient_tools(executor: &ToolExecutor, tools: Vec<Arc<dyn ToolHandler>>) {
    for handler in tools {
        let tool_name = handler.name().to_string();
        executor.register_ambient_tool(handler);
        info!("Registered ambient tool: {tool_name}");
    }
}

fn spawn_workers(orchestrator: &Arc<Orchestrator>, messenger: Messenger, mode: RunMode) {
    for worker in messenger.workers(mode) {
        let orch = orchestrator.clone();
        tokio::spawn(async move {
            orch.run_worker_filtered(worker.id, Some(worker.interface))
                .await;
        });
    }
}

/// Run `messenger` as the whole process: register its tools, spawn its
/// workers and block on the interface until it stops.
pub async fn run_dedicated(messenger: Messenger, deps: &MessengerDeps<'_>) -> Result<()> {
    let runtime = deps.runtime_for(messenger).await?;
    let built = build(messenger, deps, &runtime, RunMode::Dedicated)?;
    register_ambient_tools(&runtime.executor, built.ambient_tools);
    spawn_workers(&runtime.orchestrator, messenger, RunMode::Dedicated);
    info!("Starting {}-only mode", messenger.display_name());
    built.runner.run().await
}

/// Start `messenger` as a background task next to the REPL/orchestrator.
///
/// Returns `Ok(Some(result))` when the messenger took over the foreground
/// (`--no-repl`) and has already finished; the caller should return it.
pub async fn start_background(
    messenger: Messenger,
    deps: &MessengerDeps<'_>,
    no_repl: bool,
) -> Result<Option<Result<()>>> {
    let runtime = deps.runtime_for(messenger).await?;
    let built = build(messenger, deps, &runtime, RunMode::Background)?;
    register_ambient_tools(&runtime.executor, built.ambient_tools);

    if no_repl && messenger.foreground_without_repl() {
        info!(
            "Running {} interface in foreground (--no-repl)",
            messenger.display_name()
        );
        return Ok(Some(built.runner.run().await));
    }

    spawn_workers(&runtime.orchestrator, messenger, RunMode::Background);
    let name = messenger.display_name();
    tokio::spawn(async move {
        if let Err(e) = built.runner.run().await {
            error!("{name} interface error: {e}");
        }
    });
    Ok(None)
}
