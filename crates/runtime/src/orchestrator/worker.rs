//! Bus-based turn-processing worker for the orchestrator.
//!
//! These methods handle claiming messages from the message bus, dispatching
//! them to the appropriate `run_turn*` variant, and publishing results back.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::{bus_messages, topic, ClaimFilter, Interface, PublishRequest, ToolHandler};
use assistant_llm::ContentBlock;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{parse_interface, ExtensionRegistration, Orchestrator, TurnResult};

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
        let filter = match interface {
            Some(iface) => ClaimFilter::new().with_interface(iface),
            None => ClaimFilter::default(),
        };
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

                    debug!(
                        conversation_id = %conv_id,
                        worker_id,
                        "Processing turn request"
                    );

                    // Check for registered side-channel resources.
                    let ext = self.extension_registrations.write().await.remove(&conv_id);
                    let token_sink = self.token_sinks.write().await.remove(&conv_id);

                    // Dispatch to the appropriate processing method.
                    let result: Result<TurnResult> = if let Some(reg) = ext {
                        // Extension-tool turn (Slack, Mattermost).
                        self.run_turn_with_tools(
                            &turn_req.prompt,
                            conv_id,
                            interface,
                            reg.tools,
                            None,
                            reg.attachments,
                        )
                        .await
                    } else if let Some(sink) = token_sink {
                        // Streaming turn (CLI, Signal).
                        self.run_turn_streaming(&turn_req.prompt, conv_id, interface, sink, None)
                            .await
                    } else {
                        // Standard non-streaming turn.
                        self.run_turn(&turn_req.prompt, conv_id, interface, None)
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
                            .with_conversation_id(conv_id);
                            if let Some(bid) = msg.batch_id {
                                pub_req = pub_req.with_batch_id(bid);
                            }

                            match self.bus.publish(pub_req).await {
                                Ok(_) => {
                                    if let Err(e) = self.bus.ack(msg.id).await {
                                        warn!(
                                            error = %e,
                                            msg_id = %msg.id,
                                            "Failed to ack bus message"
                                        );
                                    }
                                    info!(
                                        conversation_id = %conv_id,
                                        worker_id,
                                        "Turn completed via worker"
                                    );
                                }
                                Err(e) => {
                                    warn!(error = %e, "Failed to publish TurnResult, nacking request");
                                    let _ = self.bus.nack(msg.id).await;
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
                            .with_conversation_id(conv_id);
                            if let Some(bid) = msg.batch_id {
                                pub_req = pub_req.with_batch_id(bid);
                            }
                            let _ = self.bus.publish(pub_req).await;

                            let _ = self.bus.fail(msg.id).await;
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
    pub async fn submit_turn(
        &self,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
    ) -> Result<TurnResult> {
        let request_id = Uuid::new_v4();
        let turn_req = bus_messages::TurnRequest {
            prompt: prompt.to_string(),
            conversation_id,
            extension_tools: vec![],
        };

        self.bus
            .publish(
                PublishRequest::new(topic::TURN_REQUEST, serde_json::to_value(&turn_req)?)
                    .with_conversation_id(conversation_id)
                    .with_interface(format!("{:?}", interface))
                    .with_reply_to(topic::TURN_RESULT)
                    .with_batch_id(request_id),
            )
            .await?;

        // Poll for the result with a 10-minute timeout.
        // Match by both conversation_id and batch_id (request_id) so
        // overlapping turns for the same conversation don't collide.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(600);
        loop {
            if tokio::time::Instant::now() > deadline {
                anyhow::bail!(
                    "submit_turn timed out waiting for result \
                     (conversation_id={conversation_id}, request_id={request_id})"
                );
            }

            let filter = ClaimFilter::new()
                .with_conversation_id(conversation_id)
                .with_batch_id(request_id);
            if let Some(msg) = self
                .bus
                .claim_filtered(topic::TURN_RESULT, "submit_turn", &filter)
                .await?
            {
                let bus_result: bus_messages::TurnResult = serde_json::from_value(msg.payload)?;
                self.bus.ack(msg.id).await?;
                return Ok(TurnResult {
                    answer: bus_result.content,
                    attachments: bus_result.attachments,
                });
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}
