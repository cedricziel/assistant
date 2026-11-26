//! MCP Apps on the client side: resolving the `ui://` resource a tool
//! declares into an [`McpApp`].

use anyhow::{Context, Result, bail};
use rmcp::model::{MetaObject, ResourceContents, Tool};
use serde_json::Value;
use tracing::{debug, warn};

use assistant_core::mcp_app::{McpApp, McpAppCsp, McpAppPermissions};

use crate::client::McpClient;

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
    let (mime_type, html, meta) = match contents {
        ResourceContents::TextResourceContents {
            mime_type,
            text,
            meta,
            ..
        } => (mime_type, text, meta),
        ResourceContents::BlobResourceContents {
            mime_type,
            blob,
            meta,
            ..
        } => {
            let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, blob)
                .context("the blob is not valid base64")?;
            let html = String::from_utf8(bytes).context("the blob is not UTF-8")?;
            (mime_type, html, meta)
        }
        _ => bail!("the resource content is of an unknown kind"),
    };
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
    let ui = match meta.as_ref().and_then(|meta| meta.0.get("ui")) {
        Some(ui) => Some(ui.clone()),
        None => listed_ui(client, uri).await,
    };
    let ui = ui.unwrap_or(Value::Null);
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
