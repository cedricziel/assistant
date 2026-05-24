//! System-prompt composition for the orchestrator.
//!
//! Builds the full system prompt from memory files and skill metadata,
//! optionally appending extension-tool instructions for messaging interfaces.

use assistant_core::ToolSpec;
use assistant_core::types::conversation::Interface;
use assistant_skills::SkillDef as SpecSkillDef;

use super::Orchestrator;

impl Orchestrator {
    /// Compose the full system prompt from memory files, available skills,
    /// and interface-specific output capabilities.
    pub(crate) async fn compose_system_prompt(&self, interface: &Interface) -> String {
        let mut prompt = self.memory_loader.load_system_prompt();
        if let Some(skills_xml) = self.available_skills_xml().await {
            prompt.push_str("\n\n");
            prompt.push_str(&skills_xml);
        }
        let caps = output_capabilities(interface);
        prompt.push_str("\n\n");
        prompt.push_str(caps);
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

/// Return a static output-capabilities block tailored to the connected interface.
///
/// This text is compiled into the binary — the model cannot edit or delete it.
/// It tells the model what rich output formats the connected client can render.
fn output_capabilities(interface: &Interface) -> &'static str {
    match interface {
        Interface::Web => {
            "## Output capabilities\n\
             \n\
             Your responses render in a rich client that supports:\n\
             - Markdown (headings, bold, italic, lists, links, blockquotes, tables)\n\
             - Fenced code blocks with syntax highlighting\n\
             - ```mermaid — rendered as a visual diagram (flowchart, sequence, state, ER, Gantt, etc.)\n\
             - ```svg — rendered as an inline SVG graphic\n\
             \n\
             Use visual formats when they genuinely aid understanding. Prefer Mermaid for \
             structured diagrams (flows, sequences, ERs). Use SVG for custom visuals, charts, \
             illustrations, or layouts that Mermaid cannot express."
        }
        Interface::Cli => {
            "## Output capabilities\n\
             \n\
             Your responses render in a terminal. Markdown formatting (bold, italic, lists, \
             code blocks) is supported. Fenced ```mermaid blocks are rendered as diagrams \
             when the terminal supports it. Prefer text-based explanations."
        }
        Interface::Slack => {
            "## Output capabilities\n\
             \n\
             Your responses render in Slack. Use mrkdwn formatting (bold, italic, code blocks, \
             lists, links). Do not use ```svg or ```mermaid — they will appear as raw code. \
             Use emoji where appropriate."
        }
        Interface::Matrix => {
            "## Output capabilities\n\
             \n\
             Your responses render in a Matrix client. Basic HTML formatting is supported \
             (bold, italic, code blocks, lists, links). Images can be sent as attachments. \
             Do not use ```svg or ```mermaid — they will appear as raw code."
        }
        Interface::Signal => {
            "## Output capabilities\n\
             \n\
             Your responses render as plain text in a mobile messaging app. No markdown, \
             no images, no diagrams. Keep responses concise and text-only."
        }
        Interface::Mattermost => {
            "## Output capabilities\n\
             \n\
             Your responses render in Mattermost. Use Markdown formatting (bold, italic, \
             code blocks, lists, links, tables). Do not use ```svg or ```mermaid — they \
             will appear as raw code."
        }
        Interface::Nextcloud => {
            "## Output capabilities\n\
             \n\
             Your responses render in Nextcloud Talk. Use Markdown formatting (bold, italic, \
             code blocks, lists, links). Do not use ```svg or ```mermaid — they will appear \
             as raw code."
        }
        Interface::Mcp => {
            "## Output capabilities\n\
             \n\
             Your responses are consumed programmatically via the MCP protocol. Return \
             structured, well-formatted text. Do not use ```svg or ```mermaid."
        }
        Interface::Scheduler => {
            "## Output capabilities\n\
             \n\
             You are running as a background scheduled task. Your output may be logged or \
             forwarded. Use plain text. Do not use ```svg or ```mermaid."
        }
        Interface::A2a => {
            "## Output capabilities\n\
             \n\
             Your responses are consumed by another agent over the A2A protocol. Return \
             structured, well-formatted text. Do not use ```svg or ```mermaid."
        }
    }
}

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

#[cfg(test)]
mod tests {
    use assistant_core::types::conversation::Interface;

    use super::output_capabilities;

    #[test]
    fn web_mentions_svg_and_mermaid() {
        let caps = output_capabilities(&Interface::Web);
        assert!(caps.contains("svg"), "Web capabilities should mention svg");
        assert!(
            caps.contains("mermaid"),
            "Web capabilities should mention mermaid"
        );
    }

    #[test]
    fn signal_omits_svg_and_mermaid() {
        let caps = output_capabilities(&Interface::Signal);
        assert!(
            !caps.contains("svg"),
            "Signal capabilities should not mention svg"
        );
        assert!(
            !caps.contains("mermaid"),
            "Signal capabilities should not mention mermaid"
        );
    }

    #[test]
    fn slack_mentions_mrkdwn() {
        let caps = output_capabilities(&Interface::Slack);
        assert!(
            caps.contains("mrkdwn"),
            "Slack capabilities should mention mrkdwn"
        );
    }

    #[test]
    fn every_variant_is_non_empty() {
        let interfaces = [
            Interface::Cli,
            Interface::Signal,
            Interface::Mcp,
            Interface::Slack,
            Interface::Mattermost,
            Interface::Nextcloud,
            Interface::Web,
            Interface::Scheduler,
            Interface::Matrix,
            Interface::A2a,
        ];
        for iface in &interfaces {
            let caps = output_capabilities(iface);
            assert!(
                !caps.is_empty(),
                "output_capabilities for {iface:?} should not be empty"
            );
        }
    }

    // ── escape_xml ────────────────────────────────────────────────────────

    use super::escape_xml;

    #[test]
    fn escape_xml_handles_all_special_chars() {
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("\"hi\""), "&quot;hi&quot;");
        assert_eq!(escape_xml("it's"), "it&apos;s");
    }

    #[test]
    fn escape_xml_preserves_safe_text() {
        assert_eq!(escape_xml(""), "");
        assert_eq!(escape_xml("plain text"), "plain text");
        assert_eq!(escape_xml("123abc"), "123abc");
        // UTF-8 / emoji passes through unchanged.
        assert_eq!(escape_xml("héllo 🦀"), "héllo 🦀");
    }

    // ── skill_location_string ─────────────────────────────────────────────

    use super::skill_location_string;
    use assistant_skills::{SkillDef, SkillSource};

    fn make_skill(dir: std::path::PathBuf) -> SkillDef {
        SkillDef {
            name: "test".into(),
            description: "x".into(),
            license: None,
            compatibility: None,
            allowed_tools: vec![],
            metadata: Default::default(),
            body: "body".into(),
            dir,
            source: SkillSource::Builtin,
        }
    }

    #[test]
    fn skill_location_returns_path_when_skill_md_exists() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("SKILL.md"), "---\nname: test\n---\n").unwrap();
        let skill = make_skill(tmp.path().to_path_buf());
        let loc = skill_location_string(&skill).expect("path should resolve");
        assert!(loc.ends_with("SKILL.md"), "got: {loc}");
    }

    #[test]
    fn skill_location_returns_none_when_skill_md_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let skill = make_skill(tmp.path().to_path_buf());
        // No SKILL.md written → returns None.
        assert!(skill_location_string(&skill).is_none());
    }
}
