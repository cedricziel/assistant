/// Crate-internal `serde` default helper used by submodule structs that
/// have `enabled: bool` fields defaulting to true. Kept here so submodules
/// can share it via `super::default_true`.
#[doc(hidden)]
pub(crate) fn default_true() -> bool {
    true
}

// ── Conversation types ────────────────────────────────────────────────────────
//
// Moved to the `conversation` submodule. Import via
// `use assistant_core::types::conversation::{Message, MessageRole, ...};`
pub mod conversation;

// ── Top-level assistant config + agent context ────────────────────────────────
//
// Moved to the `agent` submodule. Import via
// `use assistant_core::types::agent::{AssistantConfig, AgentConfig};`
pub mod agent;

// ── Channel adapter configuration ─────────────────────────────────────────────
//
// Moved to the `channels` submodule. Import via
// `use assistant_core::types::channels::{SlackConfig, MatrixConfig, ...};`
pub mod channels;

// ── Transcription / TTS configuration ─────────────────────────────────────────
//
// Moved to the `transcription` submodule. Import via
// `use assistant_core::types::transcription::{TranscriptionConfig, TtsConfig, ...};`
pub mod transcription;

// ── LLM / embedding configuration ─────────────────────────────────────────────
//
// Moved to the `llm` submodule. Import via
// `use assistant_core::types::llm::{LlmConfig, AnthropicOptions, ...};`
pub mod llm;

// ── Storage / message-bus configuration ───────────────────────────────────────
//
// Moved to the `storage` submodule. Import via
// `use assistant_core::types::storage::{StorageConfig, BusConfig, BusKind};`
pub mod storage;

// ── Skills / MCP / mirror configuration ───────────────────────────────────────
//
// Moved to the `skills_mcp` submodule. Import via
// `use assistant_core::types::skills_mcp::{SkillsConfig, McpConfig, ...};`
pub mod skills_mcp;

// ── Observability / telemetry configuration ──────────────────────────────────
//
// Moved to the `observability` submodule. Import via
// `use assistant_core::types::observability::{OtelExporter, ObservabilityConfig, ...};`
pub mod observability;

// ── Per-feature runtime configuration ─────────────────────────────────────────
//
// Moved to the `features` submodule. Import via
// `use assistant_core::types::features::{MemoryConfig, CompactionConfig, ...};`
pub mod features;
