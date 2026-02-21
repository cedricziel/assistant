use std::io::{self, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::{Context, Result};
use assistant_core::{skill::SkillSource, AssistantConfig, Interface};
use assistant_llm::{LlmClient, LlmClientConfig};
use assistant_runtime::{orchestrator::ConfirmationCallback, ReactOrchestrator};
use assistant_skills_executor::{install_skill_from_source, SkillExecutor};
use assistant_storage::{registry::SkillRegistry, RefinementStatus, StorageLayer};
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};
use tracing::{info, warn};
use uuid::Uuid;

// ── CLI confirmation callback ─────────────────────────────────────────────────

/// Implements `ConfirmationCallback` by printing a `[y/N]` prompt to stdout
/// and reading a line from stdin.
struct CliConfirmation;

impl ConfirmationCallback for CliConfirmation {
    fn confirm(&self, skill_name: &str, params: &serde_json::Value) -> bool {
        let params_str = serde_json::to_string_pretty(params).unwrap_or_default();
        print!(
            "\nSkill '{}' requires confirmation.\nParams: {}\nProceed? [y/N] ",
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

// ── Skill directories ─────────────────────────────────────────────────────────

/// Return the list of `(directory, SkillSource)` pairs to scan for skills.
fn skill_dirs() -> Vec<(PathBuf, SkillSource)> {
    let mut dirs: Vec<(PathBuf, SkillSource)> = Vec::new();

    // Builtin skills next to the binary.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let builtin = exe_dir.join("skills");
            dirs.push((builtin, SkillSource::Builtin));
        }
    }

    // User skills in ~/.assistant/skills/.
    if let Some(home) = dirs::home_dir() {
        let user_skills = home.join(".assistant").join("skills");
        dirs.push((user_skills, SkillSource::User));
    }

    dirs
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

// ── Spinner ───────────────────────────────────────────────────────────────────

/// Spawn a background task that prints a spinner until `stop` is set to `true`.
/// Returns the `Arc<AtomicBool>` stop flag and the task join handle.
/// Call `stop.store(true, Ordering::Relaxed)` then await the handle to cleanly stop.
fn start_spinner() -> (Arc<AtomicBool>, tokio::task::JoinHandle<()>) {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    let handle = tokio::spawn(async move {
        let frames = ['-', '\\', '|', '/'];
        let mut i = 0usize;
        let mut stdout = io::stdout();
        loop {
            if stop_clone.load(Ordering::Relaxed) {
                // Clear the spinner line
                print!("\r   \r");
                let _ = stdout.flush();
                break;
            }
            print!("\r{} ", frames[i % frames.len()]);
            let _ = stdout.flush();
            i += 1;
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        }
    });
    (stop, handle)
}

// ── Print help ────────────────────────────────────────────────────────────────

fn print_help() {
    println!(
        "\nAssistant REPL commands:\n\
         \n\
         /skills                       List all registered skills\n\
         /review                       Review pending skill refinement proposals\n\
         /install <path|owner/repo>    Install a skill from disk or GitHub\n\
         /model <name>                 Switch model (takes effect on next startup)\n\
         /help                         Show this help message\n\
         /quit | /exit                 Exit the assistant\n\
         \n\
         Any other input is sent to the AI assistant.\n"
    );
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize tracing.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    // 2. Resolve config path and load config.
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    let assistant_dir = home.join(".assistant");
    let user_skills_dir = assistant_dir.join("skills");
    let config_path = assistant_dir.join("config.toml");
    let config = load_config(&config_path);

    // 3. Resolve database path.
    let db_path: PathBuf = config
        .storage
        .db_path
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| Some(assistant_dir.join("assistant.db")))
        .unwrap();

    // 4. Open storage layer (creates the DB and runs migrations).
    let storage = Arc::new(
        StorageLayer::new(&db_path)
            .await
            .with_context(|| format!("Failed to open database at {}", db_path.display()))?,
    );

    // 5. Create skill registry and load skills from disk.
    let mut registry = SkillRegistry::new(storage.pool.clone())
        .await
        .context("Failed to create skill registry")?;

    let dirs_to_scan = skill_dirs();
    let dirs_ref: Vec<(&Path, SkillSource)> = dirs_to_scan
        .iter()
        .map(|(p, s)| (p.as_path(), s.clone()))
        .collect();

    registry
        .load_from_dirs(&dirs_ref)
        .await
        .context("Failed to load skills from directories")?;

    let registry = Arc::new(registry);

    // 6. Build LLM client.
    let llm_config = LlmClientConfig::from(&config.llm);
    let llm = Arc::new(LlmClient::new(llm_config).context("Failed to create LLM client")?);

    // 7. Build skill executor.
    let executor = Arc::new(SkillExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
    ));

    // 8. Build orchestrator.
    let confirmation_cb: Arc<dyn ConfirmationCallback> = Arc::new(CliConfirmation);
    let orchestrator =
        ReactOrchestrator::new(llm, storage.clone(), registry.clone(), executor, &config)
            .with_confirmation_callback(confirmation_cb);

    // 9. One conversation per session.
    let conversation_id = Uuid::new_v4();
    info!(conversation_id = %conversation_id, "Starting CLI session");

    println!(
        "Assistant ready. Model: {}  (type /help for commands)\n",
        config.llm.model
    );

    // 10. Build the reedline editor and prompt.
    let mut editor = Reedline::create();
    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::Basic("assistant".to_string()),
        DefaultPromptSegment::Empty,
    );

    // 11. REPL loop.
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
                            let skills = registry.list().await;
                            if skills.is_empty() {
                                println!("No skills registered.");
                            } else {
                                println!("\nRegistered skills ({}):\n", skills.len());
                                for s in &skills {
                                    println!(
                                        "  {:30}  tier={:8}  source={:10}  dir={}",
                                        s.name,
                                        s.tier.label(),
                                        s.source,
                                        s.dir.display()
                                    );
                                }
                                println!();
                            }
                        }

                        "review" => {
                            if let Err(e) = cmd_review(&storage, &registry).await {
                                eprintln!("Error during review: {e}");
                            }
                        }

                        "model" => {
                            if arg.is_empty() {
                                println!("Current model: {}", config.llm.model);
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
                                    &user_skills_dir,
                                    registry.clone(),
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

                // Normal user input — run through the orchestrator.
                let (stop_spinner, spinner_handle) = start_spinner();
                let turn_result = orchestrator
                    .run_turn(input, conversation_id, Interface::Cli)
                    .await;
                stop_spinner.store(true, Ordering::Relaxed);
                let _ = spinner_handle.await;

                match turn_result {
                    Ok(result) => {
                        println!("\n{}\n", result.answer);
                    }
                    Err(e) => {
                        eprintln!("\nError: {e}\n");
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
