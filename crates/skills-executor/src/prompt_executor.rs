//! Prompt-tier skill executor.
//!
//! For prompt-tier skills, the actual LLM call is handled by the runtime layer,
//! which passes the SKILL.md body as a sub-prompt directly to the language model.
//! This module is a stub that documents that behaviour and returns an explanatory
//! note when called directly (e.g. during testing).

use std::collections::HashMap;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillOutput};

pub async fn run_prompt(
    def: &SkillDef,
    _params: &HashMap<String, serde_json::Value>,
    _ctx: &ExecutionContext,
) -> Result<SkillOutput> {
    // Prompt-tier skills are resolved by the LLM directly.
    // The runtime injects the skill's SKILL.md body as a sub-prompt and the LLM
    // produces the response. If this function is reached, it means the dispatcher
    // was called outside of the runtime's normal flow.
    Ok(SkillOutput::success(format!(
        "Prompt-tier skill '{}' is resolved directly by the LLM. \
         The runtime injects the following instructions as a sub-prompt:\n\n{}",
        def.name, def.body
    )))
}
