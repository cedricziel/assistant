//! [`RequestBoundServer`] hands every request and notification to the
//! server bound in its HTTP request parts, as the `/mcp` endpoint does.

mod common;

use std::borrow::Cow;
use std::sync::Arc;

use assistant_mcp_server::{AssistantMcpServer, ListChangeListeners, RequestBoundServer};
use common::*;
use http::request::Parts;
use rmcp::model::{ClientJsonRpcMessage, GetExtensions, ProtocolVersion};
use rmcp::service::{RxJsonRpcMessage, TxJsonRpcMessage};
use rmcp::transport::{IntoTransport, Transport};
use rmcp::{RoleServer, ServiceExt};
use tokio::io::DuplexStream;

/// Attaches HTTP request parts carrying `server` to every incoming message,
/// the way rmcp's Streamable HTTP service does after the `/mcp` middleware
/// bound a server to the request.
struct BindingTransport<T> {
    inner: T,
    parts: Parts,
}

impl<T: Transport<RoleServer>> Transport<RoleServer> for BindingTransport<T> {
    type Error = T::Error;

    fn name() -> Cow<'static, str> {
        T::name()
    }

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleServer>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        self.inner.send(item)
    }

    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleServer>> {
        let mut message = self.inner.receive().await?;
        match &mut message {
            ClientJsonRpcMessage::Request(request) => {
                request.request.extensions_mut().insert(self.parts.clone());
            }
            ClientJsonRpcMessage::Notification(notification) => {
                notification
                    .notification
                    .extensions_mut()
                    .insert(self.parts.clone());
            }
            _ => {}
        }
        Some(message)
    }

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.inner.close()
    }
}

/// Serves [`RequestBoundServer`] with `server` bound to every message.
fn spawn_bound(server: AssistantMcpServer) -> DuplexStream {
    let (mut parts, ()) = http::Request::new(()).into_parts();
    parts.extensions.insert(Arc::new(server));
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let transport = BindingTransport {
        inner: IntoTransport::<RoleServer, _, _>::into_transport(server_io),
        parts,
    };
    tokio::spawn(async move {
        if let Ok(running) = RequestBoundServer.serve(transport).await {
            let _ = running.waiting().await;
        }
    });
    client_io
}

#[tokio::test]
async fn bound_legacy_session_is_notified_when_another_server_installs_a_skill() {
    let listeners = ListChangeListeners::default();
    let bound = default_stubs()
        .listeners(listeners.clone())
        .server(allow_everything());
    let (recorder, mut received) = Recorder::new(ProtocolVersion::V_2025_11_25);
    let _session = recorder.serve(spawn_bound(bound)).await.expect("handshake");
    let installer = connect(installing_server(default_stubs().listeners(listeners)).await).await;

    let (_source, params) = install_params("fresh-skill");
    installer.call_tool(params).await.unwrap();

    assert_eq!(next_received(&mut received).await, Received::ToolsChanged);
}
