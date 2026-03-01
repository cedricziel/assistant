//! Builtin handler for list-skills tool — lists all registered skills as a table.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use assistant_storage::SkillRegistry;
use async_trait::async_trait;

const DESC_TRUNCATE: usize = 60;

pub struct ListSkillsHandler {
    registry: Arc<SkillRegistry>,
}

impl ListSkillsHandler {
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl ToolHandler for ListSkillsHandler {
    fn name(&self) -> &str {
        "list-skills"
    }

    fn description(&self) -> &str {
        "List all registered skills with their name and description"
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn run(
        &self,
        _params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let skills = self.registry.list().await;

        if skills.is_empty() {
            return Ok(ToolOutput::success("No skills registered."));
        }

        // Build a formatted table
        let has_compat = skills.iter().any(|s| s.compatibility.is_some());

        let mut lines: Vec<String> = Vec::new();
        if has_compat {
            lines.push(format!(
                "{:<24} {:<60} {:<10} {}",
                "Name", "Description", "Source", "Compatibility"
            ));
            lines.push(format!(
                "{} {} {} {}",
                "-".repeat(24),
                "-".repeat(60),
                "-".repeat(10),
                "-".repeat(40),
            ));
        } else {
            lines.push(format!("{:<24} {:<70} {}", "Name", "Description", "Source"));
            lines.push(format!(
                "{} {} {}",
                "-".repeat(24),
                "-".repeat(70),
                "-".repeat(10),
            ));
        }

        for skill in &skills {
            let desc_len = if has_compat { 56 } else { DESC_TRUNCATE };
            let desc = truncate(&skill.description, desc_len);
            if has_compat {
                let compat = skill
                    .compatibility
                    .as_deref()
                    .map(|c| truncate(c, 40))
                    .unwrap_or_default();
                lines.push(format!(
                    "{:<24} {:<60} {:<10} {}",
                    skill.name, desc, skill.source, compat,
                ));
            } else {
                lines.push(format!("{:<24} {:<70} {}", skill.name, desc, skill.source,));
            }
        }

        lines.push(String::new());
        lines.push(format!("Total: {} skill(s)", skills.len()));

        Ok(ToolOutput::success(lines.join("\n")))
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut end = max_len.saturating_sub(3);
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}
