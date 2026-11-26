//! Shared event-projection layer.
//!
//! Converts an [`OrchestratorEvent`] into a protocol's wire frames. This is the
//! single, total, tested place where one orchestrator event becomes one
//! protocol's frames — adapters consume a projector instead of re-implementing
//! an inline `match` over [`OrchestratorEvent`]. Transport concerns
//! (persistence, sequencing, batching, framing, push notifications) stay in the
//! adapter.
//!
//! Phase 0 of the `protocol-adapter-platform` epic ships one projector per
//! existing wire: [`SseProjector`] (`/api` SSE) and `CliProjector` (the REPL).

use serde_json::{Value, json};

use crate::orchestrator::OrchestratorEvent;

/// Projects an [`OrchestratorEvent`] into a wire-specific frame type.
///
/// Implementations MUST be total over every [`OrchestratorEvent`] variant and
/// MUST NOT use a catch-all `_` arm, so that adding a variant fails to compile
/// until it is explicitly projected.
pub trait StreamProjector {
    /// The wire frame this projector emits.
    type Frame;

    /// Project a single event into zero or more wire frames.
    fn project(&self, event: &OrchestratorEvent) -> Vec<Self::Frame>;
}

/// A single frame for the `/api` SSE wire.
///
/// `sse_data` is the exact bytes for the SSE `data:` field; `log_payload` is the
/// canonical JSON used for durable persistence and live broadcast. These differ
/// for `token` and `agent_error`, whose SSE form is raw text while their
/// persisted form is a JSON object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseFrame {
    /// SSE `event:` name; also the persisted `event_type`.
    pub event: String,
    /// Canonical JSON payload for persistence + live broadcast.
    pub log_payload: Value,
    /// Exact bytes for the SSE `data:` field.
    pub sse_data: String,
}

/// Projects orchestrator events onto the `/api` Server-Sent-Events wire.
#[derive(Debug, Default, Clone, Copy)]
pub struct SseProjector;

impl StreamProjector for SseProjector {
    type Frame = SseFrame;

    fn project(&self, event: &OrchestratorEvent) -> Vec<SseFrame> {
        // NB: no catch-all `_` arm — adding an `OrchestratorEvent` variant must
        // fail to compile here until it is explicitly projected.
        let frame = match event {
            OrchestratorEvent::Token(token) => SseFrame {
                event: "token".to_string(),
                log_payload: json!({ "token": token }),
                // SSE carries the raw token text, not the JSON wrapper.
                sse_data: token.clone(),
            },
            OrchestratorEvent::Status {
                message,
                tool_call_id,
            } => {
                let mut p = json!({ "message": message });
                if let Some(id) = tool_call_id {
                    p["tool_call_id"] = Value::String(id.clone());
                }
                SseFrame {
                    event: "status".to_string(),
                    sse_data: p.to_string(),
                    log_payload: p,
                }
            }
            OrchestratorEvent::ToolResult {
                tool_name,
                status,
                arguments,
                result,
                tool_call_id,
                data,
            } => {
                let mut p = json!({ "tool_name": tool_name, "status": status });
                if let Some(args) = arguments {
                    p["arguments"] = args.clone();
                }
                if let Some(res) = result {
                    p["result"] = Value::String(res.clone());
                }
                if let Some(id) = tool_call_id {
                    p["tool_call_id"] = Value::String(id.clone());
                }
                if let Some(data) = data {
                    p["data"] = data.clone();
                }
                SseFrame {
                    event: "tool_result".to_string(),
                    sse_data: p.to_string(),
                    log_payload: p,
                }
            }
            OrchestratorEvent::SkillComplete {
                skill_name,
                success,
                summary,
            } => {
                let p = json!({
                    "skill_name": skill_name,
                    "success": success,
                    "summary": summary,
                });
                SseFrame {
                    event: "skill_complete".to_string(),
                    sse_data: p.to_string(),
                    log_payload: p,
                }
            }
            OrchestratorEvent::AgentError { message } => SseFrame {
                event: "agent_error".to_string(),
                log_payload: json!({ "message": message }),
                // SSE carries the raw error text, not the JSON wrapper.
                sse_data: message.clone(),
            },
            OrchestratorEvent::Thinking(content) => {
                let p = json!({ "content": content });
                SseFrame {
                    event: "thinking".to_string(),
                    sse_data: p.to_string(),
                    log_payload: p,
                }
            }
            OrchestratorEvent::SubagentStarted { agent_id, task } => {
                let p = json!({ "agent_id": agent_id, "task": task });
                SseFrame {
                    event: "subagent_started".to_string(),
                    sse_data: p.to_string(),
                    log_payload: p,
                }
            }
            OrchestratorEvent::SubagentCompleted {
                agent_id,
                status,
                summary,
            } => {
                let p = json!({
                    "agent_id": agent_id,
                    "status": status,
                    "summary": summary,
                });
                SseFrame {
                    event: "subagent_completed".to_string(),
                    sse_data: p.to_string(),
                    log_payload: p,
                }
            }
            OrchestratorEvent::SubagentEvent { agent_id, inner } => {
                let (event_name, inner_data) = project_subagent_inner(inner);
                let p = json!({ "agent_id": agent_id, "data": inner_data });
                SseFrame {
                    event: event_name.to_string(),
                    sse_data: p.to_string(),
                    log_payload: p,
                }
            }
            OrchestratorEvent::AudioReady { audio_id } => {
                let p = json!({ "audio_id": audio_id, "auto_play": true });
                SseFrame {
                    event: "audio_ready".to_string(),
                    sse_data: p.to_string(),
                    log_payload: p,
                }
            }
            OrchestratorEvent::SurfaceUpdate {
                surface_id,
                tool_call_id,
                anchor_message_id,
                anchor_call_index,
                title,
                fallback_text,
                status,
                messages,
            } => {
                let mut p = json!({
                    "surface_id": surface_id,
                    "anchor_message_id": anchor_message_id,
                    "anchor_call_index": anchor_call_index,
                    "title": title,
                    "fallback_text": fallback_text,
                    "status": status,
                    "messages": messages,
                });
                if let Some(id) = tool_call_id {
                    p["tool_call_id"] = Value::String(id.clone());
                }
                SseFrame {
                    event: "surface_update".to_string(),
                    sse_data: p.to_string(),
                    log_payload: p,
                }
            }
        };
        vec![frame]
    }
}

/// Maps a subagent's inner event to its `(event_name, data)` pair, mirroring the
/// historical `/api` wire. The inner match intentionally keeps a bound
/// catch-all (`other`) so unknown inner events degrade to a debug-formatted
/// `subagent_event`; top-level totality is still enforced by the outer `match`.
fn project_subagent_inner(inner: &OrchestratorEvent) -> (&'static str, Value) {
    match inner {
        OrchestratorEvent::Token(t) => ("subagent_token", json!({ "content": t })),
        OrchestratorEvent::Thinking(t) => ("subagent_thinking", json!({ "content": t })),
        OrchestratorEvent::Status {
            message,
            tool_call_id,
        } => {
            let mut d = json!({ "message": message });
            if let Some(id) = tool_call_id {
                d["tool_call_id"] = Value::String(id.clone());
            }
            ("subagent_status", d)
        }
        OrchestratorEvent::ToolResult {
            tool_name,
            status,
            arguments,
            result,
            tool_call_id,
            // Subagent tool calls stay compact rows: `data` is deliberately
            // not forwarded, so it is not stored twice in the event log.
            data: _,
        } => {
            let mut d = json!({
                "tool_name": tool_name,
                "status": status,
                "arguments": arguments,
                "result": result,
            });
            if let Some(id) = tool_call_id {
                d["tool_call_id"] = Value::String(id.clone());
            }
            ("subagent_tool_result", d)
        }
        other => ("subagent_event", json!({ "type": format!("{other:?}") })),
    }
}

/// Projects orchestrator events onto the CLI/REPL wire (plain stdout text).
///
/// The REPL renders only streamed assistant tokens; all other events are
/// intentionally not surfaced and project to no frames. Totality is still
/// compiler-enforced (no `_` arm), so a new variant forces an explicit decision
/// about whether the CLI should render it.
#[derive(Debug, Default, Clone, Copy)]
pub struct CliProjector;

impl StreamProjector for CliProjector {
    type Frame = String;

    fn project(&self, event: &OrchestratorEvent) -> Vec<String> {
        match event {
            OrchestratorEvent::Token(token) => vec![token.clone()],
            OrchestratorEvent::Status { .. }
            | OrchestratorEvent::ToolResult { .. }
            | OrchestratorEvent::SkillComplete { .. }
            | OrchestratorEvent::AgentError { .. }
            | OrchestratorEvent::Thinking(_)
            | OrchestratorEvent::SubagentStarted { .. }
            | OrchestratorEvent::SubagentCompleted { .. }
            | OrchestratorEvent::SubagentEvent { .. }
            | OrchestratorEvent::AudioReady { .. } => Vec::new(),
            // Totality only: surfaces are gated to interactive web turns, so
            // this arm is unreachable in practice.
            OrchestratorEvent::SurfaceUpdate { fallback_text, .. } => {
                vec![fallback_text.clone()]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One sample of every `OrchestratorEvent` variant, including a nested
    /// `SubagentEvent`, so the conformance test exercises the full surface.
    fn all_variants() -> Vec<OrchestratorEvent> {
        vec![
            OrchestratorEvent::Token("hi".into()),
            OrchestratorEvent::Status {
                message: "Calling tool".into(),
                tool_call_id: Some("call_1".into()),
            },
            OrchestratorEvent::ToolResult {
                tool_name: "file-read".into(),
                status: "ok".into(),
                arguments: Some(json!({"path": "/tmp/x"})),
                result: Some("contents".into()),
                tool_call_id: Some("call_2".into()),
                data: None,
            },
            OrchestratorEvent::SkillComplete {
                skill_name: "tdd".into(),
                success: true,
                summary: "done".into(),
            },
            OrchestratorEvent::AgentError {
                message: "boom".into(),
            },
            OrchestratorEvent::Thinking("pondering".into()),
            OrchestratorEvent::SubagentStarted {
                agent_id: "a1".into(),
                task: "research".into(),
            },
            OrchestratorEvent::SubagentCompleted {
                agent_id: "a1".into(),
                status: "ok".into(),
                summary: "found it".into(),
            },
            OrchestratorEvent::SubagentEvent {
                agent_id: "a1".into(),
                inner: Box::new(OrchestratorEvent::Token("inner".into())),
            },
            OrchestratorEvent::AudioReady {
                audio_id: "audio-1".into(),
            },
            surface_update_sample(),
        ]
    }

    fn surface_update_sample() -> OrchestratorEvent {
        OrchestratorEvent::SurfaceUpdate {
            surface_id: "s-1".into(),
            tool_call_id: Some("call_9".into()),
            anchor_message_id: "m-1".into(),
            anchor_call_index: 2,
            title: "Trip form".into(),
            fallback_text: "Trip form: choose a date".into(),
            status: "active".into(),
            messages: vec![json!({"version": "v1.0", "createSurface": {"surfaceId": "s-1"}})],
        }
    }

    /// Frames of a representative turn: a token, a top-level tool result and a
    /// subagent's tool result. Pinned by `testdata/sse_frames.golden.json` so
    /// that the wire shape only changes on purpose.
    fn golden_events() -> Vec<OrchestratorEvent> {
        vec![
            OrchestratorEvent::Token("Running it".into()),
            OrchestratorEvent::ToolResult {
                tool_name: "bash".into(),
                status: "ok".into(),
                arguments: Some(json!({"command": "ls"})),
                result: Some("a.txt".into()),
                tool_call_id: Some("call_1".into()),
                data: None,
            },
            OrchestratorEvent::SubagentEvent {
                agent_id: "a1".into(),
                inner: Box::new(OrchestratorEvent::ToolResult {
                    tool_name: "web-fetch".into(),
                    status: "ok".into(),
                    arguments: Some(json!({"url": "https://example.com"})),
                    result: Some("<html>".into()),
                    tool_call_id: Some("call_2".into()),
                    data: None,
                }),
            },
        ]
    }

    fn project_to_golden_shape(events: &[OrchestratorEvent]) -> Value {
        let frames: Vec<Value> = events
            .iter()
            .flat_map(|e| SseProjector.project(e))
            .map(|f| json!({ "event": f.event, "payload": f.log_payload }))
            .collect();
        Value::Array(frames)
    }

    fn golden() -> Value {
        serde_json::from_str(include_str!("testdata/sse_frames.golden.json"))
            .expect("golden file is valid JSON")
    }

    #[test]
    fn frames_without_data_match_the_golden_file() {
        assert_eq!(project_to_golden_shape(&golden_events()), golden());
    }

    #[test]
    fn tool_result_with_data_projects_a_data_field() {
        let frame = only(OrchestratorEvent::ToolResult {
            tool_name: "bash".into(),
            status: "ok".into(),
            arguments: None,
            result: None,
            tool_call_id: None,
            data: Some(json!({"kind": "terminal"})),
        });
        assert_eq!(frame.event, "tool_result");
        assert_eq!(frame.log_payload["data"], json!({"kind": "terminal"}));
        assert_eq!(frame.sse_data, frame.log_payload.to_string());
    }

    #[test]
    fn tool_result_without_data_has_no_data_key() {
        let frame = only(OrchestratorEvent::ToolResult {
            tool_name: "bash".into(),
            status: "ok".into(),
            arguments: None,
            result: None,
            tool_call_id: None,
            data: None,
        });
        assert!(frame.log_payload.get("data").is_none());
    }

    #[test]
    fn subagent_tool_result_never_carries_data() {
        let events: Vec<OrchestratorEvent> = golden_events()
            .into_iter()
            .map(|event| match event {
                OrchestratorEvent::SubagentEvent { agent_id, inner } => {
                    let inner = match *inner {
                        OrchestratorEvent::ToolResult {
                            tool_name,
                            status,
                            arguments,
                            result,
                            tool_call_id,
                            ..
                        } => OrchestratorEvent::ToolResult {
                            tool_name,
                            status,
                            arguments,
                            result,
                            tool_call_id,
                            data: Some(json!({"kind": "terminal"})),
                        },
                        other => other,
                    };
                    OrchestratorEvent::SubagentEvent {
                        agent_id,
                        inner: Box::new(inner),
                    }
                }
                other => other,
            })
            .collect();
        assert_eq!(
            project_to_golden_shape(&events),
            golden(),
            "subagent frames must match the golden file even when the inner result has data"
        );
    }

    #[test]
    fn sse_projection_is_total() {
        let projector = SseProjector;
        for event in all_variants() {
            let frames = projector.project(&event);
            assert!(
                !frames.is_empty(),
                "SseProjector produced no frame for {event:?}"
            );
        }
    }

    fn only(event: OrchestratorEvent) -> SseFrame {
        let frames = SseProjector.project(&event);
        assert_eq!(frames.len(), 1, "expected exactly one frame for {event:?}");
        frames.into_iter().next().expect("one frame")
    }

    #[test]
    fn token_and_agent_error_use_raw_sse_data() {
        let token = only(OrchestratorEvent::Token("hi".into()));
        assert_eq!(token.event, "token");
        assert_eq!(token.sse_data, "hi", "token SSE data must be raw text");
        assert_eq!(token.log_payload, json!({ "token": "hi" }));

        let err = only(OrchestratorEvent::AgentError {
            message: "boom".into(),
        });
        assert_eq!(err.event, "agent_error");
        assert_eq!(
            err.sse_data, "boom",
            "agent_error SSE data must be raw text"
        );
        assert_eq!(err.log_payload, json!({ "message": "boom" }));
    }

    #[test]
    fn non_raw_events_sse_data_is_serialized_payload() {
        for event in all_variants() {
            let frame = only(event.clone());
            if frame.event == "token" || frame.event == "agent_error" {
                continue; // raw-text exceptions asserted above
            }
            assert_eq!(
                frame.sse_data,
                frame.log_payload.to_string(),
                "SSE data for {} must equal serialized log payload",
                frame.event
            );
        }
    }

    #[test]
    fn event_names_match_the_wire() {
        let cases = [
            (OrchestratorEvent::Token("x".into()), "token"),
            (
                OrchestratorEvent::Status {
                    message: "m".into(),
                    tool_call_id: None,
                },
                "status",
            ),
            (
                OrchestratorEvent::ToolResult {
                    tool_name: "t".into(),
                    status: "ok".into(),
                    arguments: None,
                    result: None,
                    tool_call_id: None,
                    data: None,
                },
                "tool_result",
            ),
            (
                OrchestratorEvent::SkillComplete {
                    skill_name: "s".into(),
                    success: true,
                    summary: "y".into(),
                },
                "skill_complete",
            ),
            (
                OrchestratorEvent::AgentError {
                    message: "e".into(),
                },
                "agent_error",
            ),
            (OrchestratorEvent::Thinking("t".into()), "thinking"),
            (
                OrchestratorEvent::SubagentStarted {
                    agent_id: "a".into(),
                    task: "t".into(),
                },
                "subagent_started",
            ),
            (
                OrchestratorEvent::SubagentCompleted {
                    agent_id: "a".into(),
                    status: "ok".into(),
                    summary: "s".into(),
                },
                "subagent_completed",
            ),
            (
                OrchestratorEvent::AudioReady {
                    audio_id: "id".into(),
                },
                "audio_ready",
            ),
            (surface_update_sample(), "surface_update"),
        ];
        for (event, expected) in cases {
            assert_eq!(only(event).event, expected);
        }
    }

    #[test]
    fn subagent_inner_token_maps_to_subagent_token() {
        let frame = only(OrchestratorEvent::SubagentEvent {
            agent_id: "a1".into(),
            inner: Box::new(OrchestratorEvent::Token("inner".into())),
        });
        assert_eq!(frame.event, "subagent_token");
        assert_eq!(
            frame.log_payload,
            json!({ "agent_id": "a1", "data": { "content": "inner" } })
        );
    }

    #[test]
    fn cli_projects_token_text() {
        let frames = CliProjector.project(&OrchestratorEvent::Token("hello".into()));
        assert_eq!(frames, vec!["hello".to_string()]);
    }

    #[test]
    fn cli_ignores_non_token_events() {
        // The REPL surfaces only tokens; everything else projects to nothing.
        let frames = CliProjector.project(&OrchestratorEvent::Status {
            message: "Calling tool".into(),
            tool_call_id: None,
        });
        assert!(frames.is_empty());
    }

    #[test]
    fn subagent_inner_unknown_event_degrades_to_subagent_event() {
        let frame = only(OrchestratorEvent::SubagentEvent {
            agent_id: "a1".into(),
            inner: Box::new(OrchestratorEvent::AudioReady {
                audio_id: "id".into(),
            }),
        });
        assert_eq!(frame.event, "subagent_event");
        assert_eq!(frame.log_payload["agent_id"], json!("a1"));
        assert!(
            frame.log_payload["data"]["type"]
                .as_str()
                .is_some_and(|s| s.contains("AudioReady")),
            "unknown inner event should be debug-formatted, got {:?}",
            frame.log_payload
        );
    }

    #[test]
    fn surface_update_projects_exact_snake_case_payload() {
        let frame = only(surface_update_sample());
        assert_eq!(frame.event, "surface_update");
        assert_eq!(
            frame.log_payload,
            json!({
                "surface_id": "s-1",
                "tool_call_id": "call_9",
                "anchor_message_id": "m-1",
                "anchor_call_index": 2,
                "title": "Trip form",
                "fallback_text": "Trip form: choose a date",
                "status": "active",
                "messages": [{"version": "v1.0", "createSurface": {"surfaceId": "s-1"}}],
            })
        );
        assert_eq!(frame.sse_data, frame.log_payload.to_string());
    }

    #[test]
    fn surface_update_omits_absent_tool_call_id_and_keeps_empty_messages() {
        let frame = only(OrchestratorEvent::SurfaceUpdate {
            surface_id: "s-1".into(),
            tool_call_id: None,
            anchor_message_id: "m-1".into(),
            anchor_call_index: 0,
            title: "t".into(),
            fallback_text: "f".into(),
            status: "submitted".into(),
            messages: Vec::new(),
        });
        assert!(
            frame.log_payload.get("tool_call_id").is_none(),
            "absent tool_call_id must be omitted"
        );
        assert_eq!(frame.log_payload["messages"], json!([]));
        assert_eq!(frame.log_payload["status"], "submitted");
    }

    #[test]
    fn cli_projector_prints_surface_fallback_text() {
        assert_eq!(
            CliProjector.project(&surface_update_sample()),
            vec!["Trip form: choose a date".to_string()]
        );
    }
}
