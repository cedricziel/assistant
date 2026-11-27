//! Push the turn owner when an interaction waits (ADR-0017).
//!
//! The shared turn runner feeds every projected frame to an
//! [`InteractionPushScheduler`]. An `interaction_requested` frame starts a
//! timer; if the interaction is still unanswered when it fires, the owner's
//! subscriptions (and nobody else's) get one push with the kind's copy. An
//! `interaction_resolved` frame cancels the timer. The delay keeps the push
//! from buzzing a user who is looking at the screen: the service worker has to
//! show every push it receives, so it cannot suppress one itself.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use tracing::warn;

use assistant_runtime::interaction::InteractionKinds;
use assistant_runtime::projection::SseFrame;
use assistant_storage::PushSubscriptionStore;

use crate::push::{PushDispatcher, PushMessage};

/// How long an interaction may wait before the owner is pushed.
pub(crate) const PUSH_DELAY: Duration = Duration::from_secs(30);

/// Longest push body, in characters.
const BODY_LIMIT: usize = 80;

/// Where a turn's interaction pushes go and what they say.
struct PushTarget {
    dispatcher: Arc<PushDispatcher>,
    store: Arc<dyn PushSubscriptionStore>,
    kinds: Arc<InteractionKinds>,
    /// The turn's owner: the only user who gets the push.
    owner: String,
    conversation_id: String,
}

/// What one waiting interaction's push says, owned so the timer can move it.
struct PendingPush {
    title: String,
    body: String,
    conversation_id: String,
    tag: String,
}

impl PendingPush {
    /// The push for an `interaction_requested` frame, or `None` when the frame
    /// is something else or its kind registered no push copy.
    fn for_frame(
        kinds: &InteractionKinds,
        conversation_id: &str,
        frame: &SseFrame,
    ) -> Option<Self> {
        if frame.event != "interaction_requested" {
            return None;
        }
        let interaction_id = frame.log_payload["interaction_id"].as_str()?;
        let copy = frame.log_payload["kind"]
            .as_str()
            .and_then(|kind| kinds.get(kind))
            .map(|handler| handler.push_copy(&frame.log_payload))?;
        Some(Self {
            title: copy.title,
            body: copy.body.chars().take(BODY_LIMIT).collect(),
            conversation_id: conversation_id.to_string(),
            tag: format!("interaction-{interaction_id}"),
        })
    }

    fn message(&self) -> PushMessage<'_> {
        PushMessage {
            title: &self.title,
            body: &self.body,
            conversation_id: Some(&self.conversation_id),
            tag: Some(&self.tag),
        }
    }
}

/// Schedules and cancels the delayed push of each interaction of one turn.
/// Inert without a dispatcher, so callers feed it unconditionally.
pub(crate) struct InteractionPushScheduler {
    target: Option<Arc<PushTarget>>,
    /// How long an interaction may wait before its push goes out. Production
    /// uses [`PUSH_DELAY`]; tests shrink it to keep their clocks real.
    delay: Duration,
    /// Timers of the interactions still waiting, by interaction id.
    pending: HashMap<String, JoinHandle<()>>,
}

impl InteractionPushScheduler {
    pub(crate) fn new(
        dispatcher: Option<Arc<PushDispatcher>>,
        store: Arc<dyn PushSubscriptionStore>,
        kinds: Arc<InteractionKinds>,
        owner: String,
        conversation_id: String,
    ) -> Self {
        Self {
            target: dispatcher.map(|dispatcher| {
                Arc::new(PushTarget {
                    dispatcher,
                    store,
                    kinds,
                    owner,
                    conversation_id,
                })
            }),
            pending: HashMap::new(),
            delay: PUSH_DELAY,
        }
    }

    /// Start or cancel a timer for `frame`; other frames are ignored.
    pub(crate) fn observe(&mut self, frame: &SseFrame) {
        let Some(target) = &self.target else {
            return;
        };
        let Some(interaction_id) = frame.log_payload["interaction_id"].as_str() else {
            return;
        };
        match frame.event.as_str() {
            "interaction_requested" => {
                let Some(pending_push) =
                    PendingPush::for_frame(&target.kinds, &target.conversation_id, frame)
                else {
                    return;
                };
                let target = target.clone();
                let delay = self.delay;
                let timer = tokio::spawn(async move {
                    tokio::time::sleep(delay).await;
                    if let Err(e) = target
                        .dispatcher
                        .send_to_user(&*target.store, &target.owner, &pending_push.message())
                        .await
                    {
                        warn!("Push (interaction) failed: {e}");
                    }
                });
                if let Some(previous) = self.pending.insert(interaction_id.to_string(), timer) {
                    // A repeated request for the same id replaces its timer.
                    previous.abort();
                }
            }
            "interaction_resolved" => {
                if let Some(timer) = self.pending.remove(interaction_id) {
                    timer.abort();
                }
            }
            _ => {}
        }
    }

    /// Cancel every timer: the turn ended, so nobody is left to answer.
    pub(crate) fn finish(&mut self) {
        for (_, timer) in self.pending.drain() {
            timer.abort();
        }
    }
}

impl Drop for InteractionPushScheduler {
    fn drop(&mut self) {
        self.finish();
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use chrono::Utc;
    use serde_json::json;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use assistant_runtime::OrchestratorEvent;
    use assistant_runtime::interaction::InteractionOutcome;
    use assistant_runtime::projection::{SseProjector, StreamProjector};
    use assistant_storage::InMemoryPushSubscriptionStore;

    use super::*;

    const OWNER: &str = "alice";

    /// The test scheduler's wait before pushing; production uses [`PUSH_DELAY`].
    const TEST_DELAY: Duration = Duration::from_millis(50);

    fn frame(event: OrchestratorEvent) -> SseFrame {
        SseProjector
            .project(&event)
            .into_iter()
            .next()
            .expect("the event projects to a frame")
    }

    fn requested(id: &str, kind: &str, request: serde_json::Value) -> SseFrame {
        frame(OrchestratorEvent::InteractionRequested {
            interaction_id: id.into(),
            kind: kind.into(),
            tool_call_id: None,
            created_at: Utc::now(),
            expires_at: Utc::now(),
            renders_card: true,
            request,
        })
    }

    fn resolved(id: &str) -> SseFrame {
        frame(OrchestratorEvent::InteractionResolved {
            interaction_id: id.into(),
            kind: "tool_approval".into(),
            resolved_at: Utc::now(),
            outcome: InteractionOutcome::Cancelled,
        })
    }

    fn approval(id: &str) -> SseFrame {
        requested(id, "tool_approval", json!({"tool_name": "bash"}))
    }

    /// A scheduler for a turn owned by [`OWNER`]; the owner has one browser
    /// at `/alice` and another member one at `/bob` of `server`.
    async fn scheduler(server: &MockServer) -> InteractionPushScheduler {
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(201))
            .mount(server)
            .await;
        let store = InMemoryPushSubscriptionStore::new();
        for (path, user) in [("alice", OWNER), ("bob", "bob")] {
            let subscriber = p256::ecdsa::SigningKey::from_slice(&[7u8; 32]).unwrap();
            let p256dh =
                URL_SAFE_NO_PAD.encode(subscriber.verifying_key().to_sec1_point(false).as_bytes());
            store
                .upsert(
                    &format!("{}/{path}", server.uri()),
                    &p256dh,
                    &URL_SAFE_NO_PAD.encode([7u8; 16]),
                    Some(user),
                )
                .await
                .unwrap();
        }
        let (vapid_private, _) = crate::push::generate_vapid_keypair().unwrap();
        let mut scheduler = InteractionPushScheduler::new(
            Some(Arc::new(PushDispatcher::new(vapid_private))),
            Arc::new(store),
            Arc::new(InteractionKinds::with_builtin()),
            OWNER.into(),
            "conv-1".into(),
        );
        // Keep the test clocks real: a shorter delay than production.
        scheduler.delay = TEST_DELAY;
        scheduler
    }

    /// Request paths the push server has seen, oldest first.
    async fn pushed_paths(server: &MockServer) -> Vec<String> {
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .map(|r| r.url.path().to_string())
            .collect()
    }

    /// Let the timers run and any push they sent land: poll until `expected`
    /// requests have arrived, then give a push that should not exist time to
    /// show up anyway.
    async fn settle(server: &MockServer, expected: usize) -> Vec<String> {
        for _ in 0..100 {
            let paths = pushed_paths(server).await;
            if paths.len() >= expected {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        tokio::time::sleep(TEST_DELAY * 2).await;
        pushed_paths(server).await
    }

    #[tokio::test]
    async fn an_interaction_pending_past_the_delay_pushes_the_owner_only() {
        let server = MockServer::start().await;
        let mut scheduler = scheduler(&server).await;

        scheduler.observe(&approval("i-1"));
        tokio::time::sleep(TEST_DELAY / 2).await;
        assert!(
            pushed_paths(&server).await.is_empty(),
            "nothing is pushed before the delay"
        );

        assert_eq!(settle(&server, 1).await, ["/alice"]);
    }

    #[tokio::test]
    async fn an_interaction_resolved_before_the_delay_pushes_nobody() {
        let server = MockServer::start().await;
        let mut scheduler = scheduler(&server).await;

        scheduler.observe(&approval("i-1"));
        tokio::time::sleep(TEST_DELAY / 2).await;
        scheduler.observe(&resolved("i-1"));

        assert!(settle(&server, 0).await.is_empty());
    }

    #[tokio::test]
    async fn each_pending_interaction_pushes_once_and_a_resolved_one_not_at_all() {
        let server = MockServer::start().await;
        let mut scheduler = scheduler(&server).await;

        scheduler.observe(&approval("i-1"));
        scheduler.observe(&approval("i-2"));
        scheduler.observe(&resolved("i-2"));

        assert_eq!(settle(&server, 1).await, ["/alice"]);
    }

    #[tokio::test]
    async fn finishing_the_turn_cancels_the_waiting_pushes() {
        let server = MockServer::start().await;
        let mut scheduler = scheduler(&server).await;

        scheduler.observe(&approval("i-1"));
        scheduler.finish();

        assert!(settle(&server, 0).await.is_empty());
    }

    #[tokio::test]
    async fn without_a_dispatcher_nothing_is_scheduled() {
        let mut scheduler = InteractionPushScheduler::new(
            None,
            Arc::new(InMemoryPushSubscriptionStore::new()),
            Arc::new(InteractionKinds::with_builtin()),
            OWNER.into(),
            "conv-1".into(),
        );
        scheduler.observe(&approval("i-1"));
        assert!(scheduler.pending.is_empty());
    }

    #[test]
    fn the_push_carries_the_kinds_copy_tags_and_conversation() {
        let kinds = InteractionKinds::with_builtin();
        let pending = PendingPush::for_frame(&kinds, "conv-1", &approval("i-1"))
            .expect("an approval has a push");
        assert_eq!(pending.title, "Approval needed");
        assert!(pending.body.contains("'bash'"), "{}", pending.body);
        assert_eq!(pending.tag, "interaction-i-1");
        assert_eq!(pending.conversation_id, "conv-1");

        let message = pending.message();
        assert_eq!(message.conversation_id, Some("conv-1"));
        assert_eq!(message.tag, Some("interaction-i-1"));
    }

    #[test]
    fn a_long_question_is_truncated_to_80_characters() {
        let kinds = InteractionKinds::with_builtin();
        let long = "Which ".to_string() + &"branch ".repeat(30);
        let frame = requested(
            "q-1",
            "question",
            json!({"questions": [{"id": "q", "prompt": long}]}),
        );

        let pending = PendingPush::for_frame(&kinds, "conv-1", &frame).expect("a push");

        assert_eq!(pending.title, "The assistant has a question");
        assert_eq!(pending.body.chars().count(), BODY_LIMIT);
        assert!(long.starts_with(&pending.body), "{}", pending.body);
    }

    #[test]
    fn a_resolved_frame_starts_no_push() {
        let kinds = InteractionKinds::with_builtin();
        assert!(PendingPush::for_frame(&kinds, "conv-1", &resolved("i-1")).is_none());
    }

    #[test]
    fn a_kind_nobody_registered_gets_no_push() {
        let mut scheduler = InteractionPushScheduler::new(
            Some(Arc::new(PushDispatcher::new("AA".into()))),
            Arc::new(InMemoryPushSubscriptionStore::new()),
            Arc::new(InteractionKinds::with_builtin()),
            OWNER.into(),
            "conv-1".into(),
        );
        scheduler.observe(&requested("i-1", "mystery", json!({})));
        assert!(scheduler.pending.is_empty());
    }
}
