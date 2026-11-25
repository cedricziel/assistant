//! Authentication and authorization for the assistant.
//!
//! This crate provides:
//! - JWT signing and validation ([`jwt`])
//! - Password hashing with argon2id ([`password`])
//! - OAuth2 server logic ([`oauth2`])
//! - OIDC federation ([`oidc`])
//! - Scoped API key management ([`api_keys`])
//! - Shared auth state and scope checks ([`middleware`])
//! - Ending a user's sessions ([`session`])

pub mod api_keys;
pub mod bootstrap;
pub mod constant_time;
pub mod jwt;
pub mod middleware;
pub mod oauth2;
pub mod oidc;
pub mod password;
pub mod session;
