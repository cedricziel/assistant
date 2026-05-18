//! `StubToolDispatcher` — minimal in-memory [`ToolDispatcher`] for tests.
//!
//! Records every call and either replays canned [`ToolOutput`] responses
//! or returns a benign success. Use this in tests for orchestrator code
//! that doesn't need a real tool registry.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use assistant_core::ToolSpec;
use assistant_core::tool::{ToolDispatcher, ToolOutput};
use assistant_core::types::conversation::ExecutionContext;
#[cfg(test)]
use assistant_core::types::conversation::Interface;
use async_trait::async_trait;
use serde_json::Value;

/// A single recorded `execute` call.
#[derive(Debug, Clone)]
pub struct RecordedToolCall {
    pub name: String,
    pub params: HashMap<String, Value>,
}

#[derive(Default)]
struct StubState {
    specs: Vec<ToolSpec>,
    /// Queued responses, keyed by tool name. When a tool is invoked and a
    /// response exists for it, the first queued one is consumed (FIFO).
    /// When none queued, returns benign success.
    responses: HashMap<String, Vec<ToolOutput>>,
    recorded: Vec<RecordedToolCall>,
    mutating_names: std::collections::HashSet<String>,
}

/// Minimal [`ToolDispatcher`] for tests.
pub struct StubToolDispatcher {
    state: Arc<Mutex<StubState>>,
}

impl StubToolDispatcher {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(StubState::default())),
        }
    }

    /// Replace the tool specs returned by [`to_specs`](Self::to_specs).
    pub fn with_specs(self, specs: Vec<ToolSpec>) -> Self {
        if let Ok(mut s) = self.state.lock() {
            s.specs = specs;
        }
        self
    }

    /// Queue a response for the named tool. Subsequent `execute(name, ...)`
    /// calls consume queued responses in FIFO order.
    pub fn queue_response(self, name: impl Into<String>, output: ToolOutput) -> Self {
        if let Ok(mut s) = self.state.lock() {
            s.responses.entry(name.into()).or_default().push(output);
        }
        self
    }

    /// Mark a tool name as mutating for [`is_mutating`](Self::is_mutating).
    pub fn mark_mutating(self, name: impl Into<String>) -> Self {
        if let Ok(mut s) = self.state.lock() {
            s.mutating_names.insert(name.into());
        }
        self
    }

    /// Snapshot of every recorded `execute` call.
    pub fn recorded_calls(&self) -> Vec<RecordedToolCall> {
        match self.state.lock() {
            Ok(g) => g.recorded.clone(),
            Err(p) => p.into_inner().recorded.clone(),
        }
    }
}

impl Default for StubToolDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolDispatcher for StubToolDispatcher {
    fn to_specs(&self) -> Vec<ToolSpec> {
        match self.state.lock() {
            Ok(g) => g.specs.clone(),
            Err(p) => p.into_inner().specs.clone(),
        }
    }

    async fn execute(
        &self,
        name: &str,
        params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let mut state = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        state.recorded.push(RecordedToolCall {
            name: name.to_string(),
            params,
        });
        if let Some(queue) = state.responses.get_mut(name)
            && !queue.is_empty()
        {
            return Ok(queue.remove(0));
        }
        Ok(ToolOutput::success(format!("(stub: {name})")))
    }

    fn is_mutating(&self, name: &str) -> bool {
        match self.state.lock() {
            Ok(g) => g.mutating_names.contains(name),
            Err(p) => p.into_inner().mutating_names.contains(name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: uuid::Uuid::new_v4(),
            agent_id: "default".into(),
            turn: 0,
            interface: Interface::Cli,
            interactive: false,
            allowed_tools: None,
            depth: 0,
            user_id: None,
            org_id: None,
            space_id: None,
        }
    }

    #[tokio::test]
    async fn execute_records_calls() {
        let stub = StubToolDispatcher::new();
        let _ = stub.execute("foo", HashMap::new(), &ctx()).await.unwrap();
        let _ = stub.execute("bar", HashMap::new(), &ctx()).await.unwrap();
        let calls = stub.recorded_calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "foo");
        assert_eq!(calls[1].name, "bar");
    }

    #[tokio::test]
    async fn execute_consumes_queued_response() {
        let stub = StubToolDispatcher::new()
            .queue_response("foo", ToolOutput::success("first"))
            .queue_response("foo", ToolOutput::error("oops"));
        let r1 = stub.execute("foo", HashMap::new(), &ctx()).await.unwrap();
        let r2 = stub.execute("foo", HashMap::new(), &ctx()).await.unwrap();
        let r3 = stub.execute("foo", HashMap::new(), &ctx()).await.unwrap();
        assert_eq!(r1.content, "first");
        assert!(r1.success);
        assert_eq!(r2.content, "oops");
        assert!(!r2.success);
        // Empty queue falls back to benign success.
        assert!(r3.content.contains("stub"));
        assert!(r3.success);
    }

    #[tokio::test]
    async fn is_mutating_uses_marker() {
        let stub = StubToolDispatcher::new().mark_mutating("write");
        assert!(stub.is_mutating("write"));
        assert!(!stub.is_mutating("read"));
    }

    #[test]
    fn to_specs_returns_configured_list() {
        let stub = StubToolDispatcher::new().with_specs(vec![ToolSpec {
            name: "echo".into(),
            description: "stub".into(),
            params_schema: serde_json::json!({}),
            is_mutating: false,
            requires_confirmation: false,
        }]);
        let specs = stub.to_specs();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name, "echo");
    }
}
