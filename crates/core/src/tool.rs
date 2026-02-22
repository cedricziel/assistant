//! The `ToolHandler` trait for primitive, self-describing tools.
//!
//! Unlike [`SkillHandler`](crate::skill::SkillHandler) which works alongside
//! `SKILL.md` files, a `ToolHandler` is always self-describing — it embeds its
//! own name, description, and params schema in code.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use crate::skill::SkillOutput;
use crate::types::ExecutionContext;

/// Type alias: tool output has the same shape as skill output.
pub type ToolOutput = SkillOutput;

/// A primitive, self-describing tool handler.
///
/// Every method except `run` has a required return value (no `Option`),
/// because tools are *always* self-describing.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// The tool name (kebab-case, e.g. "file-read").
    fn name(&self) -> &str;

    /// Short description of what this tool does (1-2 sentences).
    fn description(&self) -> &str;

    /// Full JSON Schema object for the tool's parameters.
    ///
    /// Must return a proper JSON Schema with `type: "object"`, `properties`,
    /// and `required` (listing mandatory parameters). Example:
    /// ```json
    /// { "type": "object",
    ///   "properties": { "path": {"type":"string","description":"..."} },
    ///   "required": ["path"] }
    /// ```
    fn params_schema(&self) -> Value;

    /// Whether this tool mutates state (used for SafetyGate).
    fn is_mutating(&self) -> bool {
        false
    }

    /// Whether the user must confirm before this tool runs.
    fn requires_confirmation(&self) -> bool {
        false
    }

    /// Execute the tool with the given parameters.
    async fn run(
        &self,
        params: HashMap<String, Value>,
        ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput>;
}
