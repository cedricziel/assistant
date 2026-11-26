//! MCP Apps on the client side: resolving the `ui://` resource a tool
//! declares into an [`McpApp`], and serving the calls an app makes back to
//! its server.

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use rmcp::model::{MetaObject, ResourceContents, Tool};
use serde_json::{Map, Value};
use tracing::{debug, warn};

use assistant_core::mcp_app::{
    McpApp, McpAppCsp, McpAppPermissions, McpAppServer, McpAppServers, McpAppTool,
};
use assistant_core::tool::TurnScope;
use assistant_core::types::skills_mcp::McpTrustLevel;

use crate::client::{McpClient, content_bytes};

/// The only MIME type an app resource may have.
pub const APP_MIME_TYPE: &str = "text/html;profile=mcp-app";

/// The largest app HTML kept; larger documents attach no app.
const MAX_APP_HTML_BYTES: usize = 1024 * 1024;

/// The `ui://` resource `tool` declares in `_meta.ui.resourceUri`, or in the
/// deprecated flat `_meta["ui/resourceUri"]`.
pub(crate) fn ui_resource_uri(tool: &Tool) -> Option<String> {
    ui_field(tool.meta.as_ref()?, "resourceUri")
        .and_then(Value::as_str)
        .filter(|uri| uri.starts_with("ui://"))
        .map(str::to_string)
}

/// `_meta.ui.{field}`, falling back to the deprecated flat `_meta["ui/{field}"]`.
pub(crate) fn ui_field<'a>(meta: &'a MetaObject, field: &str) -> Option<&'a Value> {
    meta.0
        .get("ui")
        .and_then(|ui| ui.get(field))
        .or_else(|| meta.0.get(&format!("ui/{field}")))
}

/// Whether `audience` (`model` or `app`) may use `tool`.
///
/// MCP Apps visibility lives in `_meta.ui.visibility` and defaults to
/// `["model", "app"]`.
pub(crate) fn visible_to(tool: &Tool, audience: &str) -> bool {
    tool.meta
        .as_ref()
        .and_then(|meta| ui_field(meta, "visibility"))
        .and_then(Value::as_array)
        .is_none_or(|audiences| audiences.iter().any(|a| a.as_str() == Some(audience)))
}

/// The app at `uri` on `client`'s server, or `None` (with a warning) when it
/// cannot be read or is not an app.
pub(crate) async fn resolve_app(client: &McpClient, server: &str, uri: &str) -> Option<McpApp> {
    match read_app(client, server, uri).await {
        Ok(app) => Some(app),
        Err(error) => {
            warn!(
                server,
                uri,
                error = format!("{error:#}"),
                "no MCP app attached"
            );
            None
        }
    }
}

async fn read_app(client: &McpClient, server: &str, uri: &str) -> Result<McpApp> {
    let result = client.read_resource(uri).await?;
    let Some(contents) = result.contents.into_iter().next() else {
        bail!("the resource has no content");
    };
    let (mime_type, meta) = match &contents {
        ResourceContents::TextResourceContents {
            mime_type, meta, ..
        }
        | ResourceContents::BlobResourceContents {
            mime_type, meta, ..
        } => (mime_type.clone(), meta.clone()),
        _ => (None, None),
    };
    let html = String::from_utf8(content_bytes(contents)?).context("the resource is not UTF-8")?;
    if mime_type.as_deref() != Some(APP_MIME_TYPE) {
        bail!("served as {mime_type:?}, not {APP_MIME_TYPE}");
    }
    if html.len() > MAX_APP_HTML_BYTES {
        bail!(
            "the HTML is {} bytes, over the {MAX_APP_HTML_BYTES} byte limit",
            html.len()
        );
    }
    // The metadata of the read wins; the listing's is the fallback.
    let ui = match meta.and_then(|mut meta| meta.0.remove("ui")) {
        Some(ui) => ui,
        None => listed_ui(client, uri).await.unwrap_or(Value::Null),
    };
    Ok(McpApp {
        server: server.to_string(),
        resource_uri: uri.to_string(),
        html,
        csp: csp(&ui["csp"]),
        permissions: permissions(&ui["permissions"]),
        prefers_border: ui["prefersBorder"].as_bool(),
    })
}

/// `_meta.ui` of `uri` as `resources/list` reports it.
async fn listed_ui(client: &McpClient, uri: &str) -> Option<Value> {
    let resources = match client.list_resources().await {
        Ok(resources) => resources,
        Err(error) => {
            debug!(uri, error = format!("{error:#}"), "no listed app metadata");
            return None;
        }
    };
    resources
        .into_iter()
        .find(|resource| resource.uri == uri)?
        .meta?
        .0
        .get("ui")
        .cloned()
}

fn csp(csp: &Value) -> McpAppCsp {
    let domains = |key: &str| -> Vec<String> {
        csp[key]
            .as_array()
            .map(|list| {
                list.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    };
    McpAppCsp {
        connect_domains: domains("connectDomains"),
        resource_domains: domains("resourceDomains"),
        frame_domains: domains("frameDomains"),
        base_uri_domains: domains("baseUriDomains"),
    }
}

/// A permission is requested when its key is present and not `false`.
fn permissions(permissions: &Value) -> McpAppPermissions {
    let requested = |key: &str| {
        permissions
            .get(key)
            .is_some_and(|value| *value != Value::Bool(false))
    };
    McpAppPermissions {
        camera: requested("camera"),
        microphone: requested("microphone"),
        geolocation: requested("geolocation"),
        clipboard_write: requested("clipboardWrite"),
    }
}

// -- App-initiated calls --

/// What an app may do with `tool` on a server whose tools need confirmation
/// when `requires_confirmation` is set.
fn app_tool(tool: &Tool, requires_confirmation: bool) -> McpAppTool {
    let read_only = tool
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.read_only_hint)
        == Some(true);
    McpAppTool {
        visible_to_app: visible_to(tool, "app"),
        needs_confirmation: requires_confirmation || !read_only,
    }
}

/// A connected server and its tool listing, serving an app's calls.
pub struct ClientAppServer {
    client: Arc<McpClient>,
    tools: Arc<[Tool]>,
    requires_confirmation: bool,
}

impl ClientAppServer {
    pub fn new(client: Arc<McpClient>, tools: Arc<[Tool]>, requires_confirmation: bool) -> Self {
        Self {
            client,
            tools,
            requires_confirmation,
        }
    }
}

#[async_trait]
impl McpAppServer for ClientAppServer {
    fn tool(&self, name: &str) -> Option<McpAppTool> {
        self.tools
            .iter()
            .find(|tool| tool.name == name)
            .map(|tool| app_tool(tool, self.requires_confirmation))
    }

    async fn call_tool(&self, name: &str, arguments: Map<String, Value>) -> Result<Value> {
        let arguments = (!arguments.is_empty()).then_some(arguments);
        let result = self.client.call_tool(name, arguments).await?;
        Ok(serde_json::to_value(result)?)
    }

    async fn read_resource(&self, uri: &str) -> Result<Value> {
        let result = self.client.read_resource(uri).await?;
        Ok(serde_json::to_value(result)?)
    }
}

#[async_trait]
impl McpAppServers for crate::manager::McpClientManager {
    async fn server(
        &self,
        _scope: &TurnScope<'_>,
        server: &str,
    ) -> Result<Option<Arc<dyn McpAppServer>>> {
        let Some((client, trust)) = self.connected(server).await else {
            return Ok(None);
        };
        let tools = client.list_tools().await?;
        Ok(Some(Arc::new(ClientAppServer::new(
            client,
            tools.into(),
            trust != McpTrustLevel::Trust,
        ))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use rmcp::model::ToolAnnotations;
    use serde_json::json;

    fn tool(meta: Value) -> Tool {
        let mut tool = Tool::new("t", "d", Arc::new(serde_json::Map::new()));
        if let Value::Object(meta) = meta {
            tool = tool.with_meta(MetaObject(meta));
        }
        tool
    }

    #[test]
    fn tools_are_visible_to_apps_by_default() {
        assert!(app_tool(&tool(Value::Null), false).visible_to_app);
    }

    #[test]
    fn model_only_tool_is_not_visible_to_apps() {
        let tool = tool(json!({"ui": {"visibility": ["model"]}}));
        assert!(!app_tool(&tool, false).visible_to_app);
    }

    #[test]
    fn deprecated_flat_visibility_applies_to_apps() {
        let tool = tool(json!({"ui/visibility": ["app"]}));
        assert!(app_tool(&tool, false).visible_to_app);
    }

    #[test]
    fn only_a_read_only_tool_on_a_trusted_server_runs_unconfirmed() {
        let hinted = tool(Value::Null).with_annotations(ToolAnnotations::new().read_only(true));
        assert!(!app_tool(&hinted, false).needs_confirmation);
        assert!(
            app_tool(&hinted, true).needs_confirmation,
            "untrusted server"
        );
        assert!(
            app_tool(&tool(Value::Null), false).needs_confirmation,
            "no hint"
        );
    }
}
