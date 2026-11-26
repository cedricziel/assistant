//! `plan-update` tool — publish the Persona's working plan.
//!
//! Each call carries the whole plan; the tool keeps no state, so the latest
//! call in the timeline is the current plan. The plan travels as `plan` render
//! data (`schemas/tool-results/plan.schema.json`). The tool has no side
//! effects.

use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use assistant_core::tool_render::{PlanData, PlanStep, PlanStepStatus, ToolRenderData};
use assistant_core::types::conversation::ExecutionContext;
use assistant_core::{ToolHandler, ToolOutput};

/// Most steps a plan may have.
const MAX_STEPS: usize = 50;
/// Longest step title, in characters.
const MAX_STEP_TITLE_CHARS: usize = 200;
/// Longest plan title, in characters.
const MAX_TITLE_CHARS: usize = 120;

const STATUSES: &[&str] = &["pending", "in_progress", "completed", "failed", "skipped"];

pub struct PlanUpdateHandler;

impl PlanUpdateHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlanUpdateHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for PlanUpdateHandler {
    fn name(&self) -> &str {
        "plan-update"
    }

    fn description(&self) -> &str {
        "Publish your working plan so the user can follow your progress. Each call replaces \
         the whole plan: send every step with its current status, not only the changes."
    }

    fn params_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "maxLength": MAX_TITLE_CHARS,
                    "description": "Optional plan title."
                },
                "steps": {
                    "type": "array",
                    "maxItems": MAX_STEPS,
                    "description": "The complete, ordered list of steps.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "title": { "type": "string", "maxLength": MAX_STEP_TITLE_CHARS },
                            "status": { "type": "string", "enum": STATUSES }
                        },
                        "required": ["title", "status"]
                    }
                }
            },
            "required": ["steps"]
        })
    }

    fn output_schema(&self) -> Option<Value> {
        ToolRenderData::schema("plan")
    }

    async fn run(
        &self,
        params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        match parse_plan(&params) {
            Ok(plan) => {
                let done = plan
                    .steps
                    .iter()
                    .filter(|s| s.status == PlanStepStatus::Completed)
                    .count();
                let text = format!("Plan updated: {done}/{} steps completed", plan.steps.len());
                Ok(ToolOutput::success(text).with_render(ToolRenderData::Plan(plan)))
            }
            Err(message) => Ok(ToolOutput::error(format!("plan-update: {message}"))),
        }
    }
}

/// Validate `params` against the plan budgets. The error names the field.
fn parse_plan(params: &HashMap<String, Value>) -> Result<PlanData, String> {
    let title = match params.get("title") {
        None | Some(Value::Null) => None,
        Some(Value::String(t)) if t.chars().count() <= MAX_TITLE_CHARS => Some(t.clone()),
        Some(Value::String(_)) => {
            return Err(format!(
                "`title` must be at most {MAX_TITLE_CHARS} characters"
            ));
        }
        Some(_) => return Err("`title` must be a string".into()),
    };

    let raw_steps = params
        .get("steps")
        .and_then(Value::as_array)
        .ok_or("`steps` is required and must be an array")?;
    if raw_steps.len() > MAX_STEPS {
        return Err(format!("`steps` must have at most {MAX_STEPS} entries"));
    }

    let steps = raw_steps
        .iter()
        .enumerate()
        .map(|(i, step)| parse_step(i, step))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PlanData { title, steps })
}

fn parse_step(index: usize, step: &Value) -> Result<PlanStep, String> {
    let title = step
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`steps[{index}].title` is required and must be a string"))?;
    if title.chars().count() > MAX_STEP_TITLE_CHARS {
        return Err(format!(
            "`steps[{index}].title` must be at most {MAX_STEP_TITLE_CHARS} characters"
        ));
    }
    let status = step
        .get("status")
        .and_then(Value::as_str)
        .and_then(|s| serde_json::from_value(Value::String(s.to_string())).ok())
        .ok_or_else(|| {
            format!(
                "`steps[{index}].status` must be one of {}",
                STATUSES.join(", ")
            )
        })?;
    Ok(PlanStep {
        title: title.to_string(),
        status,
    })
}
