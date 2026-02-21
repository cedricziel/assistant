//! Builtin handler for list-skills skill — lists all registered skills as a table.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput};
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
impl SkillHandler for ListSkillsHandler {
    fn skill_name(&self) -> &str {
        "list-skills"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        _params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let skills = self.registry.list().await;

        if skills.is_empty() {
            return Ok(SkillOutput::success("No skills registered."));
        }

        // Build a formatted table
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!(
            "{:<24} {:<62} {:<8} {}",
            "Name", "Description", "Tier", "Source"
        ));
        lines.push(format!(
            "{} {} {} {}",
            "-".repeat(24),
            "-".repeat(62),
            "-".repeat(8),
            "-".repeat(10)
        ));

        for skill in &skills {
            let desc = truncate(&skill.description, DESC_TRUNCATE);
            lines.push(format!(
                "{:<24} {:<62} {:<8} {}",
                skill.name,
                desc,
                skill.tier.label(),
                skill.source,
            ));
        }

        lines.push(String::new());
        lines.push(format!("Total: {} skill(s)", skills.len()));

        Ok(SkillOutput::success(lines.join("\n")))
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        // Find a char boundary
        let mut end = max_len.saturating_sub(3);
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}
