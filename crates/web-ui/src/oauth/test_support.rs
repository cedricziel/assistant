//! Shared fixtures for OIDC tests: a mock IdP, a signing key, and org seeding.

use chrono::{DateTime, Utc};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use assistant_auth::oidc::{IdTokenClaims, Jwk, JwksResponse, OidcDiscovery};
use assistant_core::clock::{Clock, SystemClock};
use assistant_core::identity::OrgId;
use assistant_core::store::{OrgOidcConfig, OrgOidcConfigStore, OrgStore, Organization};
use assistant_storage::OrgStorageLayer;

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

const TEST_RSA_N: &str = "35Ob-7sip-13nrc0e2UbK2tna0hGR0GDh3yL_G5ts0qx6xXVKpky9Lepo9Yz2F1jQVWQsogMy9qojXXXU95DQXaSPNGjKSE97v7DmuSNIatdZxgCnnsxi6oszj3M7YGUGtxWBb3uR8Z10yvVJG6AaobyFylMfH2bDvLBR3bHEuFRqzZ_VSz8_FIxDpibCRImqko8OKsftVrAY8U5UDalf8v3bq20wsjUlJUXsdQuDxBxoPq6_8qG1JPTKsRhzXinnypEmg3Io-9pc6LL8XpVxy6KJ1eeF_Va6TJ13VgMO8iei21rAPTFaBIqg35wxVnGhbH0Hgqvtfx53W_Y3RCTyQ";
const TEST_RSA_E: &str = "AQAB";

/// Sign `claims` as an RS256 ID token that [`mount_idp`]'s JWKS verifies.
pub(crate) fn sign_test_id_token(claims: &IdTokenClaims) -> String {
    let encoding_key =
        jsonwebtoken::EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_PEM.as_bytes()).unwrap();
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("test-key-1".into());
    jsonwebtoken::encode(&header, claims, &encoding_key).unwrap()
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

/// Mount discovery and JWKS endpoints on `server` for an IdP whose issuer is
/// the server's own URI.
pub(crate) async fn mount_idp(server: &MockServer) {
    let issuer = server.uri();
    let discovery = OidcDiscovery {
        issuer: issuer.clone(),
        authorization_endpoint: format!("{issuer}/authorize"),
        token_endpoint: format!("{issuer}/token"),
        userinfo_endpoint: None,
        jwks_uri: format!("{issuer}/jwks"),
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
}

/// Number of discovery-document fetches `server` has received.
pub(crate) async fn discovery_hits(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|r| r.url.path() == "/.well-known/openid-configuration")
        .count()
}

/// Create an org with the given auth mode.
pub(crate) async fn seed_org(storage: &OrgStorageLayer, id: &str, slug: &str, auth_mode: &str) {
    let now = SystemClock.now();
    storage
        .org_store()
        .create_org(&Organization {
            id: OrgId::from(id),
            name: slug.into(),
            slug: slug.into(),
            auth_mode: auth_mode.into(),
            created_at: now,
            updated_at: now,
        })
        .await
        .unwrap();
}

/// An OIDC config for `org_id` pointing at `issuer`.
pub(crate) fn oidc_config(
    org_id: &str,
    issuer: &str,
    client_id: &str,
    updated_at: DateTime<Utc>,
) -> OrgOidcConfig {
    OrgOidcConfig {
        org_id: OrgId::from(org_id),
        issuer_url: issuer.into(),
        client_id: client_id.into(),
        client_secret: None,
        auto_provision: true,
        allowed_email_domains: Vec::new(),
        created_at: updated_at,
        updated_at,
    }
}

/// Create an org in OIDC mode with a config pointing at `issuer`.
pub(crate) async fn seed_oidc_org(
    storage: &OrgStorageLayer,
    id: &str,
    slug: &str,
    issuer: &str,
    client_id: &str,
) {
    seed_org(storage, id, slug, "oidc").await;
    storage
        .oidc_config_store()
        .upsert_oidc_config(&oidc_config(id, issuer, client_id, SystemClock.now()))
        .await
        .unwrap();
}
