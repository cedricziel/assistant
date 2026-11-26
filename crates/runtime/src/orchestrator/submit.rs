//! [`SubmitTurn`] — everything a producer decides about one turn.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use assistant_core::auth::AuthContext;
use assistant_core::bus_messages::{TurnAnchor, TurnRequest};
use assistant_core::types::conversation::{Interface, TurnIdentity};

use super::TurnLease;

/// One turn to submit through [`Orchestrator::submit`](super::Orchestrator::submit).
///
/// [`SubmitTurn::new`] is a plain, non-interactive append from an
/// unauthenticated producer; the `with_*` methods and the public fields
/// override what differs.
#[derive(Debug)]
pub struct SubmitTurn {
    /// The user's prompt text.
    pub prompt: String,
    /// Conversation to continue (or start).
    pub conversation_id: Uuid,
    /// Originating interface.
    pub interface: Interface,
    /// When the interface received the message; the worker injects it into
    /// the prompt as a time prefix.
    pub timestamp: Option<DateTime<Utc>>,
    /// User-provided attachments for this turn.
    pub attachment_ids: Vec<Uuid>,
    /// The user, org and space the turn runs for.
    pub identity: TurnIdentity,
    /// Caller-chosen request id. The SSE `run_id` doubles as it so clients can
    /// cancel by the id they already observe. A fresh id is drawn when `None`.
    pub request_id: Option<Uuid>,
    /// A human is waiting on this turn and can answer prompts (tool approvals,
    /// questions). Reaches tools as `ExecutionContext::interactive`. When
    /// `false`, tools that need a human's answer are denied instead of
    /// prompting.
    pub interactive: bool,
    /// The submitter streams the turn from another process than the one that
    /// runs it. When the worker finds no local event sink it relays each event
    /// over the bus ([`topic::TURN_EVENT`](assistant_core::bus_messages::topic::TURN_EVENT)),
    /// and the submitter reads them with [`Orchestrator::run_event_relay`].
    ///
    /// [`Orchestrator::run_event_relay`]: super::Orchestrator::run_event_relay
    pub relay_events: bool,
    /// Where the turn's rows attach in the message tree.
    pub anchor: TurnAnchor,
    /// The id to persist the user row under, when the producer pre-assigns one.
    pub user_message_id: Option<Uuid>,
    /// Model for this turn only, overriding the conversation and org defaults.
    pub model_override: Option<String>,
    /// The lease the caller already holds for this turn. [`Orchestrator::submit`]
    /// takes a shared lease itself when this is `None`.
    ///
    /// [`Orchestrator::submit`]: super::Orchestrator::submit
    pub lease: Option<TurnLease>,
}

impl SubmitTurn {
    /// A non-interactive append of `prompt` to `conversation_id`.
    pub fn new(prompt: impl Into<String>, conversation_id: Uuid, interface: Interface) -> Self {
        Self {
            prompt: prompt.into(),
            conversation_id,
            interface,
            timestamp: None,
            attachment_ids: Vec::new(),
            identity: TurnIdentity::default(),
            request_id: None,
            interactive: false,
            relay_events: false,
            anchor: TurnAnchor::default(),
            user_message_id: None,
            model_override: None,
            lease: None,
        }
    }

    /// Run the turn as the user and org of `auth`.
    pub fn with_auth(mut self, auth: &AuthContext) -> Self {
        self.identity = TurnIdentity::from_auth(auth);
        self
    }

    /// Record when the interface received the message.
    pub fn with_timestamp(mut self, timestamp: Option<DateTime<Utc>>) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Attach user-provided attachments.
    pub fn with_attachments(mut self, attachment_ids: Vec<Uuid>) -> Self {
        self.attachment_ids = attachment_ids;
        self
    }

    /// Use `request_id` as the turn's request (and run) id.
    pub fn with_request_id(mut self, request_id: Uuid) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Run the turn under a lease the caller took, such as an exclusive lease
    /// for an edit. The lease is released when the turn ends. Its request id is
    /// the turn's request id unless [`Self::with_request_id`] says otherwise.
    pub fn with_lease(mut self, lease: TurnLease) -> Self {
        self.lease = Some(lease);
        self
    }

    /// Mark whether a human is waiting on the turn.
    pub fn with_interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Ask the worker to relay the turn's events over the bus when it has no
    /// local sink.
    pub fn with_relay_events(mut self, relay_events: bool) -> Self {
        self.relay_events = relay_events;
        self
    }

    /// The bus envelope for this turn.
    pub(super) fn to_request(&self, traceparent: Option<String>) -> TurnRequest {
        TurnRequest {
            prompt: self.prompt.clone(),
            conversation_id: self.conversation_id,
            extension_tools: vec![],
            timestamp: self.timestamp,
            traceparent,
            attachment_ids: self.attachment_ids.clone(),
            interactive: self.interactive,
            relay_events: self.relay_events,
            anchor: self.anchor.clone(),
            user_message_id: self.user_message_id,
            model_override: self.model_override.clone(),
            user_id: self.identity.user_id.as_ref().map(|u| u.to_string()),
            org_id: self.identity.org_id.as_ref().map(|o| o.to_string()),
            space_id: self.identity.space_id.as_ref().map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use assistant_core::bus_messages::TurnAnchor;
    use assistant_core::types::conversation::Interface;

    use super::SubmitTurn;

    #[test]
    fn new_defaults_match_a_plain_submit() {
        let conversation_id = Uuid::new_v4();
        let turn = SubmitTurn::new("hi", conversation_id, Interface::Cli);
        assert_eq!(turn.prompt, "hi");
        assert_eq!(turn.conversation_id, conversation_id);
        assert_eq!(turn.interface, Interface::Cli);
        assert_eq!(turn.timestamp, None);
        assert!(turn.attachment_ids.is_empty());
        assert_eq!(turn.identity.user_id, None);
        assert_eq!(turn.identity.org_id, None);
        assert_eq!(turn.request_id, None);
        assert!(!turn.interactive, "a turn is not interactive by default");
        assert!(
            !turn.relay_events,
            "a turn does not relay events by default"
        );
        assert_eq!(turn.anchor, TurnAnchor::Append);
        assert_eq!(turn.user_message_id, None);
        assert_eq!(turn.model_override, None);
    }

    #[test]
    fn request_carries_the_submit_fields() {
        let attachment = Uuid::new_v4();
        let user_message_id = Uuid::new_v4();
        let mut turn = SubmitTurn::new("hi", Uuid::new_v4(), Interface::Web)
            .with_attachments(vec![attachment])
            .with_interactive(true)
            .with_relay_events(true);
        turn.anchor = TurnAnchor::Regenerate { user_message_id };
        turn.user_message_id = Some(user_message_id);
        turn.model_override = Some("m".into());

        let request = turn.to_request(Some("tp".into()));
        assert_eq!(request.prompt, "hi");
        assert_eq!(request.conversation_id, turn.conversation_id);
        assert_eq!(request.attachment_ids, vec![attachment]);
        assert!(request.interactive);
        assert!(request.relay_events);
        assert_eq!(request.traceparent.as_deref(), Some("tp"));
        assert_eq!(request.anchor, TurnAnchor::Regenerate { user_message_id });
        assert_eq!(request.user_message_id, Some(user_message_id));
        assert_eq!(request.model_override.as_deref(), Some("m"));
    }
}
