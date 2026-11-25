//! Prelude — single-import re-exports for the most-used fixture types and
//! workspace fakes.
//!
//! Test code should prefer
//!
//! ```ignore
//! use assistant_test_support::prelude::*;
//! ```
//!
//! over individual imports. As fakes land in their owning crates in later
//! phases of `openspec/changes/workspace-test-coverage-floor/`, they are
//! re-exported here:
//!
//! - Phase 3 — `FakeClock` from `assistant_core::clock` (present).
//! - Phase 5 — `InMemory*Store` types from `assistant_storage` and
//!   `ScriptedLlmProvider` from `assistant_llm_provider`.
//! - Phase 7 — `StubOrchestrationEngine`, `StubToolDispatcher`,
//!   `InMemorySkillCatalog`.

pub use assistant_core::clock::{Clock, FakeClock, SystemClock};
pub use assistant_core::integrations::{
    InMemoryIntegrationCredentialStore, InMemoryIntegrationStore, IntegrationCredentialStore,
    IntegrationStore,
};
pub use assistant_core::tool::ToolDispatcher;
pub use assistant_llm_provider::{RecordedCall, ScriptedLlmProvider};
pub use assistant_storage::{
    AgentStore, AttachmentStore, CommandEventStore, ConversationEventStore, ConversationStore,
    InMemoryAgentStore, InMemoryAttachmentStore, InMemoryCommandEventStore,
    InMemoryConversationEventStore, InMemoryConversationStore, InMemoryInteractionStore,
    InMemoryMemoryChunkStore, InMemoryMessageBus, InMemoryPersonaSkillAccessStore,
    InMemoryPersonaStore, InMemoryPushSubscriptionStore, InMemoryRefinementsStore,
    InMemoryScheduledTaskStore, InMemorySlackActiveThreadStore, InMemoryWebhookStore,
    InMemoryWorkflowStore, InteractionStore, MemoryChunkStore, PersonaSkillAccessStore,
    PersonaStore, PushSubscriptionStore, RefinementsStore, ScheduledTaskStore,
    SlackActiveThreadStore, WebhookStore, WorkflowStore,
};
pub use assistant_storage::{InMemorySkillCatalog, SkillCatalog};
pub use assistant_tool_executor::{RecordedToolCall, StubToolDispatcher};

pub use crate::fixture::{Fixture, FixtureBuilder};
