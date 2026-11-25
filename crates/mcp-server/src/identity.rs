//! Caller identity for MCP requests.
//!
//! Over HTTP, rmcp copies each request's `http::request::Parts` into the
//! request extensions, and the `/mcp` auth middleware puts the resolved
//! [`AuthContext`] into the `Parts` extensions. Over stdio there is no HTTP
//! layer, so [`with_identity`] stamps every inbound request with the identity
//! the process runs as.

use std::borrow::Cow;

use http::request::Parts;
use rmcp::model::{ClientJsonRpcMessage, Extensions, GetExtensions};
use rmcp::service::{RxJsonRpcMessage, TxJsonRpcMessage};
use rmcp::transport::{IntoTransport, Transport};
use rmcp::{ErrorData, RoleServer};

use assistant_core::auth::AuthContext;
use assistant_core::identity::{Role, SpaceId};

/// Header that selects the space when the caller holds roles in several.
///
/// Matches the web API's selector header.
const SPACE_HEADER: &str = "x-space-id";

/// The identity a request runs as, bound to one space.
#[derive(Debug)]
pub(crate) struct Caller {
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

/// Resolves the identity a request carries and binds it to a space.
///
/// Reads the [`AuthContext`] from the request extensions directly or, over
/// HTTP, from the `Parts` extensions. A request with no identity is rejected:
/// it is never treated as the system caller. The space is the one named by
/// the `x-space-id` header, or the caller's only space; the caller must hold
/// a role there, or be an org admin.
pub(crate) fn resolve_caller(extensions: &Extensions) -> Result<Caller, ErrorData> {
    let parts = extensions.get::<Parts>();
    let auth = extensions
        .get::<AuthContext>()
        .or_else(|| parts.and_then(|parts| parts.extensions.get::<AuthContext>()))
        .ok_or_else(|| ErrorData::invalid_request("request carries no caller identity", None))?;

    let selector = parts
        .and_then(|parts| parts.headers.get(SPACE_HEADER))
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty());
    let space_id = match selector {
        Some(id) => SpaceId::from(id),
        None => single_space(auth)?,
    };
    let role = match auth.role_in(&space_id) {
        Some(role) => role.clone(),
        None if auth.is_org_admin() => Role::OrgAdmin,
        None => return Err(ErrorData::invalid_request("access denied", None)),
    };
    Ok(Caller {
        auth: auth.clone(),
        space_id,
        role,
    })
}

fn single_space(auth: &AuthContext) -> Result<SpaceId, ErrorData> {
    let mut spaces = auth.space_roles.keys();
    match (spaces.next(), spaces.next()) {
        (Some(only), None) => Ok(only.clone()),
        _ => Err(ErrorData::invalid_request(
            format!("space selector required: send the {SPACE_HEADER} header"),
            None,
        )),
    }
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
    fn http_extensions(auth: Option<AuthContext>, space_header: Option<&str>) -> Extensions {
        let mut builder = http::Request::builder().uri("/mcp");
        if let Some(space) = space_header {
            builder = builder.header(SPACE_HEADER, space);
        }
        let (mut parts, ()) = builder.body(()).expect("request").into_parts();
        if let Some(auth) = auth {
            parts.extensions.insert(auth);
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
    fn reads_the_identity_the_http_middleware_put_into_the_parts() {
        let extensions = http_extensions(Some(auth(&[("s1", Role::SpaceAdmin)])), None);
        let caller = resolve_caller(&extensions).expect("resolves");
        assert_eq!(caller.auth.user_id, UserId::from("user-1"));
        assert_eq!(caller.role, Role::SpaceAdmin);
    }

    #[test]
    fn rejects_a_request_without_identity() {
        let error = resolve_caller(&Extensions::new()).expect_err("rejected");
        assert_eq!(error.code, ErrorCode::INVALID_REQUEST);
        let error = resolve_caller(&http_extensions(None, None)).expect_err("rejected");
        assert_eq!(error.code, ErrorCode::INVALID_REQUEST);
    }

    #[test]
    fn several_spaces_need_a_selector_header() {
        let identity = auth(&[("s1", Role::Member), ("s2", Role::Viewer)]);

        let error = resolve_caller(&http_extensions(Some(identity.clone()), None))
            .expect_err("selector required");
        assert!(error.message.contains(SPACE_HEADER), "{}", error.message);

        let caller = resolve_caller(&http_extensions(Some(identity), Some("s2"))).expect("s2");
        assert_eq!(caller.space_id, SpaceId::from("s2"));
        assert_eq!(caller.role, Role::Viewer);
    }

    #[test]
    fn a_space_without_a_role_is_denied() {
        let extensions = http_extensions(Some(auth(&[("s1", Role::Member)])), Some("other"));
        let error = resolve_caller(&extensions).expect_err("denied");
        assert_eq!(error.code, ErrorCode::INVALID_REQUEST);
    }

    #[test]
    fn org_admins_may_select_any_space() {
        let extensions = http_extensions(Some(auth(&[("s1", Role::OrgAdmin)])), Some("other"));
        let caller = resolve_caller(&extensions).expect("org admin");
        assert_eq!(caller.space_id, SpaceId::from("other"));
        assert_eq!(caller.role, Role::OrgAdmin);
    }
}
