//! MCP server request dispatcher.

use std::sync::Arc;

use assistant_core::Interface;
use assistant_runtime::ReactOrchestrator;
use assistant_storage::registry::SkillRegistry;
use serde_json::{json, Value};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::protocol::*;

/// Handles a single MCP JSON-RPC request and returns a response.
pub async fn handle_request(
    req: JsonRpcRequest,
    registry: Arc<SkillRegistry>,
    orchestrator: Arc<ReactOrchestrator>,
) -> JsonRpcResponse {
    debug!(method = %req.method, "MCP request");

    match req.method.as_str() {
        // ── Lifecycle ─────────────────────────────────────────────────────────
        "initialize" => {
            let result = InitializeResult {
                protocol_version: "2024-11-05",
                capabilities: ServerCapabilities {
                    tools: ToolsCapability {
                        list_changed: false,
                    },
                    resources: ResourcesCapability {
                        subscribe: false,
                        list_changed: false,
                    },
                },
                server_info: ServerInfo {
                    name: "assistant-mcp-server",
                    version: env!("CARGO_PKG_VERSION"),
                },
            };
            JsonRpcResponse::ok(req.id, serde_json::to_value(result).unwrap())
        }

        "notifications/initialized" | "ping" => JsonRpcResponse::ok(req.id, json!({})),

        // ── Tools ─────────────────────────────────────────────────────────────
        "tools/list" => {
            let tools = vec![
                McpTool {
                    name: "list_skills".to_string(),
                    description: "List all registered assistant skills with their names, descriptions, and tiers.".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "filter": {
                                "type": "string",
                                "description": "Optional filter string to match against skill names or descriptions"
                            }
                        }
                    }),
                },
                McpTool {
                    name: "invoke_skill".to_string(),
                    description: "Invoke a named assistant skill and return its output.".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "required": ["name"],
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "The skill name to invoke (e.g. 'web-fetch')"
                            },
                            "params": {
                                "type": "object",
                                "description": "Parameters to pass to the skill"
                            }
                        }
                    }),
                },
                McpTool {
                    name: "run_prompt".to_string(),
                    description: "Send a prompt to the assistant and get a full ReAct response, including any skill invocations.".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "required": ["prompt"],
                        "properties": {
                            "prompt": {
                                "type": "string",
                                "description": "The user message to process"
                            }
                        }
                    }),
                },
            ];

            JsonRpcResponse::ok(req.id, json!({ "tools": tools }))
        }

        "tools/call" => {
            let tool_name = req.params["name"].as_str().unwrap_or("").to_string();
            let tool_input = req.params["arguments"].clone();

            match tool_name.as_str() {
                "list_skills" => {
                    let filter = tool_input["filter"].as_str().unwrap_or("").to_lowercase();
                    let skills = registry.list().await;
                    let filtered: Vec<_> = skills
                        .iter()
                        .filter(|s| {
                            filter.is_empty()
                                || s.name.to_lowercase().contains(&filter)
                                || s.description.to_lowercase().contains(&filter)
                        })
                        .collect();

                    let text = filtered
                        .iter()
                        .map(|s| format!("{}\t[{}]\t{}", s.name, s.tier, s.description))
                        .collect::<Vec<_>>()
                        .join("\n");

                    let content = vec![ContentItem::text(if text.is_empty() {
                        "No skills found.".to_string()
                    } else {
                        format!("Available skills ({}):\n{}", filtered.len(), text)
                    })];

                    JsonRpcResponse::ok(req.id, json!({ "content": content }))
                }

                "invoke_skill" => {
                    let name = match tool_input["name"].as_str() {
                        Some(n) => n.to_string(),
                        None => {
                            return JsonRpcResponse::err(
                                req.id,
                                -32602,
                                "Missing required parameter 'name'",
                            );
                        }
                    };
                    let prompt = format!(
                        "Use the {} skill with parameters: {}",
                        name,
                        tool_input.get("params").unwrap_or(&Value::Null)
                    );
                    let result = orchestrator
                        .run_turn(&prompt, Uuid::new_v4(), Interface::Mcp)
                        .await;

                    match result {
                        Ok(turn) => {
                            let content = vec![ContentItem::text(turn.answer)];
                            JsonRpcResponse::ok(req.id, json!({ "content": content }))
                        }
                        Err(e) => {
                            warn!(skill = %name, error = %e, "invoke_skill failed");
                            JsonRpcResponse::err(req.id, -32603, e.to_string())
                        }
                    }
                }

                "run_prompt" => {
                    let prompt = match tool_input["prompt"].as_str() {
                        Some(p) => p.to_string(),
                        None => {
                            return JsonRpcResponse::err(
                                req.id,
                                -32602,
                                "Missing required parameter 'prompt'",
                            );
                        }
                    };
                    let result = orchestrator
                        .run_turn(&prompt, Uuid::new_v4(), Interface::Mcp)
                        .await;

                    match result {
                        Ok(turn) => {
                            let content = vec![ContentItem::text(turn.answer)];
                            JsonRpcResponse::ok(req.id, json!({ "content": content }))
                        }
                        Err(e) => {
                            warn!(error = %e, "run_prompt failed");
                            JsonRpcResponse::err(req.id, -32603, e.to_string())
                        }
                    }
                }

                other => JsonRpcResponse::err(req.id, -32601, format!("Unknown tool: {other}")),
            }
        }

        // ── Resources ─────────────────────────────────────────────────────────
        "resources/list" => {
            let skills = registry.list().await;
            let mut resources = vec![McpResource {
                uri: "skills://list".to_string(),
                name: "All Skills".to_string(),
                description: "List of all registered assistant skills".to_string(),
                mime_type: "application/json".to_string(),
            }];

            for skill in &skills {
                resources.push(McpResource {
                    uri: format!("skills://{}", skill.name),
                    name: skill.name.clone(),
                    description: skill.description.clone(),
                    mime_type: "text/markdown".to_string(),
                });
            }

            JsonRpcResponse::ok(req.id, json!({ "resources": resources }))
        }

        "resources/read" => {
            let uri = req.params["uri"].as_str().unwrap_or("").to_string();

            if uri == "skills://list" {
                let skills = registry.list().await;
                let items: Vec<_> = skills
                    .iter()
                    .map(|s| {
                        json!({
                            "name": s.name,
                            "description": s.description,
                            "tier": s.tier.label(),
                            "source": s.source.to_string(),
                            "mutating": s.mutating,
                            "confirmation_required": s.confirmation_required,
                        })
                    })
                    .collect();

                let content = vec![ContentItem::text(
                    serde_json::to_string_pretty(&json!({ "skills": items })).unwrap_or_default(),
                )];
                JsonRpcResponse::ok(req.id, json!({ "contents": content }))
            } else if let Some(skill_name) = uri.strip_prefix("skills://") {
                match registry.get(skill_name).await {
                    Some(skill) => {
                        let skill_md_path = skill.dir.join("SKILL.md");
                        let text = std::fs::read_to_string(&skill_md_path)
                            .unwrap_or_else(|_| skill.body.clone());
                        let content = vec![ContentItem::text(text)];
                        JsonRpcResponse::ok(req.id, json!({ "contents": content }))
                    }
                    None => JsonRpcResponse::err(
                        req.id,
                        -32602,
                        format!("Skill '{skill_name}' not found"),
                    ),
                }
            } else {
                JsonRpcResponse::err(req.id, -32602, format!("Unknown resource URI: {uri}"))
            }
        }

        other => {
            debug!(method = %other, "Unhandled MCP method");
            // Return empty ok for notifications (no id), error for unknown methods
            if req.id.is_some() {
                JsonRpcResponse::err(req.id, -32601, format!("Method not found: {other}"))
            } else {
                JsonRpcResponse::ok(None, json!({}))
            }
        }
    }
}
