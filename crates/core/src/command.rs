//! Slash command types shared across all interfaces.
//!
//! [`CommandDef`] describes a command's metadata (name, description, args),
//! and [`CommandResult`] carries the outcome of executing one.

use serde::{Deserialize, Serialize};

/// A single argument definition for a slash command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandArg {
    /// Argument name shown in help/autocomplete.
    pub name: String,
    /// Whether this argument is required.
    pub required: bool,
    /// Optional API endpoint that returns completions for this argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions_endpoint: Option<String>,
}

/// Definition of a slash command, used for registration and autocomplete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDef {
    /// Command name without the leading `/` (e.g. `"new"`, `"model"`).
    pub name: String,
    /// One-line description for help text and autocomplete.
    pub description: String,
    /// Grouping category (e.g. `"session"`, `"config"`, `"debug"`).
    pub category: String,
    /// Argument definitions.
    pub args: Vec<CommandArg>,
}

/// The result of executing a slash command.
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Acknowledgement text sent back to the user.
    pub ack_text: String,
}

impl CommandResult {
    /// Create a successful result with the given acknowledgement.
    pub fn ack(text: impl Into<String>) -> Self {
        Self {
            ack_text: text.into(),
        }
    }
}

/// Per-conversation configuration overrides set by slash commands.
///
/// Stored in-memory; reset when the conversation is replaced via `/new`
/// or when the process restarts.
#[derive(Debug, Clone, Default)]
pub struct ConversationConfig {
    /// Model override set by `/model <name>`.  When `Some`, the orchestrator
    /// uses this model instead of the global default.
    pub model_override: Option<String>,
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_def_construction() {
        let def = CommandDef {
            name: "model".into(),
            description: "Switch model".into(),
            category: "config".into(),
            args: vec![CommandArg {
                name: "model_name".into(),
                required: true,
                completions_endpoint: Some("/api/models".into()),
            }],
        };
        assert_eq!(def.name, "model");
        assert_eq!(def.args.len(), 1);
        assert!(def.args[0].required);
    }

    #[test]
    fn command_def_no_args() {
        let def = CommandDef {
            name: "new".into(),
            description: "Start new conversation".into(),
            category: "session".into(),
            args: vec![],
        };
        assert!(def.args.is_empty());
    }

    #[test]
    fn command_result_ack() {
        let result = CommandResult::ack("Model switched to claude-opus-4.");
        assert_eq!(result.ack_text, "Model switched to claude-opus-4.");
    }

    #[test]
    fn conversation_config_default() {
        let cfg = ConversationConfig::default();
        assert!(cfg.model_override.is_none());
    }

    #[test]
    fn conversation_config_with_override() {
        let cfg = ConversationConfig {
            model_override: Some("claude-opus-4".into()),
        };
        assert_eq!(cfg.model_override.as_deref(), Some("claude-opus-4"));
    }
}
