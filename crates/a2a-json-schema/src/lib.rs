//! A2A protocol JSON Schema definitions and Rust types.
//!
//! This crate provides the Rust type definitions for the Agent-to-Agent (A2A)
//! communication protocol (`lf.a2a.v1`), with full `serde` support for
//! JSON serialization matching the canonical JSON Schema.
//!
//! Schema files are located under `schema/`.
//!
//! # Modules
//!
//! - [`types`] -- Core domain types (Task, Message, Part, Artifact, etc.)
//! - [`agent_card`] -- Agent discovery manifest (AgentCard, AgentSkill, etc.)
//! - [`security`] -- Security scheme definitions (OAuth2, API key, mTLS, etc.)
//! - [`requests`] -- RPC request types
//! - [`responses`] -- RPC response types

pub mod agent_card;
pub mod requests;
pub mod responses;
pub mod security;
pub mod types;

// Re-export the most commonly used types at crate root.
pub use agent_card::{
    AgentCapabilities, AgentCard, AgentCardSignature, AgentExtension, AgentInterface,
    AgentProvider, AgentSkill,
};
pub use requests::*;
pub use responses::*;
pub use security::SecurityScheme;
pub use types::{
    Artifact, Message, Part, PushNotificationConfig, Role, SendMessageConfiguration, Task,
    TaskArtifactUpdateEvent, TaskState, TaskStatus, TaskStatusUpdateEvent,
};
