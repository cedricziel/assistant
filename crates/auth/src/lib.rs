//! Authentication and authorization for the assistant.
//!
//! This crate provides:
//! - JWT signing and validation ([`jwt`])
//! - Password hashing with argon2id ([`password`])
//! - OAuth2 server logic ([`oauth2`])
//! - API key management (`api_keys`) *(coming in PR 4)*
//! - OIDC federation (`oidc`) *(coming in PR 4)*
//! - Axum middleware extractors (`middleware`) *(coming in PR 4)*

pub mod jwt;
pub mod oauth2;
pub mod password;
