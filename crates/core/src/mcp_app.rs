//! MCP Apps: an interactive UI an external MCP tool declares for its result.
//!
//! The MCP client resolves the tool's `ui://` resource after a call and rides
//! the [`McpApp`] along on `ToolOutput.data` under [`McpApp::DATA_KEY`]. The
//! runtime takes it back out, persists it with the tool result and hands it
//! to the chat, which renders it. Calls the app makes later go back to its
//! server through [`McpAppServers`].

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::tool::TurnScope;

/// An app resolved for one tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpApp {
    /// The namespace of the server the tool came from, as in
    /// `mcp__{server}__{tool}`.
    pub server: String,
    /// The `ui://` resource the HTML was read from.
    pub resource_uri: String,
    pub html: String,
    #[serde(default)]
    pub csp: McpAppCsp,
    #[serde(default)]
    pub permissions: McpAppPermissions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefers_border: Option<bool>,
}

/// The origins an app may reach, from the resource's `_meta.ui.csp`. Empty
/// lists mean the restrictive default.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpAppCsp {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connect_domains: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_domains: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frame_domains: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub base_uri_domains: Vec<String>,
}

/// The device permissions an app asks for in `_meta.ui.permissions`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpAppPermissions {
    #[serde(default)]
    pub camera: bool,
    #[serde(default)]
    pub microphone: bool,
    #[serde(default)]
    pub geolocation: bool,
    #[serde(default)]
    pub clipboard_write: bool,
}

impl McpApp {
    /// The key of `ToolOutput.data` that carries the app.
    pub const DATA_KEY: &'static str = "mcp_app";

    /// Add the app to a tool's structured `data` under [`Self::DATA_KEY`].
    /// Data that is not an object is replaced, since the key needs one.
    pub fn attach_to(self, data: Option<Value>) -> Option<Value> {
        let mut object = match data {
            Some(Value::Object(object)) => object,
            _ => Map::new(),
        };
        let app = serde_json::to_value(self).ok()?;
        object.insert(Self::DATA_KEY.to_string(), app);
        Some(Value::Object(object))
    }

    /// Take the app out of a tool's `data`. Data left empty becomes `None`;
    /// a value under the key that is not an app stays where it is.
    pub fn take_from(data: &mut Option<Value>) -> Option<Self> {
        let object = data.as_mut()?.as_object_mut()?;
        let app = Self::deserialize(object.get(Self::DATA_KEY)?).ok()?;
        object.remove(Self::DATA_KEY);
        if object.is_empty() {
            *data = None;
        }
        Some(app)
    }
}

// -- App-initiated calls --

/// What an app may do with one of its server's tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpAppTool {
    /// The tool's `_meta.ui.visibility` includes `app`.
    pub visible_to_app: bool,
    /// The user must confirm a call: the tool lacks `readOnlyHint: true`, or
    /// its server is not trusted.
    pub needs_confirmation: bool,
}

/// One server, connected for the caller, that an app talks to.
#[async_trait]
pub trait McpAppServer: Send + Sync {
    /// The tool called `name`, or `None` when the server has none.
    fn tool(&self, name: &str) -> Option<McpAppTool>;

    /// Call `name` and return the MCP `CallToolResult` as JSON.
    async fn call_tool(&self, name: &str, arguments: Map<String, Value>) -> Result<Value>;

    /// Read `uri` and return the MCP `ReadResourceResult` as JSON.
    async fn read_resource(&self, uri: &str) -> Result<Value>;
}

/// Finds the server an app came from.
#[async_trait]
pub trait McpAppServers: Send + Sync {
    /// The server named `server` as `scope` sees it, or `None` when it is
    /// not one `scope` may use.
    async fn server(
        &self,
        scope: &TurnScope<'_>,
        server: &str,
    ) -> Result<Option<Arc<dyn McpAppServer>>>;
}

/// Asks each source in turn; the first that knows the server answers.
#[derive(Clone, Default)]
pub struct McpAppServerChain(Vec<Arc<dyn McpAppServers>>);

impl McpAppServerChain {
    pub fn new(sources: Vec<Arc<dyn McpAppServers>>) -> Self {
        Self(sources)
    }
}

#[async_trait]
impl McpAppServers for McpAppServerChain {
    async fn server(
        &self,
        scope: &TurnScope<'_>,
        server: &str,
    ) -> Result<Option<Arc<dyn McpAppServer>>> {
        for source in &self.0 {
            if let Some(found) = source.server(scope, server).await? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn app() -> McpApp {
        McpApp {
            server: "weather".into(),
            resource_uri: "ui://weather/forecast".into(),
            html: "<p>sunny</p>".into(),
            csp: McpAppCsp::default(),
            permissions: McpAppPermissions::default(),
            prefers_border: None,
        }
    }

    #[test]
    fn attached_app_is_taken_back_and_leaves_the_rest_of_the_data() {
        let mut data = app().attach_to(Some(json!({"temp": 21})));
        assert_eq!(McpApp::take_from(&mut data), Some(app()));
        assert_eq!(data, Some(json!({"temp": 21})), "structured data stays");
    }

    #[test]
    fn data_holding_only_the_app_becomes_none() {
        let mut data = app().attach_to(None);
        assert_eq!(McpApp::take_from(&mut data), Some(app()));
        assert_eq!(data, None);
    }

    #[test]
    fn a_value_under_the_key_that_is_not_an_app_stays() {
        let mut data = Some(json!({"mcp_app": "not an app"}));
        assert_eq!(McpApp::take_from(&mut data), None);
        assert_eq!(data, Some(json!({"mcp_app": "not an app"})));
    }

    #[test]
    fn data_without_an_app_is_untouched() {
        let mut data = Some(json!({"kind": "terminal"}));
        assert_eq!(McpApp::take_from(&mut data), None);
        assert_eq!(data, Some(json!({"kind": "terminal"})));
    }

    struct Named(&'static str);

    struct NoTools;

    #[async_trait]
    impl McpAppServer for NoTools {
        fn tool(&self, _name: &str) -> Option<McpAppTool> {
            None
        }
        async fn call_tool(&self, _name: &str, _arguments: Map<String, Value>) -> Result<Value> {
            Ok(Value::Null)
        }
        async fn read_resource(&self, _uri: &str) -> Result<Value> {
            Ok(Value::Null)
        }
    }

    #[async_trait]
    impl McpAppServers for Named {
        async fn server(
            &self,
            _scope: &TurnScope<'_>,
            server: &str,
        ) -> Result<Option<Arc<dyn McpAppServer>>> {
            Ok((server == self.0).then(|| Arc::new(NoTools) as Arc<dyn McpAppServer>))
        }
    }

    #[tokio::test]
    async fn chain_finds_the_server_in_any_source() {
        let chain = McpAppServerChain::new(vec![Arc::new(Named("a")), Arc::new(Named("b"))]);
        let scope = TurnScope {
            org_slug: "o",
            space_slug: "s",
            user_id: None,
        };
        assert!(chain.server(&scope, "b").await.unwrap().is_some());
        assert!(chain.server(&scope, "c").await.unwrap().is_none());
    }
}
