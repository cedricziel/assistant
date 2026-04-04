//! Shared text sanitization utilities for LLM output post-processing.

// ── Think-tag stripping ───────────────────────────────────────────────────────

/// Strip `<think>…</think>` blocks that some models (e.g. qwen3) embed inline
/// in their response text when not using a dedicated thinking API.
///
/// The full original text (including think blocks) is preserved in the database
/// by the orchestrator; this function only removes them before posting to users
/// so they never see raw reasoning output.
///
/// Matching is case-insensitive. If a `<think>` tag is unclosed, everything
/// from that tag onward is discarded.
pub fn strip_think_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let lower = input.to_lowercase();
    let mut pos = 0;
    while pos < input.len() {
        match lower[pos..].find("<think>") {
            Some(open_rel) => {
                let open_abs = pos + open_rel;
                result.push_str(&input[pos..open_abs]);
                match lower[open_abs..].find("</think>") {
                    Some(close_rel) => {
                        pos = open_abs + close_rel + "</think>".len();
                    }
                    None => break, // unclosed tag — discard the rest
                }
            }
            None => {
                result.push_str(&input[pos..]);
                break;
            }
        }
    }
    result.trim().to_string()
}

// ── Cite-tag stripping ────────────────────────────────────────────────────────

/// Strip `<cite index="…">…</cite>` tags that some models embed to attribute
/// sources, keeping only the inner text.
///
/// Unlike think-tags the *content* of a cite block is meaningful and must be
/// preserved; only the surrounding tags are removed.
pub fn strip_cite_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut pos = 0;
    let bytes = input.as_bytes();

    while pos < bytes.len() {
        // Look for `<cite` (case-sensitive — models emit lowercase).
        match input[pos..].find("<cite") {
            Some(open_rel) => {
                let open_abs = pos + open_rel;
                // The character right after `<cite` must be a tag boundary
                // (whitespace, `>`, or `/`) so we don't match `<cited>` etc.
                let after = open_abs + "<cite".len();
                if after < input.len() {
                    let boundary = input.as_bytes()[after];
                    if boundary != b' '
                        && boundary != b'>'
                        && boundary != b'/'
                        && boundary != b'\t'
                        && boundary != b'\n'
                    {
                        // Not a real `<cite` tag — copy the `<` and keep scanning.
                        result.push_str(&input[pos..open_abs + 1]);
                        pos = open_abs + 1;
                        continue;
                    }
                }
                // Copy everything before the tag.
                result.push_str(&input[pos..open_abs]);

                // Find the closing `>` of the opening tag.
                match input[open_abs..].find('>') {
                    Some(gt_rel) => {
                        let content_start = open_abs + gt_rel + 1;
                        // Find the matching `</cite>`.
                        match input[content_start..].find("</cite>") {
                            Some(close_rel) => {
                                // Keep the inner content.
                                result.push_str(&input[content_start..content_start + close_rel]);
                                pos = content_start + close_rel + "</cite>".len();
                            }
                            None => {
                                // Unclosed cite — keep everything after the opening tag as-is.
                                result.push_str(&input[content_start..]);
                                return result;
                            }
                        }
                    }
                    None => {
                        // Malformed opening tag — copy remainder and bail.
                        result.push_str(&input[open_abs..]);
                        return result;
                    }
                }
            }
            None => {
                result.push_str(&input[pos..]);
                break;
            }
        }
    }
    result
}

// ── Combined sanitization ─────────────────────────────────────────────────────

/// Sanitize LLM output for display to users.
///
/// Strips `<think>` reasoning blocks and `<cite>` wrapper tags (preserving
/// cite inner content). Returns `None` when nothing visible remains after
/// sanitization.
pub fn sanitize_llm_output(input: &str) -> Option<String> {
    let cleaned = strip_cite_tags(&strip_think_tags(input));
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

// ── Preview utility ───────────────────────────────────────────────────────────

/// Return a UTF-8 character-bounded prefix of `s` for log output.
///
/// Avoids flooding logs with long messages. Safe on multi-byte characters.
pub fn preview(s: &str, max_chars: usize) -> &str {
    let end = s
        .char_indices()
        .nth(max_chars)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    &s[..end]
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{preview, sanitize_llm_output, strip_cite_tags, strip_think_tags};

    // strip_think_tags

    #[test]
    fn think_tags_stripped() {
        assert_eq!(
            strip_think_tags("<think>reasoning</think>\nHello!"),
            "Hello!"
        );
    }

    #[test]
    fn think_tags_middle_stripped() {
        assert_eq!(
            strip_think_tags("Before<think>reasoning</think>After"),
            "BeforeAfter"
        );
    }

    #[test]
    fn no_think_tags_unchanged() {
        assert_eq!(strip_think_tags("Hello world"), "Hello world");
    }

    #[test]
    fn only_think_tags_returns_empty() {
        assert_eq!(strip_think_tags("<think>all thinking</think>"), "");
    }

    // strip_cite_tags

    #[test]
    fn cite_tags_stripped_content_kept() {
        assert_eq!(
            strip_cite_tags(r#"Hello <cite index="1-2">world</cite>!"#),
            "Hello world!"
        );
    }

    #[test]
    fn multiple_cite_tags_stripped() {
        let input = r#"<cite index="1">A</cite> and <cite index="2-3,4">B</cite>"#;
        assert_eq!(strip_cite_tags(input), "A and B");
    }

    #[test]
    fn no_cite_tags_unchanged() {
        assert_eq!(strip_cite_tags("Hello world"), "Hello world");
    }

    #[test]
    fn cited_tag_not_stripped() {
        let input = "<cited>foo</cited>";
        assert_eq!(strip_cite_tags(input), input);
    }

    #[test]
    fn cite_tag_with_complex_index() {
        let input = r#"<cite index="14-11,14-12,14-13,14-14,14-15,14-16">Directions here.</cite>"#;
        assert_eq!(strip_cite_tags(input), "Directions here.");
    }

    #[test]
    fn unclosed_cite_tag_keeps_content() {
        let input = r#"Before <cite index="1">unclosed content"#;
        assert_eq!(strip_cite_tags(input), "Before unclosed content");
    }

    #[test]
    fn cite_tags_in_realistic_message() {
        let input = "P3 — Freiluftparkplatz. \
            <cite index=\"14-11,14-12\">Aus Richtung A52 bleiben.</cite>\n\n\
            <cite index=\"14-22\">Parkgebühr: 7 Euro.</cite>";
        let expected = "P3 — Freiluftparkplatz. Aus Richtung A52 bleiben.\n\nParkgebühr: 7 Euro.";
        assert_eq!(strip_cite_tags(input), expected);
    }

    // sanitize_llm_output

    #[test]
    fn sanitize_strips_both_and_trims() {
        let input = "<think>reasoning</think>\nHello <cite>world</cite>!";
        assert_eq!(sanitize_llm_output(input), Some("Hello world!".to_string()));
    }

    #[test]
    fn sanitize_returns_none_when_empty_after_stripping() {
        assert_eq!(sanitize_llm_output("<think>only thinking</think>"), None);
    }

    #[test]
    fn sanitize_returns_none_for_blank_input() {
        assert_eq!(sanitize_llm_output("   "), None);
    }

    // preview

    #[test]
    fn preview_truncates_at_char_boundary() {
        assert_eq!(preview("Hello world", 5), "Hello");
    }

    #[test]
    fn preview_returns_full_string_when_shorter_than_max() {
        assert_eq!(preview("Hi", 100), "Hi");
    }

    #[test]
    fn preview_handles_multibyte_chars() {
        let s = "héllo";
        assert_eq!(preview(s, 3), "hél");
    }
}
