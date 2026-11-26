//! Turn interactions: a running turn asks its owner a typed question and
//! waits for the answer (ADR-0017).
//!
//! - [`broker`] persists interactions and waits for their answers.
//! - [`kinds`] is the registry of interaction kinds (`tool_approval`,
//!   `question`, `surface_action`, and whatever siblings add).

pub mod broker;
pub mod kinds;

pub use broker::{
    InteractionBroker, InteractionRequest, RespondError, Responder, WAITER_STALE_AFTER,
    resolved_event,
};
pub use kinds::{
    InteractionKindHandler, InteractionKinds, InteractionOutcome, MAX_ANSWER_CHARS,
    MAX_ARGUMENTS_BYTES, MAX_SURFACE_ACTION_BYTES, PushCopy, QuestionKind, SurfaceActionKind,
    ToolApprovalKind, cap_arguments,
};
