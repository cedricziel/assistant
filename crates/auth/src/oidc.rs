//! OpenID Connect (OIDC) federation.
//!
//! Lightweight OIDC implementation using `reqwest` + `jsonwebtoken` for
//! discovery, ID token validation, and user provisioning. Follows the
//! project's thin-HTTP-client philosophy — no heavy OIDC SDK dependency.

use anyhow::{Context, Result, bail};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use tracing::debug;

use assistant_core::identity::UserId;

// -- Discovery --

/// OpenID Connect discovery document (.well-known/openid-configuration).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OidcDiscovery {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    #[serde(default)]
    pub userinfo_endpoint: Option<String>,
    pub jwks_uri: String,
    #[serde(default)]
    pub response_types_supported: Vec<String>,
    #[serde(default)]
    pub subject_types_supported: Vec<String>,
    #[serde(default)]
    pub id_token_signing_alg_values_supported: Vec<String>,
}

// -- JWKS --

/// A JSON Web Key Set (JWKS) response.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JwksResponse {
    pub keys: Vec<Jwk>,
}

/// A single JSON Web Key.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Jwk {
    /// Key type (e.g. "RSA", "EC").
    pub kty: String,
    /// Key ID.
    #[serde(default)]
    pub kid: Option<String>,
    /// Algorithm (e.g. "RS256").
    #[serde(default)]
    pub alg: Option<String>,
    /// Key use ("sig" for signing).
    #[serde(default, rename = "use")]
    pub key_use: Option<String>,
    // RSA params
    #[serde(default)]
    pub n: Option<String>,
    #[serde(default)]
    pub e: Option<String>,
    // EC params
    #[serde(default)]
    pub crv: Option<String>,
    #[serde(default)]
    pub x: Option<String>,
    #[serde(default)]
    pub y: Option<String>,
}

impl Jwk {
    /// Build a `DecodingKey` from this JWK.
    fn to_decoding_key(&self) -> Result<DecodingKey> {
        match self.kty.as_str() {
            "RSA" => {
                let n = self.n.as_deref().context("RSA JWK missing 'n'")?;
                let e = self.e.as_deref().context("RSA JWK missing 'e'")?;
                DecodingKey::from_rsa_components(n, e)
                    .context("failed to build RSA decoding key from JWK")
            }
            "EC" => {
                let x = self.x.as_deref().context("EC JWK missing 'x'")?;
                let y = self.y.as_deref().context("EC JWK missing 'y'")?;
                DecodingKey::from_ec_components(x, y)
                    .context("failed to build EC decoding key from JWK")
            }
            other => bail!("unsupported JWK key type: {other}"),
        }
    }

    /// Determine the algorithm for this JWK.
    fn algorithm(&self) -> Result<Algorithm> {
        if let Some(alg) = &self.alg {
            return match alg.as_str() {
                "RS256" => Ok(Algorithm::RS256),
                "RS384" => Ok(Algorithm::RS384),
                "RS512" => Ok(Algorithm::RS512),
                "ES256" => Ok(Algorithm::ES256),
                "ES384" => Ok(Algorithm::ES384),
                other => bail!("unsupported JWK algorithm: {other}"),
            };
        }
        // Infer from key type.
        match self.kty.as_str() {
            "RSA" => Ok(Algorithm::RS256),
            "EC" => match self.crv.as_deref() {
                Some("P-256") => Ok(Algorithm::ES256),
                Some("P-384") => Ok(Algorithm::ES384),
                Some(other) => bail!("unsupported EC curve: {other}"),
                None => Ok(Algorithm::ES256),
            },
            other => bail!("cannot infer algorithm for key type: {other}"),
        }
    }
}

// -- ID Token Claims --

/// Claims extracted from an OIDC ID token.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdTokenClaims {
    /// Issuer.
    pub iss: String,
    /// Subject (unique user ID at the IdP).
    pub sub: String,
    /// Audience (our client_id).
    pub aud: IdTokenAudience,
    /// Expiration (unix timestamp).
    pub exp: i64,
    /// Issued at (unix timestamp).
    pub iat: i64,
    /// Nonce (if provided in the auth request).
    #[serde(default)]
    pub nonce: Option<String>,
    /// Email address.
    #[serde(default)]
    pub email: Option<String>,
    /// Whether the email is verified.
    #[serde(default)]
    pub email_verified: Option<bool>,
    /// Display name.
    #[serde(default)]
    pub name: Option<String>,
}

/// The `aud` claim can be a single string or an array.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum IdTokenAudience {
    Single(String),
    Multiple(Vec<String>),
}

impl IdTokenAudience {
    /// Check if this audience contains the given client ID.
    pub fn contains(&self, client_id: &str) -> bool {
        match self {
            Self::Single(s) => s == client_id,
            Self::Multiple(v) => v.iter().any(|s| s == client_id),
        }
    }
}

// -- User provisioning --

/// Information extracted from an OIDC ID token for user provisioning.
#[derive(Clone, Debug)]
pub struct OidcUserInfo {
    /// The OIDC issuer URL.
    pub idp_issuer: String,
    /// The subject claim (unique user ID at the IdP).
    pub idp_subject: String,
    /// Email address (if available and verified).
    pub email: Option<String>,
    /// Display name (if available).
    pub name: Option<String>,
}

/// Trait for user lookup/creation during OIDC provisioning.
#[async_trait::async_trait]
pub trait OidcUserStore: Send + Sync {
    /// Find a user by their IdP (issuer, subject) pair.
    async fn find_by_idp(&self, issuer: &str, subject: &str) -> Result<Option<UserId>>;
    /// Find a user by email.
    async fn find_by_email(&self, email: &str) -> Result<Option<UserId>>;
    /// Create a new user from OIDC info and return their ID.
    async fn create_user(&self, info: &OidcUserInfo) -> Result<UserId>;
    /// Link an existing user to an IdP identity.
    async fn link_idp(&self, user_id: &UserId, issuer: &str, subject: &str) -> Result<()>;
}

// -- OidcProvider --

/// Configuration for an OIDC provider.
#[derive(Clone, Debug)]
pub struct OidcConfig {
    /// The OIDC issuer URL (e.g. `https://auth.example.com/realms/main`).
    pub issuer_url: String,
    /// Our OAuth2 client_id registered with this IdP.
    pub client_id: String,
    /// Client secret for confidential clients (server-side flows).
    /// `None` for public clients using PKCE.
    pub client_secret: Option<String>,
    /// Whether to auto-provision users on first login.
    pub auto_provision: bool,
    /// If set, only allow emails matching these domains.
    pub allowed_email_domains: Option<Vec<String>>,
}

/// An OIDC identity provider.
///
/// Discovers endpoints, fetches signing keys, validates ID tokens, and
/// provisions users.
pub struct OidcProvider {
    config: OidcConfig,
    discovery: OidcDiscovery,
    jwks: JwksResponse,
    #[allow(dead_code)]
    http_client: reqwest::Client,
}

impl OidcProvider {
    /// Discover an OIDC provider from its issuer URL.
    ///
    /// Fetches `.well-known/openid-configuration` and the JWKS.
    pub async fn discover(config: OidcConfig) -> Result<Self> {
        Self::discover_with_client(config, reqwest::Client::new()).await
    }

    /// Discover using a custom HTTP client (for testing).
    pub async fn discover_with_client(
        config: OidcConfig,
        http_client: reqwest::Client,
    ) -> Result<Self> {
        let discovery_url = format!(
            "{}/.well-known/openid-configuration",
            config.issuer_url.trim_end_matches('/')
        );
        debug!(url = %discovery_url, "fetching OIDC discovery document");

        let discovery: OidcDiscovery = http_client
            .get(&discovery_url)
            .send()
            .await
            .context("failed to fetch OIDC discovery document")?
            .error_for_status()
            .context("OIDC discovery endpoint returned error")?
            .json()
            .await
            .context("failed to parse OIDC discovery document")?;

        // Verify the issuer matches what we expect.
        if discovery.issuer != config.issuer_url {
            bail!(
                "OIDC issuer mismatch: expected {}, got {}",
                config.issuer_url,
                discovery.issuer
            );
        }

        debug!(jwks_uri = %discovery.jwks_uri, "fetching JWKS");

        let jwks: JwksResponse = http_client
            .get(&discovery.jwks_uri)
            .send()
            .await
            .context("failed to fetch JWKS")?
            .error_for_status()
            .context("JWKS endpoint returned error")?
            .json()
            .await
            .context("failed to parse JWKS")?;

        Ok(Self {
            config,
            discovery,
            jwks,
            http_client,
        })
    }

    /// Get the discovery document.
    pub fn discovery(&self) -> &OidcDiscovery {
        &self.discovery
    }

    /// Get the client ID registered with this IdP.
    pub fn client_id(&self) -> &str {
        &self.config.client_id
    }

    /// Validate an ID token and return the claims.
    ///
    /// Verifies: signature (via JWKS), issuer, audience, and expiry.
    pub fn validate_id_token(
        &self,
        token: &str,
        expected_nonce: Option<&str>,
    ) -> Result<IdTokenClaims> {
        // Decode the header to find the key ID.
        let header = decode_header(token).context("failed to decode ID token header")?;

        // Find the matching key in the JWKS.
        let jwk = self.find_key(&header.kid)?;
        let algorithm = jwk.algorithm()?;
        let decoding_key = jwk.to_decoding_key()?;

        // Build validation.
        let mut validation = Validation::new(algorithm);
        validation.set_issuer(&[&self.config.issuer_url]);
        validation.set_audience(&[&self.config.client_id]);

        let token_data = decode::<IdTokenClaims>(token, &decoding_key, &validation)
            .context("ID token validation failed")?;

        let claims = token_data.claims;

        // Verify audience contains our client_id.
        if !claims.aud.contains(&self.config.client_id) {
            bail!(
                "ID token audience does not contain our client_id: {}",
                self.config.client_id
            );
        }

        // Verify nonce if expected.
        if let Some(expected) = expected_nonce {
            match &claims.nonce {
                Some(nonce) if nonce == expected => {}
                Some(nonce) => bail!("nonce mismatch: expected {expected}, got {nonce}"),
                None => bail!("expected nonce in ID token but none present"),
            }
        }

        Ok(claims)
    }

    /// Extract user info from validated ID token claims.
    pub fn extract_user_info(&self, claims: &IdTokenClaims) -> OidcUserInfo {
        OidcUserInfo {
            idp_issuer: claims.iss.clone(),
            idp_subject: claims.sub.clone(),
            email: claims.email.clone(),
            name: claims.name.clone(),
        }
    }

    /// Provision or look up a user from OIDC claims.
    ///
    /// 1. Look up by (issuer, subject) — if found, return existing user.
    /// 2. Look up by email — if found, link IdP and return.
    /// 3. If auto-provision is enabled, create a new user.
    /// 4. Otherwise, fail.
    pub async fn provision_or_lookup(
        &self,
        claims: &IdTokenClaims,
        store: &dyn OidcUserStore,
    ) -> Result<UserId> {
        let info = self.extract_user_info(claims);

        // Check email domain restriction.
        if let Some(ref domains) = self.config.allowed_email_domains {
            match &info.email {
                Some(email) => {
                    let domain = email.rsplit_once('@').map(|(_, d)| d).unwrap_or("");
                    if !domains.iter().any(|d| d == domain) {
                        bail!("email domain '{domain}' not in allowed list for this organization");
                    }
                }
                None => bail!("OIDC provider did not return an email; cannot verify domain"),
            }
        }

        // 1. Look up by IdP identity.
        if let Some(user_id) = store
            .find_by_idp(&info.idp_issuer, &info.idp_subject)
            .await?
        {
            debug!(user_id = %user_id, "found existing user by IdP identity");
            return Ok(user_id);
        }

        // 2. Look up by email and link.
        if let Some(ref email) = info.email
            && let Some(user_id) = store.find_by_email(email).await?
        {
            debug!(user_id = %user_id, email = %email, "linking existing user to IdP");
            store
                .link_idp(&user_id, &info.idp_issuer, &info.idp_subject)
                .await?;
            return Ok(user_id);
        }

        // 3. Auto-provision.
        if self.config.auto_provision {
            let user_id = store.create_user(&info).await?;
            debug!(user_id = %user_id, "auto-provisioned new user from OIDC");
            return Ok(user_id);
        }

        bail!(
            "no existing user found for OIDC subject '{}' and auto-provisioning is disabled",
            info.idp_subject
        );
    }

    /// Exchange an authorization code at the IdP's token endpoint for an ID token.
    ///
    /// Sends a `POST` to the IdP `token_endpoint` with `grant_type=authorization_code`,
    /// validates the returned `id_token`, and returns the claims.
    pub async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<IdTokenClaims> {
        let mut params = vec![
            ("grant_type", "authorization_code".to_string()),
            ("code", code.to_string()),
            ("redirect_uri", redirect_uri.to_string()),
            ("client_id", self.config.client_id.clone()),
        ];
        if let Some(ref secret) = self.config.client_secret {
            params.push(("client_secret", secret.clone()));
        }

        let resp = self
            .http_client
            .post(&self.discovery.token_endpoint)
            .form(&params)
            .send()
            .await
            .context("failed to POST to IdP token endpoint")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("IdP token endpoint returned {status}: {body}");
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            id_token: String,
        }

        let token_resp: TokenResponse = resp
            .json()
            .await
            .context("failed to parse IdP token response")?;

        self.validate_id_token(&token_resp.id_token, None)
    }

    /// Find a JWK by key ID. Falls back to the first signing key if no kid is specified.
    fn find_key(&self, kid: &Option<String>) -> Result<&Jwk> {
        if let Some(kid) = kid {
            self.jwks
                .keys
                .iter()
                .find(|k| k.kid.as_deref() == Some(kid))
                .with_context(|| format!("no JWK found with kid '{kid}'"))
        } else {
            // No kid in token header — use the first signing key.
            self.jwks
                .keys
                .iter()
                .find(|k| k.key_use.as_deref() != Some("enc"))
                .or_else(|| self.jwks.keys.first())
                .context("JWKS contains no keys")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    use chrono::Utc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // -- Test RSA key (2048-bit, generated for testing only) --

    const TEST_RSA_PRIVATE_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDfk5v7uyKn7Xee
tzR7ZRsra2drSEZHQYOHfIv8bm2zSrHrFdUqmTL0t6mj1jPYXWNBVZCyiAzL2qiN
dddT3kNBdpI80aMpIT3u/sOa5I0hq11nGAKeezGLqizOPcztgZQa3FYFve5HxnXT
K9UkboBqhvIXKUx8fZsO8sFHdscS4VGrNn9VLPz8UjEOmJsJEiaqSjw4qx+1WsBj
xTlQNqV/y/durbTCyNSUlRex1C4PEHGg+rr/yobUk9MqxGHNeKefKkSaDcij72lz
osvxelXHLoonV54X9VrpMnXdWAw7yJ6LbWsA9MVoEiqDfnDFWcaFsfQeCq+1/Hnd
b9jdEJPJAgMBAAECggEAYsYKSBzlUy44xkBnKcrBxZ12O7HbBpj9fGp8R+IbifXK
g7MKEX9MQUww4IZ+Mi0T8CXWvuEXUiqAg7qXjmBn8zBorADr5fxfKcqY7UHizgiw
w56abZy8h1j/4X/xHM69+V31jSTbdA9MN6aqTCWbizSiGLRwq6EsU17RH/rsOTzJ
yiap+GgvDBHRSMLPbH+A7A9hNZh6I6Dv9KReGUmOEKVQpl9aetJfxsCkXII7ztUL
HUQNZVluA2OqR1s7VpYO9yNmZoTtfqmUswIg2YaFXSjVgdDFNH6Rf3reufWehzKP
45ufflne4FT3f8VFXe8+FonCZE5RF2tIawC2gusWfwKBgQD1vX9Q2V5wKg/nQq/6
vVv370XmypVBajXvg0TP95E7p+EORGoDYj46LWNKZRifeGjavmM29jiqPnk8UZjC
3qEmZILZC2/IdsR7hVpEn7uyedc2wBFSD5pYR9333VsV+rbBMmexO8DdviR8L4Tr
8ESx+FsVnqkhBgGjQWjXOapf5wKBgQDo6Tlu1VZQ1ijLWYomeKDiKQloEHMSW3lP
vgd1XqTYLrR7kv6935cK6dY0ZvcfA4HE6+RU4pene/vU6bMxvSJjwNwhRhOS7CGl
k+5Mtdfq3+VhaXA5dmyYu2IrJ3WJg+9QjxAodek4kYx1W8MLJnKc2GzIuED+ewg0
PuMXTcm4zwKBgC+z820MZSq8341zAppX++xrRFSC6uph5cpy3v7H/idodWXBnhq+
DXpZqTad3WPHigM8hiH7NhDGQ96TsGXTtdCwHj5n2/E8LPQVdOpxX4xL3p1AN5yI
btvIR6yACdiAbM2gLUTYZp4k9QwuZU0vvQYXQgc2X3qLofHBFssA5LPtAoGAI3/w
zhDcQCP0QdJa+TQnqXEBywe+0kx4+AuJzXzoeT7dKXylMUGUHwi3KnOLNQHu1Jnz
ynBjFxcRskkQlAM066loo/WvZBRzqG4cwzpwN496wdc1ULzZHoppExTHmHcwkcHM
f65BJusgUn7zAo8QpxFhu1JCLceI35W6PUIQ/gcCgYEA34AmU6srEH6iJx7wFQez
vsldAzUeGESZCNai30KZPeUrFxGHNcWU72xkBJnHnZCcAejKtW/XqKd4AWY25buV
6vNdaNWeyJt50s+E8VTfGi2otEAiZh+dJxAmgLJ/PA0RDyBft2qGkKMSfS2PXdIr
q2HMPOhMSkWZ+rP+jJ76H6s=
-----END PRIVATE KEY-----"#;

    // Pre-computed base64url-encoded RSA public key components from the above key.
    const TEST_RSA_N: &str = "35Ob-7sip-13nrc0e2UbK2tna0hGR0GDh3yL_G5ts0qx6xXVKpky9Lepo9Yz2F1jQVWQsogMy9qojXXXU95DQXaSPNGjKSE97v7DmuSNIatdZxgCnnsxi6oszj3M7YGUGtxWBb3uR8Z10yvVJG6AaobyFylMfH2bDvLBR3bHEuFRqzZ_VSz8_FIxDpibCRImqko8OKsftVrAY8U5UDalf8v3bq20wsjUlJUXsdQuDxBxoPq6_8qG1JPTKsRhzXinnypEmg3Io-9pc6LL8XpVxy6KJ1eeF_Va6TJ13VgMO8iei21rAPTFaBIqg35wxVnGhbH0Hgqvtfx53W_Y3RCTyQ";
    const TEST_RSA_E: &str = "AQAB";

    fn test_encoding_key() -> jsonwebtoken::EncodingKey {
        jsonwebtoken::EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_PEM.as_bytes())
            .expect("test RSA PEM should be valid")
    }

    fn test_jwk() -> Jwk {
        Jwk {
            kty: "RSA".into(),
            kid: Some("test-key-1".into()),
            alg: Some("RS256".into()),
            key_use: Some("sig".into()),
            n: Some(TEST_RSA_N.into()),
            e: Some(TEST_RSA_E.into()),
            crv: None,
            x: None,
            y: None,
        }
    }

    /// Sign a test ID token with the test RSA key.
    fn sign_test_id_token(claims: &IdTokenClaims) -> String {
        let mut header = jsonwebtoken::Header::new(Algorithm::RS256);
        header.kid = Some("test-key-1".into());
        jsonwebtoken::encode(&header, claims, &test_encoding_key())
            .expect("signing test token should succeed")
    }

    fn make_test_claims(issuer: &str, client_id: &str) -> IdTokenClaims {
        let now = Utc::now().timestamp();
        IdTokenClaims {
            iss: issuer.into(),
            sub: "idp-user-123".into(),
            aud: IdTokenAudience::Single(client_id.into()),
            exp: now + 3600,
            iat: now,
            nonce: None,
            email: Some("alice@example.com".into()),
            email_verified: Some(true),
            name: Some("Alice".into()),
        }
    }

    /// Build mock discovery + JWKS endpoints and return an OidcProvider.
    async fn setup_mock_provider(server: &MockServer) -> OidcProvider {
        let issuer = server.uri();
        let jwks_uri = format!("{issuer}/jwks");

        let discovery = OidcDiscovery {
            issuer: issuer.clone(),
            authorization_endpoint: format!("{issuer}/authorize"),
            token_endpoint: format!("{issuer}/token"),
            userinfo_endpoint: Some(format!("{issuer}/userinfo")),
            jwks_uri: jwks_uri.clone(),
            response_types_supported: vec!["code".into()],
            subject_types_supported: vec!["public".into()],
            id_token_signing_alg_values_supported: vec!["RS256".into()],
        };

        let jwks = JwksResponse {
            keys: vec![test_jwk()],
        };

        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&discovery))
            .mount(server)
            .await;

        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&jwks))
            .mount(server)
            .await;

        let config = OidcConfig {
            issuer_url: issuer,
            client_id: "test-client".into(),
            client_secret: None,
            auto_provision: true,
            allowed_email_domains: None,
        };

        OidcProvider::discover(config).await.unwrap()
    }

    // -- In-memory OidcUserStore for testing --

    struct InMemoryUserStore {
        state: Mutex<UserStoreState>,
    }

    struct UserStoreState {
        users: Vec<StoredUser>,
        next_id: u64,
    }

    struct StoredUser {
        id: UserId,
        email: Option<String>,
        idp_links: Vec<(String, String)>, // (issuer, subject)
    }

    impl InMemoryUserStore {
        fn new() -> Self {
            Self {
                state: Mutex::new(UserStoreState {
                    users: Vec::new(),
                    next_id: 1,
                }),
            }
        }

        fn with_user(self, id: &str, email: &str, idp_links: Vec<(&str, &str)>) -> Self {
            let mut state = self.state.lock().unwrap();
            state.users.push(StoredUser {
                id: UserId::from(id),
                email: Some(email.into()),
                idp_links: idp_links
                    .into_iter()
                    .map(|(i, s)| (i.into(), s.into()))
                    .collect(),
            });
            drop(state);
            self
        }
    }

    #[async_trait::async_trait]
    impl OidcUserStore for InMemoryUserStore {
        async fn find_by_idp(&self, issuer: &str, subject: &str) -> Result<Option<UserId>> {
            let state = self.state.lock().unwrap();
            Ok(state
                .users
                .iter()
                .find(|u| u.idp_links.iter().any(|(i, s)| i == issuer && s == subject))
                .map(|u| u.id.clone()))
        }

        async fn find_by_email(&self, email: &str) -> Result<Option<UserId>> {
            let state = self.state.lock().unwrap();
            Ok(state
                .users
                .iter()
                .find(|u| u.email.as_deref() == Some(email))
                .map(|u| u.id.clone()))
        }

        async fn create_user(&self, info: &OidcUserInfo) -> Result<UserId> {
            let mut state = self.state.lock().unwrap();
            let id = UserId::from(format!("usr_{}", state.next_id));
            state.next_id += 1;
            state.users.push(StoredUser {
                id: id.clone(),
                email: info.email.clone(),
                idp_links: vec![(info.idp_issuer.clone(), info.idp_subject.clone())],
            });
            Ok(id)
        }

        async fn link_idp(&self, user_id: &UserId, issuer: &str, subject: &str) -> Result<()> {
            let mut state = self.state.lock().unwrap();
            let user = state
                .users
                .iter_mut()
                .find(|u| u.id == *user_id)
                .context("user not found")?;
            user.idp_links.push((issuer.into(), subject.into()));
            Ok(())
        }
    }

    // -- Discovery tests --

    #[tokio::test]
    async fn discover_fetches_config_and_jwks() {
        let server = MockServer::start().await;
        let provider = setup_mock_provider(&server).await;

        assert_eq!(provider.discovery().issuer, server.uri());
        assert_eq!(
            provider.discovery().authorization_endpoint,
            format!("{}/authorize", server.uri())
        );
        assert_eq!(provider.jwks.keys.len(), 1);
        assert_eq!(provider.jwks.keys[0].kid.as_deref(), Some("test-key-1"));
    }

    #[tokio::test]
    async fn discover_rejects_issuer_mismatch() {
        let server = MockServer::start().await;
        let issuer = server.uri();

        let discovery = OidcDiscovery {
            issuer: "https://wrong-issuer.example.com".into(),
            authorization_endpoint: format!("{issuer}/authorize"),
            token_endpoint: format!("{issuer}/token"),
            userinfo_endpoint: None,
            jwks_uri: format!("{issuer}/jwks"),
            response_types_supported: vec![],
            subject_types_supported: vec![],
            id_token_signing_alg_values_supported: vec![],
        };

        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&discovery))
            .mount(&server)
            .await;

        let config = OidcConfig {
            issuer_url: issuer,
            client_id: "test-client".into(),
            client_secret: None,
            auto_provision: true,
            allowed_email_domains: None,
        };

        let result = OidcProvider::discover(config).await;
        assert!(result.is_err());
        let msg = format!("{:?}", result.err().unwrap());
        assert!(
            msg.contains("issuer mismatch"),
            "expected issuer mismatch error, got: {msg}"
        );
    }

    // -- ID token validation tests --

    #[tokio::test]
    async fn validate_id_token_with_valid_signature() {
        let server = MockServer::start().await;
        let provider = setup_mock_provider(&server).await;
        let claims = make_test_claims(&server.uri(), "test-client");
        let token = sign_test_id_token(&claims);

        let result = provider.validate_id_token(&token, None).unwrap();
        assert_eq!(result.sub, "idp-user-123");
        assert_eq!(result.email.as_deref(), Some("alice@example.com"));
        assert_eq!(result.name.as_deref(), Some("Alice"));
    }

    #[tokio::test]
    async fn validate_id_token_rejects_wrong_issuer() {
        let server = MockServer::start().await;
        let provider = setup_mock_provider(&server).await;
        let claims = make_test_claims("https://wrong-issuer.example.com", "test-client");
        let token = sign_test_id_token(&claims);

        let err = provider.validate_id_token(&token, None);
        assert!(err.is_err(), "should reject token with wrong issuer");
    }

    #[tokio::test]
    async fn validate_id_token_rejects_wrong_audience() {
        let server = MockServer::start().await;
        let provider = setup_mock_provider(&server).await;
        let claims = make_test_claims(&server.uri(), "wrong-client");
        let token = sign_test_id_token(&claims);

        let err = provider.validate_id_token(&token, None);
        assert!(err.is_err(), "should reject token with wrong audience");
    }

    #[tokio::test]
    async fn validate_id_token_rejects_expired() {
        let server = MockServer::start().await;
        let provider = setup_mock_provider(&server).await;
        let mut claims = make_test_claims(&server.uri(), "test-client");
        claims.exp = Utc::now().timestamp() - 3600; // Expired 1 hour ago.
        claims.iat = Utc::now().timestamp() - 7200;
        let token = sign_test_id_token(&claims);

        let err = provider.validate_id_token(&token, None);
        assert!(err.is_err(), "should reject expired token");
    }

    #[tokio::test]
    async fn validate_id_token_checks_nonce() {
        let server = MockServer::start().await;
        let provider = setup_mock_provider(&server).await;

        // Token with nonce.
        let mut claims = make_test_claims(&server.uri(), "test-client");
        claims.nonce = Some("test-nonce-123".into());
        let token = sign_test_id_token(&claims);

        // Correct nonce passes.
        let result = provider.validate_id_token(&token, Some("test-nonce-123"));
        assert!(result.is_ok());

        // Wrong nonce fails.
        let err = provider.validate_id_token(&token, Some("wrong-nonce"));
        assert!(err.is_err());

        // Token without nonce but nonce expected fails.
        let claims_no_nonce = make_test_claims(&server.uri(), "test-client");
        let token_no_nonce = sign_test_id_token(&claims_no_nonce);
        let err = provider.validate_id_token(&token_no_nonce, Some("expected-nonce"));
        assert!(err.is_err());
    }

    // -- User provisioning tests --

    #[tokio::test]
    async fn provision_finds_existing_user_by_idp() {
        let server = MockServer::start().await;
        let provider = setup_mock_provider(&server).await;
        let claims = make_test_claims(&server.uri(), "test-client");

        let store = InMemoryUserStore::new().with_user(
            "usr_existing",
            "alice@example.com",
            vec![(&server.uri(), "idp-user-123")],
        );

        let user_id = provider.provision_or_lookup(&claims, &store).await.unwrap();
        assert_eq!(user_id, UserId::from("usr_existing"));
    }

    #[tokio::test]
    async fn provision_links_existing_user_by_email() {
        let server = MockServer::start().await;
        let provider = setup_mock_provider(&server).await;
        let claims = make_test_claims(&server.uri(), "test-client");

        // User exists by email but not linked to this IdP.
        let store = InMemoryUserStore::new().with_user("usr_email", "alice@example.com", vec![]);

        let user_id = provider.provision_or_lookup(&claims, &store).await.unwrap();
        assert_eq!(user_id, UserId::from("usr_email"));

        // Verify the IdP link was created.
        let linked = store
            .find_by_idp(&server.uri(), "idp-user-123")
            .await
            .unwrap();
        assert_eq!(linked, Some(UserId::from("usr_email")));
    }

    #[tokio::test]
    async fn provision_auto_creates_user() {
        let server = MockServer::start().await;
        let provider = setup_mock_provider(&server).await;
        let claims = make_test_claims(&server.uri(), "test-client");

        let store = InMemoryUserStore::new();
        let user_id = provider.provision_or_lookup(&claims, &store).await.unwrap();
        assert_eq!(user_id, UserId::from("usr_1"));
    }

    #[tokio::test]
    async fn provision_rejects_disallowed_email_domain() {
        let server = MockServer::start().await;

        // Override the provider config to restrict email domains.
        let issuer = server.uri();
        let jwks_uri = format!("{issuer}/jwks");

        let discovery = OidcDiscovery {
            issuer: issuer.clone(),
            authorization_endpoint: format!("{issuer}/authorize"),
            token_endpoint: format!("{issuer}/token"),
            userinfo_endpoint: None,
            jwks_uri: jwks_uri.clone(),
            response_types_supported: vec![],
            subject_types_supported: vec![],
            id_token_signing_alg_values_supported: vec![],
        };

        let jwks = JwksResponse {
            keys: vec![test_jwk()],
        };

        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&discovery))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&jwks))
            .mount(&server)
            .await;

        let config = OidcConfig {
            issuer_url: issuer.clone(),
            client_id: "test-client".into(),
            client_secret: None,
            auto_provision: true,
            allowed_email_domains: Some(vec!["acme.com".into()]),
        };

        let provider = OidcProvider::discover(config).await.unwrap();
        let claims = make_test_claims(&issuer, "test-client");

        let store = InMemoryUserStore::new();
        let err = provider.provision_or_lookup(&claims, &store).await;
        assert!(err.is_err());
        let msg = format!("{:?}", err.unwrap_err());
        assert!(
            msg.contains("not in allowed list"),
            "expected domain rejection, got: {msg}"
        );
    }

    #[tokio::test]
    async fn provision_fails_when_auto_provision_disabled() {
        let server = MockServer::start().await;
        let issuer = server.uri();
        let jwks_uri = format!("{issuer}/jwks");

        let discovery = OidcDiscovery {
            issuer: issuer.clone(),
            authorization_endpoint: format!("{issuer}/authorize"),
            token_endpoint: format!("{issuer}/token"),
            userinfo_endpoint: None,
            jwks_uri,
            response_types_supported: vec![],
            subject_types_supported: vec![],
            id_token_signing_alg_values_supported: vec![],
        };

        let jwks = JwksResponse {
            keys: vec![test_jwk()],
        };

        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&discovery))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&jwks))
            .mount(&server)
            .await;

        let config = OidcConfig {
            issuer_url: issuer.clone(),
            client_id: "test-client".into(),
            client_secret: None,
            auto_provision: false,
            allowed_email_domains: None,
        };

        let provider = OidcProvider::discover(config).await.unwrap();
        let claims = make_test_claims(&issuer, "test-client");

        let store = InMemoryUserStore::new();
        let err = provider.provision_or_lookup(&claims, &store).await;
        assert!(err.is_err());
        let msg = format!("{:?}", err.unwrap_err());
        assert!(
            msg.contains("auto-provisioning is disabled"),
            "expected auto-provision error, got: {msg}"
        );
    }

    #[test]
    fn extract_user_info_from_claims() {
        let claims = IdTokenClaims {
            iss: "https://idp.example.com".into(),
            sub: "user-42".into(),
            aud: IdTokenAudience::Single("client".into()),
            exp: 0,
            iat: 0,
            nonce: None,
            email: Some("bob@corp.com".into()),
            email_verified: Some(true),
            name: Some("Bob".into()),
        };

        // Build a minimal provider just for extract_user_info.
        let provider = OidcProvider {
            config: OidcConfig {
                issuer_url: "https://idp.example.com".into(),
                client_id: "client".into(),
                client_secret: None,
                auto_provision: false,
                allowed_email_domains: None,
            },
            discovery: OidcDiscovery {
                issuer: "https://idp.example.com".into(),
                authorization_endpoint: String::new(),
                token_endpoint: String::new(),
                userinfo_endpoint: None,
                jwks_uri: String::new(),
                response_types_supported: vec![],
                subject_types_supported: vec![],
                id_token_signing_alg_values_supported: vec![],
            },
            jwks: JwksResponse { keys: vec![] },
            http_client: reqwest::Client::new(),
        };

        let info = provider.extract_user_info(&claims);
        assert_eq!(info.idp_issuer, "https://idp.example.com");
        assert_eq!(info.idp_subject, "user-42");
        assert_eq!(info.email.as_deref(), Some("bob@corp.com"));
        assert_eq!(info.name.as_deref(), Some("Bob"));
    }

    #[test]
    fn audience_contains_single() {
        let aud = IdTokenAudience::Single("client-a".into());
        assert!(aud.contains("client-a"));
        assert!(!aud.contains("client-b"));
    }

    #[test]
    fn audience_contains_multiple() {
        let aud = IdTokenAudience::Multiple(vec!["client-a".into(), "client-b".into()]);
        assert!(aud.contains("client-a"));
        assert!(aud.contains("client-b"));
        assert!(!aud.contains("client-c"));
    }

    // -- exchange_code tests --

    #[tokio::test]
    async fn exchange_code_returns_id_token_claims() {
        let server = MockServer::start().await;
        let issuer = server.uri();

        // Set up discovery + JWKS (reuse helper).
        let provider = setup_mock_provider(&server).await;

        // Sign a test ID token that the IdP's token endpoint will return.
        let claims = make_test_claims(&issuer, "test-client");
        let id_token = sign_test_id_token(&claims);

        // Mock the IdP token endpoint.
        let token_response = serde_json::json!({
            "access_token": "idp-access-token",
            "token_type": "Bearer",
            "expires_in": 3600,
            "id_token": id_token,
        });
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&token_response))
            .mount(&server)
            .await;

        let result = provider
            .exchange_code("auth-code-123", "http://localhost/oauth/callback")
            .await
            .unwrap();

        assert_eq!(result.sub, "idp-user-123");
        assert_eq!(result.email.as_deref(), Some("alice@example.com"));
    }

    #[tokio::test]
    async fn exchange_code_with_client_secret() {
        let server = MockServer::start().await;
        let issuer = server.uri();
        let jwks_uri = format!("{issuer}/jwks");

        let discovery = OidcDiscovery {
            issuer: issuer.clone(),
            authorization_endpoint: format!("{issuer}/authorize"),
            token_endpoint: format!("{issuer}/token"),
            userinfo_endpoint: None,
            jwks_uri: jwks_uri.clone(),
            response_types_supported: vec!["code".into()],
            subject_types_supported: vec!["public".into()],
            id_token_signing_alg_values_supported: vec!["RS256".into()],
        };

        let jwks = JwksResponse {
            keys: vec![test_jwk()],
        };

        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&discovery))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&jwks))
            .mount(&server)
            .await;

        let config = OidcConfig {
            issuer_url: issuer.clone(),
            client_id: "test-client".into(),
            client_secret: Some("super-secret".into()),
            auto_provision: true,
            allowed_email_domains: None,
        };

        let provider = OidcProvider::discover(config).await.unwrap();

        let claims = make_test_claims(&issuer, "test-client");
        let id_token = sign_test_id_token(&claims);

        let token_response = serde_json::json!({
            "access_token": "idp-access-token",
            "token_type": "Bearer",
            "expires_in": 3600,
            "id_token": id_token,
        });

        // Verify that the client_secret is included in the request body.
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(wiremock::matchers::body_string_contains(
                "client_secret=super-secret",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&token_response))
            .mount(&server)
            .await;

        let result = provider
            .exchange_code("auth-code-456", "http://localhost/oauth/callback")
            .await
            .unwrap();

        assert_eq!(result.sub, "idp-user-123");
    }

    #[tokio::test]
    async fn exchange_code_rejects_error_response() {
        let server = MockServer::start().await;
        let provider = setup_mock_provider(&server).await;

        // Mock a token endpoint error.
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "invalid_grant",
                "error_description": "code expired"
            })))
            .mount(&server)
            .await;

        let err = provider
            .exchange_code("bad-code", "http://localhost/oauth/callback")
            .await;

        assert!(err.is_err(), "should reject error response from IdP");
    }
}
