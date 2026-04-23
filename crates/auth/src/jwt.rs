//! JWT signing and validation.
//!
//! Uses HS256 (HMAC-SHA256) with a 256-bit shared secret for single-server
//! deployments. Keys can be generated fresh or loaded from secret files.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::{Duration, Utc};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
};
use serde::{Deserialize, Serialize};

use assistant_core::auth::AuthContext;
use assistant_core::identity::{OrgId, Role, SpaceId, UserId};

// -- Claims --

/// JWT claims issued by the assistant server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Issuer (the assistant server URL).
    pub iss: String,
    /// Subject (user ID).
    pub sub: String,
    /// Audience.
    pub aud: String,
    /// Expiration (unix timestamp).
    pub exp: i64,
    /// Issued at (unix timestamp).
    pub iat: i64,
    /// Unique token ID.
    pub jti: String,

    /// Organization ID.
    pub org_id: String,
    /// Organization slug.
    pub org_slug: String,
    /// User email.
    pub email: String,
    /// User display name.
    pub name: String,
    /// Space → role mapping.
    pub spaces: HashMap<String, String>,
    /// OAuth2 client ID that requested this token.
    pub client_id: String,
    /// Space-separated scope string (standard OAuth2 format).
    #[serde(default)]
    pub scope: String,
    /// Optional per-scope resource ID restrictions (e.g. from API keys).
    /// Key = scope string ("personas:read"), value = allowed resource IDs.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub scope_restrictions: HashMap<String, Vec<String>>,
}

// -- Key pair --

/// A shared secret for signing and verifying JWTs (HS256).
pub struct JwtKeyPair {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl JwtKeyPair {
    /// Generate a new 256-bit random shared secret.
    pub fn generate() -> Result<Self> {
        let secret = generate_secret();
        Ok(Self {
            encoding: EncodingKey::from_secret(&secret),
            decoding: DecodingKey::from_secret(&secret),
        })
    }

    /// Create a key pair from a shared secret (HS256).
    pub fn from_secret(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }

    /// Load a key pair from a secret file, or generate and persist one.
    pub fn load_or_generate(path: &Path) -> Result<Self> {
        if path.exists() {
            let secret =
                std::fs::read(path).with_context(|| format!("reading JWT secret from {path:?}"))?;
            Ok(Self::from_secret(&secret))
        } else {
            let secret = generate_secret();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating directory for {path:?}"))?;
            }
            // On Unix, create file with 0600 atomically to avoid a race
            // where umask could briefly make the secret world-readable.
            #[cfg(unix)]
            {
                use std::io::Write;
                use std::os::unix::fs::OpenOptionsExt;
                let mut f = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(path)
                    .with_context(|| format!("writing JWT secret to {path:?}"))?;
                f.write_all(&secret)
                    .with_context(|| format!("writing JWT secret to {path:?}"))?;
            }
            #[cfg(not(unix))]
            {
                std::fs::write(path, &secret)
                    .with_context(|| format!("writing JWT secret to {path:?}"))?;
            }
            Ok(Self::from_secret(&secret))
        }
    }
}

/// Generate a 256-bit random secret.
fn generate_secret() -> Vec<u8> {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut secret = vec![0u8; 32];
    rng.fill(&mut secret[..]);
    secret
}

// -- Signer / Validator --

/// Signs and validates JWTs for the assistant server.
pub struct JwtManager {
    key_pair: JwtKeyPair,
    issuer: String,
    audience: String,
    /// Access token lifetime.
    access_ttl: Duration,
}

impl JwtManager {
    /// Create a new JWT manager.
    pub fn new(key_pair: JwtKeyPair, issuer: String, audience: String) -> Self {
        Self {
            key_pair,
            issuer,
            audience,
            access_ttl: Duration::hours(1),
        }
    }

    /// Override the default access token TTL.
    pub fn with_access_ttl(mut self, ttl: Duration) -> Self {
        self.access_ttl = ttl;
        self
    }

    /// Sign a JWT for the given auth context.
    pub fn sign(&self, ctx: &AuthContext, org_slug: &str, name: &str) -> Result<String> {
        let now = Utc::now();
        let exp = now + self.access_ttl;

        let spaces: HashMap<String, String> = ctx
            .space_roles
            .iter()
            .map(|(s, r)| (s.0.clone(), r.to_string()))
            .collect();

        let scope_str = ctx
            .scopes
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        // Preserve resource-ID restrictions (e.g. from API keys).
        let mut scope_restrictions = HashMap::new();
        for s in &ctx.scopes {
            if let Some(ids) = &s.resource_ids {
                scope_restrictions.insert(s.to_string(), ids.clone());
            }
        }

        let claims = Claims {
            iss: self.issuer.clone(),
            sub: ctx.user_id.0.clone(),
            aud: self.audience.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: uuid::Uuid::new_v4().to_string(),
            org_id: ctx.org_id.0.clone(),
            org_slug: org_slug.to_owned(),
            email: ctx.email.clone(),
            name: name.to_owned(),
            spaces,
            client_id: ctx.client_id.clone(),
            scope: scope_str,
            scope_restrictions,
        };

        let header = Header::new(Algorithm::HS256);
        encode(&header, &claims, &self.key_pair.encoding).context("failed to sign JWT")
    }

    /// Validate a JWT and return the claims.
    pub fn validate(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);

        let token_data: TokenData<Claims> =
            decode(token, &self.key_pair.decoding, &validation).context("JWT validation failed")?;

        Ok(token_data.claims)
    }

    /// Validate a JWT and reconstruct an [`AuthContext`] from the claims.
    pub fn validate_to_context(&self, token: &str) -> Result<AuthContext> {
        let claims = self.validate(token)?;
        claims_to_context(&claims)
    }
}

/// Convert decoded JWT claims into an [`AuthContext`].
pub fn claims_to_context(claims: &Claims) -> Result<AuthContext> {
    let space_roles: HashMap<SpaceId, Role> = claims
        .spaces
        .iter()
        .map(|(space_id, role_str)| {
            let role = match role_str.as_str() {
                "org_admin" => Role::OrgAdmin,
                "space_admin" => Role::SpaceAdmin,
                "member" => Role::Member,
                "viewer" => Role::Viewer,
                other => bail!("unknown role in JWT: {other}"),
            };
            Ok((SpaceId(space_id.clone()), role))
        })
        .collect::<Result<_>>()?;

    let scopes = if claims.scope.is_empty() {
        vec![]
    } else {
        claims
            .scope
            .split(' ')
            .map(|s| {
                let mut scope = parse_scope(s)?;
                // Restore resource-ID restrictions from the JWT if present.
                if let Some(ids) = claims.scope_restrictions.get(s) {
                    scope.resource_ids = Some(ids.clone());
                }
                Ok(scope)
            })
            .collect::<Result<Vec<_>>>()?
    };

    Ok(AuthContext {
        user_id: UserId(claims.sub.clone()),
        org_id: OrgId(claims.org_id.clone()),
        email: claims.email.clone(),
        space_roles,
        scopes,
        client_id: claims.client_id.clone(),
    })
}

/// Parse a scope string like `"personas:read"` into a [`Scope`].
fn parse_scope(s: &str) -> Result<assistant_core::identity::Scope> {
    use assistant_core::identity::{Action, ResourceKind, Scope};

    let (res_str, act_str) = s
        .split_once(':')
        .with_context(|| format!("invalid scope format: {s}"))?;

    let resource = match res_str {
        "personas" => ResourceKind::Personas,
        "conversations" => ResourceKind::Conversations,
        "messages" => ResourceKind::Messages,
        "skills" => ResourceKind::Skills,
        "interfaces" => ResourceKind::Interfaces,
        "bindings" => ResourceKind::Bindings,
        "users" => ResourceKind::Users,
        "org" => ResourceKind::Org,
        "api_keys" => ResourceKind::ApiKeys,
        "spaces" => ResourceKind::Spaces,
        other => bail!("unknown resource kind in scope: {other}"),
    };

    let action = match act_str {
        "read" => Action::Read,
        "write" => Action::Write,
        "delete" => Action::Delete,
        "execute" => Action::Execute,
        "manage" => Action::Manage,
        other => bail!("unknown action in scope: {other}"),
    };

    Ok(Scope::new(resource, action))
}

#[cfg(test)]
mod tests {
    use assistant_core::identity::{Action, ResourceKind, Scope};

    use super::*;

    fn make_test_context() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::Member);
        space_roles.insert(SpaceId::from("mktg"), Role::Viewer);

        AuthContext {
            user_id: UserId::from("usr_alice"),
            org_id: OrgId::from("org_acme"),
            email: "alice@acme.com".into(),
            space_roles,
            scopes: vec![
                Scope::new(ResourceKind::Personas, Action::Read),
                Scope::new(ResourceKind::Conversations, Action::Write),
            ],
            client_id: "webui".into(),
        }
    }

    #[test]
    fn sign_and_validate_roundtrip() {
        let kp = JwtKeyPair::generate().unwrap();
        let mgr = JwtManager::new(kp, "https://test.local".into(), "https://test.local".into());

        let ctx = make_test_context();
        let token = mgr.sign(&ctx, "acme", "Alice").unwrap();

        let claims = mgr.validate(&token).unwrap();
        assert_eq!(claims.sub, "usr_alice");
        assert_eq!(claims.org_id, "org_acme");
        assert_eq!(claims.email, "alice@acme.com");
        assert_eq!(claims.client_id, "webui");
        assert_eq!(claims.spaces.get("eng"), Some(&"member".to_string()));
        assert_eq!(claims.spaces.get("mktg"), Some(&"viewer".to_string()));
        assert!(claims.scope.contains("personas:read"));
        assert!(claims.scope.contains("conversations:write"));
    }

    #[test]
    fn validate_to_context_roundtrip() {
        let kp = JwtKeyPair::generate().unwrap();
        let mgr = JwtManager::new(kp, "https://test.local".into(), "https://test.local".into());

        let ctx = make_test_context();
        let token = mgr.sign(&ctx, "acme", "Alice").unwrap();

        let restored = mgr.validate_to_context(&token).unwrap();
        assert_eq!(restored.user_id, ctx.user_id);
        assert_eq!(restored.org_id, ctx.org_id);
        assert_eq!(restored.email, ctx.email);
        assert_eq!(restored.client_id, ctx.client_id);
        assert_eq!(restored.role_in(&SpaceId::from("eng")), Some(&Role::Member));
        assert_eq!(
            restored.role_in(&SpaceId::from("mktg")),
            Some(&Role::Viewer)
        );
    }

    #[test]
    fn expired_token_is_rejected() {
        let kp = JwtKeyPair::generate().unwrap();
        let mgr = JwtManager::new(kp, "https://test.local".into(), "https://test.local".into())
            .with_access_ttl(Duration::seconds(-120)); // Well past the 60s default leeway.

        let ctx = make_test_context();
        let token = mgr.sign(&ctx, "acme", "Alice").unwrap();

        let err = mgr.validate(&token);
        assert!(
            err.is_err(),
            "expected validation to fail for expired token"
        );
        // Verify the root cause is an expiration error (in the anyhow chain).
        let debug_msg = format!("{:?}", err.unwrap_err());
        assert!(
            debug_msg.contains("ExpiredSignature") || debug_msg.contains("expired"),
            "expected expiration error in chain, got: {debug_msg}"
        );
    }

    #[test]
    fn wrong_secret_is_rejected() {
        let kp1 = JwtKeyPair::from_secret(b"secret-one-that-is-long-enough!!");
        let kp2 = JwtKeyPair::from_secret(b"secret-two-that-is-long-enough!!");

        let mgr1 = JwtManager::new(
            kp1,
            "https://test.local".into(),
            "https://test.local".into(),
        );
        let mgr2 = JwtManager::new(
            kp2,
            "https://test.local".into(),
            "https://test.local".into(),
        );

        let ctx = make_test_context();
        let token = mgr1.sign(&ctx, "acme", "Alice").unwrap();

        let err = mgr2.validate(&token);
        assert!(err.is_err());
    }

    #[test]
    fn wrong_issuer_is_rejected() {
        let kp = JwtKeyPair::generate().unwrap();
        // Sign with issuer A.
        let secret = generate_secret();
        let kp_sign = JwtKeyPair::from_secret(&secret);
        let kp_verify = JwtKeyPair::from_secret(&secret);

        let mgr_sign = JwtManager::new(kp_sign, "https://a.local".into(), "https://a.local".into());
        let mgr_verify = JwtManager::new(
            kp_verify,
            "https://b.local".into(),
            "https://a.local".into(),
        );

        let ctx = make_test_context();
        let token = mgr_sign.sign(&ctx, "acme", "Alice").unwrap();

        let err = mgr_verify.validate(&token);
        assert!(err.is_err());
        let _ = kp; // suppress unused warning
    }

    #[test]
    fn parse_scope_roundtrip() {
        let scope = parse_scope("personas:read").unwrap();
        assert_eq!(scope.resource, ResourceKind::Personas);
        assert_eq!(scope.action, Action::Read);

        let scope = parse_scope("org:manage").unwrap();
        assert_eq!(scope.resource, ResourceKind::Org);
        assert_eq!(scope.action, Action::Manage);
    }

    #[test]
    fn parse_scope_invalid_format() {
        assert!(parse_scope("invalid").is_err());
        assert!(parse_scope("unknown:read").is_err());
        assert!(parse_scope("personas:unknown").is_err());
    }

    #[test]
    fn load_or_generate_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jwt.secret");

        let kp1 = JwtKeyPair::load_or_generate(&path).unwrap();
        assert!(path.exists());

        // Loading again should produce the same key.
        let kp2 = JwtKeyPair::load_or_generate(&path).unwrap();

        // Verify same key by signing+validating across instances.
        let mgr1 = JwtManager::new(
            kp1,
            "https://test.local".into(),
            "https://test.local".into(),
        );
        let mgr2 = JwtManager::new(
            kp2,
            "https://test.local".into(),
            "https://test.local".into(),
        );

        let ctx = make_test_context();
        let token = mgr1.sign(&ctx, "acme", "Alice").unwrap();
        let claims = mgr2.validate(&token).unwrap();
        assert_eq!(claims.sub, "usr_alice");
    }
}
