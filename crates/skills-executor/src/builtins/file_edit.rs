//! Builtin handler for file-edit tool — surgically replaces text in a file.

use std::collections::HashMap;
use std::fs;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;

use super::file_read::expand_tilde;

pub struct FileEditHandler;

impl FileEditHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileEditHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for FileEditHandler {
    fn name(&self) -> &str {
        "file-edit"
    }

    fn description(&self) -> &str {
        "Surgically replace the first occurrence of old_string with new_string in a file. Returns an error if old_string is not found."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "path": {"type": "string", "description": "Absolute or ~-relative path to edit"},
            "old_string": {"type": "string", "description": "Exact text to find (returns error if not found)"},
            "new_string": {"type": "string", "description": "Replacement text"}
        })
    }

    fn is_mutating(&self) -> bool {
        true
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let path_str = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'path'")),
        };

        let old_string = match params.get("old_string").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'old_string'")),
        };

        let new_string = match params.get("new_string").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'new_string'")),
        };

        let path = expand_tilde(&path_str);

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to read '{}': {}",
                    path.display(),
                    e
                )))
            }
        };

        if !content.contains(old_string.as_str()) {
            return Ok(ToolOutput::error(format!(
                "'old_string' not found in '{}'. No changes made.",
                path.display()
            )));
        }

        let patched = content.replacen(old_string.as_str(), new_string.as_str(), 1);

        match fs::write(&path, patched.as_bytes()) {
            Ok(()) => Ok(ToolOutput::success(format!("Edited '{}'", path.display()))),
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to write '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::Interface;
    use uuid::Uuid;

    fn make_ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            turn: 1,
            interface: Interface::Cli,
            interactive: false,
        }
    }

    fn params(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[tokio::test]
    async fn replaces_first_occurrence_only() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("edit.txt");
        fs::write(&file_path, "aaa bbb aaa").unwrap();

        let handler = FileEditHandler::new();
        let ctx = make_ctx();
        let p = params(&[
            (
                "path",
                serde_json::Value::String(file_path.to_str().unwrap().to_string()),
            ),
            ("old_string", serde_json::Value::String("aaa".to_string())),
            ("new_string", serde_json::Value::String("zzz".to_string())),
        ]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(result.success, "Expected success, got: {}", result.content);
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "zzz bbb aaa");
    }

    #[tokio::test]
    async fn errors_when_not_found() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("noedit.txt");
        fs::write(&file_path, "hello world").unwrap();

        let handler = FileEditHandler::new();
        let ctx = make_ctx();
        let p = params(&[
            (
                "path",
                serde_json::Value::String(file_path.to_str().unwrap().to_string()),
            ),
            (
                "old_string",
                serde_json::Value::String("missing_text".to_string()),
            ),
            (
                "new_string",
                serde_json::Value::String("replacement".to_string()),
            ),
        ]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success);
        assert!(
            result.content.contains("not found"),
            "Got: {}",
            result.content
        );
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn file_not_found() {
        let handler = FileEditHandler::new();
        let ctx = make_ctx();
        let p = params(&[
            (
                "path",
                serde_json::Value::String("/tmp/nonexistent_edit_test_12345.txt".to_string()),
            ),
            ("old_string", serde_json::Value::String("foo".to_string())),
            ("new_string", serde_json::Value::String("bar".to_string())),
        ]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success);
        assert!(
            result.content.contains("Failed to read"),
            "Got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn missing_path_param() {
        let handler = FileEditHandler::new();
        let ctx = make_ctx();
        let p = params(&[
            ("old_string", serde_json::Value::String("foo".to_string())),
            ("new_string", serde_json::Value::String("bar".to_string())),
        ]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success);
        assert!(result.content.contains("path"), "Got: {}", result.content);
    }

    #[tokio::test]
    async fn missing_old_string_param() {
        let handler = FileEditHandler::new();
        let ctx = make_ctx();
        let p = params(&[
            (
                "path",
                serde_json::Value::String("/tmp/whatever.txt".to_string()),
            ),
            ("new_string", serde_json::Value::String("bar".to_string())),
        ]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success);
        assert!(
            result.content.contains("old_string"),
            "Got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn missing_new_string_param() {
        let handler = FileEditHandler::new();
        let ctx = make_ctx();
        let p = params(&[
            (
                "path",
                serde_json::Value::String("/tmp/whatever.txt".to_string()),
            ),
            ("old_string", serde_json::Value::String("foo".to_string())),
        ]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success);
        assert!(
            result.content.contains("new_string"),
            "Got: {}",
            result.content
        );
    }

    #[test]
    fn self_describing() {
        let handler = FileEditHandler::new();
        assert!(!handler.description().is_empty());
        assert!(handler.params_schema().is_object());
        assert!(handler.is_mutating(), "FileEditHandler should be mutating");
    }
}
