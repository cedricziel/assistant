//! LLM abstractions: traits, types, and streaming primitives.
//!
//! This module is the canonical location for all LLM-related type
//! definitions and trait contracts.  Provider *implementations* live in
//! `assistant-llm-provider`; this module only contains the abstractions
//! they implement.

pub mod embedding;
pub mod provider;
pub mod stream_chunk;
pub mod tool_spec;
pub mod types;
