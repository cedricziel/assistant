//! Slash-command registry and built-in command implementations.
//!
//! [`CommandRegistry`] holds all command definitions, resolves incoming text,
//! and dispatches execution.  All messenger interfaces and the CLI share the
//! same registry instance.

use std::collections::HashMap;
use std::sync::Arc;

use assistant_core::{CommandArg, CommandDef, CommandResult, ConversationConfig};
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::compaction;
use crate::orchestrator::Orchestrator;

// -- Execution context --------------------------------------------------------

/// Context passed to each command handler during execution.
pub struct CommandContext {
    /// Conversation this command targets.
    pub conversation_id: Uuid,
    /// Shared per-conversation config map.
    pub conversation_configs: Arc<RwLock<HashMap<Uuid, ConversationConfig>>>,
    /// Orchestrator reference (for `/stop`, `/compact`, `/status`).
    pub orchestrator: Arc<Orchestrator>,
    /// Active turns map: conv_id → request_id (for `/stop`).
    pub active_turns: Arc<RwLock<HashMap<Uuid, Uuid>>>,
    /// Callback to evict the conversation key from the LRU cache (for `/new`).
    /// When called, the next message from this context will create a new conversation.
    pub evict_conversation: Option<Box<dyn FnOnce() + Send>>,
    /// Default model name from global config.
    pub default_model: String,
}

// -- Registry -----------------------------------------------------------------

/// Central registry of slash commands.
pub struct CommandRegistry {
    commands: Vec<CommandDef>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistry {
    /// Create a new registry with all built-in commands.
    pub fn new() -> Self {
        let commands = vec![
            CommandDef {
                name: "new".into(),
                description: "Start a new conversation".into(),
                category: "session".into(),
                args: vec![],
            },
            CommandDef {
                name: "stop".into(),
                description: "Cancel the current turn".into(),
                category: "session".into(),
                args: vec![],
            },
            CommandDef {
                name: "model".into(),
                description: "Switch the model for this conversation".into(),
                category: "config".into(),
                args: vec![CommandArg {
                    name: "model_name".into(),
                    required: false,
                    completions_endpoint: Some("/api/models".into()),
                }],
            },
            CommandDef {
                name: "compact".into(),
                description: "Compress conversation context".into(),
                category: "session".into(),
                args: vec![],
            },
            CommandDef {
                name: "status".into(),
                description: "Show conversation status".into(),
                category: "debug".into(),
                args: vec![],
            },
            CommandDef {
                name: "help".into(),
                description: "List available commands".into(),
                category: "debug".into(),
                args: vec![],
            },
        ];
        Self { commands }
    }

    /// Return all registered command definitions.
    pub fn list(&self) -> &[CommandDef] {
        &self.commands
    }

    /// Look up a command by name.
    pub fn get(&self, name: &str) -> Option<&CommandDef> {
        self.commands.iter().find(|c| c.name == name)
    }

    /// Try to parse a message as a slash command.
    ///
    /// Returns `Some((command_name, args))` if the text starts with `/`
    /// followed by a registered command name.  Returns `None` for unrecognized
    /// `/`-prefixed text (which should be dispatched as a normal message).
    pub fn parse(&self, text: &str) -> Option<(String, Vec<String>)> {
        let text = text.trim();
        if !text.starts_with('/') {
            return None;
        }
        let without_slash = &text[1..];
        let mut parts = without_slash.splitn(2, ' ');
        let cmd_name = parts.next()?;

        // Only match exact registered command names.
        self.get(cmd_name)?;

        let args: Vec<String> = parts
            .next()
            .map(|rest| rest.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        Some((cmd_name.to_string(), args))
    }

    /// Execute a command.
    pub async fn execute(&self, name: &str, args: &[String], ctx: CommandContext) -> CommandResult {
        match name {
            "help" => self.cmd_help(),
            "new" => self.cmd_new(ctx).await,
            "stop" => self.cmd_stop(ctx).await,
            "model" => self.cmd_model(args, ctx).await,
            "status" => self.cmd_status(ctx).await,
            "compact" => self.cmd_compact(ctx).await,
            _ => CommandResult::ack(format!("Unknown command: {name}")),
        }
    }

    // -- Built-in command implementations -------------------------------------

    fn cmd_help(&self) -> CommandResult {
        let mut lines = vec!["Available commands:".to_string()];
        for cmd in &self.commands {
            let args_hint = if cmd.args.is_empty() {
                String::new()
            } else {
                let names: Vec<_> = cmd
                    .args
                    .iter()
                    .map(|a| {
                        if a.required {
                            format!("<{}>", a.name)
                        } else {
                            format!("[{}]", a.name)
                        }
                    })
                    .collect();
                format!(" {}", names.join(" "))
            };
            lines.push(format!(
                "  /{}{} — {}",
                cmd.name, args_hint, cmd.description
            ));
        }
        CommandResult::ack(lines.join("\n"))
    }

    async fn cmd_new(&self, ctx: CommandContext) -> CommandResult {
        // Clear per-conversation config.
        ctx.conversation_configs
            .write()
            .await
            .remove(&ctx.conversation_id);

        // Evict the conversation key so the next message gets a new UUID.
        if let Some(evict) = ctx.evict_conversation {
            evict();
        }

        info!(conv_id = %ctx.conversation_id, "Conversation reset via /new");
        CommandResult::ack("New conversation started.")
    }

    async fn cmd_stop(&self, ctx: CommandContext) -> CommandResult {
        // Look up the active request_id for this conversation.
        let request_id = ctx
            .active_turns
            .read()
            .await
            .get(&ctx.conversation_id)
            .copied();

        match request_id {
            Some(rid) => {
                // Find the cancellation token in the orchestrator.
                let token = ctx
                    .orchestrator
                    .turn_cancellations
                    .read()
                    .await
                    .get(&rid)
                    .cloned();

                if let Some(t) = token {
                    t.cancel();
                    info!(conv_id = %ctx.conversation_id, request_id = %rid, "Turn cancelled via /stop");
                    CommandResult::ack("Stopped.")
                } else {
                    CommandResult::ack("Nothing to stop.")
                }
            }
            None => CommandResult::ack("Nothing to stop."),
        }
    }

    async fn cmd_model(&self, args: &[String], ctx: CommandContext) -> CommandResult {
        if args.is_empty() {
            // Show current model.
            let configs = ctx.conversation_configs.read().await;
            let current = configs
                .get(&ctx.conversation_id)
                .and_then(|c| c.model_override.as_deref());
            match current {
                Some(m) => CommandResult::ack(format!("Current model: {m} (override)")),
                None => {
                    CommandResult::ack(format!("Current model: {} (default)", ctx.default_model))
                }
            }
        } else {
            let model_name = args.join(" ");
            let mut configs = ctx.conversation_configs.write().await;
            let entry = configs
                .entry(ctx.conversation_id)
                .or_insert_with(ConversationConfig::default);
            entry.model_override = Some(model_name.clone());
            info!(conv_id = %ctx.conversation_id, model = %model_name, "Model override set via /model");
            CommandResult::ack(format!("Model switched to {model_name}."))
        }
    }

    async fn cmd_status(&self, ctx: CommandContext) -> CommandResult {
        let configs = ctx.conversation_configs.read().await;
        let conv_cfg = configs.get(&ctx.conversation_id);
        let model = conv_cfg
            .and_then(|c| c.model_override.as_deref())
            .unwrap_or(&ctx.default_model);
        let model_source = if conv_cfg.and_then(|c| c.model_override.as_ref()).is_some() {
            "override"
        } else {
            "default"
        };

        let is_active = ctx
            .active_turns
            .read()
            .await
            .contains_key(&ctx.conversation_id);

        let mut lines = vec![
            format!("Conversation: {}", ctx.conversation_id),
            format!("Model: {model} ({model_source})"),
            format!("Turn active: {}", if is_active { "yes" } else { "no" }),
        ];

        // Estimate token usage from history if possible.
        let conv_store = ctx.orchestrator.storage.conversation_store();
        if let Ok(messages) = conv_store.load_history(ctx.conversation_id).await {
            let history = crate::history::messages_to_chat_history(messages, &Default::default());
            let tokens = compaction::estimate_tokens(&history);
            lines.push(format!("Estimated tokens: ~{tokens}"));
        }

        CommandResult::ack(lines.join("\n"))
    }

    async fn cmd_compact(&self, ctx: CommandContext) -> CommandResult {
        let conv_store = ctx.orchestrator.storage.conversation_store();

        // Load conversation history.
        let messages = match conv_store.load_history(ctx.conversation_id).await {
            Ok(m) => m,
            Err(e) => {
                warn!(error = %e, "Failed to load messages for /compact");
                return CommandResult::ack("Failed to load conversation history.");
            }
        };

        let mut history = crate::history::messages_to_chat_history(messages, &Default::default());

        if history.is_empty() {
            return CommandResult::ack("Nothing to compact.");
        }

        let compacted = compaction::maybe_compact(
            &mut history,
            &ctx.orchestrator.llm,
            &ctx.orchestrator.compaction_config,
            Some((&conv_store, ctx.conversation_id)),
        )
        .await;

        if compacted {
            info!(conv_id = %ctx.conversation_id, "Context compacted via /compact");
            CommandResult::ack("Context compacted.")
        } else {
            CommandResult::ack("Nothing to compact.")
        }
    }
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lists_all_builtins() {
        let reg = CommandRegistry::new();
        let names: Vec<_> = reg.list().iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"new"));
        assert!(names.contains(&"stop"));
        assert!(names.contains(&"model"));
        assert!(names.contains(&"compact"));
        assert!(names.contains(&"status"));
        assert!(names.contains(&"help"));
        assert_eq!(names.len(), 6);
    }

    #[test]
    fn get_existing_command() {
        let reg = CommandRegistry::new();
        let cmd = reg.get("model");
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap().category, "config");
    }

    #[test]
    fn get_unknown_command() {
        let reg = CommandRegistry::new();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn parse_recognized_command_no_args() {
        let reg = CommandRegistry::new();
        let result = reg.parse("/new");
        assert_eq!(result, Some(("new".into(), vec![])));
    }

    #[test]
    fn parse_recognized_command_with_args() {
        let reg = CommandRegistry::new();
        let result = reg.parse("/model claude-opus-4");
        assert_eq!(result, Some(("model".into(), vec!["claude-opus-4".into()])));
    }

    #[test]
    fn parse_unrecognized_slash_text() {
        let reg = CommandRegistry::new();
        let result = reg.parse("/new-york pizza");
        assert!(result.is_none(), "should not match partial command names");
    }

    #[test]
    fn parse_non_slash_text() {
        let reg = CommandRegistry::new();
        assert!(reg.parse("hello world").is_none());
    }

    #[test]
    fn parse_with_leading_whitespace() {
        let reg = CommandRegistry::new();
        let result = reg.parse("  /help");
        assert_eq!(result, Some(("help".into(), vec![])));
    }

    #[test]
    fn help_output_contains_all_commands() {
        let reg = CommandRegistry::new();
        let result = reg.cmd_help();
        assert!(result.ack_text.contains("/new"));
        assert!(result.ack_text.contains("/stop"));
        assert!(result.ack_text.contains("/model"));
        assert!(result.ack_text.contains("/compact"));
        assert!(result.ack_text.contains("/status"));
        assert!(result.ack_text.contains("/help"));
    }
}
