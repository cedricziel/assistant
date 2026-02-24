//! Builtin handler for skill-file-read — reads auxiliary files from a skill directory.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput};
use assistant_storage::SkillRegistry;
use async_trait::async_trait;

pub struct SkillFileReadHandler {
    registry: Arc<SkillRegistry>,
}

impl SkillFileReadHandler {
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl SkillHandler for SkillFileReadHandler {
    fn skill_name(&self) -> &str {
        "skill-file-read"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let skill_name = match params.get("skill").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => return Ok(SkillOutput::error("Missing required parameter: skill")),
        };

        let path = match params.get("path").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => return Ok(SkillOutput::error("Missing required parameter: path")),
        };

        // Reject paths with ".." components to prevent directory traversal
        if Path::new(path)
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Ok(SkillOutput::error(
                "Invalid path: directory traversal (..) is not allowed",
            ));
        }

        // Look up the skill
        let skill = match self.registry.get(skill_name).await {
            Some(s) => s,
            None => {
                return Ok(SkillOutput::error(format!("Skill not found: {skill_name}")));
            }
        };

        let file_path = skill.dir.join(path);

        // Verify the resolved path is still within the skill directory
        let canonical_dir = match skill.dir.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                return Ok(SkillOutput::error(format!(
                    "Skill directory not accessible: {}",
                    skill.dir.display()
                )));
            }
        };
        let canonical_file = match file_path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                return Ok(SkillOutput::error(format!(
                    "File not found: {path} in skill {skill_name}"
                )));
            }
        };
        if !canonical_file.starts_with(&canonical_dir) {
            return Ok(SkillOutput::error(
                "Invalid path: resolved path is outside the skill directory",
            ));
        }

        // Read the file
        match tokio::fs::read_to_string(&canonical_file).await {
            Ok(contents) => Ok(SkillOutput::success(contents)),
            Err(e) => Ok(SkillOutput::error(format!(
                "Failed to read {path} in skill {skill_name}: {e}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::types::{ExecutionContext, Interface};
    use assistant_skills::{SkillDef as SkillsSkillDef, SkillSource};
    use assistant_storage::{SkillRegistry, StorageLayer};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn test_ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            turn: 0,
            interface: Interface::Cli,
            interactive: false,
        }
    }

    fn make_skill_def(name: &str, _dir: &std::path::Path) -> SkillDef {
        // Returns the old assistant_core::SkillDef used as the `_def` parameter
        // in execute() calls — the handler ignores it, so only the name matters.
        use assistant_core::skill::{SkillSource as OldSource, SkillTier};
        SkillDef {
            name: name.to_string(),
            description: "Test skill".to_string(),
            license: None,
            compatibility: None,
            allowed_tools: vec![],
            metadata: std::collections::HashMap::new(),
            body: String::new(),
            dir: _dir.to_path_buf(),
            tier: SkillTier::Builtin,
            mutating: false,
            confirmation_required: false,
            source: OldSource::Builtin,
        }
    }

    fn make_skills_def(name: &str, dir: &std::path::Path) -> SkillsSkillDef {
        SkillsSkillDef {
            name: name.to_string(),
            description: "Test skill".to_string(),
            license: None,
            compatibility: None,
            allowed_tools: Vec::new(),
            metadata: std::collections::HashMap::new(),
            tier: None,
            mutating: false,
            confirmation_required: false,
            params_schema: None,
            body: String::new(),
            dir: dir.to_path_buf(),
            source: SkillSource::Builtin,
        }
    }

    async fn make_registry_with(name: &str, dir: &std::path::Path) -> Arc<SkillRegistry> {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = SkillRegistry::new(storage.pool.clone()).await.unwrap();
        registry.register(make_skills_def(name, dir)).await.unwrap();
        Arc::new(registry)
    }

    fn params(skill: &str, path: &str) -> HashMap<String, serde_json::Value> {
        let mut m = HashMap::new();
        m.insert(
            "skill".to_string(),
            serde_json::Value::String(skill.to_string()),
        );
        m.insert(
            "path".to_string(),
            serde_json::Value::String(path.to_string()),
        );
        m
    }

    #[tokio::test]
    async fn returns_file_contents() {
        let tmp = TempDir::new().unwrap();
        let refs = tmp.path().join("references");
        std::fs::create_dir(&refs).unwrap();
        std::fs::write(refs.join("FORMS.md"), "# Forms\nField: foo").unwrap();

        let def = make_skill_def("my-skill", tmp.path());
        let registry = make_registry_with("my-skill", tmp.path()).await;
        let handler = SkillFileReadHandler::new(registry);

        let result = handler
            .execute(&def, params("my-skill", "references/FORMS.md"), &test_ctx())
            .await
            .unwrap();
        assert!(result.success, "expected success, got: {}", result.content);
        assert!(result.content.contains("Field: foo"));
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        let tmp = TempDir::new().unwrap();
        let def = make_skill_def("my-skill", tmp.path());
        let registry = make_registry_with("my-skill", tmp.path()).await;
        let handler = SkillFileReadHandler::new(registry);

        let result = handler
            .execute(&def, params("my-skill", "../etc/passwd"), &test_ctx())
            .await
            .unwrap();
        assert!(!result.success);
        assert!(
            result.content.contains("traversal"),
            "expected traversal error, got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn missing_skill_param_errors() {
        let tmp = TempDir::new().unwrap();
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let handler = SkillFileReadHandler::new(registry);

        let mut p = HashMap::new();
        p.insert(
            "path".to_string(),
            serde_json::Value::String("references/FORMS.md".to_string()),
        );
        let def = make_skill_def("x", tmp.path());
        let result = handler.execute(&def, p, &test_ctx()).await.unwrap();
        assert!(!result.success);
        assert!(result.content.contains("Missing required parameter: skill"));
    }

    #[tokio::test]
    async fn missing_path_param_errors() {
        let tmp = TempDir::new().unwrap();
        let def = make_skill_def("my-skill", tmp.path());
        let registry = make_registry_with("my-skill", tmp.path()).await;
        let handler = SkillFileReadHandler::new(registry);

        let mut p = HashMap::new();
        p.insert(
            "skill".to_string(),
            serde_json::Value::String("my-skill".to_string()),
        );
        let result = handler.execute(&def, p, &test_ctx()).await.unwrap();
        assert!(!result.success);
        assert!(result.content.contains("Missing required parameter: path"));
    }

    #[tokio::test]
    async fn skill_not_found_errors() {
        let tmp = TempDir::new().unwrap();
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let handler = SkillFileReadHandler::new(registry);

        let def = make_skill_def("x", tmp.path());
        let result = handler
            .execute(
                &def,
                params("nonexistent", "references/file.md"),
                &test_ctx(),
            )
            .await
            .unwrap();
        assert!(!result.success);
        assert!(
            result.content.contains("not found"),
            "got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn file_not_found_errors() {
        let tmp = TempDir::new().unwrap();
        let def = make_skill_def("my-skill", tmp.path());
        let registry = make_registry_with("my-skill", tmp.path()).await;
        let handler = SkillFileReadHandler::new(registry);

        let result = handler
            .execute(
                &def,
                params("my-skill", "references/nonexistent.md"),
                &test_ctx(),
            )
            .await
            .unwrap();
        assert!(!result.success);
    }
}
