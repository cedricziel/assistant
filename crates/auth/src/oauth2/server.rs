//! Authorization code grant and refresh token management.
//!
//! Implements the core OAuth2 authorization code flow with PKCE support
//! and refresh token rotation.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, bail};
use chrono::{DateTime, Duration, Utc};
use tokio::sync::RwLock;

use super::pkce;

// -- Types --

/// A pending authorization code waiting to be exchanged for tokens.
#[derive(Clone, Debug)]
pub struct AuthCode {
    pub code: String,
    pub user_id: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub pkce_challenge: Option<String>,
    pub expires_at: DateTime<Utc>,
}

/// A stored refresh token.
#[derive(Clone, Debug)]
pub struct StoredRefreshToken {
    pub token: String,
    pub user_id: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub expires_at: DateTime<Utc>,
    /// Whether this token has been consumed (for rotation detection).
    pub consumed: bool,
}

/// The result of exchanging an auth code or refreshing tokens.
#[derive(Clone, Debug)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    /// The user who authorized the token.
    pub user_id: String,
}

// -- AuthCodeStore trait --

/// Storage backend for authorization codes.
#[async_trait::async_trait]
pub trait AuthCodeStore: Send + Sync {
    /// Store a new authorization code.
    async fn store_code(&self, code: AuthCode) -> Result<()>;

    /// Look up and consume an authorization code (single-use).
    async fn take_code(&self, code: &str) -> Result<Option<AuthCode>>;
}

/// Storage backend for refresh tokens.
#[async_trait::async_trait]
pub trait RefreshTokenStore: Send + Sync {
    /// Store a new refresh token.
    async fn store_token(&self, token: StoredRefreshToken) -> Result<()>;

    /// Look up a refresh token without consuming it.
    async fn get_token(&self, token: &str) -> Result<Option<StoredRefreshToken>>;

    /// Mark a refresh token as consumed (for rotation).
    async fn consume_token(&self, token: &str) -> Result<()>;

    /// Revoke all refresh tokens for a user+client pair (security measure
    /// when replay is detected).
    async fn revoke_all(&self, user_id: &str, client_id: &str) -> Result<()>;

    /// Revoke all refresh tokens belonging to a user except the specified
    /// token. Used after a self-service password change to log the user out
    /// of every other session while keeping the current one alive. Returns
    /// the number of tokens revoked.
    async fn revoke_for_user_except(&self, user_id: &str, except_token: &str) -> Result<u64>;
}

// -- In-memory implementations --

/// In-memory auth code store for testing and single-server deployments.
#[derive(Clone, Default)]
pub struct InMemoryAuthCodeStore {
    codes: Arc<RwLock<HashMap<String, AuthCode>>>,
}

#[async_trait::async_trait]
impl AuthCodeStore for InMemoryAuthCodeStore {
    async fn store_code(&self, code: AuthCode) -> Result<()> {
        self.codes.write().await.insert(code.code.clone(), code);
        Ok(())
    }

    async fn take_code(&self, code: &str) -> Result<Option<AuthCode>> {
        Ok(self.codes.write().await.remove(code))
    }
}

/// In-memory refresh token store for testing and single-server deployments.
#[derive(Clone, Default)]
pub struct InMemoryRefreshTokenStore {
    tokens: Arc<RwLock<HashMap<String, StoredRefreshToken>>>,
}

#[async_trait::async_trait]
impl RefreshTokenStore for InMemoryRefreshTokenStore {
    async fn store_token(&self, token: StoredRefreshToken) -> Result<()> {
        self.tokens.write().await.insert(token.token.clone(), token);
        Ok(())
    }

    async fn get_token(&self, token: &str) -> Result<Option<StoredRefreshToken>> {
        Ok(self.tokens.read().await.get(token).cloned())
    }

    async fn consume_token(&self, token: &str) -> Result<()> {
        if let Some(t) = self.tokens.write().await.get_mut(token) {
            t.consumed = true;
        }
        Ok(())
    }

    async fn revoke_all(&self, user_id: &str, client_id: &str) -> Result<()> {
        self.tokens
            .write()
            .await
            .retain(|_, t| !(t.user_id == user_id && t.client_id == client_id));
        Ok(())
    }

    async fn revoke_for_user_except(&self, user_id: &str, except_token: &str) -> Result<u64> {
        let mut tokens = self.tokens.write().await;
        let before = tokens.len();
        tokens.retain(|tok, t| !(t.user_id == user_id && tok != except_token));
        Ok((before - tokens.len()) as u64)
    }
}

// -- OAuth2 Server --

/// The OAuth2 authorization server managing auth codes and refresh tokens.
pub struct OAuth2Server {
    code_store: Arc<dyn AuthCodeStore>,
    refresh_store: Arc<dyn RefreshTokenStore>,
    /// How long an authorization code is valid.
    code_ttl: Duration,
    /// How long a refresh token is valid.
    refresh_ttl: Duration,
}

impl OAuth2Server {
    /// Create a new OAuth2 server with the given stores.
    pub fn new(
        code_store: Arc<dyn AuthCodeStore>,
        refresh_store: Arc<dyn RefreshTokenStore>,
    ) -> Self {
        Self {
            code_store,
            refresh_store,
            code_ttl: Duration::minutes(10),
            refresh_ttl: Duration::days(30),
        }
    }

    /// Override the authorization code TTL.
    pub fn with_code_ttl(mut self, ttl: Duration) -> Self {
        self.code_ttl = ttl;
        self
    }

    /// Override the refresh token TTL.
    pub fn with_refresh_ttl(mut self, ttl: Duration) -> Self {
        self.refresh_ttl = ttl;
        self
    }

    /// Generate and store a new authorization code.
    pub async fn generate_auth_code(
        &self,
        user_id: &str,
        client_id: &str,
        redirect_uri: &str,
        pkce_challenge: Option<&str>,
        scopes: Vec<String>,
    ) -> Result<String> {
        let code = generate_random_token();
        let auth_code = AuthCode {
            code: code.clone(),
            user_id: user_id.to_owned(),
            client_id: client_id.to_owned(),
            redirect_uri: redirect_uri.to_owned(),
            scopes,
            pkce_challenge: pkce_challenge.map(|s| s.to_owned()),
            expires_at: Utc::now() + self.code_ttl,
        };
        self.code_store.store_code(auth_code).await?;
        Ok(code)
    }

    /// Exchange an authorization code for an access/refresh token pair.
    ///
    /// The code is single-use and is consumed on success.
    pub async fn exchange_auth_code(
        &self,
        code: &str,
        client_id: &str,
        redirect_uri: &str,
        pkce_verifier: Option<&str>,
    ) -> Result<TokenPair> {
        let auth_code = self
            .code_store
            .take_code(code)
            .await?
            .ok_or_else(|| anyhow::anyhow!("invalid or expired authorization code"))?;

        // Validate client_id.
        if auth_code.client_id != client_id {
            bail!("client_id mismatch");
        }

        // Validate redirect_uri.
        if auth_code.redirect_uri != redirect_uri {
            bail!("redirect_uri mismatch");
        }

        // Validate expiry.
        if Utc::now() > auth_code.expires_at {
            bail!("authorization code expired");
        }

        // Validate PKCE — mandatory per OAuth 2.1 §4.1.3.
        match (&auth_code.pkce_challenge, pkce_verifier) {
            (Some(challenge), Some(verifier)) => {
                if !pkce::verify(verifier, challenge) {
                    bail!("PKCE verification failed");
                }
            }
            (Some(_), None) => bail!("PKCE verifier required but not provided"),
            (None, Some(_)) => bail!("PKCE verifier provided but no challenge was stored"),
            (None, None) => bail!("PKCE code_challenge is required"),
        }

        // Generate tokens.
        let access_token = generate_random_token();
        let refresh_token = generate_random_token();
        let user_id = auth_code.user_id.clone();

        // Store refresh token.
        self.refresh_store
            .store_token(StoredRefreshToken {
                token: refresh_token.clone(),
                user_id: auth_code.user_id,
                client_id: auth_code.client_id,
                scopes: auth_code.scopes,
                expires_at: Utc::now() + self.refresh_ttl,
                consumed: false,
            })
            .await?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            user_id,
        })
    }

    /// Refresh an access token using a refresh token.
    ///
    /// Implements refresh token rotation: the old token is consumed and a new
    /// pair is issued. If a consumed token is reused (replay attack), all
    /// tokens for that user+client are revoked.
    pub async fn refresh(&self, refresh_token: &str, client_id: &str) -> Result<TokenPair> {
        let stored = self
            .refresh_store
            .get_token(refresh_token)
            .await?
            .ok_or_else(|| anyhow::anyhow!("invalid refresh token"))?;

        // Replay detection: if token was already consumed, revoke everything.
        if stored.consumed {
            self.refresh_store
                .revoke_all(&stored.user_id, &stored.client_id)
                .await?;
            bail!("refresh token replay detected — all tokens revoked");
        }

        // Validate client_id.
        if stored.client_id != client_id {
            bail!("client_id mismatch on refresh");
        }

        // Validate expiry.
        if Utc::now() > stored.expires_at {
            bail!("refresh token expired");
        }

        // Consume old token.
        self.refresh_store.consume_token(refresh_token).await?;

        // Issue new pair.
        let new_access = generate_random_token();
        let new_refresh = generate_random_token();
        let user_id = stored.user_id.clone();

        self.refresh_store
            .store_token(StoredRefreshToken {
                token: new_refresh.clone(),
                user_id: stored.user_id,
                client_id: stored.client_id,
                scopes: stored.scopes,
                expires_at: Utc::now() + self.refresh_ttl,
                consumed: false,
            })
            .await?;

        Ok(TokenPair {
            access_token: new_access,
            refresh_token: new_refresh,
            user_id,
        })
    }

    /// Issue a fresh access/refresh token pair for a user and client.
    ///
    /// Used by the device code flow where there is no authorization code to
    /// exchange but the user has already been authenticated via the
    /// verification step.
    pub async fn issue_token_pair(
        &self,
        user_id: &str,
        client_id: &str,
        scopes: Vec<String>,
    ) -> Result<TokenPair> {
        let access_token = generate_random_token();
        let refresh_token = generate_random_token();

        self.refresh_store
            .store_token(StoredRefreshToken {
                token: refresh_token.clone(),
                user_id: user_id.to_string(),
                client_id: client_id.to_string(),
                scopes,
                expires_at: Utc::now() + self.refresh_ttl,
                consumed: false,
            })
            .await?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            user_id: user_id.to_string(),
        })
    }
}

/// Generate a URL-safe random token (32 bytes → 43 base64url chars).
fn generate_random_token() -> String {
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

    fn make_server() -> OAuth2Server {
        OAuth2Server::new(
            Arc::new(InMemoryAuthCodeStore::default()),
            Arc::new(InMemoryRefreshTokenStore::default()),
        )
    }

    /// Generate a PKCE pair for tests that don't specifically test PKCE behaviour.
    fn test_pkce() -> (String, String) {
        let verifier = pkce::generate_verifier();
        let challenge = pkce::challenge_from_verifier(&verifier);
        (verifier, challenge)
    }

    #[tokio::test]
    async fn auth_code_exchange_with_pkce() {
        let server = make_server();
        let verifier = pkce::generate_verifier();
        let challenge = pkce::challenge_from_verifier(&verifier);

        let code = server
            .generate_auth_code(
                "usr_alice",
                "webui",
                "http://localhost/callback",
                Some(&challenge),
                vec!["personas:read".into()],
            )
            .await
            .unwrap();

        let pair = server
            .exchange_auth_code(&code, "webui", "http://localhost/callback", Some(&verifier))
            .await
            .unwrap();

        assert!(!pair.access_token.is_empty(), "should get access token");
        assert!(!pair.refresh_token.is_empty(), "should get refresh token");
    }

    #[tokio::test]
    async fn auth_code_is_single_use() {
        let server = make_server();
        let (verifier, challenge) = test_pkce();

        let code = server
            .generate_auth_code(
                "usr_alice",
                "webui",
                "http://localhost/cb",
                Some(&challenge),
                vec![],
            )
            .await
            .unwrap();

        // First exchange succeeds.
        server
            .exchange_auth_code(&code, "webui", "http://localhost/cb", Some(&verifier))
            .await
            .unwrap();

        // Second exchange fails.
        let err = server
            .exchange_auth_code(&code, "webui", "http://localhost/cb", Some(&verifier))
            .await;
        assert!(err.is_err(), "auth code should be single-use");
    }

    #[tokio::test]
    async fn wrong_pkce_verifier_is_rejected() {
        let server = make_server();
        let verifier = pkce::generate_verifier();
        let challenge = pkce::challenge_from_verifier(&verifier);

        let code = server
            .generate_auth_code(
                "usr_alice",
                "webui",
                "http://localhost/cb",
                Some(&challenge),
                vec![],
            )
            .await
            .unwrap();

        let wrong_verifier = pkce::generate_verifier();
        let err = server
            .exchange_auth_code(&code, "webui", "http://localhost/cb", Some(&wrong_verifier))
            .await;
        assert!(err.is_err(), "wrong PKCE verifier should be rejected");
    }

    #[tokio::test]
    async fn missing_pkce_verifier_is_rejected() {
        let server = make_server();
        let verifier = pkce::generate_verifier();
        let challenge = pkce::challenge_from_verifier(&verifier);

        let code = server
            .generate_auth_code(
                "usr_alice",
                "webui",
                "http://localhost/cb",
                Some(&challenge),
                vec![],
            )
            .await
            .unwrap();

        let err = server
            .exchange_auth_code(&code, "webui", "http://localhost/cb", None)
            .await;
        assert!(err.is_err(), "missing PKCE verifier should be rejected");
    }

    #[tokio::test]
    async fn wrong_client_id_is_rejected() {
        let server = make_server();
        let (verifier, challenge) = test_pkce();

        let code = server
            .generate_auth_code(
                "usr_alice",
                "webui",
                "http://localhost/cb",
                Some(&challenge),
                vec![],
            )
            .await
            .unwrap();

        let err = server
            .exchange_auth_code(
                &code,
                "wrong_client",
                "http://localhost/cb",
                Some(&verifier),
            )
            .await;
        assert!(err.is_err(), "wrong client_id should be rejected");
    }

    #[tokio::test]
    async fn wrong_redirect_uri_is_rejected() {
        let server = make_server();
        let (verifier, challenge) = test_pkce();

        let code = server
            .generate_auth_code(
                "usr_alice",
                "webui",
                "http://localhost/cb",
                Some(&challenge),
                vec![],
            )
            .await
            .unwrap();

        let err = server
            .exchange_auth_code(&code, "webui", "http://evil.com/cb", Some(&verifier))
            .await;
        assert!(err.is_err(), "wrong redirect_uri should be rejected");
    }

    #[tokio::test]
    async fn refresh_token_rotation() {
        let server = make_server();
        let (verifier, challenge) = test_pkce();

        let code = server
            .generate_auth_code(
                "usr_alice",
                "webui",
                "http://localhost/cb",
                Some(&challenge),
                vec![],
            )
            .await
            .unwrap();

        let pair1 = server
            .exchange_auth_code(&code, "webui", "http://localhost/cb", Some(&verifier))
            .await
            .unwrap();

        // Refresh yields new tokens.
        let pair2 = server.refresh(&pair1.refresh_token, "webui").await.unwrap();
        assert_ne!(
            pair1.refresh_token, pair2.refresh_token,
            "refresh should rotate tokens"
        );

        // Old refresh token cannot be reused.
        let err = server.refresh(&pair1.refresh_token, "webui").await;
        assert!(err.is_err(), "reusing consumed refresh token should fail");
    }

    #[tokio::test]
    async fn refresh_replay_revokes_all() {
        let server = make_server();
        let (verifier, challenge) = test_pkce();

        let code = server
            .generate_auth_code(
                "usr_alice",
                "webui",
                "http://localhost/cb",
                Some(&challenge),
                vec![],
            )
            .await
            .unwrap();

        let pair1 = server
            .exchange_auth_code(&code, "webui", "http://localhost/cb", Some(&verifier))
            .await
            .unwrap();

        // Normal refresh.
        let pair2 = server.refresh(&pair1.refresh_token, "webui").await.unwrap();

        // Replay attack with old token — triggers revocation.
        let err = server.refresh(&pair1.refresh_token, "webui").await;
        assert!(err.is_err(), "replay should be detected");

        // Even the new token is now revoked.
        let err2 = server.refresh(&pair2.refresh_token, "webui").await;
        assert!(err2.is_err(), "all tokens should be revoked after replay");
    }

    #[tokio::test]
    async fn revoke_for_user_except_keeps_calling_token() {
        let store = InMemoryRefreshTokenStore::default();
        let user = "usr_alice";
        let now = Utc::now();

        // Three tokens for the same user across different "sessions".
        let tokens: Vec<String> = (0..3).map(|_| generate_random_token()).collect();
        for (i, t) in tokens.iter().enumerate() {
            store
                .store_token(StoredRefreshToken {
                    token: t.clone(),
                    user_id: user.into(),
                    client_id: format!("client_{i}"),
                    scopes: vec![],
                    expires_at: now + Duration::days(30),
                    consumed: false,
                })
                .await
                .unwrap();
        }

        // Also a token for a different user — must be untouched.
        let other_token = generate_random_token();
        store
            .store_token(StoredRefreshToken {
                token: other_token.clone(),
                user_id: "usr_bob".into(),
                client_id: "client_x".into(),
                scopes: vec![],
                expires_at: now + Duration::days(30),
                consumed: false,
            })
            .await
            .unwrap();

        // Revoke all of alice's except tokens[1] (the "current" session).
        let n = store
            .revoke_for_user_except(user, &tokens[1])
            .await
            .unwrap();
        assert_eq!(n, 2, "should revoke the two non-excepted tokens");

        // Excepted token survives.
        assert!(
            store.get_token(&tokens[1]).await.unwrap().is_some(),
            "excepted token must still validate"
        );
        // The two siblings are gone.
        assert!(
            store.get_token(&tokens[0]).await.unwrap().is_none(),
            "non-excepted token must be revoked"
        );
        assert!(
            store.get_token(&tokens[2]).await.unwrap().is_none(),
            "non-excepted token must be revoked"
        );
        // Other user's token untouched.
        assert!(
            store.get_token(&other_token).await.unwrap().is_some(),
            "other user's token must not be touched"
        );
    }

    #[tokio::test]
    async fn expired_code_is_rejected() {
        let server = make_server().with_code_ttl(Duration::seconds(-10));
        let (_verifier, challenge) = test_pkce();

        let code = server
            .generate_auth_code(
                "usr_alice",
                "webui",
                "http://localhost/cb",
                Some(&challenge),
                vec![],
            )
            .await
            .unwrap();

        // Exchange will fail on expiry before PKCE is checked, but we still
        // supply the challenge to satisfy the mandatory-PKCE invariant.
        let err = server
            .exchange_auth_code(&code, "webui", "http://localhost/cb", Some(&_verifier))
            .await;
        assert!(err.is_err(), "expired code should be rejected");
    }

    #[tokio::test]
    async fn expired_refresh_token_is_rejected() {
        let server = make_server().with_refresh_ttl(Duration::seconds(-10));
        let (verifier, challenge) = test_pkce();

        let code = server
            .generate_auth_code(
                "usr_alice",
                "webui",
                "http://localhost/cb",
                Some(&challenge),
                vec![],
            )
            .await
            .unwrap();

        let pair = server
            .exchange_auth_code(&code, "webui", "http://localhost/cb", Some(&verifier))
            .await
            .unwrap();

        let err = server.refresh(&pair.refresh_token, "webui").await;
        assert!(err.is_err(), "expired refresh token should be rejected");
    }

    #[tokio::test]
    async fn exchange_without_pkce_is_rejected() {
        let server = make_server();

        let code = server
            .generate_auth_code("usr_alice", "webui", "http://localhost/cb", None, vec![])
            .await
            .unwrap();

        let err = server
            .exchange_auth_code(&code, "webui", "http://localhost/cb", None)
            .await;
        assert!(err.is_err(), "exchange without PKCE should be rejected");
    }

    #[tokio::test]
    async fn full_flow_generate_exchange_refresh() {
        let server = make_server();
        let verifier = pkce::generate_verifier();
        let challenge = pkce::challenge_from_verifier(&verifier);

        // 1. Generate auth code.
        let code = server
            .generate_auth_code(
                "usr_alice",
                "webui",
                "http://localhost/callback",
                Some(&challenge),
                vec!["personas:read".into(), "conversations:write".into()],
            )
            .await
            .unwrap();

        // 2. Exchange with PKCE.
        let pair1 = server
            .exchange_auth_code(&code, "webui", "http://localhost/callback", Some(&verifier))
            .await
            .unwrap();

        // 3. Refresh.
        let pair2 = server.refresh(&pair1.refresh_token, "webui").await.unwrap();
        assert_ne!(pair1.access_token, pair2.access_token, "new access token");
        assert_ne!(
            pair1.refresh_token, pair2.refresh_token,
            "new refresh token"
        );

        // 4. Refresh again with new token.
        let pair3 = server.refresh(&pair2.refresh_token, "webui").await.unwrap();
        assert_ne!(pair2.refresh_token, pair3.refresh_token, "rotated again");
    }

    #[tokio::test]
    async fn issue_token_pair_for_device_flow() {
        let server = make_server();

        let pair = server
            .issue_token_pair("usr_bob", "cli", vec!["personas:read".into()])
            .await
            .unwrap();

        assert!(!pair.access_token.is_empty(), "should get access token");
        assert!(!pair.refresh_token.is_empty(), "should get refresh token");
        assert_eq!(pair.user_id, "usr_bob");

        // The refresh token should be usable.
        let pair2 = server.refresh(&pair.refresh_token, "cli").await.unwrap();
        assert_ne!(pair.refresh_token, pair2.refresh_token, "should rotate");
    }
}
