//! Builtin handler for `load_skill` tool — loads skill body text into context.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use assistant_storage::SkillRegistry;
use async_trait::async_trait;

pub struct LoadSkillHandler {
    registry: Arc<SkillRegistry>,
}

impl LoadSkillHandler {
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl ToolHandler for LoadSkillHandler {
    fn name(&self) -> &str {
        "load-skill"
    }

    fn description(&self) -> &str {
        "Load the body text of a skill into context by name. Returns the skill's full Markdown body."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Skill name (kebab-case)"
                }
            },
            "required": ["name"]
        })
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let skill_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => return Ok(ToolOutput::error("Missing required parameter 'name'")),
        };

        match self.registry.get(&skill_name).await {
            Some(skill) => {
                if skill.body.is_empty() {
                    Ok(ToolOutput::error(format!(
                        "Skill '{}' has no body text.",
                        skill_name
                    )))
                } else {
                    Ok(ToolOutput::success(skill.body.clone()))
                }
            }
            None => Ok(ToolOutput::error(format!(
                "Skill '{}' not found in registry.",
                skill_name
            ))),
        }
    }
}
