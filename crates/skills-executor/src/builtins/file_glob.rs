//! Builtin handler for file-glob tool — finds files matching a glob pattern.

use std::collections::HashMap;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;

const DEFAULT_LIMIT: usize = 200;

pub struct FileGlobHandler;

impl FileGlobHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileGlobHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for FileGlobHandler {
    fn name(&self) -> &str {
        "file-glob"
    }

    fn description(&self) -> &str {
        "Find files and directories matching a glob pattern. Returns a newline-separated list of matching paths."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "Glob pattern, e.g. **/*.rs or ~/notes/*.md"},
                "limit": {"type": "number", "description": "Max results (default: 200)"}
            },
            "required": ["pattern"]
        })
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let pattern_raw = match params.get("pattern").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'pattern'")),
        };

        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_LIMIT);

        // Expand ~ in the pattern prefix (before any glob metacharacter).
        let pattern = if let Some(rest) = pattern_raw.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                format!("{}/{}", home.display(), rest)
            } else {
                pattern_raw
            }
        } else {
            pattern_raw
        };

        let entries = match glob::glob(&pattern) {
            Ok(paths) => paths,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Invalid glob pattern '{}': {}",
                    pattern, e
                )))
            }
        };

        let mut results: Vec<String> = Vec::new();
        let mut truncated = false;

        for entry in entries {
            if results.len() >= limit {
                truncated = true;
                break;
            }
            match entry {
                Ok(path) => results.push(path.display().to_string()),
                Err(e) => results.push(format!("[error reading entry: {}]", e)),
            }
        }

        if results.is_empty() {
            return Ok(ToolOutput::success(format!(
                "No files matched pattern '{}'",
                pattern
            )));
        }

        let mut output = results.join("\n");
        if truncated {
            output.push_str(&format!("\n\n[Results truncated at {} entries]", limit));
        }

        Ok(ToolOutput::success(output))
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
    async fn finds_matching_files() {
        let dir = tempfile::TempDir::new().unwrap();
        for name in &["one.txt", "two.txt", "three.txt"] {
            std::fs::write(dir.path().join(name), "content").unwrap();
        }

        let handler = FileGlobHandler::new();
        let ctx = make_ctx();
        let pattern = format!("{}/*.txt", dir.path().display());
        let p = params(&[("pattern", serde_json::Value::String(pattern))]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(result.success, "Expected success, got: {}", result.content);
        assert!(
            result.content.contains("one.txt"),
            "Missing one.txt in: {}",
            result.content
        );
        assert!(
            result.content.contains("two.txt"),
            "Missing two.txt in: {}",
            result.content
        );
        assert!(
            result.content.contains("three.txt"),
            "Missing three.txt in: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn respects_limit() {
        let dir = tempfile::TempDir::new().unwrap();
        for i in 0..5 {
            std::fs::write(dir.path().join(format!("file{}.txt", i)), "content").unwrap();
        }

        let handler = FileGlobHandler::new();
        let ctx = make_ctx();
        let pattern = format!("{}/*.txt", dir.path().display());
        let p = params(&[
            ("pattern", serde_json::Value::String(pattern)),
            ("limit", serde_json::json!(2)),
        ]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(result.success);
        assert!(
            result.content.contains("[Results truncated"),
            "Expected truncation marker, got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn no_matches() {
        let dir = tempfile::TempDir::new().unwrap();

        let handler = FileGlobHandler::new();
        let ctx = make_ctx();
        let pattern = format!("{}/*.nonexistent_ext", dir.path().display());
        let p = params(&[("pattern", serde_json::Value::String(pattern))]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(result.success);
        assert!(
            result.content.contains("No files matched"),
            "Got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn missing_pattern_param() {
        let handler = FileGlobHandler::new();
        let ctx = make_ctx();
        let p = params(&[]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success);
        assert!(
            result.content.contains("pattern"),
            "Got: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn invalid_pattern_returns_error() {
        let handler = FileGlobHandler::new();
        let ctx = make_ctx();
        let p = params(&[(
            "pattern",
            serde_json::Value::String("/tmp/[invalid".to_string()),
        )]);

        let result = handler.run(p, &ctx).await.unwrap();
        assert!(!result.success);
        assert!(
            result.content.contains("Invalid glob pattern"),
            "Got: {}",
            result.content
        );
    }

    #[test]
    fn self_describing() {
        let handler = FileGlobHandler::new();
        assert!(!handler.description().is_empty());
        assert!(handler.params_schema().is_object());
        assert!(
            !handler.is_mutating(),
            "FileGlobHandler should not be mutating"
        );
    }
}
