//! Authentication and authorization for the assistant.
//!
//! This crate provides:
//! - JWT signing and validation ([`jwt`])
//! - Password hashing with argon2id ([`password`])
//! - OAuth2 server logic ([`oauth2`])
//! - OIDC federation ([`oidc`])
//! - API key management (`api_keys`) *(coming next)*
//! - Axum middleware extractors (`middleware`) *(coming next)*

pub mod jwt;
pub mod oauth2;
pub mod oidc;
pub mod password;
