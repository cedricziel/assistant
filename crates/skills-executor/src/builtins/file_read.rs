//! Builtin handler for file-read tool — reads any file from disk.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;

const DEFAULT_LIMIT: usize = 8_000;

pub struct FileReadHandler;

impl FileReadHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileReadHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for FileReadHandler {
    fn name(&self) -> &str {
        "file-read"
    }

    fn description(&self) -> &str {
        "Read the contents of any file from disk. Returns text content, optionally truncated to a character limit."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "path": {"type": "string", "description": "Absolute or ~-relative path to the file"},
            "limit": {"type": "number", "description": "Max chars to return (default: 8000)"}
        })
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

        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_LIMIT);

        let path = expand_tilde(&path_str);

        match fs::read_to_string(&path) {
            Ok(content) => {
                let text = content.trim_end();
                let output = if text.len() > limit {
                    let mut end = limit;
                    while end > 0 && !text.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!(
                        "File: {}\n\n{}\n\n[Content truncated at {} characters]",
                        path.display(),
                        &text[..end],
                        limit
                    )
                } else {
                    format!("File: {}\n\n{}", path.display(), text)
                };
                Ok(ToolOutput::success(output))
            }
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to read '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}

pub(crate) fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::Interface;
    use std::io::Write;
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
    async fn reads_existing_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("hello.txt");
        let mut f = std::fs::File::create(&file_path).unwrap();
        write!(f, "Hello, world!").unwrap();

        let handler = FileReadHandler::new();
        let ctx = make_ctx();
        let p = params(&[(
            "path",
            serde_json::Value::String(file_path.to_str().unwrap().to_string()),
        )]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(result.success, "Expected success, got: {}", result.content);
        assert!(
            result.content.contains("Hello, world!"),
            "Expected content to contain 'Hello, world!', got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn respects_limit() {
        let dir = tempfile::TempDir::new().unwrap();
        let file_path = dir.path().join("big.txt");
        let content = "a".repeat(500);
        std::fs::write(&file_path, &content).unwrap();

        let handler = FileReadHandler::new();
        let ctx = make_ctx();
        let p = params(&[
            (
                "path",
                serde_json::Value::String(file_path.to_str().unwrap().to_string()),
            ),
            ("limit", serde_json::json!(100)),
        ]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(result.success);
        assert!(
            result.content.contains("[Content truncated"),
            "Expected truncation marker, got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn missing_file_returns_error() {
        let handler = FileReadHandler::new();
        let ctx = make_ctx();
        let p = params(&[(
            "path",
            serde_json::Value::String("/tmp/nonexistent_test_file_12345.txt".to_string()),
        )]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success, "Expected error for missing file");
        assert!(
            result.content.contains("Failed to read"),
            "Got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn missing_path_param() {
        let handler = FileReadHandler::new();
        let ctx = make_ctx();
        let p = params(&[]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success);
        assert!(
            result.content.contains("path"),
            "Expected error about 'path', got: {}",
            result.content
        );
    }

    #[test]
    fn expand_tilde_with_prefix() {
        let path = expand_tilde("~/some/path");
        assert!(
            !path.to_str().unwrap().starts_with("~/"),
            "Tilde was not expanded: {}",
            path.display()
        );
    }

    #[test]
    fn expand_tilde_without_prefix() {
        let path = expand_tilde("/absolute/path");
        assert_eq!(path, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn self_describing() {
        let handler = FileReadHandler::new();
        assert!(
            !handler.description().is_empty(),
            "FileReadHandler should provide a description"
        );
        assert!(
            handler.params_schema().is_object(),
            "FileReadHandler should provide a params schema"
        );
        assert!(
            !handler.is_mutating(),
            "FileReadHandler should not be mutating"
        );
    }
}
