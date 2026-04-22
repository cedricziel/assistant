//! OpenAI LLM provider with API key and Codex OAuth support.

pub mod oauth;
mod provider;

pub use oauth::OAuthManager;
pub use provider::{OpenAIProvider, OpenAIProviderConfig};
