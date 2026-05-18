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
pub use assistant_storage::{
    AgentStore, AttachmentStore, CommandEventStore, ConversationEventStore, ConversationStore,
    InMemoryAgentStore, InMemoryAttachmentStore, InMemoryCommandEventStore,
    InMemoryConversationEventStore, InMemoryConversationStore, InMemoryLogStore,
    InMemoryPushSubscriptionStore, InMemoryScheduledTaskStore, InMemorySlackActiveThreadStore,
    InMemoryTraceStore, InMemoryWebhookStore, LogStore, PushSubscriptionStore, ScheduledTaskStore,
    SlackActiveThreadStore, TraceStore, WebhookStore,
};

pub use crate::fixture::{Fixture, FixtureBuilder};
