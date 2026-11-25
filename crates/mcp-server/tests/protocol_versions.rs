//! Protocol revision handling: `2026-07-28` and `2025-11-25` are served,
//! anything older is not.

mod common;

use std::time::Duration;

use common::*;
use rmcp::model::{
    ClientCapabilities, ClientConfig, ClientJsonRpcMessage, ClientRequest, DiscoverRequest,
    DiscoverRequestParams, ErrorCode, Implementation, ListToolsRequest, ProtocolVersion, RequestId,
    RequestMetaObject, ServerJsonRpcMessage, ServerResult,
};
use rmcp::transport::{IntoTransport, Transport};
use rmcp::{RoleClient, ServiceExt};
use serde_json::json;

const SUPPORTED: [&str; 2] = ["2026-07-28", "2025-11-25"];

fn spawn_default_server() -> tokio::io::DuplexStream {
    spawn_server(default_stubs().server(allow_everything()))
}

/// A raw client-side endpoint, for sending requests rmcp's client would not.
fn raw_client() -> impl Transport<RoleClient> {
    IntoTransport::<RoleClient, _, _>::into_transport(spawn_default_server())
}

fn meta(version: ProtocolVersion) -> RequestMetaObject {
    RequestMetaObject::with_client_context(
        version,
        Implementation::new("test-client", "0.0.0"),
        ClientCapabilities::default(),
    )
}

fn discover_request(version: ProtocolVersion) -> ClientJsonRpcMessage {
    let mut request = DiscoverRequest::new(DiscoverRequestParams {});
    request.extensions.insert(meta(version));
    ClientJsonRpcMessage::request(
        ClientRequest::DiscoverRequest(request),
        RequestId::Number(1),
    )
}

#[tokio::test]
async fn discover_reports_both_supported_revisions() {
    let mut transport = raw_client();
    transport
        .send(discover_request(ProtocolVersion::V_2026_07_28))
        .await
        .unwrap();

    let Some(ServerJsonRpcMessage::Response(response)) = transport.receive().await else {
        panic!("expected a discover response");
    };
    let ServerResult::DiscoverResult(result) = response.result else {
        panic!("expected a discover result, got {:?}", response.result);
    };

    let versions: Vec<&str> = result
        .supported_versions
        .iter()
        .map(ProtocolVersion::as_str)
        .collect();
    for expected in SUPPORTED {
        assert!(versions.contains(&expected), "{expected} in {versions:?}");
    }
    assert_eq!(versions.len(), SUPPORTED.len(), "{versions:?}");
    assert!(result.capabilities.tools.is_some());
    assert_eq!(
        result.server_info().map(|info| info.name),
        Some("assistant-mcp-server".to_string())
    );
}

/// Connects an rmcp client that asks for `requested` in its `initialize`.
async fn connect_requesting(
    requested: ProtocolVersion,
) -> rmcp::service::RunningService<RoleClient, ClientConfig> {
    let mut config = ClientConfig::default();
    config.protocol_version = requested;
    config
        .serve(spawn_default_server())
        .await
        .expect("handshake")
}

#[tokio::test]
async fn initialize_at_2025_11_25_serves_tools_and_resources() {
    let client = connect_requesting(ProtocolVersion::V_2025_11_25).await;

    assert_eq!(
        client.peer_info().unwrap().protocol_version,
        ProtocolVersion::V_2025_11_25
    );
    assert!(!client.list_all_tools().await.unwrap().is_empty());
    client.list_all_resources().await.unwrap();
}

#[tokio::test]
async fn initialize_at_2024_11_05_is_never_agreed_to() {
    // rmcp answers `initialize` with the newest revision the server supports;
    // a client that only speaks 2024-11-05 then disconnects.
    let client = connect_requesting(ProtocolVersion::V_2024_11_05).await;
    assert_eq!(
        client.peer_info().unwrap().protocol_version,
        ProtocolVersion::V_2025_11_25
    );
}

#[tokio::test]
async fn per_request_2024_11_05_is_rejected_with_the_supported_list() {
    let mut transport = raw_client();
    // Discover is a bootstrap probe, so version enforcement happens on the
    // first lifecycle request. Send one carrying the unsupported revision.
    let mut list = ListToolsRequest::default();
    list.extensions.insert(meta(ProtocolVersion::V_2024_11_05));
    transport
        .send(ClientJsonRpcMessage::request(
            ClientRequest::ListToolsRequest(list),
            RequestId::Number(2),
        ))
        .await
        .unwrap();

    let error = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(message) = transport.receive().await {
            if let ServerJsonRpcMessage::Error(e) = message {
                return Some(e.error);
            }
        }
        None
    })
    .await
    .expect("the server answers an unsupported revision")
    .expect("an error response");
    assert_eq!(error.code, ErrorCode::UNSUPPORTED_PROTOCOL_VERSION);
    assert_eq!(
        error.data,
        Some(json!({
            "requested": "2024-11-05",
            "supported": ["2025-11-25", "2026-07-28"],
        }))
    );
}
