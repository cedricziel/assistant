//! Authentication and authorization for the assistant.
//!
//! This crate provides:
//! - JWT signing and validation ([`jwt`])
//! - Password hashing with argon2id ([`password`])
//! - OAuth2 server logic ([`oauth2`])
//! - OIDC federation ([`oidc`])
//! - Scoped API key management ([`api_keys`])
//! - Axum middleware extractors ([`middleware`])
//! - [`AuthProvider`](assistant_core::auth::AuthProvider) implementations ([`providers`])

pub mod api_keys;
pub mod bootstrap;
pub mod jwt;
pub mod middleware;
pub mod oauth2;
pub mod oidc;
pub mod password;
pub mod providers;
