//! Orchestrator — the main turn-processing loop that wires together the
//! LLM client, tool executor, and skill registry.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::bus_messages::TurnAnchor;
use assistant_core::llm::call_context::LlmCallContext;
use assistant_core::types::conversation::{
    ExecutionContext, Interface, Message, MessageRole, TurnIdentity,
};
use assistant_core::{
    Attachment, AttachmentMeta, ChatHistoryMessage, ChatRole, ContentBlock, LlmProvider,
    LlmResponse, MemoryLoader, MessageBus, StreamChunk, ToolHandler, ToolSpec, TurnScope,
    TurnTiming, TurnToolSource, is_resizable_mime_type, is_supported_mime_type, is_text_mime_type,
    strip_html_comments,
};
use assistant_storage::{
    AttachmentStore as _AttachmentStoreTrait, SkillRegistry, StorageLayer,
    conversations::ConversationStore,
};
use assistant_tool_executor::{ToolExecutor, ToolOverlay};
use assistant_transcription::{AudioStore, TtsProvider};
use async_trait::async_trait;
use opentelemetry::{
    Context as OtelContext, KeyValue, global,
    trace::{Span as _, Status as OtelStatus, TraceContextExt, Tracer as _},
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, debug, info, info_span, warn};
use uuid::Uuid;

use crate::history::{FinalAnswerMeta, TurnHistory, persist_final_answer};
pub(crate) use turn_store::TurnStore;
use turn_timing::{CallMarks, TurnTimer};

// ── Submodules ────────────────────────────────────────────────────────────────

mod branch;
mod cancelled_reply;
mod dispatch;
mod prompt;
mod relay;
mod surface_hook;
use self::surface_hook::{SurfaceCall, SurfaceTurn};
pub mod stream_event;
mod turn_anchor;
mod turn_cancellations;
mod turn_control;
mod turn_registry;
mod turn_store;
mod turn_timing;

mod subagent;
mod submit;
mod worker;

#[cfg(test)]
mod tests;

pub use branch::{BRANCH_CANCEL_WAIT, BranchConflict};
pub use cancelled_reply::{CANCELLED_REPLY_PLACEHOLDER, CancelledTurn};
pub use relay::RunEventRelay;
pub use stream_event::OrchestratorEvent;
pub use submit::SubmitTurn;
pub use turn_registry::{TurnLease, TurnMode, TurnRegistry};

// ── Public types ──────────────────────────────────────────────────────────────

/// Which answers a [`ConfirmationCallback`] may offer the user.
///
/// The once answers (`AllowOnce`, `RejectOnce`) are always available; the
/// "always" answers persist a rule and are offered only when the request
/// carries a suggested rule to save (and no configured ask rule forbids it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ApprovalOptions {
    /// `AllowAlways` may be offered.
    pub allow_always: bool,
    /// `RejectAlways` may be offered.
    pub reject_always: bool,
}

impl ApprovalOptions {
    /// Offer only `AllowOnce` and `RejectOnce`.
    pub const fn once_only() -> Self {
        Self {
            allow_always: false,
            reject_always: false,
        }
    }

    /// Offer both "always" answers.
    pub const fn with_always() -> Self {
        Self {
            allow_always: true,
            reject_always: true,
        }
    }

    /// Whether neither "always" answer may be offered.
    pub const fn is_once_only(&self) -> bool {
        !self.allow_always && !self.reject_always
    }
}

/// One tool call awaiting a human decision.
pub struct ConfirmationRequest<'a> {
    pub tool_name: &'a str,
    pub params: &'a serde_json::Value,
    pub ctx: &'a ExecutionContext,
    /// Provider id of the call, when the turn streams one.
    pub tool_call_id: Option<&'a str>,
    /// Why the permission pipeline could not decide on its own.
    pub reason: &'a str,
    /// Which answers may be offered.
    pub options: ApprovalOptions,
    /// The rule an "always" answer would save, when one is offered.
    pub suggested_rule: Option<&'a str>,
    /// Stream sink for the turn, when it has one.
    pub event_sink: Option<&'a mpsc::Sender<OrchestratorEvent>>,
}

/// A human's (or a timeout's) answer to a [`ConfirmationRequest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationAnswer {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
    /// Nobody answered before the request timed out.
    Expired,
    /// The turn ended or was cancelled while the request was open.
    Cancelled,
}

/// Callback trait for requesting user confirmation before executing a tool.
///
/// A runtime without a callback denies every call the pipeline leaves
/// undecided.
#[async_trait]
pub trait ConfirmationCallback: Send + Sync {
    /// Ask for a decision on `req`. May wait for as long as the human takes.
    async fn confirm(&self, req: ConfirmationRequest<'_>) -> ConfirmationAnswer;
}

/// Outcome of [`Orchestrator::cancel_turn`].
///
/// Distinguishes "we found and triggered a token" from "no in-flight turn
/// matched the request_id" — the latter is not an error, just an authoritative
/// signal to the caller that the turn already finished, errored, or was never
/// registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelOutcome {
    /// A cancellation token was registered for this `request_id` and has been
    /// triggered. The turn worker will abort on its next yield point and
    /// `submit` will return an error containing the cancellation marker.
    Cancelled,
    /// No matching turn was registered — either the turn already finished
    /// (success or error), was never started, or used a different request_id.
    NotFound,
}

/// Error message fragment that appears in [`anyhow::Error`] when a turn was
/// aborted via [`Orchestrator::cancel_turn`]. Callers (e.g. the SSE handler
/// in `assistant-web-ui`) match on this substring to emit a final
/// `agent_error` event with the structured cancellation reason.
pub const TURN_CANCELLED_MARKER: &str = "turn_cancelled";

/// Additional error message fragment present, beside [`TURN_CANCELLED_MARKER`],
/// when the turn was cancelled before any worker claimed it. Nothing of such a
/// turn ran: the user message was never added to the conversation history.
pub const TURN_CANCELLED_BEFORE_START_MARKER: &str = "cancelled_before_start";

/// How often a worker refreshes the bus claim of the turn it is running.
///
/// Bus claims are reaped, and NATS `ack_wait` expires, after 5 minutes; a
/// 60 second heartbeat keeps a long tool run or a turn waiting on a human
/// from being redelivered to a second worker.
pub const DEFAULT_BUS_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

/// How long a worker defers a turn whose conversation is held by an exclusive
/// turn. Each deferral is a redelivery, and the bus gives up after
/// `MAX_TURN_REDELIVERIES`, so this must stay long enough for one branching
/// turn to finish.
pub const DEFAULT_TURN_DEFER_DELAY: Duration = Duration::from_secs(15);

/// The result of a single orchestrator turn.
pub struct TurnResult {
    /// The assistant's final answer to the user.
    pub answer: String,
    /// File attachments collected from tool outputs during the turn.
    ///
    /// Interfaces should deliver these to the user (e.g. save to disk in the
    /// CLI, upload in Slack/Mattermost).
    pub attachments: Vec<Attachment>,
    /// IDs of attachments persisted to the [`AttachmentStore`] during the turn.
    ///
    /// These are the durable references that can be passed on the bus and
    /// used by interfaces to load bytes for delivery.
    pub attachment_ids: Vec<Uuid>,
    /// The UUID of the persisted assistant message in the database.
    /// `None` when the message could not be saved or the ID was unavailable.
    pub message_id: Option<Uuid>,
    /// `true` if any tool dispatch in this turn returned an `Err(...)`.
    /// Used to gate post-turn skill-learner evaluation.
    pub had_errors: bool,
    /// How long the turn took. `None` when the turn was not measured.
    pub timing: Option<TurnTiming>,
}

// ── Internal enums ────────────────────────────────────────────────────────────

/// Outcome of processing an `end_turn` tool call.
pub(crate) enum EndTurnOutcome {
    /// `end_turn` was called alongside real tool calls — deferred.
    Deferred,
    /// `end_turn` rejected because a reply tool is available but not yet used.
    Rejected,
    /// `end_turn` accepted; the turn is complete.
    Accepted,
}

/// Outcome of processing a `FinalAnswer` from the LLM when extension tools
/// are active.
pub(crate) enum FinalAnswerOutcome {
    Done(TurnResult),
    Retry,
}

/// Outcome of dispatching a single global (executor) tool call.
pub(crate) enum DispatchOutcome {
    /// The permission gate (or the user, via confirmation) denied the call.
    Denied,
    /// The tool was executed and its result finalized.
    Executed { had_error: bool },
}

/// Per-conversation extension tool registration consumed by the worker.
///
/// Interfaces (Slack, Mattermost) register their per-turn tools and
/// attachments before publishing a `TurnRequest` to the bus.  The worker
/// removes the registration when processing the request.
#[derive(Clone)]
pub(crate) struct ExtensionRegistration {
    pub(crate) tools: Vec<Arc<dyn ToolHandler>>,
    pub(crate) attachments: Vec<ContentBlock>,
}

// ── Built-in extension tools ──────────────────────────────────────────────────

/// Build the `end_turn` ToolSpec that `run_turn_with_tools` always injects.
///
/// The tool carries no real handler — the orchestrator loop detects it by name
/// and exits cleanly.  Exposing it as a proper tool gives the LLM a first-class,
/// typed way to signal "I'm done" without having to return a plain FinalAnswer.
fn end_turn_spec() -> ToolSpec {
    ToolSpec {
        name: "end_turn".to_string(),
        description: "Signal that this turn is complete. Call this once you have sent your reply \
             (or decided no reply is needed). The `reason` field is optional and used for \
             logging only."
            .to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "reason": {
                    "type": "string",
                    "description": "Brief reason the turn is ending (e.g. \"replied\", \"no reply needed\"). Used for logging only."
                }
            }
        }),
        is_mutating: false,
        requires_confirmation: false,
        requires_rich_client: false,
    }
}

/// Parse an interface string back to the [`Interface`] enum.
///
/// Matches the `Debug` format that the codebase uses for serialisation
/// (e.g. `"Cli"`, `"Slack"`).  Falls back to [`Interface::Cli`] for
/// unknown values.
pub(crate) fn parse_interface(s: &str) -> Interface {
    match s.to_lowercase().as_str() {
        "cli" => Interface::Cli,
        "signal" => Interface::Signal,
        "mcp" => Interface::Mcp,
        "slack" => Interface::Slack,
        "mattermost" => Interface::Mattermost,
        "nextcloud" => Interface::Nextcloud,
        "matrix" => Interface::Matrix,
        "web" => Interface::Web,
        "scheduler" => Interface::Scheduler,
        _ => Interface::Cli,
    }
}

// ── Orchestrator ──────────────────────────────────────────────────────────────

/// Drives the tool-calling loop for a single conversation turn.
///
/// Each call to [`run_turn`] performs the following high-level algorithm:
///
/// 1. Ensure a conversation row exists in SQLite.
/// 2. Persist the user message.
/// 3. Load all registered tool specs from the executor.
/// 4. Repeatedly call the LLM until it returns a `FinalAnswer` or the
///    iteration limit is reached.
/// 5. For each `ToolCall` response: optionally confirm with the user,
///    execute the tool, emit an OpenTelemetry span, and append an
///    `OBSERVATION` to the conversation history.
/// 6. Persist the final assistant message and return [`TurnResult`].
pub struct Orchestrator {
    /// The LLM provider used for chat and embeddings.
    pub llm: Arc<dyn LlmProvider>,
    pub(crate) storage: Arc<StorageLayer>,
    pub(crate) executor: Arc<ToolExecutor>,
    pub(crate) registry: Arc<SkillRegistry>,
    /// Durable message bus for decoupled inter-component communication.
    pub(crate) bus: Arc<dyn MessageBus>,
    pub(crate) max_iterations: usize,
    pub(crate) confirmation_callback: Option<Arc<dyn ConfirmationCallback>>,
    /// Gate evaluated before every tool call (static rules, decision model).
    pub(crate) permission_policy: assistant_core::permission::PermissionPolicy,
    /// Memory loader used to rebuild the system prompt at the start of every
    /// turn so that writes made by memory tools are reflected immediately.
    pub(crate) memory_loader: MemoryLoader,
    /// When true, record full message content on LLM spans (PII-sensitive).
    pub(crate) trace_content: bool,
    /// Per-conversation event sinks for streaming turns dispatched through
    /// the bus.  Consumed (removed) by the worker when processing.
    pub(crate) token_sinks: tokio::sync::RwLock<HashMap<Uuid, mpsc::Sender<OrchestratorEvent>>>,
    /// Per-conversation extension tool registrations for interface-specific
    /// turns dispatched through the bus.  Consumed by the worker.
    pub(crate) extension_registrations: tokio::sync::RwLock<HashMap<Uuid, ExtensionRegistration>>,
    /// Cancellation tokens for running subagents, keyed by agent ID.
    pub(crate) agent_cancellations: tokio::sync::RwLock<HashMap<String, CancellationToken>>,
    /// Cancellation tokens for in-flight bus turns, keyed by request ID
    /// (the `batch_id` / `correlation_id` set by `submit`).
    /// When `submit` times out it cancels the token so the worker
    /// drops the in-progress turn future rather than running to completion.
    /// Also remembers requests cancelled before a worker claimed them, so the
    /// worker skips them.
    pub(crate) turn_cancellations: tokio::sync::RwLock<turn_cancellations::TurnCancellations>,
    /// The turns running in this process, per conversation.
    pub(crate) turn_registry: TurnRegistry,
    /// Where pending interactions of a turn are recorded. Branch operations
    /// read it to tell a turn waiting for the user from one still working.
    pub(crate) interactions: Arc<dyn assistant_storage::InteractionStore>,
    /// How long the worker defers a turn that claims a conversation held by an
    /// exclusive (branching) turn. Tests shorten it with `with_turn_defer_delay`.
    pub(crate) turn_defer_delay: Duration,
    /// OTel metric instruments for GenAI and operational metrics.
    pub(crate) metrics: crate::MetricsRecorder,
    /// Active assistant agent ID for memory/workspace conversation scoping.
    pub(crate) agent_id: String,
    /// Registry of live channel adapters, used by the scheduler to inject
    /// platform tools for scheduler-originated turns.
    pub adapter_registry: crate::AdapterRegistry,
    /// Context compaction configuration.
    pub(crate) compaction_config: assistant_core::types::features::CompactionConfig,
    /// How long `submit` waits for the worker to return a result before
    /// timing out. Defaults to 10 800 s (3 h). Configurable per persona via
    /// `with_submit_timeout`.
    pub(crate) submit_timeout_secs: u64,
    /// How often the worker refreshes the bus claim of the turn it is
    /// running. Must stay well below the bus reap / `ack_wait` timeout
    /// (5 min). Tests shorten it with `with_bus_heartbeat_interval`.
    pub(crate) bus_heartbeat_interval: Duration,
    /// Optional audio store for retrieving TTS blobs produced by the
    /// `voice-response` tool.  When present, synthesised audio is appended
    /// to `TurnResult::attachments` so channel adapters can deliver it.
    pub(crate) audio_store: Option<Arc<AudioStore>>,
    /// Speaks the answer of a voice turn that came back without audio.
    tts: Option<Arc<dyn TtsProvider>>,
    /// The format spoken replies are delivered in (`[tts] format`).
    reply_format: assistant_core::types::transcription::TtsFormat,
    /// Per-conversation settings from slash commands (`/model`). Shared with
    /// the command contexts of every interface.
    pub(crate) conversation_configs:
        Arc<tokio::sync::RwLock<HashMap<Uuid, assistant_core::ConversationConfig>>>,
    /// Providers built for `/model` overrides, by model name.
    pub(crate) model_providers: std::sync::Mutex<HashMap<String, Arc<dyn LlmProvider>>>,
    /// Provider settings an override model is combined with.
    pub(crate) llm_config: assistant_core::types::llm::LlmConfig,
    /// Log of tool calls made while a skill was active; feeds skill
    /// self-improvement.
    pub(crate) skill_executions: Arc<dyn assistant_storage::SkillExecutionStore>,
    /// Learning configuration for autonomous skill creation and improvement.
    pub(crate) learning_config: assistant_core::types::features::LearningConfig,
    /// Clock used for timestamp generation on persisted records and for
    /// scheduler-related comparisons. Defaults to `Arc::new(SystemClock)`;
    /// tests inject a `FakeClock` via [`Self::with_clock`].
    pub(crate) clock: Arc<dyn assistant_core::clock::Clock>,
    /// Where each turn gets the tools only it may see, with the space it is
    /// asked for.
    pub(crate) turn_tool_source: Option<TurnToolBinding>,
    /// Where `render-surface` results are persisted (see `surface_hook`).
    pub(crate) surface_store: Arc<dyn assistant_storage::SurfaceStore>,
}

/// A [`TurnToolSource`] bound to the space this orchestrator runs.
pub(crate) struct TurnToolBinding {
    source: Arc<dyn TurnToolSource>,
    org_slug: String,
    space_slug: String,
}

impl Orchestrator {
    /// Create a new orchestrator.
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        storage: Arc<StorageLayer>,
        executor: Arc<ToolExecutor>,
        registry: Arc<SkillRegistry>,
        bus: Arc<dyn MessageBus>,
        config: &assistant_core::types::agent::OrgConfig,
    ) -> Self {
        let memory_loader = MemoryLoader::new(config);
        memory_loader.ensure_defaults();

        let compaction_config = config.compaction.clone();

        let skill_executions = Arc::new(storage.skill_execution_store());
        let interactions = Arc::new(storage.interaction_store());
        let surface_store = Arc::new(storage.surface_store());

        Self {
            llm,
            storage,
            executor,
            registry,
            bus,
            max_iterations: config.llm.max_iterations,
            confirmation_callback: None,
            permission_policy: Self::build_permission_policy(&config.permissions, &config.llm),
            memory_loader,
            trace_content: config.observability.trace_content,
            token_sinks: tokio::sync::RwLock::new(HashMap::new()),
            extension_registrations: tokio::sync::RwLock::new(HashMap::new()),
            agent_cancellations: tokio::sync::RwLock::new(HashMap::new()),
            turn_cancellations: tokio::sync::RwLock::new(Default::default()),
            turn_registry: TurnRegistry::default(),
            interactions,
            turn_defer_delay: DEFAULT_TURN_DEFER_DELAY,
            metrics: crate::MetricsRecorder::new(),
            agent_id: config.agent.id.clone(),
            compaction_config,
            submit_timeout_secs: 10_800,
            bus_heartbeat_interval: DEFAULT_BUS_HEARTBEAT_INTERVAL,
            adapter_registry: crate::AdapterRegistry::new(),
            audio_store: None,
            tts: None,
            reply_format: config
                .tts
                .as_ref()
                .and_then(|t| t.format)
                .unwrap_or_default(),
            conversation_configs: Arc::default(),
            model_providers: std::sync::Mutex::default(),
            llm_config: config.llm.clone(),
            skill_executions,
            learning_config: config.learning.clone(),
            clock: Arc::new(assistant_core::clock::SystemClock),
            turn_tool_source: None,
            surface_store,
        }
    }

    /// Persist surfaces in `store` instead of the space database. Tests use
    /// this to inspect rows without SQLite.
    pub fn with_surface_store(mut self, store: Arc<dyn assistant_storage::SurfaceStore>) -> Self {
        self.surface_store = store;
        self
    }

    /// Give every turn the tools `source` returns for `(org_slug, space_slug)`
    /// and the turn's user. They exist for that turn only, never shadow a
    /// registered tool, and go through the same permission gate.
    pub fn with_turn_tool_source(
        mut self,
        source: Arc<dyn TurnToolSource>,
        org_slug: impl Into<String>,
        space_slug: impl Into<String>,
    ) -> Self {
        self.turn_tool_source = Some(TurnToolBinding {
            source,
            org_slug: org_slug.into(),
            space_slug: space_slug.into(),
        });
        self
    }

    /// The tools this turn alone may use.
    async fn turn_overlay(&self, identity: &TurnIdentity) -> ToolOverlay {
        let Some(binding) = &self.turn_tool_source else {
            return ToolOverlay::default();
        };
        let scope = TurnScope {
            org_slug: &binding.org_slug,
            space_slug: &binding.space_slug,
            user_id: identity.user_id.as_ref(),
        };
        ToolOverlay::new(binding.source.tools_for_turn(&scope).await)
    }

    /// Override the `submit` timeout for this orchestrator instance.
    ///
    /// Call this after [`Orchestrator::new`] to apply a per-persona timeout
    /// loaded from the database.  The value must be > 0.
    pub fn with_submit_timeout(mut self, secs: u64) -> Self {
        self.submit_timeout_secs = secs.max(1);
        self
    }

    /// Override how often the worker refreshes the bus claim of a running turn.
    pub fn with_bus_heartbeat_interval(mut self, interval: Duration) -> Self {
        self.bus_heartbeat_interval = interval;
        self
    }

    /// Override how long the worker defers a turn held back by an exclusive lease.
    pub fn with_turn_defer_delay(mut self, delay: Duration) -> Self {
        self.turn_defer_delay = delay;
        self
    }

    /// Inject a [`Clock`](assistant_core::clock::Clock) implementation.
    ///
    /// Production binaries leave this as the default `SystemClock`. Tests
    /// pass `Arc::new(FakeClock::new(...))` to assert timestamp-dependent
    /// behavior against virtual time.
    pub fn with_clock(mut self, clock: Arc<dyn assistant_core::clock::Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Read pending interactions from `store` instead of the space database.
    pub fn with_interaction_store(
        mut self,
        store: Arc<dyn assistant_storage::InteractionStore>,
    ) -> Self {
        self.interactions = store;
        self
    }

    /// Return a reference to the message bus.
    pub fn bus(&self) -> &Arc<dyn MessageBus> {
        &self.bus
    }

    /// Reader for the events a worker in another process relays for the run
    /// `run_id` of `conversation_id` (see [`SubmitTurn::relay_events`]).
    pub fn run_event_relay(&self, conversation_id: Uuid, run_id: Uuid) -> RunEventRelay {
        RunEventRelay::new(self.bus.clone(), &self.agent_id, conversation_id, run_id)
    }

    /// Cancel an in-flight turn by `request_id` (the same UUID `submit`
    /// assigned, also used as the SSE `run_id` when callers pass it through
    /// [`Self::submit`]).
    ///
    /// Cancellation is propagated via the existing per-turn `CancellationToken`
    /// stored in [`Self::turn_cancellations`]; the worker's `tokio::select!`
    /// aborts the in-flight LLM/tool work on the next yield point, and
    /// `submit` returns an error containing the cancellation marker.
    ///
    /// Returns [`CancelOutcome::Cancelled`] if a matching token was found,
    /// [`CancelOutcome::NotFound`] if the turn has already finished or was
    /// never registered. The caller is responsible for any side effects
    /// (e.g. emitting a final SSE `agent_error` event for streaming clients).
    pub async fn cancel_turn(&self, request_id: Uuid) -> CancelOutcome {
        if self.turn_cancellations.write().await.cancel(request_id) {
            info!(request_id = %request_id, "cancel_turn: cancelled in-flight turn");
            CancelOutcome::Cancelled
        } else {
            CancelOutcome::NotFound
        }
    }

    /// True when `request_id` currently has a registered cancellation token —
    /// i.e. a turn is in flight under this id. Exposed primarily for tests.
    pub async fn is_turn_in_flight(&self, request_id: Uuid) -> bool {
        self.turn_cancellations.read().await.contains(request_id)
    }

    /// The registry of turns running in this process. It is per process: a
    /// turn another service runs against the same database is not visible
    /// here.
    pub fn turn_registry(&self) -> &TurnRegistry {
        &self.turn_registry
    }

    /// True while any turn runs in `conversation_id` on this orchestrator.
    pub fn conversation_turn_in_flight(&self, conversation_id: Uuid) -> bool {
        self.turn_registry.in_flight(conversation_id)
    }

    /// Attach an [`AudioStore`] so that audio produced by the `voice-response`
    /// tool is included in `TurnResult::attachments`.
    pub fn with_audio_store(mut self, store: Arc<AudioStore>) -> Self {
        self.audio_store = Some(store);
        self
    }

    /// Speak the answer of a voice turn with `tts` when the model replied in
    /// text only.
    pub fn with_tts(mut self, tts: Arc<dyn TtsProvider>) -> Self {
        self.tts = Some(tts);
        self
    }

    /// Static rules from config, plus the decision model when it is enabled.
    /// A decision model that cannot be built (e.g. missing API key) is
    /// reported and left out: undecided calls then fall back to a human or a
    /// denial, so this fails closed.
    fn build_permission_policy(
        cfg: &assistant_core::types::permissions::PermissionsConfig,
        llm: &assistant_core::types::llm::LlmConfig,
    ) -> assistant_core::permission::PermissionPolicy {
        let policy = assistant_core::permission::PermissionPolicy::from_config(cfg);
        if !cfg.decision_model.enabled {
            return policy;
        }
        match crate::decision_model::JevDecider::from_config(&cfg.decision_model, llm) {
            Ok(model) => policy.with_model(Arc::new(model)),
            Err(e) => {
                tracing::error!("permission decision model disabled: {e}");
                policy
            }
        }
    }

    /// Replace the permission policy (e.g. to attach a decision model).
    pub fn with_permission_policy(
        mut self,
        policy: assistant_core::permission::PermissionPolicy,
    ) -> Self {
        self.permission_policy = policy;
        self
    }

    /// Let the permission policy read the calling user's saved rules. They
    /// apply to every turn that carries the user's identity, whatever its
    /// interface.
    pub fn with_user_rules(
        mut self,
        source: Arc<dyn assistant_core::permission::UserRuleSource>,
    ) -> Self {
        self.permission_policy = self.permission_policy.clone().with_user_rules(source);
        self
    }

    /// Per-conversation settings set by slash commands. Pass this to
    /// [`CommandContext`](crate::CommandContext) so `/model` reaches turns.
    pub fn conversation_configs(
        &self,
    ) -> Arc<tokio::sync::RwLock<HashMap<Uuid, assistant_core::ConversationConfig>>> {
        Arc::clone(&self.conversation_configs)
    }

    /// Options for agent-loop calls.
    /// [`chat_options`](Self::chat_options), asking `llm` for a spoken
    /// answer on a voice turn when it can give one.
    fn turn_chat_options(
        &self,
        voice_turn: bool,
        llm: &dyn LlmProvider,
    ) -> assistant_core::ChatOptions {
        assistant_core::ChatOptions {
            audio_reply: (voice_turn && llm.chat_options_support().audio_output)
                .then_some(self.reply_format),
            ..self.chat_options()
        }
    }

    fn chat_options(&self) -> assistant_core::ChatOptions {
        assistant_core::ChatOptions {
            parallel_tool_calls: self.llm_config.parallel_tool_calls,
            ..Default::default()
        }
    }

    /// `config` with its context window capped at what `llm` reports now.
    /// Read per turn, so a window the provider learns after startup (or a
    /// `/model` override) applies on the next turn.
    pub(crate) fn compaction_for(
        config: &assistant_core::types::features::CompactionConfig,
        llm: &dyn LlmProvider,
    ) -> assistant_core::types::features::CompactionConfig {
        let mut config = config.clone();
        if let Some(window) = llm.capabilities().context_window_tokens {
            config.context_window_tokens = config.context_window_tokens.min(window);
        }
        config
    }

    /// The provider a turn of `conversation_id` runs on: the model the turn
    /// itself asks for, else the one its `/model` override names, else the
    /// default.
    /// The provider is [`ready`](LlmProvider::ready), so the capabilities a
    /// turn reads are final.
    async fn llm_for(
        &self,
        conversation_id: Uuid,
        turn_model: Option<&str>,
    ) -> Result<Arc<dyn LlmProvider>> {
        let llm = self.select_llm(conversation_id, turn_model).await?;
        llm.ready().await;
        Ok(llm)
    }

    async fn select_llm(
        &self,
        conversation_id: Uuid,
        turn_model: Option<&str>,
    ) -> Result<Arc<dyn LlmProvider>> {
        let model = match turn_model {
            Some(model) => Some(model.to_string()),
            None => self
                .conversation_configs
                .read()
                .await
                .get(&conversation_id)
                .and_then(|config| config.model_override.clone()),
        };
        let Some(model) = model.filter(|m| m != self.llm.model_name()) else {
            return Ok(Arc::clone(&self.llm));
        };

        let mut providers = self
            .model_providers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(provider) = providers.get(&model) {
            return Ok(Arc::clone(provider));
        }
        let provider =
            assistant_llm_provider::create_provider(&assistant_core::types::llm::LlmConfig {
                model: model.clone(),
                ..self.llm_config.clone()
            })
            .map_err(|e| anyhow::anyhow!("cannot use model '{model}': {e}"))?;
        providers.insert(model, Arc::clone(&provider));
        Ok(provider)
    }

    /// The gate every tool call passes before it runs.
    pub fn permission_policy(&self) -> &assistant_core::permission::PermissionPolicy {
        &self.permission_policy
    }

    /// Attach a confirmation callback (used by the CLI interface).
    pub fn with_confirmation_callback(mut self, cb: Arc<dyn ConfirmationCallback>) -> Self {
        self.confirmation_callback = Some(cb);
        self
    }

    /// The persona this orchestrator runs turns as.
    ///
    /// Turns, workspace files and the bus worker claim filter are all scoped
    /// to this id, so a conversation owned by another persona cannot be
    /// processed here.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Return the path to this persona's HEARTBEAT.md.
    pub fn heartbeat_path(&self) -> std::path::PathBuf {
        self.memory_loader.heartbeat_path().to_path_buf()
    }

    /// Return the path to BOOT.md (per-session startup hook).
    pub fn boot_path(&self) -> &Path {
        self.memory_loader.boot_path()
    }

    /// Run the per-session startup hook (BOOT.md).
    ///
    /// Reads BOOT.md from the configured path.  If the file exists and contains
    /// non-comment, non-empty content, its text is submitted as a single silent
    /// turn through the message bus.
    ///
    /// Call this once per session, before the first interactive turn.  Returns
    /// `Ok(true)` if a boot turn was executed, `Ok(false)` if skipped.
    pub async fn run_boot(
        &self,
        conversation_id: uuid::Uuid,
        interface: Interface,
    ) -> Result<bool> {
        let boot_path = self.memory_loader.boot_path();
        if !boot_path.exists() {
            debug!("No BOOT.md found, skipping startup hook");
            return Ok(false);
        }

        let raw = tokio::fs::read_to_string(boot_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read BOOT.md: {e}"))?;

        let stripped = strip_html_comments(&raw);
        if stripped.is_empty() {
            debug!("BOOT.md is empty or comment-only, skipping startup hook");
            return Ok(false);
        }

        info!(path = %boot_path.display(), "Running BOOT.md startup hook");
        match self
            .submit(
                SubmitTurn::new(stripped.as_str(), conversation_id, interface)
                    .with_auth(&assistant_core::auth::AuthContext::system()),
            )
            .await
        {
            Ok(turn) => {
                info!(
                    answer_len = turn.answer.len(),
                    "BOOT.md startup hook completed"
                );
                Ok(true)
            }
            Err(e) => {
                warn!(error = %e, "BOOT.md startup hook failed");
                Err(e)
            }
        }
    }

    // ── Turn entry points ─────────────────────────────────────────────────────

    /// Process one turn of the conversation with per-turn extension tools.
    ///
    /// Extension tools are injected by the calling interface (e.g. Slack,
    /// Mattermost) and are checked before the global tool executor.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_turn_with_tools(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        extensions: Vec<Arc<dyn ToolHandler>>,
        trace_cx: Option<&OtelContext>,
        attachments: Vec<ContentBlock>,
        attachment_ids: Vec<Uuid>,
        identity: TurnIdentity,
    ) -> Result<TurnResult> {
        let turn_span = info_span!(
            "conversation_turn",
            %conversation_id,
            interface = ?interface,
            extension_tools = extensions.len()
        );
        let call_context = LlmCallContext::for_turn(conversation_id, &identity);
        call_context
            .scope(self.run_turn_with_tools_impl(
                user_message,
                conversation_id,
                interface,
                extensions,
                trace_cx,
                attachments,
                None,
                attachment_ids,
                identity,
            ))
            .instrument(turn_span)
            .await
    }

    /// Like [`run_turn_with_tools`] but streams LLM text tokens (including the
    /// text content of tool-call arguments) through `token_sink` as they arrive.
    ///
    /// Intended for messaging interfaces (e.g. Slack) that want to update a
    /// placeholder message progressively while the turn is in progress.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_turn_with_tools_streaming(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        extensions: Vec<Arc<dyn ToolHandler>>,
        trace_cx: Option<&OtelContext>,
        attachments: Vec<ContentBlock>,
        token_sink: mpsc::Sender<OrchestratorEvent>,
        attachment_ids: Vec<Uuid>,
        identity: TurnIdentity,
    ) -> Result<TurnResult> {
        let turn_span = info_span!(
            "conversation_turn",
            %conversation_id,
            interface = ?interface,
            extension_tools = extensions.len(),
            streaming = true
        );
        let call_context = LlmCallContext::for_turn(conversation_id, &identity);
        call_context
            .scope(self.run_turn_with_tools_impl(
                user_message,
                conversation_id,
                interface,
                extensions,
                trace_cx,
                attachments,
                Some(token_sink),
                attachment_ids,
                identity,
            ))
            .instrument(turn_span)
            .await
    }

    /// Process one turn of the conversation.
    pub async fn run_turn(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        trace_cx: Option<&OtelContext>,
        attachment_ids: Vec<Uuid>,
        identity: TurnIdentity,
    ) -> Result<TurnResult> {
        let call_context = LlmCallContext::for_turn(conversation_id, &identity);
        call_context
            .scope(self.run_turn_core(
                user_message,
                conversation_id,
                interface,
                None,
                trace_cx,
                attachment_ids,
                identity,
            ))
            .await
    }

    /// Like [`run_turn`] but streams final-answer tokens through `token_sink`.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_turn_streaming(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        token_sink: mpsc::Sender<OrchestratorEvent>,
        trace_cx: Option<&OtelContext>,
        attachment_ids: Vec<Uuid>,
        identity: TurnIdentity,
    ) -> Result<TurnResult> {
        let call_context = LlmCallContext::for_turn(conversation_id, &identity);
        call_context
            .scope(self.run_turn_core(
                user_message,
                conversation_id,
                interface,
                Some(token_sink),
                trace_cx,
                attachment_ids,
                identity,
            ))
            .await
    }

    // ── Turn implementations ──────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    async fn run_turn_with_tools_impl(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        extensions: Vec<Arc<dyn ToolHandler>>,
        trace_cx: Option<&OtelContext>,
        attachments: Vec<ContentBlock>,
        token_sink: Option<mpsc::Sender<OrchestratorEvent>>,
        attachment_ids: Vec<Uuid>,
        identity: TurnIdentity,
    ) -> Result<TurnResult> {
        self.metrics
            .record_turn(&self.agent_id, None, &format!("{interface:?}"));
        info!("Starting turn with extension tools");

        let (_conv_cx, turn_cx) = setup_turn_trace(trace_cx, conversation_id, &interface);

        // Build extension lookup: name -> handler.
        let ext_map: HashMap<String, Arc<dyn ToolHandler>> = extensions
            .iter()
            .map(|h| (h.name().to_string(), h.clone()))
            .collect();

        // Build extension ToolSpecs for LLM listing.
        let mut ext_specs: Vec<ToolSpec> = extensions
            .iter()
            .map(|h| ToolSpec {
                name: h.name().to_string(),
                description: h.description().to_string(),
                params_schema: h.params_schema(),
                is_mutating: h.is_mutating(),
                requires_confirmation: h.requires_confirmation(),
                requires_rich_client: h.requires_rich_client(),
            })
            .collect();

        // Always inject `end_turn` unless the caller already provided one.
        if !ext_specs.iter().any(|s| s.name == "end_turn") && !ext_map.contains_key("end_turn") {
            ext_specs.push(end_turn_spec());
        }

        // 1-3. Set up conversation, load prior history, persist user message.
        let (conv_store, mut history, base_turn) = self
            .prepare_history(
                user_message,
                conversation_id,
                attachments,
                &attachment_ids,
                &self.agent_id,
                &identity,
            )
            .await?;
        let summary_store = self.storage.conversation_summary_store();

        // 4. Load global tool specs and merge with extensions.
        let has_reply_ext = ext_specs.iter().any(|s| s.name.contains("reply"));
        let llm = self
            .llm_for(conversation_id, identity.model_override.as_deref())
            .await?;
        let voice_turn = history.has_audio();
        if !llm.chat_options_support().audio_input {
            history.drop_audio();
        }
        let chat_options = self.turn_chat_options(voice_turn, llm.as_ref());
        let compaction_config = Self::compaction_for(&self.compaction_config, llm.as_ref());
        let provider_caps = llm.capabilities();
        let overlay = self.turn_overlay(&identity).await;
        let rich_client = Self::rich_client_turn(&interface, identity.interactive, 0);
        let global_specs = Self::filter_tool_specs(
            self.executor.to_specs_with(&overlay),
            &provider_caps,
            rich_client,
        );
        let all_specs: Vec<ToolSpec> = ext_specs
            .iter()
            .cloned()
            .chain(
                global_specs
                    .into_iter()
                    .filter(|s| !has_reply_ext || !s.name.contains("post")),
            )
            .collect();

        let base_system_prompt = self.compose_system_prompt(&interface).await;
        let system_prompt = Self::build_extension_system_prompt(&base_system_prompt, &ext_specs);

        let mut turn_ended = false;
        let mut replied = false;
        let mut turn_attachments: Vec<Attachment> = Vec::new();
        let mut turn_attachment_ids: Vec<Uuid> = Vec::new();
        let mut turn_had_errors = false;
        let mut active_skill: Option<String> = None;
        let mut surface_turn = SurfaceTurn::new(base_turn);

        // 5. Tool-calling loop.
        for iteration in 0..self.max_iterations {
            debug!(iteration, "Extension-tools loop iteration");

            // Pre-call compaction: compact before sending to the LLM so we
            // never exceed the context window on the call itself.  Uses the
            // total estimator (history + system prompt + tool specs).
            {
                let estimated =
                    crate::compaction::estimate_total_tokens(&system_prompt, &history, &all_specs);
                if crate::compaction::should_compact(estimated, &compaction_config) {
                    crate::compaction::maybe_compact(
                        &mut history,
                        &llm,
                        &compaction_config,
                        Some((&summary_store, conversation_id)),
                    )
                    .await;
                }
            }

            // Hard-truncate as a safety net: if the estimate still exceeds the
            // hard limit after compaction, drop oldest messages.
            crate::compaction::hard_truncate(
                &system_prompt,
                &mut history,
                &all_specs,
                &compaction_config,
            );

            let ctx = ExecutionContext {
                conversation_id,
                agent_id: self.agent_id.clone(),
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: identity.interactive,
                run_id: identity.run_id,
                allowed_tools: None,
                depth: 0,
                user_id: identity.user_id.clone(),
                org_id: identity.org_id.clone(),
                space_id: identity.space_id.clone(),
            };

            let mut llm_span = crate::otel_spans::start_llm_span(
                conversation_id,
                llm.as_ref(),
                iteration,
                &turn_cx,
                self.trace_content,
                &system_prompt,
                &history,
                &all_specs,
            );
            let llm_traceparent = crate::otel_spans::traceparent_from_span(&llm_span);
            let llm_start = std::time::Instant::now();
            let response = if let Some(ref oe_sink) = token_sink {
                // Adapt OrchestratorEvent sink → StreamChunk sink expected by chat_streaming.
                let (chunk_tx, mut chunk_rx) = mpsc::channel::<StreamChunk>(64);
                let oe_sink_clone = oe_sink.clone();
                let forward_handle = tokio::spawn(async move {
                    while let Some(chunk) = chunk_rx.recv().await {
                        let event = match chunk {
                            StreamChunk::Text(t) => OrchestratorEvent::Token(t),
                            StreamChunk::Thinking(t) => OrchestratorEvent::Thinking(t),
                        };
                        let _ = oe_sink_clone.send(event).await;
                    }
                });
                let result = llm
                    .chat_with(
                        &system_prompt,
                        &history,
                        &all_specs,
                        &chat_options,
                        Some(chunk_tx),
                    )
                    .await;
                // Wait for the forwarding task to drain any buffered tokens before
                // continuing — ensures callers see all Token events after this await.
                forward_handle.await.ok();
                result
            } else {
                llm.chat_with(&system_prompt, &history, &all_specs, &chat_options, None)
                    .await
            };
            let llm_elapsed = llm_start.elapsed();
            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    crate::history::persist_error_recovery(&conv_store, conversation_id).await;
                    self.metrics
                        .record_error(&self.agent_id, "llm_error", "run_turn_with_tools");
                    turn_cx.span().set_status(OtelStatus::Error {
                        description: std::borrow::Cow::Owned(e.to_string()),
                    });
                    return Err(e);
                }
            };
            crate::otel_spans::finish_llm_span(
                &mut llm_span,
                response.meta(),
                &response,
                self.trace_content,
                Some((
                    &self.metrics,
                    &self.agent_id,
                    llm.provider_name(),
                    llm_elapsed,
                )),
            );

            // Post-call compaction: use the accurate token count reported by
            // the provider as a secondary signal to catch any cases the pre-call
            // estimator missed.
            if let Some(tokens) = response.meta().input_tokens
                && crate::compaction::should_compact(tokens, &compaction_config)
            {
                crate::compaction::maybe_compact(
                    &mut history,
                    &llm,
                    &compaction_config,
                    Some((&summary_store, conversation_id)),
                )
                .await;
            }

            match response {
                LlmResponse::FinalAnswer(text, meta) => {
                    let outcome = Self::handle_final_answer_with_extensions(
                        replied,
                        &text,
                        iteration,
                        base_turn,
                        conversation_id,
                        &self.agent_id,
                        &interface,
                        &ext_map,
                        &conv_store,
                        &self.storage.message_timing_store(),
                        turn_had_errors,
                        FinalAnswerMeta {
                            traceparent: llm_traceparent,
                            timing: None,
                            model: Some(llm.model_name().to_string()),
                        },
                    )
                    .await?;
                    match outcome {
                        FinalAnswerOutcome::Done(mut result) => {
                            let media = self
                                .reply_media(voice_turn, &text, meta.media, &turn_attachments)
                                .await;
                            self.persist_response_media(
                                conversation_id,
                                result.message_id,
                                media,
                                &mut turn_attachments,
                                &mut turn_attachment_ids,
                            )
                            .await;
                            result.attachments = turn_attachments;
                            result.attachment_ids = turn_attachment_ids;
                            return Ok(result);
                        }
                        FinalAnswerOutcome::Retry => continue,
                    }
                }

                LlmResponse::ToolCalls(tool_call_resp) => {
                    // Emit batch thinking (non-streaming path) before tool processing.
                    if let Some(ref thinking) = tool_call_resp.thinking
                        && let Some(ref sink) = token_sink
                    {
                        let _ = sink
                            .send(OrchestratorEvent::Thinking(thinking.clone()))
                            .await;
                    }
                    let tool_call_items = tool_call_resp.items;
                    info!(
                        count = tool_call_items.len(),
                        iteration, "LLM requested tool execution(s)"
                    );

                    let anchor_message_id = Self::persist_tool_calls(
                        &mut history,
                        &conv_store,
                        conversation_id,
                        base_turn + iteration as i64 + 1,
                        &tool_call_items,
                        tool_call_resp.provider_state.as_ref(),
                    )
                    .await;

                    let has_real_calls = tool_call_items.iter().any(|t| t.name != "end_turn");

                    for (call_index, tool_call_item) in tool_call_items.into_iter().enumerate() {
                        let name = tool_call_item.name;
                        let params = tool_call_item.params;
                        let tool_call_id = tool_call_item.id;
                        let turn_index = base_turn + iteration as i64 + 1;
                        let mut otel_span = crate::otel_spans::start_tool_span(
                            conversation_id,
                            iteration,
                            turn_index,
                            &interface,
                            &name,
                            &params,
                            &turn_cx,
                            active_skill.as_deref(),
                        );

                        if name == "end_turn" {
                            let outcome = Self::handle_end_turn(
                                has_real_calls,
                                replied,
                                &ext_map,
                                &params,
                                iteration,
                                conversation_id,
                                turn_index,
                                &mut otel_span,
                                &mut history,
                                &conv_store,
                            )
                            .await;
                            match outcome {
                                EndTurnOutcome::Deferred | EndTurnOutcome::Rejected => continue,
                                EndTurnOutcome::Accepted => {
                                    turn_ended = true;
                                    break;
                                }
                            }
                        }

                        // Extension tools take priority and bypass the safety gate.
                        if let Some(handler) = ext_map.get(&name) {
                            debug!(tool = %name, "Dispatching to extension handler");

                            if let Some(sink) = token_sink.as_ref() {
                                let _ = sink
                                    .send(OrchestratorEvent::Status {
                                        message: format!("Calling tool: {name}"),
                                        tool_call_id: tool_call_id.clone(),
                                    })
                                    .await;
                            }

                            let params_map = value_to_params_map(&params);

                            let start = std::time::Instant::now();
                            let exec_result = handler.run(params_map, &ctx).await;
                            let elapsed = start.elapsed();

                            let finalized = self
                                .finalize_tool_result(
                                    &name,
                                    Some(&params),
                                    exec_result,
                                    elapsed,
                                    &mut otel_span,
                                    &mut history,
                                    &conv_store,
                                    conversation_id,
                                    turn_index,
                                    &mut turn_attachments,
                                    &mut turn_attachment_ids,
                                    &mut active_skill,
                                    token_sink.as_ref(),
                                    tool_call_id.as_deref(),
                                    None,
                                )
                                .await;
                            turn_had_errors |= finalized.had_error;
                        } else {
                            // Global executor path.
                            let builtin_span = info_span!(
                                "tool_handler",
                                tool = %name,
                                source = "builtin"
                            );
                            let outcome = self
                                .dispatch_global_tool(
                                    &name,
                                    &params,
                                    &ctx,
                                    &mut otel_span,
                                    &mut history,
                                    &conv_store,
                                    conversation_id,
                                    turn_index,
                                    &mut turn_attachments,
                                    &mut turn_attachment_ids,
                                    &overlay,
                                    &mut active_skill,
                                    &builtin_span,
                                    token_sink.as_ref(),
                                    tool_call_id.as_deref(),
                                    Some(SurfaceCall {
                                        turn: &mut surface_turn,
                                        anchor_message_id,
                                        call_index,
                                    }),
                                )
                                .await;
                            match outcome {
                                DispatchOutcome::Denied => continue,
                                DispatchOutcome::Executed { had_error } => {
                                    turn_had_errors |= had_error;
                                }
                            }
                        }

                        if name.contains("reply") || name.contains("post") || name.contains("react")
                        {
                            replied = true;
                        }
                    }

                    if turn_ended || replied {
                        crate::conversation_indexer::spawn_index(
                            conversation_id,
                            self.agent_id.clone(),
                            Arc::clone(&self.storage),
                            Arc::clone(&self.llm),
                        );
                        return Ok(TurnResult {
                            answer: String::new(),
                            attachments: turn_attachments,
                            attachment_ids: turn_attachment_ids,
                            message_id: None,
                            had_errors: turn_had_errors,
                            timing: None,
                        });
                    }
                }

                LlmResponse::Thinking(text, _meta) => {
                    debug!(iteration, "LLM emitted thinking step");
                    if let Some(sink) = token_sink.as_ref() {
                        let _ = sink.send(OrchestratorEvent::Thinking(text.clone())).await;
                    }
                    let thinking_msg = {
                        let mut m = assistant_core::types::conversation::Message::assistant(
                            conversation_id,
                            format!("<think>{text}</think>"),
                        );
                        m.turn = base_turn + iteration as i64 + 1;
                        m
                    };
                    if let Err(e) = conv_store.save_message(&thinking_msg).await {
                        warn!("Failed to persist thinking step: {e}");
                    }
                    history.push(
                        ChatHistoryMessage::Text {
                            role: ChatRole::Assistant,
                            content: text,
                        },
                        Some(thinking_msg.id),
                    );
                }
            }
        }

        crate::history::persist_error_recovery(&conv_store, conversation_id).await;
        self.metrics
            .record_error(&self.agent_id, "max_iterations", "run_turn_with_tools");
        turn_cx.span().set_status(OtelStatus::Error {
            description: std::borrow::Cow::Owned(format!(
                "Max iterations ({}) reached without a final answer",
                self.max_iterations
            )),
        });
        anyhow::bail!(
            "Max iterations ({}) reached without a final answer",
            self.max_iterations
        );
    }

    /// Shared implementation for [`run_turn`] and [`run_turn_streaming`].
    #[allow(clippy::too_many_arguments)]
    async fn run_turn_core(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        token_sink: Option<mpsc::Sender<OrchestratorEvent>>,
        trace_cx: Option<&OtelContext>,
        attachment_ids: Vec<Uuid>,
        identity: TurnIdentity,
    ) -> Result<TurnResult> {
        let streaming = token_sink.is_some();
        self.metrics
            .record_turn(&self.agent_id, None, &format!("{interface:?}"));
        info!(
            conversation_id = %conversation_id,
            interface = ?interface,
            streaming,
            "Starting turn"
        );

        let mut timer = TurnTimer::new(self.clock.now_instant());
        let (_conv_cx, turn_cx) = setup_turn_trace(trace_cx, conversation_id, &interface);

        // 1-3. Set up conversation, load prior history, persist user message.
        let (conv_store, mut history, base_turn) = self
            .prepare_history(
                user_message,
                conversation_id,
                Vec::new(),
                &attachment_ids,
                &self.agent_id,
                &identity,
            )
            .await?;
        let summary_store = self.storage.conversation_summary_store();

        // 4. Load all registered tool specs.
        let llm = self
            .llm_for(conversation_id, identity.model_override.as_deref())
            .await?;
        let voice_turn = history.has_audio();
        if !llm.chat_options_support().audio_input {
            history.drop_audio();
        }
        let chat_options = self.turn_chat_options(voice_turn, llm.as_ref());
        let compaction_config = Self::compaction_for(&self.compaction_config, llm.as_ref());
        let provider_caps = llm.capabilities();
        let overlay = self.turn_overlay(&identity).await;
        let rich_client = Self::rich_client_turn(&interface, identity.interactive, 0);
        let tool_specs = Self::filter_tool_specs(
            self.executor.to_specs_with(&overlay),
            &provider_caps,
            rich_client,
        );

        // 5. Build the system prompt fresh from disk.
        let system_prompt = self.compose_system_prompt(&interface).await;

        // 6. Tool-calling loop.
        let mut turn_attachments: Vec<Attachment> = Vec::new();
        let mut turn_attachment_ids: Vec<Uuid> = Vec::new();
        let mut turn_tool_count: usize = 0;
        let mut turn_had_errors = false;
        let mut active_skill: Option<String> = None;
        let mut surface_turn = SurfaceTurn::new(base_turn);

        for iteration in 0..self.max_iterations {
            let iteration_span = info_span!("turn_iteration", iteration);
            debug!(parent: &iteration_span, iteration, "Tool-calling loop iteration");

            // Pre-call compaction using the total token estimator (includes
            // system prompt and tool specs for a more accurate picture).
            {
                let estimated =
                    crate::compaction::estimate_total_tokens(&system_prompt, &history, &tool_specs);
                if crate::compaction::should_compact(estimated, &compaction_config) {
                    crate::compaction::maybe_compact(
                        &mut history,
                        &llm,
                        &compaction_config,
                        Some((&summary_store, conversation_id)),
                    )
                    .await;
                }
            }

            // Hard-truncate as a safety net: if the estimate still exceeds the
            // hard limit after compaction (e.g. a single huge tool result),
            // drop oldest messages to avoid a 400 from the provider.
            crate::compaction::hard_truncate(
                &system_prompt,
                &mut history,
                &tool_specs,
                &compaction_config,
            );

            let ctx = ExecutionContext {
                conversation_id,
                agent_id: self.agent_id.clone(),
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: identity.interactive,
                run_id: identity.run_id,
                allowed_tools: None,
                depth: 0,
                user_id: identity.user_id.clone(),
                org_id: identity.org_id.clone(),
                space_id: identity.space_id.clone(),
            };

            let mut llm_span = crate::otel_spans::start_llm_span(
                conversation_id,
                llm.as_ref(),
                iteration,
                &turn_cx,
                self.trace_content,
                &system_prompt,
                &history,
                &tool_specs,
            );
            let llm_traceparent = crate::otel_spans::traceparent_from_span(&llm_span);
            let llm_start = std::time::Instant::now();
            let call_start = self.clock.now_instant();
            let (response, call_end, marks) = if let Some(ref oe_sink) = token_sink {
                // Adapt OrchestratorEvent sink → StreamChunk sink expected by chat_streaming.
                let (chunk_tx, mut chunk_rx) = mpsc::channel::<StreamChunk>(64);
                let oe_sink_clone = oe_sink.clone();
                let marks = Arc::new(tokio::sync::Mutex::new(CallMarks::default()));
                let forward_marks = Arc::clone(&marks);
                let forward_clock = Arc::clone(&self.clock);
                let forward_handle = tokio::spawn(async move {
                    while let Some(chunk) = chunk_rx.recv().await {
                        // Read the clock before any await so the instant is
                        // the chunk's arrival, not the lock's.
                        let arrived = forward_clock.now_instant();
                        {
                            let mut m = forward_marks.lock().await;
                            m.first_chunk.get_or_insert(arrived);
                            if matches!(chunk, StreamChunk::Text(_)) {
                                m.first_text.get_or_insert(arrived);
                            }
                        }
                        let event = match chunk {
                            StreamChunk::Text(t) => OrchestratorEvent::Token(t),
                            StreamChunk::Thinking(t) => OrchestratorEvent::Thinking(t),
                        };
                        let _ = oe_sink_clone.send(event).await;
                    }
                });
                let result = llm
                    .chat_with(
                        &system_prompt,
                        &history,
                        &tool_specs,
                        &chat_options,
                        Some(chunk_tx),
                    )
                    .instrument(iteration_span.clone())
                    .await;
                let call_end = self.clock.now_instant();
                // Wait for the forwarding task to drain any buffered tokens before
                // continuing — ensures callers see all Token events after this await.
                forward_handle.await.ok();
                let marks = *marks.lock().await;
                (result, call_end, marks)
            } else {
                let result = llm
                    .chat_with(&system_prompt, &history, &tool_specs, &chat_options, None)
                    .instrument(iteration_span.clone())
                    .await;
                (result, self.clock.now_instant(), CallMarks::default())
            };
            let llm_elapsed = llm_start.elapsed();
            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    crate::history::persist_error_recovery(&conv_store, conversation_id)
                        .instrument(iteration_span.clone())
                        .await;
                    let label = if streaming {
                        "run_turn_streaming"
                    } else {
                        "run_turn"
                    };
                    self.metrics
                        .record_error(&self.agent_id, "llm_error", label);
                    turn_cx.span().set_status(OtelStatus::Error {
                        description: std::borrow::Cow::Owned(e.to_string()),
                    });
                    return Err(e);
                }
            };
            timer.record_call(call_start, call_end, marks, response.meta().output_tokens);
            crate::otel_spans::finish_llm_span(
                &mut llm_span,
                response.meta(),
                &response,
                self.trace_content,
                Some((
                    &self.metrics,
                    &self.agent_id,
                    llm.provider_name(),
                    llm_elapsed,
                )),
            );

            // Post-call compaction using the accurate provider-reported token count.
            if let Some(tokens) = response.meta().input_tokens
                && crate::compaction::should_compact(tokens, &compaction_config)
            {
                crate::compaction::maybe_compact(
                    &mut history,
                    &llm,
                    &compaction_config,
                    Some((&summary_store, conversation_id)),
                )
                .await;
            }

            match response {
                LlmResponse::FinalAnswer(text, meta) => {
                    info!(iteration, "LLM returned final answer");

                    let (saved_message_id, timing) = if !text.trim().is_empty() {
                        let timing = timer.finish(self.clock.now_instant());
                        let assistant_msg = {
                            let mut m = Message::assistant(conversation_id, &text);
                            m.turn = base_turn + iteration as i64 + 1;
                            m
                        };
                        let msg_id = assistant_msg.id;
                        persist_final_answer(
                            &conv_store,
                            &self.storage.message_timing_store(),
                            assistant_msg,
                            FinalAnswerMeta {
                                traceparent: llm_traceparent,
                                timing: Some(timing.clone()),
                                model: Some(llm.model_name().to_string()),
                            },
                        )
                        .instrument(iteration_span.clone())
                        .await?;
                        (Some(msg_id), Some(timing))
                    } else {
                        (None, None)
                    };

                    crate::conversation_indexer::spawn_index(
                        conversation_id,
                        self.agent_id.clone(),
                        Arc::clone(&self.storage),
                        Arc::clone(&self.llm),
                    );

                    // Spawn post-turn skill evaluation (fire-and-forget).
                    if self.learning_config.enabled && self.learning_config.auto_create_skills {
                        crate::skill_learner::spawn_post_turn_eval(
                            crate::skill_learner::TurnContext {
                                conversation_id,
                                agent_id: self.agent_id.clone(),
                                tool_count: turn_tool_count,
                                had_errors: turn_had_errors,
                                active_skill: active_skill.clone(),
                                history: history.to_vec(),
                            },
                            self.learning_config.clone(),
                            Arc::clone(&self.storage),
                            Arc::clone(&self.registry),
                            Arc::clone(&self.llm),
                        );
                    }

                    let media = self
                        .reply_media(voice_turn, &text, meta.media, &turn_attachments)
                        .await;
                    self.persist_response_media(
                        conversation_id,
                        saved_message_id,
                        media,
                        &mut turn_attachments,
                        &mut turn_attachment_ids,
                    )
                    .await;
                    return Ok(TurnResult {
                        answer: text,
                        attachments: turn_attachments,
                        attachment_ids: turn_attachment_ids,
                        message_id: saved_message_id,
                        had_errors: turn_had_errors,
                        timing,
                    });
                }

                LlmResponse::ToolCalls(tool_call_resp) => {
                    // Emit batch thinking before tool processing.
                    if let Some(ref thinking) = tool_call_resp.thinking
                        && let Some(ref sink) = token_sink
                    {
                        let _ = sink
                            .send(OrchestratorEvent::Thinking(thinking.clone()))
                            .await;
                    }
                    let tool_call_items = tool_call_resp.items;
                    info!(
                        count = tool_call_items.len(),
                        iteration, "LLM requested tool execution(s)"
                    );

                    let anchor_message_id = Self::persist_tool_calls(
                        &mut history,
                        &conv_store,
                        conversation_id,
                        base_turn + iteration as i64 + 1,
                        &tool_call_items,
                        tool_call_resp.provider_state.as_ref(),
                    )
                    .instrument(iteration_span.clone())
                    .await;

                    for (call_index, tool_call_item) in tool_call_items.into_iter().enumerate() {
                        let name = tool_call_item.name;
                        let params = tool_call_item.params;
                        let tool_call_id = tool_call_item.id;
                        let turn_index = base_turn + iteration as i64 + 1;
                        let mut otel_span = crate::otel_spans::start_tool_span(
                            conversation_id,
                            iteration,
                            turn_index,
                            &interface,
                            &name,
                            &params,
                            &turn_cx,
                            active_skill.as_deref(),
                        );

                        let outcome = self
                            .dispatch_global_tool(
                                &name,
                                &params,
                                &ctx,
                                &mut otel_span,
                                &mut history,
                                &conv_store,
                                conversation_id,
                                turn_index,
                                &mut turn_attachments,
                                &mut turn_attachment_ids,
                                &overlay,
                                &mut active_skill,
                                &iteration_span,
                                token_sink.as_ref(),
                                tool_call_id.as_deref(),
                                Some(SurfaceCall {
                                    turn: &mut surface_turn,
                                    anchor_message_id,
                                    call_index,
                                }),
                            )
                            .await;
                        match outcome {
                            DispatchOutcome::Denied => continue,
                            DispatchOutcome::Executed { had_error } => {
                                turn_had_errors |= had_error;
                            }
                        }
                        turn_tool_count += 1;
                    }
                }

                LlmResponse::Thinking(text, _meta) => {
                    debug!(iteration, "LLM emitted thinking step");
                    if let Some(sink) = token_sink.as_ref() {
                        let _ = sink.send(OrchestratorEvent::Thinking(text.clone())).await;
                    }
                    let thinking_msg = {
                        let mut m =
                            Message::assistant(conversation_id, format!("<think>{text}</think>"));
                        m.turn = base_turn + iteration as i64 + 1;
                        m
                    };
                    if let Err(e) = conv_store.save_message(&thinking_msg).await {
                        warn!("Failed to persist thinking step: {e}");
                    }
                    history.push(
                        ChatHistoryMessage::Text {
                            role: ChatRole::Assistant,
                            content: text,
                        },
                        Some(thinking_msg.id),
                    );
                }
            }
        }

        // Reached iteration limit.
        crate::history::persist_error_recovery(&conv_store, conversation_id).await;
        let label = if streaming {
            "run_turn_streaming"
        } else {
            "run_turn"
        };
        self.metrics
            .record_error(&self.agent_id, "max_iterations", label);
        turn_cx.span().set_status(OtelStatus::Error {
            description: std::borrow::Cow::Owned(format!(
                "Max iterations ({}) reached without a final answer",
                self.max_iterations
            )),
        });
        anyhow::bail!(
            "Max iterations ({}) reached without a final answer",
            self.max_iterations
        );
    }

    // ── Attachment helpers ─────────────────────────────────────────────────────

    /// `media` plus, for a voice turn that produced no audio (neither with
    /// the answer nor from a tool such as `voice-response`), the answer
    /// spoken by the TTS provider. A failed synthesis leaves the text only.
    async fn reply_media(
        &self,
        voice_turn: bool,
        answer: &str,
        mut media: Vec<assistant_core::GeneratedMedia>,
        tool_attachments: &[Attachment],
    ) -> Vec<assistant_core::GeneratedMedia> {
        let has_audio = media
            .iter()
            .any(|m| m.kind == assistant_core::MediaKind::Audio)
            || tool_attachments
                .iter()
                .any(|a| a.mime_type.starts_with("audio/"));
        let Some(tts) = self.tts.as_ref().filter(|_| voice_turn && !has_audio) else {
            return media;
        };
        if answer.trim().is_empty() {
            return media;
        }
        let request = assistant_transcription::TtsRequest {
            text: answer.to_owned(),
            voice: None,
            format: None,
            speed: None,
        };
        match tts.synthesize(request).await {
            Ok(spoken) => media.push(assistant_core::GeneratedMedia {
                kind: assistant_core::MediaKind::Audio,
                mime: spoken.mime_type,
                data: spoken.audio_data.into(),
            }),
            Err(e) => warn!(error = %e, "Failed to speak the voice turn's answer"),
        }
        media
    }

    /// Stores media generated with a final answer as attachments of
    /// `message_id`, the saved assistant message, and adds them to the
    /// turn's attachments so interfaces deliver them like tool output.
    async fn persist_response_media(
        &self,
        conversation_id: Uuid,
        message_id: Option<Uuid>,
        media: Vec<assistant_core::GeneratedMedia>,
        attachments: &mut Vec<assistant_core::Attachment>,
        attachment_ids: &mut Vec<Uuid>,
    ) {
        let store = self.storage.attachment_store();
        for (index, item) in media.into_iter().enumerate() {
            let kind = match item.kind {
                assistant_core::MediaKind::Image => "image",
                assistant_core::MediaKind::Audio => "audio",
            };
            let filename = format!(
                "{kind}-{}.{}",
                index + 1,
                assistant_core::attachment::extension_for_mime(&item.mime)
            );
            let meta = assistant_core::AttachmentMeta {
                id: Uuid::new_v4(),
                message_id,
                conversation_id,
                agent_id: self.agent_id.clone(),
                filename: filename.clone(),
                mime_type: item.mime.clone(),
                size_bytes: item.data.len() as u64,
                created_at: self.clock.now(),
            };
            match store.store(&meta, &item.data).await {
                Ok(()) => {
                    debug!(attachment_id = %meta.id, %filename, "Persisted generated media");
                    attachment_ids.push(meta.id);
                    attachments.push(assistant_core::Attachment::new(
                        filename,
                        item.mime,
                        item.data.to_vec(),
                    ));
                }
                Err(e) => warn!(%filename, error = %e, "Failed to persist generated media"),
            }
        }
    }

    /// Build a map of `message_id → [ContentBlock]` for all attachments in a
    /// conversation that are linked to a message.  Used to replay attachments
    /// in chat history so the LLM can reference previously-sent content.
    ///
    /// A user message in `path` also gets the attachments an edit carried
    /// forward to it, which belong to an earlier version of the message.
    ///
    /// Images are resized and encoded as `ContentBlock::Image`.  PDFs become
    /// `ContentBlock::Document`.  Text files are inlined as `ContentBlock::Text`.
    async fn build_attachment_map(
        &self,
        conversation_id: Uuid,
        path: &[Message],
    ) -> anyhow::Result<crate::history::AttachmentMap> {
        let store = self.storage.attachment_store();
        let mut owned: Vec<(Uuid, AttachmentMeta)> = store
            .list_for_conversation(conversation_id)
            .await?
            .into_iter()
            .filter_map(|meta| Some((meta.message_id?, meta)))
            .collect();
        for message in path.iter().filter(|m| m.role == MessageRole::User) {
            for meta in store.list_for_message(message.id).await? {
                if meta.message_id != Some(message.id) {
                    owned.push((message.id, meta));
                }
            }
        }

        let mut map = crate::history::AttachmentMap::new();
        for (msg_id, meta) in owned {
            if !is_supported_mime_type(&meta.mime_type) {
                continue;
            }
            match store.load_bytes(meta.id).await {
                Ok(bytes) => {
                    let block =
                        attachment_to_content_block(&meta, &bytes, self.llm.provider_name());
                    map.entry(msg_id).or_default().push(block);
                }
                Err(e) => {
                    warn!(
                        attachment_id = %meta.id,
                        error = %e,
                        "Failed to load attachment for history replay; skipping"
                    );
                }
            }
        }
        Ok(map)
    }

    // ── History setup ─────────────────────────────────────────────────────────

    pub(crate) async fn prepare_history(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        attachments: Vec<ContentBlock>,
        attachment_ids: &[Uuid],
        agent_id: &str,
        identity: &TurnIdentity,
    ) -> Result<(TurnStore, TurnHistory, i64)> {
        // Note: returns the concrete `TurnStore` because the function is an
        // internal helper called only from this impl. External consumer
        // signatures (helper fns in dispatch.rs, turn_control.rs, history.rs,
        // web-ui handlers) take `&dyn ConversationStore`; the trait is what
        // crosses crate / function-API boundaries.
        let conv_store = TurnStore::new(
            self.storage.conversation_store_for_agent(agent_id),
            identity.run_id,
        );
        conv_store
            .create_conversation_with_id(conversation_id, None)
            .await?;

        let turn_anchor::AnchoredTurn {
            prior,
            base_turn,
            user,
            regenerated,
            carried,
        } = self
            .begin_anchored_turn(&conv_store, conversation_id, user_message, identity)
            .await?;

        if base_turn == 0 && identity.anchor == TurnAnchor::Append {
            self.metrics.conversation_count.add(1, &[]);
        }

        // Link uploaded attachments to the persisted user message so that
        // `build_attachment_map()` can find them on subsequent turns.
        if let Some(user_msg) = &user
            && !attachment_ids.is_empty()
        {
            let store = self.storage.attachment_store();
            for &att_id in attachment_ids {
                if let Err(e) = store.link_to_message(att_id, user_msg.id).await {
                    warn!(
                        attachment_id = %att_id,
                        message_id = %user_msg.id,
                        error = %e,
                        "Failed to link attachment to message"
                    );
                }
            }
        }

        // Build an attachment map for history replay: load all attachments
        // linked to messages on this path and base64-encode them so the LLM
        // sees previously-sent images. A regenerate also needs the map for
        // the user message it answers.
        let mut map_path = prior.clone();
        map_path.extend(regenerated.clone());
        let att_map = self
            .build_attachment_map(conversation_id, &map_path)
            .await
            .unwrap_or_default();

        let summaries = self.storage.conversation_summary_store();
        let mut history =
            crate::history::build_turn_history(prior, conversation_id, &summaries, &att_map)
                .await?;
        crate::history::sanitize_history(&mut history);

        // The turn's user message: the prompt, or for a regenerate the stored
        // message with the attachments it already has.
        let (user_message, mut all_attachments) = match &regenerated {
            Some(user) => (
                user.content.as_str(),
                att_map.get(&user.id).cloned().unwrap_or_default(),
            ),
            None => (user_message, attachments),
        };

        // Load stored attachments by ID and convert to the appropriate ContentBlock.
        // A regenerate has no new attachments: the stored message has its own.
        let new_ids: &[Uuid] = if regenerated.is_some() {
            &[]
        } else {
            attachment_ids
        };
        let to_load: Vec<Uuid> = new_ids
            .iter()
            .copied()
            .chain(carried.into_iter().filter(|id| !new_ids.contains(id)))
            .collect();
        if !to_load.is_empty() {
            let store = self.storage.attachment_store();
            for &att_id in &to_load {
                match store.load_bytes(att_id).await {
                    Ok(bytes) => {
                        let meta = store.get_meta(att_id).await?;
                        let block =
                            attachment_to_content_block(&meta, &bytes, self.llm.provider_name());
                        all_attachments.push(block);
                    }
                    Err(e) => {
                        warn!(
                            attachment_id = %att_id,
                            error = %e,
                            "Failed to load attachment bytes; skipping"
                        );
                    }
                }
            }

            // Warn when sending images to Ollama — model may not support vision.
            if !all_attachments.is_empty() && self.llm.provider_name() == "ollama" {
                warn!(
                    "Ollama model may not support vision. \
                     Image attachments included but may be ignored by the model."
                );
            }
        }

        let user_row_id = user.as_ref().or(regenerated.as_ref()).map(|m| m.id);
        if all_attachments.is_empty() {
            history.push(
                ChatHistoryMessage::Text {
                    role: ChatRole::User,
                    content: user_message.to_string(),
                },
                user_row_id,
            );
        } else {
            let mut blocks = vec![ContentBlock::Text(user_message.to_string())];
            blocks.extend(all_attachments);
            history.push(
                ChatHistoryMessage::MultimodalUser { content: blocks },
                user_row_id,
            );
        }

        Ok((conv_store, history, base_turn))
    }
}

// ── Module-level helpers ───────────────────────────────────────────────────────

/// Set up the two-level OTel trace hierarchy used by every turn variant.
///
/// Returns `(conv_cx, turn_cx)`.  The caller **must** keep `conv_cx` alive
/// (bind it to `_conv_cx`) so the conversation span is not dropped early.
fn setup_turn_trace(
    trace_cx: Option<&OtelContext>,
    conversation_id: Uuid,
    interface: &Interface,
) -> (OtelContext, OtelContext) {
    let tracer = global::tracer("assistant.orchestrator");
    let conv_cx = match trace_cx {
        Some(cx) => cx.clone(),
        None => {
            let mut span = tracer.start("conversation");
            span.set_attribute(KeyValue::new(
                "conversation_id",
                conversation_id.to_string(),
            ));
            span.set_attribute(KeyValue::new("interface", format!("{interface:?}")));
            OtelContext::current().with_span(span)
        }
    };
    let mut otel_turn = tracer.start_with_context("turn", &conv_cx);
    otel_turn.set_attribute(KeyValue::new(
        "conversation_id",
        conversation_id.to_string(),
    ));
    otel_turn.set_attribute(KeyValue::new("interface", format!("{interface:?}")));
    let turn_cx = conv_cx.with_span(otel_turn);
    (conv_cx, turn_cx)
}

/// Convert a [`serde_json::Value`] to a flat `HashMap<String, Value>`.
///
/// If the value is an `Object`, its entries are cloned into the map.
/// Any other variant (or `Null`) yields an empty map.
pub(crate) fn value_to_params_map(value: &serde_json::Value) -> HashMap<String, serde_json::Value> {
    if let serde_json::Value::Object(map) = value {
        map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    } else {
        HashMap::new()
    }
}

// ── Image resize helpers ─────────────────────────────────────────────────────

/// Maximum encoded image size per provider (bytes).  Images exceeding this
/// limit are resized down before base64 encoding.
fn max_image_bytes_for_provider(provider: &str) -> usize {
    match provider {
        "anthropic" => 5 * 1024 * 1024, // ~5 MB
        "openai" => 20 * 1024 * 1024,   // ~20 MB
        _ => 5 * 1024 * 1024,           // conservative default
    }
}

/// Resize the raw image bytes (if necessary) and return a base64-encoded
/// string suitable for `ContentBlock::Image`.
///
/// The image is decoded, resized to fit within the provider's size limit,
/// and re-encoded in the original format.  If decoding or resizing fails,
/// the original bytes are returned as-is (best-effort).
fn resize_and_encode(raw: &[u8], mime_type: &str, provider: &str) -> String {
    use base64::Engine as _;
    use image::ImageFormat;

    let limit = max_image_bytes_for_provider(provider);

    // If already under the limit, skip decoding entirely.
    if raw.len() <= limit {
        return base64::engine::general_purpose::STANDARD.encode(raw);
    }

    let format = match mime_type {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        "image/gif" => ImageFormat::Gif,
        "image/webp" => ImageFormat::WebP,
        _ => {
            // Unknown format — return as-is.
            return base64::engine::general_purpose::STANDARD.encode(raw);
        }
    };

    let img = match image::load_from_memory_with_format(raw, format) {
        Ok(img) => img,
        Err(_) => {
            return base64::engine::general_purpose::STANDARD.encode(raw);
        }
    };

    // Iteratively halve dimensions until the encoded output fits.
    let mut current = img;
    for _ in 0..6 {
        let (w, h) = (current.width() / 2, current.height() / 2);
        if w == 0 || h == 0 {
            break;
        }
        current = current.resize(w, h, image::imageops::FilterType::Lanczos3);

        let mut buf = std::io::Cursor::new(Vec::new());
        if current.write_to(&mut buf, format).is_ok() {
            let encoded_bytes = buf.into_inner();
            if encoded_bytes.len() <= limit {
                return base64::engine::general_purpose::STANDARD.encode(&encoded_bytes);
            }
        }
    }

    // Fallback: return original bytes encoded.
    base64::engine::general_purpose::STANDARD.encode(raw)
}

/// Convert an attachment's bytes into the appropriate [`ContentBlock`] based
/// on its MIME type.
///
/// - **Images** → `ContentBlock::Image` (resized + base64 encoded)
/// - **PDF** → `ContentBlock::Document` (base64 encoded, no resize)
/// - **Audio** → `ContentBlock::Audio` (base64 encoded)
/// - **Text files** (plain, markdown, CSV, JSON) → `ContentBlock::Text`
///   (UTF-8 decoded, wrapped with filename delimiters)
fn attachment_to_content_block(
    meta: &AttachmentMeta,
    bytes: &[u8],
    provider: &str,
) -> ContentBlock {
    use base64::Engine as _;

    if is_resizable_mime_type(&meta.mime_type) {
        let encoded = resize_and_encode(bytes, &meta.mime_type, provider);
        return ContentBlock::Image {
            media_type: meta.mime_type.clone(),
            data: encoded,
        };
    }

    if is_text_mime_type(&meta.mime_type) {
        const MAX_INLINE_TEXT_BYTES: usize = 256 * 1024;
        let truncated = bytes.len() > MAX_INLINE_TEXT_BYTES;
        let inline_bytes = if truncated {
            &bytes[..MAX_INLINE_TEXT_BYTES]
        } else {
            bytes
        };
        let text = String::from_utf8_lossy(inline_bytes);
        let suffix = if truncated {
            format!(
                "\n--- truncated: showing first {} of {} bytes ---",
                MAX_INLINE_TEXT_BYTES,
                bytes.len()
            )
        } else {
            String::new()
        };
        return ContentBlock::Text(format!(
            "--- file: {} ---\n{}{}\n--- end file ---",
            meta.filename, text, suffix
        ));
    }

    if meta.mime_type.starts_with("audio/") {
        return ContentBlock::Audio {
            media_type: meta.mime_type.clone(),
            data: base64::engine::general_purpose::STANDARD.encode(bytes),
        };
    }

    if meta.mime_type == "application/pdf" {
        if provider != "anthropic" {
            return ContentBlock::Text(format!(
                "--- file: {} (application/pdf, {} bytes) ---\n\
                 PDF content not available: native PDF support requires the Anthropic provider.\n\
                 --- end file ---",
                meta.filename, meta.size_bytes
            ));
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        return ContentBlock::Document {
            media_type: meta.mime_type.clone(),
            data: encoded,
        };
    }

    // Unsupported type — include a placeholder so the LLM knows a file was attached.
    ContentBlock::Text(format!(
        "--- file: {} ({}, {} bytes) ---",
        meta.filename, meta.mime_type, meta.size_bytes
    ))
}
