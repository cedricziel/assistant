//! MCP server request dispatcher.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use assistant_core::{ExecutionContext, Interface};
use assistant_runtime::Orchestrator;
use assistant_storage::registry::SkillRegistry;
use assistant_tool_executor::{install_skill_from_source, ToolExecutor};
use serde_json::{json, Value};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::protocol::*;

/// Handles a single MCP JSON-RPC request and returns a response.
pub async fn handle_request(
    req: JsonRpcRequest,
    registry: Arc<SkillRegistry>,
    executor: Arc<ToolExecutor>,
    orchestrator: Arc<Orchestrator>,
    user_skills_dir: PathBuf,
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
            JsonRpcResponse::ok(
                req.id,
                serde_json::to_value(result).unwrap_or(serde_json::json!({})),
            )
        }

        "notifications/initialized" | "ping" => JsonRpcResponse::ok(req.id, json!({})),

        // ── Tools ─────────────────────────────────────────────────────────────
        "tools/list" => {
            // Use ToolExecutor specs for the tools list.
            let tool_specs = executor.to_specs();
            let mut tools: Vec<McpTool> = tool_specs
                .iter()
                .map(|spec| McpTool {
                    name: spec.name.clone(),
                    description: spec.description.clone(),
                    input_schema: spec.params_schema.clone(),
                })
                .collect();

            // Management tools appended after the per-tool entries.
            tools.push(McpTool {
                name: "run-prompt".to_string(),
                description:
                    "Send a prompt through the orchestrator loop (may invoke multiple tools)."
                        .to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["prompt"],
                    "properties": {
                        "prompt": { "type": "string", "description": "The user message to process" }
                    }
                }),
            });
            tools.push(McpTool {
                name: "install-skill".to_string(),
                description: "Install a skill from a local path or GitHub (owner/repo[/path]).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "required": ["source"],
                    "properties": {
                        "source": { "type": "string", "description": "Local path or owner/repo[/sub/path]" }
                    }
                }),
            });

            JsonRpcResponse::ok(req.id, json!({ "tools": tools }))
        }

        "tools/call" => {
            let tool_name = req.params["name"].as_str().unwrap_or("").to_string();
            let tool_input = req.params["arguments"].clone();

            // ── Management tools ──────────────────────────────────────────────
            if tool_name == "run-prompt" {
                let Some(prompt) = tool_input["prompt"].as_str() else {
                    return JsonRpcResponse::err(
                        req.id,
                        -32602,
                        "Missing required parameter 'prompt'",
                    );
                };
                return match orchestrator
                    .submit_turn(prompt, Uuid::new_v4(), Interface::Mcp)
                    .await
                {
                    Ok(turn) => {
                        let content = vec![ContentItem::text(turn.answer)];
                        JsonRpcResponse::ok(req.id, json!({ "content": content }))
                    }
                    Err(e) => {
                        warn!(error = %e, "run_prompt failed");
                        JsonRpcResponse::err(req.id, -32603, e.to_string())
                    }
                };
            }

            if tool_name == "install-skill" {
                let Some(source) = tool_input["source"].as_str() else {
                    return JsonRpcResponse::err(
                        req.id,
                        -32602,
                        "Missing required parameter 'source'",
                    );
                };
                return match install_skill_from_source(source, &user_skills_dir, registry.clone())
                    .await
                {
                    Ok(name) => {
                        let content = vec![ContentItem::text(format!("Skill '{name}' installed."))];
                        JsonRpcResponse::ok(req.id, json!({ "content": content }))
                    }
                    Err(e) => {
                        warn!(source, error = %e, "install_skill failed");
                        JsonRpcResponse::err(req.id, -32603, e.to_string())
                    }
                };
            }

            // ── Per-tool dynamic dispatch via ToolExecutor ─────────────────────
            let params: HashMap<String, Value> = if let Value::Object(map) = &tool_input {
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else {
                HashMap::new()
            };

            let ctx = ExecutionContext {
                conversation_id: Uuid::new_v4(),
                turn: 0,
                interface: Interface::Mcp,
                interactive: false,
                allowed_tools: None,
                depth: 0,
            };

            match executor.execute(&tool_name, params, &ctx).await {
                Ok(output) => {
                    let content = vec![ContentItem::text(output.content)];
                    JsonRpcResponse::ok(req.id, json!({ "content": content }))
                }
                Err(e) => {
                    warn!(tool = %tool_name, error = %e, "Tool execution failed");
                    JsonRpcResponse::err(req.id, -32603, e.to_string())
                }
            }
        }

        // ── Resources ─────────────────────────────────────────────────────────
        // Resources still use the SkillRegistry (SKILL.md knowledge packages).
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

                // Auxiliary files (scripts/, references/, assets/)
                for (category, rel_path) in skill.auxiliary_files() {
                    let filename = rel_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    resources.push(McpResource {
                        uri: format!("skills://{}/{}", skill.name, rel_path.display()),
                        name: filename.to_string(),
                        description: format!(
                            "{} file for {} skill",
                            category.dir_name(),
                            skill.name
                        ),
                        mime_type: category.mime_type().to_string(),
                    });
                }
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
                            "source": s.source.to_string(),
                        })
                    })
                    .collect();

                let content = vec![ContentItem::text(
                    serde_json::to_string_pretty(&json!({ "skills": items })).unwrap_or_default(),
                )];
                JsonRpcResponse::ok(req.id, json!({ "contents": content }))
            } else if let Some(path) = uri.strip_prefix("skills://") {
                let segments: Vec<&str> = path.splitn(3, '/').collect();
                match segments.len() {
                    // skills://<name> — return SKILL.md
                    1 => {
                        let skill_name = segments[0];
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
                    }
                    // skills://<name>/<category>/<filename> — return auxiliary file
                    3 if matches!(segments[1], "scripts" | "references" | "assets") => {
                        let skill_name = segments[0];
                        let category = segments[1];
                        let filename = segments[2];
                        match registry.get(skill_name).await {
                            Some(skill) => {
                                let file_path = skill.dir.join(category).join(filename);
                                match std::fs::read_to_string(&file_path) {
                                    Ok(text) => {
                                        let content = vec![ContentItem::text(text)];
                                        JsonRpcResponse::ok(
                                            req.id,
                                            json!({ "contents": content }),
                                        )
                                    }
                                    Err(_) => JsonRpcResponse::err(
                                        req.id,
                                        -32602,
                                        format!(
                                            "File '{category}/{filename}' not found in skill '{skill_name}'"
                                        ),
                                    ),
                                }
                            }
                            None => JsonRpcResponse::err(
                                req.id,
                                -32602,
                                format!("Skill '{skill_name}' not found"),
                            ),
                        }
                    }
                    _ => {
                        JsonRpcResponse::err(req.id, -32602, format!("Invalid resource URI: {uri}"))
                    }
                }
            } else {
                JsonRpcResponse::err(req.id, -32602, format!("Unknown resource URI: {uri}"))
            }
        }

        other => {
            debug!(method = %other, "Unhandled MCP method");
            if req.id.is_some() {
                JsonRpcResponse::err(req.id, -32601, format!("Method not found: {other}"))
            } else {
                JsonRpcResponse::ok(None, json!({}))
            }
        }
    }
}

// ── skill_to_mcp_tool is no longer needed (using ToolExecutor specs directly) ──
