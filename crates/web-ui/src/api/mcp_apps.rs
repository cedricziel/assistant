//! MCP Apps API: the app an external MCP tool attached to its result.

use serde::Serialize;
use utoipa::ToSchema;

use assistant_core::mcp_app::{McpApp, McpAppCsp, McpAppPermissions};

// -- Views --

/// The interactive UI an external MCP tool declared for its result.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct McpAppView {
    /// Namespace of the server the tool came from, as in
    /// `mcp__{server}__{tool}`.
    pub server: String,
    /// The `ui://` resource the HTML was read from.
    pub resource_uri: String,
    /// The app document, served as `text/html;profile=mcp-app`.
    pub html: String,
    pub csp: McpAppCspView,
    pub permissions: McpAppPermissionsView,
    /// Whether the app asks to be drawn with a border.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefers_border: Option<bool>,
}

/// The origins the app may reach. An empty list means the restrictive default.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct McpAppCspView {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connect_domains: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resource_domains: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub frame_domains: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub base_uri_domains: Vec<String>,
}

/// The device permissions the app asks for.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct McpAppPermissionsView {
    pub camera: bool,
    pub microphone: bool,
    pub geolocation: bool,
    pub clipboard_write: bool,
}

impl From<McpApp> for McpAppView {
    fn from(app: McpApp) -> Self {
        Self {
            server: app.server,
            resource_uri: app.resource_uri,
            html: app.html,
            csp: app.csp.into(),
            permissions: app.permissions.into(),
            prefers_border: app.prefers_border,
        }
    }
}

impl From<McpAppCsp> for McpAppCspView {
    fn from(csp: McpAppCsp) -> Self {
        Self {
            connect_domains: csp.connect_domains,
            resource_domains: csp.resource_domains,
            frame_domains: csp.frame_domains,
            base_uri_domains: csp.base_uri_domains,
        }
    }
}

impl From<McpAppPermissions> for McpAppPermissionsView {
    fn from(p: McpAppPermissions) -> Self {
        Self {
            camera: p.camera,
            microphone: p.microphone,
            geolocation: p.geolocation,
            clipboard_write: p.clipboard_write,
        }
    }
}

/// The app stored on a tool row, if it parses.
pub(crate) fn stored_app(raw: Option<&str>) -> Option<McpApp> {
    serde_json::from_str(raw?).ok()
}
