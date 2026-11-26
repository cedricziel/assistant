//! Run event relay: carries a turn's [`OrchestratorEvent`]s from the process
//! that runs it to the process that streams it (ADR-0017, D2).
//!
//! The worker half ([`spawn_relay_sink`]) publishes each event on
//! [`topic::TURN_EVENT`] with the run id as `batch_id`. The streaming half
//! ([`RunEventRelay`]) claims those messages and hands the events back in
//! order.

use std::sync::Arc;
use std::time::Duration;

use assistant_core::{ClaimFilter, MessageBus, PublishRequest, topic};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::{debug, warn};
use uuid::Uuid;

use super::OrchestratorEvent;

/// `Token` and `Thinking` events are coalesced to at most one bus message per
/// window, so a streaming turn writes a few rows per second rather than one
/// per token.
pub(super) const COALESCE_WINDOW: Duration = Duration::from_millis(200);

/// How long the worker waits for the relay task to flush once the turn ends.
const FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

/// Worker-side event sink for a relayed turn.
pub(super) struct RelaySink {
    tx: Option<mpsc::Sender<OrchestratorEvent>>,
    task: JoinHandle<()>,
}

impl RelaySink {
    /// The sender the turn emits its events on.
    pub(super) fn sender(&self) -> Option<mpsc::Sender<OrchestratorEvent>> {
        self.tx.clone()
    }

    /// Wait until every event the turn emitted is on the bus. Call it after
    /// the turn returns and before its result is published, so the streaming
    /// side never sees the result ahead of the last events.
    pub(super) async fn finish(mut self) {
        self.tx = None;
        if tokio::time::timeout(FLUSH_TIMEOUT, &mut self.task)
            .await
            .is_err()
        {
            warn!("run event relay did not flush in time; dropping its tail");
            self.task.abort();
        }
    }
}

/// Where a relayed event is published.
#[derive(Clone)]
pub(super) struct RelayTarget {
    pub bus: Arc<dyn MessageBus>,
    pub agent_id: String,
    pub conversation_id: Uuid,
    pub run_id: Uuid,
}

/// Start publishing the events sent on the returned sink to the bus,
/// coalescing runs of `Token` or `Thinking` events per `window`.
pub(super) fn spawn_relay_sink(target: RelayTarget, window: Duration) -> RelaySink {
    let (tx, rx) = mpsc::channel(64);
    let task = tokio::spawn(relay_events(target, rx, window));
    RelaySink { tx: Some(tx), task }
}

/// The text a coalesced event carries and which variant to rebuild.
#[derive(Clone, Copy, PartialEq)]
enum TextKind {
    Token,
    Thinking,
}

impl TextKind {
    fn event(self, text: String) -> OrchestratorEvent {
        match self {
            Self::Token => OrchestratorEvent::Token(text),
            Self::Thinking => OrchestratorEvent::Thinking(text),
        }
    }
}

async fn relay_events(
    target: RelayTarget,
    mut rx: mpsc::Receiver<OrchestratorEvent>,
    window: Duration,
) {
    // Buffered text and the instant it must go out by.
    let mut pending: Option<(TextKind, String, Instant)> = None;
    loop {
        let event = match &pending {
            Some((_, _, flush_at)) => {
                tokio::select! {
                    event = rx.recv() => event,
                    _ = tokio::time::sleep_until(*flush_at) => {
                        flush(&target, &mut pending).await;
                        continue;
                    }
                }
            }
            None => rx.recv().await,
        };
        let Some(event) = event else { break };
        let (kind, text) = match event {
            OrchestratorEvent::Token(text) => (TextKind::Token, text),
            OrchestratorEvent::Thinking(text) => (TextKind::Thinking, text),
            other => {
                // Order matters: buffered text goes out before the event that
                // followed it.
                flush(&target, &mut pending).await;
                publish(&target, &other).await;
                continue;
            }
        };
        match &mut pending {
            Some((buffered, buffer, _)) if *buffered == kind => buffer.push_str(&text),
            _ => {
                flush(&target, &mut pending).await;
                pending = Some((kind, text, Instant::now() + window));
            }
        }
    }
    flush(&target, &mut pending).await;
}

async fn flush(target: &RelayTarget, pending: &mut Option<(TextKind, String, Instant)>) {
    if let Some((kind, text, _)) = pending.take() {
        publish(target, &kind.event(text)).await;
    }
}

async fn publish(target: &RelayTarget, event: &OrchestratorEvent) {
    let payload = match serde_json::to_value(event) {
        Ok(payload) => payload,
        Err(e) => {
            warn!(run_id = %target.run_id, error = %e, "failed to serialize relayed event");
            return;
        }
    };
    let request = PublishRequest::new(topic::TURN_EVENT, payload)
        .with_agent_id(target.agent_id.clone())
        .with_conversation_id(target.conversation_id)
        .with_batch_id(target.run_id);
    if let Err(e) = target.bus.publish(request).await {
        warn!(run_id = %target.run_id, error = %e, "failed to publish relayed event");
    }
}

/// Streaming-side reader of one run's relayed events.
///
/// Created with [`Orchestrator::run_event_relay`](super::Orchestrator::run_event_relay).
pub struct RunEventRelay {
    bus: Arc<dyn MessageBus>,
    filter: ClaimFilter,
}

impl RunEventRelay {
    pub(super) fn new(
        bus: Arc<dyn MessageBus>,
        agent_id: &str,
        conversation_id: Uuid,
        run_id: Uuid,
    ) -> Self {
        Self {
            bus,
            filter: ClaimFilter::new()
                .with_agent_id(agent_id)
                .with_conversation_id(conversation_id)
                .with_batch_id(run_id),
        }
    }

    /// The oldest relayed event not read yet, or `None` when none is waiting.
    pub async fn next(&self) -> Option<OrchestratorEvent> {
        loop {
            let message = match self
                .bus
                .claim_filtered(topic::TURN_EVENT, "run-event-relay", &self.filter)
                .await
            {
                Ok(Some(message)) => message,
                Ok(None) => return None,
                Err(e) => {
                    warn!(error = %e, "failed to claim a relayed event");
                    return None;
                }
            };
            let event = serde_json::from_value::<OrchestratorEvent>(message.payload.clone());
            // Ack either way: a message that does not parse never will.
            if let Err(e) = self.bus.ack(message.id).await {
                warn!(message_id = %message.id, error = %e, "failed to ack a relayed event");
            }
            match event {
                Ok(event) => return Some(event),
                Err(e) => {
                    debug!(message_id = %message.id, error = %e, "dropping an unreadable relayed event")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use assistant_storage::StorageLayer;
    use chrono::Utc;
    use serde_json::json;

    use crate::interaction::InteractionOutcome;

    use super::*;

    const WINDOW: Duration = Duration::from_millis(60);

    async fn target() -> (RelayTarget, Arc<dyn MessageBus>) {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
        let target = RelayTarget {
            bus: bus.clone(),
            agent_id: "default".into(),
            conversation_id: Uuid::new_v4(),
            run_id: Uuid::new_v4(),
        };
        (target, bus)
    }

    async fn published(bus: &Arc<dyn MessageBus>) -> Vec<serde_json::Value> {
        let mut messages = bus.list(topic::TURN_EVENT, None, 100).await.unwrap();
        messages.sort_by_key(|m| m.created_at);
        messages.into_iter().map(|m| m.payload).collect()
    }

    fn token(text: &str) -> OrchestratorEvent {
        OrchestratorEvent::Token(text.into())
    }

    #[tokio::test]
    async fn a_burst_of_text_is_one_message_and_keeps_its_order() {
        let (target, bus) = target().await;
        let sink = spawn_relay_sink(target, WINDOW);
        let tx = sink.sender().unwrap();
        for event in [
            token("a"),
            token("b"),
            OrchestratorEvent::Thinking("t".into()),
            OrchestratorEvent::Status {
                message: "working".into(),
                tool_call_id: None,
            },
            token("c"),
        ] {
            tx.send(event).await.unwrap();
        }
        drop(tx);
        sink.finish().await;

        let events = published(&bus).await;
        let shapes: Vec<_> = events
            .iter()
            .map(|e| (e["type"].as_str().unwrap(), e["data"].clone()))
            .collect();
        assert_eq!(
            shapes,
            vec![
                ("token", json!("ab")),
                ("thinking", json!("t")),
                (
                    "status",
                    json!({"message": "working", "tool_call_id": null})
                ),
                ("token", json!("c")),
            ]
        );
    }

    #[tokio::test]
    async fn text_further_apart_than_the_window_goes_out_separately() {
        let (target, bus) = target().await;
        let sink = spawn_relay_sink(target, WINDOW);
        let tx = sink.sender().unwrap();
        tx.send(token("a")).await.unwrap();
        tokio::time::sleep(WINDOW * 3).await;
        assert_eq!(
            published(&bus).await.len(),
            1,
            "the buffered text goes out when the window closes, not at turn end"
        );
        tx.send(token("b")).await.unwrap();
        drop(tx);
        sink.finish().await;

        assert_eq!(published(&bus).await.len(), 2);
    }

    #[tokio::test]
    async fn the_reader_returns_one_runs_events_in_order_then_none() {
        let (target, bus) = target().await;
        let other_run = RelayTarget {
            run_id: Uuid::new_v4(),
            ..target.clone()
        };
        for (t, text) in [(&target, "one"), (&other_run, "other"), (&target, "two")] {
            publish(t, &token(text)).await;
        }

        let relay = RunEventRelay::new(bus, "default", target.conversation_id, target.run_id);
        let mut read = Vec::new();
        while let Some(OrchestratorEvent::Token(text)) = relay.next().await {
            read.push(text);
        }
        assert_eq!(read, vec!["one", "two"]);
        assert!(relay.next().await.is_none());
    }

    #[test]
    fn the_wire_form_round_trips_every_nested_shape() {
        let events = vec![
            OrchestratorEvent::SubagentEvent {
                agent_id: "a".into(),
                inner: Box::new(token("x")),
            },
            OrchestratorEvent::InteractionResolved {
                interaction_id: "i".into(),
                kind: "tool_approval".into(),
                resolved_at: Utc::now(),
                outcome: InteractionOutcome::Resolved {
                    response: json!({"decision": "allow"}),
                    rule_id: None,
                },
            },
            OrchestratorEvent::InteractionResolved {
                interaction_id: "i".into(),
                kind: "question".into(),
                resolved_at: Utc::now(),
                outcome: InteractionOutcome::Expired,
            },
        ];
        for event in events {
            let wire = serde_json::to_value(&event).unwrap();
            let back: OrchestratorEvent = serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(serde_json::to_value(&back).unwrap(), wire);
        }
    }
}
