//! Dynamic client registration (RFC 7591).
//!
//! Manages OAuth2 client registration, lookup, and validation.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, bail};
use tokio::sync::RwLock;

use assistant_core::auth::{ClientInfo, ClientRegistration};

// `ClientStore` lives in `assistant_core::auth`; re-export for back-compat
// with existing callers that imported from this module.
pub use assistant_core::auth::ClientStore;

// -- In-memory implementation --

/// In-memory client store for testing and single-server deployments.
#[derive(Clone, Default)]
pub struct InMemoryClientStore {
    clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
}

#[async_trait::async_trait]
impl ClientStore for InMemoryClientStore {
    async fn register(&self, info: ClientInfo) -> Result<()> {
        self.clients
            .write()
            .await
            .insert(info.client_id.clone(), info);
        Ok(())
    }

    async fn get(&self, client_id: &str) -> Result<Option<ClientInfo>> {
        Ok(self.clients.read().await.get(client_id).cloned())
    }

    async fn list(&self) -> Result<Vec<ClientInfo>> {
        Ok(self.clients.read().await.values().cloned().collect())
    }
}

// -- Client registrar --

/// Validates and registers OAuth2 clients per RFC 7591.
pub struct ClientRegistrar {
    store: Arc<dyn ClientStore>,
}

/// Allowed grant types for validation.
const ALLOWED_GRANT_TYPES: &[&str] = &[
    "authorization_code",
    "refresh_token",
    "client_credentials",
    "urn:ietf:params:oauth:grant-type:device_code",
];

/// Allowed response types for validation.
const ALLOWED_RESPONSE_TYPES: &[&str] = &["code"];

/// Allowed token endpoint auth methods.
const ALLOWED_AUTH_METHODS: &[&str] = &["none", "client_secret_post", "client_secret_basic"];

impl ClientRegistrar {
    /// Create a new client registrar.
    pub fn new(store: Arc<dyn ClientStore>) -> Self {
        Self { store }
    }

    /// Register a new OAuth2 client.
    ///
    /// Validates the registration request, generates a `client_id`, and
    /// optionally generates a `client_secret` for confidential clients.
    pub async fn register(&self, req: ClientRegistration) -> Result<ClientInfo> {
        // Validate client name.
        if req.client_name.trim().is_empty() {
            bail!("client_name must not be empty");
        }

        // Validate redirect URIs.
        if req.redirect_uris.is_empty() {
            bail!("at least one redirect_uri is required");
        }
        for uri in &req.redirect_uris {
            validate_redirect_uri(uri)?;
        }

        // Validate grant types.
        if req.grant_types.is_empty() {
            bail!("at least one grant_type is required");
        }
        for gt in &req.grant_types {
            if !ALLOWED_GRANT_TYPES.contains(&gt.as_str()) {
                bail!("unsupported grant_type: {gt}");
            }
        }

        // Validate response types if provided.
        if let Some(ref rts) = req.response_types {
            for rt in rts {
                if !ALLOWED_RESPONSE_TYPES.contains(&rt.as_str()) {
                    bail!("unsupported response_type: {rt}");
                }
            }
        }

        // Validate auth method.
        let auth_method = req.token_endpoint_auth_method.as_deref().unwrap_or("none");
        if !ALLOWED_AUTH_METHODS.contains(&auth_method) {
            bail!("unsupported token_endpoint_auth_method: {auth_method}");
        }
        let is_confidential =
            auth_method == "client_secret_post" || auth_method == "client_secret_basic";

        // Cross-validate: authorization_code grant requires "code" response type (RFC 7591 §2).
        if req.grant_types.contains(&"authorization_code".to_string()) {
            let has_code = req
                .response_types
                .as_ref()
                .is_some_and(|rts| rts.contains(&"code".to_string()));
            if !has_code {
                bail!(
                    "grant_types includes 'authorization_code' but response_types does not include 'code'"
                );
            }
        }

        // Generate client_id.
        let client_id = format!("cid_{}", uuid::Uuid::new_v4().as_simple());

        // Generate client_secret for confidential clients.
        let client_secret = if is_confidential {
            Some(generate_client_secret())
        } else {
            None
        };

        let info = ClientInfo {
            client_id,
            client_name: req.client_name,
            client_secret,
            redirect_uris: req.redirect_uris,
            grant_types: req.grant_types,
            token_endpoint_auth_method: auth_method.to_owned(),
        };

        self.store.register(info.clone()).await?;
        Ok(info)
    }

    /// Look up a registered client by ID.
    pub async fn get(&self, client_id: &str) -> Result<Option<ClientInfo>> {
        self.store.get(client_id).await
    }

    /// List all registered clients.
    pub async fn list(&self) -> Result<Vec<ClientInfo>> {
        self.store.list().await
    }
}

/// Validate a redirect URI.
fn validate_redirect_uri(uri: &str) -> Result<()> {
    // Must be a valid absolute URI.
    let parsed: url::Url = uri
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid redirect_uri '{uri}': {e}"))?;

    // Must use HTTPS or localhost HTTP.
    match parsed.scheme() {
        "https" => {}
        "http" => {
            let host = parsed.host_str().unwrap_or("");
            if host != "localhost" && host != "127.0.0.1" && host != "[::1]" {
                bail!("redirect_uri with http:// is only allowed for localhost, got: {uri}");
            }
        }
        other => bail!("redirect_uri must use https:// or http://localhost, got scheme: {other}"),
    }

    // Must not contain a fragment.
    if parsed.fragment().is_some() {
        bail!("redirect_uri must not contain a fragment: {uri}");
    }

    Ok(())
}

/// Generate a cryptographically random client secret.
fn generate_client_secret() -> String {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use rand::Rng;

    let mut rng = rand::rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_registrar() -> ClientRegistrar {
        ClientRegistrar::new(Arc::new(InMemoryClientStore::default()))
    }

    fn valid_registration() -> ClientRegistration {
        ClientRegistration {
            client_name: "Test App".into(),
            redirect_uris: vec!["https://example.com/callback".into()],
            grant_types: vec!["authorization_code".into()],
            response_types: Some(vec!["code".into()]),
            token_endpoint_auth_method: None,
        }
    }

    #[tokio::test]
    async fn register_public_client() {
        let reg = make_registrar();
        let info = reg.register(valid_registration()).await.unwrap();

        assert!(
            info.client_id.starts_with("cid_"),
            "should have cid_ prefix"
        );
        assert_eq!(info.client_name, "Test App");
        assert!(info.client_secret.is_none(), "public client has no secret");
        assert_eq!(info.token_endpoint_auth_method, "none");
    }

    #[tokio::test]
    async fn register_confidential_client() {
        let reg = make_registrar();
        let mut req = valid_registration();
        req.token_endpoint_auth_method = Some("client_secret_post".into());

        let info = reg.register(req).await.unwrap();
        assert!(
            info.client_secret.is_some(),
            "confidential client should have a secret"
        );
    }

    #[tokio::test]
    async fn lookup_registered_client() {
        let reg = make_registrar();
        let info = reg.register(valid_registration()).await.unwrap();

        let found = reg.get(&info.client_id).await.unwrap();
        assert!(found.is_some(), "should find registered client");
        assert_eq!(found.unwrap().client_name, "Test App");
    }

    #[tokio::test]
    async fn lookup_unknown_client_returns_none() {
        let reg = make_registrar();
        let found = reg.get("nonexistent").await.unwrap();
        assert!(found.is_none(), "unknown client should return None");
    }

    #[tokio::test]
    async fn list_clients() {
        let reg = make_registrar();
        reg.register(valid_registration()).await.unwrap();

        let mut req2 = valid_registration();
        req2.client_name = "Second App".into();
        reg.register(req2).await.unwrap();

        let all = reg.list().await.unwrap();
        assert_eq!(all.len(), 2, "should have 2 registered clients");
    }

    #[tokio::test]
    async fn empty_client_name_is_rejected() {
        let reg = make_registrar();
        let mut req = valid_registration();
        req.client_name = "  ".into();
        assert!(reg.register(req).await.is_err());
    }

    #[tokio::test]
    async fn empty_redirect_uris_is_rejected() {
        let reg = make_registrar();
        let mut req = valid_registration();
        req.redirect_uris = vec![];
        assert!(reg.register(req).await.is_err());
    }

    #[tokio::test]
    async fn http_non_localhost_redirect_is_rejected() {
        let reg = make_registrar();
        let mut req = valid_registration();
        req.redirect_uris = vec!["http://evil.com/callback".into()];
        assert!(reg.register(req).await.is_err());
    }

    #[tokio::test]
    async fn http_localhost_redirect_is_allowed() {
        let reg = make_registrar();
        let mut req = valid_registration();
        req.redirect_uris = vec!["http://localhost:8080/callback".into()];
        assert!(reg.register(req).await.is_ok());
    }

    #[tokio::test]
    async fn fragment_in_redirect_is_rejected() {
        let reg = make_registrar();
        let mut req = valid_registration();
        req.redirect_uris = vec!["https://example.com/callback#fragment".into()];
        assert!(reg.register(req).await.is_err());
    }

    #[tokio::test]
    async fn unsupported_grant_type_is_rejected() {
        let reg = make_registrar();
        let mut req = valid_registration();
        req.grant_types = vec!["implicit".into()];
        assert!(reg.register(req).await.is_err());
    }

    #[tokio::test]
    async fn unsupported_response_type_is_rejected() {
        let reg = make_registrar();
        let mut req = valid_registration();
        req.response_types = Some(vec!["token".into()]);
        assert!(reg.register(req).await.is_err());
    }

    #[tokio::test]
    async fn unsupported_auth_method_is_rejected() {
        let reg = make_registrar();
        let mut req = valid_registration();
        req.token_endpoint_auth_method = Some("private_key_jwt".into());
        let err = reg.register(req).await.unwrap_err();
        assert!(
            err.to_string()
                .contains("unsupported token_endpoint_auth_method"),
            "should reject unsupported auth method, got: {err}"
        );
    }

    #[tokio::test]
    async fn auth_code_grant_requires_code_response_type() {
        let reg = make_registrar();
        let mut req = valid_registration();
        req.grant_types = vec!["authorization_code".into()];
        req.response_types = None; // Missing response_types
        let err = reg.register(req).await.unwrap_err();
        assert!(
            err.to_string()
                .contains("response_types does not include 'code'"),
            "should require code response type, got: {err}"
        );
    }

    #[tokio::test]
    async fn auth_code_grant_with_code_response_type_succeeds() {
        let reg = make_registrar();
        let mut req = valid_registration();
        req.grant_types = vec!["authorization_code".into(), "refresh_token".into()];
        req.response_types = Some(vec!["code".into()]);
        assert!(reg.register(req).await.is_ok());
    }

    #[tokio::test]
    async fn device_code_grant_type_is_allowed() {
        let reg = make_registrar();
        let mut req = valid_registration();
        req.grant_types = vec!["urn:ietf:params:oauth:grant-type:device_code".into()];
        let info = reg.register(req).await.unwrap();
        assert!(
            info.grant_types
                .contains(&"urn:ietf:params:oauth:grant-type:device_code".to_string()),
            "device code grant should be allowed"
        );
    }
}
