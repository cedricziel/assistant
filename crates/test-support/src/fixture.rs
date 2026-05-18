//! Fixture composition for `assistant` workspace tests.
//!
//! [`FixtureBuilder`] wires together the in-memory persistence fakes,
//! [`FakeClock`], [`ScriptedLlmProvider`], and [`InMemoryMessageBus`] so
//! consumer tests can call
//!
//! ```ignore
//! use assistant_test_support::prelude::*;
//! let fx = FixtureBuilder::new().build().await;
//! ```
//!
//! and get a ready-to-use fixture without booting SQLite. Override
//! individual fakes with the `with_*` builders.
//!
//! Wiring grows as later openspec/changes/workspace-test-coverage-floor
//! phases land:
//! - Phase 3 — `with_clock(FakeClock)` (present)
//! - Phase 5 — `with_canned_llm_responses`, in-memory store accessors
//!   (present)
//! - Phase 7 — `Arc<dyn OrchestrationEngine>` once those traits land

use std::sync::Arc;

use assistant_core::LlmProvider;
use assistant_core::MessageBus;
use assistant_core::clock::{Clock, FakeClock, SystemClock};
use assistant_llm_provider::ScriptedLlmProvider;
use assistant_storage::{
    AgentStore, AttachmentStore, CommandEventStore, ConversationEventStore, ConversationStore,
    InMemoryAgentStore, InMemoryAttachmentStore, InMemoryCommandEventStore,
    InMemoryConversationEventStore, InMemoryConversationStore, InMemoryLogStore,
    InMemoryMemoryChunkStore, InMemoryMessageBus, InMemoryMetricsStore,
    InMemoryPersonaSkillAccessStore, InMemoryPersonaStore, InMemoryPushSubscriptionStore,
    InMemoryRefinementsStore, InMemoryScheduledTaskStore, InMemorySlackActiveThreadStore,
    InMemoryTraceStore, InMemoryWebhookStore, InMemoryWorkflowStore, LogStore, MemoryChunkStore,
    MetricsStore, PersonaSkillAccessStore, PersonaStore, PushSubscriptionStore, RefinementsStore,
    ScheduledTaskStore, SlackActiveThreadStore, TraceStore, WebhookStore, WorkflowStore,
};

/// Builder for a composed [`Fixture`].
///
/// Default-constructs every fake to a fresh in-memory state with a
/// `SystemClock`-backed clock and a benign empty [`ScriptedLlmProvider`].
/// Override per-field with `with_*` methods.
pub struct FixtureBuilder {
    clock: Arc<dyn Clock>,
    llm: Arc<dyn LlmProvider>,
}

impl FixtureBuilder {
    /// Start a new builder with all defaults.
    pub fn new() -> Self {
        Self {
            clock: Arc::new(SystemClock),
            llm: Arc::new(ScriptedLlmProvider::new()),
        }
    }

    /// Inject a custom [`Clock`] implementation. Tests pass
    /// `Arc::new(FakeClock::new(...))` to drive virtual time.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Use a [`FakeClock`] anchored at the given epoch second. Convenience
    /// wrapper around `with_clock(Arc::new(FakeClock::new(...)))`.
    pub fn with_fake_clock_at(mut self, epoch_secs: i64) -> Self {
        let dt = chrono::DateTime::from_timestamp(epoch_secs, 0).unwrap_or_else(chrono::Utc::now);
        self.clock = Arc::new(FakeClock::new(dt));
        self
    }

    /// Replace the LLM provider. Use this to inject a
    /// [`ScriptedLlmProvider`] with canned responses for the test.
    pub fn with_llm(mut self, llm: Arc<dyn LlmProvider>) -> Self {
        self.llm = llm;
        self
    }

    /// Convenience: install a [`ScriptedLlmProvider`] pre-seeded with the
    /// given canned responses.
    pub fn with_canned_llm_responses(
        mut self,
        responses: Vec<assistant_core::LlmResponse>,
    ) -> Self {
        self.llm = Arc::new(ScriptedLlmProvider::new().with_canned_responses(responses));
        self
    }

    /// Materialise the [`Fixture`].
    ///
    /// All in-memory stores are constructed fresh. They share no state
    /// across `build()` calls.
    pub async fn build(self) -> Fixture {
        Fixture {
            clock: self.clock,
            llm: self.llm,
            message_bus: Arc::new(InMemoryMessageBus::new()),
            conversation_store: Arc::new(InMemoryConversationStore::new()),
            trace_store: Arc::new(InMemoryTraceStore::new()),
            log_store: Arc::new(InMemoryLogStore::new()),
            attachment_store: Arc::new(InMemoryAttachmentStore::new()),
            push_subscription_store: Arc::new(InMemoryPushSubscriptionStore::new()),
            command_event_store: Arc::new(InMemoryCommandEventStore::new()),
            slack_active_thread_store: Arc::new(InMemorySlackActiveThreadStore::new()),
            webhook_store: Arc::new(InMemoryWebhookStore::new()),
            agent_store: Arc::new(InMemoryAgentStore::new()),
            scheduled_task_store: Arc::new(InMemoryScheduledTaskStore::new()),
            conversation_event_store: Arc::new(InMemoryConversationEventStore::new()),
            metrics_store: Arc::new(InMemoryMetricsStore::new()),
            refinements_store: Arc::new(InMemoryRefinementsStore::new()),
            persona_store: Arc::new(InMemoryPersonaStore::new()),
            workflow_store: Arc::new(InMemoryWorkflowStore::new()),
            memory_chunk_store: Arc::new(InMemoryMemoryChunkStore::new()),
            persona_skill_access_store: Arc::new(InMemoryPersonaSkillAccessStore::new()),
        }
    }
}

impl Default for FixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Composed test fixture.
///
/// All stores are exposed as `Arc<dyn TraitName>` so consumer code can
/// freely `Arc::clone(...)` them into any code path that takes the
/// matching trait.
pub struct Fixture {
    pub clock: Arc<dyn Clock>,
    pub llm: Arc<dyn LlmProvider>,
    pub message_bus: Arc<dyn MessageBus>,
    pub conversation_store: Arc<dyn ConversationStore>,
    pub trace_store: Arc<dyn TraceStore>,
    pub log_store: Arc<dyn LogStore>,
    pub attachment_store: Arc<dyn AttachmentStore>,
    pub push_subscription_store: Arc<dyn PushSubscriptionStore>,
    pub command_event_store: Arc<dyn CommandEventStore>,
    pub slack_active_thread_store: Arc<dyn SlackActiveThreadStore>,
    pub webhook_store: Arc<dyn WebhookStore>,
    pub agent_store: Arc<dyn AgentStore>,
    pub scheduled_task_store: Arc<dyn ScheduledTaskStore>,
    pub conversation_event_store: Arc<dyn ConversationEventStore>,
    pub metrics_store: Arc<dyn MetricsStore>,
    pub refinements_store: Arc<dyn RefinementsStore>,
    pub persona_store: Arc<dyn PersonaStore>,
    pub workflow_store: Arc<dyn WorkflowStore>,
    pub memory_chunk_store: Arc<dyn MemoryChunkStore>,
    pub persona_skill_access_store: Arc<dyn PersonaSkillAccessStore>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::LlmResponse;
    use uuid::Uuid;

    #[tokio::test]
    async fn build_creates_empty_fixture() {
        let fx = FixtureBuilder::new().build().await;
        let convs = fx.conversation_store.list_conversations().await.unwrap();
        assert!(convs.is_empty());
        let traces = fx.trace_store.list_recent(10).await.unwrap();
        assert!(traces.is_empty());
    }

    #[tokio::test]
    async fn build_with_canned_llm_responses_replays_them() {
        let fx = FixtureBuilder::new()
            .with_canned_llm_responses(vec![LlmResponse::FinalAnswer(
                "scripted reply".into(),
                Default::default(),
            )])
            .build()
            .await;
        let resp = fx.llm.chat("sys", &[], &[]).await.unwrap();
        match resp {
            LlmResponse::FinalAnswer(text, _) => assert_eq!(text, "scripted reply"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test]
    async fn build_with_fake_clock_at_anchors_time() {
        let fx = FixtureBuilder::new()
            .with_fake_clock_at(1_700_000_000)
            .build()
            .await;
        let t = fx.clock.now();
        assert_eq!(t.timestamp(), 1_700_000_000);
    }

    #[tokio::test]
    async fn fixture_stores_are_independent_per_build() {
        let fx1 = FixtureBuilder::new().build().await;
        let fx2 = FixtureBuilder::new().build().await;
        fx1.conversation_store
            .create_conversation(Some("only in fx1"))
            .await
            .unwrap();
        let in_fx2 = fx2.conversation_store.list_conversations().await.unwrap();
        assert!(in_fx2.is_empty());
    }

    #[tokio::test]
    async fn message_bus_publish_and_claim_roundtrip() {
        use assistant_core::PublishRequest;
        let fx = FixtureBuilder::new().build().await;
        let id = fx
            .message_bus
            .publish(PublishRequest {
                topic: "test.topic".into(),
                payload: serde_json::json!({"x": 1}),
                user_id: None,
                agent_id: None,
                conversation_id: None,
                interface: None,
                reply_to: None,
                correlation_id: None,
                causation_id: None,
                batch_id: None,
            })
            .await
            .unwrap();
        let claimed = fx
            .message_bus
            .claim("test.topic", "test-worker")
            .await
            .unwrap()
            .expect("claim should return the published message");
        assert_eq!(claimed.id, id);
    }

    #[tokio::test]
    async fn agent_store_create_and_get() {
        let fx = FixtureBuilder::new().build().await;
        let conv = Uuid::new_v4();
        fx.agent_store
            .create("agent-1", None, &conv.to_string(), "child", "do task", 1)
            .await
            .unwrap();
        let got = fx.agent_store.get("agent-1").await.unwrap().unwrap();
        assert_eq!(got.task, "do task");
    }
}
