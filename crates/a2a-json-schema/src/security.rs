//! Security scheme definitions for the A2A protocol.
//!
//! Covers API key, HTTP auth, OAuth2, OpenID Connect, and mTLS schemes,
//! following the OpenAPI 3.2 Security Scheme Object model.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::StringList;

/// Defines a security scheme that can be used to secure an agent's endpoints.
///
/// This is a discriminated union based on the OpenAPI 3.2 Security Scheme
/// Object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityScheme {
    /// API key-based authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_security_scheme: Option<ApiKeySecurityScheme>,

    /// HTTP authentication (Basic, Bearer, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_auth_security_scheme: Option<HttpAuthSecurityScheme>,

    /// OAuth 2.0 authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth2_security_scheme: Option<OAuth2SecurityScheme>,

    /// OpenID Connect authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_id_connect_security_scheme: Option<OpenIdConnectSecurityScheme>,

    /// Mutual TLS authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtls_security_scheme: Option<MutualTlsSecurityScheme>,
}

/// Defines a security scheme using an API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySecurityScheme {
    /// An optional description for the security scheme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The location of the API key: "query", "header", or "cookie".
    pub location: String,

    /// The name of the header, query, or cookie parameter to be used.
    pub name: String,
}

/// Defines a security scheme using HTTP authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpAuthSecurityScheme {
    /// An optional description for the security scheme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The HTTP Authentication scheme (e.g., "Bearer").
    pub scheme: String,

    /// A hint for how the bearer token is formatted (e.g., "JWT").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer_format: Option<String>,
}

/// Defines a security scheme using OAuth 2.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2SecurityScheme {
    /// An optional description for the security scheme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Configuration for the supported OAuth 2.0 flows.
    pub flows: OAuthFlows,

    /// URL to the OAuth2 authorization server metadata (RFC 8414).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth2_metadata_url: Option<String>,
}

/// Defines a security scheme using OpenID Connect.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenIdConnectSecurityScheme {
    /// An optional description for the security scheme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The OpenID Connect Discovery URL.
    pub open_id_connect_url: String,
}

/// Defines a security scheme using mTLS authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutualTlsSecurityScheme {
    /// An optional description for the security scheme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Configuration for supported OAuth 2.0 flows.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlows {
    /// Configuration for the OAuth Authorization Code flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<AuthorizationCodeOAuthFlow>,

    /// Configuration for the OAuth Client Credentials flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_credentials: Option<ClientCredentialsOAuthFlow>,

    /// Deprecated: Use Authorization Code + PKCE instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implicit: Option<ImplicitOAuthFlow>,

    /// Deprecated: Use Authorization Code + PKCE or Device Code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<PasswordOAuthFlow>,

    /// Configuration for the OAuth Device Code flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_code: Option<DeviceCodeOAuthFlow>,
}

/// OAuth 2.0 Authorization Code flow configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationCodeOAuthFlow {
    /// The authorization URL.
    pub authorization_url: String,

    /// The token URL.
    pub token_url: String,

    /// The URL for obtaining refresh tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,

    /// Available scopes for the OAuth2 security scheme.
    pub scopes: HashMap<String, String>,

    /// Indicates if PKCE (RFC 7636) is required.
    #[serde(default)]
    pub pkce_required: bool,
}

/// OAuth 2.0 Client Credentials flow configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCredentialsOAuthFlow {
    /// The token URL.
    pub token_url: String,

    /// The URL for obtaining refresh tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,

    /// Available scopes for the OAuth2 security scheme.
    pub scopes: HashMap<String, String>,
}

/// Deprecated: Use Authorization Code + PKCE instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplicitOAuthFlow {
    /// The authorization URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<String>,

    /// The URL for obtaining refresh tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,

    /// Available scopes.
    #[serde(default)]
    pub scopes: HashMap<String, String>,
}

/// Deprecated: Use Authorization Code + PKCE or Device Code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordOAuthFlow {
    /// The token URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_url: Option<String>,

    /// The URL for obtaining refresh tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,

    /// Available scopes.
    #[serde(default)]
    pub scopes: HashMap<String, String>,
}

/// OAuth 2.0 Device Code flow configuration (RFC 8628).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeOAuthFlow {
    /// The device authorization endpoint URL.
    pub device_authorization_url: String,

    /// The token URL.
    pub token_url: String,

    /// The URL for obtaining refresh tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,

    /// Available scopes for the OAuth2 security scheme.
    pub scopes: HashMap<String, String>,
}

/// Defines the security requirements for an agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityRequirement {
    /// A map of security schemes to the required scopes.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub schemes: HashMap<String, StringList>,
}
