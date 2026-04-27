//! [`AuthProvider`] implementations.
//!
//! Provides [`PasswordAuthProvider`] for username/password authentication
//! and [`OidcAuthProvider`] for federated login via an external IdP.
//! Both issue the assistant's own JWTs regardless of the upstream identity
//! source.

use std::sync::Arc;

use anyhow::{Result, bail};
use async_trait::async_trait;
use tracing::debug;

use assistant_core::auth::{
    AuthContext, AuthProvider, AuthorizeRequest, AuthorizeResponse, ClientInfo, ClientRegistration,
    TokenGrant, TokenResponse,
};

use crate::api_keys::{self, ApiKeyStore};
use crate::jwt::JwtManager;
use crate::oauth2::clients::ClientRegistrar;
use crate::oauth2::device::DeviceCodeManager;
use crate::oauth2::server::OAuth2Server;
use crate::oidc::OidcProvider;
use crate::password;

// -- User store trait for password auth --

/// Trait for looking up users by email and verifying passwords.
#[async_trait]
pub trait PasswordUserStore: Send + Sync {
    /// Find a user by email and return (user_id, password_hash, org_slug, display_name).
    async fn find_by_email(&self, email: &str) -> Result<Option<PasswordUserRecord>>;

    /// Build an AuthContext for the given user (populates space roles, scopes, etc.).
    async fn build_context(&self, user_id: &str) -> Result<AuthContext>;
}

/// A user record with password hash for verification.
#[derive(Clone, Debug)]
pub struct PasswordUserRecord {
    pub user_id: String,
    pub password_hash: String,
    pub org_slug: String,
    pub display_name: String,
}

// -- PasswordAuthProvider --

/// Authenticates users with email + password, issues JWTs via the OAuth2 server.
pub struct PasswordAuthProvider {
    jwt_manager: Arc<JwtManager>,
    oauth2_server: Arc<OAuth2Server>,
    device_manager: Arc<DeviceCodeManager>,
    client_registrar: Arc<ClientRegistrar>,
    api_key_store: Arc<dyn ApiKeyStore>,
    user_store: Arc<dyn PasswordUserStore>,
}

impl PasswordAuthProvider {
    pub fn new(
        jwt_manager: Arc<JwtManager>,
        oauth2_server: Arc<OAuth2Server>,
        device_manager: Arc<DeviceCodeManager>,
        client_registrar: Arc<ClientRegistrar>,
        api_key_store: Arc<dyn ApiKeyStore>,
        user_store: Arc<dyn PasswordUserStore>,
    ) -> Self {
        Self {
            jwt_manager,
            oauth2_server,
            device_manager,
            client_registrar,
            api_key_store,
            user_store,
        }
    }
}

#[async_trait]
impl AuthProvider for PasswordAuthProvider {
    async fn authenticate(&self, token: &str) -> Result<AuthContext> {
        // Try JWT first.
        if let Ok(ctx) = self.jwt_manager.validate_to_context(token) {
            return Ok(ctx);
        }

        // Try API key.
        if token.starts_with("ask_live_") {
            return api_keys::resolve_key(token, self.api_key_store.as_ref()).await;
        }

        bail!("invalid token: not a valid JWT or API key")
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse> {
        // Verify the client exists.
        let _client = self
            .client_registrar
            .get(&req.client_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("unknown client_id: {}", req.client_id))?;

        // Password mode: render a login form.
        Ok(AuthorizeResponse::LoginRequired {
            client_id: req.client_id,
            state: req.state,
        })
    }

    async fn token(&self, grant: TokenGrant) -> Result<TokenResponse> {
        match grant {
            TokenGrant::AuthorizationCode {
                code,
                redirect_uri,
                client_id,
                code_verifier,
            } => {
                let pair = self
                    .oauth2_server
                    .exchange_auth_code(&code, &client_id, &redirect_uri, code_verifier.as_deref())
                    .await?;

                let ctx = self.user_store.build_context(&pair.user_id).await?;
                let record = self
                    .user_store
                    .find_by_email(&ctx.email)
                    .await?
                    .unwrap_or_else(|| PasswordUserRecord {
                        user_id: pair.user_id.clone(),
                        password_hash: String::new(),
                        org_slug: "default".into(),
                        display_name: "User".into(),
                    });

                let access_token =
                    self.jwt_manager
                        .sign(&ctx, &record.org_slug, &record.display_name)?;

                Ok(TokenResponse {
                    access_token,
                    token_type: "Bearer".into(),
                    expires_in: 3600,
                    refresh_token: Some(pair.refresh_token),
                    scope: None,
                })
            }

            TokenGrant::RefreshToken {
                refresh_token,
                client_id,
            } => {
                let pair = self
                    .oauth2_server
                    .refresh(&refresh_token, &client_id)
                    .await?;

                let ctx = self.user_store.build_context(&pair.user_id).await?;
                let record = self
                    .user_store
                    .find_by_email(&ctx.email)
                    .await?
                    .unwrap_or_else(|| PasswordUserRecord {
                        user_id: pair.user_id.clone(),
                        password_hash: String::new(),
                        org_slug: "default".into(),
                        display_name: "User".into(),
                    });

                let access_token =
                    self.jwt_manager
                        .sign(&ctx, &record.org_slug, &record.display_name)?;

                Ok(TokenResponse {
                    access_token,
                    token_type: "Bearer".into(),
                    expires_in: 3600,
                    refresh_token: Some(pair.refresh_token),
                    scope: None,
                })
            }

            TokenGrant::DeviceCode {
                device_code,
                client_id: _,
            } => {
                let poll_result = self.device_manager.poll(&device_code).await?;
                match poll_result {
                    crate::oauth2::device::PollResult::Authorized { user_id, scopes: _ } => {
                        let ctx = self.user_store.build_context(&user_id).await?;
                        let record = self
                            .user_store
                            .find_by_email(&ctx.email)
                            .await?
                            .unwrap_or_else(|| PasswordUserRecord {
                                user_id: user_id.clone(),
                                password_hash: String::new(),
                                org_slug: "default".into(),
                                display_name: "User".into(),
                            });

                        let access_token =
                            self.jwt_manager
                                .sign(&ctx, &record.org_slug, &record.display_name)?;

                        Ok(TokenResponse {
                            access_token,
                            token_type: "Bearer".into(),
                            expires_in: 3600,
                            refresh_token: None,
                            scope: None,
                        })
                    }
                    crate::oauth2::device::PollResult::Pending => {
                        bail!("authorization_pending")
                    }
                    crate::oauth2::device::PollResult::Expired => {
                        bail!("expired_token")
                    }
                }
            }

            TokenGrant::ClientCredentials { .. } => {
                bail!("client_credentials grant not supported in password mode")
            }
        }
    }

    async fn register_client(&self, req: ClientRegistration) -> Result<ClientInfo> {
        self.client_registrar.register(req).await
    }
}

// -- OidcAuthProvider --

/// Authenticates users via an external OIDC IdP, issues our own JWTs.
pub struct OidcAuthProvider {
    jwt_manager: Arc<JwtManager>,
    oauth2_server: Arc<OAuth2Server>,
    device_manager: Arc<DeviceCodeManager>,
    client_registrar: Arc<ClientRegistrar>,
    api_key_store: Arc<dyn ApiKeyStore>,
    oidc_provider: Arc<OidcProvider>,
    user_store: Arc<dyn PasswordUserStore>,
}

impl OidcAuthProvider {
    pub fn new(
        jwt_manager: Arc<JwtManager>,
        oauth2_server: Arc<OAuth2Server>,
        device_manager: Arc<DeviceCodeManager>,
        client_registrar: Arc<ClientRegistrar>,
        api_key_store: Arc<dyn ApiKeyStore>,
        oidc_provider: Arc<OidcProvider>,
        user_store: Arc<dyn PasswordUserStore>,
    ) -> Self {
        Self {
            jwt_manager,
            oauth2_server,
            device_manager,
            client_registrar,
            api_key_store,
            oidc_provider,
            user_store,
        }
    }
}

#[async_trait]
impl AuthProvider for OidcAuthProvider {
    async fn authenticate(&self, token: &str) -> Result<AuthContext> {
        // Same as password provider — both issue our JWTs.
        if let Ok(ctx) = self.jwt_manager.validate_to_context(token) {
            return Ok(ctx);
        }

        if token.starts_with("ask_live_") {
            return api_keys::resolve_key(token, self.api_key_store.as_ref()).await;
        }

        bail!("invalid token: not a valid JWT or API key")
    }

    async fn authorize(&self, req: AuthorizeRequest) -> Result<AuthorizeResponse> {
        let _client = self
            .client_registrar
            .get(&req.client_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("unknown client_id: {}", req.client_id))?;

        // OIDC mode: redirect to the external IdP.
        let discovery = self.oidc_provider.discovery();
        let redirect_url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}",
            discovery.authorization_endpoint,
            urlencoding::encode(self.oidc_provider.client_id()),
            urlencoding::encode(&req.redirect_uri),
            urlencoding::encode(req.state.as_deref().unwrap_or("")),
        );

        debug!(url = %redirect_url, "redirecting to OIDC IdP");

        Ok(AuthorizeResponse::Redirect { url: redirect_url })
    }

    async fn token(&self, grant: TokenGrant) -> Result<TokenResponse> {
        match grant {
            TokenGrant::AuthorizationCode {
                code,
                redirect_uri,
                client_id,
                code_verifier,
            } => {
                let pair = self
                    .oauth2_server
                    .exchange_auth_code(&code, &client_id, &redirect_uri, code_verifier.as_deref())
                    .await?;

                let ctx = self.user_store.build_context(&pair.user_id).await?;
                let record = self
                    .user_store
                    .find_by_email(&ctx.email)
                    .await?
                    .unwrap_or_else(|| PasswordUserRecord {
                        user_id: pair.user_id.clone(),
                        password_hash: String::new(),
                        org_slug: "default".into(),
                        display_name: "User".into(),
                    });

                let access_token =
                    self.jwt_manager
                        .sign(&ctx, &record.org_slug, &record.display_name)?;

                Ok(TokenResponse {
                    access_token,
                    token_type: "Bearer".into(),
                    expires_in: 3600,
                    refresh_token: Some(pair.refresh_token),
                    scope: None,
                })
            }

            TokenGrant::RefreshToken {
                refresh_token,
                client_id,
            } => {
                let pair = self
                    .oauth2_server
                    .refresh(&refresh_token, &client_id)
                    .await?;

                let ctx = self.user_store.build_context(&pair.user_id).await?;
                let record = self
                    .user_store
                    .find_by_email(&ctx.email)
                    .await?
                    .unwrap_or_else(|| PasswordUserRecord {
                        user_id: pair.user_id.clone(),
                        password_hash: String::new(),
                        org_slug: "default".into(),
                        display_name: "User".into(),
                    });

                let access_token =
                    self.jwt_manager
                        .sign(&ctx, &record.org_slug, &record.display_name)?;

                Ok(TokenResponse {
                    access_token,
                    token_type: "Bearer".into(),
                    expires_in: 3600,
                    refresh_token: Some(pair.refresh_token),
                    scope: None,
                })
            }

            TokenGrant::DeviceCode {
                device_code,
                client_id: _,
            } => {
                let poll_result = self.device_manager.poll(&device_code).await?;
                match poll_result {
                    crate::oauth2::device::PollResult::Authorized { user_id, scopes: _ } => {
                        let ctx = self.user_store.build_context(&user_id).await?;
                        let record = self
                            .user_store
                            .find_by_email(&ctx.email)
                            .await?
                            .unwrap_or_else(|| PasswordUserRecord {
                                user_id: user_id.clone(),
                                password_hash: String::new(),
                                org_slug: "default".into(),
                                display_name: "User".into(),
                            });

                        let access_token =
                            self.jwt_manager
                                .sign(&ctx, &record.org_slug, &record.display_name)?;

                        Ok(TokenResponse {
                            access_token,
                            token_type: "Bearer".into(),
                            expires_in: 3600,
                            refresh_token: None,
                            scope: None,
                        })
                    }
                    crate::oauth2::device::PollResult::Pending => {
                        bail!("authorization_pending")
                    }
                    crate::oauth2::device::PollResult::Expired => {
                        bail!("expired_token")
                    }
                }
            }

            TokenGrant::ClientCredentials { .. } => {
                bail!("client_credentials grant not supported in OIDC mode")
            }
        }
    }

    async fn register_client(&self, req: ClientRegistration) -> Result<ClientInfo> {
        self.client_registrar.register(req).await
    }
}

/// Helper to validate a password login and generate an auth code.
///
/// Used by the web-ui POST /oauth/authorize endpoint after the user
/// submits their email + password.
pub async fn validate_password_login(
    email: &str,
    plain_password: &str,
    client_id: &str,
    redirect_uri: &str,
    pkce_challenge: Option<&str>,
    user_store: &dyn PasswordUserStore,
    oauth2_server: &OAuth2Server,
) -> Result<String> {
    let record = user_store
        .find_by_email(email)
        .await?
        .ok_or_else(|| anyhow::anyhow!("invalid email or password"))?;

    let valid = password::verify_password(plain_password, &record.password_hash)?;
    if !valid {
        bail!("invalid email or password");
    }

    debug!(user_id = %record.user_id, "password login successful");

    let code = oauth2_server
        .generate_auth_code(
            &record.user_id,
            client_id,
            redirect_uri,
            pkce_challenge,
            vec![], // Full scopes resolved at token exchange from user's roles.
        )
        .await?;

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    use assistant_core::identity::{Action, OrgId, ResourceKind, Role, Scope, SpaceId, UserId};

    use crate::api_keys::InMemoryApiKeyStore;
    use crate::jwt::JwtKeyPair;
    use crate::oauth2::clients::{ClientRegistrar, InMemoryClientStore};
    use crate::oauth2::device::{DeviceCodeManager, InMemoryDeviceCodeStore};
    use crate::oauth2::server::{InMemoryAuthCodeStore, InMemoryRefreshTokenStore, OAuth2Server};

    // -- In-memory password user store for tests --

    struct TestUserStore {
        users: Vec<PasswordUserRecord>,
        contexts: HashMap<String, AuthContext>,
    }

    impl TestUserStore {
        fn new() -> Self {
            let password_hash = password::hash_password("secret123").unwrap();
            let user_id = "usr_alice".to_string();

            let mut space_roles = HashMap::new();
            space_roles.insert(SpaceId::from("eng"), Role::Member);

            let ctx = AuthContext {
                user_id: UserId::from("usr_alice"),
                org_id: OrgId::from("acme"),
                email: "alice@acme.com".into(),
                space_roles,
                scopes: vec![
                    Scope::new(ResourceKind::Personas, Action::Read),
                    Scope::new(ResourceKind::Conversations, Action::Read),
                    Scope::new(ResourceKind::Conversations, Action::Write),
                ],
                client_id: "webui".into(),
            };

            let mut contexts = HashMap::new();
            contexts.insert(user_id.clone(), ctx);

            Self {
                users: vec![PasswordUserRecord {
                    user_id,
                    password_hash,
                    org_slug: "acme".into(),
                    display_name: "Alice".into(),
                }],
                contexts,
            }
        }
    }

    #[async_trait]
    impl PasswordUserStore for TestUserStore {
        async fn find_by_email(&self, email: &str) -> Result<Option<PasswordUserRecord>> {
            // Simplistic: map alice@acme.com → usr_alice.
            if email == "alice@acme.com" {
                Ok(self.users.first().cloned())
            } else {
                Ok(None)
            }
        }

        async fn build_context(&self, user_id: &str) -> Result<AuthContext> {
            self.contexts
                .get(user_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("user not found: {user_id}"))
        }
    }

    #[allow(clippy::type_complexity)]
    fn make_test_infra() -> (
        Arc<JwtManager>,
        Arc<OAuth2Server>,
        Arc<DeviceCodeManager>,
        Arc<ClientRegistrar>,
        Arc<InMemoryApiKeyStore>,
        Arc<TestUserStore>,
    ) {
        let secret = b"test-secret-for-jwt-signing-32b!";
        let kp = JwtKeyPair::from_secret(secret);
        let jwt_manager = Arc::new(JwtManager::new(
            kp,
            "https://test.local".into(),
            "https://test.local".into(),
        ));

        let oauth2_server = Arc::new(OAuth2Server::new(
            Arc::new(InMemoryAuthCodeStore::default()),
            Arc::new(InMemoryRefreshTokenStore::default()),
        ));

        let device_manager = Arc::new(DeviceCodeManager::new(
            Arc::new(InMemoryDeviceCodeStore::default()),
            "https://test.local/device".into(),
        ));

        let client_registrar = Arc::new(ClientRegistrar::new(Arc::new(
            InMemoryClientStore::default(),
        )));

        let api_key_store = Arc::new(InMemoryApiKeyStore::new());
        let user_store = Arc::new(TestUserStore::new());

        (
            jwt_manager,
            oauth2_server,
            device_manager,
            client_registrar,
            api_key_store,
            user_store,
        )
    }

    #[tokio::test]
    async fn password_provider_authenticate_jwt() {
        let (
            jwt_manager,
            oauth2_server,
            device_manager,
            client_registrar,
            api_key_store,
            user_store,
        ) = make_test_infra();

        let provider = PasswordAuthProvider::new(
            jwt_manager.clone(),
            oauth2_server,
            device_manager,
            client_registrar,
            api_key_store,
            user_store.clone(),
        );

        // Sign a token and authenticate.
        let ctx = user_store.build_context("usr_alice").await.unwrap();
        let token = jwt_manager.sign(&ctx, "acme", "Alice").unwrap();

        let result = provider.authenticate(&token).await.unwrap();
        assert_eq!(result.user_id, UserId::from("usr_alice"));
    }

    #[tokio::test]
    async fn password_provider_authenticate_api_key() {
        let (
            jwt_manager,
            oauth2_server,
            device_manager,
            client_registrar,
            api_key_store,
            user_store,
        ) = make_test_infra();

        // Store an API key.
        let key = api_keys::generate_key();
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::Member);
        let record = api_keys::ApiKeyRecord {
            id: "key_1".into(),
            user_id: UserId::from("usr_alice"),
            name: "Test".into(),
            key_hash: key.key_hash.clone(),
            key_prefix: key.key_prefix.clone(),
            org_id: OrgId::from("acme"),
            space_roles,
            scopes: vec![Scope::new(ResourceKind::Personas, Action::Read)],
            expires_at: None,
            created_at: chrono::Utc::now(),
        };
        api_key_store.store_key(record).await.unwrap();

        let provider = PasswordAuthProvider::new(
            jwt_manager,
            oauth2_server,
            device_manager,
            client_registrar,
            api_key_store,
            user_store,
        );

        let result = provider.authenticate(&key.plaintext).await.unwrap();
        assert_eq!(result.user_id, UserId::from("usr_alice"));
        assert_eq!(result.client_id, "api_key");
    }

    #[tokio::test]
    async fn password_provider_authenticate_invalid() {
        let (
            jwt_manager,
            oauth2_server,
            device_manager,
            client_registrar,
            api_key_store,
            user_store,
        ) = make_test_infra();

        let provider = PasswordAuthProvider::new(
            jwt_manager,
            oauth2_server,
            device_manager,
            client_registrar,
            api_key_store,
            user_store,
        );

        let err = provider.authenticate("invalid-token").await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn password_provider_authorize_returns_login_required() {
        let (
            jwt_manager,
            oauth2_server,
            device_manager,
            client_registrar,
            api_key_store,
            user_store,
        ) = make_test_infra();

        // Register a client first.
        use assistant_core::auth::ClientRegistration;
        client_registrar
            .register(ClientRegistration {
                client_name: "Test App".into(),
                redirect_uris: vec!["http://localhost:4040/callback".into()],
                grant_types: vec!["authorization_code".into()],
                response_types: Some(vec!["code".into()]),
                token_endpoint_auth_method: None,
            })
            .await
            .unwrap();

        let clients = client_registrar.list().await.unwrap();
        let client_id = clients[0].client_id.clone();

        let provider = PasswordAuthProvider::new(
            jwt_manager,
            oauth2_server,
            device_manager,
            client_registrar,
            api_key_store,
            user_store,
        );

        let req = AuthorizeRequest {
            client_id,
            redirect_uri: "http://localhost:4040/callback".into(),
            response_type: "code".into(),
            scope: None,
            state: Some("test-state".into()),
            code_challenge: None,
            code_challenge_method: None,
        };

        let resp = provider.authorize(req).await.unwrap();
        match resp {
            AuthorizeResponse::LoginRequired { state, .. } => {
                assert_eq!(state.as_deref(), Some("test-state"));
            }
            other => panic!("expected LoginRequired, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn validate_password_login_success() {
        let (_, oauth2_server, _, client_registrar, _, user_store) = make_test_infra();

        // Register a client.
        use assistant_core::auth::ClientRegistration;
        client_registrar
            .register(ClientRegistration {
                client_name: "Test App".into(),
                redirect_uris: vec!["http://localhost:4040/callback".into()],
                grant_types: vec!["authorization_code".into()],
                response_types: Some(vec!["code".into()]),
                token_endpoint_auth_method: None,
            })
            .await
            .unwrap();

        let clients = client_registrar.list().await.unwrap();
        let client_id = &clients[0].client_id;

        let code = validate_password_login(
            "alice@acme.com",
            "secret123",
            client_id,
            "http://localhost:4040/callback",
            None,
            user_store.as_ref(),
            &oauth2_server,
        )
        .await
        .unwrap();

        assert!(!code.is_empty(), "auth code should be non-empty");
    }

    #[tokio::test]
    async fn validate_password_login_wrong_password() {
        let (_, oauth2_server, _, _, _, user_store) = make_test_infra();

        let err = validate_password_login(
            "alice@acme.com",
            "wrong-password",
            "client-1",
            "http://localhost/cb",
            None,
            user_store.as_ref(),
            &oauth2_server,
        )
        .await;

        assert!(err.is_err());
        let msg = format!("{:?}", err.unwrap_err());
        assert!(msg.contains("invalid email or password"));
    }

    #[tokio::test]
    async fn validate_password_login_unknown_email() {
        let (_, oauth2_server, _, _, _, user_store) = make_test_infra();

        let err = validate_password_login(
            "unknown@acme.com",
            "secret123",
            "client-1",
            "http://localhost/cb",
            None,
            user_store.as_ref(),
            &oauth2_server,
        )
        .await;

        assert!(err.is_err());
    }
}
