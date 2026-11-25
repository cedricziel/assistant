//! MCP server handler backed by the assistant's tool, skill and orchestration facades.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, PaginatedRequestParams,
    ProtocolVersion, ReadResourceRequestParams, ReadResourceResponse, ServerCapabilities,
    ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::{Value, json};
use tracing::warn;
use uuid::Uuid;

use assistant_core::permission::{PermissionPolicy, PermissionRequest};
use assistant_core::tool::ToolDispatcher;
use assistant_core::types::conversation::{ExecutionContext, Interface};
use assistant_runtime::OrchestrationEngine;
use assistant_storage::{SkillCatalog, SkillRegistry};
use assistant_tool_executor::install_skill_from_source;

use crate::resources;

const RUN_PROMPT: &str = "run-prompt";
const INSTALL_SKILL: &str = "install-skill";

/// Revisions this server implements, oldest first. Pinned rather than taken from
/// rmcp's known set so a rmcp upgrade cannot silently advertise older revisions.
const SUPPORTED_VERSIONS: &[ProtocolVersion] =
    &[ProtocolVersion::V_2025_11_25, ProtocolVersion::V_2026_07_28];

/// Serves the assistant's tools and skills to MCP clients.
///
/// Every `tools/call` is authorized by the permission policy; MCP callers
/// cannot be prompted, so calls the policy leaves undecided are refused.
#[derive(Clone)]
pub struct AssistantMcpServer {
    catalog: Arc<dyn SkillCatalog>,
    executor: Arc<dyn ToolDispatcher>,
    orchestrator: Arc<dyn OrchestrationEngine>,
    policy: PermissionPolicy,
    user_skills_dir: PathBuf,
    install_registry: Option<Arc<SkillRegistry>>,
}

impl AssistantMcpServer {
    pub fn new(
        orchestrator: Arc<dyn OrchestrationEngine>,
        executor: Arc<dyn ToolDispatcher>,
        catalog: Arc<dyn SkillCatalog>,
        policy: PermissionPolicy,
        user_skills_dir: PathBuf,
    ) -> Self {
        Self {
            catalog,
            executor,
            orchestrator,
            policy,
            user_skills_dir,
            install_registry: None,
        }
    }

    /// Enables the `install-skill` tool, which writes into `user_skills_dir` and
    /// registers the result in `registry`.
    pub fn with_install_registry(mut self, registry: Arc<SkillRegistry>) -> Self {
        self.install_registry = Some(registry);
        self
    }

    fn tool_list(&self) -> Vec<Tool> {
        let mut tools: Vec<Tool> = self
            .executor
            .to_specs()
            .into_iter()
            .map(|spec| tool(spec.name, spec.description, spec.params_schema))
            .collect();
        tools.push(tool(
            RUN_PROMPT,
            "Send a prompt through the orchestrator loop (may invoke multiple tools).",
            json!({
                "type": "object",
                "required": ["prompt"],
                "properties": {
                    "prompt": { "type": "string", "description": "The user message to process" }
                }
            }),
        ));
        tools.push(tool(
            INSTALL_SKILL,
            "Install a skill from a local path or GitHub (owner/repo[/path]).",
            json!({
                "type": "object",
                "required": ["source"],
                "properties": {
                    "source": { "type": "string", "description": "Local path or owner/repo[/sub/path]" }
                }
            }),
        ));
        tools
    }

    fn requires_confirmation(&self, tool_name: &str) -> bool {
        tool_name == INSTALL_SKILL
            || self
                .executor
                .to_specs()
                .iter()
                .any(|spec| spec.name == tool_name && spec.requires_confirmation)
    }

    async fn run_prompt(&self, input: &Value) -> Result<CallToolResult, ErrorData> {
        let prompt = required_str(input, "prompt")?;
        let turn = self
            .orchestrator
            .submit_turn(
                &assistant_core::auth::AuthContext::system(),
                prompt,
                Uuid::new_v4(),
                Interface::Mcp,
                None,
            )
            .await
            .map_err(|e| {
                warn!(error = %e, "run_prompt failed");
                ErrorData::internal_error(e.to_string(), None)
            })?;
        Ok(text_result(turn.answer))
    }

    async fn install_skill(&self, input: &Value) -> Result<CallToolResult, ErrorData> {
        let source = required_str(input, "source")?;
        let Some(registry) = self.install_registry.clone() else {
            return Err(ErrorData::internal_error(
                "Skill installation requires a SkillRegistry — none configured",
                None,
            ));
        };
        let name = install_skill_from_source(source, &self.user_skills_dir, registry)
            .await
            .map_err(|e| {
                warn!(source, error = %e, "install_skill failed");
                ErrorData::internal_error(e.to_string(), None)
            })?;
        Ok(text_result(format!("Skill '{name}' installed.")))
    }
}

fn tool(
    name: impl Into<Cow<'static, str>>,
    description: impl Into<Cow<'static, str>>,
    schema: Value,
) -> Tool {
    let schema = match schema {
        Value::Object(map) => map,
        _ => serde_json::Map::from_iter([("type".to_owned(), json!("object"))]),
    };
    Tool::new(name, description, schema)
}

fn text_result(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(text)])
}

fn required_str<'a>(input: &'a Value, name: &str) -> Result<&'a str, ErrorData> {
    input[name].as_str().ok_or_else(|| {
        ErrorData::invalid_params(format!("Missing required parameter '{name}'"), None)
    })
}

impl ServerHandler for AssistantMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new(
            "assistant-mcp-server",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(SUPPORTED_VERSIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(self.tool_list()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let tool_name = request.name.as_ref();
        let arguments = request.arguments.unwrap_or_default();
        let input = Value::Object(arguments.clone());

        // `run-prompt` is exempt from the policy: its tool calls pass the
        // orchestrator's own gate.
        if tool_name == RUN_PROMPT {
            return self.run_prompt(&input).await.map(Into::into);
        }

        // MCP callers cannot be prompted, so the policy decides alone.
        let ctx = ExecutionContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "default".to_string(),
            turn: 0,
            interface: Interface::Mcp,
            interactive: false,
            allowed_tools: None,
            depth: 0,
            user_id: None,
            org_id: None,
            space_id: None,
        };
        let requires_confirmation = self.requires_confirmation(tool_name);
        let resolution = self
            .policy
            .evaluate(&PermissionRequest {
                tool_name,
                params: &input,
                ctx: &ctx,
                requires_confirmation,
                task_context: None,
            })
            .await;
        if let Some(refusal) = resolution.refusal(tool_name) {
            warn!(tool = %tool_name, %refusal, "MCP tool call refused");
            return Ok(CallToolResult::error(vec![ContentBlock::text(refusal)]).into());
        }

        if tool_name == INSTALL_SKILL {
            return self.install_skill(&input).await.map(Into::into);
        }

        let params: HashMap<String, Value> = arguments.into_iter().collect();
        let output = self
            .executor
            .execute(tool_name, params, &ctx)
            .await
            .map_err(|e| {
                warn!(tool = %tool_name, error = %e, "Tool execution failed");
                ErrorData::internal_error(e.to_string(), None)
            })?;
        Ok(text_result(output.content).into())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(resources::list(self.catalog.as_ref()).await)
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(resources::templates())
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        resources::read(self.catalog.as_ref(), &request.uri)
            .await
            .map(Into::into)
    }
}
