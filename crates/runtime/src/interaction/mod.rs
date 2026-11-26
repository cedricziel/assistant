//! Turn interactions: a running turn asks its owner a typed question and
//! waits for the answer (ADR-0017).
//!
//! - [`broker`] persists interactions and waits for their answers.
//! - [`kinds`] is the registry of interaction kinds (`tool_approval`,
//!   `question`, and whatever siblings add).

pub mod broker;
pub mod kinds;

pub use broker::{InteractionBroker, InteractionRequest, RespondError, Responder};
pub use kinds::{
    InteractionKindHandler, InteractionKinds, InteractionOutcome, MAX_ANSWER_CHARS,
    MAX_ARGUMENTS_BYTES, PushCopy, QuestionKind, ToolApprovalKind, cap_arguments,
};
