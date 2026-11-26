//! Fixtures shared by the integration tests: an rmcp client wired to an
//! [`AssistantMcpServer`] over an in-memory duplex pipe.

#![allow(dead_code, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use assistant_core::auth::AuthContext;
use assistant_core::permission::PermissionPolicy;
use assistant_core::tool::ToolDispatcher;
use assistant_core::types::permissions::PermissionsConfig;
use assistant_mcp_server::{AssistantMcpServer, serve_transport, with_identity};
use assistant_runtime::{OrchestrationEngine, StubOrchestrationEngine, TurnResult};
use assistant_skills::{SkillDef, SkillSource};
use assistant_storage::{InMemorySkillCatalog, SkillCatalog, SkillRegistry, StorageLayer};
use assistant_tool_executor::StubToolDispatcher;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, ErrorCode, ReadResourceResult,
    ResourceContents,
};
use rmcp::service::{RoleClient, RunningService, ServiceError};
use rmcp::{ErrorData, ServiceExt};
use serde_json::Value;
use tokio::io::DuplexStream;

pub type Client = RunningService<RoleClient, ()>;

pub fn make_skill(name: &str) -> SkillDef {
    SkillDef {
        name: name.to_string(),
        description: format!("test skill {name}"),
        license: None,
        compatibility: None,
        allowed_tools: Vec::new(),
        metadata: std::collections::HashMap::new(),
        body: format!("body of {name}"),
        dir: std::path::PathBuf::from(format!("/tmp/{name}")),
        source: SkillSource::Builtin,
    }
}

/// Policy for tests that exercise dispatch rather than permissions.
pub fn allow_everything() -> PermissionPolicy {
    policy(&["*"], &[])
}

pub fn policy(allow: &[&str], deny: &[&str]) -> PermissionPolicy {
    let rules = |texts: &[&str]| {
        texts
            .iter()
            .map(|t| t.parse().expect("valid rule"))
            .collect()
    };
    PermissionPolicy::from_config(&PermissionsConfig {
        allow: rules(allow),
        deny: rules(deny),
        ..PermissionsConfig::default()
    })
}

pub struct Stubs {
    pub orchestrator: Arc<dyn OrchestrationEngine>,
    pub executor: Arc<dyn ToolDispatcher>,
    pub catalog: Arc<dyn SkillCatalog>,
}

pub fn default_stubs() -> Stubs {
    Stubs {
        orchestrator: Arc::new(StubOrchestrationEngine::new()),
        executor: Arc::new(StubToolDispatcher::new()),
        catalog: Arc::new(InMemorySkillCatalog::new()),
    }
}

impl Stubs {
    pub fn orchestrator(mut self, orchestrator: Arc<dyn OrchestrationEngine>) -> Self {
        self.orchestrator = orchestrator;
        self
    }

    pub fn executor(mut self, executor: Arc<dyn ToolDispatcher>) -> Self {
        self.executor = executor;
        self
    }

    pub fn skills(mut self, skills: Vec<SkillDef>) -> Self {
        self.catalog = Arc::new(InMemorySkillCatalog::with_skills(skills));
        self
    }

    pub fn server(self, policy: PermissionPolicy) -> AssistantMcpServer {
        let skills_dir = tempfile::tempdir().expect("tempdir").keep();
        AssistantMcpServer::new(
            self.orchestrator,
            self.executor,
            self.catalog,
            policy,
            skills_dir,
        )
    }

    pub async fn connect(self) -> Client {
        connect(self.server(allow_everything())).await
    }

    pub async fn connect_with_policy(self, policy: PermissionPolicy) -> Client {
        connect(self.server(policy)).await
    }
}

pub async fn install_registry() -> Arc<SkillRegistry> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap())
}

/// Serves `server` on one end of a duplex pipe, with requests carrying no
/// caller identity, and returns the other end.
pub fn spawn_anonymous_server(server: AssistantMcpServer) -> DuplexStream {
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(serve_transport(server, server_io));
    client_io
}

/// Serves `server` on one end of a duplex pipe, with every request carrying
/// `auth`, and returns the other end.
pub fn spawn_server_as(server: AssistantMcpServer, auth: AuthContext) -> DuplexStream {
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(serve_transport(server, with_identity(server_io, auth)));
    client_io
}

/// Serves `server` as the local system identity, like stdio does.
pub fn spawn_server(server: AssistantMcpServer) -> DuplexStream {
    spawn_server_as(server, AuthContext::system())
}

/// An rmcp client connected to `server` as the local system identity.
pub async fn connect(server: AssistantMcpServer) -> Client {
    connect_to(spawn_server(server)).await
}

/// An rmcp client connected to `server` as `auth`.
pub async fn connect_as(server: AssistantMcpServer, auth: AuthContext) -> Client {
    connect_to(spawn_server_as(server, auth)).await
}

/// An rmcp client connected to `server` whose requests carry no identity.
pub async fn connect_anonymous(server: AssistantMcpServer) -> Client {
    connect_to(spawn_anonymous_server(server)).await
}

async fn connect_to(io: DuplexStream) -> Client {
    ().serve(io).await.expect("client handshake")
}

pub fn args(value: Value) -> serde_json::Map<String, Value> {
    value.as_object().expect("object arguments").clone()
}

pub fn call_params(name: &'static str, arguments: Value) -> CallToolRequestParams {
    CallToolRequestParams::new(name).with_arguments(args(arguments))
}

pub fn turn(answer: &str) -> TurnResult {
    TurnResult {
        answer: answer.to_string(),
        attachments: Vec::new(),
        attachment_ids: Vec::new(),
        message_id: None,
        had_errors: false,
        timing: None,
    }
}

/// The protocol error behind a failed request.
pub fn protocol_error(error: ServiceError) -> ErrorData {
    match error {
        ServiceError::McpError(data) => data,
        other => panic!("expected a protocol error, got {other:?}"),
    }
}

pub fn code(error: ServiceError) -> ErrorCode {
    protocol_error(error).code
}

pub fn text_of(result: &CallToolResult) -> String {
    match result.content.first() {
        Some(ContentBlock::Text(text)) => text.text.clone(),
        other => panic!("expected a text block, got {other:?}"),
    }
}

pub fn resource_text(result: &ReadResourceResult) -> String {
    match result.contents.first() {
        Some(ResourceContents::TextResourceContents { text, .. }) => text.clone(),
        other => panic!("expected text contents, got {other:?}"),
    }
}
