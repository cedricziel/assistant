//! Builtin handlers for memory-read, memory-write, and memory-search skills.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput};
use assistant_storage::StorageLayer;
use async_trait::async_trait;

// ---------------------------------------------------------------------------
// MemoryReadHandler
// ---------------------------------------------------------------------------

pub struct MemoryReadHandler {
    storage: Arc<StorageLayer>,
}

impl MemoryReadHandler {
    pub fn new(storage: Arc<StorageLayer>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl SkillHandler for MemoryReadHandler {
    fn skill_name(&self) -> &str {
        "memory-read"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let key = match params.get("key").and_then(|v| v.as_str()) {
            Some(k) => k.to_string(),
            None => {
                return Ok(SkillOutput::error("Missing required parameter 'key'"));
            }
        };

        let store = self.storage.memory_store();
        match store.get(&key).await? {
            Some(entry) => {
                let time_ago = format_time_ago(entry.updated_at);
                Ok(SkillOutput::success(format!(
                    "Your {} is {} (saved {}).",
                    key, entry.value, time_ago
                )))
            }
            None => Ok(SkillOutput::success(format!(
                "I don't have anything stored under '{}'.",
                key
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryWriteHandler
// ---------------------------------------------------------------------------

pub struct MemoryWriteHandler {
    storage: Arc<StorageLayer>,
}

impl MemoryWriteHandler {
    pub fn new(storage: Arc<StorageLayer>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl SkillHandler for MemoryWriteHandler {
    fn skill_name(&self) -> &str {
        "memory-write"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let key = match params.get("key").and_then(|v| v.as_str()) {
            Some(k) => k.to_string(),
            None => {
                return Ok(SkillOutput::error("Missing required parameter 'key'"));
            }
        };

        let value = match params.get("value") {
            Some(v) => match v.as_str() {
                Some(s) => s.to_string(),
                None => v.to_string(),
            },
            None => {
                return Ok(SkillOutput::error("Missing required parameter 'value'"));
            }
        };

        let store = self.storage.memory_store();
        store.set(&key, &value, "assistant").await?;

        Ok(SkillOutput::success(format!(
            "Got it — I'll remember that {} = {}.",
            key, value
        )))
    }
}

// ---------------------------------------------------------------------------
// MemorySearchHandler
// ---------------------------------------------------------------------------

pub struct MemorySearchHandler {
    storage: Arc<StorageLayer>,
}

impl MemorySearchHandler {
    pub fn new(storage: Arc<StorageLayer>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl SkillHandler for MemorySearchHandler {
    fn skill_name(&self) -> &str {
        "memory-search"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let store = self.storage.memory_store();

        let entries = if query.is_empty() {
            store.list_all().await?
        } else {
            store.search(&query).await?
        };

        if entries.is_empty() {
            if query.is_empty() {
                return Ok(SkillOutput::success("No memories stored yet."));
            } else {
                return Ok(SkillOutput::success(format!(
                    "No memories found matching '{}'.",
                    query
                )));
            }
        }

        let mut lines = Vec::with_capacity(entries.len() + 1);
        if query.is_empty() {
            lines.push(format!("All stored memories ({} entries):", entries.len()));
        } else {
            lines.push(format!(
                "Memories matching '{}' ({} entries):",
                query,
                entries.len()
            ));
        }

        for entry in &entries {
            let time_ago = format_time_ago(entry.updated_at);
            lines.push(format!("  - {}: {} ({})", entry.key, entry.value, time_ago));
        }

        Ok(SkillOutput::success(lines.join("\n")))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_time_ago(dt: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(dt);
    let secs = diff.num_seconds();

    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        let mins = secs / 60;
        if mins == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", mins)
        }
    } else if secs < 86400 {
        let hours = secs / 3600;
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else {
        let days = secs / 86400;
        if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", days)
        }
    }
}
