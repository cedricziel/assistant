use assistant_core::SkillDef;

/// Build the system prompt for ReAct fallback mode.
///
/// The prompt instructs the model to reason step-by-step and emit
/// structured `THOUGHT:` / `ACTION:` / `ANSWER:` lines so the
/// `ReActParser` can extract tool calls without native function-calling.
pub fn build_system_prompt(skills: &[&SkillDef]) -> String {
    let mut prompt = String::new();

    prompt.push_str(
        "You are a helpful AI assistant with access to a set of skills (tools) \
         that you can invoke to answer user requests.\n\n",
    );

    // List available skills
    if skills.is_empty() {
        prompt.push_str("You currently have no skills available.\n\n");
    } else {
        prompt.push_str("## Available Skills\n\n");
        for skill in skills {
            prompt.push_str(&format!("### `{}`\n", skill.name));
            prompt.push_str(&format!("{}\n", skill.description.trim()));

            // If there is a params schema, show a compact summary
            if let Some(schema) = skill.params_schema() {
                if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
                    if !props.is_empty() {
                        prompt.push_str("\nParameters:\n");
                        for (param_name, param_schema) in props {
                            let desc = param_schema
                                .get("description")
                                .and_then(|d| d.as_str())
                                .unwrap_or("(no description)");
                            let type_str = param_schema
                                .get("type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("any");
                            prompt.push_str(&format!("  - `{param_name}` ({type_str}): {desc}\n"));
                        }
                    }
                }
            }
            prompt.push('\n');
        }
    }

    prompt.push_str(
        "## Response Format\n\n\
         You MUST respond using **exactly** one of the two formats below.\n\n\
         ### Format 1 — Invoke a skill\n\
         Use this when you need to call a skill to gather information or perform an action:\n\n\
         ```\n\
         THOUGHT: <your reasoning about what to do next>\n\
         ACTION: {\"name\": \"<skill-name>\", \"params\": {\"key\": \"value\"}}\n\
         ```\n\n\
         ### Format 2 — Final answer\n\
         Use this when you have enough information to answer the user directly:\n\n\
         ```\n\
         ANSWER: <your final response to the user>\n\
         ```\n\n\
         ## Rules\n\n\
         - Always start with `THOUGHT:` before `ACTION:` to show your reasoning.\n\
         - The `ACTION:` value must be valid JSON on a single line.\n\
         - After receiving an `OBSERVATION:` from a skill execution, continue with another\n\
           `THOUGHT:` / `ACTION:` pair or conclude with `ANSWER:`.\n\
         - Do **not** include any text outside the specified format.\n\
         - Skill names must match exactly (case-sensitive).\n\n\
         ## Example\n\n\
         User: What is the weather in Berlin?\n\n\
         ```\n\
         THOUGHT: The user wants the current weather in Berlin. I should use the web-fetch skill\n\
                  to retrieve a weather page.\n\
         ACTION: {\"name\": \"web-fetch\", \"params\": {\"url\": \"https://wttr.in/Berlin?format=3\"}}\n\
         ```\n\n\
         OBSERVATION: Berlin: ⛅️  +12°C\n\n\
         ```\n\
         THOUGHT: I now have the weather data.\n\
         ANSWER: The current weather in Berlin is partly cloudy with a temperature of 12°C.\n\
         ```\n",
    );

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_with_no_skills() {
        let prompt = build_system_prompt(&[]);
        assert!(prompt.contains("no skills available"));
        assert!(prompt.contains("THOUGHT:"));
        assert!(prompt.contains("ACTION:"));
        assert!(prompt.contains("ANSWER:"));
    }

    #[test]
    fn prompt_lists_skill_names() {
        use assistant_core::skill::SkillSource;
        use assistant_core::{SkillDef, SkillTier};
        use std::collections::HashMap;
        use std::path::PathBuf;

        let skill = SkillDef {
            name: "memory-read".to_string(),
            description: "Read from long-term memory.".to_string(),
            license: None,
            compatibility: None,
            allowed_tools: vec![],
            metadata: HashMap::new(),
            body: String::new(),
            dir: PathBuf::from("/tmp/test"),
            tier: SkillTier::Builtin,
            mutating: false,
            confirmation_required: false,
            source: SkillSource::Builtin,
        };

        let prompt = build_system_prompt(&[&skill]);
        assert!(prompt.contains("memory-read"));
        assert!(prompt.contains("Read from long-term memory."));
    }
}
