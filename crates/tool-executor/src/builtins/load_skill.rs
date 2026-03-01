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
                    let mut output = ToolOutput::success(skill.body.clone());
                    // Propagate the skill's allowed-tools list as structured
                    // data so the orchestrator can enforce tool restrictions.
                    if !skill.allowed_tools.is_empty() {
                        output = output.with_data(serde_json::json!({
                            "allowed_tools": skill.allowed_tools,
                        }));
                    }
                    Ok(output)
                }
            }
            None => Ok(ToolOutput::error(format!(
                "Skill '{}' not found in registry.",
                skill_name
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::types::Interface;
    use assistant_skills::SkillDef;
    use assistant_storage::StorageLayer;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn test_ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            turn: 0,
            interface: Interface::Cli,
            interactive: false,
            allowed_tools: None,
            depth: 0,
        }
    }

    #[tokio::test]
    async fn load_skill_with_allowed_tools_includes_data() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());

        let skill = SkillDef {
            name: "restricted".into(),
            description: "A restricted skill".into(),
            license: None,
            compatibility: None,
            allowed_tools: vec!["bash".into(), "file-read".into()],
            metadata: HashMap::new(),
            body: "# Restricted skill body".into(),
            dir: PathBuf::from("/tmp/restricted"),
            source: assistant_skills::SkillSource::Builtin,
        };
        registry.register(skill).await.unwrap();

        let handler = LoadSkillHandler::new(registry);
        let mut params = HashMap::new();
        params.insert("name".into(), serde_json::json!("restricted"));

        let output = handler.run(params, &test_ctx()).await.unwrap();
        assert!(output.success);
        assert_eq!(output.content, "# Restricted skill body");

        let data = output.data.expect("should have structured data");
        let tools = data["allowed_tools"].as_array().expect("should be array");
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0], "bash");
        assert_eq!(tools[1], "file-read");
    }

    #[tokio::test]
    async fn load_skill_without_allowed_tools_has_no_data() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());

        let skill = SkillDef {
            name: "open".into(),
            description: "An open skill".into(),
            license: None,
            compatibility: None,
            allowed_tools: vec![],
            metadata: HashMap::new(),
            body: "# Open skill body".into(),
            dir: PathBuf::from("/tmp/open"),
            source: assistant_skills::SkillSource::Builtin,
        };
        registry.register(skill).await.unwrap();

        let handler = LoadSkillHandler::new(registry);
        let mut params = HashMap::new();
        params.insert("name".into(), serde_json::json!("open"));

        let output = handler.run(params, &test_ctx()).await.unwrap();
        assert!(output.success);
        assert!(
            output.data.is_none(),
            "should not have data when no allowed_tools"
        );
    }
}
