//! System-prompt composition for the orchestrator.
//!
//! Builds the full system prompt from memory files and skill metadata,
//! optionally appending extension-tool instructions for messaging interfaces.

use assistant_llm::ToolSpec;
use assistant_skills::SkillDef as SpecSkillDef;

use super::Orchestrator;

impl Orchestrator {
    /// Compose the full system prompt from memory files and available skills.
    pub(crate) async fn compose_system_prompt(&self) -> String {
        let mut prompt = self.memory_loader.load_system_prompt();
        if let Some(skills_xml) = self.available_skills_xml().await {
            prompt.push_str("\n\n");
            prompt.push_str(&skills_xml);
        }
        prompt
    }

    /// Render the available skills as an XML block for the system prompt.
    ///
    /// Skills are filtered by the active persona's access mode (all/whitelist/blacklist).
    async fn available_skills_xml(&self) -> Option<String> {
        let skills = self
            .registry
            .list_for_persona(&self.agent_id, &self.storage.pool)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    persona = %self.agent_id,
                    error = %e,
                    "Failed to filter skills by persona; returning empty list"
                );
                vec![]
            });
        if skills.is_empty() {
            return None;
        }

        let mut buf = String::new();
        buf.push_str("<available_skills>\n");
        for skill in &skills {
            buf.push_str("  <skill>\n");
            buf.push_str(&format!("    <name>{}</name>\n", escape_xml(&skill.name)));
            buf.push_str(&format!(
                "    <description>{}</description>\n",
                escape_xml(&skill.description)
            ));
            if let Some(location) = skill_location_string(skill) {
                buf.push_str(&format!(
                    "    <location>{}</location>\n",
                    escape_xml(&location)
                ));
            }
            buf.push_str("  </skill>\n");
        }
        buf.push_str("</available_skills>");
        Some(buf)
    }

    /// Build the system prompt for a turn that has extension tools.
    ///
    /// When no extension tool specs are provided, the base prompt is returned
    /// unchanged.  Otherwise we append instructions that tell the LLM how to
    /// use reply / react / block-reply tools and when to call `end_turn`.
    pub(crate) fn build_extension_system_prompt(
        base_system_prompt: &str,
        ext_specs: &[ToolSpec],
    ) -> String {
        if ext_specs.is_empty() {
            return base_system_prompt.to_string();
        }

        let plain_reply: Vec<&str> = ext_specs
            .iter()
            .filter(|s| {
                (s.name.contains("reply") || s.name.contains("post")) && !s.name.contains("block")
            })
            .map(|s| s.name.as_str())
            .collect();
        let block_reply: Vec<&str> = ext_specs
            .iter()
            .filter(|s| s.name.contains("block"))
            .map(|s| s.name.as_str())
            .collect();
        let react_tools: Vec<&str> = ext_specs
            .iter()
            .filter(|s| s.name.contains("react"))
            .map(|s| s.name.as_str())
            .collect();

        let has_reply = !plain_reply.is_empty() || !block_reply.is_empty();
        let has_react = !react_tools.is_empty();

        let ack_instruction = if has_reply && has_react {
            let plain_names = plain_reply.join("`, `");
            let block_names = block_reply.join("`, `");
            let react_names = react_tools.join("`, `");
            let block_clause = if !block_names.is_empty() {
                format!(" or `{block_names}` for rich Block Kit layouts")
            } else {
                String::new()
            };
            format!(
                "Before calling `end_turn` you MUST send exactly one reply to the user.\n\
                 - Use `{plain_names}` for plain-text or mrkdwn responses{block_clause}.\n\
                 - Use `{react_names}` only for a brief emoji-only acknowledgement \
                   (e.g. `thumbsup`, `white_check_mark`) when no text is needed.\n\
                 Call at most ONE reply tool per turn — never call two reply tools \
                 or call the same tool twice.\n"
            )
        } else if has_reply {
            let plain_names = plain_reply.join("`, `");
            let block_names = block_reply.join("`, `");
            let block_clause = if !block_names.is_empty() {
                format!(" or `{block_names}` for rich Block Kit layouts")
            } else {
                String::new()
            };
            format!(
                "Before calling `end_turn` you MUST reply to the user exactly once \
                 using `{plain_names}`{block_clause}. \
                 Never call a reply tool more than once per turn.\n"
            )
        } else if has_react {
            let react_names = react_tools.join("`, `");
            format!(
                "Before calling `end_turn` you MUST acknowledge the user \
                 using `{react_names}` (exactly once).\n"
            )
        } else {
            String::new()
        };

        format!(
            "{base_system_prompt}\n\n---\n\n\
             You are operating inside a messaging interface. \
             {ack_instruction}\
             When you have finished all work, call `end_turn` to signal completion."
        )
    }
}

// ── Module-level helpers ───────────────────────────────────────────────────────

fn skill_location_string(skill: &SpecSkillDef) -> Option<String> {
    let path = skill.dir.join("SKILL.md");
    if path.exists() {
        Some(path.display().to_string())
    } else {
        None
    }
}

fn escape_xml(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
