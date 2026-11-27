//! The surface hook: turns a `render-surface` result into a stored surface
//! and a [`OrchestratorEvent::SurfaceUpdate`].
//!
//! The handler only validates (design D2): it returns the validated surface
//! or patch in `ToolOutput.data`. The hook runs in `finalize_tool_result`
//! before anything else sees that data, so the surface never reaches the
//! `tool_result` frame or the persisted tool row. It assigns the id, anchors
//! the surface to its tool call, merges updates into the stored surface and
//! enforces the per-turn call limit.

use std::collections::HashSet;

use serde_json::{Value, json};
use tracing::warn;
use uuid::Uuid;

use assistant_a2ui::{
    CATALOG_ID, Component, SurfaceDraft, SurfaceMessages, SurfacePatch, UrlPolicy, validate,
};
use assistant_core::ToolOutput;
use assistant_core::identity::UserId;
use assistant_storage::{NewSurface, Surface, SurfaceChanges, SurfaceStatus, Transition};

use crate::interaction::{InteractionOutcome, InteractionRequest};

use super::{Orchestrator, OrchestratorEvent};

/// Name of the built-in tool this hook serves.
pub(crate) const RENDER_SURFACE_TOOL: &str = "render-surface";

/// Successful `render-surface` calls (creates and updates) allowed per turn.
const MAX_SURFACE_CALLS_PER_TURN: u32 = 5;

/// The interaction kind an in-turn wait registers (see `InteractionKinds`).
const SURFACE_ACTION_KIND: &str = "surface_action";

const PROTOCOL_VERSION: &str = "1.0";

/// Surface bookkeeping that lives as long as one turn.
pub(crate) struct SurfaceTurn {
    /// Index of the turn's user message: the turn a surface is created in.
    base_turn: i64,
    /// Successful calls so far.
    calls: u32,
}

impl SurfaceTurn {
    pub(crate) fn new(base_turn: i64) -> Self {
        Self {
            base_turn,
            calls: 0,
        }
    }
}

/// Where one `render-surface` call sits: the turn state plus the tool-call
/// message and index the call came from.
pub(crate) struct SurfaceCall<'a> {
    pub(crate) turn: &'a mut SurfaceTurn,
    pub(crate) anchor_message_id: Uuid,
    pub(crate) call_index: usize,
    /// The run and its owner, who an in-turn wait is addressed to.
    pub(crate) run_id: Option<Uuid>,
    pub(crate) user_id: Option<UserId>,
}

/// A surface change that was persisted.
struct Applied {
    surface: Surface,
    messages: Vec<Value>,
    observation: String,
    /// The Button action names, when the model asked the turn to wait.
    wait_for: Option<Vec<String>>,
}

impl Orchestrator {
    /// Apply a `render-surface` result. Takes `output.data` so it cannot be
    /// forwarded or persisted as generic tool data, and replaces the output
    /// with an error when the surface cannot be applied.
    pub(crate) async fn surface_hook(
        &self,
        mut output: ToolOutput,
        call: Option<SurfaceCall<'_>>,
        conversation_id: Uuid,
        event_sink: Option<&tokio::sync::mpsc::Sender<OrchestratorEvent>>,
        tool_call_id: Option<&str>,
    ) -> ToolOutput {
        let Some(data) = output.data.take() else {
            // Validation failure, or the "cannot display UI" fallback.
            return output;
        };
        if !output.success {
            return output;
        }
        let Some(call) = call else {
            return ToolOutput::error("render-surface: this turn cannot display UI.");
        };
        if call.turn.calls >= MAX_SURFACE_CALLS_PER_TURN {
            return ToolOutput::error(format!(
                "render-surface: at most {MAX_SURFACE_CALLS_PER_TURN} surface calls are allowed \
                 per turn. Describe the rest in text."
            ));
        }
        let applied = if data.get("surface").is_some() {
            self.create_surface(&data, &call, conversation_id, tool_call_id)
                .await
        } else {
            self.update_surface(&data, call.turn.base_turn, conversation_id)
                .await
        };
        let applied = match applied {
            Ok(applied) => applied,
            Err(message) => return ToolOutput::error(format!("render-surface: {message}")),
        };
        call.turn.calls += 1;

        if let Some(sink) = event_sink {
            let s = &applied.surface;
            let _ = sink
                .send(OrchestratorEvent::SurfaceUpdate {
                    surface_id: s.id.to_string(),
                    tool_call_id: s.tool_call_id.clone(),
                    anchor_message_id: s.anchor_message_id.to_string(),
                    anchor_call_index: u32::try_from(s.anchor_call_index).unwrap_or_default(),
                    title: s.title.clone(),
                    fallback_text: s.fallback_text.clone(),
                    status: "active".to_string(),
                    messages: applied.messages,
                })
                .await;
        }
        match applied.wait_for {
            Some(actions) => {
                let observation = self
                    .wait_for_action(&applied.surface, actions, &call, event_sink, tool_call_id)
                    .await;
                ToolOutput::success(observation)
            }
            None => ToolOutput::success(applied.observation),
        }
    }

    /// Hold the turn on a `surface_action` interaction for `surface` and say
    /// what happened. The surface was announced first, so the interaction
    /// always follows its `surface_update`. Only a resolved action closes the
    /// surface; every other ending leaves it `active`.
    async fn wait_for_action(
        &self,
        surface: &Surface,
        actions: Vec<String>,
        call: &SurfaceCall<'_>,
        event_sink: Option<&tokio::sync::mpsc::Sender<OrchestratorEvent>>,
        tool_call_id: Option<&str>,
    ) -> String {
        let (Some(broker), Some(run_id), Some(user)) =
            (&self.interaction_broker, call.run_id, call.user_id.as_ref())
        else {
            return format!(
                "Surface shown to the user (surface_id: {}). This turn cannot wait for an \
                 answer, so continue without it.",
                surface.id
            );
        };
        let request = json!({
            "surface_id": surface.id,
            "tool_call_id": tool_call_id,
            "title": surface.title,
            "actions": actions,
            "send_data_model": surface.send_data_model,
        });
        let asked = broker
            .request(
                InteractionRequest {
                    conversation_id: surface.conversation_id.to_string(),
                    run_id: run_id.to_string(),
                    user_id: user.0.clone(),
                    kind: SURFACE_ACTION_KIND.to_string(),
                    tool_call_id: tool_call_id.map(str::to_string),
                    request: request.clone(),
                    turn_deadline: None,
                },
                event_sink,
            )
            .await;
        let outcome = match asked {
            Ok((_, outcome)) => outcome,
            Err(e) => {
                warn!(error = %e, "Failed to wait for a surface action");
                return "The surface is shown, but waiting for the user's answer failed. \
                        Continue without it."
                    .into();
            }
        };
        let observation = broker
            .kinds()
            .get(SURFACE_ACTION_KIND)
            .map(|kind| kind.observation(&request, &outcome))
            .unwrap_or_default();
        let InteractionOutcome::Resolved { response, .. } = &outcome else {
            return observation;
        };
        if response["declined"] == true {
            return observation;
        }
        self.submit_surface(surface, response, event_sink, observation)
            .await
    }

    /// Mark `surface` submitted with the answer's action and announce it.
    /// Returns `observation`, or says the surface was submitted another way
    /// when this answer lost the race.
    async fn submit_surface(
        &self,
        surface: &Surface,
        response: &Value,
        event_sink: Option<&tokio::sync::mpsc::Sender<OrchestratorEvent>>,
        observation: String,
    ) -> String {
        // An in-turn answer is carried by no user message; the id only has to
        // be unique so `action_message_map` never confuses two surfaces.
        let marked = self
            .surface_store
            .mark_submitted(surface.id, &response["action"], Uuid::new_v4())
            .await;
        match marked {
            Ok(Transition::Applied) => {}
            Ok(Transition::Conflict) => {
                return "This surface was already submitted in a new message, so this answer \
                        was not used."
                    .into();
            }
            Err(e) => {
                warn!(error = %e, "Failed to mark a surface submitted");
                return observation;
            }
        }
        if let Some(sink) = event_sink {
            let _ = sink
                .send(OrchestratorEvent::SurfaceUpdate {
                    surface_id: surface.id.to_string(),
                    tool_call_id: surface.tool_call_id.clone(),
                    anchor_message_id: surface.anchor_message_id.to_string(),
                    anchor_call_index: u32::try_from(surface.anchor_call_index).unwrap_or_default(),
                    title: surface.title.clone(),
                    fallback_text: surface.fallback_text.clone(),
                    status: "submitted".to_string(),
                    messages: Vec::new(),
                })
                .await;
        }
        observation
    }

    async fn create_surface(
        &self,
        data: &Value,
        call: &SurfaceCall<'_>,
        conversation_id: Uuid,
        tool_call_id: Option<&str>,
    ) -> Result<Applied, String> {
        let draft: SurfaceDraft = serde_json::from_value(data["surface"].clone())
            .map_err(|e| format!("the validated surface is unreadable: {e}"))?;
        let id = Uuid::new_v4();
        let new = NewSurface {
            id,
            conversation_id,
            turn: call.turn.base_turn,
            anchor_message_id: call.anchor_message_id,
            anchor_call_index: i64::try_from(call.call_index).unwrap_or_default(),
            tool_call_id: tool_call_id.map(str::to_string),
            title: draft.title.clone(),
            fallback_text: draft.fallback_text.clone(),
            catalog_id: CATALOG_ID.to_string(),
            protocol_version: PROTOCOL_VERSION.to_string(),
            components: serde_json::to_value(&draft.components).map_err(encode_error)?,
            data_model: draft.data_model.clone().unwrap_or_else(|| json!({})),
            send_data_model: draft.send_data_model,
        };
        let surface = self.surface_store.create(&new).await.map_err(|e| {
            warn!(error = %e, "Failed to persist surface");
            "the surface could not be saved".to_string()
        })?;
        let messages = vec![
            serde_json::to_value(SurfaceMessages::create(&draft, &id.to_string()))
                .map_err(encode_error)?,
        ];
        let wait_for = (data.get("wait_for_action") == Some(&Value::Bool(true)))
            .then(|| draft.declared_action_names().into_iter().collect());
        Ok(Applied {
            observation: format!(
                "Surface shown to the user (surface_id: {id}). To change it later in this turn, \
                 call render-surface with this surface_id."
            ),
            surface,
            messages,
            wait_for,
        })
    }

    async fn update_surface(
        &self,
        data: &Value,
        base_turn: i64,
        conversation_id: Uuid,
    ) -> Result<Applied, String> {
        let id = data
            .get("surface_id")
            .and_then(Value::as_str)
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or("`surface_id` is not a valid surface id")?;
        let patch: SurfacePatch = serde_json::from_value(data["patch"].clone())
            .map_err(|e| format!("the validated patch is unreadable: {e}"))?;

        let stored = self.surface_store.get(id).await.map_err(|e| {
            warn!(error = %e, "Failed to load surface");
            "the surface could not be loaded".to_string()
        })?;
        // A foreign surface looks the same as a missing one.
        let stored = stored
            .filter(|s| s.conversation_id == conversation_id)
            .ok_or("no surface with that id exists in this conversation")?;
        if stored.status != SurfaceStatus::Active {
            return Err("that surface was already submitted by the user and cannot change".into());
        }
        if stored.turn != base_turn {
            return Err(
                "that surface is no longer open because it was created in an earlier \
                        turn. Create a new surface with render-surface instead."
                    .into(),
            );
        }

        let current = SurfaceDraft {
            title: stored.title.clone(),
            fallback_text: stored.fallback_text.clone(),
            components: serde_json::from_value::<Vec<Component>>(stored.components.clone())
                .map_err(|e| format!("the stored surface is unreadable: {e}"))?,
            data_model: Some(stored.data_model.clone()),
            send_data_model: stored.send_data_model,
        };
        let policy = merge_policy(data, &current);
        let merged = validate(&current, &policy)
            .map_err(|_| "the stored surface no longer validates".to_string())?
            .merge(&patch, &policy)
            .map_err(|issues| serde_json::to_string(&issues).unwrap_or_default())?;
        let merged = merged.draft();

        let changes = SurfaceChanges {
            title: Some(merged.title.clone()),
            fallback_text: Some(merged.fallback_text.clone()),
            components: Some(serde_json::to_value(&merged.components).map_err(encode_error)?),
            data_model: Some(merged.data_model.clone().unwrap_or_else(|| json!({}))),
        };
        let surface = self
            .surface_store
            .update(id, &changes)
            .await
            .map_err(|e| {
                warn!(error = %e, "Failed to persist surface update");
                "the surface could not be saved".to_string()
            })?
            .ok_or("the surface disappeared while it was being updated")?;
        let messages = SurfaceMessages::update(&id.to_string(), &patch)
            .iter()
            .map(|m| serde_json::to_value(m).map_err(encode_error))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Applied {
            observation: format!("Surface {id} updated."),
            surface,
            messages,
            wait_for: None,
        })
    }
}

/// The policy for validating the merged surface: the attachments the handler
/// found owned for the patch plus those the stored surface already uses.
fn merge_policy(data: &Value, current: &SurfaceDraft) -> UrlPolicy {
    let policy = data.get("url_policy");
    let mut allowed: HashSet<Uuid> = current.attachment_ids();
    allowed.extend(
        policy
            .and_then(|p| p.get("allowed_attachment_ids"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|v| Uuid::parse_str(v.as_str()?).ok()),
    );
    let image_hosts = policy
        .and_then(|p| p.get("image_hosts"))
        .and_then(|h| serde_json::from_value::<Vec<String>>(h.clone()).ok());
    UrlPolicy {
        allowed_attachment_ids: allowed,
        image_hosts,
    }
}

fn encode_error(e: serde_json::Error) -> String {
    format!("cannot encode the surface: {e}")
}
