//! MCP JSON-RPC 2.0 protocol types for the client side.
//!
//! These types mirror the MCP specification from the client perspective:
//! we *build* requests and *parse* responses.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── JSON-RPC 2.0 envelope ─────────────────────────────────────────────────────

/// A JSON-RPC 2.0 request we send to the server.
#[derive(Debug, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Build a request that expects a response (has an `id`).
    pub fn call(id: u64, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0",
            id: Some(id),
            method: method.into(),
            params,
        }
    }

    /// Build a notification (no `id`, no response expected).
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0",
            id: None,
            method: method.into(),
            params,
        }
    }
}

/// A JSON-RPC 2.0 response or notification received from the server.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcMessage {
    #[allow(dead_code)]
    pub jsonrpc: Option<String>,
    /// Present for responses; absent for server-initiated notifications.
    pub id: Option<Value>,
    /// Present on success responses.
    pub result: Option<Value>,
    /// Present on error responses.
    pub error: Option<JsonRpcError>,
    /// Present on server-initiated notifications (no `id`).
    pub method: Option<String>,
    /// Parameters for server-initiated notifications.
    pub params: Option<Value>,
}

impl JsonRpcMessage {
    /// True if the `id` field is absent or `null`.
    fn id_absent_or_null(&self) -> bool {
        self.id.as_ref().is_none_or(|v| v.is_null())
    }

    /// True if this is a server-initiated notification (no `id`, has `method`).
    ///
    /// Handles servers that send `"id": null` instead of omitting the field.
    pub fn is_notification(&self) -> bool {
        self.id_absent_or_null() && self.method.is_some()
    }

    /// True if this is a response to a request we sent.
    pub fn is_response(&self) -> bool {
        !self.id_absent_or_null()
    }

    /// Extract the numeric request ID, if present.
    pub fn request_id(&self) -> Option<u64> {
        self.id.as_ref().and_then(|v| v.as_u64())
    }

    /// Unwrap the result, returning an error if the response was an error.
    pub fn into_result(self) -> anyhow::Result<Value> {
        if let Some(err) = self.error {
            anyhow::bail!("JSON-RPC error {}: {}", err.code, err.message);
        }
        self.result
            .ok_or_else(|| anyhow::anyhow!("JSON-RPC response missing both result and error"))
    }
}

/// JSON-RPC error object.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

// ── MCP initialize ────────────────────────────────────────────────────────────

/// Client capabilities sent during `initialize`.
#[derive(Debug, Serialize)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
}

/// Whether the client supports `roots/list`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootsCapability {
    pub list_changed: bool,
}

/// Client info sent during `initialize`.
#[derive(Debug, Serialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// Parameters for the `initialize` request.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

/// Server response to `initialize`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    #[serde(default)]
    pub capabilities: ServerCapabilities,
    #[serde(default)]
    pub server_info: Option<ServerInfo>,
}

/// Server capabilities advertised in the `initialize` response.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    #[serde(default)]
    pub tools: Option<ToolsCapability>,
    #[serde(default)]
    pub resources: Option<ResourcesCapability>,
    #[serde(default)]
    pub prompts: Option<PromptsCapability>,
}

/// Server capabilities for tools.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    #[serde(default)]
    pub list_changed: bool,
}

/// Server capabilities for resources.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesCapability {
    #[serde(default)]
    pub subscribe: bool,
    #[serde(default)]
    pub list_changed: bool,
}

/// Server capabilities for prompts.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsCapability {
    #[serde(default)]
    pub list_changed: bool,
}

/// Server identification.
#[derive(Debug, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
}

// ── tools/list ────────────────────────────────────────────────────────────────

/// Response payload for `tools/list`.
#[derive(Debug, Deserialize)]
pub struct ToolListResult {
    pub tools: Vec<RemoteTool>,
}

/// A tool advertised by a remote MCP server.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTool {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_object_schema")]
    pub input_schema: Value,
}

fn default_object_schema() -> Value {
    serde_json::json!({ "type": "object", "properties": {} })
}

// ── tools/call ────────────────────────────────────────────────────────────────

/// Parameters for `tools/call`.
#[derive(Debug, Serialize)]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: Value,
}

/// Response payload for `tools/call`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    #[serde(default)]
    pub content: Vec<ContentItem>,
    #[serde(default)]
    pub is_error: Option<bool>,
}

/// A content block in a tool result or resource.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentItem {
    /// Content type: `"text"`, `"image"`, `"resource"`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Text content (present when `kind == "text"`).
    #[serde(default)]
    pub text: Option<String>,
    /// Base64-encoded data (present when `kind == "image"`).
    #[serde(default)]
    pub data: Option<String>,
    /// MIME type (present when `kind == "image"`).
    #[serde(default)]
    pub mime_type: Option<String>,
}

// ── resources/list ────────────────────────────────────────────────────────────

/// Response payload for `resources/list`.
#[derive(Debug, Deserialize)]
pub struct ResourceListResult {
    pub resources: Vec<RemoteResource>,
}

/// A resource advertised by a remote MCP server.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteResource {
    pub uri: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
}

// ── resources/read ────────────────────────────────────────────────────────────

/// Parameters for `resources/read`.
#[derive(Debug, Serialize)]
pub struct ResourceReadParams {
    pub uri: String,
}

/// Response payload for `resources/read`.
#[derive(Debug, Deserialize)]
pub struct ResourceReadResult {
    pub contents: Vec<ResourceContent>,
}

/// A content block in a resource read response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceContent {
    pub uri: String,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    /// Base64-encoded binary content.
    #[serde(default)]
    pub blob: Option<String>,
}

// ── MCP protocol constants ────────────────────────────────────────────────────

/// The MCP protocol version we advertise during initialization.
///
/// `2024-11-05` is the latest stable MCP specification version and is widely
/// supported by existing servers. Newer draft versions (e.g. `2025-03-26` for
/// Streamable HTTP) exist but are not yet finalized; update this constant when
/// a newer stable version is published and server adoption is sufficient.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Standard MCP method names.
pub mod method {
    pub const INITIALIZE: &str = "initialize";
    pub const INITIALIZED: &str = "notifications/initialized";
    pub const TOOLS_LIST: &str = "tools/list";
    pub const TOOLS_CALL: &str = "tools/call";
    pub const RESOURCES_LIST: &str = "resources/list";
    pub const RESOURCES_READ: &str = "resources/read";
    pub const PING: &str = "ping";
}

/// Standard MCP notification names.
pub mod notification {
    pub const TOOLS_LIST_CHANGED: &str = "notifications/tools/list_changed";
    pub const RESOURCES_LIST_CHANGED: &str = "notifications/resources/list_changed";
    pub const CANCELLED: &str = "notifications/cancelled";
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_rpc_request_call_serializes() {
        let req = JsonRpcRequest::call(1, "tools/list", None);
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 1);
        assert_eq!(json["method"], "tools/list");
        assert!(json.get("params").is_none());
    }

    #[test]
    fn json_rpc_request_notification_omits_id() {
        let req = JsonRpcRequest::notification("notifications/initialized", None);
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("id").is_none());
    }

    #[test]
    fn json_rpc_message_success_response() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let msg: JsonRpcMessage = serde_json::from_str(raw).unwrap();
        assert!(msg.is_response());
        assert!(!msg.is_notification());
        assert_eq!(msg.request_id(), Some(1));
        let result = msg.into_result().unwrap();
        assert!(result["tools"].is_array());
    }

    #[test]
    fn json_rpc_message_error_response() {
        let raw =
            r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message":"Method not found"}}"#;
        let msg: JsonRpcMessage = serde_json::from_str(raw).unwrap();
        let err = msg.into_result().unwrap_err();
        assert!(err.to_string().contains("Method not found"));
    }

    #[test]
    fn json_rpc_message_notification() {
        let raw = r#"{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}"#;
        let msg: JsonRpcMessage = serde_json::from_str(raw).unwrap();
        assert!(msg.is_notification());
        assert!(!msg.is_response());
        assert_eq!(
            msg.method.as_deref(),
            Some("notifications/tools/list_changed")
        );
    }

    #[test]
    fn json_rpc_message_notification_with_null_id() {
        // Some servers send `"id": null` instead of omitting the field.
        let raw = r#"{"jsonrpc":"2.0","id":null,"method":"notifications/tools/list_changed"}"#;
        let msg: JsonRpcMessage = serde_json::from_str(raw).unwrap();
        assert!(
            msg.is_notification(),
            "null id should be treated as notification"
        );
        assert!(
            !msg.is_response(),
            "null id should not be treated as response"
        );
        assert_eq!(msg.request_id(), None);
    }

    #[test]
    fn remote_tool_deserializes_minimal() {
        let raw = r#"{"name":"echo","inputSchema":{"type":"object","properties":{"msg":{"type":"string"}}}}"#;
        let tool: RemoteTool = serde_json::from_str(raw).unwrap();
        assert_eq!(tool.name, "echo");
        assert!(tool.description.is_none());
    }

    #[test]
    fn remote_tool_deserializes_full() {
        let raw = r#"{
            "name": "search",
            "description": "Search the web",
            "inputSchema": {
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"]
            }
        }"#;
        let tool: RemoteTool = serde_json::from_str(raw).unwrap();
        assert_eq!(tool.name, "search");
        assert_eq!(tool.description.as_deref(), Some("Search the web"));
    }

    #[test]
    fn tool_call_result_deserializes() {
        let raw = r#"{
            "content": [
                { "type": "text", "text": "hello world" }
            ]
        }"#;
        let result: ToolCallResult = serde_json::from_str(raw).unwrap();
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].kind, "text");
        assert_eq!(result.content[0].text.as_deref(), Some("hello world"));
        assert!(result.is_error.is_none());
    }

    #[test]
    fn tool_call_result_with_error_flag() {
        let raw = r#"{
            "content": [{ "type": "text", "text": "something went wrong" }],
            "isError": true
        }"#;
        let result: ToolCallResult = serde_json::from_str(raw).unwrap();
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn initialize_result_deserializes() {
        let raw = r#"{
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": true },
                "resources": { "subscribe": false, "listChanged": false }
            },
            "serverInfo": { "name": "test-server", "version": "1.0" }
        }"#;
        let result: InitializeResult = serde_json::from_str(raw).unwrap();
        assert_eq!(result.protocol_version, "2024-11-05");
        assert!(result.capabilities.tools.is_some());
        assert_eq!(result.server_info.as_ref().unwrap().name, "test-server");
    }

    #[test]
    fn initialize_result_minimal() {
        let raw = r#"{"protocolVersion":"2024-11-05","capabilities":{}}"#;
        let result: InitializeResult = serde_json::from_str(raw).unwrap();
        assert!(result.capabilities.tools.is_none());
        assert!(result.server_info.is_none());
    }
}
