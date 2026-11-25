//! Fixtures shared by the integration tests: an rmcp client wired to an
//! [`AssistantMcpServer`] over an in-memory duplex pipe.

#![allow(dead_code, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use assistant_core::auth::AuthContext;
use assistant_core::permission::PermissionPolicy;
use assistant_core::tool::ToolDispatcher;
use assistant_core::types::permissions::PermissionsConfig;
use assistant_mcp_server::{
    AssistantMcpServer, ListChangeListeners, serve_transport, with_identity,
};
use assistant_runtime::{OrchestrationEngine, StubOrchestrationEngine, TurnResult};
use assistant_skills::{SkillDef, SkillSource};
use assistant_storage::{InMemorySkillCatalog, SkillCatalog, SkillRegistry, StorageLayer};
use assistant_tool_executor::StubToolDispatcher;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ClientConfig, ContentBlock, ErrorCode,
    ProgressNotificationParam, ProtocolVersion, ReadResourceResult, ResourceContents,
};
use rmcp::service::{NotificationContext, RoleClient, RunningService, ServiceError};
use rmcp::{ClientHandler, ErrorData, ServiceExt};
use serde_json::{Value, json};
use tokio::io::DuplexStream;
use tokio::sync::mpsc;

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
    pub listeners: ListChangeListeners,
}

pub fn default_stubs() -> Stubs {
    Stubs {
        orchestrator: Arc::new(StubOrchestrationEngine::new()),
        executor: Arc::new(StubToolDispatcher::new()),
        catalog: Arc::new(InMemorySkillCatalog::new()),
        listeners: ListChangeListeners::default(),
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

    pub fn listeners(mut self, listeners: ListChangeListeners) -> Self {
        self.listeners = listeners;
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
            self.listeners,
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

/// A server built from `stubs` whose `install-skill` tool installs into a
/// fresh directory.
pub async fn installing_server(stubs: Stubs) -> AssistantMcpServer {
    stubs
        .server(allow_everything())
        .with_install_registry(install_registry().await)
}

/// Writes a skill named `name` into a fresh directory and returns
/// `install-skill` arguments that install it.
pub fn install_params(name: &str) -> (tempfile::TempDir, CallToolRequestParams) {
    let source = tempfile::tempdir().expect("tempdir");
    let skill_dir = source.path().join(name);
    std::fs::create_dir(&skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: from a test\n---\nbody\n"),
    )
    .expect("SKILL.md");
    let params = call_params(
        "install-skill",
        json!({"source": skill_dir.to_str().expect("utf-8 path")}),
    );
    (source, params)
}

/// A notification a [`Recorder`] client received outside any subscription.
#[derive(Debug, Clone, PartialEq)]
pub enum Received {
    ToolsChanged,
    PromptsChanged,
    ResourcesChanged,
    Progress(ProgressNotificationParam),
}

/// A client handler that records the notifications it receives.
pub struct Recorder {
    version: ProtocolVersion,
    received: mpsc::UnboundedSender<Received>,
}

impl Recorder {
    /// A recorder that initializes at `version`, with the receiving end of
    /// its notifications.
    pub fn new(version: ProtocolVersion) -> (Self, mpsc::UnboundedReceiver<Received>) {
        let (received, rx) = mpsc::unbounded_channel();
        (Self { version, received }, rx)
    }

    fn record(&self, notification: Received) {
        let _ = self.received.send(notification);
    }
}

impl ClientHandler for Recorder {
    fn get_info(&self) -> ClientConfig {
        let mut config = ClientConfig::default();
        config.protocol_version = self.version.clone();
        config
    }

    async fn on_tool_list_changed(&self, _context: NotificationContext<RoleClient>) {
        self.record(Received::ToolsChanged);
    }

    async fn on_prompt_list_changed(&self, _context: NotificationContext<RoleClient>) {
        self.record(Received::PromptsChanged);
    }

    async fn on_resource_list_changed(&self, _context: NotificationContext<RoleClient>) {
        self.record(Received::ResourcesChanged);
    }

    async fn on_progress(
        &self,
        params: ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        self.record(Received::Progress(params));
    }
}

/// Receives the next recorded notification, failing after five seconds.
pub async fn next_received(rx: &mut mpsc::UnboundedReceiver<Received>) -> Received {
    tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("a notification within five seconds")
        .expect("the recorder is alive")
}
