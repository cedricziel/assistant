pub mod adapter_registry;
pub mod bootstrap;
pub mod channel_runner;
pub mod command_registry;
pub(crate) mod compaction;
pub(crate) mod conversation_indexer;
pub mod decision_model;
pub mod feedback_telemetry;
pub mod follow_ups;
pub(crate) mod history;
pub mod interaction;
pub mod interface_runner;
pub mod interface_trait;
pub mod memory_indexer;
pub mod metrics;
pub mod orchestration;
pub mod orchestrator;
pub mod org_context;
pub(crate) mod otel_spans;
pub mod projection;
pub mod scheduler;
pub mod skill_improver;
pub(crate) mod skill_learner;
pub mod space_runtime;
pub mod space_supervisor;
pub mod telemetry;
pub mod title_generator;
pub mod webhook_dispatch;
pub mod worker_plan;

pub use adapter_registry::AdapterRegistry;
pub use bootstrap::spawn_memory_indexer;
pub use channel_runner::ChannelRunner;
pub use command_registry::{CommandContext, CommandRegistry};
pub use feedback_telemetry::{FeedbackTelemetrySink, OtelFeedbackTelemetrySink};
pub use interface_runner::InterfaceRunner;
pub use interface_trait::AssistantInterface;
pub use metrics::MetricsRecorder;
pub use orchestration::{OrchestrationEngine, RecordedSubmitTurn, StubOrchestrationEngine};
pub use orchestrator::{
    CANCELLED_REPLY_PLACEHOLDER, CancelOutcome, Orchestrator, OrchestratorEvent, RunEventRelay,
    SubmitTurn, TURN_CANCELLED_BEFORE_START_MARKER, TURN_CANCELLED_MARKER, TurnLease, TurnMode,
    TurnRegistry, TurnResult,
};
pub use org_context::{OrgConfigAdjust, OrgContext, OrgContexts, build_llm};
pub use otel_spans::{FeedbackLabel, FeedbackTelemetry, start_conversation_context};
pub use scheduler::spawn_scheduler;
pub use space_runtime::{SpaceRuntime, SpaceRuntimes};
pub use space_supervisor::{SpaceRef, SpaceSupervisor, SpaceTaskSpawner};
pub use telemetry::{OtelGuard, init_tracing};
pub use title_generator::{TitleGeneratorWorker, spawn_title_generator_worker};
pub use worker_plan::{BinaryRole, CoreWorkerPlan, CoreWorkerSpec, core_worker_plan};
