//! OpenAI LLM provider with API key and Codex OAuth support.
//!
//! Supports two authentication modes:
//! - **API key** (`OPENAI_API_KEY`) — standard pay-per-use billing.
//! - **OAuth PKCE** (Codex subscription) — authenticates via ChatGPT sign-in
//!   so usage is billed against the user's Codex plan quota.

mod oauth;
mod provider;

pub use oauth::OAuthManager;
pub use provider::{OpenAIProvider, OpenAIProviderConfig};
