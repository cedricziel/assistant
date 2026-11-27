//! Turn interactions: a running turn asks its owner a typed question and
//! waits for the answer (ADR-0017).
//!
//! - [`broker`] persists interactions and waits for their answers.
//! - [`ask_user`] is the `ask-user` tool that puts questions to the owner.
//! - [`confirmation`] answers tool approvals through the broker.
//! - [`kinds`] is the registry of interaction kinds (`tool_approval`,
//!   `question`, and whatever siblings add).

pub mod ask_user;
pub mod broker;
pub mod confirmation;
pub mod kinds;

pub use ask_user::{ASK_USER_TOOL, AskUserTool, ToolCallScope};
pub use broker::{
    InteractionBroker, InteractionRequest, RespondError, Responder, WAITER_STALE_AFTER,
    resolved_event,
};
pub use confirmation::{InteractiveConfirmation, RoutedConfirmation};
pub use kinds::{
    InteractionKindHandler, InteractionKinds, InteractionOutcome, MAX_ANSWER_CHARS,
    MAX_ARGUMENTS_BYTES, PushCopy, QuestionKind, SavedRule, ToolApprovalKind, cap_arguments,
};
