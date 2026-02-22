//! Builtin handler for file-write tool — writes content to any file on disk.

use std::collections::HashMap;
use std::fs;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;

use super::file_read::expand_tilde;

pub struct FileWriteHandler;

impl FileWriteHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileWriteHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for FileWriteHandler {
    fn name(&self) -> &str {
        "file-write"
    }

    fn description(&self) -> &str {
        "Write content to a file, creating it and parent directories if needed. Completely replaces existing content."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Absolute or ~-relative path to write"},
                "content": {"type": "string", "description": "Content to write"}
            },
            "required": ["path", "content"]
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

        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'content'")),
        };

        let path = expand_tilde(&path_str);

        // Create parent directories if needed.
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Ok(ToolOutput::error(format!(
                    "Failed to create directories for '{}': {}",
                    path.display(),
                    e
                )));
            }
        }

        match fs::write(&path, content.as_bytes()) {
            Ok(()) => Ok(ToolOutput::success(format!(
                "Written {} bytes to '{}'",
                content.len(),
                path.display()
            ))),
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
    async fn writes_new_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("new.txt");

        let handler = FileWriteHandler::new();
        let ctx = make_ctx();
        let p = params(&[
            (
                "path",
                serde_json::Value::String(file_path.to_str().unwrap().to_string()),
            ),
            (
                "content",
                serde_json::Value::String("hello content".to_string()),
            ),
        ]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(result.success, "Expected success, got: {}", result.content);
        assert!(file_path.exists());
        let written = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, "hello content");
    }

    #[tokio::test]
    async fn creates_parent_dirs() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("a").join("b").join("c.txt");

        let handler = FileWriteHandler::new();
        let ctx = make_ctx();
        let p = params(&[
            (
                "path",
                serde_json::Value::String(file_path.to_str().unwrap().to_string()),
            ),
            (
                "content",
                serde_json::Value::String("nested content".to_string()),
            ),
        ]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(result.success, "Expected success, got: {}", result.content);
        assert!(file_path.exists());
        let written = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, "nested content");
    }

    #[tokio::test]
    async fn overwrites_existing_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("overwrite.txt");
        fs::write(&file_path, "original").unwrap();

        let handler = FileWriteHandler::new();
        let ctx = make_ctx();
        let p = params(&[
            (
                "path",
                serde_json::Value::String(file_path.to_str().unwrap().to_string()),
            ),
            ("content", serde_json::Value::String("replaced".to_string())),
        ]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(result.success);
        let written = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, "replaced");
    }

    #[tokio::test]
    async fn missing_path_param() {
        let handler = FileWriteHandler::new();
        let ctx = make_ctx();
        let p = params(&[(
            "content",
            serde_json::Value::String("something".to_string()),
        )]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success);
        assert!(result.content.contains("path"), "Got: {}", result.content);
    }

    #[tokio::test]
    async fn missing_content_param() {
        let handler = FileWriteHandler::new();
        let ctx = make_ctx();
        let p = params(&[(
            "path",
            serde_json::Value::String("/tmp/whatever.txt".to_string()),
        )]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success);
        assert!(
            result.content.contains("content"),
            "Got: {}",
            result.content
        );
    }

    #[test]
    fn self_describing() {
        let handler = FileWriteHandler::new();
        assert!(!handler.description().is_empty());
        assert!(handler.params_schema().is_object());
        assert!(handler.is_mutating(), "FileWriteHandler should be mutating");
    }
}
