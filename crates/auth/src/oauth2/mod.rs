//! OAuth2 authorization server logic.
//!
//! Sub-modules are added in PR 3:
//! - [`pkce`] — PKCE challenge/verification
//! - [`server`] — Authorization code grant + refresh tokens
//! - [`device`] — Device code flow
//! - [`clients`] — Dynamic client registration (RFC 7591)
