//! Caller identity for MCP requests.
//!
//! Over HTTP, rmcp copies each request's `http::request::Parts` into the
//! request extensions, and the `/mcp` auth middleware puts the resolved
//! [`Caller`] into the `Parts` extensions. Over stdio
//! there is no HTTP layer, so [`with_identity`] stamps every inbound request
//! with the identity the process runs as.

use std::borrow::Cow;

use http::request::Parts;
use rmcp::model::{ClientJsonRpcMessage, Extensions, GetExtensions};
use rmcp::service::{RxJsonRpcMessage, TxJsonRpcMessage};
use rmcp::transport::{IntoTransport, Transport};
use rmcp::{ErrorData, RoleServer};

use assistant_core::auth::AuthContext;
use assistant_core::identity::{Role, SpaceId};

/// The identity a request runs as, bound to one space, with the caller's
/// effective role there.
#[derive(Clone, Debug)]
pub struct Caller {
    pub auth: AuthContext,
    pub space_id: SpaceId,
    pub role: Role,
}

impl Caller {
    /// Whether the caller's role in the space may run tools.
    ///
    /// Only the role decides: the `mcp:execute` scope an MCP token carries
    /// does not name the individual resources the web API's scope checks do.
    pub fn may_run_tools(&self) -> bool {
        !matches!(self.role, Role::Viewer)
    }
}

/// Resolves the identity a request carries and the space it is bound to.
///
/// Over HTTP the `/mcp` middleware put the whole [`Caller`] into the `Parts`
/// extensions. Over stdio the [`AuthContext`] is a direct extension and must hold
/// a role in exactly one space. A request with no identity is rejected: it is
/// never treated as the system caller.
pub(crate) fn resolve_caller(extensions: &Extensions) -> Result<Caller, ErrorData> {
    if let Some(parts) = extensions.get::<Parts>() {
        return parts
            .extensions
            .get::<Caller>()
            .cloned()
            .ok_or_else(no_identity);
    }

    let auth = extensions.get::<AuthContext>().ok_or_else(no_identity)?;
    let mut spaces = auth.space_roles.iter();
    let (space_id, role) = match (spaces.next(), spaces.next()) {
        (Some(only), None) => only,
        _ => {
            return Err(ErrorData::invalid_request(
                "the caller must hold a role in exactly one space",
                None,
            ));
        }
    };
    Ok(Caller {
        auth: auth.clone(),
        space_id: space_id.clone(),
        role: role.clone(),
    })
}

fn no_identity() -> ErrorData {
    ErrorData::invalid_request("request carries no caller identity", None)
}

/// Wraps `transport` so every request and notification it delivers carries
/// `auth` as a direct extension.
pub fn with_identity<T, E, A>(
    transport: T,
    auth: AuthContext,
) -> impl Transport<RoleServer, Error = E> + 'static
where
    T: IntoTransport<RoleServer, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    IdentityTransport {
        inner: transport.into_transport(),
        auth,
    }
}

struct IdentityTransport<T> {
    inner: T,
    auth: AuthContext,
}

impl<T> Transport<RoleServer> for IdentityTransport<T>
where
    T: Transport<RoleServer>,
{
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
                request.request.extensions_mut().insert(self.auth.clone());
            }
            ClientJsonRpcMessage::Notification(notification) => {
                notification
                    .notification
                    .extensions_mut()
                    .insert(self.auth.clone());
            }
            _ => {}
        }
        Some(message)
    }

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.inner.close()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rmcp::model::ErrorCode;

    use assistant_core::identity::{OrgId, UserId};

    use super::*;

    fn auth(roles: &[(&str, Role)]) -> AuthContext {
        AuthContext {
            user_id: UserId::from("user-1"),
            org_id: OrgId::from("org-1"),
            email: "user@example.com".into(),
            space_roles: roles
                .iter()
                .map(|(space, role)| (SpaceId::from(*space), role.clone()))
                .collect::<HashMap<_, _>>(),
            scopes: Vec::new(),
            client_id: "client".into(),
        }
    }

    /// What rmcp's HTTP service delivers: the request `Parts`, carrying
    /// whatever the auth middleware inserted into the HTTP extensions.
    fn http_extensions(caller: Option<Caller>) -> Extensions {
        let (mut parts, ()) = http::Request::builder()
            .uri("/mcp")
            .body(())
            .expect("request")
            .into_parts();
        if let Some(caller) = caller {
            parts.extensions.insert(caller);
        }
        let mut extensions = Extensions::new();
        extensions.insert(parts);
        extensions
    }

    fn direct_extensions(auth: AuthContext) -> Extensions {
        let mut extensions = Extensions::new();
        extensions.insert(auth);
        extensions
    }

    #[test]
    fn reads_a_direct_identity_extension() {
        let caller =
            resolve_caller(&direct_extensions(auth(&[("s1", Role::Member)]))).expect("resolves");
        assert_eq!(caller.space_id, SpaceId::from("s1"));
        assert_eq!(caller.role, Role::Member);
    }

    #[test]
    fn a_direct_identity_with_several_spaces_is_rejected() {
        let identity = auth(&[("s1", Role::Member), ("s2", Role::Viewer)]);
        let error = resolve_caller(&direct_extensions(identity)).expect_err("ambiguous");
        assert_eq!(error.code, ErrorCode::INVALID_REQUEST);
    }

    #[test]
    fn uses_the_caller_the_http_middleware_bound() {
        let caller = Caller {
            auth: auth(&[("s1", Role::Member), ("s2", Role::Viewer)]),
            space_id: SpaceId::from("s2"),
            role: Role::Viewer,
        };
        let resolved = resolve_caller(&http_extensions(Some(caller))).expect("resolves");
        assert_eq!(resolved.auth.user_id, UserId::from("user-1"));
        assert_eq!(resolved.space_id, SpaceId::from("s2"));
        assert_eq!(resolved.role, Role::Viewer);
    }

    #[test]
    fn rejects_a_request_without_identity() {
        let error = resolve_caller(&Extensions::new()).expect_err("rejected");
        assert_eq!(error.code, ErrorCode::INVALID_REQUEST);
        let error = resolve_caller(&http_extensions(None)).expect_err("rejected");
        assert_eq!(error.code, ErrorCode::INVALID_REQUEST);
    }
}
