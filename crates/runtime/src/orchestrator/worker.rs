//! Bus-based turn-processing worker for the orchestrator.
//!
//! These methods handle claiming messages from the message bus, dispatching
//! them to the appropriate `run_turn*` variant, and publishing results back.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::{bus_messages, topic, ClaimFilter, Interface, PublishRequest, ToolHandler};
use assistant_llm::ContentBlock;
use chrono::{DateTime, Local, Utc};
use opentelemetry::{
    trace::{Span as _, SpanKind, TraceContextExt},
    Context as OtelContext, KeyValue,
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{parse_interface, ExtensionRegistration, Orchestrator, TurnResult};
use crate::webhook_dispatch;

impl Orchestrator {
    // ── Bus-based turn processing ────────────────────────────────────────────

    /// Register a token sink for a streaming turn.
    ///
    /// Call this *before* publishing the [`TurnRequest`](bus_messages::TurnRequest)
    /// to the bus.  The worker will consume (remove) the sink when it processes
    /// the request, routing tokens through it via
    /// [`run_turn_streaming`](Self::run_turn_streaming).
    pub async fn register_token_sink(&self, conversation_id: Uuid, sink: mpsc::Sender<String>) {
        self.token_sinks.write().await.insert(conversation_id, sink);
    }

    /// Register extension tools and attachments for an interface-specific turn.
    ///
    /// Call this *before* publishing the [`TurnRequest`](bus_messages::TurnRequest)
    /// to the bus.  The worker will consume the registration when it processes
    /// the request, routing to
    /// [`run_turn_with_tools`](Self::run_turn_with_tools).
    pub async fn register_extensions(
        &self,
        conversation_id: Uuid,
        tools: Vec<Arc<dyn ToolHandler>>,
        attachments: Vec<ContentBlock>,
    ) {
        self.extension_registrations.write().await.insert(
            conversation_id,
            ExtensionRegistration { tools, attachments },
        );
    }

    /// Record a deduplicated Slack event for observability.
    pub fn record_slack_duplicate_event(&self, event_kind: &str) {
        self.metrics
            .record_slack_duplicate_event(&self.agent_id, event_kind);
    }

    /// Run the turn-processing worker loop.
    ///
    /// Claims messages from the [`topic::TURN_REQUEST`] topic and dispatches
    /// them to the appropriate processing method:
    ///
    /// - **Extension tools registered** → [`run_turn_with_tools`](Self::run_turn_with_tools)
    /// - **Token sink registered** → [`run_turn_streaming`](Self::run_turn_streaming)
    /// - **Neither** → [`run_turn`](Self::run_turn)
    ///
    /// After processing, a [`TurnResult`](bus_messages::TurnResult) is
    /// published to [`topic::TURN_RESULT`].
    ///
    /// This method runs indefinitely and should be spawned as a background
    /// task.  It exits when the tokio task is cancelled / dropped.
    ///
    /// ```rust,ignore
    /// let orch = Arc::new(orchestrator);
    /// tokio::spawn({
    ///     let orch = orch.clone();
    ///     async move { orch.run_worker("worker-1").await }
    /// });
    /// ```
    pub async fn run_worker(&self, worker_id: &str) {
        self.run_worker_filtered(worker_id, None).await;
    }

    /// Run a turn-processing worker that only claims messages for the given
    /// interface.  Pass `None` to claim messages for any interface (the
    /// original `run_worker` behaviour).
    ///
    /// When multiple services share the same SQLite database, each service
    /// should scope its worker to its own interface so one service doesn't
    /// steal turns from another.
    pub async fn run_worker_filtered(&self, worker_id: &str, interface: Option<&str>) {
        info!(worker_id, ?interface, "Turn worker started");
        let mut filter = ClaimFilter::new().with_agent_id(self.agent_id.clone());
        if let Some(iface) = interface {
            filter = filter.with_interface(iface);
        }
        loop {
            match self
                .bus
                .claim_filtered(topic::TURN_REQUEST, worker_id, &filter)
                .await
            {
                Ok(Some(msg)) => {
                    let turn_req: bus_messages::TurnRequest =
                        match serde_json::from_value(msg.payload.clone()) {
                            Ok(req) => req,
                            Err(e) => {
                                warn!(
                                    error = %e,
                                    msg_id = %msg.id,
                                    "Failed to deserialize TurnRequest"
                                );
                                let _ = self.bus.fail(msg.id).await;
                                continue;
                            }
                        };

                    let interface = msg
                        .interface
                        .as_deref()
                        .map(parse_interface)
                        .unwrap_or(Interface::Cli);

                    let conv_id = turn_req.conversation_id;
                    let trace_cx = turn_req
                        .traceparent
                        .as_deref()
                        .map(crate::otel_spans::context_from_traceparent);
                    let consume_parent_cx = trace_cx.clone().unwrap_or_else(OtelContext::current);
                    let mut bus_consume_span = crate::otel_spans::start_bus_span(
                        SpanKind::Consumer,
                        topic::TURN_REQUEST,
                        Some(conv_id),
                        &consume_parent_cx,
                    );
                    bus_consume_span
                        .set_attribute(KeyValue::new("bus.message_id", msg.id.to_string()));
                    if let Some(batch_id) = msg.batch_id {
                        bus_consume_span
                            .set_attribute(KeyValue::new("bus.batch_id", batch_id.to_string()));
                        bus_consume_span.set_attribute(KeyValue::new(
                            "submit.request_id",
                            batch_id.to_string(),
                        ));
                    }
                    if let Some(correlation_id) = msg.correlation_id {
                        bus_consume_span.set_attribute(KeyValue::new(
                            "bus.correlation_id",
                            correlation_id.to_string(),
                        ));
                    }
                    let bus_consume_cx = consume_parent_cx.with_span(bus_consume_span);
                    let interface_name = format!("{:?}", interface);

                    debug!(
                        conversation_id = %conv_id,
                        worker_id,
                        "Processing turn request"
                    );

                    // Prepend a timestamp prefix so the agent knows when the
                    // message arrived: `[YYYY-MM-DD HH:MM:SS TZ] <prompt>`.
                    let prompt = if let Some(ts) = turn_req.timestamp {
                        let local = ts.with_timezone(&Local);
                        format!(
                            "[{}] {}",
                            local.format("%Y-%m-%d %H:%M:%S %Z"),
                            turn_req.prompt
                        )
                    } else {
                        turn_req.prompt.clone()
                    };

                    // Check for registered side-channel resources.
                    let ext = self.extension_registrations.write().await.remove(&conv_id);
                    let token_sink = self.token_sinks.write().await.remove(&conv_id);

                    // Dispatch to the appropriate processing method.
                    let result: Result<TurnResult> = if let Some(reg) = ext {
                        // Extension-tool turn (Slack, Mattermost).
                        self.run_turn_with_tools(
                            &prompt,
                            conv_id,
                            interface,
                            reg.tools,
                            Some(&bus_consume_cx),
                            reg.attachments,
                        )
                        .await
                    } else if let Some(sink) = token_sink {
                        // Streaming turn (CLI, Signal).
                        self.run_turn_streaming(
                            &prompt,
                            conv_id,
                            interface,
                            sink,
                            Some(&bus_consume_cx),
                        )
                        .await
                    } else {
                        // Standard non-streaming turn.
                        self.run_turn(&prompt, conv_id, interface, Some(&bus_consume_cx))
                            .await
                    };

                    match result {
                        Ok(turn_result) => {
                            let bus_result = bus_messages::TurnResult {
                                conversation_id: conv_id,
                                content: turn_result.answer,
                                turn: 0,
                                attachments: turn_result.attachments,
                            };

                            // Propagate batch_id from the request so submit_turn
                            // can match the result to its specific request.
                            let mut pub_req = PublishRequest::new(
                                topic::TURN_RESULT,
                                serde_json::to_value(&bus_result).unwrap_or_default(),
                            )
                            .with_agent_id(self.agent_id.clone())
                            .with_conversation_id(conv_id)
                            .with_causation_id(msg.id);
                            if let Some(bid) = msg.batch_id {
                                pub_req = pub_req.with_batch_id(bid);
                            } else if msg.correlation_id.is_some() {
                                self.metrics.record_submit_result_unmatched(
                                    &self.agent_id,
                                    &interface_name,
                                    "missing_request_id",
                                );
                                warn!(
                                    conversation_id = %conv_id,
                                    msg_id = %msg.id,
                                    "turn.request missing batch_id; submit_turn correlation may fail"
                                );
                            }
                            if let Some(correlation_id) = msg.correlation_id.or(msg.batch_id) {
                                pub_req = pub_req.with_correlation_id(correlation_id);
                            }

                            let mut produce_result_span = crate::otel_spans::start_bus_span(
                                SpanKind::Producer,
                                topic::TURN_RESULT,
                                Some(conv_id),
                                &bus_consume_cx,
                            );
                            match self.bus.publish(pub_req).await {
                                Ok(published_id) => {
                                    produce_result_span.set_attribute(KeyValue::new(
                                        "bus.message_id",
                                        published_id.to_string(),
                                    ));
                                    produce_result_span
                                        .set_attribute(KeyValue::new("bus.status", "ok"));
                                    produce_result_span.end();
                                    if let Err(e) = self.bus.ack(msg.id).await {
                                        warn!(
                                            error = %e,
                                            msg_id = %msg.id,
                                            "Failed to ack bus message"
                                        );
                                    }
                                    let event_payload = serde_json::json!({
                                        "conversation_id": conv_id,
                                        "content": bus_result.content,
                                        "turn": bus_result.turn,
                                        "attachments": bus_result.attachments,
                                        "interface": interface_name,
                                    });
                                    if let Err(e) = webhook_dispatch::dispatch_event(
                                        self.storage.as_ref(),
                                        &self.agent_id,
                                        topic::TURN_RESULT,
                                        event_payload,
                                    )
                                    .await
                                    {
                                        warn!(
                                            conversation_id = %conv_id,
                                            error = %e,
                                            "Failed to dispatch turn.result webhooks"
                                        );
                                    }
                                    info!(
                                        conversation_id = %conv_id,
                                        worker_id,
                                        "Turn completed via worker"
                                    );
                                    bus_consume_cx
                                        .span()
                                        .set_attribute(KeyValue::new("bus.status", "ok"));
                                    bus_consume_cx.span().end();
                                }
                                Err(e) => {
                                    produce_result_span
                                        .set_attribute(KeyValue::new("bus.status", "error"));
                                    produce_result_span.set_attribute(KeyValue::new("error", true));
                                    produce_result_span.set_attribute(KeyValue::new(
                                        "error.message",
                                        e.to_string(),
                                    ));
                                    produce_result_span.end();
                                    warn!(error = %e, "Failed to publish TurnResult, nacking request");
                                    let _ = self.bus.nack(msg.id).await;
                                    bus_consume_cx
                                        .span()
                                        .set_attribute(KeyValue::new("bus.status", "error"));
                                    bus_consume_cx
                                        .span()
                                        .set_attribute(KeyValue::new("error", true));
                                    bus_consume_cx.span().set_attribute(KeyValue::new(
                                        "error.message",
                                        e.to_string(),
                                    ));
                                    bus_consume_cx.span().end();
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                conversation_id = %conv_id,
                                worker_id,
                                "Turn failed in worker"
                            );

                            // Publish a failure TurnResult so submit_turn
                            // callers get an immediate error instead of
                            // waiting until timeout.
                            let err_result = bus_messages::TurnResult {
                                conversation_id: conv_id,
                                content: format!("Turn failed: {e}"),
                                turn: 0,
                                attachments: vec![],
                            };
                            let mut pub_req = PublishRequest::new(
                                topic::TURN_RESULT,
                                serde_json::to_value(&err_result).unwrap_or_default(),
                            )
                            .with_agent_id(self.agent_id.clone())
                            .with_conversation_id(conv_id)
                            .with_causation_id(msg.id);
                            if let Some(bid) = msg.batch_id {
                                pub_req = pub_req.with_batch_id(bid);
                            } else if msg.correlation_id.is_some() {
                                self.metrics.record_submit_result_unmatched(
                                    &self.agent_id,
                                    &interface_name,
                                    "missing_request_id",
                                );
                                warn!(
                                    conversation_id = %conv_id,
                                    msg_id = %msg.id,
                                    "turn.request missing batch_id; submit_turn correlation may fail"
                                );
                            }
                            if let Some(correlation_id) = msg.correlation_id.or(msg.batch_id) {
                                pub_req = pub_req.with_correlation_id(correlation_id);
                            }
                            let mut produce_result_span = crate::otel_spans::start_bus_span(
                                SpanKind::Producer,
                                topic::TURN_RESULT,
                                Some(conv_id),
                                &bus_consume_cx,
                            );
                            match self.bus.publish(pub_req).await {
                                Ok(published_id) => {
                                    produce_result_span.set_attribute(KeyValue::new(
                                        "bus.message_id",
                                        published_id.to_string(),
                                    ));
                                    produce_result_span
                                        .set_attribute(KeyValue::new("bus.status", "ok"));
                                }
                                Err(pub_err) => {
                                    produce_result_span
                                        .set_attribute(KeyValue::new("bus.status", "error"));
                                    produce_result_span.set_attribute(KeyValue::new("error", true));
                                    produce_result_span.set_attribute(KeyValue::new(
                                        "error.message",
                                        pub_err.to_string(),
                                    ));
                                }
                            }
                            produce_result_span.end();

                            let err_payload = serde_json::json!({
                                "conversation_id": conv_id,
                                "content": err_result.content,
                                "turn": err_result.turn,
                                "attachments": err_result.attachments,
                                "interface": interface_name,
                            });
                            if let Err(dispatch_err) = webhook_dispatch::dispatch_event(
                                self.storage.as_ref(),
                                &self.agent_id,
                                topic::TURN_RESULT,
                                err_payload,
                            )
                            .await
                            {
                                warn!(
                                    conversation_id = %conv_id,
                                    error = %dispatch_err,
                                    "Failed to dispatch failed turn.result webhooks"
                                );
                            }

                            let _ = self.bus.fail(msg.id).await;
                            bus_consume_cx
                                .span()
                                .set_attribute(KeyValue::new("bus.status", "error"));
                            bus_consume_cx
                                .span()
                                .set_attribute(KeyValue::new("error", true));
                            bus_consume_cx
                                .span()
                                .set_attribute(KeyValue::new("error.message", e.to_string()));
                            bus_consume_cx.span().end();
                        }
                    }
                }
                Ok(None) => {
                    // No pending messages — back off.
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    error!(error = %e, worker_id, "Turn worker claim error");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Submit a turn through the message bus and wait for the result.
    ///
    /// Publishes a [`TurnRequest`](bus_messages::TurnRequest) to the bus and
    /// polls for the corresponding [`TurnResult`](bus_messages::TurnResult).
    /// Requires [`run_worker`](Self::run_worker) to be running in a
    /// background task.
    ///
    /// # Parameters
    /// * `prompt` — the user message
    /// * `conversation_id` — conversation to continue (or start)
    /// * `interface` — originating interface
    /// * `timestamp` — when the message was received (injected as a prefix into
    ///   the prompt by the worker)
    pub async fn submit_turn(
        &self,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<TurnResult> {
        self.submit_turn_with_metadata(
            prompt,
            conversation_id,
            interface,
            timestamp,
            HashMap::new(),
        )
        .await
    }

    /// Submit a turn through the message bus and wait for the result, enriched
    /// with interface-provided metadata attributes.
    pub async fn submit_turn_with_metadata(
        &self,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
        timestamp: Option<DateTime<Utc>>,
        submit_metadata: HashMap<String, String>,
    ) -> Result<TurnResult> {
        let request_id = Uuid::new_v4();
        let interface_label = format!("{interface:?}");
        let interface_cx = crate::otel_spans::start_interface_root_context(
            &interface,
            "submit_turn",
            Some(conversation_id),
        );
        interface_cx
            .span()
            .set_attribute(KeyValue::new("submit.request_id", request_id.to_string()));
        interface_cx
            .span()
            .set_attribute(KeyValue::new("bus.correlation_id", request_id.to_string()));
        for (key, value) in &submit_metadata {
            interface_cx
                .span()
                .set_attribute(KeyValue::new(key.clone(), value.clone()));
        }
        info!(
            conversation_id = %conversation_id,
            request_id = %request_id,
            interface = %interface_label,
            submit_state = "pending",
            "submit_turn waiter state"
        );
        let turn_req = bus_messages::TurnRequest {
            prompt: prompt.to_string(),
            conversation_id,
            extension_tools: vec![],
            timestamp,
            traceparent: crate::otel_spans::traceparent_from_context(&interface_cx),
        };

        let mut produce_request_span = crate::otel_spans::start_bus_span(
            SpanKind::Producer,
            topic::TURN_REQUEST,
            Some(conversation_id),
            &interface_cx,
        );
        for (key, value) in &submit_metadata {
            produce_request_span.set_attribute(KeyValue::new(key.clone(), value.clone()));
        }
        let publish_result = self
            .bus
            .publish(
                PublishRequest::new(topic::TURN_REQUEST, serde_json::to_value(&turn_req)?)
                    .with_agent_id(self.agent_id.clone())
                    .with_conversation_id(conversation_id)
                    .with_interface(format!("{:?}", interface))
                    .with_reply_to(topic::TURN_RESULT)
                    .with_correlation_id(request_id)
                    .with_batch_id(request_id),
            )
            .await;
        match publish_result {
            Ok(message_id) => {
                produce_request_span
                    .set_attribute(KeyValue::new("bus.message_id", message_id.to_string()));
                produce_request_span
                    .set_attribute(KeyValue::new("bus.batch_id", request_id.to_string()));
                produce_request_span
                    .set_attribute(KeyValue::new("bus.correlation_id", request_id.to_string()));
                produce_request_span
                    .set_attribute(KeyValue::new("submit.request_id", request_id.to_string()));
                produce_request_span.set_attribute(KeyValue::new("bus.status", "ok"));
                produce_request_span.end();
            }
            Err(e) => {
                produce_request_span.set_attribute(KeyValue::new("bus.status", "error"));
                produce_request_span.set_attribute(KeyValue::new("error", true));
                produce_request_span.set_attribute(KeyValue::new("error.message", e.to_string()));
                produce_request_span.end();
                return Err(e);
            }
        }

        // Poll for the result with a 10-minute timeout.
        // Match by both conversation_id and batch_id (request_id) so
        // overlapping turns for the same conversation don't collide.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(600);
        let late_result_deadline = deadline + Duration::from_secs(2);
        let filter = ClaimFilter::new()
            .with_agent_id(self.agent_id.clone())
            .with_conversation_id(conversation_id)
            .with_batch_id(request_id);
        loop {
            let now = tokio::time::Instant::now();
            if now > late_result_deadline {
                self.metrics
                    .record_submit_timeout(&self.agent_id, &interface_label);
                info!(
                    conversation_id = %conversation_id,
                    request_id = %request_id,
                    interface = %interface_label,
                    submit_state = "timeout",
                    "submit_turn waiter state"
                );
                anyhow::bail!(
                    "submit_turn timed out waiting for result \
                     (conversation_id={conversation_id}, request_id={request_id})"
                );
            }

            if let Some(msg) = self
                .bus
                .claim_filtered(topic::TURN_RESULT, "submit_turn", &filter)
                .await?
            {
                if msg.batch_id != Some(request_id) {
                    self.metrics.record_submit_result_unmatched(
                        &self.agent_id,
                        &interface_label,
                        "batch_id_mismatch",
                    );
                    warn!(
                        conversation_id = %conversation_id,
                        request_id = %request_id,
                        msg_id = %msg.id,
                        msg_batch_id = ?msg.batch_id,
                        submit_state = "mismatch",
                        "submit_turn received mismatched turn.result batch_id"
                    );
                    self.bus.nack(msg.id).await?;
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }
                let mut consume_result_span = crate::otel_spans::start_bus_span(
                    SpanKind::Consumer,
                    topic::TURN_RESULT,
                    Some(conversation_id),
                    &interface_cx,
                );
                consume_result_span
                    .set_attribute(KeyValue::new("bus.message_id", msg.id.to_string()));
                consume_result_span
                    .set_attribute(KeyValue::new("bus.batch_id", request_id.to_string()));
                consume_result_span
                    .set_attribute(KeyValue::new("bus.correlation_id", request_id.to_string()));
                consume_result_span
                    .set_attribute(KeyValue::new("submit.request_id", request_id.to_string()));
                let bus_result: bus_messages::TurnResult = serde_json::from_value(msg.payload)?;
                if bus_result.conversation_id != conversation_id {
                    self.metrics.record_submit_result_unmatched(
                        &self.agent_id,
                        &interface_label,
                        "conversation_id_mismatch",
                    );
                    warn!(
                        conversation_id = %conversation_id,
                        request_id = %request_id,
                        msg_id = %msg.id,
                        result_conversation_id = %bus_result.conversation_id,
                        submit_state = "mismatch",
                        "submit_turn received mismatched turn.result conversation_id"
                    );
                    self.bus.nack(msg.id).await?;
                    consume_result_span.set_attribute(KeyValue::new("bus.status", "mismatch"));
                    consume_result_span.end();
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }
                self.bus.ack(msg.id).await?;
                consume_result_span.set_attribute(KeyValue::new("bus.status", "ok"));
                consume_result_span.end();
                if now > deadline {
                    self.metrics
                        .record_submit_late_result(&self.agent_id, &interface_label);
                    info!(
                        conversation_id = %conversation_id,
                        request_id = %request_id,
                        interface = %interface_label,
                        submit_state = "late_result",
                        "submit_turn waiter state"
                    );
                } else {
                    info!(
                        conversation_id = %conversation_id,
                        request_id = %request_id,
                        interface = %interface_label,
                        submit_state = "matched",
                        "submit_turn waiter state"
                    );
                }
                return Ok(TurnResult {
                    answer: bus_result.content,
                    attachments: bus_result.attachments,
                });
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}
