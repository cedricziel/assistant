//! Ollama LLM provider for the assistant runtime.
//!
//! This crate exposes [`OllamaProvider`], which implements
//! [`assistant_llm::LlmProvider`] backed by the Ollama `/api/chat` endpoint.
//!
//! ## Usage
//!
//! ```no_run
//! use std::sync::Arc;
//! use assistant_provider_ollama::OllamaProvider;
//! use assistant_core::LlmConfig;
//!
//! let provider: Arc<dyn assistant_llm::LlmProvider> =
//!     Arc::new(OllamaProvider::from_llm_config(&LlmConfig::default()).unwrap());
//! ```

mod provider;

pub use provider::{OllamaConfig, OllamaProvider};
